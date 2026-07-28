use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{Emitter, State};

type SharedMaster = Arc<Mutex<Box<dyn MasterPty + Send>>>;
type SharedChild = Arc<Mutex<Box<dyn Child + Send>>>;
type SharedTerminalState = Arc<Mutex<TerminalState>>;

struct TerminalSession {
    master: SharedMaster,
    child: SharedChild,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

struct TerminalState {
    next_id: u32,
    sessions: HashMap<u32, TerminalSession>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            next_id: 1,
            sessions: HashMap::new(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutput {
    session_id: u32,
    data: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalExit {
    session_id: u32,
    code: Option<u32>,
}

fn safe_home_path(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Could not access terminal directory: {error}"))?;
    let canonical_home = home
        .canonicalize()
        .map_err(|error| format!("Could not verify home directory: {error}"))?;

    if !canonical.starts_with(&canonical_home) {
        return Err("For safety, qedit terminals can only start in your home directory.".into());
    }

    if !canonical.is_dir() {
        return Err("The terminal working directory is not a folder.".into());
    }

    Ok(canonical)
}

fn shell_command() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()))
    }
}

#[tauri::command]
fn terminal_spawn(
    app: tauri::AppHandle,
    state: State<'_, SharedTerminalState>,
    cwd: String,
    cols: u16,
    rows: u16,
) -> Result<u32, String> {
    let cwd = safe_home_path(&cwd)?;
    let shell = shell_command()?;
    let pty = native_pty_system()
        .openpty(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Could not create terminal: {error}"))?;
    let mut command = CommandBuilder::new(shell);
    command.cwd(cwd);
    let child = pty
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Could not start terminal process: {error}"))?;
    drop(pty.slave);

    let master: SharedMaster = Arc::new(Mutex::new(pty.master));
    let reader = master
        .lock()
        .map_err(|_| "Terminal master lock was poisoned".to_string())?
        .try_clone_reader()
        .map_err(|error| format!("Could not read terminal output: {error}"))?;
    let writer = master
        .lock()
        .map_err(|_| "Terminal master lock was poisoned".to_string())?
        .take_writer()
        .map_err(|error| format!("Could not write to terminal: {error}"))?;
    let child: SharedChild = Arc::new(Mutex::new(child));

    let session_id = {
        let mut terminals = state
            .lock()
            .map_err(|_| "Terminal state lock was poisoned".to_string())?;
        let id = terminals.next_id;
        terminals.next_id = terminals.next_id.wrapping_add(1).max(1);
        terminals.sessions.insert(
            id,
            TerminalSession {
                master: Arc::clone(&master),
                child: Arc::clone(&child),
                writer: Arc::new(Mutex::new(writer)),
            },
        );
        id
    };

    let output_app = app.clone();
    thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(size) => {
                    let _ = output_app.emit(
                        "terminal://output",
                        TerminalOutput {
                            session_id,
                            data: String::from_utf8_lossy(&buffer[..size]).into_owned(),
                        },
                    );
                }
            }
        }
    });

    let exit_app = app.clone();
    let exit_state = Arc::clone(state.inner());
    thread::spawn(move || {
        let code = child
            .lock()
            .ok()
            .and_then(|mut process| process.wait().ok())
            .map(|status| status.exit_code());
        if let Ok(mut terminals) = exit_state.lock() {
            terminals.sessions.remove(&session_id);
        }
        let _ = exit_app.emit("terminal://exit", TerminalExit { session_id, code });
    });

    Ok(session_id)
}

#[tauri::command]
fn terminal_write(
    state: State<'_, SharedTerminalState>,
    session_id: u32,
    data: String,
) -> Result<(), String> {
    let terminals = state
        .lock()
        .map_err(|_| "Terminal state lock was poisoned".to_string())?;
    let session = terminals
        .sessions
        .get(&session_id)
        .ok_or_else(|| "Terminal session is closed".to_string())?;
    let mut writer = session
        .writer
        .lock()
        .map_err(|_| "Terminal writer lock was poisoned".to_string())?;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|error| format!("Could not write to terminal: {error}"))
}

#[tauri::command]
fn terminal_resize(
    state: State<'_, SharedTerminalState>,
    session_id: u32,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let terminals = state
        .lock()
        .map_err(|_| "Terminal state lock was poisoned".to_string())?;
    let session = terminals
        .sessions
        .get(&session_id)
        .ok_or_else(|| "Terminal session is closed".to_string())?;
    let result = session
        .master
        .lock()
        .map_err(|_| "Terminal master lock was poisoned".to_string())?
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Could not resize terminal: {error}"));
    result
}

#[tauri::command]
fn terminal_close(state: State<'_, SharedTerminalState>, session_id: u32) -> Result<(), String> {
    let mut terminals = state
        .lock()
        .map_err(|_| "Terminal state lock was poisoned".to_string())?;
    if let Some(session) = terminals.sessions.remove(&session_id) {
        let _ = session
            .child
            .lock()
            .map_err(|_| "Terminal child lock was poisoned".to_string())?
            .kill();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Mutex::new(TerminalState::default())))
        .invoke_handler(tauri::generate_handler![
            terminal_spawn,
            terminal_write,
            terminal_resize,
            terminal_close
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
