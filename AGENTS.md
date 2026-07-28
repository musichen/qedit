# Project agent memory

This file is the project's committed home for project-intrinsic agent knowledge: build, test, release, architecture, and sharp-edge notes that should travel with the code.

- Add durable project-specific notes here as they are discovered through real work.

## Architecture

- **Monorepo**: pnpm workspaces + Turbo (`turbo.json`, `pnpm-workspace.yaml`). Packages under `apps/*`, `packages/**`, `tooling/*`.
- **Frontend**: `apps/web` — TanStack Start (React + Vite router). See `apps/web/src/router.tsx` for route setup, `apps/web/vite.config.ts` for plugins.
- **Desktop shell**: Tauri 2 in `apps/web/src-tauri/`. Plugins: `fs` (scope `$HOME/**`), `shell` (open), `dialog`. Config at `tauri.conf.json`, capabilities at `capabilities/default.json`.
- **Static shell invariant**: Tauri loads `dist/client` as plain static files with no SSR server, so the build must emit a prerendered SPA shell at `dist/client/index.html` carrying the router bootstrap payload (`spa.prerender.outputPath: '/index'` in `vite.config.ts`). A hand-written or hand-generated `index.html` has no bootstrap payload and boots the app to a blank page, so there is deliberately no source `apps/web/index.html` — the document markup lives in `src/routes/__root.tsx`. Guarded by `apps/web/src/__tests__/spa-shell.test.ts`.
- **Generated Tauri files**: `apps/web/src-tauri/gen/**` is regenerated (minified) by the Tauri CLI on every build and is excluded from oxfmt — never hand-format or hand-edit it.
- **Database**: `packages/db` — Drizzle ORM + SQLite schema in `src/schema.ts` (`sessions`, `recent_files`), config in `drizzle.config.ts`. Sharp edge: the `db` singleton exported from `src/client.ts` is still an **in-memory** store that only mirrors that schema — nothing is persisted across app restarts yet, and SQLite is not opened anywhere.
- **Editor**: Monaco is bundled from the local `monaco-editor` package by `apps/web/src/lib/monaco-setup.ts`, which overrides `@monaco-editor/react`'s default CDN loader and wires the Vite `?worker` bundles. Keep it that way — the packaged desktop app must load the editor with no network access.
- **UI library**: `packages/ui` — shadcn components (Tailwind v4) plus `cn()` utility.
- **Shared**: `packages/shared` — `cn` utility, event emitter context, registry pattern, mode utils.
- **Tooling**: `tooling/typescript` — shared `base.json` tsconfig.

## Commands

| Command | Scope |
|---|---|
| `pnpm install` | Install all workspace deps |
| `pnpm run dev` | Start all dev servers (Tauri: `pnpm --filter @qedit/web tauri dev`) |
| `pnpm run lint` | oxlint across workspace |
| `pnpm run format` | oxfmt --check |
| `pnpm run format:fix` | oxfmt auto-fix |
| `pnpm run typecheck` | tsc --noEmit per workspace via Turbo |
| `pnpm run test` | vitest across workspace via Turbo |
| `pnpm run build` | Vite build via Turbo |
| `cd apps/web && pnpm tauri build` | Full Tauri desktop build (Rust + frontend) |
| `pnpm run db:generate` | drizzle-kit generate migrations |

### Check & verify pipeline

| Command | Scope |
|---|---|
| `pnpm run check` | Fast CI — lint, format, typecheck, tests, Vite build, + Rust compilation check |
| `pnpm run check:native` | Rust-only: `cargo check` on the Tauri workspace (catches broken plugin configs, JSON errors, compilation failures without a full build) |
| `pnpm run build:native` | Full Tauri native build (app bundle only, no DMG/installer) |
| `pnpm run verify` | Full pipeline: `check` + native build + smoke launch test (runs the binary for 3s, kills it, fails if it panics) |

## Constraints

- TypeScript strict mode throughout (`strict: true`, `noUnusedLocals`, `noUnusedParameters`).
- `verbatimModuleSyntax: true` — use `import type` for type-only imports.
- ESM (`"type": "module"`).
- No Supabase, Stripe, auth, or SaaS code. This is a file editor.
- Tauri fs scope limited to `$HOME/**`. No network permissions beyond localhost.

## Maintaining this file

Keep this file for knowledge useful to almost every future agent session in this project.
Do not repeat what the codebase already shows; point to the authoritative file or command instead.
Prefer rewriting or pruning existing entries over appending new ones.
When updating this file, preserve this bar for all agents and keep entries concise.
