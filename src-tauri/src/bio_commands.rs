use crate::fs_commands::{format_byte_size, resolve_path};
use crate::models::{
    ArchiveEntry, ArchiveSummary, BamHeaderData, ContigInfo, GenomeRefInfo, ProgramInfo,
    ReadGroupInfo, SamRecord, SamViewResult, TrackGenomeDetection,
};
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

pub fn get_genomes_config_path() -> PathBuf {
    crate::fs_commands::dirs_home().join(".config/flashbrowse/genomes.json")
}

fn save_genomes_to_config(genomes: &[GenomeRefInfo]) -> Result<(), String> {
    let cfg_path = get_genomes_config_path();
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(genomes).map_err(|e| e.to_string())?;
    fs::write(cfg_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn discover_default_genomes() -> Vec<GenomeRefInfo> {
    let home = crate::fs_commands::dirs_home();
    let mut genomes = Vec::new();

    // 1. Check GRCh38 / hg38
    let hg38_fasta_candidates = vec![
        home.join("ref/genomes/GRCh38-GIABv3/GRCh38_GIABv3_no_alt_analysis_set_maskedGRC_decoys_MAP2K3_KMT2C_KCNJ18.fasta"),
        home.join("ref/GRCh38.fasta"),
        home.join("ref/hg38.fasta"),
        home.join("ref/hg38.fa"),
        home.join("genomes/hg38/hg38.fa"),
        home.join(".genome/hg38.fa"),
    ];
    let hg38_gtf_candidates = vec![
        home.join("ref/genomes/GRCh38-GIABv3/gencode.v46.annotation.sorted.gtf.gz"),
        home.join("ref/genomes/rseqc/hg38_GENCODE.v38.bed"),
        home.join("ref/gencode.v38.annotation.gtf"),
        home.join("ref/gencode.v46.annotation.gtf"),
    ];

    let mut hg38_fasta = None;
    let mut hg38_fai = None;
    for cand in hg38_fasta_candidates {
        if cand.exists() {
            let fai = PathBuf::from(format!("{}.fai", cand.display()));
            if fai.exists() {
                hg38_fai = Some(fai.to_string_lossy().to_string());
            }
            hg38_fasta = Some(cand.to_string_lossy().to_string());
            break;
        }
    }
    let hg38_gtf = hg38_gtf_candidates.into_iter().find(|p| p.exists()).map(|p| p.to_string_lossy().to_string());

    genomes.push(GenomeRefInfo {
        id: "hg38".to_string(),
        name: "GRCh38 / hg38".to_string(),
        fasta_path: hg38_fasta.clone(),
        fai_path: hg38_fai.clone(),
        gtf_path: hg38_gtf,
        is_available: hg38_fasta.is_some() && hg38_fai.is_some(),
    });

    // 2. Check GRCh37 / hg19
    let hg19_fasta_candidates = vec![
        home.join("ref/genomes/hg19_pickett/ref_genome.fa"),
        home.join("ref/hg19.fasta"),
        home.join("ref/hg19.fa"),
        home.join("ref/hs37d5.fa"),
    ];
    let hg19_gtf_candidates = vec![
        home.join("ref/genomes/hg19_pickett/ref_annot.gtf"),
        home.join("ref/hg19.refGene.gtf"),
    ];

    let mut hg19_fasta = None;
    let mut hg19_fai = None;
    for cand in hg19_fasta_candidates {
        if cand.exists() {
            let fai = PathBuf::from(format!("{}.fai", cand.display()));
            if fai.exists() {
                hg19_fai = Some(fai.to_string_lossy().to_string());
            }
            hg19_fasta = Some(cand.to_string_lossy().to_string());
            break;
        }
    }
    let hg19_gtf = hg19_gtf_candidates.into_iter().find(|p| p.exists()).map(|p| p.to_string_lossy().to_string());

    genomes.push(GenomeRefInfo {
        id: "hg19".to_string(),
        name: "GRCh37 / hg19 / hs37d5".to_string(),
        fasta_path: hg19_fasta.clone(),
        fai_path: hg19_fai.clone(),
        gtf_path: hg19_gtf,
        is_available: hg19_fasta.is_some() && hg19_fai.is_some(),
    });

    genomes
}

#[tauri::command]
pub fn get_configured_genomes() -> Result<Vec<GenomeRefInfo>, String> {
    let cfg_path = get_genomes_config_path();
    if cfg_path.exists() {
        if let Ok(content) = fs::read_to_string(&cfg_path) {
            if let Ok(mut list) = serde_json::from_str::<Vec<GenomeRefInfo>>(&content) {
                // Verify availability
                for g in &mut list {
                    let fasta_ok = g.fasta_path.as_ref().map(|p| resolve_path(p).exists()).unwrap_or(false);
                    let fai_ok = g.fai_path.as_ref().map(|p| resolve_path(p).exists()).unwrap_or(false);
                    g.is_available = fasta_ok && fai_ok;
                }
                return Ok(list);
            }
        }
    }

    let defaults = discover_default_genomes();
    let _ = save_genomes_to_config(&defaults);
    Ok(defaults)
}

#[tauri::command]
pub fn save_configured_genome(genome: GenomeRefInfo) -> Result<Vec<GenomeRefInfo>, String> {
    let mut current = get_configured_genomes().unwrap_or_default();
    if let Some(idx) = current.iter().position(|g| g.id == genome.id) {
        current[idx] = genome;
    } else {
        current.push(genome);
    }
    save_genomes_to_config(&current)?;
    Ok(current)
}

#[tauri::command]
pub async fn detect_track_genomes(paths: Vec<String>) -> Result<Vec<TrackGenomeDetection>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut results = Vec::new();
        let samtools = find_tool_executable("samtools");

        for p in paths {
            let res = resolve_path(&p);
            let name = res.file_name().unwrap_or_default().to_string_lossy().to_string();
            let ext = res.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();

            if (ext == "bam" || ext == "cram" || ext == "sam") && samtools.is_some() {
                let out = Command::new(samtools.as_ref().unwrap())
                    .arg("view")
                    .arg("-H")
                    .arg(&res)
                    .output();

                if let Ok(o) = out {
                    if o.status.success() {
                        let header = String::from_utf8_lossy(&o.stdout);
                        let mut chr1_len = None;
                        for line in header.lines() {
                            if line.starts_with("@SQ") {
                                let mut is_chr1 = false;
                                let mut len = None;
                                for part in line.split('\t') {
                                    if part == "SN:chr1" || part == "SN:1" {
                                        is_chr1 = true;
                                    } else if let Some(stripped) = part.strip_prefix("LN:") {
                                        len = stripped.parse::<u64>().ok();
                                    }
                                }
                                if is_chr1 {
                                    chr1_len = len;
                                    break;
                                }
                            }
                        }

                        let (build, label) = match chr1_len {
                            Some(248_956_422) => ("hg38".to_string(), "GRCh38 / hg38".to_string()),
                            Some(249_250_621) => ("hg19".to_string(), "GRCh37 / hg19".to_string()),
                            Some(248_387_328) => ("t2t".to_string(), "T2T-CHM13".to_string()),
                            Some(195_471_971) => ("mm10".to_string(), "GRCm38 / mm10".to_string()),
                            Some(195_154_279) => ("mm39".to_string(), "GRCm39 / mm39".to_string()),
                            Some(l) => ("custom".to_string(), format!("Custom (chr1: {} bp)", l)),
                            None => ("unknown".to_string(), "Okänt genom".to_string()),
                        };

                        results.push(TrackGenomeDetection {
                            path: p,
                            name,
                            detected_build: build,
                            detected_label: label,
                            chr1_len,
                        });
                        continue;
                    }
                }
            }

            // Fallback for VCF/BED or files where header couldn't be parsed
            let lower = name.to_lowercase();
            let (build, label) = if lower.contains("hg38") || lower.contains("grch38") {
                ("hg38".to_string(), "GRCh38 / hg38".to_string())
            } else if lower.contains("hg19") || lower.contains("grch37") || lower.contains("hs37d5") || lower.contains("b37") {
                ("hg19".to_string(), "GRCh37 / hg19".to_string())
            } else {
                ("unknown".to_string(), "Okänt genom".to_string())
            };

            results.push(TrackGenomeDetection {
                path: p,
                name,
                detected_build: build,
                detected_label: label,
                chr1_len: None,
            });
        }

        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Find a matching local reference genome FASTA if available
pub fn find_matching_reference_file(detected_build: &str) -> Option<String> {
    let configured = get_configured_genomes().unwrap_or_default();
    let lower = detected_build.to_lowercase();

    for g in &configured {
        if (lower.contains("38") && g.id == "hg38") || (lower.contains("19") && g.id == "hg19") || (lower.contains("37") && g.id == "hg19") {
            if let Some(ref fa) = g.fasta_path {
                if resolve_path(fa).exists() {
                    return Some(fa.clone());
                }
            }
        }
    }

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
    genome_id: Option<String>,
    ref_path: Option<String>,
    gtf_path: Option<String>,
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

        // Resolve reference & GTF
        let configured = get_configured_genomes().unwrap_or_default();
        let target_genome = if let Some(gid) = genome_id {
            configured.iter().find(|g| g.id == gid).cloned()
        } else {
            configured.iter().find(|g| g.id == "hg38").or_else(|| configured.first()).cloned()
        };

        let effective_ref = ref_path.or_else(|| target_genome.as_ref().and_then(|g| g.fasta_path.clone()));
        let effective_gtf = gtf_path.or_else(|| target_genome.as_ref().and_then(|g| g.gtf_path.clone()));

        if let Some(r) = effective_ref {
            let res_ref = resolve_path(&r);
            if res_ref.exists() {
                cmd.arg("-r").arg(res_ref.to_string_lossy().to_string());
            }
        }

        if let Some(g) = effective_gtf {
            let res_gtf = resolve_path(&g);
            if res_gtf.exists() {
                cmd.arg("-g").arg(res_gtf.to_string_lossy().to_string());
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

static RSNAP_SERVER_PROCESS: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RsnapServerInfo {
    pub is_running: bool,
    pub pid: Option<u32>,
    pub port: u16,
    pub bam_dir: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IgvResponse {
    pub success: bool,
    pub message: String,
}

/// Start rsnap background server
#[tauri::command]
pub fn start_rsnap_server(
    bam_dir: Option<String>,
    genome_id: Option<String>,
    port: Option<u16>,
) -> Result<RsnapServerInfo, String> {
    let mut lock = RSNAP_SERVER_PROCESS.lock().unwrap();
    if let Some(ref mut child) = *lock {
        match child.try_wait() {
            Ok(None) => {
                // Server is already running
                return Ok(RsnapServerInfo {
                    is_running: true,
                    pid: Some(child.id()),
                    port: port.unwrap_or(5555),
                    bam_dir,
                });
            }
            _ => {
                *lock = None;
            }
        }
    }

    let rsnap_bin = find_tool_executable("rsnap")
        .ok_or_else(|| "rsnap executable not found in dev/bin or PATH".to_string())?;

    let mut cmd = Command::new(rsnap_bin);
    cmd.arg("--server");

    let resolved_bam_dir = if let Some(ref d) = bam_dir {
        let res = resolve_path(d);
        if res.is_dir() {
            let s = res.to_string_lossy().to_string();
            cmd.arg("--bam-dir").arg(&s);
            Some(s)
        } else {
            None
        }
    } else {
        None
    };

    let configured = get_configured_genomes().unwrap_or_default();
    let target_genome = if let Some(gid) = genome_id {
        configured.iter().find(|g| g.id == gid).cloned()
    } else {
        configured.iter().find(|g| g.id == "hg38").or_else(|| configured.first()).cloned()
    };

    if let Some(g) = target_genome {
        if let Some(ref fa) = g.fasta_path {
            let res_fa = resolve_path(fa);
            if res_fa.exists() {
                cmd.arg("-r").arg(res_fa.to_string_lossy().to_string());
            }
        }
        if let Some(ref gtf) = g.gtf_path {
            let res_gtf = resolve_path(gtf);
            if res_gtf.exists() {
                cmd.arg("-g").arg(res_gtf.to_string_lossy().to_string());
            }
        }
    }

    let child = cmd.spawn().map_err(|e| format!("Kunde inte starta rsnap server: {}", e))?;
    let pid = child.id();
    *lock = Some(child);

    Ok(RsnapServerInfo {
        is_running: true,
        pid: Some(pid),
        port: port.unwrap_or(5555),
        bam_dir: resolved_bam_dir,
    })
}

/// Stop running rsnap background server
#[tauri::command]
pub fn stop_rsnap_server() -> Result<bool, String> {
    let mut lock = RSNAP_SERVER_PROCESS.lock().unwrap();
    if let Some(mut child) = lock.take() {
        let _ = child.kill();
        let _ = child.wait();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Query status of rsnap background server
#[tauri::command]
pub fn get_rsnap_server_status() -> Result<RsnapServerInfo, String> {
    let mut lock = RSNAP_SERVER_PROCESS.lock().unwrap();
    if let Some(ref mut child) = *lock {
        match child.try_wait() {
            Ok(None) => {
                return Ok(RsnapServerInfo {
                    is_running: true,
                    pid: Some(child.id()),
                    port: 5555,
                    bam_dir: None,
                });
            }
            _ => {
                *lock = None;
            }
        }
    }
    Ok(RsnapServerInfo {
        is_running: false,
        pid: None,
        port: 5555,
        bam_dir: None,
    })
}

/// Launch rsnap interactive viewer (standalone or connecting to server)
#[tauri::command]
pub fn launch_rsnap(
    paths: Vec<String>,
    region: Option<String>,
    genome_id: Option<String>,
    ref_path: Option<String>,
    gtf_path: Option<String>,
    connect_to_server: Option<bool>,
    server_address: Option<String>,
) -> Result<(), String> {
    let rsnap_bin = find_tool_executable("rsnap")
        .ok_or_else(|| "rsnap executable not found in dev/bin or PATH".to_string())?;

    let mut cmd = Command::new(rsnap_bin);
    cmd.arg("--viewer");

    if connect_to_server.unwrap_or(false) {
        let addr = server_address.unwrap_or_else(|| "localhost:5555".to_string());
        cmd.arg("--remote").arg(addr);
    }

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
            cmd.arg("-p").arg(r.trim());
        }
    }

    // Resolve reference genome FASTA & GTF
    let configured = get_configured_genomes().unwrap_or_default();
    let target_genome = if let Some(gid) = genome_id {
        configured.iter().find(|g| g.id == gid).cloned()
    } else {
        configured.iter().find(|g| g.id == "hg38").or_else(|| configured.first()).cloned()
    };

    let effective_ref = ref_path.or_else(|| target_genome.as_ref().and_then(|g| g.fasta_path.clone()));
    let effective_gtf = gtf_path.or_else(|| target_genome.as_ref().and_then(|g| g.gtf_path.clone()));

    if let Some(rf) = effective_ref {
        let res_ref = resolve_path(&rf);
        if res_ref.exists() {
            cmd.arg("-r").arg(res_ref.to_string_lossy().to_string());
        }
    }

    if let Some(gtf) = effective_gtf {
        let res_gtf = resolve_path(&gtf);
        if res_gtf.exists() {
            cmd.arg("-g").arg(res_gtf.to_string_lossy().to_string());
        }
    }

    cmd.spawn().map_err(|e| format!("Kunde inte starta rsnap viewer: {}", e))?;
    Ok(())
}

fn igv_http_request(port: u16, path_and_query: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(1500))
        .map_err(|_| format!("Kunde inte ansluta till IGV på port {}. Kontrollera att IGV är igång och 'Enable port' är aktiverat i IGV Preferences.", port))?;

    stream.set_read_timeout(Some(Duration::from_millis(2000))).ok();
    stream.set_write_timeout(Some(Duration::from_millis(2000))).ok();

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        path_and_query, port
    );

    stream.write_all(request.as_bytes()).map_err(|e| format!("Write failed: {}", e))?;

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    Ok(response)
}

/// Send tracks and locus to running IGV desktop instance
#[tauri::command]
pub fn send_to_igv(
    paths: Vec<String>,
    locus: Option<String>,
    genome: Option<String>,
    port: Option<u16>,
) -> Result<IgvResponse, String> {
    let igv_port = port.unwrap_or(60151);

    // 1. Optionally switch genome if provided
    if let Some(ref g) = genome {
        let trimmed = g.trim();
        if !trimmed.is_empty() {
            let encoded_genome = urlencoding::encode(trimmed);
            let query = format!("/genome?id={}", encoded_genome);
            let _ = igv_http_request(igv_port, &query);
        }
    }

    // 2. Load files
    let mut loaded_count = 0;
    for p in &paths {
        let res = resolve_path(p);
        let path_str = res.to_string_lossy().to_string();
        let encoded_path = urlencoding::encode(&path_str);
        let query = format!("/load?file={}", encoded_path);
        match igv_http_request(igv_port, &query) {
            Ok(_) => { loaded_count += 1; }
            Err(e) => {
                return Err(format!("IGV fel vid inläsning av {}: {}", p, e));
            }
        }
    }

    // 3. Optionally navigate to locus
    if let Some(ref loc) = locus {
        let trimmed = loc.trim();
        if !trimmed.is_empty() {
            let encoded_locus = urlencoding::encode(trimmed);
            let query = format!("/goto?locus={}", encoded_locus);
            let _ = igv_http_request(igv_port, &query);
        }
    }

    Ok(IgvResponse {
        success: true,
        message: format!("Skickade {} spår till IGV (port {})", loaded_count, igv_port),
    })
}

/// Check if IGV HTTP port is reachable
#[tauri::command]
pub fn check_igv_status(port: Option<u16>) -> bool {
    let igv_port = port.unwrap_or(60151);
    igv_http_request(igv_port, "/").is_ok() || igv_http_request(igv_port, "/ping").is_ok()
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

/// Parse SAM bitwise flag into descriptive strings
pub fn parse_sam_flags(flag: u16) -> Vec<String> {
    let mut descs = Vec::new();
    if flag & 0x1 != 0 { descs.push("PAIRED".to_string()); }
    if flag & 0x2 != 0 { descs.push("PROPER_PAIR".to_string()); }
    if flag & 0x4 != 0 { descs.push("UNMAP".to_string()); }
    if flag & 0x8 != 0 { descs.push("MUNMAP".to_string()); }
    if flag & 0x10 != 0 { descs.push("REVERSE".to_string()); }
    if flag & 0x20 != 0 { descs.push("MREVERSE".to_string()); }
    if flag & 0x40 != 0 { descs.push("READ1".to_string()); }
    if flag & 0x80 != 0 { descs.push("READ2".to_string()); }
    if flag & 0x100 != 0 { descs.push("SECONDARY".to_string()); }
    if flag & 0x200 != 0 { descs.push("QCFAIL".to_string()); }
    if flag & 0x400 != 0 { descs.push("DUP".to_string()); }
    if flag & 0x800 != 0 { descs.push("SUPPLEMENTARY".to_string()); }
    descs
}

/// Fetch paginated SAM alignment records using `samtools view` (with optional genomic locus/region)
#[tauri::command]
pub async fn get_bam_alignments(
    path: String,
    region: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<SamViewResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let resolved_path = resolve_path(&path);
        if !resolved_path.exists() {
            return Err(format!("File does not exist: {}", resolved_path.display()));
        }

        let samtools = find_tool_executable("samtools")
            .ok_or_else(|| "samtools hittades inte i PATH".to_string())?;

        let rec_limit = limit.unwrap_or(50);
        let rec_offset = offset.unwrap_or(0);
        let total_needed = rec_offset + rec_limit;

        let mut cmd = Command::new(&samtools);
        cmd.arg("view");
        cmd.arg(&resolved_path);

        if let Some(ref reg) = region {
            let trimmed = reg.trim();
            if !trimmed.is_empty() {
                cmd.arg(trimmed);
            }
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Kunde inte starta samtools view: {}", e))?;

        let stdout = child.stdout.take().ok_or_else(|| "Kunde inte öppna samtools stdout".to_string())?;
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);

        let mut records = Vec::new();
        let mut raw_lines = Vec::new();
        let mut current_idx = 0;
        let mut has_more = false;

        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => break,
            };

            if current_idx >= rec_offset && current_idx < total_needed {
                raw_lines.push(line.clone());
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 11 {
                    let qname = parts[0].to_string();
                    let flag: u16 = parts[1].parse().unwrap_or(0);
                    let flag_desc = parse_sam_flags(flag);
                    let rname = parts[2].to_string();
                    let pos: i64 = parts[3].parse().unwrap_or(0);
                    let mapq: u8 = parts[4].parse().unwrap_or(0);
                    let cigar = parts[5].to_string();
                    let rnext = parts[6].to_string();
                    let pnext: i64 = parts[7].parse().unwrap_or(0);
                    let tlen: i64 = parts[8].parse().unwrap_or(0);
                    let seq = parts[9].to_string();
                    let qual = parts[10].to_string();
                    let tags = if parts.len() > 11 {
                        parts[11..].iter().map(|s| s.to_string()).collect()
                    } else {
                        Vec::new()
                    };

                    records.push(SamRecord {
                        qname,
                        flag,
                        flag_desc,
                        rname,
                        pos,
                        mapq,
                        cigar,
                        rnext,
                        pnext,
                        tlen,
                        seq,
                        qual,
                        tags,
                        raw_line: line,
                    });
                }
            } else if current_idx >= total_needed {
                has_more = true;
                break;
            }
            current_idx += 1;
        }

        // Cleanly terminate samtools process as soon as we have our slice of lines
        let _ = child.kill();
        let _ = child.wait();

        Ok(SamViewResult {
            records,
            region,
            total_fetched: current_idx.saturating_sub(rec_offset).min(rec_limit),
            offset: rec_offset,
            limit: rec_limit,
            has_more,
            raw_output: raw_lines.join("\n"),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

