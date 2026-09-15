//! Which alignments came from the same reads.
//!
//! A sample gets realigned, subset, sorted and renamed, and a directory ends up
//! holding several BAMs whose relationship is only visible in their headers:
//! the aligner's command line names the FASTQ files, later steps name the BAM
//! they were derived from, and read groups carry the sequencing unit. This
//! module reads that provenance and matches files on it.

use crate::fs_commands::{format_byte_size, resolve_path};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// Reading a header per candidate costs an exec each; this keeps a mistaken
/// "search everything" from turning into thousands of processes.
const MAX_CANDIDATES: usize = 400;

/// Header-derived identity of one alignment file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BamProvenance {
    pub path: String,
    pub name: String,
    /// FASTQ file names found in aligner command lines (@PG CL).
    pub fastqs: Vec<String>,
    /// BAM/CRAM files this one was derived from, also from @PG CL.
    pub source_bams: Vec<String>,
    /// @RG ID and PU values - the sequencing unit the reads came off.
    pub read_groups: Vec<String>,
    /// @RG SM: the sample name.
    pub sample: Option<String>,
}

/// One file related to the reference, and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedBam {
    pub path: String,
    pub name: String,
    pub formatted_size: String,
    /// Human-readable reasons, strongest first.
    pub matched_on: Vec<String>,
    pub provenance: BamProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedBamResult {
    pub reference: BamProvenance,
    pub related: Vec<RelatedBam>,
    /// Candidates whose header could not be read (not indexed, gone, remote).
    pub unreadable: Vec<String>,
    /// How many candidates were examined, after the cap.
    pub examined: usize,
    pub capped: bool,
}

fn basename(token: &str) -> String {
    Path::new(token)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| token.to_string())
}

fn is_fastq(token: &str) -> bool {
    let lower = token.to_lowercase();
    let stripped = lower
        .strip_suffix(".gz")
        .or_else(|| lower.strip_suffix(".bz2"))
        .unwrap_or(&lower);
    stripped.ends_with(".fastq") || stripped.ends_with(".fq")
}

/// Read-group and sample values that say nothing about origin.
///
/// GATK writes `ID:ArtificialHaplotypeRG SM:HC` into every file it calls
/// variants on, so two unrelated samples would otherwise "share a read group".
/// Bare numbers and placeholders are just as common across unrelated files.
fn is_uninformative_id(value: &str) -> bool {
    let v = value.trim().to_ascii_lowercase();
    v.is_empty()
        || v.chars().all(|c| c.is_ascii_digit())
        || matches!(
            v.as_str(),
            "artificialhaplotyperg"
                | "hc"
                | "none"
                | "null"
                | "unknown"
                | "na"
                | "n/a"
                | "sample"
                | "default"
                | "rg1"
                | "group1"
        )
}

fn is_alignment(token: &str) -> bool {
    let lower = token.to_lowercase();
    lower.ends_with(".bam") || lower.ends_with(".cram") || lower.ends_with(".sam")
}

/// Pull provenance out of a header.
///
/// Command lines are split on whitespace and commas: STAR takes
/// `--readFilesIn a_1.fastq.gz a_2.fastq.gz`, while some tools comma-separate
/// the pair.
pub fn provenance_from_header(path: &str, header: &str) -> BamProvenance {
    let mut fastqs = BTreeSet::new();
    let mut source_bams = BTreeSet::new();
    let mut read_groups = BTreeSet::new();
    let mut sample = None;

    let self_name = basename(path);

    for line in header.lines() {
        if line.starts_with("@PG") {
            for field in line.split('\t') {
                let Some(cl) = field.strip_prefix("CL:") else {
                    continue;
                };
                for token in cl.split([' ', '\t', ',', '=']) {
                    let token = token.trim_matches(|c| c == '"' || c == '\'');
                    if token.is_empty() {
                        continue;
                    }
                    if is_fastq(token) {
                        fastqs.insert(basename(token));
                    } else if is_alignment(token) {
                        let name = basename(token);
                        // A file listing itself says nothing about its origin.
                        if name != self_name {
                            source_bams.insert(name);
                        }
                    }
                }
            }
        } else if line.starts_with("@RG") {
            for field in line.split('\t') {
                if let Some(v) = field.strip_prefix("PU:").or_else(|| field.strip_prefix("ID:")) {
                    if !is_uninformative_id(v) {
                        read_groups.insert(v.to_string());
                    }
                } else if let Some(v) = field.strip_prefix("SM:") {
                    if !is_uninformative_id(v) && sample.is_none() {
                        sample = Some(v.to_string());
                    }
                }
            }
        }
    }

    BamProvenance {
        path: path.to_string(),
        name: self_name,
        fastqs: fastqs.into_iter().collect(),
        source_bams: source_bams.into_iter().collect(),
        read_groups: read_groups.into_iter().collect(),
        sample,
    }
}

/// Provenance already read, keyed by what would change if the file changed.
///
/// The index holds no header information, so every query execs samtools once
/// per candidate. Sizes and timestamps are already known to be stable for
/// finished alignments, so the second query over the same directory is free.
static CACHE: Mutex<Option<HashMap<CacheKey, BamProvenance>>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    path: String,
    len: u64,
    modified_secs: u64,
}

impl CacheKey {
    fn for_path(path: &str) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        let modified_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(Self {
            path: path.to_string(),
            len: meta.len(),
            modified_secs,
        })
    }
}

fn cached_provenance(samtools: &Path, path: &str) -> Option<BamProvenance> {
    let key = CacheKey::for_path(path);

    if let Some(key) = &key {
        if let Some(hit) = CACHE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .and_then(|c| c.get(key))
        {
            return Some(hit.clone());
        }
    }

    let header = read_header(samtools, path)?;
    let prov = provenance_from_header(path, &header);

    if let Some(key) = key {
        CACHE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_or_insert_with(HashMap::new)
            .insert(key, prov.clone());
    }
    Some(prov)
}

fn read_header(samtools: &Path, path: &str) -> Option<String> {
    let out = Command::new(samtools)
        .arg("view")
        .arg("-H")
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Why `other` is related to `reference`, strongest reason first.
///
/// An empty result means unrelated as far as the headers can tell.
pub fn match_reasons(reference: &BamProvenance, other: &BamProvenance) -> Vec<String> {
    let mut reasons = Vec::new();

    let shared_fastq: Vec<&String> = reference
        .fastqs
        .iter()
        .filter(|f| other.fastqs.contains(f))
        .collect();
    if !shared_fastq.is_empty() {
        reasons.push(format!(
            "Samma FASTQ: {}",
            shared_fastq
                .iter()
                .map(|f| f.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Derived from the same alignment, or one derived from the other.
    let shared_source: Vec<&String> = reference
        .source_bams
        .iter()
        .filter(|b| other.source_bams.contains(b))
        .collect();
    if !shared_source.is_empty() {
        reasons.push(format!(
            "Samma käll-BAM: {}",
            shared_source
                .iter()
                .map(|b| b.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if other.source_bams.contains(&reference.name) {
        reasons.push(format!("Härledd från {}", reference.name));
    }
    if reference.source_bams.contains(&other.name) {
        reasons.push(format!("{} är källa till denna", other.name));
    }

    let shared_rg: Vec<&String> = reference
        .read_groups
        .iter()
        .filter(|r| other.read_groups.contains(r))
        .collect();
    if !shared_rg.is_empty() {
        reasons.push(format!(
            "Samma läsgrupp: {}",
            shared_rg
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Sample alone is the weakest signal - a sample can have unrelated runs -
    // so it only counts when nothing stronger matched.
    if reasons.is_empty() {
        if let (Some(a), Some(b)) = (&reference.sample, &other.sample) {
            if a == b {
                reasons.push(format!("Samma prov (SM:{a})"));
            }
        }
    }

    reasons
}

/// Find which of `candidates` came from the same reads as `reference`.
#[tauri::command]
pub async fn find_related_bams(
    reference: String,
    candidates: Vec<String>,
) -> Result<RelatedBamResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let samtools = crate::bio_commands::find_tool_executable("samtools")
            .ok_or_else(|| "samtools hittades inte i dev/bin eller PATH".to_string())?;

        let reference_path = resolve_path(&reference).to_string_lossy().to_string();
        let reference_prov = cached_provenance(&samtools, &reference_path)
            .ok_or_else(|| format!("Kunde inte läsa headern för {reference_path}"))?;

        let to_check: Vec<String> = candidates
            .iter()
            .map(|c| resolve_path(c).to_string_lossy().to_string())
            .filter(|c| *c != reference_path)
            .collect();
        let capped = to_check.len() > MAX_CANDIDATES;
        let to_check: Vec<String> = to_check.into_iter().take(MAX_CANDIDATES).collect();

        let related = Mutex::new(Vec::new());
        let unreadable = Mutex::new(Vec::new());

        // One samtools exec per candidate, so run a few at a time. Eight keeps
        // a network filesystem busy without swamping a laptop.
        let next = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..8.min(to_check.len().max(1)) {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(path) = to_check.get(i) else {
                        return;
                    };

                    let Some(prov) = cached_provenance(&samtools, path) else {
                        unreadable.lock().unwrap().push(path.clone());
                        continue;
                    };
                    let reasons = match_reasons(&reference_prov, &prov);
                    if reasons.is_empty() {
                        continue;
                    }

                    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    related.lock().unwrap().push(RelatedBam {
                        name: prov.name.clone(),
                        path: path.clone(),
                        formatted_size: format_byte_size(size),
                        matched_on: reasons,
                        provenance: prov,
                    });
                });
            }
        });

        let mut related = related.into_inner().unwrap_or_default();
        // Strongest evidence first, then by name so the order is stable.
        related.sort_by(|a, b| {
            b.matched_on
                .len()
                .cmp(&a.matched_on.len())
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(RelatedBamResult {
            reference: reference_prov,
            related,
            unreadable: unreadable.into_inner().unwrap_or_default(),
            examined: to_check.len(),
            capped,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

// MARK: - Read-type classification
//
// Sorting alignments by what they are - RNA, short read, HiFi, ONT - is only
// possible from the header, and no single field carries it. @RG PL is the
// intended place but is often missing entirely (most of the files this was
// developed against have no @RG at all), and when present it holds an
// instrument name like "NovaSeq" rather than the spec's "ILLUMINA". So the
// aligner and its command line are read as well, and the reason is reported
// alongside the verdict so a wrong guess is visible rather than silent.

/// What kind of data an alignment holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadType {
    Rna,
    ShortRead,
    HiFi,
    Ont,
    /// Long read, but HiFi and ONT could not be told apart.
    LongRead,
    /// Nothing in the header says what this is.
    Unknown,
}

impl ReadType {
    pub fn id(&self) -> &'static str {
        match self {
            ReadType::Rna => "rna",
            ReadType::ShortRead => "sr",
            ReadType::HiFi => "hifi",
            ReadType::Ont => "ont",
            ReadType::LongRead => "lr",
            ReadType::Unknown => "unknown",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ReadType::Rna => "RNA",
            ReadType::ShortRead => "Kortläsning",
            ReadType::HiFi => "PacBio HiFi",
            ReadType::Ont => "ONT",
            ReadType::LongRead => "Långläsning",
            ReadType::Unknown => "Okänd typ",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentClass {
    pub path: String,
    pub name: String,
    pub read_type: ReadType,
    pub type_id: String,
    pub type_label: String,
    /// What the verdict was based on, e.g. "@PG STAR" or "@RG PL:NovaSeq".
    pub evidence: Option<String>,
    /// Platform as written in the header, when there is one.
    pub platform: Option<String>,
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

/// Classify from the platform field, which may hold either a spec value
/// (ILLUMINA, PACBIO, ONT) or the instrument name people actually write.
fn read_type_from_platform(platform: &str) -> Option<ReadType> {
    let p = platform.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| p.contains(n));

    if has(&["pacbio", "revio", "sequel", "smrt"]) {
        // CLR is the old noisy chemistry; anything else from PacBio is HiFi.
        return Some(if p.contains("clr") { ReadType::LongRead } else { ReadType::HiFi });
    }
    if has(&["ont", "nanopore", "promethion", "minion", "gridion"]) {
        return Some(ReadType::Ont);
    }
    if has(&[
        "illumina", "novaseq", "hiseq", "nextseq", "miseq", "iseq", "nova-x", "bgi", "mgi",
        "dnbseq", "element", "aviti", "ultima",
    ]) {
        return Some(ReadType::ShortRead);
    }
    None
}

/// Classify from the aligner name and its command line.
fn read_type_from_program(program: &str, command_line: &str) -> Option<ReadType> {
    let pn = program.to_ascii_lowercase();
    let cl = command_line.to_ascii_lowercase();

    // Spliced aligners mean RNA whatever the platform.
    if matches!(pn.as_str(), "star" | "hisat2" | "hisat" | "tophat" | "subread" | "segemehl") {
        return Some(ReadType::Rna);
    }
    if cl.contains("-ax splice") || cl.contains("-x splice") || cl.contains("--sjdbgtffile") {
        return Some(ReadType::Rna);
    }

    // minimap2 presets name the chemistry outright.
    if cl.contains("map-ont") || cl.contains("lr:hqae") {
        return Some(ReadType::Ont);
    }
    if cl.contains("map-hifi") || cl.contains("asm20") || pn == "pbmm2" {
        return Some(ReadType::HiFi);
    }
    if cl.contains("map-pb") {
        return Some(ReadType::LongRead);
    }
    if matches!(pn.as_str(), "dorado" | "guppy" | "minibar") {
        return Some(ReadType::Ont);
    }

    // flashmap takes the read type as its subcommand.
    if pn == "flashmap" {
        if contains_word(&cl, "rna") {
            return Some(ReadType::Rna);
        }
        if contains_word(&cl, "sr") {
            return Some(ReadType::ShortRead);
        }
        if contains_word(&cl, "lr") {
            return Some(if cl.contains("ont-like") { ReadType::Ont } else { ReadType::HiFi });
        }
    }

    if matches!(pn.as_str(), "bwa" | "bowtie2" | "bowtie" | "novoalign" | "snap" | "strobealign") {
        return Some(ReadType::ShortRead);
    }
    if matches!(pn.as_str(), "winnowmap" | "ngmlr" | "lra" | "rs-lra" | "rs_lra") {
        return Some(ReadType::LongRead);
    }

    None
}

/// Last resort: the names of the FASTQ files that went in.
fn read_type_from_filenames(fastqs: &[String]) -> Option<ReadType> {
    let joined = fastqs.join(" ").to_ascii_lowercase();
    if joined.is_empty() {
        return None;
    }
    if joined.contains("hifi") || joined.contains("ccs") {
        return Some(ReadType::HiFi);
    }
    if joined.contains("ont") || joined.contains("nanopore") || joined.contains("promethion") {
        return Some(ReadType::Ont);
    }
    if joined.contains("rna") || joined.contains("transcriptome") || joined.contains("cdna") {
        return Some(ReadType::Rna);
    }
    // A pair of read files is short-read sequencing in practice. This needs
    // two actual inputs, so it cannot fire on a single file name.
    if fastqs.len() >= 2 && (joined.contains("_r1") || joined.contains("_1.")) {
        return Some(ReadType::ShortRead);
    }
    None
}

/// Work out what kind of data a header describes, and say why.
pub fn classify_header(path: &str, header: &str) -> AlignmentClass {
    let prov = provenance_from_header(path, header);
    let mut platform = None;

    for line in header.lines() {
        if !line.starts_with("@RG") {
            continue;
        }
        for field in line.split('\t') {
            if let Some(v) = field.strip_prefix("PL:").or_else(|| field.strip_prefix("PM:")) {
                if !is_uninformative_id(v) {
                    platform.get_or_insert_with(|| v.to_string());
                }
            }
        }
    }

    if let Some(p) = &platform {
        if let Some(rt) = read_type_from_platform(p) {
            return build_class(path, prov, rt, Some(format!("@RG PL:{p}")), platform.clone());
        }
    }

    // Walk the @PG chain; the first informative program wins, which is the one
    // closest to the raw reads.
    for line in header.lines() {
        if !line.starts_with("@PG") {
            continue;
        }
        let mut program = String::new();
        let mut command_line = String::new();
        for field in line.split('\t') {
            if let Some(v) = field.strip_prefix("PN:") {
                program = v.to_string();
            } else if let Some(v) = field.strip_prefix("CL:") {
                command_line = v.to_string();
            }
        }
        if program.eq_ignore_ascii_case("samtools") {
            continue;
        }
        if let Some(rt) = read_type_from_program(&program, &command_line) {
            let evidence = if program.is_empty() {
                "@PG".to_string()
            } else {
                format!("@PG {program}")
            };

            // An aligner that handles both chemistries (rs-lra, winnowmap) only
            // says "long read". If the reads themselves are named hifi or ont,
            // that is more specific and does not contradict the aligner.
            if rt == ReadType::LongRead {
                if let Some(refined) = read_type_from_filenames(&prov.fastqs) {
                    if matches!(refined, ReadType::HiFi | ReadType::Ont) {
                        return build_class(
                            path,
                            prov,
                            refined,
                            Some(format!("{evidence} + filnamn")),
                            platform,
                        );
                    }
                }
            }

            return build_class(path, prov, rt, Some(evidence), platform);
        }
    }

    if let Some(rt) = read_type_from_filenames(&prov.fastqs) {
        let evidence = format!("filnamn: {}", prov.fastqs.join(", "));
        return build_class(path, prov, rt, Some(evidence), platform);
    }

    // Nothing in the header at all - common when the aligner ran in a pipe.
    // The file's own name is the only thing left, and people do name files
    // after the chemistry. Reported as a guess from the name, so it can be
    // told apart from something the header actually said.
    if let Some(rt) = read_type_from_filenames(std::slice::from_ref(&prov.name)) {
        let evidence = format!("gissat från filnamnet {}", prov.name);
        return build_class(path, prov, rt, Some(evidence), platform);
    }

    build_class(path, prov, ReadType::Unknown, None, platform)
}

fn build_class(
    path: &str,
    prov: BamProvenance,
    read_type: ReadType,
    evidence: Option<String>,
    platform: Option<String>,
) -> AlignmentClass {
    AlignmentClass {
        path: path.to_string(),
        name: prov.name,
        type_id: read_type.id().to_string(),
        type_label: read_type.label().to_string(),
        read_type,
        evidence,
        platform,
    }
}

/// Classify a set of alignments, for grouping them into virtual folders.
#[tauri::command]
pub async fn classify_alignments(paths: Vec<String>) -> Result<Vec<AlignmentClass>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let samtools = crate::bio_commands::find_tool_executable("samtools")
            .ok_or_else(|| "samtools hittades inte i dev/bin eller PATH".to_string())?;

        let paths: Vec<String> = paths
            .iter()
            .map(|p| resolve_path(p).to_string_lossy().to_string())
            .take(MAX_CANDIDATES)
            .collect();

        let classes = Mutex::new(Vec::new());
        let next = std::sync::atomic::AtomicUsize::new(0);

        std::thread::scope(|scope| {
            for _ in 0..8.min(paths.len().max(1)) {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(path) = paths.get(i) else {
                        return;
                    };
                    let class = match read_header(&samtools, path) {
                        Some(header) => classify_header(path, &header),
                        None => build_class(
                            path,
                            BamProvenance {
                                name: basename(path),
                                ..Default::default()
                            },
                            ReadType::Unknown,
                            None,
                            None,
                        ),
                    };
                    classes.lock().unwrap().push(class);
                });
            }
        });

        let mut classes = classes.into_inner().unwrap_or_default();
        classes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(classes)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAR_HEADER: &str = "@HD\tVN:1.6\tSO:coordinate\n\
@SQ\tSN:chr1\tLN:248956422\n\
@RG\tID:UPAL021_R\tSM:UPAL021\tLB:lib1\tPU:HWI-C1JT2.4.ATCACG\n\
@PG\tID:STAR\tPN:STAR\tVN:2.7.10a\tCL:STAR --runThreadN 20 --genomeDir /ref/star --readFilesIn prealignment/merged/UPAL021_R_fastq1.fastq.gz prealignment/merged/UPAL021_R_fastq2.fastq.gz --readFilesCommand zcat\n";

    // The classification rules below are written against headers taken from
    // real files, not invented ones.

    #[test]
    fn a_spliced_aligner_means_rna() {
        let c = classify_header("/data/UPAL021_R.bam", STAR_HEADER);
        assert_eq!(c.read_type, ReadType::Rna);
        assert_eq!(c.evidence.as_deref(), Some("@PG STAR"));
    }

    #[test]
    fn an_instrument_name_counts_as_a_platform() {
        // Real files write PL:NovaSeq, not the spec's PL:ILLUMINA.
        let header = "@RG\tID:D26.L001\tSM:D26\tPL:NovaSeq\tPU:22JYK7LT1.L001\n";
        let c = classify_header("/data/d26.bam", header);
        assert_eq!(c.read_type, ReadType::ShortRead);
        assert_eq!(c.platform.as_deref(), Some("NovaSeq"));
    }

    #[test]
    fn minimap2_presets_name_the_chemistry() {
        let ont = classify_header(
            "/d/a.bam",
            "@PG\tID:minimap2\tPN:minimap2\tCL:minimap2 -ax map-ont ref.fa reads.fq.gz\n",
        );
        assert_eq!(ont.read_type, ReadType::Ont);

        let hifi = classify_header(
            "/d/b.bam",
            "@PG\tID:minimap2\tPN:minimap2\tCL:minimap2 -ax map-hifi ref.fa reads.fq.gz\n",
        );
        assert_eq!(hifi.read_type, ReadType::HiFi);

        let rna = classify_header(
            "/d/c.bam",
            "@PG\tID:minimap2\tPN:minimap2\tCL:minimap2 -ax splice ref.fa reads.fq.gz\n",
        );
        assert_eq!(rna.read_type, ReadType::Rna);
    }

    #[test]
    fn flashmap_takes_the_read_type_from_its_subcommand() {
        let sr = classify_header(
            "/d/sr.bam",
            "@PG\tID:flashmap\tPN:flashmap\tCL:flashmap sr -i ref.fmi -1 a_R1.fastq.gz -2 a_R2.fastq.gz\n",
        );
        assert_eq!(sr.read_type, ReadType::ShortRead);

        let rna = classify_header(
            "/d/rna.bam",
            "@PG\tID:flashmap\tPN:flashmap\tCL:flashmap rna -i rna.fmi -1 x_1.fq.gz -2 x_2.fq.gz\n",
        );
        assert_eq!(rna.read_type, ReadType::Rna);

        let ont = classify_header(
            "/d/ont.bam",
            "@PG\tID:flashmap\tPN:flashmap\tCL:flashmap lr --ont-like -i ref.fmi reads.fq.gz\n",
        );
        assert_eq!(ont.read_type, ReadType::Ont);
    }

    #[test]
    fn samtools_is_skipped_when_looking_for_the_aligner() {
        // samtools wraps everything; the aligner underneath is what matters.
        let header = "@PG\tID:samtools\tPN:samtools\tCL:samtools sort -o out.bam -\n\
@PG\tID:bwa\tPN:bwa\tCL:bwa mem ref.fa a_R1.fq.gz a_R2.fq.gz\n";
        let c = classify_header("/d/out.bam", header);
        assert_eq!(c.read_type, ReadType::ShortRead);
        assert_eq!(c.evidence.as_deref(), Some("@PG bwa"));
    }

    #[test]
    fn a_long_read_aligner_is_refined_by_the_read_names() {
        // rs-lra handles both chemistries, so it only establishes "long read";
        // the reads say which one.
        let hifi = classify_header(
            "/d/a.bam",
            "@PG\tID:rs-lra\tPN:rs-lra\tVN:0.1.0\tCL:rs-lra -i ref.fmi -q hg002-hifi-chr20.fastq.gz -o a.bam\n",
        );
        assert_eq!(hifi.read_type, ReadType::HiFi);
        assert_eq!(hifi.evidence.as_deref(), Some("@PG rs-lra + filnamn"));

        // With nothing to refine it, long read is the honest answer.
        let plain = classify_header(
            "/d/b.bam",
            "@PG\tID:rs-lra\tPN:rs-lra\tVN:0.1.0\tCL:rs-lra -i ref.fmi -q reads.fastq.gz -o b.bam\n",
        );
        assert_eq!(plain.read_type, ReadType::LongRead);
    }

    #[test]
    fn file_names_are_the_last_resort() {
        let header = "@PG\tID:unknownaligner\tPN:unknownaligner\tCL:unknownaligner sample_hifi_ccs.fastq.gz\n";
        let c = classify_header("/d/x.bam", header);
        assert_eq!(c.read_type, ReadType::HiFi);
        assert!(c.evidence.unwrap().starts_with("filnamn"));
    }

    #[test]
    fn a_name_that_states_the_chemistry_is_used_as_a_last_guess() {
        // Real file here: hg002-hifi-chr20.fastq.rslra.bam, whose header holds
        // nothing but the sort step.
        let header = "@PG\tID:samtools\tPN:samtools\tCL:samtools sort -o out.bam -\n";
        let c = classify_header("/data/hg002-hifi-chr20.fastq.rslra.bam", header);
        assert_eq!(c.read_type, ReadType::HiFi);
        assert!(c.evidence.unwrap().starts_with("gissat från filnamnet"));
    }

    #[test]
    fn a_header_with_nothing_to_go_on_says_so() {
        // Very common in practice: the aligner ran in a pipe and only the
        // samtools sort step left a @PG line.
        let header = "@PG\tID:samtools\tPN:samtools\tCL:samtools sort -o HG002.bam -\n";
        let c = classify_header("/data/HG002.bam", header);
        assert_eq!(c.read_type, ReadType::Unknown);
        assert!(c.evidence.is_none());
    }

    #[test]
    fn reads_fastqs_and_read_groups_from_a_star_header() {
        let p = provenance_from_header("/data/UPAL021_R.bam", STAR_HEADER);
        assert_eq!(
            p.fastqs,
            vec!["UPAL021_R_fastq1.fastq.gz", "UPAL021_R_fastq2.fastq.gz"]
        );
        assert_eq!(p.sample.as_deref(), Some("UPAL021"));
        assert!(p.read_groups.contains(&"HWI-C1JT2.4.ATCACG".to_string()));
        assert!(p.source_bams.is_empty(), "no BAM in the STAR command line");
    }

    #[test]
    fn a_slice_records_the_bam_it_came_from() {
        let header = format!(
            "{STAR_HEADER}@PG\tID:samtools\tPN:samtools\tPP:STAR\tCL:samtools view -b /data/UPAL021_R.bam 12:6,534,517-6,580,000\n"
        );
        let slice = provenance_from_header("/data/gapdh.bam", &header);
        assert_eq!(slice.source_bams, vec!["UPAL021_R.bam"]);

        let parent = provenance_from_header("/data/UPAL021_R.bam", STAR_HEADER);
        let reasons = match_reasons(&parent, &slice);
        assert!(
            reasons.iter().any(|r| r.contains("Samma FASTQ")),
            "same reads: {reasons:?}"
        );
        assert!(
            reasons.iter().any(|r| r.contains("Härledd från")),
            "and derived from it: {reasons:?}"
        );
    }

    #[test]
    fn a_file_does_not_count_itself_as_its_own_source() {
        let header = "@PG\tID:samtools\tCL:samtools sort -o mine.bam mine.bam\n";
        let p = provenance_from_header("/data/mine.bam", header);
        assert!(p.source_bams.is_empty());
    }

    #[test]
    fn gatks_artificial_read_group_is_not_evidence() {
        // Seen in a real file: GATK adds this to everything it calls variants
        // on, so matching on it would relate every such BAM to every other.
        let gatk = |sample: &str, flowcell: &str| {
            format!(
                "@RG\tID:{sample}.L001.TGTT\tSM:{sample}\tPL:NovaSeq\tPU:{flowcell}.L001.TGTT\tLB:{sample}\n\
                 @RG\tID:ArtificialHaplotypeRG\tSM:HC\tCN:BI\n"
            )
        };

        let a = provenance_from_header("/data/a.bam", &gatk("D26-04588_T", "22JYK7LT1"));
        let b = provenance_from_header("/data/b.bam", &gatk("D26-99999_N", "9XXXXLT2"));
        assert!(
            match_reasons(&a, &b).is_empty(),
            "two unrelated GATK outputs must not match: {:?}",
            match_reasons(&a, &b)
        );

        // The real per-lane read groups still count.
        let same_run = provenance_from_header("/data/c.bam", &gatk("D26-04588_T", "22JYK7LT1"));
        assert!(match_reasons(&a, &same_run)
            .iter()
            .any(|r| r.contains("Samma läsgrupp")));
    }

    #[test]
    fn a_numeric_read_group_is_too_common_to_mean_anything() {
        let a = provenance_from_header("/a.bam", "@RG\tID:1\tSM:tumour\n");
        let b = provenance_from_header("/b.bam", "@RG\tID:1\tSM:normal\n");
        assert!(match_reasons(&a, &b).is_empty());
    }

    #[test]
    fn unrelated_alignments_do_not_match() {
        let a = provenance_from_header("/data/a.bam", STAR_HEADER);
        let other = "@RG\tID:other\tSM:OTHER\tPU:FLOWCELL2.1\n\
@PG\tID:bwa\tCL:bwa mem ref.fa other_R1.fastq.gz other_R2.fastq.gz\n";
        let b = provenance_from_header("/data/b.bam", other);
        assert!(match_reasons(&a, &b).is_empty());
    }

    #[test]
    fn the_same_sample_alone_is_reported_as_the_weak_match_it_is() {
        let a = provenance_from_header("/data/a.bam", STAR_HEADER);
        let rerun = "@RG\tID:run2\tSM:UPAL021\tPU:FLOWCELL9.2\n\
@PG\tID:bwa\tCL:bwa mem ref.fa other_R1.fastq.gz\n";
        let b = provenance_from_header("/data/b.bam", rerun);

        let reasons = match_reasons(&a, &b);
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("Samma prov"), "{reasons:?}");
    }

    #[test]
    fn bwa_style_command_lines_are_understood_too() {
        let header = "@PG\tID:bwa\tPN:bwa\tCL:bwa mem -t 8 ref.fa reads_R1.fq.gz reads_R2.fq.gz\n";
        let p = provenance_from_header("/data/x.bam", header);
        assert_eq!(p.fastqs, vec!["reads_R1.fq.gz", "reads_R2.fq.gz"]);
    }
}
