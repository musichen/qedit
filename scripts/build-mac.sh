#!/usr/bin/env bash
set -euo pipefail

# Tauri's default DMG bundler runs Finder AppleScript to arrange icons. That
# requires an interactive GUI session and is unavailable in CI/headless hosts.
# Build the app normally, then use the generated create-dmg script in its
# non-Finder mode so the release artifact remains reproducible and valid.
pnpm exec tauri build --bundles app

APP_BUNDLE="src-tauri/target/release/bundle/macos/qedit.app"
DMG_SCRIPT="src-tauri/target/release/bundle/dmg/bundle_dmg.sh"
DMG_VERSION=$(node -p "JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version")
TAURI_ARCH=$(rustc -vV | sed -n 's/^host: //p' | sed 's/-apple-darwin$//')
DMG_PATH="src-tauri/target/release/bundle/dmg/qedit_${DMG_VERSION}_${TAURI_ARCH}.dmg"

if [[ ! -d "$APP_BUNDLE" || ! -x "$APP_BUNDLE/Contents/MacOS/qedit" ]]; then
  echo "macOS build: app bundle not found: $APP_BUNDLE" >&2
  exit 1
fi
if [[ ! -x "$DMG_SCRIPT" ]]; then
  echo "macOS build: generated DMG script not found: $DMG_SCRIPT" >&2
  exit 1
fi

rm -f "$DMG_PATH"
bash "$DMG_SCRIPT" --skip-jenkins "$DMG_PATH" "$APP_BUNDLE"
