#!/usr/bin/env bash
set -euo pipefail

# Tauri's default DMG bundler runs Finder AppleScript to arrange icons. That
# requires an interactive GUI session and is unavailable in CI/headless hosts.
# tauri-bundler skips that step when CI is set, so the dmg target stays
# reproducible and valid on both developer machines and headless runners.
CI=true pnpm exec tauri build --bundles dmg

APP_BUNDLE="src-tauri/target/release/bundle/macos/qedit.app"
DMG_VERSION=$(node -p "JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version")
TAURI_ARCH=$(rustc -vV | sed -n 's/^host: //p' | sed 's/-apple-darwin$//')
DMG_PATH="src-tauri/target/release/bundle/dmg/qedit_${DMG_VERSION}_${TAURI_ARCH}.dmg"

if [[ ! -d "$APP_BUNDLE" || ! -x "$APP_BUNDLE/Contents/MacOS/qedit" ]]; then
  echo "macOS build: app bundle not found: $APP_BUNDLE" >&2
  exit 1
fi
if [[ ! -f "$DMG_PATH" ]]; then
  echo "macOS build: DMG not produced: $DMG_PATH" >&2
  exit 1
fi
