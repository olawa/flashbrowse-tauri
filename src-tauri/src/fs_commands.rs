use crate::models::{DirectoryIndexGroup, DirectorySummary, DiskInfo, FileItem};
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

pub fn resolve_path(path: &str) -> PathBuf {
    if path.is_empty() || path == "~" {
        dirs_home()
    } else if path.starts_with("~/") {
        dirs_home().join(&path[2..])
    } else if path.starts_with('~') {
        dirs_home().join(&path[1..])
    } else {
        PathBuf::from(path)
    }
}

#[tauri::command]
pub fn list_directory(path: &str, show_hidden: bool) -> Result<Vec<FileItem>, String> {
    let resolved_path = resolve_path(path);

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
pub async fn calculate_dir_size(path: String) -> Result<DirectorySummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let target = resolve_path(&path);
        if !target.exists() {
            return Err("Directory does not exist".to_string());
        }

        let mut total_size_bytes = 0u64;
        let mut total_files = 0usize;
        let mut total_dirs = 0usize;

        for entry in walkdir::WalkDir::new(&target)
            .max_depth(256)
            .into_iter()
            .filter_map(|e| e.ok())
        {
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
            path,
            total_items: total_files + total_dirs,
            total_dirs,
            total_files,
            total_size_bytes,
            formatted_total_size: format_byte_size(total_size_bytes),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn trash_items(paths: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        for p in paths {
            let path = resolve_path(&p);
            if path.exists() {
                trash::delete(&path).map_err(|e| format!("Failed to trash {}: {}", p, e))?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn copy_items(paths: Vec<String>, destination_dir: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dest = resolve_path(&destination_dir);
        if !dest.is_dir() {
            return Err("Destination is not a directory".to_string());
        }

        for p in paths {
            let src = resolve_path(&p);
            if let Some(file_name) = src.file_name() {
                let target = dest.join(file_name);
                if src.is_dir() {
                    copy_dir_recursive(&src, &target).map_err(|e| e.to_string())?;
                } else {
                    fs::copy(&src, &target).map_err(|e| format!("Failed to copy {}: {}", p, e))?;
                }
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_symlink() {
        let target = fs::read_link(src)?;
        #[cfg(unix)]
        {
            return std::os::unix::fs::symlink(&target, dst);
        }
        #[cfg(windows)]
        {
            if target.is_dir() {
                return std::os::windows::fs::symlink_dir(&target, dst);
            } else {
                return std::os::windows::fs::symlink_file(&target, dst);
            }
        }
    }

    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_child = dst.join(entry.file_name());

        if entry_path.is_symlink() {
            let link_target = fs::read_link(&entry_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&link_target, &dest_child)?;
            #[cfg(windows)]
            {
                if link_target.is_dir() {
                    let _ = std::os::windows::fs::symlink_dir(&link_target, &dest_child);
                } else {
                    let _ = std::os::windows::fs::symlink_file(&link_target, &dest_child);
                }
            }
        } else if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry_path, &dest_child)?;
        } else {
            fs::copy(&entry_path, dest_child)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn move_items(paths: Vec<String>, destination_dir: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dest = resolve_path(&destination_dir);
        if !dest.is_dir() {
            return Err("Destination is not a directory".to_string());
        }

        for p in paths {
            let src = resolve_path(&p);
            if let Some(file_name) = src.file_name() {
                let target = dest.join(file_name);
                fs::rename(&src, &target).map_err(|e| format!("Failed to move {}: {}", p, e))?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn create_directory(parent: &str, name: &str) -> Result<String, String> {
    let new_path = resolve_path(parent).join(name);
    fs::create_dir_all(&new_path).map_err(|e| format!("Failed to create folder: {}", e))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn create_file(parent: &str, name: &str) -> Result<String, String> {
    let new_path = resolve_path(parent).join(name);
    fs::File::create(&new_path).map_err(|e| format!("Failed to create file: {}", e))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn rename_item(path: &str, new_name: &str) -> Result<String, String> {
    let old_path = resolve_path(path);
    let parent = old_path.parent().ok_or("Invalid parent directory")?;
    let new_path = parent.join(new_name);
    fs::rename(&old_path, &new_path).map_err(|e| format!("Failed to rename: {}", e))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_in_default(path: &str) -> Result<(), String> {
    let resolved = resolve_path(path);
    let path_str = resolved.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path_str])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn reveal_in_os(path: &str) -> Result<(), String> {
    let resolved = resolve_path(path);
    let path_str = resolved.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path_str])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = resolved.parent().unwrap_or_else(|| Path::new("/"));
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path_str])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn quick_look(path: &str) -> Result<(), String> {
    let resolved = resolve_path(path);
    let path_str = resolved.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("qlmanage")
            .args(["-p", &path_str])
            .spawn()
            .map_err(|e| format!("Failed to launch QuickLook: {}", e))?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        open_in_default(&path_str)?;
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_detached_inspector(app: tauri::AppHandle, path: Option<String>) -> Result<(), String> {
    use tauri::{Emitter, Manager};
    let target_url = if let Some(ref p) = path {
        format!("index.html?window=inspector&path={}", urlencoding::encode(p))
    } else {
        "index.html?window=inspector".to_string()
    };

    if let Some(window) = app.get_webview_window("inspector") {
        if let Some(ref p) = path {
            let _ = app.emit("inspector-sync-path", p);
            let _ = app.emit("inspector-cast-path", p);
        }
        if !window.is_visible().unwrap_or(false) {
            let _ = window.show();
        }
        let _ = window.set_focus();
    } else {
        let _win = tauri::WebviewWindowBuilder::new(
            &app,
            "inspector",
            tauri::WebviewUrl::App(target_url.into()),
        )
        .title("Flashbrowse Inspector")
        .inner_size(850.0, 650.0)
        .min_inner_size(500.0, 400.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn scan_directory_index(
    root_path: String,
    extensions: Vec<String>,
    max_depth: Option<usize>,
) -> Result<Vec<DirectoryIndexGroup>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base_root = resolve_path(&root_path);
        if !base_root.exists() {
            return Err(format!("Sökvägen finns inte: {}", base_root.display()));
        }

        let depth = max_depth.unwrap_or(8);
        let normalized_exts: Vec<String> = extensions
            .iter()
            .map(|e| e.to_lowercase().trim_start_matches('.').to_string())
            .collect();

        use std::collections::BTreeMap;
        let mut grouped_items: BTreeMap<PathBuf, Vec<FileItem>> = BTreeMap::new();

        let walker = walkdir::WalkDir::new(&base_root)
            .max_depth(depth)
            .follow_links(false)
            .into_iter();

        for entry in walker.filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.file_type().is_dir() {
                let lower = name.to_lowercase();
                // Skip hidden folders (.git, .Trash, etc.) and heavy build caches
                if name.starts_with('.') && name != "." {
                    return false;
                }
                // Exclude system / TCC protected user directories that trigger permission popups
                if lower == "music"
                    || lower == "pictures"
                    || lower == "photos library.photoslibrary"
                    || lower == "movies"
                    || lower == "podcasts"
                    || lower == "library"
                    || lower == "node_modules"
                    || lower == "target"
                    || lower == "build"
                    || lower == "dist"
                    || lower == ".trash"
                    || lower == "caches"
                    || lower == ".cache"
                    || lower == ".cargo"
                    || lower == ".rustup"
                    || lower == ".npm"
                {
                    return false;
                }
            }
            true
        }).filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let entry_path = entry.path();
            let file_name = entry_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let lower_name = file_name.to_lowercase();

            // Match exact extension, dot extension or compound extensions (e.g. .vcf.gz, .fastq.gz, .tar.gz)
            let is_match = normalized_exts.iter().any(|ext| {
                if lower_name.ends_with(&format!(".{}", ext)) || lower_name == *ext {
                    true
                } else if let Some(file_ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    file_ext.eq_ignore_ascii_case(ext)
                } else {
                    false
                }
            });

            if is_match {
                if let Some(parent_dir) = entry_path.parent() {
                    let metadata = entry.metadata().ok();
                    let is_symlink = entry.path_is_symlink();
                    let is_dir = false;
                    let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let formatted_size = format_byte_size(size_bytes);

                    let modified_time = metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                    let dt: DateTime<Local> = modified_time.into();
                    let modified_timestamp = dt.timestamp();
                    let formatted_modified = dt.format("%Y-%m-%d %H:%M").to_string();

                    let extension = entry_path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    let permissions = metadata
                        .as_ref()
                        .map(get_permissions_string)
                        .unwrap_or_else(|| "---------".to_string());

                    let file_item = FileItem {
                        name: file_name,
                        path: entry_path.to_string_lossy().to_string(),
                        is_dir,
                        is_symlink,
                        size_bytes,
                        formatted_size,
                        modified_timestamp,
                        formatted_modified,
                        extension,
                        is_hidden: false,
                        permissions,
                        item_count: None,
                    };

                    grouped_items.entry(parent_dir.to_path_buf()).or_default().push(file_item);
                }
            }
        }

        let base_root_str = base_root.to_string_lossy().to_string();
        let mut result_groups = Vec::new();

        for (dir_path, mut items) in grouped_items {
            items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            let dir_str = dir_path.to_string_lossy().to_string();
            let dir_name = dir_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "/".to_string());

            let relative_path = if dir_str.starts_with(&base_root_str) {
                let rel = dir_str[base_root_str.len()..].trim_start_matches('/');
                if rel.is_empty() {
                    "./".to_string()
                } else {
                    format!("./{}", rel)
                }
            } else {
                dir_str.clone()
            };

            result_groups.push(DirectoryIndexGroup {
                directory_path: dir_str,
                directory_name: if dir_name.is_empty() { "/".to_string() } else { dir_name },
                relative_path,
                items,
            });
        }

        // Sort groups alphabetically by relative path
        result_groups.sort_by(|a, b| a.relative_path.to_lowercase().cmp(&b.relative_path.to_lowercase()));

        Ok(result_groups)
    }).await.map_err(|e| e.to_string())?
}
