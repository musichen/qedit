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
  or quitting the app asks the same way. Closing a clean tab drops its buffer, so
  reopening the file re-reads it from disk. **Cmd/Ctrl+Shift+T** reopens recently
  closed tabs, and **Cmd/Ctrl+Shift+R** re-reads the active file from disk.
- **Save As** writes the buffer to a path you pick in the native dialog and moves
  the tab to it; it refuses a target that is already open in another tab.
- Quick open (**Cmd/Ctrl+P**) searches the workspace files and your recent files
  by name or path, with arrow keys, Home/End, and Enter to open.
- A file that is still loading, or that failed to read, cannot be edited or
  saved - the status bar reports the reason. The status bar also shows the file
  path, cursor position, indentation, language, the current state (loading,
  unsaved, saving, saved, or failed), and any save error.
- An integrated terminal panel sits below the editor and runs your `$SHELL`
  (`%COMSPEC%` on Windows) in the open project directory, or your home directory
  when no folder is open. It follows the workspace root, reports each tab's state
  (starting, running, exited, or error), and its shells are terminated when the
  app exits. **Cmd/Ctrl+\`** moves focus between the editor and the terminal.
- The terminal panel supports multiple tabs: the **+** button opens another
  session, double-click or **F2** renames a tab, drag-and-drop or
  **Cmd/Ctrl+Arrow Left/Right** reorders tabs, and switching tabs keeps every
  session's output and shell state alive in the background.

### Shortcuts

| Shortcut | Action |
|---|---|
| **Cmd/Ctrl+O** | Open file |
| **Cmd/Ctrl+Shift+O** | Open folder |
| **Cmd/Ctrl+S** | Save |
| **Cmd/Ctrl+Shift+S** | Save as |
| **Cmd/Ctrl+W** | Close active tab |
| **Cmd/Ctrl+Shift+T** | Reopen last closed tab |
| **Cmd/Ctrl+Shift+R** | Reload active file from disk |
| **Cmd/Ctrl+PageDown** / **Ctrl+Tab** | Next tab |
| **Cmd/Ctrl+PageUp** / **Ctrl+Shift+Tab** | Previous tab |
| **Cmd/Ctrl+\`** | Toggle focus between editor and terminal |
| **Cmd/Ctrl+PageDown** / **Ctrl+Tab** (in terminal) | Next terminal tab |
| **Cmd/Ctrl+PageUp** / **Ctrl+Shift+Tab** (in terminal) | Previous terminal tab |
| **Cmd/Ctrl+1-9** (in terminal) | Jump to terminal tab N |
| **Cmd/Ctrl+W** (in terminal) | Close active terminal tab |
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
