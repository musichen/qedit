#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${1:-}" == -- ]]; then shift; fi
TARGET="${1:-}"
VERSION="${2:-}"
OUT_DIR="${3:-}"

usage() {
  echo "Usage: $0 <macos|windows|linux>-<arm64|x64> <version> [output-dir]" >&2
  exit 2
}

[[ "$TARGET" =~ ^(macos|windows|linux)-(arm64|x64)$ ]] || usage
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$ ]] || usage

PLATFORM="${TARGET%-*}"
ARCH="${TARGET##*-}"
OUT_DIR="${OUT_DIR:-$PROJECT_ROOT/dist/release/v${VERSION}/${TARGET}}"

case "$OUT_DIR" in
  "$PROJECT_ROOT"/dist/release/*) ;;
  *)
    echo "Error: output directory must be below $PROJECT_ROOT/dist/release" >&2
    exit 1
    ;;
esac

metadata_version() {
  node -p "JSON.parse(require('fs').readFileSync(process.argv[1], 'utf8')).version" "$1"
}

PACKAGE_VERSION=$(metadata_version "$PROJECT_ROOT/package.json")
TAURI_VERSION=$(metadata_version "$PROJECT_ROOT/src-tauri/tauri.conf.json")
CARGO_VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$PROJECT_ROOT/src-tauri/Cargo.toml" | head -n 1)
if ! [[ "$VERSION" == "$PACKAGE_VERSION" && "$VERSION" == "$TAURI_VERSION" && "$VERSION" == "$CARGO_VERSION" ]]; then
  echo "Error: release version $VERSION does not match project metadata" >&2
  echo "  package.json: $PACKAGE_VERSION" >&2
  echo "  src-tauri/tauri.conf.json: $TAURI_VERSION" >&2
  echo "  src-tauri/Cargo.toml: $CARGO_VERSION" >&2
  exit 1
fi

if ! command -v rustc >/dev/null 2>&1; then
  echo "Error: rustc is required to prove this build is running on the requested native runner" >&2
  exit 1
fi
HOST_TRIPLE=$(rustc -vV | sed -n 's/^host: //p')
EXPECTED_TRIPLE=""
case "$TARGET" in
  macos-arm64) EXPECTED_TRIPLE='aarch64-apple-darwin' ;;
  macos-x64) EXPECTED_TRIPLE='x86_64-apple-darwin' ;;
  windows-arm64) EXPECTED_TRIPLE='aarch64-pc-windows-msvc' ;;
  windows-x64) EXPECTED_TRIPLE='x86_64-pc-windows-msvc' ;;
  linux-arm64) EXPECTED_TRIPLE='aarch64-unknown-linux-gnu' ;;
  linux-x64) EXPECTED_TRIPLE='x86_64-unknown-linux-gnu' ;;
esac
if [[ "$HOST_TRIPLE" != "$EXPECTED_TRIPLE" ]]; then
  echo "Error: $TARGET requires native runner toolchain $EXPECTED_TRIPLE; detected $HOST_TRIPLE" >&2
  echo "Cross-compilation is intentionally not attempted. Route this target to its matrix runner." >&2
  exit 1
fi

BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"
rm -rf "$BUNDLE_DIR" "$OUT_DIR"
mkdir -p "$OUT_DIR"

cd "$PROJECT_ROOT"
echo "Building qedit v$VERSION for $TARGET on $HOST_TRIPLE"
case "$PLATFORM" in
  macos)
    # Reuse build-mac.sh rather than calling tauri build directly so this path
    # gets the same headless-Finder DMG retry (see scripts/build-mac.sh) instead
    # of hard-failing when create-dmg's AppleScript step has no live Finder.
    bash "$PROJECT_ROOT/scripts/build-mac.sh"
    ;;
  windows) pnpm exec tauri build --bundles msi,nsis ;;
  linux) pnpm exec tauri build --bundles deb,appimage ;;
esac

copy_one() {
  local search_dir="$1"
  local pattern="$2"
  local destination="$3"
  local -a matches=()
  while IFS= read -r -d '' candidate; do
    matches+=("$candidate")
  done < <(find "$search_dir" -type f -name "$pattern" ! -name 'rw.*.dmg' -print0 2>/dev/null)
  if (( ${#matches[@]} != 1 )); then
    echo "Error: expected exactly one $pattern below $search_dir, found ${#matches[@]}" >&2
    exit 1
  fi
  cp "${matches[0]}" "$OUT_DIR/$destination"
}

case "$PLATFORM" in
  macos)
    APP_PATH="$BUNDLE_DIR/macos/qedit.app"
    [[ -x "$APP_PATH/Contents/MacOS/qedit" ]] || {
      echo "Error: expected executable app bundle was not produced: $APP_PATH" >&2
      exit 1
    }
    command -v ditto >/dev/null 2>&1 || {
      echo "Error: ditto is required to package the macOS .app bundle" >&2
      exit 1
    }
    ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$OUT_DIR/qedit-v${VERSION}-macos-${ARCH}.app.zip"
    copy_one "$BUNDLE_DIR/dmg" "*_${VERSION}_*.dmg" "qedit-v${VERSION}-macos-${ARCH}.dmg"
    ;;
  windows)
    copy_one "$BUNDLE_DIR/msi" "*_${VERSION}_*.msi" "qedit-v${VERSION}-windows-${ARCH}.msi"
    copy_one "$BUNDLE_DIR/nsis" "*_${VERSION}_*.exe" "qedit-v${VERSION}-windows-${ARCH}-nsis.exe"
    ;;
  linux)
    copy_one "$BUNDLE_DIR/deb" "*_${VERSION}_*.deb" "qedit-v${VERSION}-linux-${ARCH}.deb"
    copy_one "$BUNDLE_DIR/appimage" "*_${VERSION}_*.AppImage" "qedit-v${VERSION}-linux-${ARCH}.AppImage"
    ;;
esac

node - "$OUT_DIR/manifest.json" "$TARGET" "$VERSION" "$HOST_TRIPLE" <<'NODE'
const fs = require('node:fs');
const [manifestPath, target, version, host] = process.argv.slice(2);
const files = fs.readdirSync(require('node:path').dirname(manifestPath))
  .filter((name) => name !== 'manifest.json' && name !== 'signing.json')
  .sort();
fs.writeFileSync(manifestPath, JSON.stringify({
  schema: 1,
  product: 'qedit',
  version,
  target,
  host,
  artifacts: files,
  signing: { status: 'unsigned', reason: 'No signing hook has run yet' },
}, null, 2) + '\n');
NODE

echo "Built deterministic artifacts in $OUT_DIR"
