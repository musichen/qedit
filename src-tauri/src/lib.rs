use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{Emitter, Manager, RunEvent, Runtime, State};

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

fn stop_child(child: &mut Box<dyn Child + Send + Sync>) {
    let _ = child.kill();
    let _ = child.wait();
}

fn configured_shell(shell: Option<&OsStr>, account_shell: Option<&OsStr>) -> OsString {
    shell
        .filter(|value| !value.is_empty())
        .or_else(|| account_shell.filter(|value| !value.is_empty()))
        .map(OsStr::to_os_string)
        .unwrap_or_else(|| {
            #[cfg(target_os = "windows")]
            {
                OsString::from("cmd.exe")
            }
            #[cfg(not(target_os = "windows"))]
            {
                OsString::from("/bin/sh")
            }
        })
}

#[cfg(unix)]
fn account_shell() -> Option<OsString> {
    use std::ffi::CStr;
    use std::mem::MaybeUninit;

    // A GUI-launched Tauri process often does not inherit SHELL. Read the
    // account's configured shell directly, matching the platform terminal's
    // fallback instead of assuming that every user runs zsh. getpwuid_r (with
    // caller-owned storage) is used instead of getpwuid because the latter
    // returns a pointer into libc's non-reentrant static buffer: Tauri
    // dispatches commands on a thread pool, so two concurrent terminal_spawn
    // calls could otherwise race on the same static passwd storage.
    let mut passwd = MaybeUninit::<libc::passwd>::uninit();
    let mut result: *mut libc::passwd = std::ptr::null_mut();
    let mut buffer_len: usize = 1024;

    loop {
        let mut buffer = vec![0 as libc::c_char; buffer_len];
        let status = unsafe {
            libc::getpwuid_r(
                libc::getuid(),
                passwd.as_mut_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut result,
            )
        };

        if status == 0 {
            if result.is_null() {
                return None;
            }
            let shell = unsafe { CStr::from_ptr((*passwd.as_ptr()).pw_shell) };
            let shell = shell.to_str().ok()?.trim();
            return (!shell.is_empty()).then(|| OsString::from(shell));
        }

        if status == libc::ERANGE && buffer_len < 1 << 20 {
            buffer_len *= 2;
            continue;
        }

        return None;
    }
}

#[cfg(windows)]
fn account_shell() -> Option<OsString> {
    std::env::var_os("COMSPEC").filter(|value| !value.is_empty())
}

fn shell_command() -> OsString {
    let shell = std::env::var_os("SHELL").filter(|value| !value.is_empty());
    if let Some(shell) = shell {
        return configured_shell(Some(shell.as_os_str()), None);
    }
    configured_shell(None, account_shell().as_deref())
}

#[cfg(not(target_os = "windows"))]
fn shell_family(shell: &OsStr) -> &'static str {
    let name = Path::new(shell)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("");

    match name {
        "bash" | "zsh" | "sh" | "dash" | "ksh" | "fish" => "posix",
        "csh" | "tcsh" => "csh",
        "pwsh" | "pwsh-preview" | "powershell" => "pwsh",
        _ => "unknown",
    }
}

fn shell_arguments(shell: &OsStr) -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        let _ = shell;
        &[]
    }

    #[cfg(not(target_os = "windows"))]
    {
        // One direct shell process gets both login and interactive startup.
        // Do not invoke `sh -c "$SHELL -il"`: that would duplicate startup
        // and can recurse when a user's startup file launches the shell.
        // Combined "-il" is only valid for POSIX-family shells; csh/tcsh
        // reject the combined form (login must be the sole flag) and pwsh
        // has no equivalent flag at all, so an unconditional "-il" would
        // make those shells exit immediately instead of starting a terminal.
        match shell_family(shell) {
            "posix" => &["-il"],
            "csh" => &["-l"],
            _ => &[],
        }
    }
}

fn terminal_environment(cwd: &Path, cols: u16, rows: u16) -> Vec<(&'static str, OsString)> {
    let home = dirs::home_dir()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| cwd.to_path_buf());
    let path = std::env::var_os("PATH").unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        {
            OsString::from("C:\\Windows\\System32;C:\\Windows")
        }
        #[cfg(not(target_os = "windows"))]
        {
            OsString::from("/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin")
        }
    });
    let lang = std::env::var_os("LANG").unwrap_or_else(|| OsString::from("C.UTF-8"));

    vec![
        ("HOME", home.into_os_string()),
        ("PATH", path),
        ("LANG", lang),
        ("TERM", OsString::from("xterm-256color")),
        ("COLUMNS", OsString::from(cols.max(20).to_string())),
        ("LINES", OsString::from(rows.max(2).to_string())),
    ]
}

fn shell_command_builder(
    shell: OsString,
    cwd: &Path,
    cols: u16,
    rows: u16,
    home_override: Option<&OsStr>,
) -> CommandBuilder {
    let mut command = CommandBuilder::new(&shell);
    command.args(shell_arguments(&shell));
    command.cwd(cwd);
    command.env("SHELL", &shell);
    for (key, value) in terminal_environment(cwd, cols, rows) {
        command.env(key, value);
    }
    // Test-only: point HOME/ZDOTDIR at an isolated, dotfile-free directory so
    // regression probes exercise qedit's login/interactive argv and env
    // contract without depending on (and potentially hanging on) whatever
    // shell-integration hooks the developer's real dotfiles happen to source.
    // Production callers never pass this, so real sessions are unaffected.
    if let Some(home) = home_override {
        command.env("HOME", home);
        command.env("ZDOTDIR", home);
    }
    command
}

#[tauri::command]
fn terminal_spawn<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, SharedTerminalState>,
    cwd: String,
    cols: u16,
    rows: u16,
) -> Result<u32, String> {
    terminal_spawn_with_shell(app, state, cwd, cols, rows, shell_command(), None)
}

fn terminal_spawn_with_shell<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, SharedTerminalState>,
    cwd: String,
    cols: u16,
    rows: u16,
    shell: OsString,
    home_override: Option<OsString>,
) -> Result<u32, String> {
    let cwd = safe_home_path(&cwd)?;
    let pty = native_pty_system()
        .openpty(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Could not create terminal: {error}"))?;
    let command = shell_command_builder(shell, &cwd, cols, rows, home_override.as_deref());
    let mut child = pty
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Could not start terminal process: {error}"))?;
    drop(pty.slave);

    let master: SharedMaster = Arc::new(Mutex::new(pty.master));
    let reader = match master.lock() {
        Ok(master) => match master.try_clone_reader() {
            Ok(reader) => reader,
            Err(error) => {
                stop_child(&mut child);
                return Err(format!("Could not read terminal output: {error}"));
            }
        },
        Err(_) => {
            stop_child(&mut child);
            return Err("Terminal master lock was poisoned".to_string());
        }
    };
    let writer = match master.lock() {
        Ok(master) => match master.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                stop_child(&mut child);
                return Err(format!("Could not write to terminal: {error}"));
            }
        },
        Err(_) => {
            stop_child(&mut child);
            return Err("Terminal master lock was poisoned".to_string());
        }
    };
    let killer: SharedKiller = Arc::new(Mutex::new(child.clone_killer()));

    let session_id = {
        let mut terminals = match state.lock() {
            Ok(terminals) => terminals,
            Err(_) => {
                stop_child(&mut child);
                return Err("Terminal state lock was poisoned".to_string());
            }
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver};
    use std::time::{Duration, Instant};
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::Listener;

    fn test_app() -> tauri::App<MockRuntime> {
        mock_builder()
            .manage(Arc::new(Mutex::new(TerminalState::default())) as SharedTerminalState)
            .build(mock_context(noop_assets()))
            .expect("failed to build mock app")
    }

    fn field(payload: &str, key: &str) -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(payload).unwrap()[key].clone()
    }

    /// Drain terminal output until `marker` shows up, returning the transcript.
    fn read_until(rx: &Receiver<String>, marker: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut transcript = String::new();

        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(chunk) => {
                    transcript.push_str(&chunk);

                    if transcript.contains(marker) {
                        return transcript;
                    }
                }
                Err(_) => continue,
            }
        }

        panic!("timed out waiting for {marker:?}; transcript so far: {transcript:?}");
    }

    #[test]
    fn shell_selection_prefers_configured_shell_then_account_fallback() {
        assert_eq!(
            configured_shell(Some(OsStr::new("/bin/bash")), Some(OsStr::new("/bin/zsh"))),
            OsString::from("/bin/bash")
        );
        assert_eq!(
            configured_shell(None, Some(OsStr::new("/bin/zsh"))),
            OsString::from("/bin/zsh")
        );
        assert_eq!(configured_shell(None, None), OsString::from("/bin/sh"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn posix_shells_get_login_and_interactive_startup_flags() {
        for shell in ["/bin/zsh", "/bin/bash", "/bin/sh", "/bin/dash", "/bin/ksh", "/usr/bin/fish"] {
            assert_eq!(shell_arguments(OsStr::new(shell)), &["-il"]);
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn csh_family_gets_login_only_startup_flag() {
        for shell in ["/bin/csh", "/bin/tcsh"] {
            assert_eq!(shell_arguments(OsStr::new(shell)), &["-l"]);
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn non_posix_and_unknown_shells_get_no_startup_flags() {
        for shell in ["/usr/bin/pwsh", "/usr/local/bin/nu"] {
            assert!(shell_arguments(OsStr::new(shell)).is_empty());
        }
    }

    #[test]
    fn terminal_environment_sets_shell_essentials_and_dimensions() {
        let cwd = Path::new("/tmp");
        let environment = terminal_environment(cwd, 120, 40)
            .into_iter()
            .collect::<HashMap<_, _>>();

        assert_eq!(environment["TERM"], OsString::from("xterm-256color"));
        assert_eq!(environment["COLUMNS"], OsString::from("120"));
        assert_eq!(environment["LINES"], OsString::from("40"));
        assert!(environment["HOME"]
            .to_str()
            .is_some_and(|value| !value.is_empty()));
        assert!(environment["PATH"]
            .to_str()
            .is_some_and(|value| !value.is_empty()));
        assert!(environment["LANG"]
            .to_str()
            .is_some_and(|value| !value.is_empty()));
    }

    /// Create an empty, dotfile-free directory to use as a probe shell's HOME.
    /// Real developer dotfiles (~/.bash_profile, ~/.zshrc, ...) may source
    /// third-party shell-integration hooks that expect a live terminal
    /// handshake; under the test PTY harness that handshake never arrives,
    /// so the shell can hang before ever reaching the injected probe command.
    /// Isolating HOME (and ZDOTDIR, which zsh also honors) sidesteps that
    /// without touching the login/interactive argv or environment contract
    /// under test.
    fn isolated_home() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qedit-test-home-{}-{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("should create isolated home");
        dir
    }

    #[cfg(unix)]
    #[test]
    fn bash_and_zsh_start_as_interactive_login_shells_with_terminal_environment() {
        for shell in ["/bin/bash", "/bin/zsh"] {
            if !Path::new(shell).is_file() {
                continue;
            }

            let app = test_app();
            let handle = app.handle().clone();
            let (output_tx, output_rx) = channel::<String>();
            handle.listen("terminal://output", move |event| {
                let data = field(event.payload(), "data");
                let _ = output_tx.send(data.as_str().unwrap_or_default().to_string());
            });

            let home = isolated_home();
            let session_id = terminal_spawn_with_shell(
                handle.clone(),
                handle.state::<SharedTerminalState>(),
                env!("CARGO_MANIFEST_DIR").to_string(),
                80,
                24,
                OsString::from(shell),
                Some(home.clone().into_os_string()),
            )
            .expect("shell should spawn");
            terminal_write(
                handle.state::<SharedTerminalState>(),
                session_id,
                "printf 'qedit-s%s:%s:%s\\n' 'hell' \"$TERM\" \"$-\"\n".to_string(),
            )
            .expect("shell should accept input");

            let transcript = read_until(&output_rx, "qedit-shell:");
            let shell_flags = transcript
                .split("qedit-shell:")
                .nth(1)
                .and_then(|line| line.split(':').nth(1))
                .unwrap_or_default();
            assert!(
                transcript.contains("qedit-shell:xterm-256color:"),
                "{shell} should receive TERM, got: {transcript:?}"
            );
            assert!(
                shell_flags.contains('i'),
                "{shell} should be interactive, got: {transcript:?}"
            );
            terminal_close(handle.state::<SharedTerminalState>(), session_id)
                .expect("shell should close");
            let _ = std::fs::remove_dir_all(&home);
        }
    }

    #[test]
    fn terminal_session_runs_commands_resizes_and_cleans_up() {
        let app = test_app();
        let handle = app.handle().clone();
        let (output_tx, output_rx) = channel::<String>();
        let (exit_tx, exit_rx) = channel::<Option<u32>>();

        handle.listen("terminal://output", move |event| {
            let data = field(event.payload(), "data");
            let _ = output_tx.send(data.as_str().unwrap_or_default().to_string());
        });
        handle.listen("terminal://exit", move |event| {
            let code = field(event.payload(), "code");
            let _ = exit_tx.send(code.as_u64().map(|value| value as u32));
        });

        // The repository checkout is inside $HOME, so it is a valid project cwd.
        let project = env!("CARGO_MANIFEST_DIR").to_string();
        let session_id = terminal_spawn(
            handle.clone(),
            handle.state::<SharedTerminalState>(),
            project.clone(),
            80,
            24,
        )
        .expect("terminal should spawn");

        let state = handle.state::<SharedTerminalState>();

        terminal_write(
            state.clone(),
            session_id,
            "printf 'qedit-p%s:%s\\n' 'wd' \"$PWD\"\n".to_string(),
        )
        .expect("terminal should accept input");
        let transcript = read_until(&output_rx, "qedit-pwd:");
        println!("--- terminal transcript ---\n{transcript}\n--- end transcript ---");
        let canonical_project = std::fs::canonicalize(&project).unwrap();
        assert!(
            transcript.contains(&format!("qedit-pwd:{}", canonical_project.display())),
            "shell should start in the project directory, got: {transcript:?}"
        );

        terminal_write(
            state.clone(),
            session_id,
            "printf 'qedit-e%s:%s:%s:%s:%s:%s\\n' 'nv' \"$TERM\" \"$HOME\" \"$LANG\" \"$-\" \"$(command -v clear)\"\n".to_string(),
        )
        .expect("terminal should accept environment probe");
        let environment = read_until(&output_rx, "qedit-env:");
        println!("--- terminal environment ---\n{environment}\n--- end environment ---");
        assert!(
            environment.contains("qedit-env:xterm-256color:"),
            "shell should receive a compatible TERM, got: {environment:?}"
        );
        assert!(
            environment
                .split("qedit-env:")
                .nth(1)
                .and_then(|line| line.split(':').nth(3))
                .is_some_and(|flags| flags.contains('i')),
            "shell should be interactive, got: {environment:?}"
        );
        terminal_write(
            state.clone(),
            session_id,
            "clear >/dev/null; printf 'qedit-c%s:ok\\n' 'lear'\n".to_string(),
        )
        .expect("terminal should accept clear probe");
        let clear_output = read_until(&output_rx, "qedit-clear:ok");
        assert!(
            clear_output.contains("qedit-clear:ok"),
            "clear should run successfully, got: {clear_output:?}"
        );

        terminal_resize(state.clone(), session_id, 120, 40).expect("terminal should resize");
        terminal_write(
            state.clone(),
            session_id,
            "printf 'qedit-si%s:%s\\n' 'ze' \"$(stty size)\"\n".to_string(),
        )
        .expect("terminal should accept input");
        let resized = read_until(&output_rx, "qedit-size:");
        println!("--- resize transcript ---\n{resized}\n--- end transcript ---");
        assert!(
            resized.contains("qedit-size:40 120"),
            "resize should reach the PTY, got: {resized:?}"
        );

        terminal_close(state.clone(), session_id).expect("terminal should close");
        exit_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("closing a session should emit an exit event");
        assert!(
            state.lock().unwrap().sessions.is_empty(),
            "closed sessions must not stay registered"
        );

        assert_eq!(
            terminal_write(state, session_id, "echo late\n".to_string()),
            Err("Terminal session is closed".to_string())
        );
    }

    #[test]
    fn terminal_spawn_rejects_paths_outside_home() {
        let app = test_app();
        let handle = app.handle().clone();
        let error = terminal_spawn(
            handle.clone(),
            handle.state::<SharedTerminalState>(),
            "/tmp".to_string(),
            80,
            24,
        )
        .expect_err("terminals must stay inside the home directory");

        assert!(
            error.contains("home directory"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn exiting_the_app_kills_every_terminal() {
        let app = test_app();
        let handle = app.handle().clone();
        let (exit_tx, exit_rx) = channel::<Option<u32>>();
        handle.listen("terminal://exit", move |event| {
            let _ = exit_tx.send(field(event.payload(), "code").as_u64().map(|c| c as u32));
        });

        terminal_spawn(
            handle.clone(),
            handle.state::<SharedTerminalState>(),
            env!("CARGO_MANIFEST_DIR").to_string(),
            80,
            24,
        )
        .expect("terminal should spawn");

        kill_all_terminals(&handle.state::<SharedTerminalState>());

        exit_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("app exit should terminate running shells");
        assert!(handle
            .state::<SharedTerminalState>()
            .lock()
            .unwrap()
            .sessions
            .is_empty());
    }
}
