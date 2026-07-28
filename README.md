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

The desktop app is intentionally root-oriented: frontend code is in `src/`, and
Tauri code/configuration is in `src-tauri/`. Run `pnpm run tauri:dev` for the
desktop shell, `pnpm run check` for the fast validation pipeline, and
`pnpm run verify` for a native build plus smoke launch.

See `AGENTS.md` for the full command reference and architecture notes.
