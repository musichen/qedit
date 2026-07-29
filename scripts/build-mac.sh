#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"
BUILD_LOG="$(mktemp -t qedit-build-mac.XXXXXXXX)"

# create-dmg runs an optional "Finder-prettifying" AppleScript that needs a live
# Finder plus Automation permission. Headless or unattended sessions (CI, ssh,
# sandboxed agents) fail it with AppleEvent errors such as -1712 (timed out) or
# -1719 (no assistive access), and create-dmg then aborts the whole DMG. That
# step only positions icons, so it must never fail the build: retry with Tauri's
# CI path, which passes --skip-jenkins to create-dmg and skips the AppleScript.
# Tauri swallows bundle_dmg.sh output, so the only signal it leaves is the
# generic "error running bundle_dmg.sh". Retrying on that is still safe: a
# presentation-independent failure (hdiutil, mount, detach, compression) fails
# the retry too, and the final DMG assertion below refuses a false success.
DMG_STAGE_FAILURE_PATTERN='error running bundle_dmg\.sh|Failed running AppleScript|AppleEvent timed out|Not authorized to send Apple events|\(-1712\)|\(-1719\)|\(-1743\)'

cleanup_dmg_temps() {
  if [[ -d "$BUNDLE_DIR" ]]; then
    find "$BUNDLE_DIR" -type f -name 'rw.*.dmg' -exec rm -f {} +
  fi
}

cleanup() {
  cleanup_dmg_temps
  rm -f "$BUILD_LOG"
}

trap cleanup EXIT
cleanup_dmg_temps

run_bundler() {
  local status=0
  if [[ "${1:-}" == "degraded" ]]; then
    CI=true pnpm exec tauri build --bundles app,dmg 2>&1 | tee "$BUILD_LOG" || status=${PIPESTATUS[0]}
  else
    pnpm exec tauri build --bundles app,dmg 2>&1 | tee "$BUILD_LOG" || status=${PIPESTATUS[0]}
  fi
  return "$status"
}

cd "$PROJECT_ROOT"
if command -v rustup >/dev/null 2>&1; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin || true
fi

if ! run_bundler; then
  if grep -Eq "$DMG_STAGE_FAILURE_PATTERN" "$BUILD_LOG"; then
    echo "warning: DMG creation failed inside bundle_dmg.sh, usually because create-dmg could not drive Finder (AppleEvents unavailable or timed out)." >&2
    echo "warning: retrying with DMG presentation disabled; the DMG ships without custom icon positioning." >&2
    cleanup_dmg_temps
    if ! run_bundler degraded; then
      echo "macOS bundling failed even with the DMG presentation step disabled." >&2
      exit 1
    fi
  else
    echo "macOS bundling failed." >&2
    exit 1
  fi
fi

APP_PATH="$BUNDLE_DIR/macos/qedit.app"
if [[ ! -d "$APP_PATH" ]]; then
  echo "Expected macOS app bundle was not produced: $APP_PATH" >&2
  exit 1
fi

shopt -s nullglob
DMG_PATHS=()
for candidate in "$BUNDLE_DIR"/dmg/*.dmg; do
  [[ "$(basename "$candidate")" == rw.*.dmg ]] && continue
  DMG_PATHS+=("$candidate")
done
if (( ${#DMG_PATHS[@]} != 1 )); then
  echo "Expected one final DMG bundle in $BUNDLE_DIR/dmg" >&2
  exit 1
fi

echo "Finished macOS bundles:"
echo "  $APP_PATH"
echo "  ${DMG_PATHS[0]}"
