use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{Emitter, Manager, RunEvent, State};

type SharedMaster = Arc<Mutex<Box<dyn MasterPty + Send>>>;
type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;
type SharedKiller = Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>;
type SharedTerminalState = Arc<Mutex<TerminalState>>;

struct TerminalSession {
    master: SharedMaster,
    killer: SharedKiller,
    writer: SharedWriter,
}

impl TerminalSession {
    fn kill(&self) {
        if let Ok(mut killer) = self.killer.lock() {
            let _ = killer.kill();
        }
    }
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
    let mut child = pty
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
    let killer: SharedKiller = Arc::new(Mutex::new(child.clone_killer()));

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
                killer,
                writer: Arc::new(Mutex::new(writer)),
            },
        );
        id
    };

    let output_app = app.clone();
    thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0_u8; 8192];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(size) => {
                    pending.extend_from_slice(&buffer[..size]);
                    let data = take_decodable_prefix(&mut pending);

                    if data.is_empty() {
                        continue;
                    }

                    let _ =
                        output_app.emit("terminal://output", TerminalOutput { session_id, data });
                }
            }
        }
    });

    let exit_app = app.clone();
    let exit_state = Arc::clone(state.inner());
    thread::spawn(move || {
        let code = child.wait().ok().map(|status| status.exit_code());
        if let Ok(mut terminals) = exit_state.lock() {
            terminals.sessions.remove(&session_id);
        }
        let _ = exit_app.emit("terminal://exit", TerminalExit { session_id, code });
    });

    Ok(session_id)
}

/// Decode every complete UTF-8 sequence buffered so far, leaving an incomplete
/// trailing sequence in `pending` so it can be joined with the next PTY read.
fn take_decodable_prefix(pending: &mut Vec<u8>) -> String {
    let mut decoded = String::new();

    loop {
        match std::str::from_utf8(pending) {
            Ok(text) => {
                decoded.push_str(text);
                pending.clear();

                return decoded;
            }
            Err(error) => {
                let valid = error.valid_up_to();

                if let Ok(text) = std::str::from_utf8(&pending[..valid]) {
                    decoded.push_str(text);
                }

                match error.error_len() {
                    None => {
                        pending.drain(..valid);

                        return decoded;
                    }
                    Some(invalid) => {
                        decoded.push(char::REPLACEMENT_CHARACTER);
                        pending.drain(..valid + invalid);
                    }
                }
            }
        }
    }
}

fn session_handle<T>(
    state: &State<'_, SharedTerminalState>,
    session_id: u32,
    select: impl Fn(&TerminalSession) -> T,
) -> Result<T, String> {
    let terminals = state
        .lock()
        .map_err(|_| "Terminal state lock was poisoned".to_string())?;

    terminals
        .sessions
        .get(&session_id)
        .map(select)
        .ok_or_else(|| "Terminal session is closed".to_string())
}

#[tauri::command]
fn terminal_write(
    state: State<'_, SharedTerminalState>,
    session_id: u32,
    data: String,
) -> Result<(), String> {
    let shared_writer = session_handle(&state, session_id, |session| Arc::clone(&session.writer))?;
    let mut writer = shared_writer
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
    let shared_master = session_handle(&state, session_id, |session| Arc::clone(&session.master))?;

    let master = shared_master
        .lock()
        .map_err(|_| "Terminal master lock was poisoned".to_string())?;

    master
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Could not resize terminal: {error}"))
}

#[tauri::command]
fn terminal_close(state: State<'_, SharedTerminalState>, session_id: u32) -> Result<(), String> {
    let session = {
        let mut terminals = state
            .lock()
            .map_err(|_| "Terminal state lock was poisoned".to_string())?;

        terminals.sessions.remove(&session_id)
    };

    // Dropping the session releases the master PTY and its writer, so the shell
    // sees EOF/SIGHUP even if the kill signal does not reach it.
    if let Some(session) = session {
        session.kill();
    }

    Ok(())
}

fn kill_all_terminals(state: &SharedTerminalState) {
    let sessions = match state.lock() {
        Ok(mut terminals) => terminals.sessions.drain().collect::<Vec<_>>(),
        Err(_) => return,
    };

    for (_, session) in sessions {
        session.kill();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Mutex::new(TerminalState::default())) as SharedTerminalState)
        .invoke_handler(tauri::generate_handler![
            terminal_spawn,
            terminal_write,
            terminal_resize,
            terminal_close
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|handle, event| {
        if matches!(event, RunEvent::Exit) {
            kill_all_terminals(&handle.state::<SharedTerminalState>());
        }
    });
}
