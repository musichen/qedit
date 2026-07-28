import tailwindcss from '@tailwindcss/vite';
import { devtools } from '@tanstack/devtools-vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import viteReact from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import { resolve } from 'path';

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
        const { writeFileSync, readFileSync, readdirSync, existsSync } = require('fs');
        const { join } = require('path');
        const template = readFileSync(resolve(__dirname, 'index.html'), 'utf-8');
        const distClient = resolve(__dirname, 'dist/client');
        if (!existsSync(distClient)) return;
        const assetsDir = join(distClient, 'assets');
        if (!existsSync(assetsDir)) return;
        const files = readdirSync(assetsDir);
        const jsEntry = files.find(f => f.startsWith('dist-js-') && f.endsWith('.js'));
        const cssFiles = files.filter(f => f.endsWith('.css'));
        let cssLink = '';
        let jsScript = '';
        if (jsEntry) {
          jsScript = `<script type="module" crossorigin src="/assets/${jsEntry}"></script>`;
        }
        cssFiles.forEach(f => {
          cssLink += `<link rel="stylesheet" href="/assets/${f}">\n  `;
        });
        const html = template
          .replace('</head>', `${cssLink}</head>`)
          .replace('<div id="root"></div>', `<div id="root"></div>\n  ${jsScript}`);
        writeFileSync(join(distClient, 'index.html'), html);
        console.log('✓ Generated dist/client/index.html for Tauri');
      },
    },
  ],
});

export default config;
