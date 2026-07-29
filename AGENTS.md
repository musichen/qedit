# Project agent memory

This file is the project's committed home for project-intrinsic agent knowledge: build, test, release, architecture, and sharp-edge notes that should travel with the code.

- Add durable project-specific notes here as they are discovered through real work.

## Architecture

- **Monorepo**: pnpm workspaces + Turbo (`turbo.json`, `pnpm-workspace.yaml`). The desktop app lives at the project root; reusable packages are under `packages/**` and shared tooling under `tooling/**`.
- **Frontend**: root `src/` — TanStack Start (React + Vite router). See `src/router.tsx` for route setup and `vite.config.ts` for plugins.
- **Desktop shell**: Tauri 2 in root `src-tauri/`. Plugins: `fs` (scope `$HOME` plus `$HOME/**`, declared in the capability so dialog-picked paths stay inside home), `shell` (open), `dialog`. Config at `src-tauri/tauri.conf.json`, capabilities at `src-tauri/capabilities/default.json`.
- **Static shell invariant**: Tauri loads `dist/client` as plain static files with no SSR server, so the build must emit a prerendered SPA shell at `dist/client/index.html` carrying the router bootstrap payload (`spa.prerender.outputPath: '/index'` in `vite.config.ts`). A hand-written or hand-generated `index.html` has no bootstrap payload and boots the app to a blank page, so there is deliberately no source `index.html` — the document markup lives in `src/routes/__root.tsx`. Guarded by `src/__tests__/spa-shell.test.ts`.
- **Generated Tauri files**: `src-tauri/gen/**` is regenerated (minified) by the Tauri CLI on every build and is excluded from oxfmt — never hand-format or hand-edit it.
- **macOS packaging**: `scripts/build-mac.sh` is the DMG entrypoint; it removes Tauri's `rw.*.dmg` temporary images, requests both `app,dmg` targets, and asserts both final bundles - an executable `qedit.app`, plus exactly one DMG named for the current `tauri.conf.json` version, so a stale final from an earlier version neither satisfies nor breaks the check. Sharp edge: create-dmg's optional Finder-prettifying AppleScript needs a live Finder plus Automation permission, so headless or unattended sessions abort the entire DMG with AppleEvent errors (-1712, -1719). Tauri swallows create-dmg's output and leaves only `error running bundle_dmg.sh`, so the script retries that failure once with `CI=true` (Tauri then passes `--skip-jenkins`) and warns that the DMG ships with **degraded presentation** - valid and mountable, but without custom icon positioning or background. Any other DMG failure fails the retry as well, and the final-bundle assertion still refuses a false success. Covered headlessly by `src/__tests__/mac-packaging.test.ts`, which drives the script against a stubbed `pnpm`.
- **App icons**: `src-tauri/icons/qedit_logo.svg` is the canonical source and the only artwork file kept in the repo. Sharp edge: it is an SVG wrapper around a 1024x1024 base64 PNG, not true vector art, so it cannot be rescaled above 1024px without loss and `src/__tests__/tauri-icon.test.ts` compares the generated icons against that embedded bitmap. Regenerate the full platform set with `pnpm tauri icon src-tauri/icons/qedit_logo.svg -o src-tauri/icons`; `src-tauri/tauri.conf.json` intentionally bundles the generated `icons/icon.png`.
- **Database**: `packages/db` - Drizzle ORM + SQLite schema in `src/schema.ts` (`sessions`, `recent_files`, `recent_projects`), config in `drizzle.config.ts`. Sharp edge: the `db` singleton exported from `src/client.ts` is still an **in-memory** store that only mirrors that schema - nothing is persisted across app restarts yet, and SQLite is not opened anywhere. Recent files/projects are therefore session-scoped; wiring the existing tables to native SQLite is the bounded follow-up.
- **Terminal**: the integrated terminal is a Rust-owned PTY (`portable-pty`) in `src-tauri/src/lib.rs` exposed as the `terminal_spawn`/`terminal_write`/`terminal_resize`/`terminal_close` commands, driven by an Xterm.js front end in `src/components/TerminalPanel.tsx`. The spawn command must set `TERM=xterm-256color`: GUI-launched Tauri processes do not inherit a terminal type, and interactive shell line editing otherwise redraws cursor/erase sequences incorrectly. Invariants, all covered by the `#[cfg(test)]` tests in `lib.rs`: every session cwd goes through `safe_home_path` (canonicalized and rejected outside `$HOME`), a closed session is deregistered so later writes fail, and `RunEvent::Exit` kills every live shell so no orphan processes survive the app.
- **Terminal tabs**: `TerminalPanel` keeps one mounted Xterm instance and native session per terminal tab, while `src/lib/terminal-tabs.ts` owns active/dirty/running/closed state, ordering, and close fallback behavior. Inactive terminals remain mounted but hidden so their output and shell state survive switching; only the active instance is fit, resized, and focused. Tab rename, drag/drop, keyboard reorder, and terminal-specific shortcuts are frontend/session-scoped.
- **Editor**: Monaco is bundled from the local `monaco-editor` package by root `src/lib/monaco-setup.ts`, which overrides `@monaco-editor/react`'s default CDN loader and wires the Vite `?worker` bundles. Keep it that way — the packaged desktop app must load the editor with no network access.
- **Workspace mutations and search**: native create/rename/delete/Save As operations are centralized in `src/lib/workspace-bridge.ts`; `WorkspaceContext` handles `qedit:workspace-refresh` so the Explorer invalidates immediately after disk changes. Quick Open scans visible workspace files on demand, while the tree remains lazy for browsing.
- **Markdown**: `.md` files use the local React-node renderer in `src/lib/markdown.tsx`; it intentionally supports a small safe subset and only enables `http(s)` and `mailto` links. Monaco remains the editing fallback through the Edit/Preview toggle in `src/components/Editor.tsx`.
- **UI library**: `packages/ui` — shadcn components (Tailwind v4) plus `cn()` utility.
- **Shared**: `packages/shared` — `cn` utility, event emitter context, registry pattern, mode utils.
- **Tooling**: `tooling/typescript` — shared `base.json` tsconfig.

## Commands

| Command | Scope |
|---|---|
| `pnpm install` | Install all workspace deps |
| `pnpm run dev` | Start the root Vite development server |
| `pnpm run tauri:dev` | Start the Tauri desktop development shell (which starts Vite) |
| `pnpm run lint` | oxlint across workspace |
| `pnpm run lint:shell` | shellcheck over `scripts/*.sh`; skips with a notice when shellcheck is absent unless `QEDIT_REQUIRE_SHELLCHECK=1` |
| `pnpm run format` | oxfmt --check |
| `pnpm run format:fix` | oxfmt auto-fix |
| `pnpm run typecheck` | Root tsc plus package typechecks via Turbo |
| `pnpm run test` | Root Vitest plus package tests via Turbo |
| `pnpm run build` | Root Vite production build |
| `pnpm run build:native` | Full Tauri desktop build (Rust + frontend) |
| `pnpm run db:generate` | drizzle-kit generate migrations |

### Check & verify pipeline

| Command | Scope |
|---|---|
| `pnpm run check` | Fast CI — lint, format, typecheck, web tests, Rust tests, Vite build, + Rust compilation check |
| `pnpm run check:native` | Rust-only: `cargo check` on the Tauri workspace (catches broken plugin configs, JSON errors, compilation failures without a full build) |
| `pnpm run test:native` | Rust unit tests (terminal/PTY behaviour) via `cargo test`. Spawns real shells through the `tauri` `test` feature; part of `pnpm run check` |
| `pnpm run build:native` | Full Tauri native build (app bundle only, no DMG/installer) |
| `pnpm run smoke:native` | `scripts/native-smoke.sh` — launches the built app, fails if it exits during startup or ignores SIGTERM. On macOS it launches the packaged `.app` and asserts the embedded `qedit.icns` matches the current artwork |
| `pnpm run verify` | Full pipeline: `check` + `build:native` + `smoke:native` |
| `pnpm run build:mac` | `scripts/build-mac.sh` - DMG release bundle. Sharp edge: it builds `app,dmg` because the dmg-only target deletes `qedit.app` as an intermediate; `CI=true` is set only on the retry after the Finder presentation step fails (see the macOS packaging note above) |
| `pnpm run build:all` | Host bundle only (`scripts/build-all.sh`): Tauri cannot cross-build complete bundles, so CI must run it once per platform runner |
| `pnpm run release <version>` | `scripts/release.sh` — refuses unless the version matches `package.json`, `tauri.conf.json`, and `Cargo.toml`; set `DRY_RUN=1` to stop before tagging |

Release matrix: `scripts/release-build.sh` only accepts a native Rust host triple for each target (`macos-{arm64,x64}`, `windows-{arm64,x64}`, `linux-{arm64,x64}`), then writes deterministic artifacts below `dist/release/v<version>/<target>`. Linux AppImage bundling uses the `bundle.linux.appimage.files` mapping in `src-tauri/tauri.conf.json` to provide the shell plugin's opener without requiring host `xdg-utils`; the command also sets explicit CI/headless environment variables. `scripts/release-sign-prepare.sh` installs the macOS identity before Tauri creates the DMG when credentials are configured; otherwise the workflow records an unsigned public preview. `scripts/release-sign.sh` records signed/unsigned status, and `QEDIT_REQUIRE_SIGNING=1` makes missing credentials release-blocking. `scripts/release-verify.sh all` refuses incomplete or stale matrices, requires matching per-target signing metadata, and emits `SHA256SUMS` plus deterministic provenance. The authoritative hosted-runner matrix and publish flow are `.github/workflows/release.yml`; local publication uses `scripts/release.sh` and `gh-axi`.

## Constraints

- TypeScript strict mode throughout (`strict: true`, `noUnusedLocals`, `noUnusedParameters`).
- `verbatimModuleSyntax: true` — use `import type` for type-only imports.
- ESM (`"type": "module"`).
- No Supabase, Stripe, auth, or SaaS code. This is a file editor.
- Tauri fs scope limited to `$HOME` and `$HOME/**`. No network permissions beyond localhost.

## Maintaining this file

Keep this file for knowledge useful to almost every future agent session in this project.
Do not repeat what the codebase already shows; point to the authoritative file or command instead.
Prefer rewriting or pruning existing entries over appending new ones.
When updating this file, preserve this bar for all agents and keep entries concise.
