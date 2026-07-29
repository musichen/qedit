import { describe, expect, it } from 'vitest';

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const projectRoot = join(import.meta.dirname, '..', '..');
const packageJson = JSON.parse(
  readFileSync(join(projectRoot, 'package.json'), 'utf-8'),
) as { scripts?: Record<string, string> };

describe('macOS packaging lifecycle', () => {
  it('uses the checked-in clean build wrapper', () => {
    expect(packageJson.scripts?.['build:mac']).toBe(
      'bash scripts/build-mac.sh',
    );
    expect(existsSync(join(projectRoot, 'scripts', 'build-mac.sh'))).toBe(true);
  });

  it('requests both the app and DMG targets and cleans Tauri temp images', () => {
    const buildScript = readFileSync(
      join(projectRoot, 'scripts', 'build-mac.sh'),
      'utf-8',
    );

    expect(buildScript).toContain('tauri build --bundles app,dmg');
    expect(buildScript).toMatch(/rw\.\*\.dmg/);
    expect(buildScript).toContain('trap cleanup_dmg_temps EXIT');
  });

  it('excludes Tauri temp images from the final DMG assertion', () => {
    const buildScript = readFileSync(
      join(projectRoot, 'scripts', 'build-mac.sh'),
      'utf-8',
    );

    const assertionStart = buildScript.indexOf('DMG_PATHS=()');
    expect(assertionStart).toBeGreaterThan(-1);

    const assertionBlock = buildScript.slice(assertionStart);
    expect(assertionBlock).toMatch(
      /== rw\.\*\.dmg \]\][\s\S]*continue[\s\S]*\$\{#DMG_PATHS\[@\]\} != 1/,
    );
  });

  it('keeps macOS target installation tolerant of a missing rustup', () => {
    const buildScript = readFileSync(
      join(projectRoot, 'scripts', 'build-mac.sh'),
      'utf-8',
    );

    expect(buildScript).toMatch(
      /if command -v rustup[\s\S]*rustup target add aarch64-apple-darwin x86_64-apple-darwin \|\| true/,
    );
  });

  it('does not collect Tauri temporary images as release artifacts', () => {
    const releaseScript = readFileSync(
      join(projectRoot, 'scripts', 'release.sh'),
      'utf-8',
    );

    expect(releaseScript).toContain('! -name "rw.*.dmg"');
  });
});
