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
    pub kind: String, // "code", "text", "image", "table", "hex", "too_large", "error"
    pub text_content: Option<String>,
    pub language: Option<String>,
    pub line_count: Option<usize>,
    pub image_base64: Option<String>,
    pub image_mime: Option<String>,
    pub table_headers: Option<Vec<String>>,
    pub table_rows: Option<Vec<Vec<String>>>,
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
