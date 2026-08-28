use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size_bytes: u64,
    pub formatted_size: String,
    pub modified_timestamp: i64,
    pub formatted_modified: String,
    pub extension: String,
    pub is_hidden: bool,
    pub permissions: String,
    pub item_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub formatted_total: String,
    pub formatted_available: String,
    pub formatted_used: String,
    pub percentage_used: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewContent {
    pub kind: String, // "code", "text", "html", "pdf", "markdown", "image", "svg", "video", "audio", "table", "notebook", "hex", "too_large", "error"
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub pdf_base64: Option<String>,
    pub media_base64: Option<String>,
    pub media_mime: Option<String>,
    pub language: Option<String>,
    pub language_name: Option<String>,
    pub language_emoji: Option<String>,
    pub line_count: Option<usize>,
    pub image_base64: Option<String>,
    pub image_mime: Option<String>,
    pub table_headers: Option<Vec<String>>,
    pub table_rows: Option<Vec<Vec<String>>>,
    pub sheet_names: Option<Vec<String>>,
    pub hex_lines: Option<Vec<String>>,
    pub file_size_bytes: u64,
    pub formatted_size: String,
    pub modified_str: String,
    pub permissions_str: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub new_cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabCompletionResult {
    pub completed_line: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorySummary {
    pub path: String,
    pub total_items: usize,
    pub total_dirs: usize,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub formatted_total_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContigInfo {
    pub name: String,
    pub length: u64,
    pub formatted_length: String,
    pub assembly: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadGroupInfo {
    pub id: String,
    pub sample: Option<String>,
    pub platform: Option<String>,
    pub library: Option<String>,
    pub center: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInfo {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub command_line: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamHeaderData {
    pub detected_reference: String, // e.g. "GRCh38 / hg38", "GRCh37 / hs37d5", "mm10", "Unknown"
    pub reference_matched_path: Option<String>,
    pub contigs: Vec<ContigInfo>,
    pub total_contigs: usize,
    pub total_genome_length: u64,
    pub formatted_genome_length: String,
    pub read_groups: Vec<ReadGroupInfo>,
    pub programs: Vec<ProgramInfo>,
    pub raw_header: String,
    pub has_index: bool,
    pub index_type: Option<String>, // "BAI" or "CRAI" or "CSI"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub name: String,
    pub size_bytes: u64,
    pub formatted_size: String,
    pub is_dir: bool,
    pub modified_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveSummary {
    pub path: String,
    pub entries: Vec<ArchiveEntry>,
    pub total_files: usize,
    pub total_uncompressed_bytes: u64,
    pub formatted_uncompressed_size: String,
}
