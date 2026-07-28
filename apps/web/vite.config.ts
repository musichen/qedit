import tailwindcss from '@tailwindcss/vite';
import { devtools } from '@tanstack/devtools-vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import viteReact from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

import { writeFileSync, readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const config = defineConfig({
  envPrefix: ['VITE_'],
  resolve: {
    tsconfigPaths: true,
  },
  ssr: {
    noExternal: [/^@qedit\//],
  },
  plugins: [
    devtools(),
    tailwindcss(),
    tanstackStart({
      importProtection: {
        behavior: 'error',
      },
    }),
    viteReact(),
    {
      name: 'generate-index-html',
      apply: 'build',
      closeBundle() {
        const template = readFileSync(join(__dirname, 'index.html'), 'utf-8');
        const distClient = join(__dirname, 'dist/client');
        if (!existsSync(distClient)) return;
        const assetsDir = join(distClient, 'assets');
        if (!existsSync(assetsDir)) return;
        const files = readdirSync(assetsDir);
        // The TanStack Start client entry chunk is named index-*.js
        const jsEntry = files.find(
          (f: string) => f.startsWith('index-') && f.endsWith('.js'),
        );
        const cssFiles = files.filter((f: string) => f.endsWith('.css'));
        let cssLink = '';
        let jsScript = '';
        if (jsEntry) {
          jsScript = `<script type="module" crossorigin src="/assets/${jsEntry}"></script>`;
        }
        for (const f of cssFiles) {
          cssLink += `<link rel="stylesheet" href="/assets/${f}">\n  `;
        }
        const html = template
          .replace('</head>', `${cssLink}</head>`)
          .replace(
            '<div id="root"></div>',
            `<div id="root"></div>\n  ${jsScript}`,
          );
        writeFileSync(join(distClient, 'index.html'), html);
        console.log('✓ Generated dist/client/index.html for Tauri');
      },
    },
  ],
});

const __filename = fileURLToPath(import.meta.url);
const __dirname = join(__filename, '..');

export default config;
