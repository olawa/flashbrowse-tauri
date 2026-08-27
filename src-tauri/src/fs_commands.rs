use crate::models::{DirectorySummary, DiskInfo, FileItem};
use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::Disks;

pub fn format_byte_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn get_permissions_string(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        let r = |m: u32, bit: u32, ch: char| if (m & bit) != 0 { ch } else { '-' };
        format!(
            "{}{}{}{}{}{}{}{}{}",
            r(mode, 0o400, 'r'),
            r(mode, 0o200, 'w'),
            r(mode, 0o100, 'x'),
            r(mode, 0o040, 'r'),
            r(mode, 0o020, 'w'),
            r(mode, 0o010, 'x'),
            r(mode, 0o004, 'r'),
            r(mode, 0o002, 'w'),
            r(mode, 0o001, 'x'),
        )
    }
    #[cfg(not(unix))]
    {
        if metadata.permissions().readonly() {
            "r--r--r--".to_string()
        } else {
            "rw-rw-rw-".to_string()
        }
    }
}

#[tauri::command]
pub fn get_home_directory() -> String {
    dirs_home().to_string_lossy().to_string()
}

pub fn dirs_home() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"))
    }
}

#[tauri::command]
pub fn list_directory(path: &str, show_hidden: bool) -> Result<Vec<FileItem>, String> {
    let resolved_path = if path.starts_with('~') {
        let home = dirs_home();
        home.join(path.trim_start_matches("~/").trim_start_matches('~'))
    } else {
        PathBuf::from(path)
    };

    if !resolved_path.exists() {
        return Err(format!("Path does not exist: {}", resolved_path.display()));
    }

    let read_dir = fs::read_dir(&resolved_path)
        .map_err(|e| format!("Failed to read directory {}: {}", resolved_path.display(), e))?;

    let mut items = Vec::new();

    for entry_res in read_dir {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_name = entry.file_name().to_string_lossy().to_string();
        let is_hidden = file_name.starts_with('.');

        if is_hidden && !show_hidden {
            continue;
        }

        let entry_path = entry.path();
        let symlink_metadata = fs::symlink_metadata(&entry_path);
        let metadata = fs::metadata(&entry_path).or_else(|_| symlink_metadata.as_ref().map(|m| m.clone()));

        let is_symlink = entry.file_type().map(|t| t.is_symlink()).unwrap_or(false);
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        let meta_ref = metadata.as_ref().ok();
        let size_bytes = meta_ref.map(|m| m.len()).unwrap_or(0);
        let formatted_size = if is_dir {
            "--".to_string()
        } else {
            format_byte_size(size_bytes)
        };

        let modified_time = meta_ref
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let dt: DateTime<Local> = modified_time.into();
        let modified_timestamp = dt.timestamp();
        let formatted_modified = dt.format("%Y-%m-%d %H:%M").to_string();

        let extension = if is_dir {
            "folder".to_string()
        } else {
            entry_path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        };

        let permissions = meta_ref
            .map(get_permissions_string)
            .unwrap_or_else(|| "---------".to_string());

        items.push(FileItem {
            name: file_name,
            path: entry_path.to_string_lossy().to_string(),
            is_dir,
            is_symlink,
            size_bytes,
            formatted_size,
            modified_timestamp,
            formatted_modified,
            extension,
            is_hidden,
            permissions,
            item_count: None,
        });
    }

    // Sort: directories first, then alphabetical case-insensitive
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
pub fn get_disk_info(path: &str) -> Result<DiskInfo, String> {
    let check_path = PathBuf::from(if path.is_empty() { "/" } else { path });
    let disks = Disks::new_with_refreshed_list();

    // Find the disk that matches or is the parent of check_path
    let mut best_match: Option<&sysinfo::Disk> = None;
    let mut longest_mount_len = 0;

    for disk in disks.list() {
        let mount_str = disk.mount_point().to_string_lossy();
        if check_path.starts_with(disk.mount_point()) && mount_str.len() >= longest_mount_len {
            longest_mount_len = mount_str.len();
            best_match = Some(disk);
        }
    }

    if let Some(disk) = best_match.or_else(|| disks.list().first()) {
        let total_bytes = disk.total_space();
        let available_bytes = disk.available_space();
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let percentage_used = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        Ok(DiskInfo {
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_bytes,
            available_bytes,
            used_bytes,
            formatted_total: format_byte_size(total_bytes),
            formatted_available: format_byte_size(available_bytes),
            formatted_used: format_byte_size(used_bytes),
            percentage_used,
        })
    } else {
        Err("No disk info found".to_string())
    }
}

#[tauri::command]
pub fn calculate_dir_size(path: &str) -> Result<DirectorySummary, String> {
    let target = Path::new(path);
    if !target.exists() {
        return Err("Directory does not exist".to_string());
    }

    let mut total_size_bytes = 0u64;
    let mut total_files = 0usize;
    let mut total_dirs = 0usize;

    for entry in walkdir::WalkDir::new(target).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total_files += 1;
            if let Ok(meta) = entry.metadata() {
                total_size_bytes += meta.len();
            }
        } else if entry.file_type().is_dir() && entry.path() != target {
            total_dirs += 1;
        }
    }

    Ok(DirectorySummary {
        path: path.to_string(),
        total_items: total_files + total_dirs,
        total_dirs,
        total_files,
        total_size_bytes,
        formatted_total_size: format_byte_size(total_size_bytes),
    })
}

#[tauri::command]
pub fn trash_items(paths: Vec<String>) -> Result<(), String> {
    for p in paths {
        let path = Path::new(&p);
        if path.exists() {
            trash::delete(path).map_err(|e| format!("Failed to trash {}: {}", p, e))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn copy_items(paths: Vec<String>, destination_dir: String) -> Result<(), String> {
    let dest = Path::new(&destination_dir);
    if !dest.is_dir() {
        return Err("Destination is not a directory".to_string());
    }

    for p in paths {
        let src = Path::new(&p);
        if let Some(file_name) = src.file_name() {
            let target = dest.join(file_name);
            if src.is_dir() {
                copy_dir_recursive(src, &target).map_err(|e| e.to_string())?;
            } else {
                fs::copy(src, target).map_err(|e| format!("Failed to copy {}: {}", p, e))?;
            }
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), dest_child)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn move_items(paths: Vec<String>, destination_dir: String) -> Result<(), String> {
    let dest = Path::new(&destination_dir);
    if !dest.is_dir() {
        return Err("Destination is not a directory".to_string());
    }

    for p in paths {
        let src = Path::new(&p);
        if let Some(file_name) = src.file_name() {
            let target = dest.join(file_name);
            fs::rename(src, target).map_err(|e| format!("Failed to move {}: {}", p, e))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn create_directory(parent: &str, name: &str) -> Result<String, String> {
    let new_path = Path::new(parent).join(name);
    fs::create_dir_all(&new_path).map_err(|e| format!("Failed to create folder: {}", e))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn create_file(parent: &str, name: &str) -> Result<String, String> {
    let new_path = Path::new(parent).join(name);
    fs::File::create(&new_path).map_err(|e| format!("Failed to create file: {}", e))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn rename_item(path: &str, new_name: &str) -> Result<String, String> {
    let old_path = Path::new(path);
    let parent = old_path.parent().ok_or("Invalid parent directory")?;
    let new_path = parent.join(new_name);
    fs::rename(old_path, &new_path).map_err(|e| format!("Failed to rename: {}", e))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_in_default(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn reveal_in_os(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = Path::new(path).parent().unwrap_or_else(|| Path::new("/"));
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
