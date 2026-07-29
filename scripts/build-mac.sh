#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"

cleanup_dmg_temps() {
  if [[ -d "$BUNDLE_DIR" ]]; then
    find "$BUNDLE_DIR" -type f -name 'rw.*.dmg' -exec rm -f {} +
  fi
}

trap cleanup_dmg_temps EXIT
cleanup_dmg_temps

cd "$PROJECT_ROOT"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm exec tauri build --bundles app,dmg

APP_PATH="$BUNDLE_DIR/macos/qedit.app"
if [[ ! -d "$APP_PATH" ]]; then
  echo "Expected macOS app bundle was not produced: $APP_PATH" >&2
  exit 1
fi

shopt -s nullglob
DMG_PATHS=("$BUNDLE_DIR"/dmg/*.dmg)
if (( ${#DMG_PATHS[@]} != 1 )) || [[ "$(basename "${DMG_PATHS[0]}")" == rw.*.dmg ]]; then
  echo "Expected one final DMG bundle in $BUNDLE_DIR/dmg" >&2
  exit 1
fi

echo "Finished macOS bundles:"
echo "  $APP_PATH"
echo "  ${DMG_PATHS[0]}"
