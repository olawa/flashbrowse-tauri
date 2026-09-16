//! Companion files: the sidecars and mates that must travel with a file.
//!
//! Moving a BAM without its .bai leaves something that still opens in a file
//! manager but is broken for every tool downstream - IGV just says it cannot
//! load it. The same goes for a VCF without its .tbi, a reference without its
//! .fai, and half of a paired FASTQ set. This module finds those relatives so
//! transfers can offer to bring them along.

use crate::fs_commands::{format_byte_size, resolve_path};
use crate::ssh_commands::{sh_quote, ssh_base_args};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// What kind of relative a companion is, so the UI can explain itself.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompanionKind {
    /// An index the primary file is useless without (.bai, .tbi, .fai …).
    Index,
    /// A checksum written next to the data (.md5, .sha256).
    Checksum,
    /// The other half of a pair, e.g. R2 for R1.
    Mate,
    /// QC output describing this file, written next to it by rs-qc.
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Companion {
    pub path: String,
    pub name: String,
    pub kind: CompanionKind,
    pub formatted_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionSet {
    /// The selected file these companions belong to.
    pub primary: String,
    pub companions: Vec<Companion>,
}

/// Candidate companion paths for `path`, before checking what exists.
///
/// Both naming conventions are covered: samtools writes `sample.bam.bai`, while
/// Picard and older pipelines write `sample.bai`.
fn candidates(path: &str) -> Vec<(String, CompanionKind)> {
    let mut out: Vec<(String, CompanionKind)> = Vec::new();

    // Checksums sit next to anything.
    for ext in ["md5", "sha256", "sha1"] {
        out.push((format!("{}.{}", path, ext), CompanionKind::Checksum));
    }

    // Index files, by data type.
    if let Some(stem) = strip_ext_ci(path, ".bam") {
        out.push((format!("{}.bai", path), CompanionKind::Index));
        out.push((format!("{}.csi", path), CompanionKind::Index));
        // Picard and older pipelines drop the .bam instead of appending.
        out.push((format!("{}.bai", stem), CompanionKind::Index));
    } else if let Some(stem) = strip_ext_ci(path, ".cram") {
        out.push((format!("{}.crai", path), CompanionKind::Index));
        out.push((format!("{}.crai", stem), CompanionKind::Index));
    } else if strip_ext_ci(path, ".gz").is_some()
        || strip_ext_ci(path, ".bgz").is_some()
        || strip_ext_ci(path, ".bcf").is_some()
    {
        // Tabix-indexed: vcf.gz, bed.gz, gff.gz, gtf.gz, bcf …
        out.push((format!("{}.tbi", path), CompanionKind::Index));
        out.push((format!("{}.csi", path), CompanionKind::Index));
        out.push((format!("{}.gzi", path), CompanionKind::Index));
    }

    for fasta in [".fa", ".fasta", ".fa.gz", ".fasta.gz", ".fna"] {
        if let Some(stem) = strip_ext_ci(path, fasta) {
            out.push((format!("{}.fai", path), CompanionKind::Index));
            out.push((format!("{}.gzi", path), CompanionKind::Index));
            // Sequence dictionaries replace the extension instead of appending.
            out.push((format!("{}.dict", stem), CompanionKind::Index));
            break;
        }
    }

    if let Some(mate) = mate_path(path) {
        out.push((mate, CompanionKind::Mate));
    }

    // QC results belong with the data they describe: a BAM that arrives
    // without its QC summary has to be re-run to be judged.
    if let Some(stem) = [".bam", ".cram", ".sam"]
        .iter()
        .find_map(|ext| strip_ext_ci(path, ext))
    {
        for module in ["align", "dna", "rna", "atac", "contam"] {
            out.push((format!("{stem}.{module}.summary.json"), CompanionKind::Report));
            out.push((format!("{stem}.{module}.summary.txt"), CompanionKind::Report));
        }
        out.push((format!("{stem}.report.html"), CompanionKind::Report));
    }

    out
}

/// Strip `ext` from the end of `path`, ignoring case.
///
/// Returns None when the path does not end with it - including when the cut
/// would land inside a multi-byte character, which plain slicing would panic on.
fn strip_ext_ci<'a>(path: &'a str, ext: &str) -> Option<&'a str> {
    let split = path.len().checked_sub(ext.len())?;
    let head = path.get(..split)?;
    let tail = path.get(split..)?;
    tail.eq_ignore_ascii_case(ext).then_some(head)
}

/// The other half of a paired-end FASTQ, if this looks like one.
///
/// Covers the Illumina form (`_R1_001.fastq.gz`) and the short form used by
/// SRA and most aligners (`_1.fq.gz`).
pub fn mate_path(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_string_lossy().to_string();
    let parent = Path::new(path).parent()?.to_string_lossy().to_string();

    let swapped = if name.contains("_R1") {
        name.replacen("_R1", "_R2", 1)
    } else if name.contains("_R2") {
        name.replacen("_R2", "_R1", 1)
    } else if let Some(pos) = name.find("_1.") {
        format!("{}_2.{}", &name[..pos], &name[pos + 3..])
    } else if let Some(pos) = name.find("_2.") {
        format!("{}_1.{}", &name[..pos], &name[pos + 3..])
    } else {
        return None;
    };

    if swapped == name {
        return None;
    }
    Some(if parent.is_empty() {
        swapped
    } else {
        format!("{}/{}", parent, swapped)
    })
}

/// Find companions for local files.
fn local_companions(paths: &[String]) -> Vec<CompanionSet> {
    let selected: std::collections::HashSet<String> = paths.iter().cloned().collect();

    paths
        .iter()
        .map(|p| {
            let resolved = resolve_path(p).to_string_lossy().to_string();
            let companions = candidates(&resolved)
                .into_iter()
                // Something the user already selected is not a companion to add.
                .filter(|(candidate, _)| !selected.contains(candidate))
                .filter_map(|(candidate, kind)| {
                    let meta = std::fs::metadata(&candidate).ok()?;
                    Some(Companion {
                        name: Path::new(&candidate)
                            .file_name()?
                            .to_string_lossy()
                            .to_string(),
                        path: candidate,
                        kind,
                        formatted_size: format_byte_size(meta.len()),
                    })
                })
                .collect::<Vec<_>>();

            CompanionSet {
                primary: p.clone(),
                companions,
            }
        })
        .collect()
}

/// Find companions on a remote host with a single ssh round trip.
///
/// Prints `size<TAB>path` for each candidate that exists; anything else is
/// silently skipped, so a missing file costs nothing.
fn remote_companions(host: &str, paths: &[String]) -> Result<Vec<CompanionSet>, String> {
    let selected: std::collections::HashSet<String> = paths.iter().cloned().collect();

    let mut all_candidates: Vec<(String, String, CompanionKind)> = Vec::new();
    for p in paths {
        for (candidate, kind) in candidates(p) {
            if !selected.contains(&candidate) {
                all_candidates.push((p.clone(), candidate, kind));
            }
        }
    }
    if all_candidates.is_empty() {
        return Ok(paths
            .iter()
            .map(|p| CompanionSet { primary: p.clone(), companions: Vec::new() })
            .collect());
    }

    let checks: Vec<String> = all_candidates
        .iter()
        .map(|(_, candidate, _)| {
            let q = sh_quote(candidate);
            format!(
                "[ -f {q} ] && printf '%s\\t%s\\n' \"$(wc -c < {q} | tr -d ' ')\" {q}"
            )
        })
        .collect();

    let mut args = ssh_base_args();
    args.push(host.to_string());
    args.push(format!("{}; true", checks.join("; ")));

    let out = Command::new("ssh")
        .args(&args)
        .output()
        .map_err(|e| format!("Kunde inte söka följeslagarfiler: {}", e))?;

    let mut found: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some((size, path)) = line.split_once('\t') {
            found.insert(path.trim().to_string(), size.trim().parse().unwrap_or(0));
        }
    }

    Ok(paths
        .iter()
        .map(|p| CompanionSet {
            primary: p.clone(),
            companions: all_candidates
                .iter()
                .filter(|(primary, _, _)| primary == p)
                .filter_map(|(_, candidate, kind)| {
                    let size = found.get(candidate)?;
                    Some(Companion {
                        name: Path::new(candidate)
                            .file_name()?
                            .to_string_lossy()
                            .to_string(),
                        path: candidate.clone(),
                        kind: *kind,
                        formatted_size: format_byte_size(*size),
                    })
                })
                .collect(),
        })
        .collect())
}

/// Index files, checksums and pair mates that belong with `paths`.
///
/// Returns one entry per input path, with an empty list when nothing was found.
#[tauri::command]
pub async fn find_companions(
    paths: Vec<String>,
    is_ssh: bool,
    ssh_host: String,
) -> Result<Vec<CompanionSet>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if is_ssh {
            remote_companions(&ssh_host, &paths)
        } else {
            Ok(local_companions(&paths))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate_paths(path: &str) -> Vec<String> {
        candidates(path).into_iter().map(|(p, _)| p).collect()
    }

    #[test]
    fn bam_files_look_for_both_index_conventions() {
        let found = candidate_paths("/data/sample.bam");
        assert!(found.contains(&"/data/sample.bam.bai".to_string()), "samtools style");
        assert!(found.contains(&"/data/sample.bai".to_string()), "picard style");
        assert!(found.contains(&"/data/sample.bam.md5".to_string()));
    }

    #[test]
    fn bgzipped_files_look_for_tabix_indexes() {
        let found = candidate_paths("/data/calls.vcf.gz");
        assert!(found.contains(&"/data/calls.vcf.gz.tbi".to_string()));
        assert!(found.contains(&"/data/calls.vcf.gz.csi".to_string()));
    }

    #[test]
    fn references_look_for_fai_and_dict() {
        let found = candidate_paths("/ref/hg38.fa");
        assert!(found.contains(&"/ref/hg38.fa.fai".to_string()));
        assert!(found.contains(&"/ref/hg38.dict".to_string()));
    }

    #[test]
    fn handles_paths_that_are_not_plain_ascii() {
        // A cut landing inside a multi-byte character must not panic.
        let found = candidate_paths("/data/prov–körning.bam");
        assert!(found.contains(&"/data/prov–körning.bam.bai".to_string()));
        assert!(candidate_paths("/data/åäö").len() >= 3);
        assert!(candidate_paths("é").len() >= 3);
    }

    #[test]
    fn finds_the_other_read_of_a_pair() {
        assert_eq!(
            mate_path("/run/Sample_S1_R1_001.fastq.gz").as_deref(),
            Some("/run/Sample_S1_R2_001.fastq.gz")
        );
        assert_eq!(
            mate_path("/run/Sample_S1_R2_001.fastq.gz").as_deref(),
            Some("/run/Sample_S1_R1_001.fastq.gz")
        );
        assert_eq!(
            mate_path("/run/SRR123_1.fastq.gz").as_deref(),
            Some("/run/SRR123_2.fastq.gz")
        );
        assert_eq!(mate_path("/run/notes.txt"), None);
        assert_eq!(mate_path("/run/sample.bam"), None);
    }

    #[test]
    fn only_reports_companions_that_exist() {
        let dir = std::env::temp_dir().join(format!("fb_companion_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let bam = dir.join("sample.bam");
        std::fs::write(&bam, b"bam").unwrap();
        std::fs::write(dir.join("sample.bam.bai"), b"index").unwrap();
        // No .md5 on disk: it must not be reported.

        let sets = local_companions(&[bam.to_string_lossy().to_string()]);
        assert_eq!(sets.len(), 1);
        let names: Vec<&str> = sets[0].companions.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["sample.bam.bai"]);
        assert_eq!(sets[0].companions[0].kind, CompanionKind::Index);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn qc_results_travel_with_the_alignment() {
        let found = candidate_paths("/data/sample.bam");
        assert!(found.contains(&"/data/sample.align.summary.json".to_string()));
        assert!(found.contains(&"/data/sample.dna.summary.txt".to_string()));
        assert!(found.contains(&"/data/sample.report.html".to_string()));
    }

    #[test]
    fn already_selected_files_are_not_offered_again() {
        let dir = std::env::temp_dir().join(format!("fb_companion_sel_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let bam = dir.join("sample.bam");
        let bai = dir.join("sample.bam.bai");
        std::fs::write(&bam, b"bam").unwrap();
        std::fs::write(&bai, b"index").unwrap();

        let sets = local_companions(&[
            bam.to_string_lossy().to_string(),
            bai.to_string_lossy().to_string(),
        ]);
        assert!(sets[0].companions.is_empty(), "the .bai was already selected");

        std::fs::remove_dir_all(&dir).ok();
    }
}
