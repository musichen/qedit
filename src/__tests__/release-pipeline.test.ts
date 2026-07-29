import { describe, expect, it } from 'vitest';

import { spawnSync } from 'node:child_process';
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const projectRoot = join(import.meta.dirname, '..', '..');
const targets = [
  'macos-arm64',
  'macos-x64',
  'windows-arm64',
  'windows-x64',
  'linux-arm64',
  'linux-x64',
] as const;

function createMatrix(version: string) {
  const root = mkdtempSync(join(tmpdir(), 'qedit-release-'));
  for (const target of targets) {
    const dir = join(root, target);
    mkdirSync(dir, { recursive: true });
    const platform = target.split('-')[0];
    const arch = target.split('-')[1];
    const artifacts =
      platform === 'macos'
        ? [
            `qedit-v${version}-macos-${arch}.app.zip`,
            `qedit-v${version}-macos-${arch}.dmg`,
          ]
        : platform === 'windows'
          ? [
              `qedit-v${version}-windows-${arch}.msi`,
              `qedit-v${version}-windows-${arch}-nsis.exe`,
            ]
          : [
              `qedit-v${version}-linux-${arch}.deb`,
              `qedit-v${version}-linux-${arch}.AppImage`,
            ];
    for (const artifact of artifacts)
      writeFileSync(join(dir, artifact), `${target}\n`);
    writeFileSync(
      join(dir, 'manifest.json'),
      `${JSON.stringify({
        schema: 1,
        product: 'qedit',
        version,
        target,
        host: 'test',
        artifacts,
      })}\n`,
    );
    writeFileSync(
      join(dir, 'signing.json'),
      `${JSON.stringify({
        schema: 1,
        target,
        version,
        status:
          platform === 'macos'
            ? 'signed-and-notarized'
            : platform === 'windows'
              ? 'signed'
              : 'not-applicable',
        reason: 'test fixture',
      })}\n`,
    );
  }
  return root;
}

describe('release pipeline contract', () => {
  it('routes all six native targets and removes the placeholder provenance workflow', () => {
    const workflow = readFileSync(
      join(projectRoot, '.github/workflows/release.yml'),
      'utf8',
    );
    expect(workflow).toContain('macos-14');
    expect(workflow).toContain('macos-15-intel');
    expect(workflow).toContain('windows-11-arm');
    expect(workflow).toContain('ubuntu-24.04-arm');
    expect(workflow.match(/target: /g)).toHaveLength(6);
    expect(workflow).toContain('actions/attest-build-provenance@v2');
  });

  it('verifies a complete matrix and emits checksums and provenance', () => {
    const root = createMatrix('0.1.0');
    try {
      const result = spawnSync(
        'bash',
        [join(projectRoot, 'scripts/release-verify.sh'), 'all', '0.1.0', root],
        {
          cwd: projectRoot,
          encoding: 'utf8',
          env: {
            ...process.env,
            QEDIT_REQUIRE_SIGNED: '1',
            SOURCE_DATE_EPOCH: '0',
          },
        },
      );
      expect(result.status).toBe(0);
      expect(readFileSync(join(root, 'SHA256SUMS'), 'utf8')).toContain(
        'macos-arm64',
      );
      expect(readFileSync(join(root, 'provenance.json'), 'utf8')).toContain(
        'qedit',
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it('fails closed when one target is missing', () => {
    const root = createMatrix('0.1.0');
    try {
      rmSync(join(root, 'windows-arm64'), { recursive: true, force: true });
      const result = spawnSync(
        'bash',
        [join(projectRoot, 'scripts/release-verify.sh'), 'all', '0.1.0', root],
        {
          cwd: projectRoot,
          encoding: 'utf8',
          env: { ...process.env, QEDIT_REQUIRE_SIGNED: '1' },
        },
      );
      expect(result.status).not.toBe(0);
      expect(`${result.stdout}${result.stderr}`).toContain('windows-arm64');
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it('rejects stale artifacts mixed into an otherwise complete target', () => {
    const root = createMatrix('0.1.0');
    try {
      writeFileSync(
        join(root, 'linux-x64', 'qedit-v0.0.9-linux-x64.deb'),
        'stale\n',
      );
      const result = spawnSync(
        'bash',
        [join(projectRoot, 'scripts/release-verify.sh'), 'all', '0.1.0', root],
        {
          cwd: projectRoot,
          encoding: 'utf8',
          env: { ...process.env, QEDIT_REQUIRE_SIGNED: '1' },
        },
      );
      expect(result.status).not.toBe(0);
      expect(`${result.stdout}${result.stderr}`).toContain('unexpected stale');
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
