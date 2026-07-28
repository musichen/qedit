import tailwindcss from '@tailwindcss/vite';
import { devtools } from '@tanstack/devtools-vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import viteReact from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

import { writeFileSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

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
      // Only the client build produces the HTML the Tauri webview loads
      applyToEnvironment: (environment) => environment.name === 'client',
      writeBundle(options, bundle) {
        const outDir = options.dir;

        if (!outDir) {
          this.error(
            'generate-index-html: the client build produced no output directory, so index.html cannot be emitted for Tauri.',
          );
        }

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

        const htmlPath = join(outDir, 'index.html');
        writeFileSync(htmlPath, html);
        console.log(`✓ Generated ${htmlPath} for Tauri`);
      },
    },
  ],
});
