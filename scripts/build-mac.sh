#!/usr/bin/env bash
set -euo pipefail

# Tauri's default DMG bundler runs Finder AppleScript to arrange icons. That
# requires an interactive GUI session and is unavailable in CI/headless hosts.
# tauri-bundler skips that step when CI is set, so the dmg target stays
# reproducible and valid on both developer machines and headless runners.
CI=true pnpm exec tauri build --bundles dmg

APP_BUNDLE="src-tauri/target/release/bundle/macos/qedit.app"
DMG_DIR="src-tauri/target/release/bundle/dmg"

if [[ ! -d "$APP_BUNDLE" || ! -x "$APP_BUNDLE/Contents/MacOS/qedit" ]]; then
  echo "macOS build: app bundle not found: $APP_BUNDLE" >&2
  exit 1
fi

# Discover the artifact instead of predicting its name: tauri-bundler derives
# the version from tauri.conf.json and labels the architecture itself (x64 on
# Intel, aarch64 on Apple Silicon).
declare -a DMGS=()
while IFS= read -r -d '' dmg; do
  DMGS+=("$dmg")
done < <(find "$DMG_DIR" -maxdepth 1 -type f -name '*.dmg' -print0 2>/dev/null)
if [[ ${#DMGS[@]} -eq 0 ]]; then
  echo "macOS build: DMG not produced in $DMG_DIR" >&2
  exit 1
fi

printf 'macOS build: %s\n' "${DMGS[@]}"
