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
- The Explorer toolbar creates a new empty file in the open workspace, refreshes
  after file mutations, and exposes filename/path search. New files stay inside
  the workspace and `$HOME`; cancellation and invalid destinations leave the
  workspace unchanged. Renaming refuses a destination that already exists.
- Open files stack in the tab bar; a dot marks unsaved edits. Closing a modified
  tab asks for confirmation and discards those edits, and opening another folder
  or quitting the app asks the same way. Closing a clean tab drops its buffer, so
  reopening the file re-reads it from disk. **Cmd/Ctrl+Shift+T** reopens recently
  closed tabs, and **Cmd/Ctrl+Shift+R** re-reads the active file from disk.
- **Save As** writes the buffer to a path you pick in the native dialog and moves
  the tab to it; it refuses a target that is already open in another tab.
- Quick open (**Cmd/Ctrl+P**) searches the workspace files and your recent files
  by name or path, with arrow keys, Home/End, and Enter to open.
- Markdown files open with a rendered Preview by default; the Edit/Preview
  toggle returns to Monaco editing. Rendering is local and safe, with readable
  headings, lists, code blocks, and restricted links.
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

## Release operator sequence

Releases use native runners: `macos-14` and `macos-15-intel`,
`windows-11-arm` and `windows-2022`, and `ubuntu-24.04-arm` and
`ubuntu-24.04`. The workflow refuses cross-compilation and partial matrices.
Windows ARM is a GitHub-hosted public-preview runner; if it is unavailable,
provision a self-hosted Windows ARM64 runner with `self-hosted,windows,ARM64`
labels or report that target as unsupported.

For a local dry-run:

```sh
pnpm run release:dry -- 0.1.0
```

The normal path is to commit matching version metadata on a non-default branch
and let `.github/workflows/release.yml` build, verify, attest, and publish all
six targets. macOS produces `.app.zip` plus `.dmg`; Windows produces MSI and
NSIS installers; Linux produces DEB and AppImage. The default is an explicitly
unsigned public preview: missing signing secrets do not block the build, and
each manifest, signing record, and release note names the actual status. To
make signing a release-blocking requirement, set the repository Actions
variable `QEDIT_REQUIRE_SIGNING=1` and configure every required secret:

- macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`
- Windows: `WINDOWS_CERTIFICATE_BASE64`, `WINDOWS_CERTIFICATE_PASSWORD`,
  `WINDOWS_TIMESTAMP_URL`

Linux packages remain `not-applicable` for code signing. A missing or partial
credential set never produces a signed status; it either records an unsigned
preview or fails with the exact missing values when signed publishing is
enabled.

For a verified local matrix:

```sh
QEDIT_REQUIRE_SIGNED=1 pnpm run release:verify -- all 0.1.0
QEDIT_RELEASE_ARTIFACTS=dist/release/v0.1.0 pnpm run release -- 0.1.0
```

If an upload fails after the tag exists, do not overwrite the artifact
directory. Re-run verification, inspect `gh-axi release view v<version>`, and
upload only missing assets with `gh-axi release upload v<version> <files...>`.
For a bad immutable release, confirm the exact tag, then use
`gh-axi release delete` and recreate it; never force-push a release tag.
