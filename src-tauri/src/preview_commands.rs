use crate::fs_commands::{format_byte_size, get_permissions_string, resolve_path};
use crate::models::PreviewContent;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Local};
use flate2::read::MultiGzDecoder;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

fn decompress_gz_head(file_path: &Path, max_uncompressed_bytes: usize) -> Result<Vec<u8>, String> {
    let file = File::open(file_path).map_err(|e| e.to_string())?;
    let mut decoder = MultiGzDecoder::new(file);
    let mut buffer = vec![0u8; max_uncompressed_bytes];
    let mut total_read = 0;

    while total_read < max_uncompressed_bytes {
        match decoder.read(&mut buffer[total_read..]) {
            Ok(0) => break,
            Ok(n) => total_read += n,
            Err(e) => {
                if total_read > 0 {
                    break;
                }
                return Err(e.to_string());
            }
        }
    }
    buffer.truncate(total_read);
    Ok(buffer)
}

pub(crate) fn detect_language_meta(filename: &str, code: &str) -> (&'static str, &'static str, &'static str) {
    let lower = filename.to_lowercase();
    let ext = Path::new(filename)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Exact filename matching
    if lower == "makefile" || lower.starts_with("makefile.") {
        return ("makefile", "Makefile", "🔨");
    }
    if lower == "dockerfile" || lower.starts_with("dockerfile.") {
        return ("dockerfile", "Dockerfile", "🐳");
    }
    if lower == "snakefile" || lower.ends_with(".smk") {
        return ("python", "Snakemake", "🐍");
    }
    if lower == "gemfile" || lower == "rakefile" {
        return ("ruby", "Ruby", "💎");
    }
    if lower.starts_with(".bash") || lower.starts_with(".zsh") || lower == ".profile" {
        return ("shell", "Shell Script", "🐚");
    }
    if lower == ".env" || lower.starts_with(".env.") {
        return ("ini", "Environment Config", "⚙️");
    }
    if lower == ".gitignore" || lower == ".dockerignore" {
        return ("plaintext", "Ignore Config", "🚫");
    }

    match ext.as_str() {
        "py" | "pyw" => ("python", "Python", "🐍"),
        "sh" | "bash" | "zsh" | "fish" | "command" => ("shell", "Shell Script", "🐚"),
        "r" | "rmd" => ("r", "R Script", "📊"),
        "swift" => ("swift", "Swift", "⚡"),
        "rs" => ("rust", "Rust", "🦀"),
        "c" | "h" => ("c", "C Code", "🇨"),
        "cpp" | "hpp" | "cc" | "cxx" | "c++" | "h++" => ("cpp", "C++ Code", "⚙️"),
        "go" => ("go", "Go", "🐹"),
        "js" | "mjs" | "cjs" => ("javascript", "JavaScript", "🟨"),
        "jsx" => ("javascript", "React JSX", "⚛️"),
        "ts" | "mts" | "cts" => ("typescript", "TypeScript", "🟦"),
        "tsx" => ("typescript", "React TSX", "⚛️"),
        "svelte" => ("svelte", "Svelte Component", "🔥"),
        "json" | "jsonl" | "geojson" => ("json", "JSON Data", "📦"),
        "yaml" | "yml" => ("yaml", "YAML Config", "📄"),
        "toml" => ("toml", "TOML Config", "⚙️"),
        "ini" | "cfg" | "conf" | "config" => ("ini", "Config File", "⚙️"),
        "sql" => ("sql", "SQL Query", "🗄️"),
        "html" | "htm" | "xhtml" => ("html", "HTML Document", "🌐"),
        "xml" | "plist" | "kml" => ("xml", "XML Data", "📑"),
        "svg" => ("xml", "SVG Vector", "🎨"),
        "css" => ("css", "CSS Stylesheet", "🎨"),
        "scss" | "sass" | "less" => ("scss", "SCSS Stylesheet", "🎨"),
        "md" | "markdown" => ("markdown", "Markdown", "📝"),
        "java" => ("java", "Java", "☕"),
        "kt" | "kts" => ("kotlin", "Kotlin", "🟣"),
        "lua" => ("lua", "Lua", "🌙"),
        "pl" | "pm" => ("perl", "Perl", "🐪"),
        "rb" => ("ruby", "Ruby", "💎"),
        "php" => ("php", "PHP", "🐘"),
        "fasta" | "fa" | "fna" | "faa" => ("plaintext", "FASTA Sequence", "🧬"),
        "bed" => ("plaintext", "BED Genomic Regions", "🧬"),
        "gtf" | "gff" | "gff3" => ("plaintext", "GTF/GFF Annotation", "🧬"),
        "vcf" => ("plaintext", "VCF Variants", "🧬"),
        "sam" => ("plaintext", "SAM Alignment", "🧬"),
        "log" => ("plaintext", "Log File", "📋"),
        "txt" | "text" => ("plaintext", "Plain Text", "📄"),
        _ => {
            if code.starts_with("#!") {
                let first_line = code.lines().next().unwrap_or("").to_lowercase();
                if first_line.contains("python") {
                    return ("python", "Python", "🐍");
                }
                if first_line.contains("bash") || first_line.contains("sh") || first_line.contains("zsh") {
                    return ("shell", "Shell Script", "🐚");
                }
                if first_line.contains("rscript") {
                    return ("r", "R Script", "📊");
                }
                if first_line.contains("node") {
                    return ("javascript", "JavaScript", "🟨");
                }
            }
            ("plaintext", "Plain Text", "📄")
        }
    }
}

fn parse_excel_preview(file_path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>, Vec<String>), String> {
    use calamine::{open_workbook_auto, Reader, Data};

    let mut workbook = open_workbook_auto(file_path).map_err(|e| e.to_string())?;
    let sheet_names = workbook.sheet_names().to_vec();

    if let Some(first_sheet_name) = sheet_names.first().cloned() {
        if let Ok(range) = workbook.worksheet_range(&first_sheet_name) {
            let mut headers = Vec::new();
            let mut rows = Vec::new();

            for (row_idx, row) in range.rows().enumerate() {
                if row_idx > 300 { break; } // Up to 300 rows for preview
                
                let string_cells: Vec<String> = row.iter().map(|cell| {
                    match cell {
                        Data::Empty => String::new(),
                        Data::String(s) => s.clone(),
                        Data::Float(f) => {
                            if f.fract() == 0.0 && *f >= -1e15 && *f <= 1e15 {
                                format!("{:.0}", f)
                            } else {
                                format!("{:.4}", f).trim_end_matches('0').trim_end_matches('.').to_string()
                            }
                        },
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(d) => format!("{:.2}", d),
                        Data::Error(e) => format!("{:?}", e),
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                    }
                }).collect();

                if row_idx == 0 {
                    headers = string_cells;
                } else if string_cells.iter().any(|c| !c.is_empty()) {
                    rows.push(string_cells);
                }
            }

            return Ok((headers, rows, sheet_names));
        }
    }

    Err("No sheet found in workbook".to_string())
}

fn parse_document_html(file_path: &Path) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/usr/bin/textutil")
            .args(["-convert", "html", "-stdout", &file_path.to_string_lossy()])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            let html = String::from_utf8_lossy(&output.stdout).to_string();
            // Wrap in pleasant container styling
            let styled_html = format!(
                r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>
                body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; padding: 24px; color: #1e293b; background: #ffffff; line-height: 1.6; max-width: 800px; margin: 0 auto; }}
                h1, h2, h3, h4 {{ color: #0f172a; margin-top: 1.2em; margin-bottom: 0.5em; }}
                table {{ border-collapse: collapse; width: 100%; margin: 16px 0; }}
                td, th {{ border: 1px solid #cbd5e1; padding: 8px 12px; text-align: left; }}
                p {{ margin: 0.8em 0; }}
                </style></head><body>{}</body></html>"#,
                html
            );
            return Ok(styled_html);
        } else {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(err);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Document HTML preview requires macOS textutil".to_string())
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

pub(crate) fn parse_table_preview(content: &str, delimiter: char) -> (Vec<String>, Vec<Vec<String>>) {
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
            language_name: None,
            language_emoji: None,
            line_count: None,
            image_base64: None,
            image_mime: None,
            table_headers: None,
            table_rows: None,
            sheet_names: None,
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
    let filename = resolved_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let file_path = resolved_path.as_path();

    // 1. Excel / OpenDocument Spreadsheets (.xlsx, .xls, .ods, .xlsb)
    if ext == "xlsx" || ext == "xls" || ext == "ods" || ext == "xlsb" {
        if let Ok((headers, rows, sheet_names)) = parse_excel_preview(file_path) {
            return Ok(PreviewContent {
                kind: "table".to_string(),
                text_content: None,
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("excel".to_string()),
                language_name: Some("Excel Spreadsheet".to_string()),
                language_emoji: Some("📊".to_string()),
                line_count: Some(rows.len() + 1),
                image_base64: None,
                image_mime: None,
                table_headers: Some(headers),
                table_rows: Some(rows),
                sheet_names: Some(sheet_names),
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 2. Word & Rich Office Documents (.docx, .doc, .rtf, .rtfd, .odt)
    if ext == "docx" || ext == "doc" || ext == "rtf" || ext == "rtfd" || ext == "odt" || ext == "webarchive" {
        if let Ok(html) = parse_document_html(file_path) {
            return Ok(PreviewContent {
                kind: "html".to_string(),
                text_content: None,
                html_content: Some(html),
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("document".to_string()),
                language_name: Some(if ext == "doc" || ext == "docx" { "Word Document".to_string() } else { "Rich Document".to_string() }),
                language_emoji: Some("📄".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 3. Jupyter Notebooks (.ipynb)
    if ext == "ipynb" {
        let max_nb = 5 * 1024 * 1024;
        let mut file = File::open(file_path).map_err(|e| e.to_string())?;
        let to_read = (file_size_bytes as usize).min(max_nb);
        let mut buffer = vec![0u8; to_read];
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        buffer.truncate(bytes_read);

        if let Ok(text) = String::from_utf8(buffer) {
            return Ok(PreviewContent {
                kind: "notebook".to_string(),
                text_content: Some(text),
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("python".to_string()),
                language_name: Some("Jupyter Notebook".to_string()),
                language_emoji: Some("🪐".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 4. HTML / MultiQC / FastQC Report Preview
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
                language_name: Some("HTML Document".to_string()),
                language_emoji: Some("🌐".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 5. PDF Document Preview
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
                language_name: Some("PDF Document".to_string()),
                language_emoji: Some("📄".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
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
            language_name: Some("PDF Document".to_string()),
            language_emoji: Some("📄".to_string()),
            line_count: None,
            image_base64: None,
            image_mime: None,
            table_headers: None,
            table_rows: None,
            sheet_names: None,
            hex_lines: Some(hex_lines),
            file_size_bytes,
            formatted_size,
            modified_str,
            permissions_str,
            error_message: None,
        });
    }

    // 6. Markdown Preview
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
                language_name: Some("Markdown".to_string()),
                language_emoji: Some("📝".to_string()),
                line_count: Some(line_count),
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 7. Video Preview
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
                language_name: Some("Video File".to_string()),
                language_emoji: Some("🎬".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 8. Audio Preview
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
                language_name: Some("Audio File".to_string()),
                language_emoji: Some("🎵".to_string()),
                line_count: None,
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 9. Image Preview (including SVG)
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
                language_name: None,
                language_emoji: None,
                line_count: None,
                image_base64: None,
                image_mime: Some(mime.to_string()),
                table_headers: None,
                table_rows: None,
                sheet_names: None,
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
            language_name: if ext == "svg" { Some("SVG Vector".to_string()) } else { Some("Image".to_string()) },
            language_emoji: if ext == "svg" { Some("🎨".to_string()) } else { Some("🖼️".to_string()) },
            line_count: None,
            image_base64: Some(base64_str),
            image_mime: Some(mime.to_string()),
            table_headers: None,
            table_rows: None,
            sheet_names: None,
            hex_lines: None,
            file_size_bytes,
            formatted_size,
            modified_str,
            permissions_str,
            error_message: None,
        });
    }

    // 10. CSV / TSV Table Preview
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
                    language_name: Some(if ext == "csv" { "CSV Table".to_string() } else { "TSV Table".to_string() }),
                    language_emoji: Some("📊".to_string()),
                    line_count: Some(rows.len() + 1),
                    image_base64: None,
                    image_mime: None,
                    table_headers: Some(headers),
                    table_rows: Some(rows),
                    sheet_names: None,
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

    // 11. Gzip / Bgzip Compressed Files (.gz, .bgz, .vcf.gz, .fastq.gz, .tsv.gz, etc.)
    if ext == "gz" || ext == "bgz" || filename.ends_with(".gz") || filename.ends_with(".bgz") {
        let max_decompressed = max_bytes.unwrap_or(512 * 1024);
        if let Ok(uncompressed_bytes) = decompress_gz_head(file_path, max_decompressed) {
            let is_binary = uncompressed_bytes.iter().take(1024).any(|&b| b == 0);
            if !is_binary {
                if let Ok(text) = String::from_utf8(uncompressed_bytes) {
                    let inner_filename = if filename.ends_with(".gz") {
                        &filename[..filename.len() - 3]
                    } else if filename.ends_with(".bgz") {
                        &filename[..filename.len() - 4]
                    } else {
                        &filename
                    };

                    let inner_ext = Path::new(inner_filename)
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    // If it's a gzipped CSV/TSV table
                    if inner_ext == "csv" || inner_ext == "tsv" || inner_ext == "tab" {
                        let delimiter = if inner_ext == "csv" { ',' } else { '\t' };
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
                                language_name: Some(format!("{} (gzip)", if inner_ext == "csv" { "CSV Table" } else { "TSV Table" })),
                                language_emoji: Some("📊".to_string()),
                                line_count: Some(rows.len() + 1),
                                image_base64: None,
                                image_mime: None,
                                table_headers: Some(headers),
                                table_rows: Some(rows),
                                sheet_names: None,
                                hex_lines: None,
                                file_size_bytes,
                                formatted_size,
                                modified_str,
                                permissions_str,
                                error_message: None,
                            });
                        }
                    }

                    // For FASTQ / VCF / BED / GTF / Code / Text
                    let (lang_id, lang_name, lang_emoji) = detect_language_meta(inner_filename, &text);
                    let line_count = text.lines().count();
                    return Ok(PreviewContent {
                        kind: "code".to_string(),
                        text_content: Some(text),
                        html_content: None,
                        pdf_base64: None,
                        media_base64: None,
                        media_mime: None,
                        language: Some(lang_id.to_string()),
                        language_name: Some(format!("{} (gzip)", lang_name)),
                        language_emoji: Some(lang_emoji.to_string()),
                        line_count: Some(line_count),
                        image_base64: None,
                        image_mime: None,
                        table_headers: None,
                        table_rows: None,
                        sheet_names: None,
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
    }

    // 12. Text / Code Preview (with 100+ language detection!)
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
            let (lang_id, lang_name, lang_emoji) = detect_language_meta(&filename, &text);
            return Ok(PreviewContent {
                kind: "code".to_string(),
                text_content: Some(text),
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some(lang_id.to_string()),
                language_name: Some(lang_name.to_string()),
                language_emoji: Some(lang_emoji.to_string()),
                line_count: Some(line_count),
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str,
                error_message: None,
            });
        }
    }

    // 13. Binary File Preview (Do NOT dump hex dump by default, return kind: "binary")
    let hex_sample_size = (file_size_bytes as usize).min(512);
    let mut hex_buf = vec![0u8; hex_sample_size];
    let _ = file.seek(SeekFrom::Start(0));
    let read_hex = file.read(&mut hex_buf).unwrap_or(0);
    hex_buf.truncate(read_hex);

    let hex_lines = format_hex_dump(&hex_buf);

    Ok(PreviewContent {
        kind: "binary".to_string(),
        text_content: None,
        html_content: None,
        pdf_base64: None,
        media_base64: None,
        media_mime: None,
        language: Some("binary".to_string()),
        language_name: Some(format!("Binär fil (.{ext})")),
        language_emoji: Some("📦".to_string()),
        line_count: None,
        image_base64: None,
        image_mime: None,
        table_headers: None,
        table_rows: None,
        sheet_names: None,
        hex_lines: Some(hex_lines),
        file_size_bytes,
        formatted_size,
        modified_str,
        permissions_str,
        error_message: None,
    })
}
