import { afterEach, describe, expect, it } from 'vitest';

import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
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
    expect(buildScript).toContain('trap cleanup EXIT');
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

    expect(releaseScript).toMatch(/! -name ['"]rw\.\*\.dmg['"]/);
  });
});

/**
 * Drives scripts/build-mac.sh against a throwaway project root with a stubbed
 * `pnpm`, so the AppleScript fallback can be exercised headlessly without
 * running a real Tauri build.
 */
type StubMode = 'dmg-stage-then-ok' | 'dmg-stage-always' | 'unrelated-failure';

// Tauri swallows create-dmg's own output, so a Finder/AppleEvent failure only
// ever surfaces as this generic bundle_dmg.sh error.
const DMG_FAILURE_OUTPUT = [
  'Running bundle_dmg.sh',
  'failed to bundle project: error running bundle_dmg.sh',
];

const STUB_PNPM = `#!/usr/bin/env bash
set -euo pipefail

printf 'CI=%s\\n' "\${CI-}" >> "$STUB_DIR/invocations"
attempt=$(wc -l < "$STUB_DIR/invocations" | tr -d ' ')

bundle="$STUB_PROJECT_ROOT/src-tauri/target/release/bundle"
mkdir -p "$bundle/dmg" "$bundle/macos"
# Every bundling attempt leaves a Tauri staging image behind.
: > "$bundle/dmg/rw.$attempt.qedit_0.1.0_aarch64.dmg"

fail_dmg_stage() {
${DMG_FAILURE_OUTPUT.map((line) => `  echo "${line}"`).join('\n')}
  exit 1
}

case "$STUB_MODE" in
  dmg-stage-then-ok)
    if [[ "\${CI-}" == "true" ]]; then
      mkdir -p "$bundle/macos/qedit.app"
      : > "$bundle/dmg/qedit_0.1.0_aarch64.dmg"
      exit 0
    fi
    fail_dmg_stage
    ;;
  dmg-stage-always) fail_dmg_stage ;;
  unrelated-failure)
    echo "error: could not compile \\\`qedit\\\`" >&2
    exit 101
    ;;
esac
`;

let workspace: string | undefined;

function runBuildMac(mode: StubMode) {
  workspace = mkdtempSync(join(tmpdir(), 'qedit-build-mac-'));
  const fakeRoot = join(workspace, 'project');
  const stubDir = join(workspace, 'stub');
  mkdirSync(join(fakeRoot, 'scripts'), { recursive: true });
  mkdirSync(stubDir);

  copyFileSync(
    join(projectRoot, 'scripts', 'build-mac.sh'),
    join(fakeRoot, 'scripts', 'build-mac.sh'),
  );
  writeFileSync(join(stubDir, 'pnpm'), STUB_PNPM, { mode: 0o755 });
  chmodSync(join(stubDir, 'pnpm'), 0o755);
  writeFileSync(join(stubDir, 'invocations'), '');

  const result = spawnSync(
    'bash',
    [join(fakeRoot, 'scripts', 'build-mac.sh')],
    {
      encoding: 'utf-8',
      env: {
        // A minimal PATH keeps a real rustup out of the stubbed run.
        PATH: `${stubDir}:/usr/bin:/bin`,
        HOME: workspace,
        STUB_DIR: stubDir,
        STUB_PROJECT_ROOT: fakeRoot,
        STUB_MODE: mode,
      },
    },
  );

  const bundleDir = join(fakeRoot, 'src-tauri', 'target', 'release', 'bundle');

  return {
    status: result.status,
    output: `${result.stdout}${result.stderr}`,
    attempts: readFileSync(join(stubDir, 'invocations'), 'utf-8')
      .split('\n')
      .filter(Boolean),
    dmgFiles: existsSync(join(bundleDir, 'dmg'))
      ? readdirSync(join(bundleDir, 'dmg')).sort()
      : [],
    appExists: existsSync(join(bundleDir, 'macos', 'qedit.app')),
  };
}

afterEach(() => {
  if (workspace) {
    rmSync(workspace, { recursive: true, force: true });
    workspace = undefined;
  }
});

describe('macOS DMG build without Finder automation', () => {
  it('retries with DMG presentation disabled and still emits the bundles', () => {
    const run = runBuildMac('dmg-stage-then-ok');

    expect(run.status).toBe(0);
    expect(run.attempts).toEqual(['CI=', 'CI=true']);
    expect(run.output).toContain('could not drive Finder');
    expect(run.output).toContain('without custom icon positioning');
    expect(run.appExists).toBe(true);
    expect(run.dmgFiles).toEqual(['qedit_0.1.0_aarch64.dmg']);
  });

  it('keeps other bundling failures fatal without a retry', () => {
    const run = runBuildMac('unrelated-failure');

    expect(run.status).toBe(1);
    expect(run.attempts).toEqual(['CI=']);
    expect(run.output).toContain('macOS bundling failed.');
    expect(run.dmgFiles).toEqual([]);
  });

  it('fails when even the degraded DMG build cannot produce a bundle', () => {
    const run = runBuildMac('dmg-stage-always');

    expect(run.status).toBe(1);
    expect(run.attempts).toEqual(['CI=', 'CI=true']);
    expect(run.output).toContain(
      'failed even with the DMG presentation step disabled',
    );
    expect(run.appExists).toBe(false);
    expect(run.dmgFiles).toEqual([]);
  });
});
