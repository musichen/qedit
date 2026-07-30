# qedit website

The qedit marketing site is a small Vite + React static app. It builds to
`website/dist` and has no runtime service or API dependency.

## Local development

```sh
pnpm install
pnpm --filter @qedit/website dev
pnpm --filter @qedit/website build
pnpm --filter @qedit/website test:unit
```

The project-pages build uses `/qedit/` as its base path. The Pages workflow
sets `VITE_BASE_PATH` explicitly. Once deployed, the site is available at
<https://musichen.github.io/qedit/>. If the site later moves to a repository
named `qedit.github.io` or to a custom domain, set `VITE_BASE_PATH=/` in the
workflow and configure the domain in the repository's Pages settings; no source
code changes are needed. External URLs and release metadata live in
`website/src/content.ts`.
