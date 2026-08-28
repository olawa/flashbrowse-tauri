use crate::fs_commands::{format_byte_size, get_permissions_string, resolve_path};
use crate::models::PreviewContent;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Local};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};

fn detect_language(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "py" | "pyw" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "svelte" => "svelte",
        "html" | "htm" => "html",
        "css" | "scss" | "sass" | "less" => "css",
        "json" | "jsonc" => "json",
        "md" | "markdown" => "markdown",
        "sh" | "bash" | "zsh" => "shell",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "c" | "h" => "c",
        "cpp" | "cxx" | "cc" | "hpp" => "cpp",
        "swift" => "swift",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "r" | "rmd" => "r",
        "sql" => "sql",
        "xml" | "plist" => "xml",
        "dockerfile" => "dockerfile",
        "vcf" | "bed" | "sam" | "gtf" | "gff" => "shell",
        _ => "plaintext",
    }
}

fn is_image_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "bmp" => Some("image/bmp"),
        "ico" => Some("image/x-icon"),
        _ => None,
    }
}

fn is_video_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "ogv" => Some("video/ogg"),
        _ => None,
    }
}

fn is_audio_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "flac" => Some("audio/flac"),
        "m4a" | "aac" => Some("audio/mp4"),
        _ => None,
    }
}

fn format_hex_dump(bytes: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let offset = format!("{:08x}", i * 16);
        let mut hex_parts = Vec::new();
        let mut ascii_parts = String::new();

        for &b in chunk {
            hex_parts.push(format!("{:02x}", b));
            if b >= 32 && b <= 126 {
                ascii_parts.push(b as char);
            } else {
                ascii_parts.push('.');
            }
        }

        while hex_parts.len() < 16 {
            hex_parts.push("  ".to_string());
        }

        let hex_str = format!("{}  {}", hex_parts[..8].join(" "), hex_parts[8..].join(" "));
        lines.push(format!("{}  {}  |{}|", offset, hex_str, ascii_parts));
    }
    lines
}

fn parse_table_preview(content: &str, delimiter: char) -> (Vec<String>, Vec<Vec<String>>) {
    let mut lines = content.lines().take(50);
    let headers = if let Some(first_line) = lines.next() {
        first_line
            .split(delimiter)
            .map(|s| s.trim_matches('"').trim().to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut rows = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<String> = line
            .split(delimiter)
            .map(|s| s.trim_matches('"').trim().to_string())
            .collect();
        rows.push(cols);
    }

    (headers, rows)
}

#[tauri::command]
pub fn get_preview(path: &str, max_bytes: Option<usize>) -> Result<PreviewContent, String> {
    let resolved_path = resolve_path(path);

    if !resolved_path.exists() {
        return Err(format!("Path does not exist: {}", resolved_path.display()));
    }

    let metadata = fs::metadata(&resolved_path).map_err(|e| e.to_string())?;
    let file_size_bytes = metadata.len();
    let formatted_size = format_byte_size(file_size_bytes);

    let modified_time = metadata
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let dt: DateTime<Local> = modified_time.into();
    let modified_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();
    let permissions_str = get_permissions_string(&metadata);

    if resolved_path.is_dir() {
        return Ok(PreviewContent {
            kind: "directory".to_string(),
            text_content: None,
            html_content: None,
            pdf_base64: None,
            media_base64: None,
            media_mime: None,
            language: None,
            line_count: None,
            image_base64: None,
            image_mime: None,
            table_headers: None,
            table_rows: None,
            hex_lines: None,
            file_size_bytes,
            formatted_size: "--".to_string(),
            modified_str,
            permissions_str,
            error_message: None,
        });
    }

    let ext = resolved_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let file_path = resolved_path.as_path();

    // 1. HTML / MultiQC / FastQC Report Preview
    if ext == "html" || ext == "htm" {
        let max_html = 10 * 1024 * 1024; // 10 MB
        let mut file = File::open(file_path).map_err(|e| e.to_string())?;
        let to_read = (file_size_bytes as usize).min(max_html);
        let mut buffer = vec![0u8; to_read];
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        buffer.truncate(bytes_read);

        if let Ok(text) = String::from_utf8(buffer) {
            return Ok(PreviewContent {
                kind: "html".to_string(),
                text_content: Some(text.clone()),
                html_content: Some(text),
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("html".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 2. PDF Document Preview
    if ext == "pdf" {
        if file_size_bytes > 50 * 1024 * 1024 {
            return Ok(PreviewContent {
                kind: "too_large".to_string(),
                text_content: None,
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("pdf".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: Some("PDF file is too large (>50 MB) for instant preview".to_string()),
            });
        }

        let bytes = fs::read(file_path).map_err(|e| e.to_string())?;
        let pdf_base64 = BASE64.encode(&bytes);

        // Generate small hex sample for raw view toggle
        let hex_sample = &bytes[..bytes.len().min(512)];
        let hex_lines = format_hex_dump(hex_sample);

        return Ok(PreviewContent {
            kind: "pdf".to_string(),
            text_content: None,
            html_content: None,
            pdf_base64: Some(pdf_base64),
            media_base64: None,
            media_mime: Some("application/pdf".to_string()),
            language: Some("pdf".to_string()),
            line_count: None,
            image_base64: None,
            image_mime: None,
            table_headers: None,
            table_rows: None,
            hex_lines: Some(hex_lines),
            file_size_bytes,
            formatted_size,
            modified_str,
            permissions_str,
            error_message: None,
        });
    }

    // 3. Markdown Preview
    if ext == "md" || ext == "markdown" {
        let max_md = 4 * 1024 * 1024;
        let mut file = File::open(file_path).map_err(|e| e.to_string())?;
        let to_read = (file_size_bytes as usize).min(max_md);
        let mut buffer = vec![0u8; to_read];
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        buffer.truncate(bytes_read);

        if let Ok(text) = String::from_utf8(buffer) {
            let line_count = text.lines().count();
            return Ok(PreviewContent {
                kind: "markdown".to_string(),
                text_content: Some(text),
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("markdown".to_string()),
                line_count: Some(line_count),
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 4. Video Preview
    if let Some(mime) = is_video_ext(&ext) {
        if file_size_bytes <= 60 * 1024 * 1024 {
            let bytes = fs::read(file_path).map_err(|e| e.to_string())?;
            let b64 = BASE64.encode(&bytes);
            return Ok(PreviewContent {
                kind: "video".to_string(),
                text_content: None,
                html_content: None,
                pdf_base64: None,
                media_base64: Some(b64),
                media_mime: Some(mime.to_string()),
                language: Some("video".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 5. Audio Preview
    if let Some(mime) = is_audio_ext(&ext) {
        if file_size_bytes <= 40 * 1024 * 1024 {
            let bytes = fs::read(file_path).map_err(|e| e.to_string())?;
            let b64 = BASE64.encode(&bytes);
            return Ok(PreviewContent {
                kind: "audio".to_string(),
                text_content: None,
                html_content: None,
                pdf_base64: None,
                media_base64: Some(b64),
                media_mime: Some(mime.to_string()),
                language: Some("audio".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 6. Image Preview (including SVG)
    if let Some(mime) = is_image_ext(&ext) {
        if file_size_bytes > 20 * 1024 * 1024 {
            return Ok(PreviewContent {
                kind: "too_large".to_string(),
                text_content: None,
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: None,
                line_count: None,
                image_base64: None,
                image_mime: Some(mime.to_string()),
                table_headers: None,
                table_rows: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: Some("Image is too large (>20 MB) for instant preview".to_string()),
            });
        }

        let bytes = fs::read(file_path).map_err(|e| e.to_string())?;
        let base64_str = BASE64.encode(&bytes);
        let text_content = if ext == "svg" {
            String::from_utf8(bytes.clone()).ok()
        } else {
            None
        };

        return Ok(PreviewContent {
            kind: if ext == "svg" { "svg".to_string() } else { "image".to_string() },
            text_content,
            html_content: None,
            pdf_base64: None,
            media_base64: None,
            media_mime: None,
            language: if ext == "svg" { Some("svg".to_string()) } else { None },
            line_count: None,
            image_base64: Some(base64_str),
            image_mime: Some(mime.to_string()),
            table_headers: None,
            table_rows: None,
            hex_lines: None,
            file_size_bytes,
            formatted_size,
            modified_str,
            permissions_str,
            error_message: None,
        });
    }

    // 7. CSV / TSV Table Preview
    if ext == "csv" || ext == "tsv" || ext == "tab" {
        let max_read = 512 * 1024; // 512 KB
        let mut file = File::open(file_path).map_err(|e| e.to_string())?;
        let mut buffer = vec![0u8; max_read.min(file_size_bytes as usize)];
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        buffer.truncate(bytes_read);

        if let Ok(text) = String::from_utf8(buffer) {
            let delimiter = if ext == "csv" { ',' } else { '\t' };
            let (headers, rows) = parse_table_preview(&text, delimiter);
            if !headers.is_empty() {
                return Ok(PreviewContent {
                    kind: "table".to_string(),
                    text_content: Some(text),
                    html_content: None,
                    pdf_base64: None,
                    media_base64: None,
                    media_mime: None,
                    language: Some("table".to_string()),
                    line_count: Some(rows.len() + 1),
                    image_base64: None,
                    image_mime: None,
                    table_headers: Some(headers),
                    table_rows: Some(rows),
                    hex_lines: None,
                    file_size_bytes,
                    formatted_size,
                    modified_str,
                    permissions_str,
                    error_message: None,
                });
            }
        }
    }

    // 8. Text / Code Preview
    let max_read_limit = max_bytes.unwrap_or(512 * 1024); // 512 KB
    let mut file = File::open(file_path).map_err(|e| e.to_string())?;
    let to_read = (file_size_bytes as usize).min(max_read_limit);
    let mut buffer = vec![0u8; to_read];
    let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
    buffer.truncate(bytes_read);

    // Check if binary (contains null bytes)
    let is_binary = buffer.iter().take(1024).any(|&b| b == 0);

    if !is_binary {
        if let Ok(text) = String::from_utf8(buffer) {
            let line_count = text.lines().count();
            let lang = detect_language(&ext);
            return Ok(PreviewContent {
                kind: "code".to_string(),
                text_content: Some(text),
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some(lang.to_string()),
                line_count: Some(line_count),
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 9. Binary Hex Preview fallback
    let hex_sample_size = (file_size_bytes as usize).min(512);
    let mut hex_buf = vec![0u8; hex_sample_size];
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let read_hex = file.read(&mut hex_buf).map_err(|e| e.to_string())?;
    hex_buf.truncate(read_hex);

    let hex_lines = format_hex_dump(&hex_buf);

    Ok(PreviewContent {
        kind: "hex".to_string(),
        text_content: None,
        html_content: None,
        pdf_base64: None,
        media_base64: None,
        media_mime: None,
        language: Some("hex".to_string()),
        line_count: Some(hex_lines.len()),
        image_base64: None,
        image_mime: None,
        table_headers: None,
        table_rows: None,
        hex_lines: Some(hex_lines),
        file_size_bytes,
        formatted_size,
        modified_str,
        permissions_str,
        error_message: None,
    })
}
