# qedit

rust-based lightweight editor

A Tauri 2 desktop app: a Rust shell around a TanStack Start + React frontend with
Monaco as the editing surface.

## Usage

- The sidebar lists your home directory; click a file to open it in a tab.
  Recently opened files appear under **Recent**.
- Open files stack in the tab bar. Closing a tab with unsaved edits keeps the
  buffer, so reopening the file restores those edits; closing a clean tab
  re-reads it from disk.
- Save the active file with **Cmd+S** (**Ctrl+S** on Windows/Linux). A file that
  is still loading, or that failed to read, cannot be edited or saved — the
  status bar reports the reason.
- The status bar shows the file path, cursor position, indentation, language,
  and any save error.

File access is limited to `$HOME`. Recent files and editor sessions are not yet
persisted across restarts.

## Development

See `AGENTS.md` for the command reference and architecture notes.
