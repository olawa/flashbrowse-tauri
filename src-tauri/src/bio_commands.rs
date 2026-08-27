use crate::fs_commands::{format_byte_size, resolve_path};
use crate::models::{ArchiveEntry, ArchiveSummary, BamHeaderData, ContigInfo, ProgramInfo, ReadGroupInfo};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

/// Locate bioinformatics tools in common dev/system paths
pub fn find_tool_executable(name: &str) -> Option<PathBuf> {
    let home = crate::fs_commands::dirs_home();
    let candidates = vec![
        home.join("dev/bin").join(name),
        home.join(".cargo/bin").join(name),
        home.join("bin").join(name),
        home.join(".local/bin").join(name),
        PathBuf::from("/opt/homebrew/bin").join(name),
        PathBuf::from("/usr/local/bin").join(name),
        PathBuf::from("/usr/bin").join(name),
        PathBuf::from("/bin").join(name),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_env) {
            let full = dir.join(name);
            if full.is_file() {
                return Some(full);
            }
        }
    }

    None
}

/// Find a matching local reference genome FASTA if available
pub fn find_matching_reference_file(detected_build: &str) -> Option<String> {
    let home = crate::fs_commands::dirs_home();
    let search_roots = vec![
        home.join(".genome"),
        home.join("genome"),
        home.join("genomes"),
        home.join("data/ref"),
        home.join("ref"),
        home.join("references"),
        home.join("dev/genomes"),
        home.join("dev/ref"),
        PathBuf::from("/data/ref"),
        PathBuf::from("/ref"),
    ];

    let pattern = match detected_build {
        b if b.contains("GRCh38") || b.contains("hg38") => vec!["hg38", "GRCh38", "grch38"],
        b if b.contains("GRCh37") || b.contains("hs37d5") || b.contains("b37") => vec!["hs37d5", "b37", "GRCh37", "grch37"],
        b if b.contains("hg19") => vec!["hg19"],
        b if b.contains("mm10") || b.contains("GRCm38") => vec!["mm10", "GRCm38"],
        b if b.contains("mm39") || b.contains("GRCm39") => vec!["mm39", "GRCm39"],
        b if b.contains("T2T") => vec!["t2t", "chm13", "T2T"],
        _ => return None,
    };

    for root in search_roots {
        if !root.exists() || !root.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "fa" || ext_str == "fasta" || ext_str == "fna" {
                        let name_lower = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                        for pat in &pattern {
                            if name_lower.contains(&pat.to_lowercase()) {
                                return Some(p.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Inspect BAM / CRAM / SAM header, extract read groups, tools (@PG), contigs (@SQ) and auto-detect reference
#[tauri::command]
pub async fn get_bam_header(path: String) -> Result<BamHeaderData, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let resolved_path = resolve_path(&path);

        if !resolved_path.exists() {
            return Err(format!("File does not exist: {}", resolved_path.display()));
        }

        // 1. Check for index file
        let path_str = resolved_path.to_string_lossy().to_string();
        let bai_path = PathBuf::from(format!("{}.bai", path_str));
        let bai_alt = resolved_path.with_extension("bai");
        let crai_path = PathBuf::from(format!("{}.crai", path_str));
        let crai_alt = resolved_path.with_extension("crai");
        let csi_path = PathBuf::from(format!("{}.csi", path_str));

        let (has_index, index_type) = if bai_path.exists() || bai_alt.exists() {
            (true, Some("BAI".to_string()))
        } else if crai_path.exists() || crai_alt.exists() {
            (true, Some("CRAI".to_string()))
        } else if csi_path.exists() {
            (true, Some("CSI".to_string()))
        } else {
            (false, None)
        };

        // 2. Read SAM/BAM header via samtools
        let samtools_bin = find_tool_executable("samtools").ok_or_else(|| {
            "samtools is required to inspect BAM/CRAM headers. Please install samtools (e.g. 'brew install samtools').".to_string()
        })?;

        let output = Command::new(samtools_bin)
            .arg("view")
            .arg("-H")
            .arg(&resolved_path)
            .output()
            .map_err(|e| format!("Failed to run samtools view -H: {}", e))?;

        let raw_header = if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(format!("samtools error: {}", err));
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        if raw_header.is_empty() {
            return Err("BAM header is empty or invalid.".to_string());
        }

        // 3. Parse header lines
        let mut contigs: Vec<ContigInfo> = Vec::new();
        let mut read_groups: Vec<ReadGroupInfo> = Vec::new();
        let mut programs: Vec<ProgramInfo> = Vec::new();
        let mut total_genome_length: u64 = 0;

        let mut chr1_len: Option<u64> = None;
        let mut chr1_name: Option<String> = None;

        for line in raw_header.lines() {
            if line.starts_with("@SQ") {
                let mut name = String::new();
                let mut len: u64 = 0;
                let mut assembly: Option<String> = None;

                for part in line.split('\t').skip(1) {
                    if let Some(stripped) = part.strip_prefix("SN:") {
                        name = stripped.to_string();
                    } else if let Some(stripped) = part.strip_prefix("LN:") {
                        len = stripped.parse().unwrap_or(0);
                    } else if let Some(stripped) = part.strip_prefix("AS:") {
                        assembly = Some(stripped.to_string());
                    }
                }

                if !name.is_empty() {
                    if name == "chr1" || name == "1" {
                        chr1_len = Some(len);
                        chr1_name = Some(name.clone());
                    }
                    total_genome_length += len;
                    contigs.push(ContigInfo {
                        name,
                        length: len,
                        formatted_length: format_contig_length(len),
                        assembly,
                    });
                }
            } else if line.starts_with("@RG") {
                let mut id = String::new();
                let mut sample: Option<String> = None;
                let mut platform: Option<String> = None;
                let mut library: Option<String> = None;
                let mut center: Option<String> = None;

                for part in line.split('\t').skip(1) {
                    if let Some(stripped) = part.strip_prefix("ID:") {
                        id = stripped.to_string();
                    } else if let Some(stripped) = part.strip_prefix("SM:") {
                        sample = Some(stripped.to_string());
                    } else if let Some(stripped) = part.strip_prefix("PL:") {
                        platform = Some(stripped.to_string());
                    } else if let Some(stripped) = part.strip_prefix("LB:") {
                        library = Some(stripped.to_string());
                    } else if let Some(stripped) = part.strip_prefix("CN:") {
                        center = Some(stripped.to_string());
                    }
                }

                if !id.is_empty() {
                    read_groups.push(ReadGroupInfo {
                        id,
                        sample,
                        platform,
                        library,
                        center,
                    });
                }
            } else if line.starts_with("@PG") {
                let mut id = String::new();
                let mut name: Option<String> = None;
                let mut version: Option<String> = None;
                let mut command_line: Option<String> = None;

                for part in line.split('\t').skip(1) {
                    if let Some(stripped) = part.strip_prefix("ID:") {
                        id = stripped.to_string();
                    } else if let Some(stripped) = part.strip_prefix("PN:") {
                        name = Some(stripped.to_string());
                    } else if let Some(stripped) = part.strip_prefix("VN:") {
                        version = Some(stripped.to_string());
                    } else if let Some(stripped) = part.strip_prefix("CL:") {
                        command_line = Some(stripped.to_string());
                    }
                }

                if !id.is_empty() {
                    programs.push(ProgramInfo {
                        id,
                        name,
                        version,
                        command_line,
                    });
                }
            }
        }

        // 4. Auto-detect Reference Genome Build
        let detected_reference = match (chr1_name.as_deref(), chr1_len) {
            (Some("chr1"), Some(248_956_422)) => "GRCh38 / hg38".to_string(),
            (Some("1"), Some(249_250_621)) => "GRCh37 / hs37d5 / b37".to_string(),
            (Some("chr1"), Some(249_250_621)) => "hg19 (UCSC)".to_string(),
            (Some("chr1"), Some(248_387_328)) => "T2T-CHM13 v2.0".to_string(),
            (Some("chr1"), Some(195_471_971)) => "GRCm38 / mm10 (Mus musculus)".to_string(),
            (Some("chr1"), Some(195_154_279)) => "GRCm39 / mm39 (Mus musculus)".to_string(),
            (Some(n), Some(l)) => format!("Custom ({} = {} bp)", n, l),
            _ => "Unknown Genome".to_string(),
        };

        let reference_matched_path = find_matching_reference_file(&detected_reference);
        let total_contigs = contigs.len();

        Ok(BamHeaderData {
            detected_reference,
            reference_matched_path,
            contigs,
            total_contigs,
            total_genome_length,
            formatted_genome_length: format_contig_length(total_genome_length),
            read_groups,
            programs,
            raw_header,
            has_index,
            index_type,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn format_contig_length(len: u64) -> String {
    if len >= 1_000_000_000 {
        format!("{:.2} Gbp", len as f64 / 1_000_000_000.0)
    } else if len >= 1_000_000 {
        format!("{:.2} Mbp", len as f64 / 1_000_000.0)
    } else if len >= 1_000 {
        format!("{:.1} kbp", len as f64 / 1_000.0)
    } else {
        format!("{} bp", len)
    }
}

/// Generate a high-speed rsnap PNG snapshot for a region and return Base64 image
#[tauri::command]
pub async fn generate_rsnap_snapshot(
    bam_path: String,
    region: String,
    ref_path: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let rsnap_bin = find_tool_executable("rsnap")
            .ok_or_else(|| "rsnap executable not found in dev/bin or PATH".to_string())?;

        let resolved_bam = resolve_path(&bam_path);
        let resolved_bam_str = resolved_bam.to_string_lossy().to_string();

        let temp_file = tempfile::Builder::new()
            .prefix("flashbrowse_rsnap_")
            .suffix(".png")
            .tempfile()
            .map_err(|e| format!("Failed to create temporary file: {}", e))?;
        let temp_path = temp_file.path().to_path_buf();
        let temp_out_str = temp_path.to_string_lossy().to_string();

        let mut cmd = Command::new(rsnap_bin);
        cmd.arg("-b").arg(&resolved_bam_str);
        cmd.arg("-p").arg(&region);
        cmd.arg("-o").arg(&temp_out_str);

        if let Some(r) = ref_path {
            let res_ref = resolve_path(&r);
            if res_ref.exists() {
                cmd.arg("-r").arg(res_ref.to_string_lossy().to_string());
            }
        }

        let output = cmd.output().map_err(|e| format!("Failed to execute rsnap: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("rsnap failed: {}", err));
        }

        if !temp_path.exists() {
            return Err("rsnap did not generate output image".to_string());
        }

        let mut f = File::open(&temp_path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).map_err(|e| e.to_string())?;

        let b64 = STANDARD.encode(&bytes);
        Ok(b64)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Launch rsnap interactive viewer
#[tauri::command]
pub fn launch_rsnap(
    paths: Vec<String>,
    region: Option<String>,
    ref_path: Option<String>,
) -> Result<(), String> {
    let rsnap_bin = find_tool_executable("rsnap")
        .ok_or_else(|| "rsnap executable not found in dev/bin or PATH".to_string())?;

    let mut cmd = Command::new(rsnap_bin);
    cmd.arg("--viewer");

    for p in &paths {
        let res_path = resolve_path(p);
        let path_str = res_path.to_string_lossy().to_string();
        let ext = res_path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if ext == "bam" || ext == "cram" || ext == "sam" {
            cmd.arg("-b").arg(path_str);
        } else if ext == "vcf" || ext == "bcf" || path_str.ends_with(".vcf.gz") {
            cmd.arg("-v").arg(path_str);
        } else if ext == "bed" || ext == "bw" || ext == "bigwig" {
            cmd.arg("--peak-track").arg(path_str);
        } else {
            cmd.arg("-b").arg(path_str);
        }
    }

    if let Some(r) = region {
        if !r.trim().is_empty() {
            cmd.arg("-p").arg(r);
        }
    }

    if let Some(rf) = ref_path {
        let res_ref = resolve_path(&rf);
        if res_ref.exists() {
            cmd.arg("-r").arg(res_ref.to_string_lossy().to_string());
        }
    }

    cmd.spawn().map_err(|e| format!("Failed to launch rsnap: {}", e))?;
    Ok(())
}

/// Run rs-qc alignment QC and return summary report
#[tauri::command]
pub async fn run_rs_qc(bam_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let rsqc_bin = find_tool_executable("rs-qc")
            .ok_or_else(|| "rs-qc executable not found in dev/bin or PATH".to_string())?;

        let resolved_path = resolve_path(&bam_path);

        let output = Command::new(rsqc_bin)
            .arg("align")
            .arg("-i")
            .arg(&resolved_path)
            .output()
            .map_err(|e| format!("Failed to execute rs-qc: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(format!("rs-qc error: {}\n{}", stderr, stdout));
        }

        Ok(stdout)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// List archive contents (.tar.gz, .zip, .tar, .tgz) without extracting to disk
#[tauri::command]
pub async fn list_archive_contents(path: String) -> Result<ArchiveSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let resolved_path = resolve_path(&path);

        if !resolved_path.exists() {
            return Err(format!("File does not exist: {}", resolved_path.display()));
        }

        let path_str = resolved_path.to_string_lossy().to_string();
        let ext = resolved_path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let file_name = resolved_path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let mut entries: Vec<ArchiveEntry> = Vec::new();
        let mut total_uncompressed_bytes: u64 = 0;

        if ext == "zip" {
            let output = Command::new("unzip")
                .arg("-l")
                .arg(&path_str)
                .output()
                .map_err(|e| format!("Failed to run unzip -l: {}", e))?;

            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().skip(3) {
                if line.contains("-----") || line.trim().is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(size) = parts[0].parse::<u64>() {
                        let date_str = format!("{} {}", parts[1], parts[2]);
                        let name = parts[3..].join(" ");
                        let is_dir = name.ends_with('/');
                        total_uncompressed_bytes += size;
                        entries.push(ArchiveEntry {
                            name,
                            size_bytes: size,
                            formatted_size: format_byte_size(size),
                            is_dir,
                            modified_str: date_str,
                        });
                    }
                }
            }
        } else if ext == "tar" || ext == "tgz" || file_name.ends_with(".tar.gz") || file_name.ends_with(".tar.bz2") || file_name.ends_with(".tar.xz") {
            let output = Command::new("tar")
                .arg("-tvf")
                .arg(&path_str)
                .output()
                .map_err(|e| format!("Failed to run tar -tvf: {}", e))?;

            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().take(1000) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let is_dir = parts[0].starts_with('d');
                    let size = parts[4].parse::<u64>().unwrap_or(0);
                    let date_str = format!("{} {} {}", parts[5], parts.get(6).unwrap_or(&""), parts.get(7).unwrap_or(&""));
                    let name = if parts.len() >= 9 { parts[8..].join(" ") } else { parts[parts.len() - 1].to_string() };
                    total_uncompressed_bytes += size;
                    entries.push(ArchiveEntry {
                        name,
                        size_bytes: size,
                        formatted_size: format_byte_size(size),
                        is_dir,
                        modified_str: date_str,
                    });
                }
            }
        } else {
            return Err(format!("Unsupported archive format: {}", ext));
        }

        let total_files = entries.len();
        Ok(ArchiveSummary {
            path: path_str,
            entries,
            total_files,
            total_uncompressed_bytes,
            formatted_uncompressed_size: format_byte_size(total_uncompressed_bytes),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
