use crate::fs_commands::{dirs_home, resolve_path};
use crate::models::{TabCompletionResult, TerminalOutput};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const COMMON_COMMANDS: &[&str] = &[
    "cd", "ls", "pwd", "mkdir", "rmdir", "cp", "mv", "rm", "touch", "cat", "less", "more", "head", "tail",
    "grep", "egrep", "fgrep", "find", "du", "df", "chmod", "chown", "ps", "top", "kill", "pkill", "killall",
    "open", "clear", "echo", "export", "source", "which", "where", "env", "history", "alias", "unalias",
    "git", "ssh", "scp", "rsync", "sftp", "curl", "wget", "tar", "gzip", "gunzip", "zip", "unzip",
    "python", "python3", "pip", "pip3", "conda", "mamba", "micromamba", "snakemake", "nextflow",
    "rsnap", "samtools", "bcftools", "bedtools", "tabix", "bgzip", "fastqc", "multiqc", "bwa", "bowtie2", "minimap2",
    "nano", "vim", "vi", "emacs", "code", "zsh", "bash", "sh", "brew", "cargo", "rustc", "swift", "swiftc",
    "make", "cmake", "docker", "singularity", "apptainer", "slurm", "sbatch", "squeue", "scancel",
];

/// The terminal collects output and shows it when the command finishes, so a
/// command that never exits - `tail -f`, something waiting on stdin, a hung
/// network mount - would leave the pane stuck with no way to cancel. The limit
/// is generous enough for real work (sorting a BAM, an alignment run) while
/// still ending a hang.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(600);

/// Output beyond this is dropped rather than buffered into the UI. `find /` or a
/// tool looping on errors can produce gigabytes.
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

/// Read a stream to EOF, keeping at most MAX_OUTPUT_BYTES.
///
/// Reading continues past the cap (discarding the rest) so the child never
/// blocks on a full pipe - that would hang the command we are trying to bound.
fn read_capped<R: Read + Send + 'static>(mut stream: R) -> std::thread::JoinHandle<(String, bool)> {
    std::thread::spawn(move || {
        let mut kept: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut truncated = false;

        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let room = MAX_OUTPUT_BYTES.saturating_sub(kept.len());
                    if room == 0 {
                        truncated = true;
                    } else {
                        let take = n.min(room);
                        kept.extend_from_slice(&chunk[..take]);
                        if take < n {
                            truncated = true;
                        }
                    }
                }
            }
        }

        (String::from_utf8_lossy(&kept).to_string(), truncated)
    })
}

/// Run one shell command with a wall-clock bound and a cap on collected output.
fn run_shell(
    trimmed: &str,
    working_dir: &PathBuf,
    timeout: Duration,
) -> Result<TerminalOutput, String> {
    #[cfg(not(target_os = "windows"))]
    let mut command = Command::new("sh");
    #[cfg(not(target_os = "windows"))]
    command.arg("-c").arg(trimmed);

    #[cfg(target_os = "windows")]
    let mut command = Command::new("cmd");
    #[cfg(target_os = "windows")]
    command.args(["/c", trimmed]);

    // stdin is closed: an interactive command fails immediately instead of
    // waiting forever for input the terminal pane cannot provide.
    let mut child = command
        .current_dir(&working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout_reader = child.stdout.take().map(read_capped);
    let stderr_reader = child.stderr.take().map(read_capped);

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("Failed to wait for command: {}", e)),
        }
    };

    let (mut stdout, stdout_truncated) = stdout_reader
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let (mut stderr, stderr_truncated) = stderr_reader
        .and_then(|h| h.join().ok())
        .unwrap_or_default();

    if stdout_truncated {
        stdout.push_str(&format!(
            "\n[Utdata klippt vid {} MB]\n",
            MAX_OUTPUT_BYTES / (1024 * 1024)
        ));
    }
    if stderr_truncated {
        stderr.push_str(&format!(
            "\n[Felutdata klippt vid {} MB]\n",
            MAX_OUTPUT_BYTES / (1024 * 1024)
        ));
    }
    if timed_out {
        stderr.push_str(&format!(
            "\n[Kommandot avbröts efter {} sekunder]\n",
            timeout.as_secs()
        ));
    }

    Ok(TerminalOutput {
        stdout,
        stderr,
        exit_code: status.and_then(|s| s.code()).unwrap_or(-1),
        new_cwd: None,
    })
}

#[tauri::command]
pub async fn run_command(cmd: String, cwd: String) -> Result<TerminalOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return Ok(TerminalOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
                new_cwd: None,
            });
        }

        let working_dir = resolve_path(&cwd);

        // 1. Handle "cd" command directly
        if trimmed == "cd" || trimmed == "cd ~" {
            let home = dirs_home().to_string_lossy().to_string();
            return Ok(TerminalOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
                new_cwd: Some(home),
            });
        } else if trimmed.starts_with("cd ") {
            let target_str = trimmed[3..].trim().trim_matches('"').trim_matches('\'');
            let target_path = if target_str == "~" || target_str.starts_with("~/") {
                resolve_path(target_str)
            } else if target_str.starts_with('/') {
                PathBuf::from(target_str)
            } else {
                working_dir.join(target_str)
            };

            if target_path.is_dir() {
                if let Ok(canonical) = target_path.canonicalize() {
                    return Ok(TerminalOutput {
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code: 0,
                        new_cwd: Some(canonical.to_string_lossy().to_string()),
                    });
                } else {
                    return Ok(TerminalOutput {
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code: 0,
                        new_cwd: Some(target_path.to_string_lossy().to_string()),
                    });
                }
            } else {
                return Ok(TerminalOutput {
                    stdout: String::new(),
                    stderr: format!("cd: no such file or directory: {}\n", target_str),
                    exit_code: 1,
                    new_cwd: None,
                });
            }
        }

        // 2. Generic shell execution, bounded in time and output size
        run_shell(trimmed, &working_dir, COMMAND_TIMEOUT)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn tab_complete(input: &str, cwd: &str) -> Result<TabCompletionResult, String> {
    if input.is_empty() {
        return Ok(TabCompletionResult {
            completed_line: String::new(),
            suggestions: Vec::new(),
        });
    }

    let working_dir = if cwd.is_empty() || cwd == "~" {
        dirs_home()
    } else if cwd.starts_with('~') {
        dirs_home().join(cwd.trim_start_matches("~/").trim_start_matches('~'))
    } else {
        PathBuf::from(cwd)
    };

    let (prefix, token) = split_last_token(input);

    let is_first_word = prefix.trim().is_empty()
        || prefix.ends_with("| ")
        || prefix.ends_with("&& ")
        || prefix.ends_with("; ")
        || prefix.ends_with("|| ");

    // 1. First word command completion
    if is_first_word && !token.contains('/') && !token.starts_with('.') && !token.starts_with('~') {
        let token_lower = token.to_lowercase();
        let matches: Vec<String> = COMMON_COMMANDS
            .iter()
            .filter(|cmd| cmd.to_lowercase().starts_with(&token_lower))
            .map(|s| s.to_string())
            .collect();

        if matches.len() == 1 {
            return Ok(TabCompletionResult {
                completed_line: format!("{}{}{}", prefix, matches[0], " "),
                suggestions: Vec::new(),
            });
        } else if matches.len() > 1 {
            let lcp = longest_common_prefix(&matches);
            if lcp.len() > token.len() {
                return Ok(TabCompletionResult {
                    completed_line: format!("{}{}", prefix, lcp),
                    suggestions: Vec::new(),
                });
            } else {
                return Ok(TabCompletionResult {
                    completed_line: input.to_string(),
                    suggestions: matches,
                });
            }
        }
    }

    // 2. File / Directory Path Completion
    let unescaped_token = token.replace("\\ ", " ").replace("\\(", "(").replace("\\)", ")");
    let has_tilde = unescaped_token.starts_with('~');
    let resolved_token = if has_tilde {
        dirs_home().join(unescaped_token.trim_start_matches("~/").trim_start_matches('~'))
    } else {
        PathBuf::from(&unescaped_token)
    };

    let (parent_dir_path, partial_name, token_dir_prefix) = if unescaped_token.ends_with('/') {
        let parent = if has_tilde {
            resolved_token.clone()
        } else if unescaped_token.starts_with('/') {
            PathBuf::from(&unescaped_token)
        } else {
            working_dir.join(&unescaped_token)
        };
        (parent, String::new(), unescaped_token.clone())
    } else if let Some(last_slash_idx) = unescaped_token.rfind('/') {
        let dir_part = &unescaped_token[..=last_slash_idx];
        let name_part = &unescaped_token[last_slash_idx + 1..];
        let parent = if has_tilde {
            dirs_home().join(dir_part.trim_start_matches("~/").trim_start_matches('~'))
        } else if dir_part.starts_with('/') {
            PathBuf::from(dir_part)
        } else {
            working_dir.join(dir_part)
        };
        (parent, name_part.to_string(), dir_part.to_string())
    } else {
        (working_dir.clone(), unescaped_token.clone(), String::new())
    };

    if !parent_dir_path.exists() {
        return Ok(TabCompletionResult {
            completed_line: input.to_string(),
            suggestions: Vec::new(),
        });
    }

    let read_entries = fs::read_dir(&parent_dir_path).map_err(|e| e.to_string())?;
    let partial_lower = partial_name.to_lowercase();

    let mut matches: Vec<(String, bool)> = Vec::new();
    for entry in read_entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if partial_name.is_empty() {
            if !name.starts_with('.') {
                matches.push((name, is_dir));
            }
        } else if partial_name.starts_with('.') {
            if name.to_lowercase().starts_with(&partial_lower) {
                matches.push((name, is_dir));
            }
        } else {
            if !name.starts_with('.') && name.to_lowercase().starts_with(&partial_lower) {
                matches.push((name, is_dir));
            }
        }
    }

    matches.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    if matches.is_empty() {
        return Ok(TabCompletionResult {
            completed_line: input.to_string(),
            suggestions: Vec::new(),
        });
    }

    let match_names: Vec<String> = matches.iter().map(|m| m.0.clone()).collect();

    if matches.len() == 1 {
        let (matched_name, is_dir) = &matches[0];
        let suffix = if *is_dir { "/" } else { " " };
        let escaped_name = escape_shell_chars(matched_name);
        let completed = format!("{}{}{}{}", prefix, token_dir_prefix, escaped_name, suffix);

        Ok(TabCompletionResult {
            completed_line: completed,
            suggestions: Vec::new(),
        })
    } else {
        let lcp = longest_common_prefix(&match_names);
        if lcp.len() > partial_name.len() {
            let escaped_lcp = escape_shell_chars(&lcp);
            let completed = format!("{}{}{}", prefix, token_dir_prefix, escaped_lcp);
            Ok(TabCompletionResult {
                completed_line: completed,
                suggestions: Vec::new(),
            })
        } else {
            let suggestions: Vec<String> = matches
                .iter()
                .map(|(name, is_dir)| {
                    if *is_dir {
                        format!("{}/", name)
                    } else {
                        name.clone()
                    }
                })
                .collect();

            Ok(TabCompletionResult {
                completed_line: input.to_string(),
                suggestions,
            })
        }
    }
}

fn split_last_token(input: &str) -> (&str, &str) {
    let mut last_space = None;
    let mut is_escaped = false;

    for (idx, ch) in input.char_indices() {
        if ch == '\\' {
            is_escaped = !is_escaped;
        } else {
            if ch == ' ' && !is_escaped {
                last_space = Some(idx);
            }
            is_escaped = false;
        }
    }

    if let Some(space_idx) = last_space {
        (&input[..=space_idx], &input[space_idx + 1..])
    } else {
        ("", input)
    }
}

fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = &strings[0];
    let mut common_len = first.len();

    for s in &strings[1..] {
        let mut matching = 0;
        for (c1, c2) in first.chars().zip(s.chars()) {
            if c1.to_lowercase().to_string() == c2.to_lowercase().to_string() {
                matching += c1.len_utf8();
            } else {
                break;
            }
        }
        common_len = common_len.min(matching);
    }

    first[..common_len].to_string()
}

fn escape_shell_chars(str: &str) -> String {
    str.replace(' ', "\\ ")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('&', "\\&")
        .replace(';', "\\;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> PathBuf {
        std::env::temp_dir()
    }

    #[test]
    fn a_command_that_never_exits_is_killed() {
        let started = Instant::now();
        let out = run_shell("sleep 30", &cwd(), Duration::from_millis(300)).expect("run");
        assert!(started.elapsed() < Duration::from_secs(5), "should not wait for the child");
        assert!(out.stderr.contains("avbröts"), "stderr was: {}", out.stderr);
    }

    #[test]
    fn output_is_capped_without_blocking_the_child() {
        // Writes far more than the cap; the child must still be able to finish.
        let script = format!(
            "i=0; while [ $i -lt {} ]; do printf '%0.sx' $(seq 1 1000); i=$((i+1)); done; echo done",
            (MAX_OUTPUT_BYTES / 1000) + 500
        );
        let out = run_shell(&script, &cwd(), Duration::from_secs(60)).expect("run");
        assert_eq!(out.exit_code, 0, "the child should exit normally");
        assert!(out.stdout.contains("klippt"), "expected a truncation notice");
        assert!(out.stdout.len() < MAX_OUTPUT_BYTES + 1024);
    }

    #[test]
    fn stdin_is_closed_so_interactive_commands_do_not_hang() {
        let started = Instant::now();
        let out = run_shell("cat", &cwd(), Duration::from_secs(30)).expect("run");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.is_empty());
    }
}
