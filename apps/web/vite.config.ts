import tailwindcss from '@tailwindcss/vite';
import { devtools } from '@tanstack/devtools-vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import viteReact from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

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
      // Tauri serves dist/client as plain static files with no SSR server, so
      // the build must prerender a SPA shell that carries the router bootstrap
      // payload the client entry hydrates against. `outputPath: '/index'` makes
      // that shell dist/client/index.html, which is what the webview loads.
      spa: {
        enabled: true,
        prerender: {
          outputPath: '/index',
        },
      },
      importProtection: {
        behavior: 'error',
      },
    }),
    viteReact(),
  ],
});
