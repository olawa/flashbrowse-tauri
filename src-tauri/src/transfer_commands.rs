//! Streaming file transfers with progress, cancellation and resume.
//!
//! The previous implementation shelled out to `scp` and waited for it to exit,
//! so a 200 GB BAM showed a spinner for half an hour, could not be cancelled,
//! and started from zero if the connection dropped. This module runs `rsync`
//! instead - one invocation per selected item, so the UI can name the file it
//! is on - parses its progress output, and reports it as Tauri events.
//!
//! `rsync` is not everywhere (notably not on Windows), so every path has an
//! `scp` fallback. The fallback has no progress, but it still works.

use crate::fs_commands::{resolve_path, ConflictStrategy, CONFLICT_ERROR_PREFIX};
use crate::ssh_commands::{scp_base_args, sh_quote, ssh_base_args};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// What the UI shows while a transfer runs. Emitted as `transfer-progress`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub id: String,
    /// Name of the item currently being transferred.
    pub current_file: String,
    pub files_done: usize,
    pub files_total: usize,
    /// 0-100 across the whole batch, counting finished items plus the current
    /// item's own progress.
    pub percent: f32,
    /// Human-readable rate from rsync, e.g. "12.34MB/s". Empty when unknown.
    pub speed: String,
    /// rsync's estimate for the current item, e.g. "0:01:12". Empty when unknown.
    pub eta: String,
    pub done: bool,
    pub cancelled: bool,
    pub error: Option<String>,
}

impl TransferProgress {
    fn new(id: &str, files_total: usize) -> Self {
        Self {
            id: id.to_string(),
            current_file: String::new(),
            files_done: 0,
            files_total,
            percent: 0.0,
            speed: String::new(),
            eta: String::new(),
            done: false,
            cancelled: false,
            error: None,
        }
    }
}

/// Children of running transfers, so cancel_transfer can kill them.
static RUNNING: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);

fn running_lock() -> std::sync::MutexGuard<'static, Option<HashMap<String, u32>>> {
    RUNNING.lock().unwrap_or_else(|p| p.into_inner())
}

fn register(id: &str, pid: u32) {
    let mut guard = running_lock();
    guard.get_or_insert_with(HashMap::new).insert(id.to_string(), pid);
}

fn unregister(id: &str) {
    if let Some(map) = running_lock().as_mut() {
        map.remove(id);
    }
}

fn is_cancelled(id: &str) -> bool {
    running_lock()
        .as_ref()
        .map(|m| !m.contains_key(id))
        .unwrap_or(true)
}

/// Where one side of a transfer lives.
#[derive(Clone)]
pub struct Endpoint {
    pub is_ssh: bool,
    pub host: String,
}

impl Endpoint {
    /// The path as rsync/scp should see it: quoted for the remote shell when
    /// the endpoint is remote, plain otherwise.
    fn spec(&self, path: &str) -> String {
        if self.is_ssh {
            format!("{}:{}", self.host, sh_quote(path))
        } else {
            path.to_string()
        }
    }
}

fn rsync_binary() -> Option<PathBuf> {
    crate::bio_commands::find_tool_executable("rsync")
}

/// The ssh command rsync should use, carrying our connection options.
///
/// Reuses the multiplexed control socket, so one rsync per file does not mean
/// one TCP handshake and one authentication per file.
fn rsync_ssh_option() -> String {
    let opts: Vec<String> = ssh_base_args()
        .into_iter()
        .filter(|a| a != "--")
        .collect();
    format!("ssh {}", opts.join(" "))
}

/// Parse one rsync `--info=progress2` chunk: "  1,234,567  45%  12.34MB/s  0:00:12".
fn parse_progress_line(line: &str) -> Option<(f32, String, String)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let percent_idx = fields.iter().position(|f| f.ends_with('%'))?;
    let percent = fields[percent_idx].trim_end_matches('%').parse::<f32>().ok()?;
    let speed = fields.get(percent_idx + 1).copied().unwrap_or("").to_string();
    let eta = fields.get(percent_idx + 2).copied().unwrap_or("").to_string();
    Some((percent, speed, eta))
}

/// Read rsync's output and report progress until the child exits.
///
/// rsync separates progress updates with carriage returns rather than newlines,
/// so the stream is split on both.
fn pump_progress<F: FnMut(f32, String, String)>(stdout: impl Read, mut on_progress: F) {
    let mut reader = BufReader::new(stdout);
    let mut chunk: Vec<u8> = Vec::new();

    loop {
        chunk.clear();
        // Progress updates end with \r; final lines end with \n.
        let read = reader.read_until(b'\r', &mut chunk);
        match read {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&chunk);
                for part in text.split(['\r', '\n']) {
                    if let Some((percent, speed, eta)) = parse_progress_line(part) {
                        on_progress(percent, speed, eta);
                    }
                }
            }
        }
    }
}

/// Which items already exist at the destination.
fn existing_at_destination(
    dest: &Endpoint,
    dest_dir: &str,
    names: &[String],
) -> Result<Vec<String>, String> {
    if names.is_empty() {
        return Ok(Vec::new());
    }

    if !dest.is_ssh {
        let dir = resolve_path(dest_dir);
        return Ok(names
            .iter()
            .filter(|n| dir.join(n).symlink_metadata().is_ok())
            .cloned()
            .collect());
    }

    // One probe for the whole batch: print the names that exist remotely.
    let checks: Vec<String> = names
        .iter()
        .map(|n| {
            let full = format!("{}/{}", dest_dir.trim_end_matches('/'), n);
            format!("[ -e {} ] && printf '%s\\n' {}", sh_quote(&full), sh_quote(n))
        })
        .collect();
    let script = format!("{}; true", checks.join("; "));

    let mut args = ssh_base_args();
    args.push(dest.host.clone());
    args.push(script);

    let out = Command::new("ssh")
        .args(&args)
        .output()
        .map_err(|e| format!("Kunde inte kontrollera målmappen: {}", e))?;

    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

pub struct TransferSpec {
    pub source: Endpoint,
    pub source_paths: Vec<String>,
    pub dest: Endpoint,
    pub dest_dir: String,
    pub strategy: ConflictStrategy,
}

/// Run a transfer, reporting progress through `app` when one is given.
///
/// Returns the summary message shown when it finishes.
pub fn run_transfer(app: Option<AppHandle>, id: &str, spec: TransferSpec) -> Result<String, String> {
    let TransferSpec {
        source,
        source_paths,
        dest,
        dest_dir,
        strategy,
    } = spec;

    if source_paths.is_empty() {
        return Ok("Inga filer valda".to_string());
    }

    let names: Vec<String> = source_paths
        .iter()
        .filter_map(|p| {
            Path::new(p.trim_end_matches('/'))
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .collect();

    // Check both local and remote destinations before moving a single byte.
    if strategy == ConflictStrategy::Fail {
        let existing = existing_at_destination(&dest, &dest_dir, &names)?;
        if !existing.is_empty() {
            return Err(format!("{}{}", CONFLICT_ERROR_PREFIX, existing.join("\n")));
        }
    }

    if !dest.is_ssh {
        let dir = resolve_path(&dest_dir);
        if !dir.is_dir() {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("Kunde inte skapa målmapp {}: {}", dir.display(), e))?;
        }
    }

    let mut progress = TransferProgress::new(id, source_paths.len());
    let emit = |p: &TransferProgress| {
        if let Some(app) = &app {
            let _ = app.emit("transfer-progress", p);
        }
    };
    emit(&progress);

    // rsync cannot copy between two remote hosts; scp -3 can.
    let both_remote = source.is_ssh && dest.is_ssh;
    let rsync = if both_remote { None } else { rsync_binary() };

    register(id, 0);

    let result = (|| -> Result<(), String> {
        for (index, path) in source_paths.iter().enumerate() {
            if is_cancelled(id) {
                return Ok(());
            }

            progress.current_file = names.get(index).cloned().unwrap_or_default();
            progress.files_done = index;
            progress.percent = (index as f32 / source_paths.len() as f32) * 100.0;
            emit(&progress);

            let target = resolve_target(&dest, &dest_dir, &progress.current_file, strategy);

            match &rsync {
                Some(bin) => run_rsync(
                    bin, id, &source, path, &dest, &target, index, source_paths.len(),
                    &mut progress, &emit,
                )?,
                // Without rsync: copy locally through the filesystem, and reach
                // remote hosts with scp. Neither reports progress.
                None if !source.is_ssh && !dest.is_ssh => copy_local(path, &target)?,
                None => run_scp(id, &source, path, &dest, &target)?,
            }
        }
        Ok(())
    })();

    let cancelled = is_cancelled(id);
    unregister(id);

    progress.done = true;
    progress.cancelled = cancelled;
    progress.files_done = if cancelled { progress.files_done } else { source_paths.len() };
    progress.percent = if cancelled { progress.percent } else { 100.0 };
    progress.error = result.as_ref().err().cloned();
    emit(&progress);

    result?;

    if cancelled {
        Ok("Överföring avbruten".to_string())
    } else {
        Ok(format!("Överförde {} objekt", source_paths.len()))
    }
}

/// Destination path for one item, applying the keep-both rule locally.
///
/// Remote destinations keep the plain name: renaming there would need another
/// round trip per file, and the conflict probe above already gave the user the
/// choice.
fn resolve_target(dest: &Endpoint, dest_dir: &str, name: &str, strategy: ConflictStrategy) -> String {
    if dest.is_ssh || strategy != ConflictStrategy::Rename {
        return format!("{}/{}", dest_dir.trim_end_matches('/'), name);
    }
    let unique = crate::fs_commands::unique_target(&resolve_path(dest_dir).join(name));
    unique.to_string_lossy().to_string()
}

/// Local copy for platforms without rsync (Windows, mainly).
fn copy_local(source_path: &str, target: &str) -> Result<(), String> {
    let src = resolve_path(source_path);
    let dst = PathBuf::from(target);
    if src.is_dir() {
        crate::fs_commands::copy_dir_recursive(&src, &dst).map_err(|e| e.to_string())
    } else {
        std::fs::copy(&src, &dst)
            .map(|_| ())
            .map_err(|e| format!("Kunde inte kopiera {}: {}", source_path, e))
    }
}

#[allow(clippy::too_many_arguments)]
fn run_rsync<F: Fn(&TransferProgress)>(
    bin: &Path,
    id: &str,
    source: &Endpoint,
    source_path: &str,
    dest: &Endpoint,
    target: &str,
    index: usize,
    total: usize,
    progress: &mut TransferProgress,
    emit: &F,
) -> Result<(), String> {
    let mut cmd = Command::new(bin);
    cmd.arg("-a")
        // --partial keeps what was transferred, so a cancelled or dropped
        // transfer resumes instead of starting over.
        .arg("--partial")
        .arg("--info=progress2")
        .arg("--no-inc-recursive");

    if source.is_ssh || dest.is_ssh {
        cmd.arg("-e").arg(rsync_ssh_option());
    }

    cmd.arg("--")
        .arg(source.spec(source_path))
        .arg(dest.spec(target))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Kunde inte starta rsync: {}", e))?;
    register(id, child.id());

    let stdout = child.stdout.take();
    let mut stderr_reader = child.stderr.take();

    if let Some(stdout) = stdout {
        pump_progress(stdout, |percent, speed, eta| {
            // Weight this item's progress into the batch total.
            progress.percent = ((index as f32 + percent / 100.0) / total as f32) * 100.0;
            progress.speed = speed;
            progress.eta = eta;
            emit(progress);
        });
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    let mut stderr = String::new();
    if let Some(reader) = stderr_reader.as_mut() {
        let _ = reader.read_to_string(&mut stderr);
    }

    if status.success() || is_cancelled(id) {
        Ok(())
    } else {
        Err(format!(
            "Överföring misslyckades ({}): {}",
            source_path,
            stderr.trim()
        ))
    }
}

/// Fallback for hosts without rsync, and for remote-to-remote copies.
fn run_scp(
    id: &str,
    source: &Endpoint,
    source_path: &str,
    dest: &Endpoint,
    target: &str,
) -> Result<(), String> {
    let mut args: Vec<String> = vec!["-r".into()];
    if source.is_ssh && dest.is_ssh {
        args.push("-3".into());
    }
    args.extend(scp_base_args());
    args.push(source.spec(source_path));
    args.push(dest.spec(target));

    let mut child = Command::new("scp")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Kunde inte starta scp: {}", e))?;
    register(id, child.id());

    let mut stderr_reader = child.stderr.take();
    let status = child.wait().map_err(|e| e.to_string())?;

    let mut stderr = String::new();
    if let Some(reader) = stderr_reader.as_mut() {
        let _ = reader.read_to_string(&mut stderr);
    }

    if status.success() || is_cancelled(id) {
        Ok(())
    } else {
        Err(format!("Överföring misslyckades ({}): {}", source_path, stderr.trim()))
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_transfer(
    app: AppHandle,
    id: String,
    source_is_ssh: bool,
    source_ssh_host: String,
    source_paths: Vec<String>,
    dest_is_ssh: bool,
    dest_ssh_host: String,
    dest_dir: String,
    on_conflict: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_transfer(
            Some(app),
            &id,
            TransferSpec {
                source: Endpoint { is_ssh: source_is_ssh, host: source_ssh_host },
                source_paths,
                dest: Endpoint { is_ssh: dest_is_ssh, host: dest_ssh_host },
                dest_dir,
                strategy: ConflictStrategy::parse(on_conflict),
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Stop a running transfer. Partially transferred data is kept, so starting the
/// same transfer again resumes it.
#[tauri::command]
pub fn cancel_transfer(id: String) -> Result<bool, String> {
    let pid = {
        let mut guard = running_lock();
        match guard.as_mut().and_then(|m| m.remove(&id)) {
            Some(pid) => pid,
            None => return Ok(false),
        }
    };

    if pid != 0 {
        // SIGTERM rather than SIGKILL: rsync then closes cleanly and leaves the
        // partial file in place for --partial to resume from.
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }
    Ok(true)
}

/// Whether progress reporting is available, so the UI can say so up front.
#[tauri::command]
pub fn transfer_backend() -> String {
    match rsync_binary() {
        Some(_) => "rsync".to_string(),
        None => "scp".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rsync_progress_output() {
        let (percent, speed, eta) =
            parse_progress_line("      1,234,567  45%   12.34MB/s    0:00:12").expect("parsed");
        assert_eq!(percent, 45.0);
        assert_eq!(speed, "12.34MB/s");
        assert_eq!(eta, "0:00:12");
    }

    #[test]
    fn ignores_lines_without_a_percentage() {
        assert!(parse_progress_line("sending incremental file list").is_none());
        assert!(parse_progress_line("").is_none());
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fb_transfer_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("dst")).unwrap();
        dir
    }

    fn local(paths: Vec<String>, dest: &Path, strategy: ConflictStrategy) -> TransferSpec {
        TransferSpec {
            source: Endpoint { is_ssh: false, host: String::new() },
            source_paths: paths,
            dest: Endpoint { is_ssh: false, host: String::new() },
            dest_dir: dest.to_string_lossy().to_string(),
            strategy,
        }
    }

    #[test]
    fn transfers_a_local_file_and_keeps_the_contents() {
        let dir = temp_dir("copy");
        let src = dir.join("src/reads.fastq");
        std::fs::write(&src, b"@read1\nACGT\n").unwrap();

        let msg = run_transfer(
            None,
            "test-copy",
            local(vec![src.to_string_lossy().to_string()], &dir.join("dst"), ConflictStrategy::Fail),
        )
        .expect("transfer");

        assert!(msg.contains("1"), "summary was: {msg}");
        assert_eq!(
            std::fs::read(dir.join("dst/reads.fastq")).unwrap(),
            b"@read1\nACGT\n"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_existing_destination_stops_the_transfer_before_it_starts() {
        let dir = temp_dir("conflict");
        let src = dir.join("src/sample.bam");
        std::fs::write(&src, b"new").unwrap();
        std::fs::write(dir.join("dst/sample.bam"), b"precious").unwrap();

        let err = run_transfer(
            None,
            "test-conflict",
            local(vec![src.to_string_lossy().to_string()], &dir.join("dst"), ConflictStrategy::Fail),
        )
        .expect_err("should refuse");

        assert!(err.starts_with(CONFLICT_ERROR_PREFIX));
        assert!(err.contains("sample.bam"));
        assert_eq!(std::fs::read(dir.join("dst/sample.bam")).unwrap(), b"precious");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn keep_both_lands_next_to_the_existing_file() {
        let dir = temp_dir("keepboth");
        let src = dir.join("src/sample.bam");
        std::fs::write(&src, b"new").unwrap();
        std::fs::write(dir.join("dst/sample.bam"), b"precious").unwrap();

        run_transfer(
            None,
            "test-keepboth",
            local(vec![src.to_string_lossy().to_string()], &dir.join("dst"), ConflictStrategy::Rename),
        )
        .expect("transfer");

        assert_eq!(std::fs::read(dir.join("dst/sample.bam")).unwrap(), b"precious");
        assert_eq!(std::fs::read(dir.join("dst/sample 2.bam")).unwrap(), b"new");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cancelling_stops_the_transfer_and_keeps_partial_data() {
        if rsync_binary().is_none() {
            return; // Cancellation is only meaningful for the rsync path.
        }

        let dir = temp_dir("cancel");
        let src = dir.join("src/big.bin");
        // Large enough that the copy is still running when we cancel.
        let block = vec![b'x'; 1024 * 1024];
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&src).unwrap();
            for _ in 0..400 {
                f.write_all(&block).unwrap();
            }
        }

        let paths = vec![src.to_string_lossy().to_string()];
        let dest = dir.join("dst");
        let handle = std::thread::spawn(move || {
            run_transfer(None, "test-cancel", local(paths, &dest, ConflictStrategy::Fail))
        });

        // Wait for rsync to register itself, then stop it.
        let mut waited = 0;
        while is_cancelled("test-cancel") && waited < 5000 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            waited += 20;
        }
        std::thread::sleep(std::time::Duration::from_millis(120));
        let cancelled = cancel_transfer("test-cancel".to_string()).unwrap();

        let msg = handle.join().unwrap().expect("transfer should end cleanly");
        assert!(cancelled, "cancel should find the running transfer");
        assert!(msg.contains("avbruten"), "summary was: {msg}");

        // rsync --partial leaves what it managed to copy, so a retry resumes.
        let copied: u64 = std::fs::read_dir(dir.join("dst"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();
        assert!(copied < 400 * 1024 * 1024, "should not have copied everything");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn progress_is_read_from_real_rsync_output() {
        let Some(bin) = rsync_binary() else { return };

        let dir = temp_dir("progress");
        let src = dir.join("src/big.bin");
        {
            use std::io::Write;
            let block = vec![b'x'; 1024 * 1024];
            let mut f = std::fs::File::create(&src).unwrap();
            for _ in 0..120 {
                f.write_all(&block).unwrap();
            }
        }

        let mut child = Command::new(bin)
            .arg("-a")
            .arg("--partial")
            .arg("--info=progress2")
            .arg("--no-inc-recursive")
            .arg("--")
            .arg(&src)
            .arg(dir.join("dst/big.bin"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("rsync");

        let mut samples: Vec<f32> = Vec::new();
        pump_progress(child.stdout.take().unwrap(), |percent, _speed, _eta| {
            samples.push(percent);
        });
        child.wait().unwrap();

        assert!(!samples.is_empty(), "rsync reported no progress at all");
        assert_eq!(samples.last().copied(), Some(100.0), "should end at 100%");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remote_paths_are_quoted_local_ones_are_not() {
        let remote = Endpoint { is_ssh: true, host: "cluster".into() };
        let local = Endpoint { is_ssh: false, host: String::new() };
        assert_eq!(remote.spec("/data/my run.bam"), "cluster:'/data/my run.bam'");
        assert_eq!(local.spec("/data/my run.bam"), "/data/my run.bam");
    }
}
