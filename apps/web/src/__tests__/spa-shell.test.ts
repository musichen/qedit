import { describe, it, expect } from 'vitest';

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Tauri serves dist/client as plain static files, so the client entry can only
// hydrate if the build emits a prerendered SPA shell carrying the router
// bootstrap payload. A hand-written index.html has no bootstrap and boots to a
// blank page, so guard both halves of that contract here.
const webRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const viteConfig = readFileSync(join(webRoot, 'vite.config.ts'), 'utf-8');

describe('Tauri static client build', () => {
  it('enables TanStack Start SPA mode', () => {
    expect(viteConfig).toMatch(/spa:\s*{\s*enabled:\s*true/);
  });

  it('writes the SPA shell to the index.html Tauri loads', () => {
    expect(viteConfig).toMatch(/outputPath:\s*'\/index'/);
  });

  it('does not hand-generate an index.html without a bootstrap payload', () => {
    expect(viteConfig).not.toContain('generate-index-html');
    expect(viteConfig).not.toContain('writeFileSync');
  });
});

describe('client entry', () => {
  it('hydrates the prerendered document with StartClient', () => {
    const clientEntry = readFileSync(
      join(webRoot, 'src', 'client.tsx'),
      'utf-8',
    );

    expect(clientEntry).toContain('StartClient');
    expect(clientEntry).toContain('hydrateRoot');
  });
});
