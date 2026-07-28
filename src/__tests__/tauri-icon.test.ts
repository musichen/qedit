import { describe, expect, it } from 'vitest';

import { averageGrid, decodePng, maxGridDifference } from './support/png';

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const tauriRoot = join(projectRoot, 'src-tauri');
const iconRoot = join(tauriRoot, 'icons');
const tauriConfig = readFileSync(join(tauriRoot, 'tauri.conf.json'), 'utf-8');

const GRID_SIZE = 8;
const MAX_CHANNEL_DRIFT = 12;

function sourceArtwork() {
  const svg = readFileSync(join(iconRoot, 'qedit_logo.svg'), 'utf-8');
  const embedded = svg.match(
    /data:image\/png;base64,(?<payload>[A-Za-z0-9+/=]+)/,
  );
  const payload = embedded?.groups?.payload;
  if (!payload) {
    throw new Error('qedit_logo.svg no longer embeds a base64 PNG');
  }
  return decodePng(Buffer.from(payload, 'base64'));
}

describe('Tauri app icon', () => {
  it('bundles the generated icon from the root Tauri config', () => {
    expect(tauriConfig).toContain('"icon": ["icons/icon.png"]');
    expect(existsSync(join(iconRoot, 'icon.png'))).toBe(true);
  });

  it('uses the vector source and keeps the complete generated icon set', () => {
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

  // Only the full-bleed resamples are comparable; the platform variants (iOS,
  // Android, Square*) are padded or alpha-flattened by the icon generator.
  it.each(['icon.png', '128x128@2x.png', '128x128.png'])(
    'generates %s from the current qedit_logo.svg artwork',
    (icon) => {
      const source = averageGrid(sourceArtwork(), GRID_SIZE);
      const generated = averageGrid(
        decodePng(readFileSync(join(iconRoot, icon))),
        GRID_SIZE,
      );

      expect(maxGridDifference(source, generated)).toBeLessThan(
        MAX_CHANNEL_DRIFT,
      );
    },
  );
});
