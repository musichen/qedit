# qedit

rust-based lightweight editor

A Tauri 2 desktop app: a Rust shell around a TanStack Start + React frontend with
Monaco as the editing surface.

## Usage

- Start from the welcome screen: **Open File** loads a single file, **Open
  Folder** loads a project into the Explorer sidebar. Both use the native
  dialogs, and recently opened files and projects are listed for one click
  reopening.
- The Explorer lists the open workspace and expands folders on demand. Hidden
  entries, symlinks, and `node_modules`, `target`, `dist`, and `.turbo` are
  skipped.
- Open files stack in the tab bar; a dot marks unsaved edits. Closing a modified
  tab asks for confirmation and discards those edits, and opening another folder
  closes the current tabs the same way. Closing a clean tab drops its buffer, so
  reopening the file re-reads it from disk.
- A file that is still loading, or that failed to read, cannot be edited or
  saved - the status bar reports the reason. The status bar also shows the file
  path, cursor position, indentation, language, and any save error.
- An integrated terminal sits below the editor and runs your `$SHELL`
  (`%COMSPEC%` on Windows) in the open project directory, or your home directory
  when no folder is open. It
  follows the workspace root and its shells are terminated when the app exits.

### Shortcuts

| Shortcut | Action |
|---|---|
| **Cmd/Ctrl+O** | Open file |
| **Cmd/Ctrl+Shift+O** | Open folder |
| **Cmd/Ctrl+S** | Save |
| **Cmd/Ctrl+Shift+S** | Save as |
| **Cmd/Ctrl+W** | Close active tab |
| **Cmd/Ctrl+P** | Quick open |
| **Cmd/Ctrl+F** | Find in file |

File, folder, and terminal access is limited to `$HOME`. Recent files, recent
projects, and editor sessions are not yet persisted across restarts.

## Development

The desktop app is intentionally root-oriented: frontend code is in `src/`, and
Tauri code/configuration is in `src-tauri/`. Run `pnpm run tauri:dev` for the
desktop shell, `pnpm run check` for the fast validation pipeline, and
`pnpm run verify` for a native build plus smoke launch.

See `AGENTS.md` for the full command reference and architecture notes.
