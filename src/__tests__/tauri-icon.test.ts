import { describe, expect, it } from 'vitest';

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const tauriRoot = join(projectRoot, 'src-tauri');
const tauriConfig = readFileSync(join(tauriRoot, 'tauri.conf.json'), 'utf-8');

describe('Tauri app icon', () => {
  it('bundles the generated icon from the root Tauri config', () => {
    expect(tauriConfig).toContain('"icon": ["icons/icon.png"]');
    expect(existsSync(join(tauriRoot, 'icons', 'icon.png'))).toBe(true);
  });

  it('keeps the centered artwork source used to regenerate platform icons', () => {
    expect(
      existsSync(join(tauriRoot, 'icons', 'qedit_logo_centered.png')),
    ).toBe(true);
    expect(existsSync(join(tauriRoot, 'icons', 'qedit_logo.svg'))).toBe(true);
  });
});
