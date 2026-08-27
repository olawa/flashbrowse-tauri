use crate::fs_commands::format_byte_size;
use crate::models::{FileItem, TerminalOutput};
use std::process::Command;

#[tauri::command]
pub fn ssh_list_directory(host: &str, path: &str) -> Result<Vec<FileItem>, String> {
    let clean_path = if path.is_empty() { "~" } else { path };
    let escaped_path = clean_path.replace('\'', "'\\''");

    // Run remote ls script
    let remote_script = format!("cd '{}' 2>/dev/null && pwd && ls -la", escaped_path);

    let output = Command::new("ssh")
        .args([
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=5",
            host,
            &remote_script,
        ])
        .output()
        .map_err(|e| format!("SSH command failed to launch: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("SSH error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout_str.lines();

    // First line is canonical pwd
    let current_pwd = lines.next().unwrap_or("~").trim().to_string();

    let mut items = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("total ") || trimmed.is_empty() {
            continue;
        }

        // Parse ls -la output: permissions, links, owner, group, size, month, day, time/year, name
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

    Ok(items)
}

#[tauri::command]
pub fn ssh_run_command(host: &str, cmd: &str, cwd: &str) -> Result<TerminalOutput, String> {
    let clean_cwd = if cwd.is_empty() { "~" } else { cwd };
    let escaped_cwd = clean_cwd.replace('\'', "'\\''");
    let remote_script = format!("cd '{}' 2>/dev/null && {}", escaped_cwd, cmd);

    let output = Command::new("ssh")
        .args([
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=8",
            host,
            &remote_script,
        ])
        .output()
        .map_err(|e| format!("SSH command execution failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(TerminalOutput {
        stdout,
        stderr,
        exit_code,
        new_cwd: None,
    })
}
