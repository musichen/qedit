import { describe, expect, it } from 'vitest';

import { spawnSync } from 'node:child_process';
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
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
        signing: {
          status:
            platform === 'macos'
              ? 'signed-and-notarized'
              : platform === 'windows'
                ? 'signed'
                : 'not-applicable',
          reason: 'test fixture',
        },
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
    expect(workflow).toContain('QEDIT_REQUIRE_SIGNING');
    expect(workflow).toContain('Resolve macOS signing mode');
    expect(workflow).toContain("steps.macos-signing.outputs.enabled == 'true'");
    expect(workflow).not.toContain(
      "APPLE_SIGNING_IDENTITY: ${{ steps.macos-signing.outputs.enabled == 'true' && secrets.APPLE_SIGNING_IDENTITY || '' }}",
    );
    expect(workflow).not.toMatch(/APPLE_SIGNING_IDENTITY:\s*''/);
    const signedBuild =
      workflow.match(
        /      - name: Build native bundle with macOS signing[\s\S]*?(?=\n      - name:)/,
      )?.[0] ?? '';
    expect(signedBuild).toContain(
      "if: startsWith(matrix.target, 'macos-') && steps.macos-signing.outputs.enabled == 'true'",
    );
    expect(signedBuild).toContain(
      'APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}',
    );
    const unsignedBuild =
      workflow.match(
        /      - name: Build native bundle unsigned macOS preview[\s\S]*?(?=\n      - name:)/,
      )?.[0] ?? '';
    expect(unsignedBuild).toContain(
      "if: startsWith(matrix.target, 'macos-') && steps.macos-signing.outputs.enabled != 'true'",
    );
    expect(unsignedBuild).not.toMatch(/^\s+APPLE_SIGNING_IDENTITY:/m);
    expect(unsignedBuild).not.toContain('\n        env:');
    expect(workflow).toContain('CI: true');
    expect(workflow).toContain('path: dist/release');
    expect(workflow).not.toContain('xdg-utils');
  });

  it('keeps Linux bundling headless and target verification scoped to its artifact directory', () => {
    const releaseBuild = readFileSync(
      join(projectRoot, 'scripts/release-build.sh'),
      'utf8',
    );
    const workflow = readFileSync(
      join(projectRoot, '.github/workflows/release.yml'),
      'utf8',
    );
    expect(releaseBuild).toContain(
      'CI=true NO_AT_BRIDGE=1 pnpm exec tauri build --bundles deb,appimage',
    );
    const macBuild = readFileSync(
      join(projectRoot, 'scripts/build-mac.sh'),
      'utf8',
    );
    expect(macBuild).toContain('codesign --sign - --force --deep');
    expect(macBuild).toContain('pnpm exec tauri build --bundles app');
    expect(macBuild).toContain('pnpm exec tauri bundle --bundles dmg');
    expect(workflow).not.toContain("QEDIT_REQUIRE_SIGNED: '1'");
    expect(workflow).toContain(
      'dist/release/v${{ steps.version.outputs.version }}/${{ matrix.target }}',
    );
  });

  it('uses a local automation identity and rerun-safe release refs', () => {
    const workflow = readFileSync(
      join(projectRoot, '.github/workflows/release.yml'),
      'utf8',
    );
    const tagStep =
      workflow.match(
        /      - name: Ensure release branch and tag[\s\S]*?(?=\n      - name: Generate release notes)/,
      )?.[0] ?? '';

    expect(tagStep).toContain(
      "git config --local user.name 'qedit release automation'",
    );
    expect(tagStep).toContain(
      "git config --local user.email 'qedit-release@localhost'",
    );
    expect(tagStep).not.toContain('git config --global');
    expect(tagStep).toContain('refs/heads/$branch');
    expect(tagStep).toContain('refs/tags/$tag');
    expect(tagStep).toContain('git show-ref --verify --quiet');
    expect(tagStep).toContain('git tag -a "$tag" -m "Release $tag"');
  });

  it('bundles a portable xdg-open fallback for AppImage builds', () => {
    const config = JSON.parse(
      readFileSync(join(projectRoot, 'src-tauri/tauri.conf.json'), 'utf8'),
    );
    const source = config.bundle.linux.appimage.files['/usr/bin/xdg-open'];
    const shimPath = join(projectRoot, 'src-tauri', source);
    const shim = readFileSync(shimPath, 'utf8');

    expect(source).toBe('files/xdg-open');
    expect(config.plugins.shell.open).toBe(true);
    expect(JSON.stringify(config)).not.toContain('signingIdentity');
    expect(statSync(shimPath).mode & 0o111).not.toBe(0);
    expect(shim).toContain('/usr/bin/xdg-open');
    expect(shim).toContain('gio open');
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

  it('fails closed when macOS signing is required but credentials are incomplete', () => {
    const root = mkdtempSync(join(tmpdir(), 'qedit-signing-required-'));
    const manifestPath = join(root, 'manifest.json');
    const env = {
      ...process.env,
      QEDIT_REQUIRE_SIGNING: '1',
      APPLE_CERTIFICATE: '',
      APPLE_CERTIFICATE_PASSWORD: '',
      APPLE_SIGNING_IDENTITY: 'Developer ID Application: stale identity',
      APPLE_ID: '',
      APPLE_PASSWORD: '',
      APPLE_TEAM_ID: '',
    };
    try {
      const prepare = spawnSync(
        'bash',
        [join(projectRoot, 'scripts/release-sign-prepare.sh'), 'macos-arm64'],
        { cwd: projectRoot, encoding: 'utf8', env },
      );
      expect(prepare.status).not.toBe(0);
      expect(`${prepare.stdout}${prepare.stderr}`).toContain(
        'macOS signing is required',
      );

      writeFileSync(
        manifestPath,
        `${JSON.stringify({
          schema: 1,
          product: 'qedit',
          version: '0.1.0',
          target: 'macos-arm64',
          host: 'test',
          artifacts: [],
        })}\n`,
      );
      const sign = spawnSync(
        'bash',
        [
          join(projectRoot, 'scripts/release-sign.sh'),
          'macos-arm64',
          '0.1.0',
          root,
        ],
        { cwd: projectRoot, encoding: 'utf8', env },
      );
      expect(sign.status).not.toBe(0);
      expect(`${sign.stdout}${sign.stderr}`).toContain(
        'macOS signing is required',
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

  it('verifies a downloaded target directory and requires matching signing metadata', () => {
    const root = createMatrix('0.1.0');
    try {
      const result = spawnSync(
        'bash',
        [
          join(projectRoot, 'scripts/release-verify.sh'),
          'linux-x64',
          '0.1.0',
          join(root, 'linux-x64'),
        ],
        { cwd: projectRoot, encoding: 'utf8' },
      );
      expect(result.status).toBe(0);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it('records unsigned preview status when signing credentials are absent or incomplete', () => {
    const root = mkdtempSync(join(tmpdir(), 'qedit-signing-'));
    const manifestPath = join(root, 'manifest.json');
    try {
      const prepare = spawnSync(
        'bash',
        [join(projectRoot, 'scripts/release-sign-prepare.sh'), 'macos-arm64'],
        {
          cwd: projectRoot,
          encoding: 'utf8',
          env: {
            ...process.env,
            QEDIT_REQUIRE_SIGNING: '0',
            APPLE_CERTIFICATE: '',
            APPLE_CERTIFICATE_PASSWORD: '',
            APPLE_SIGNING_IDENTITY: 'Developer ID Application: stale identity',
            APPLE_ID: '',
            APPLE_PASSWORD: '',
            APPLE_TEAM_ID: '',
          },
        },
      );
      expect(prepare.status).toBe(0);
      expect(`${prepare.stdout}${prepare.stderr}`).toContain(
        'ad-hoc signed public preview',
      );
      writeFileSync(
        manifestPath,
        `${JSON.stringify({
          schema: 1,
          product: 'qedit',
          version: '0.1.0',
          target: 'macos-arm64',
          host: 'test',
          artifacts: [],
        })}\n`,
      );
      const result = spawnSync(
        'bash',
        [
          join(projectRoot, 'scripts/release-sign.sh'),
          'macos-arm64',
          '0.1.0',
          root,
        ],
        {
          cwd: projectRoot,
          encoding: 'utf8',
          env: {
            ...process.env,
            QEDIT_REQUIRE_SIGNING: '0',
            APPLE_CERTIFICATE: '',
            APPLE_CERTIFICATE_PASSWORD: '',
            APPLE_SIGNING_IDENTITY: 'Developer ID Application: stale identity',
            APPLE_ID: '',
            APPLE_PASSWORD: '',
            APPLE_TEAM_ID: '',
          },
        },
      );
      expect(result.status).toBe(0);
      const status = JSON.parse(
        readFileSync(join(root, 'signing.json'), 'utf8'),
      );
      const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
      expect(status.status).toBe('unsigned');
      expect(manifest.signing).toEqual({
        status: status.status,
        reason: status.reason,
      });
      expect(status.reason).toContain('absent or incomplete');
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
