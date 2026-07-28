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

  it('uses the vector source and keeps the complete generated icon set', () => {
    const iconRoot = join(tauriRoot, 'icons');
    const generatedIcons = [
      '32x32.png',
      '64x64.png',
      '128x128.png',
      '128x128@2x.png',
      'icon.png',
      'icon.icns',
      'icon.ico',
      'StoreLogo.png',
      'Square30x30Logo.png',
      'ios/AppIcon-512@2x.png',
      'android/mipmap-xxxhdpi/ic_launcher.png',
    ];

    expect(existsSync(join(iconRoot, 'qedit_logo.svg'))).toBe(true);
    expect(existsSync(join(iconRoot, 'qedit_logo_centered.png'))).toBe(false);
    for (const icon of generatedIcons) {
      expect(existsSync(join(iconRoot, icon))).toBe(true);
    }
  });
});
