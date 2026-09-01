use crate::fs_commands::format_byte_size;
use crate::models::{FileItem, PreviewContent, TerminalOutput};
use std::path::Path;
use std::process::Command;

pub fn ssh_base_args() -> Vec<&'static str> {
    vec![
        "-o", "ControlMaster=auto",
        "-o", "ControlPath=/tmp/fb_ssh_%h_%p_%r",
        "-o", "ControlPersist=15m",
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=15",
        "-o", "ServerAliveInterval=15",
        "-o", "ServerAliveCountMax=3",
        "-o", "StrictHostKeyChecking=accept-new",
    ]
}

pub fn scp_base_args() -> Vec<&'static str> {
    vec![
        "-o", "ControlMaster=auto",
        "-o", "ControlPath=/tmp/fb_ssh_%h_%p_%r",
        "-o", "ControlPersist=15m",
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=15",
        "-o", "StrictHostKeyChecking=accept-new",
    ]
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SshDirectoryResult {
    pub current_path: String,
    pub items: Vec<FileItem>,
}

#[tauri::command]
pub async fn ssh_list_directory(host: String, path: String) -> Result<SshDirectoryResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let remote_script = if path.is_empty() || path == "~" {
            "cd ~ && pwd && ls -la".to_string()
        } else if path.starts_with("~/") {
            let rest = &path[2..].replace('\'', "'\\''");
            format!("cd \"${{HOME}}/{}\" && pwd && ls -la", rest)
        } else {
            let escaped_path = path.replace('\'', "'\\''");
            format!("cd '{}' && pwd && ls -la", escaped_path)
        };

        let mut args = ssh_base_args();
        args.push(&host);
        args.push(&remote_script);

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .map_err(|e| format!("Kunde inte starta SSH-klient: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let msg = if err.is_empty() {
                format!("SSH-anslutning misslyckades till {} (kod {:?})", host, output.status.code())
            } else {
                format!("SSH-fel ({host}): {err}")
            };
            return Err(msg);
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout_str.lines();

        // First line is canonical pwd from remote server
        let current_pwd = lines.next().unwrap_or("~").trim().to_string();

        let mut items = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("total ") || trimmed.is_empty() {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 9 {
                continue;
            }

            let permissions = parts[0].to_string();
            let is_dir = permissions.starts_with('d');
            let is_symlink = permissions.starts_with('l');

            let size_bytes: u64 = parts[4].parse().unwrap_or(0);
            let formatted_size = if is_dir {
                "--".to_string()
            } else {
                format_byte_size(size_bytes)
            };

            let date_str = format!("{} {} {}", parts[5], parts[6], parts[7]);
            let name = parts[8..].join(" ");

            if name == "." || name == ".." {
                continue;
            }

            let is_hidden = name.starts_with('.');
            let item_path = if current_pwd.ends_with('/') {
                format!("{}{}", current_pwd, name)
            } else {
                format!("{}/{}", current_pwd, name)
            };

            let extension = if is_dir {
                "folder".to_string()
            } else {
                name.split('.').last().unwrap_or("").to_lowercase()
            };

            items.push(FileItem {
                name,
                path: item_path,
                is_dir,
                is_symlink,
                size_bytes,
                formatted_size,
                modified_timestamp: 0,
                formatted_modified: date_str,
                extension,
                is_hidden,
                permissions,
                item_count: None,
            });
        }

        items.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(SshDirectoryResult {
            current_path: current_pwd,
            items,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn get_or_fetch_ssh_cached_file(host: &str, remote_path: &str) -> Result<std::path::PathBuf, String> {
    let cache_dir = std::env::temp_dir().join("flashbrowse_ssh_cache");
    let _ = std::fs::create_dir_all(&cache_dir);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&(host, remote_path), &mut hasher);
    let hash = std::hash::Hasher::finish(&hasher);

    let file_name = Path::new(remote_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "remote_file".to_string());

    let cached_path = cache_dir.join(format!("{:x}_{}", hash, file_name));

    let remote_src = format!("{}:'{}'", host, remote_path.replace('\'', "'\\''"));
    let mut args = scp_base_args();
    let cached_str = cached_path.to_string_lossy().to_string();
    args.push(&remote_src);
    args.push(&cached_str);

    let out = Command::new("scp")
        .args(&args)
        .output()
        .map_err(|e| format!("Kunde inte köra scp för förhandsgranskning: {}", e))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("Kunde inte ladda ner fil för förhandsgranskning: {}", err));
    }

    Ok(cached_path)
}

#[tauri::command]
pub async fn ssh_get_preview(host: String, path: String) -> Result<PreviewContent, String> {
    let ext = Path::new(&path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let is_rich_doc = matches!(
        ext.as_str(),
        "xlsx" | "xls" | "ods" | "xlsb" | "docx" | "doc" | "pdf" | "ipynb" | "odt" | "rtf" | "epub"
    );

    if is_rich_doc {
        let host_c = host.clone();
        let path_c = path.clone();
        let cached_res = tauri::async_runtime::spawn_blocking(move || {
            get_or_fetch_ssh_cached_file(&host_c, &path_c)
        })
        .await
        .map_err(|e| e.to_string())?;

        if let Ok(cached_path) = cached_res {
            if let Ok(preview) = crate::preview_commands::get_preview(&cached_path.to_string_lossy(), None) {
                return Ok(preview);
            }
        }
    }

    tauri::async_runtime::spawn_blocking(move || {
        let escaped_path = path.replace('\'', "'\\''");
        let ext = Path::new(&path)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let filename = Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_gz = ext == "gz" || ext == "bgz" || filename.ends_with(".vcf.gz") || filename.ends_with(".fastq.gz") || filename.ends_with(".tsv.gz") || filename.ends_with(".csv.gz");

        // Stat remote file to get size and modified date
        let stat_cmd = format!("stat -c '%s %Y' '{}' 2>/dev/null || stat -f '%z %m' '{}' 2>/dev/null", escaped_path, escaped_path);
        let mut stat_args = ssh_base_args();
        stat_args.push(&host);
        stat_args.push(&stat_cmd);
        let stat_out = Command::new("ssh")
            .args(&stat_args)
            .output();

        let (file_size_bytes, formatted_size, modified_str) = if let Ok(s) = stat_out {
            let str_val = String::from_utf8_lossy(&s.stdout);
            let parts: Vec<&str> = str_val.split_whitespace().collect();
            let size: u64 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
            let mtime: i64 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
            let dt = chrono::DateTime::from_timestamp(mtime, 0).unwrap_or_default();
            (size, format_byte_size(size), dt.format("%Y-%m-%d %H:%M:%S").to_string())
        } else {
            (0, "--".to_string(), "".to_string())
        };

        // 1. Image preview over SSH
        if ["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str()) {
            let b64_cmd = format!("base64 '{}' 2>/dev/null | head -c 5000000", escaped_path);
            let mut img_args = ssh_base_args();
            img_args.push(&host);
            img_args.push(&b64_cmd);
            let out = Command::new("ssh")
                .args(&img_args)
                .output()
                .map_err(|e| e.to_string())?;
            let b64_clean = String::from_utf8_lossy(&out.stdout).replace(['\n', '\r'], "");
            let mime = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                "gif" => "image/gif",
                _ => "image/png",
            };
            return Ok(PreviewContent {
                kind: "image".to_string(),
                text_content: None,
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: None,
                language_name: None,
                language_emoji: None,
                line_count: None,
                image_base64: Some(b64_clean),
                image_mime: Some(mime.to_string()),
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str: "".to_string(),
                error_message: None,
            });
        }

        if ext == "svg" {
            let cat_cmd = format!("head -c 262144 '{}' 2>/dev/null", escaped_path);
            let mut svg_args = ssh_base_args();
            svg_args.push(&host);
            svg_args.push(&cat_cmd);
            let out = Command::new("ssh")
                .args(&svg_args)
                .output()
                .map_err(|e| e.to_string())?;
            let svg_str = String::from_utf8_lossy(&out.stdout).to_string();
            return Ok(PreviewContent {
                kind: "svg".to_string(),
                text_content: Some(svg_str),
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("xml".to_string()),
                language_name: Some("SVG Vector".to_string()),
                language_emoji: Some("🎨".to_string()),
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
                permissions_str: "".to_string(),
                error_message: None,
            });
        }

        // 2. Text / Code / GZ decompression
        let remote_read_cmd = if is_gz {
            format!("gzip -dc '{}' 2>/dev/null | head -c 262144 || zcat '{}' 2>/dev/null | head -c 262144", escaped_path, escaped_path)
        } else {
            format!("head -c 262144 '{}' 2>/dev/null", escaped_path)
        };

        let mut read_args = ssh_base_args();
        read_args.push(&host);
        read_args.push(&remote_read_cmd);
        let out = Command::new("ssh")
            .args(&read_args)
            .output()
            .map_err(|e| format!("SSH fel: {}", e))?;

        if !out.status.success() && out.stdout.is_empty() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() { "Kunde inte läsa filen över SSH".to_string() } else { err });
        }

        let raw_bytes = out.stdout;
        if raw_bytes.is_empty() {
            return Ok(PreviewContent {
                kind: "text".to_string(),
                text_content: Some("(Tom fil)".to_string()),
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("plaintext".to_string()),
                language_name: Some("Tom fil".to_string()),
                language_emoji: Some("📄".to_string()),
                line_count: Some(0),
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str: "".to_string(),
                error_message: None,
            });
        }

        // Check if binary
        let is_binary = raw_bytes.iter().take(1024).any(|&b| b == 0);
        if is_binary && !is_gz {
            return Ok(PreviewContent {
                kind: "binary".to_string(),
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
                formatted_size,
                modified_str,
                permissions_str: "".to_string(),
                error_message: None,
            });
        }

        let text_content = String::from_utf8_lossy(&raw_bytes).to_string();

        // 3. Tabeller (TSV / CSV)
        if ext == "tsv" || ext == "csv" || ext == "tab" || filename.ends_with(".tsv.gz") || filename.ends_with(".csv.gz") {
            let delimiter = if ext == "csv" || filename.ends_with(".csv.gz") { ',' } else { '\t' };
            let (headers, rows) = crate::preview_commands::parse_table_preview(&text_content, delimiter);
            return Ok(PreviewContent {
                kind: "table".to_string(),
                text_content: None,
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: None,
                language_name: None,
                language_emoji: None,
                line_count: Some(rows.len()),
                image_base64: None,
                image_mime: None,
                table_headers: Some(headers),
                table_rows: Some(rows),
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str: "".to_string(),
                error_message: None,
            });
        }

        if ext == "md" || ext == "markdown" {
            return Ok(PreviewContent {
                kind: "markdown".to_string(),
                text_content: Some(text_content.clone()),
                html_content: None,
                pdf_base64: None,
                media_base64: None,
                media_mime: None,
                language: Some("markdown".to_string()),
                language_name: Some("Markdown".to_string()),
                language_emoji: Some("📝".to_string()),
                line_count: Some(text_content.lines().count()),
                image_base64: None,
                image_mime: None,
                table_headers: None,
                table_rows: None,
                sheet_names: None,
                hex_lines: None,
                file_size_bytes,
                formatted_size,
                modified_str,
                permissions_str: "".to_string(),
                error_message: None,
            });
        }

        let (lang, lang_name, lang_emoji) = crate::preview_commands::detect_language_meta(&filename, &text_content);
        let final_lang_name = if is_gz { format!("{} (gzip)", lang_name) } else { lang_name.to_string() };

        Ok(PreviewContent {
            kind: "code".to_string(),
            text_content: Some(text_content.clone()),
            html_content: None,
            pdf_base64: None,
            media_base64: None,
            media_mime: None,
            language: Some(lang.to_string()),
            language_name: Some(final_lang_name),
            language_emoji: Some(lang_emoji.to_string()),
            line_count: Some(text_content.lines().count()),
            image_base64: None,
            image_mime: None,
            table_headers: None,
            table_rows: None,
            sheet_names: None,
            hex_lines: None,
            file_size_bytes,
            formatted_size,
            modified_str,
            permissions_str: "".to_string(),
            error_message: None,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ssh_run_command(host: String, cmd: String, cwd: String) -> Result<TerminalOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let remote_script = if cwd.is_empty() || cwd == "~" {
            format!("cd ~ && {}", cmd)
        } else if cwd.starts_with("~/") {
            let rest = &cwd[2..].replace('\'', "'\\''");
            format!("cd \"${{HOME}}/{}\" && {}", rest, cmd)
        } else {
            let escaped_cwd = cwd.replace('\'', "'\\''");
            format!("cd '{}' && {}", escaped_cwd, cmd)
        };

        let mut run_args = ssh_base_args();
        run_args.push(&host);
        run_args.push(&remote_script);

        let output = Command::new("ssh")
            .args(&run_args)
            .output()
            .map_err(|e| format!("Kunde inte köra SSH-kommando: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(TerminalOutput {
            stdout,
            stderr,
            exit_code,
            new_cwd: None,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
