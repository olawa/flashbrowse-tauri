//! Running rs-qc, and finding the results of earlier runs.
//!
//! rs-qc writes its output as a set of files sharing one prefix:
//! `<prefix>.<module>.summary.json` alongside tables, plots and a text report.
//! Writing that prefix next to the data is what makes a QC run permanent and
//! attached to the file it describes - there is no database, the results sit
//! beside the BAM and travel with it.
//!
//! Which module to run is not a question the user should have to answer: an
//! RNA alignment needs `rna` with an annotation, a DNA alignment needs `dna`,
//! and reads need `fastq`. The read type already comes out of the header
//! (see [`crate::provenance_commands`]), so it picks the module.

use crate::bio_commands::{find_tool_executable, writable_output_dir};
use crate::fs_commands::resolve_path;
use crate::provenance_commands::{classify_header, ReadType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// The rs-qc subcommands this app drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QcModule {
    /// General alignment QC: mapping rates, MAPQ, insert size, clipping.
    Align,
    /// Depth and breadth of coverage.
    Dna,
    /// Gene-body coverage, 3' bias, read distribution. Needs an annotation.
    Rna,
    /// Quality, GC, adapters, duplication, k-mers - for reads, not alignments.
    Fastq,
}

impl QcModule {
    pub fn subcommand(&self) -> &'static str {
        match self {
            QcModule::Align => "align",
            QcModule::Dna => "dna",
            QcModule::Rna => "rna",
            QcModule::Fastq => "fastq",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "align" => Some(QcModule::Align),
            "dna" => Some(QcModule::Dna),
            "rna" => Some(QcModule::Rna),
            "fastq" => Some(QcModule::Fastq),
            _ => None,
        }
    }
}

/// Every module whose results may sit next to a file.
const MODULES: [&str; 6] = ["align", "dna", "rna", "fastq", "atac", "contam"];

fn is_fastq_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let stripped = lower
        .strip_suffix(".gz")
        .or_else(|| lower.strip_suffix(".bz2"))
        .unwrap_or(&lower);
    stripped.ends_with(".fastq") || stripped.ends_with(".fq")
}

fn is_alignment_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".bam") || lower.ends_with(".cram")
}

/// The prefix rs-qc outputs use for `path`: the file name without its
/// extension, in a directory we can write to.
fn output_prefix(path: &Path) -> (PathBuf, String) {
    let dir = writable_output_dir(path.parent());
    let stem = path
        .file_name()
        .map(|n| {
            let n = n.to_string_lossy();
            // Trim only the last extension: "sample.bam" -> "sample", but
            // "reads.fastq.gz" -> "reads.fastq", which is how rs-qc names
            // things too.
            match n.rsplit_once('.') {
                Some((head, _)) if !head.is_empty() => head.to_string(),
                _ => n.to_string(),
            }
        })
        .unwrap_or_else(|| "sample".to_string());
    (dir, stem)
}

/// Results of an earlier rs-qc run, found on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcResult {
    pub module: String,
    pub summary_json: String,
    pub summary_text: Option<String>,
    pub report_html: Option<String>,
    pub plots: Vec<String>,
    pub tables: Vec<String>,
    /// When the run happened, from the summary file's timestamp.
    pub generated: String,
    /// A few headline numbers, so the UI can show something without parsing.
    pub headline: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcStatus {
    pub path: String,
    pub name: String,
    /// Which module would run for this file, given what it is.
    pub suggested_module: String,
    pub results: Vec<QcResult>,
}

/// Pull a handful of numbers out of a summary for the file list.
fn headline_metrics(json: &serde_json::Value, module: &str) -> Vec<(String, String)> {
    let m = &json["metrics"];
    let num = |key: &str| m.get(key).and_then(|v| v.as_f64());
    let mut out = Vec::new();

    let percent = |part: Option<f64>, whole: Option<f64>| match (part, whole) {
        (Some(p), Some(w)) if w > 0.0 => Some(format!("{:.1}%", 100.0 * p / w)),
        _ => None,
    };

    match module {
        "align" => {
            if let Some(v) = num("total_records") {
                out.push(("Reads".into(), format!("{}", v as u64)));
            }
            if let Some(p) = percent(num("mapped_records"), num("total_records")) {
                out.push(("Mappade".into(), p));
            }
            if let Some(p) = percent(num("duplicate_records"), num("total_records")) {
                out.push(("Dubbletter".into(), p));
            }
        }
        "dna" => {
            for (key, label) in [("mean_depth", "Medeldjup"), ("callable_fraction", "Callable")] {
                if let Some(v) = num(key) {
                    out.push((label.into(), format!("{v:.2}")));
                }
            }
        }
        "rna" => {
            if let Some(v) = num("total_reads") {
                out.push(("Reads".into(), format!("{}", v as u64)));
            }
            if let Some(v) = num("active_genes") {
                out.push(("Gener".into(), format!("{}", v as u64)));
            }
            if let Some(v) = num("mtdna_fraction") {
                out.push(("mtDNA".into(), format!("{:.1}%", v * 100.0)));
            }
        }
        _ => {}
    }

    // Fall back to whatever the module put first, rather than showing nothing.
    if out.is_empty() {
        if let Some(obj) = m.as_object() {
            for (k, v) in obj.iter().take(3) {
                if let Some(n) = v.as_f64() {
                    out.push((k.clone(), format!("{n}")));
                }
            }
        }
    }
    out
}

fn file_time(path: &Path) -> String {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

/// Find what an earlier rs-qc run left next to `path`.
fn results_for(path: &Path) -> Vec<QcResult> {
    let (dir, stem) = output_prefix(path);
    let mut results = Vec::new();

    for module in MODULES {
        let summary = dir.join(format!("{stem}.{module}.summary.json"));
        if !summary.is_file() {
            continue;
        }

        let parsed: serde_json::Value = std::fs::read_to_string(&summary)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(serde_json::Value::Null);

        let optional = |candidate: PathBuf| candidate.is_file().then(|| candidate.to_string_lossy().to_string());

        // Everything sharing the prefix belongs to this run.
        let mut plots = Vec::new();
        let mut tables = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with(&format!("{stem}.")) {
                    continue;
                }
                let full = entry.path().to_string_lossy().to_string();
                if name.ends_with(".svg") {
                    plots.push(full);
                } else if name.ends_with(".tsv") {
                    tables.push(full);
                }
            }
        }
        plots.sort();
        tables.sort();

        results.push(QcResult {
            headline: headline_metrics(&parsed, module),
            generated: file_time(&summary),
            summary_text: optional(dir.join(format!("{stem}.{module}.summary.txt"))),
            report_html: optional(dir.join(format!("{stem}.report.html")))
                .or_else(|| optional(dir.join(format!("{stem}.{module}.report.html")))),
            plots,
            tables,
            module: module.to_string(),
            summary_json: summary.to_string_lossy().to_string(),
        });
    }

    results
}

/// Which module fits this file, from its type and - for alignments - its header.
///
/// None means rs-qc has nothing to say about this file. Answering "dna" for a
/// text file would put a QC button on something that can only fail.
fn suggested_module(path: &str) -> Option<QcModule> {
    if is_fastq_path(path) {
        return Some(QcModule::Fastq);
    }
    if !is_alignment_path(path) {
        return None;
    }

    let samtools = find_tool_executable("samtools");
    if let Some(samtools) = samtools {
        if let Ok(out) = Command::new(samtools).arg("view").arg("-H").arg(path).output() {
            if out.status.success() {
                let header = String::from_utf8_lossy(&out.stdout);
                if classify_header(path, &header).read_type == ReadType::Rna {
                    return Some(QcModule::Rna);
                }
            }
        }
    }
    // Coverage QC is the useful default for a DNA alignment; `align` adds
    // nothing it does not already cover.
    Some(QcModule::Dna)
}

/// Report what QC exists for these files, and what would run for them.
#[tauri::command]
pub async fn qc_status(paths: Vec<String>) -> Result<Vec<QcStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(|p| {
                let resolved = resolve_path(p);
                let path = resolved.to_string_lossy().to_string();
                QcStatus {
                    name: resolved
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    suggested_module: suggested_module(&path)
                        .map(|m| m.subcommand().to_string())
                        .unwrap_or_default(),
                    results: results_for(&resolved),
                    path,
                }
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Progress of a batch QC run, emitted as `qc-progress`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcProgress {
    pub id: String,
    pub current_file: String,
    pub module: String,
    pub files_done: usize,
    pub files_total: usize,
    pub done: bool,
    pub cancelled: bool,
    /// Per-file failures. One bad file does not stop the batch.
    pub failures: Vec<String>,
}

static RUNNING_QC: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);

fn qc_lock() -> std::sync::MutexGuard<'static, Option<HashMap<String, u32>>> {
    RUNNING_QC.lock().unwrap_or_else(|p| p.into_inner())
}

fn qc_cancelled(id: &str) -> bool {
    qc_lock().as_ref().map(|m| !m.contains_key(id)).unwrap_or(true)
}

/// Run rs-qc over a set of files, one at a time.
///
/// rs-qc parallelises internally, so it is given every core and the files are
/// walked in sequence: running several at once with a share of the cores each
/// only adds contention, and makes the progress report meaningless.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn run_qc_batch(
    app: AppHandle,
    id: String,
    paths: Vec<String>,
    threads: Option<usize>,
    module_override: Option<String>,
    annotation: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let rsqc = find_tool_executable("rs-qc")
            .ok_or_else(|| "rs-qc hittades inte i dev/bin eller PATH".to_string())?;
        if paths.is_empty() {
            return Ok("Inga filer valda".to_string());
        }

        let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8));
        let forced = module_override.as_deref().and_then(QcModule::parse);

        let mut progress = QcProgress {
            id: id.clone(),
            current_file: String::new(),
            module: String::new(),
            files_done: 0,
            files_total: paths.len(),
            done: false,
            cancelled: false,
            failures: Vec::new(),
        };
        let emit = |p: &QcProgress| {
            let _ = app.emit("qc-progress", p);
        };

        qc_lock().get_or_insert_with(HashMap::new).insert(id.clone(), 0);
        emit(&progress);

        for (index, raw) in paths.iter().enumerate() {
            if qc_cancelled(&id) {
                break;
            }

            let path = resolve_path(raw);
            let path_str = path.to_string_lossy().to_string();
            let Some(module) = forced.or_else(|| suggested_module(&path_str)) else {
                progress.failures.push(format!(
                    "{}: rs-qc läser BAM, CRAM och FASTQ - inte den här filtypen",
                    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
                ));
                continue;
            };
            let (dir, stem) = output_prefix(&path);

            progress.files_done = index;
            progress.current_file = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            progress.module = module.subcommand().to_string();
            emit(&progress);

            let mut cmd = Command::new(&rsqc);
            cmd.arg(module.subcommand())
                .arg("-i")
                .arg(&path)
                .arg("-o")
                .arg(dir.join(&stem))
                .current_dir(&dir);

            // Only the modules that take them; rs-qc rejects unknown flags.
            if matches!(module, QcModule::Align | QcModule::Dna | QcModule::Fastq) {
                cmd.arg("-t").arg(threads.to_string());
            }
            if module == QcModule::Rna {
                let Some(annotation) = annotation.as_deref() else {
                    progress.failures.push(format!(
                        "{}: RNA-QC kräver en annoteringsfil (GTF) - ingen är konfigurerad för genomet",
                        progress.current_file
                    ));
                    continue;
                };
                cmd.arg("-a").arg(annotation);
            }
            // Paired reads are QC:d together, which is what --paired means.
            if module == QcModule::Fastq {
                if let Some(mate) = crate::companion_commands::mate_path(&path_str) {
                    if Path::new(&mate).is_file() {
                        cmd.arg("-i").arg(&mate).arg("--paired");
                    }
                }
            }

            match cmd.output() {
                Ok(out) if out.status.success() => {}
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    progress
                        .failures
                        .push(format!("{}: {}", progress.current_file, err.trim()));
                }
                Err(e) => progress
                    .failures
                    .push(format!("{}: kunde inte starta rs-qc: {e}", progress.current_file)),
            }
        }

        let cancelled = qc_cancelled(&id);
        if let Some(map) = qc_lock().as_mut() {
            map.remove(&id);
        }

        progress.done = true;
        progress.cancelled = cancelled;
        progress.files_done = if cancelled { progress.files_done } else { paths.len() };
        emit(&progress);

        let failures = progress.failures.len();
        Ok(match (cancelled, failures) {
            (true, _) => "QC avbruten".to_string(),
            (false, 0) => format!("QC klar för {} filer", paths.len()),
            (false, n) => format!("QC klar för {} filer, {n} misslyckades", paths.len() - n),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Stop a batch after the file it is on.
#[tauri::command]
pub fn cancel_qc(id: String) -> Result<bool, String> {
    Ok(qc_lock().as_mut().map(|m| m.remove(&id).is_some()).unwrap_or(false))
}

/// Combine summaries from several runs into one report, the way a batch is
/// usually read: per sample is fine, across samples is what shows an outlier.
#[tauri::command]
pub async fn build_qc_report(summaries: Vec<String>, output_prefix: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let rsqc = find_tool_executable("rs-qc")
            .ok_or_else(|| "rs-qc hittades inte i dev/bin eller PATH".to_string())?;
        if summaries.is_empty() {
            return Err("Inga QC-resultat att sammanställa".to_string());
        }

        let out_path = resolve_path(&output_prefix);
        let dir = writable_output_dir(out_path.parent());
        let prefix = dir.join(
            out_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "rs-qc".to_string()),
        );

        let mut cmd = Command::new(rsqc);
        cmd.arg("report");
        for summary in &summaries {
            cmd.arg("-i").arg(resolve_path(summary));
        }
        cmd.arg("-o").arg(&prefix).current_dir(&dir);

        let out = cmd
            .output()
            .map_err(|e| format!("Kunde inte starta rs-qc report: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "rs-qc report misslyckades: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }

        let html = format!("{}.report.html", prefix.to_string_lossy());
        if Path::new(&html).is_file() {
            Ok(html)
        } else {
            Err("rs-qc report skrev ingen rapport".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prefix_drops_one_extension() {
        let (_, stem) = output_prefix(Path::new("/data/sample.bam"));
        assert_eq!(stem, "sample");

        // rs-qc names FASTQ output after everything but the compression suffix.
        let (_, stem) = output_prefix(Path::new("/data/reads.fastq.gz"));
        assert_eq!(stem, "reads.fastq");
    }

    #[test]
    fn reads_get_the_fastq_module_without_reading_a_header() {
        assert_eq!(suggested_module("/data/reads_R1.fastq.gz"), Some(QcModule::Fastq));
        assert_eq!(suggested_module("/data/reads.fq"), Some(QcModule::Fastq));
    }

    #[test]
    fn a_file_rs_qc_cannot_read_gets_no_module() {
        // Without this, every text file in a listing offered a QC run that
        // could only fail.
        assert_eq!(suggested_module("/data/notes.txt"), None);
        assert_eq!(suggested_module("/data/report.pdf"), None);
    }

    #[test]
    fn headline_metrics_turn_counts_into_percentages() {
        let json = serde_json::json!({
            "module": "align",
            "metrics": { "total_records": 1000, "mapped_records": 950, "duplicate_records": 100 }
        });
        let headline = headline_metrics(&json, "align");
        assert_eq!(headline[0], ("Reads".to_string(), "1000".to_string()));
        assert_eq!(headline[1], ("Mappade".to_string(), "95.0%".to_string()));
        assert_eq!(headline[2], ("Dubbletter".to_string(), "10.0%".to_string()));
    }

    #[test]
    fn a_summary_next_to_the_file_is_found_and_read() {
        let dir = std::env::temp_dir().join(format!("fb_qc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let bam = dir.join("sample.bam");
        std::fs::write(&bam, b"bam").unwrap();
        std::fs::write(
            dir.join("sample.align.summary.json"),
            r#"{"module":"align","metrics":{"total_records":10,"mapped_records":9}}"#,
        )
        .unwrap();
        std::fs::write(dir.join("sample.align.summary.txt"), b"report").unwrap();
        std::fs::write(dir.join("sample.align.mapq.tsv"), b"table").unwrap();
        std::fs::write(dir.join("sample.align.qc.svg"), b"<svg/>").unwrap();
        // A different sample's results must not be attributed to this one.
        std::fs::write(dir.join("other.align.summary.json"), b"{}").unwrap();

        let results = results_for(&bam);
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.module, "align");
        assert!(r.summary_text.is_some());
        assert_eq!(r.tables.len(), 1);
        assert_eq!(r.plots.len(), 1);
        assert_eq!(r.headline[0].1, "10");
        assert!(!r.generated.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_file_without_results_reports_none() {
        let dir = std::env::temp_dir().join(format!("fb_qc_none_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let bam = dir.join("fresh.bam");
        std::fs::write(&bam, b"bam").unwrap();

        assert!(results_for(&bam).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
