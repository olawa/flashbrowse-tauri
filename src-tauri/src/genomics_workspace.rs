//! Lightweight project-wide metadata for the genomics workspace.
//! Expensive read checks and index creation are explicit user actions.

use crate::bio_commands::find_tool_executable;
use crate::fs_commands::resolve_path;
use flate2::read::MultiGzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_BATCH: usize = 50;
const MAX_CONTIGS: usize = 1000;
const MAX_HEADER_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceContig {
    pub name: String,
    pub length: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomicsFileInfo {
    pub path: String,
    pub kind: String,
    pub samples: Vec<String>,
    pub sample_source: String,
    pub sample_count: usize,
    pub contigs: Vec<ReferenceContig>,
    pub contigs_truncated: bool,
    pub build: Option<String>,
    pub header_status: String,
    pub header_message: Option<String>,
    pub index_status: String,
    pub index_path: Option<String>,
    pub quickcheck_status: String,
    pub quickcheck_message: Option<String>,
}

fn file_kind(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.ends_with(".bam") || n.ends_with(".cram") || n.ends_with(".sam") {
        "alignment"
    } else if n.ends_with(".vcf") || n.ends_with(".vcf.gz") || n.ends_with(".bcf") {
        "variant"
    } else if n.ends_with(".fastq")
        || n.ends_with(".fq")
        || n.ends_with(".fastq.gz")
        || n.ends_with(".fq.gz")
    {
        "reads"
    } else if n.ends_with(".fa")
        || n.ends_with(".fasta")
        || n.ends_with(".fna")
        || n.ends_with(".fa.gz")
        || n.ends_with(".fasta.gz")
    {
        "reference"
    } else if n.ends_with(".bed") || n.ends_with(".bed.gz") {
        "regions"
    } else {
        "other"
    }
}

fn guessed_sample(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let mut stem = name;
    for suffix in [
        ".fastq.gz",
        ".fq.gz",
        ".fastq",
        ".fq",
        ".bam",
        ".cram",
        ".sam",
        ".vcf.gz",
        ".vcf",
        ".bcf",
    ] {
        if lower.ends_with(suffix) {
            stem = &stem[..stem.len() - suffix.len()];
            break;
        }
    }
    for suffix in [".sorted", ".dedup", ".recal", ".variants"] {
        if stem.to_ascii_lowercase().ends_with(suffix) {
            stem = &stem[..stem.len() - suffix.len()];
            break;
        }
    }
    for suffixes in [["_001", ""], ["_R1", "_R2"], ["_r1", "_r2"], ["_1", "_2"]] {
        for suffix in suffixes {
            if !suffix.is_empty() && stem.ends_with(suffix) {
                stem = &stem[..stem.len() - suffix.len()];
                break;
            }
        }
    }
    let bytes = stem.as_bytes();
    if bytes.len() >= 5
        && bytes[bytes.len() - 5..].starts_with(b"_L")
        && bytes[bytes.len() - 3..].iter().all(u8::is_ascii_digit)
    {
        stem = &stem[..stem.len() - 5];
    }
    if let Some(pos) = stem.rfind("_S") {
        if stem[pos + 2..].chars().all(|c| c.is_ascii_digit()) {
            stem = &stem[..pos];
        }
    }
    let stem = stem.trim_matches(['_', '.', '-']);
    if stem.is_empty() {
        None
    } else {
        Some(stem.to_owned())
    }
}

fn index_candidates(path: &Path, kind: &str) -> Vec<PathBuf> {
    let name = path.to_string_lossy();
    let lower = name.to_ascii_lowercase();
    match kind {
        "alignment" if lower.ends_with(".bam") => vec![
            PathBuf::from(format!("{name}.bai")),
            path.with_extension("bai"),
            PathBuf::from(format!("{name}.csi")),
            path.with_extension("csi"),
        ],
        "alignment" if lower.ends_with(".cram") => vec![
            PathBuf::from(format!("{name}.crai")),
            path.with_extension("crai"),
        ],
        "variant" if lower.ends_with(".vcf.gz") || lower.ends_with(".bcf") => vec![
            PathBuf::from(format!("{name}.tbi")),
            PathBuf::from(format!("{name}.csi")),
        ],
        "reference" => vec![PathBuf::from(format!("{name}.fai"))],
        _ => Vec::new(),
    }
}

fn index_health(path: &Path, kind: &str) -> (String, Option<String>) {
    let candidates = index_candidates(path, kind);
    if candidates.is_empty() {
        return ("not_applicable".into(), None);
    }
    let source_time = fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut found = Vec::new();
    for candidate in candidates {
        if let Ok(metadata) = fs::metadata(&candidate) {
            let fresh = match (source_time, metadata.modified().ok()) {
                (Some(source), Some(index)) => index >= source,
                _ => false,
            };
            found.push((fresh, candidate));
        }
    }
    if let Some((_, path)) = found.iter().find(|(fresh, _)| *fresh) {
        return ("present".into(), Some(path.to_string_lossy().into_owned()));
    }
    if let Some((_, path)) = found.first() {
        return ("stale".into(), Some(path.to_string_lossy().into_owned()));
    }
    ("missing".into(), None)
}

fn add_contig(info: &mut GenomicsFileInfo, name: &str, length: Option<u64>) {
    if name.is_empty() {
        return;
    }
    if info.contigs.len() >= MAX_CONTIGS {
        info.contigs_truncated = true;
        return;
    }
    if !info.contigs.iter().any(|c| c.name == name) {
        info.contigs.push(ReferenceContig {
            name: name.into(),
            length,
        });
    }
}

fn parse_alignment_header(info: &mut GenomicsFileInfo, header: &str) {
    let mut samples = BTreeSet::new();
    for line in header.lines() {
        if line.starts_with("@SQ\t") {
            let mut name = "";
            let mut length = None;
            for field in line.split('\t').skip(1) {
                if let Some(v) = field.strip_prefix("SN:") {
                    name = v;
                }
                if let Some(v) = field.strip_prefix("LN:") {
                    length = v.parse().ok();
                }
            }
            add_contig(info, name, length);
        } else if line.starts_with("@RG\t") {
            for field in line.split('\t').skip(1) {
                if let Some(sample) = field.strip_prefix("SM:") {
                    if !sample.trim().is_empty() {
                        samples.insert(sample.to_owned());
                    }
                }
            }
        }
    }
    if !samples.is_empty() {
        info.sample_count = samples.len();
        info.samples = samples.into_iter().take(100).collect();
        info.sample_source = "header".into();
    }
}

fn parse_vcf_header(info: &mut GenomicsFileInfo, header: &str) -> Result<(), String> {
    let mut found_columns = false;
    for line in header.lines() {
        if let Some(text) = line.strip_prefix("##contig=<") {
            let mut name = "";
            let mut length = None;
            for field in text.trim_end_matches('>').split(',') {
                if let Some(v) = field.strip_prefix("ID=") {
                    name = v.trim_matches('"');
                }
                if let Some(v) = field
                    .strip_prefix("length=")
                    .or_else(|| field.strip_prefix("Length="))
                {
                    length = v.parse().ok();
                }
            }
            add_contig(info, name, length);
        } else if line.starts_with("#CHROM\t") {
            let columns: Vec<&str> = line.split('\t').collect();
            info.sample_count = columns.len().saturating_sub(9);
            if info.sample_count > 0 {
                info.samples = columns
                    .iter()
                    .skip(9)
                    .take(100)
                    .map(|s| (*s).to_owned())
                    .collect();
                info.sample_source = "header".into();
            }
            found_columns = true;
            break;
        }
    }
    if !found_columns {
        return Err("VCF-headern saknar #CHROM-rad".into());
    }
    Ok(())
}

fn parse_fai(info: &mut GenomicsFileInfo, text: &str) {
    for line in text.lines() {
        let mut fields = line.split('\t');
        let name = fields.next().unwrap_or("");
        let length = fields.next().and_then(|v| v.parse().ok());
        add_contig(info, name, length);
    }
}

fn detect_build(contigs: &[ReferenceContig]) -> Option<String> {
    let chr1 = contigs.iter().find(|c| c.name == "chr1" || c.name == "1")?;
    match chr1.length? {
        248_956_422 => Some("GRCh38/hg38".into()),
        249_250_621 => Some("GRCh37/hg19".into()),
        248_387_328 => Some("T2T-CHM13".into()),
        195_471_971 => Some("mm10".into()),
        195_154_279 => Some("mm39".into()),
        _ => None,
    }
}

fn read_text_header(path: &Path, compressed: bool) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader: Box<dyn Read> = if compressed {
        Box::new(MultiGzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut text = String::new();
    let mut lines = BufReader::new(reader.take(MAX_HEADER_BYTES)).lines();
    while let Some(line) = lines.next() {
        let line = line.map_err(|e| e.to_string())?;
        text.push_str(&line);
        text.push('\n');
        if line.starts_with("#CHROM\t") {
            return Ok(text);
        }
    }
    Err("VCF-headern är ofullständig eller större än 8 MB".into())
}

fn command_header(bin: &str, args: &[&str], path: &Path) -> Result<String, String> {
    let tool = find_tool_executable(bin).ok_or_else(|| format!("{bin} finns inte installerat"))?;
    let output = Command::new(tool)
        .args(args)
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

fn inspect_file(path: &Path, quickcheck: bool) -> GenomicsFileInfo {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let kind = file_kind(&name).to_string();
    let (index_status, index_path) = index_health(path, &kind);
    let guessed = if matches!(kind.as_str(), "alignment" | "variant" | "reads") {
        guessed_sample(&name)
    } else {
        None
    };
    let mut info = GenomicsFileInfo {
        path: path.to_string_lossy().into_owned(),
        kind: kind.clone(),
        samples: guessed.iter().cloned().collect(),
        sample_source: if guessed.is_some() {
            "filename"
        } else {
            "unknown"
        }
        .into(),
        sample_count: usize::from(guessed.is_some()),
        contigs: Vec::new(),
        contigs_truncated: false,
        build: None,
        header_status: "not_applicable".into(),
        header_message: None,
        index_status,
        index_path,
        quickcheck_status: "not_run".into(),
        quickcheck_message: None,
    };
    if !path.is_file() {
        info.header_status = "error".into();
        info.header_message = Some("Filen finns inte längre".into());
        return info;
    }
    let lower = name.to_ascii_lowercase();
    let header_result = match kind.as_str() {
        "alignment" => command_header("samtools", &["view", "-H"], path)
            .map(|h| parse_alignment_header(&mut info, &h)),
        "variant" if lower.ends_with(".bcf") => command_header("bcftools", &["view", "-h"], path)
            .and_then(|h| parse_vcf_header(&mut info, &h)),
        "variant" => read_text_header(path, lower.ends_with(".gz"))
            .and_then(|h| parse_vcf_header(&mut info, &h)),
        "reference" if info.index_status == "stale" => {
            Err("FASTA-index är äldre än referensfilen".into())
        }
        "reference" => match &info.index_path {
            Some(fai) => fs::read_to_string(fai)
                .map_err(|e| e.to_string())
                .map(|text| parse_fai(&mut info, &text)),
            None => Err("FASTA-index (.fai) saknas".into()),
        },
        "regions" => {
            let result = (|| -> Result<(), String> {
                let file = File::open(path).map_err(|e| e.to_string())?;
                let reader: Box<dyn Read> = if lower.ends_with(".gz") {
                    Box::new(MultiGzDecoder::new(file))
                } else {
                    Box::new(file)
                };
                for (line_number, line) in
                    BufReader::new(reader.take(1024 * 1024)).lines().enumerate()
                {
                    if line_number >= 1000 {
                        info.contigs_truncated = true;
                        break;
                    }
                    let line = line.map_err(|e| e.to_string())?;
                    if line.starts_with('#')
                        || line.starts_with("track ")
                        || line.starts_with("browser ")
                    {
                        continue;
                    }
                    add_contig(&mut info, line.split('\t').next().unwrap_or(""), None);
                }
                Ok(())
            })();
            result
        }
        _ => Ok(()),
    };
    if kind != "reads" && kind != "other" {
        match header_result {
            Ok(()) => info.header_status = "read".into(),
            Err(message) => {
                info.header_status = if message.contains("finns inte installerat") {
                    "unavailable"
                } else {
                    "error"
                }
                .into();
                info.header_message = Some(message);
            }
        }
    }
    info.build = detect_build(&info.contigs);
    if quickcheck && kind == "alignment" {
        match find_tool_executable("samtools") {
            None => {
                info.quickcheck_status = "unavailable".into();
                info.quickcheck_message = Some("samtools finns inte installerat".into());
            }
            Some(tool) => match Command::new(tool)
                .args(["quickcheck", "-v"])
                .arg(path)
                .output()
            {
                Ok(out) if out.status.success() => info.quickcheck_status = "passed".into(),
                Ok(out) => {
                    info.quickcheck_status = "failed".into();
                    info.quickcheck_message = Some(
                        format!(
                            "{} {}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        )
                        .trim()
                        .into(),
                    );
                }
                Err(e) => {
                    info.quickcheck_status = "unavailable".into();
                    info.quickcheck_message = Some(e.to_string());
                }
            },
        }
    }
    info
}

#[tauri::command]
pub async fn inspect_genomics_files(
    paths: Vec<String>,
    quickcheck: bool,
) -> Result<Vec<GenomicsFileInfo>, String> {
    if paths.len() > MAX_BATCH {
        return Err(format!("Högst {MAX_BATCH} filer per anrop"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        Ok(paths
            .into_iter()
            .map(|raw| {
                if raw.starts_with("ssh://") {
                    return Err("Fjärrfiler stöds inte i projektanalysen".to_string());
                }
                let path = resolve_path(&raw);
                Ok(inspect_file(&path, quickcheck))
            })
            .collect::<Result<Vec<_>, String>>()?)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_genomics_index(path: String) -> Result<GenomicsFileInfo, String> {
    if path.starts_with("ssh://") {
        return Err("Fjärrfiler stöds inte här".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let path = resolve_path(&path);
        if !path.is_file() {
            return Err("Filen finns inte".to_string());
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        let (tool, subcommand) = if name.ends_with(".bam") || name.ends_with(".cram") {
            ("samtools", "index")
        } else if name.ends_with(".vcf.gz") || name.ends_with(".bcf") {
            ("bcftools", "index")
        } else if file_kind(&name) == "reference" {
            ("samtools", "faidx")
        } else {
            return Err("Indexering stöds inte för den filtypen".to_string());
        };
        let executable =
            find_tool_executable(tool).ok_or_else(|| format!("{tool} finns inte installerat"))?;
        let mut command = Command::new(executable);
        command.arg(subcommand);
        if tool == "bcftools" {
            command.arg("-f");
        }
        let output = command.arg(&path).output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Indexering misslyckades: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(inspect_file(&path, false))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> GenomicsFileInfo {
        inspect_file(Path::new("/missing/sample_R1.fastq.gz"), false)
    }

    #[test]
    fn sample_guess_removes_lane_read_and_index_suffixes() {
        assert_eq!(
            guessed_sample("Patient-A_S3_L002_R1_001.fastq.gz"),
            Some("Patient-A".into())
        );
        assert_eq!(guessed_sample("Case.sorted.bam"), Some("Case".into()));
        assert_eq!(guessed_sample("Case.vcf.gz"), Some("Case".into()));
        assert_eq!(
            guessed_sample("Ärende_S3_L002_R1_001.fastq.gz"),
            Some("Ärende".into())
        );
    }

    #[test]
    fn metadata_from_headers_keeps_sample_source_explicit() {
        let mut bam = info();
        parse_alignment_header(
            &mut bam,
            "@SQ\tSN:chr1\tLN:248956422\n@RG\tID:a\tSM:Patient-A\n",
        );
        assert_eq!(bam.samples, ["Patient-A"]);
        assert_eq!(bam.sample_source, "header");
        assert_eq!(detect_build(&bam.contigs), Some("GRCh38/hg38".into()));
        let mut vcf = info();
        parse_vcf_header(&mut vcf, "##contig=<ID=chr1,length=248956422>\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tPatient-A\n").unwrap();
        assert_eq!(vcf.samples, ["Patient-A"]);
        assert_eq!(vcf.sample_count, 1);
        assert_eq!(vcf.contigs[0].length, Some(248_956_422));
    }

    #[test]
    fn index_health_distinguishes_missing_stale_and_present() {
        let temp = tempfile::tempdir().unwrap();
        let bam = temp.path().join("reads.bam");
        fs::write(&bam, "data").unwrap();
        assert_eq!(index_health(&bam, "alignment").0, "missing");
        let bai = temp.path().join("reads.bam.bai");
        fs::write(&bai, "index").unwrap();
        assert_eq!(index_health(&bam, "alignment").0, "present");
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&bam, "new data").unwrap();
        assert_eq!(index_health(&bam, "alignment").0, "stale");
    }

    #[test]
    fn incomplete_vcf_never_looks_like_verified_metadata() {
        let mut vcf = info();
        assert!(parse_vcf_header(&mut vcf, "##contig=<ID=1,length=1000>\n").is_err());
    }

    #[test]
    fn plain_vcf_header_provides_sample_and_contig_without_scanning_variants() {
        let temp = tempfile::tempdir().unwrap();
        let vcf = temp.path().join("cohort.vcf");
        fs::write(&vcf, "##fileformat=VCFv4.2\n##contig=<ID=chr1,length=248956422>\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tPatient-A\nchr1\t1\t.\tA\tG\t.\tPASS\t.\tGT\t0/1\n").unwrap();
        let result = inspect_file(&vcf, false);
        assert_eq!(result.header_status, "read");
        assert_eq!(result.sample_source, "header");
        assert_eq!(result.samples, ["Patient-A"]);
        assert_eq!(result.build, Some("GRCh38/hg38".into()));
    }

    #[test]
    fn indexed_bam_can_be_checked_and_indexed_on_demand() {
        let Some(samtools) = find_tool_executable("samtools") else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let sam = temp.path().join("sample.sam");
        let bam = temp.path().join("sample.bam");
        fs::write(
            &sam,
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:248956422\n@RG\tID:lane1\tSM:Patient-A\n",
        )
        .unwrap();
        let output = Command::new(samtools)
            .args(["view", "-b", "-o"])
            .arg(&bam)
            .arg(&sam)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let before = inspect_file(&bam, true);
        assert_eq!(before.index_status, "missing");
        assert_eq!(before.samples, ["Patient-A"]);
        assert_eq!(before.quickcheck_status, "passed");
        let after = tauri::async_runtime::block_on(create_genomics_index(
            bam.to_string_lossy().into_owned(),
        ))
        .unwrap();
        assert_eq!(after.index_status, "present");
        assert!(after.index_path.unwrap().ends_with(".bai"));
        File::open(&bam)
            .unwrap()
            .set_modified(std::time::SystemTime::now())
            .unwrap();
        let bai = PathBuf::from(format!("{}.bai", bam.display()));
        File::open(&bai)
            .unwrap()
            .set_modified(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
        assert_eq!(index_health(&bam, "alignment").0, "stale");
        let rebuilt = tauri::async_runtime::block_on(create_genomics_index(
            bam.to_string_lossy().into_owned(),
        ))
        .unwrap();
        assert_eq!(rebuilt.index_status, "present");
    }
}
