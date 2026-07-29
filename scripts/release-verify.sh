#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${1:-}" == -- ]]; then shift; fi
MODE="${1:-}"
VERSION="${2:-}"
ROOT="${3:-$PROJECT_ROOT/dist/release/v${VERSION}}"
[[ "$MODE" == all || "$MODE" =~ ^(macos|windows|linux)-(arm64|x64)$ ]] || {
  echo "Usage: $0 <all|target> <version> [release-root-or-target-dir]" >&2
  exit 2
}
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$ ]] || { echo "Error: invalid version: $VERSION" >&2; exit 2; }

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi
}

verify_target() {
  local target="$1"
  local dir="$ROOT"
  [[ "$MODE" == all ]] && dir="$ROOT/$target"
  [[ -d "$dir" ]] || { echo "Error: missing target directory: $dir" >&2; exit 1; }
  [[ -s "$dir/manifest.json" ]] || { echo "Error: missing manifest: $dir/manifest.json" >&2; exit 1; }
  node - "$dir/manifest.json" "$target" "$VERSION" <<'NODE'
const fs = require('node:fs');
const [path, target, version] = process.argv.slice(2);
const manifest = JSON.parse(fs.readFileSync(path, 'utf8'));
if (manifest.product !== 'qedit' || manifest.target !== target || manifest.version !== version) {
  throw new Error(`manifest mismatch: expected qedit ${target} v${version}`);
}
NODE

  local expected=()
  case "$target" in
    macos-arm64) expected=("qedit-v${VERSION}-macos-arm64.app.zip" "qedit-v${VERSION}-macos-arm64.dmg") ;;
    macos-x64) expected=("qedit-v${VERSION}-macos-x64.app.zip" "qedit-v${VERSION}-macos-x64.dmg") ;;
    windows-arm64) expected=("qedit-v${VERSION}-windows-arm64.msi" "qedit-v${VERSION}-windows-arm64-nsis.exe") ;;
    windows-x64) expected=("qedit-v${VERSION}-windows-x64.msi" "qedit-v${VERSION}-windows-x64-nsis.exe") ;;
    linux-arm64) expected=("qedit-v${VERSION}-linux-arm64.deb" "qedit-v${VERSION}-linux-arm64.AppImage") ;;
    linux-x64) expected=("qedit-v${VERSION}-linux-x64.deb" "qedit-v${VERSION}-linux-x64.AppImage") ;;
  esac
  local artifact
  for artifact in "${expected[@]}"; do
    [[ -s "$dir/$artifact" ]] || { echo "Error: missing or empty expected artifact: $dir/$artifact" >&2; exit 1; }
  done
  local entry basename_entry allowed
  for entry in "$dir"/*; do
    [[ -f "$entry" ]] || continue
    basename_entry="$(basename "$entry")"
    allowed=0
    [[ "$basename_entry" == manifest.json || "$basename_entry" == signing.json ]] && allowed=1
    for artifact in "${expected[@]}"; do [[ "$basename_entry" == "$artifact" ]] && allowed=1; done
    if (( allowed == 0 )); then
      echo "Error: unexpected stale or partial artifact in $dir: $basename_entry" >&2
      exit 1
    fi
  done
  local rw_dmg
  for rw_dmg in "$dir"/rw.*.dmg; do
    [[ -e "$rw_dmg" ]] || continue
    echo "Error: temporary DMG image found in release output: $rw_dmg" >&2
    exit 1
  done
  if [[ "${QEDIT_REQUIRE_SIGNED:-}" == 1 && "$target" != linux-* ]]; then
    [[ -s "$dir/signing.json" ]] || { echo "Error: signing status missing for $target" >&2; exit 1; }
    node - "$dir/signing.json" "$target" <<'NODE'
const fs = require('node:fs');
const [path, target] = process.argv.slice(2);
const status = JSON.parse(fs.readFileSync(path, 'utf8'));
const allowed = target.startsWith('macos-') ? 'signed-and-notarized' : 'signed';
if (status.status !== allowed) throw new Error(`${target} is not ${allowed}: ${status.status} (${status.reason})`);
NODE
  fi
  echo "verified $target: ${expected[*]}"
}

TARGETS=(macos-arm64 macos-x64 windows-arm64 windows-x64 linux-arm64 linux-x64)
if [[ "$MODE" == all ]]; then
  [[ -d "$ROOT" ]] || { echo "Error: missing release root: $ROOT" >&2; exit 1; }
  for target in "${TARGETS[@]}"; do verify_target "$target"; done
  checksum_file="$ROOT/SHA256SUMS"
  : > "$checksum_file"
  for target in "${TARGETS[@]}"; do
    for artifact in "$ROOT/$target"/*; do
      case "$(basename "$artifact")" in
        qedit-v*.app.zip|qedit-v*.dmg|qedit-v*.msi|qedit-v*.exe|qedit-v*.deb|qedit-v*.AppImage)
          printf '%s  %s\n' "$(hash_file "$artifact")" "$target/$(basename "$artifact")" >> "$checksum_file" ;;
      esac
    done
  done
  sort -o "$checksum_file" "$checksum_file"
  node - "$ROOT/provenance.json" "$VERSION" "$ROOT/SHA256SUMS" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const childProcess = require('node:child_process');
const [out, version, sumsPath] = process.argv.slice(2);
const subjects = fs.readFileSync(sumsPath, 'utf8').trim().split('\n').filter(Boolean).map((line) => {
  const [sha, name] = line.split(/\s+/);
  return { name, sha256: sha };
});
const generatedAt = process.env.SOURCE_DATE_EPOCH
  ? new Date(Number(process.env.SOURCE_DATE_EPOCH) * 1000).toISOString()
  : new Date().toISOString();
const sourceRevision = process.env.GITHUB_SHA || childProcess.execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const invocationId = `${process.env.GITHUB_RUN_ID || 'local'}:${sourceRevision}`;
fs.writeFileSync(out, JSON.stringify({
  schema: 'https://in-toto.io/Statement/v1',
  product: 'qedit',
  version,
  predicateType: 'https://slsa.dev/provenance/v1',
  subject: subjects,
  buildDefinition: {
    buildType: 'https://github.com/musichen/qedit/.github/workflows/release.yml',
    externalParameters: { workflow: process.env.GITHUB_WORKFLOW || null, ref: process.env.GITHUB_REF || null },
    resolvedDependencies: [{ uri: 'git+https://github.com/musichen/qedit', digest: { sha1: sourceRevision } }],
  },
  runDetails: { builder: { id: process.env.GITHUB_RUN_ID ? `github-actions:${process.env.GITHUB_RUN_ID}` : 'local' }, metadata: { invocationId, finishedOn: generatedAt } },
}, null, 2) + '\n');
NODE
  echo "verified complete qedit v$VERSION release matrix"
else
  verify_target "$MODE"
fi
