use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub struct PtySession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
}

static SESSIONS: Mutex<Option<HashMap<String, PtySession>>> = Mutex::new(None);

pub fn remove_session(session_id: &str) {
    if let Ok(mut lock) = SESSIONS.lock() {
        if let Some(map) = lock.as_mut() {
            if let Some(mut session) = map.remove(session_id) {
                let _ = session.child.kill();
            }
        }
    }
}

#[tauri::command]
pub fn pty_spawn(
    app: AppHandle,
    session_id: String,
    session_type: String,
    cwd: Option<String>,
    host: Option<String>,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    // Kill existing session with same ID if any
    remove_session(&session_id);

    let pty_system = native_pty_system();
    let pty_size = PtySize {
        rows: if rows == 0 { 24 } else { rows },
        cols: if cols == 0 { 80 } else { cols },
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(pty_size)
        .map_err(|e| format!("Kunde inte skapa PTY: {}", e))?;

    let cmd = if session_type == "ssh" {
        let target_host = host.ok_or_else(|| "SSH-värd saknas för SSH-session".to_string())?;
        let mut builder = CommandBuilder::new("ssh");
        builder.arg("-t");
        builder.arg("-o");
        builder.arg("ControlMaster=auto");
        builder.arg("-o");
        builder.arg(crate::ssh_commands::control_path_option());
        builder.arg("-o");
        builder.arg("ControlPersist=15m");
        builder.arg("-o");
        builder.arg("ServerAliveInterval=15");
        builder.arg("-o");
        builder.arg("ServerAliveCountMax=3");
        builder.arg("-o");
        builder.arg("StrictHostKeyChecking=accept-new");
        builder.arg("--");
        builder.arg(&target_host);

        if let Some(remote_dir) = cwd {
            let trimmed = remote_dir.trim();
            if !trimmed.is_empty() && trimmed != "~" {
                let quoted = crate::ssh_commands::sh_quote(trimmed);
                builder.arg(format!("cd {} 2>/dev/null; exec $SHELL -l", quoted));
            }
        }
        builder.env("TERM", "xterm-256color");
        builder.env("COLORTERM", "truecolor");
        builder
    } else {
        // Local session
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            #[cfg(target_os = "windows")]
            {
                "powershell.exe".to_string()
            }
            #[cfg(not(target_os = "windows"))]
            {
                "/bin/zsh".to_string()
            }
        });

        let mut builder = CommandBuilder::new(&shell);
        builder.arg("-l"); // Login shell to load user environment & rc files

        if let Some(dir) = cwd {
            let trimmed = dir.trim();
            if !trimmed.is_empty() {
                builder.cwd(crate::fs_commands::resolve_path(trimmed));
            } else {
                builder.cwd(crate::fs_commands::dirs_home());
            }
        } else {
            builder.cwd(crate::fs_commands::dirs_home());
        }

        builder.env("TERM", "xterm-256color");
        builder.env("COLORTERM", "truecolor");
        builder
    };

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Kunde inte starta terminalprocess: {}", e))?;

    // Drop slave explicitly so master receives EOF on child termination
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Kunde inte klona PTY-läsare: {}", e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Kunde inte ta PTY-skrivare: {}", e))?;

    let id_for_thread = session_id.clone();
    let app_for_thread = app.clone();

    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    remove_session(&id_for_thread);
                    let _ = app_for_thread.emit(&format!("pty-exit-{}", id_for_thread), ());
                    break;
                }
                Ok(n) => {
                    let encoded = BASE64.encode(&buf[..n]);
                    let _ = app_for_thread.emit(&format!("pty-output-{}", id_for_thread), encoded);
                }
                Err(_) => {
                    remove_session(&id_for_thread);
                    let _ = app_for_thread.emit(&format!("pty-exit-{}", id_for_thread), ());
                    break;
                }
            }
        }
    });

    let session = PtySession {
        writer,
        master: pair.master,
        child,
    };

    let mut lock = SESSIONS.lock().map_err(|e| e.to_string())?;
    if lock.is_none() {
        *lock = Some(HashMap::new());
    }
    if let Some(map) = lock.as_mut() {
        map.insert(session_id, session);
    }

    Ok(())
}

#[tauri::command]
pub fn pty_write(session_id: String, data: String) -> Result<(), String> {
    let mut lock = SESSIONS.lock().map_err(|e| e.to_string())?;
    if let Some(map) = lock.as_mut() {
        if let Some(session) = map.get_mut(&session_id) {
            session
                .writer
                .write_all(data.as_bytes())
                .map_err(|e| format!("Kunde inte skriva till PTY: {}", e))?;
            session
                .writer
                .flush()
                .map_err(|e| format!("Kunde inte spola PTY-skrivare: {}", e))?;
            return Ok(());
        }
    }
    Err(format!("PTY-session hittades inte: {}", session_id))
}

#[tauri::command]
pub fn pty_resize(session_id: String, rows: u16, cols: u16) -> Result<(), String> {
    let mut lock = SESSIONS.lock().map_err(|e| e.to_string())?;
    if let Some(map) = lock.as_mut() {
        if let Some(session) = map.get_mut(&session_id) {
            session
                .master
                .resize(PtySize {
                    rows: if rows == 0 { 24 } else { rows },
                    cols: if cols == 0 { 80 } else { cols },
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| format!("Kunde inte ändra storlek på PTY: {}", e))?;
            return Ok(());
        }
    }
    Err(format!("PTY-session hittades inte: {}", session_id))
}

#[tauri::command]
pub fn pty_kill(session_id: String) -> Result<(), String> {
    remove_session(&session_id);
    Ok(())
}

#[tauri::command]
pub fn pty_has_session(session_id: String) -> Result<bool, String> {
    let lock = SESSIONS.lock().map_err(|e| e.to_string())?;
    if let Some(map) = lock.as_ref() {
        Ok(map.contains_key(&session_id))
    } else {
        Ok(false)
    }
}
