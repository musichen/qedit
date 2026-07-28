import tailwindcss from '@tailwindcss/vite';
import { devtools } from '@tanstack/devtools-vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import viteReact from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

import { writeFileSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const clientOutDir = resolve(__dirname, 'dist/client');

export default defineConfig({
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
      writeBundle(options, bundle) {
        // Only the client build produces the HTML the Tauri webview loads
        if (!options.dir || resolve(options.dir) !== clientOutDir) return;

        const entryChunks = Object.values(bundle).filter(
          (output) => output.type === 'chunk' && output.isEntry,
        );

        if (entryChunks.length !== 1) {
          this.error(
            `generate-index-html: expected exactly 1 entry chunk in the client bundle, found ${entryChunks.length}. Refusing to emit an index.html that would load a blank page.`,
          );
        }

        const jsEntry = entryChunks[0]!.fileName;
        const cssFiles = Object.keys(bundle).filter((f) => f.endsWith('.css'));

        const cssLinks = cssFiles
          .map((f) => `<link rel="stylesheet" href="/${f}">\n  `)
          .join('');
        const jsScript = `<script type="module" crossorigin src="/${jsEntry}"></script>`;

        const template = readFileSync(join(__dirname, 'index.html'), 'utf-8');
        const html = template
          .replace('</head>', `${cssLinks}</head>`)
          .replace(
            '<div id="root"></div>',
            `<div id="root"></div>\n  ${jsScript}`,
          );

        writeFileSync(join(clientOutDir, 'index.html'), html);
        console.log('✓ Generated dist/client/index.html for Tauri');
      },
    },
  ],
});
