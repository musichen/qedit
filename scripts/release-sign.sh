#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${1:-}" == -- ]]; then shift; fi
TARGET="${1:-}"
VERSION="${2:-}"
OUT_DIR="${3:-$PROJECT_ROOT/dist/release/v${VERSION}/${TARGET}}"
[[ "$TARGET" =~ ^(macos|windows|linux)-(arm64|x64)$ ]] || {
  echo "Usage: $0 <target> <version> [output-dir]" >&2
  exit 2
}

STATUS_FILE="$OUT_DIR/signing.json"
write_status() {
  node - "$STATUS_FILE" "$TARGET" "$VERSION" "$1" "$2" <<'NODE'
const fs = require('node:fs');
const [path, target, version, status, reason] = process.argv.slice(2);
fs.writeFileSync(path, JSON.stringify({ schema: 1, target, version, status, reason }, null, 2) + '\n');
NODE
}

require_values() {
  local missing=()
  local name
  for name in "$@"; do
    [[ -n "${!name:-}" ]] || missing+=("$name")
  done
  if (( ${#missing[@]} > 0 )); then
    echo "Error: $TARGET publishing requires configured signing credentials: ${missing[*]}" >&2
    exit 1
  fi
}

if [[ "$TARGET" == linux-* ]]; then
  write_status 'not-applicable' 'Linux artifacts are not code signed by this pipeline'
  exit 0
fi

if [[ "$TARGET" == macos-* ]]; then
  if [[ "${QEDIT_REQUIRE_SIGNING:-}" != 1 && -z "${APPLE_CERTIFICATE:-}" && -z "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    write_status 'unsigned' 'APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY, APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID are not configured'
    echo "warning: macOS artifacts are unsigned; set QEDIT_REQUIRE_SIGNING=1 to make this a release-blocking error" >&2
    exit 0
  fi
  require_values APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
  command -v security >/dev/null 2>&1 || { echo 'Error: macOS signing requires the security command' >&2; exit 1; }
  command -v codesign >/dev/null 2>&1 || { echo 'Error: macOS signing requires codesign' >&2; exit 1; }
  command -v xcrun >/dev/null 2>&1 || { echo 'Error: macOS notarization requires xcrun' >&2; exit 1; }

  KEYCHAIN="$OUT_DIR/qedit-signing.keychain-db"
  CERT_FILE="$OUT_DIR/qedit-signing.p12"
  cleanup() {
    security delete-keychain "$KEYCHAIN" >/dev/null 2>&1 || true
    rm -f "$CERT_FILE"
  }
  trap cleanup EXIT
  printf '%s' "$APPLE_CERTIFICATE" | base64 --decode > "$CERT_FILE"
  security create-keychain -p "$APPLE_CERTIFICATE_PASSWORD" "$KEYCHAIN" >/dev/null
  security set-keychain-settings -lut 21600 "$KEYCHAIN"
  security unlock-keychain -p "$APPLE_CERTIFICATE_PASSWORD" "$KEYCHAIN"
  security import "$CERT_FILE" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign "$KEYCHAIN" >/dev/null
  security list-keychains -d user -s "$KEYCHAIN"
  security default-keychain -s "$KEYCHAIN"

  APP_PATH="$PROJECT_ROOT/src-tauri/target/release/bundle/macos/qedit.app"
  [[ -x "$APP_PATH/Contents/MacOS/qedit" ]] || { echo "Error: app bundle missing for signing: $APP_PATH" >&2; exit 1; }
  codesign --deep --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$APP_PATH"
  codesign --verify --deep --strict "$APP_PATH"
  ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$OUT_DIR/qedit-v${VERSION}-macos-${TARGET##*-}.app.zip"
  DMG_PATH="$OUT_DIR/qedit-v${VERSION}-macos-${TARGET##*-}.dmg"
  codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$DMG_PATH"
  xcrun notarytool submit "$DMG_PATH" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
  xcrun stapler staple "$DMG_PATH"
  write_status 'signed-and-notarized' 'Signed with Developer ID and notarized with Apple notarytool'
  exit 0
fi

if [[ "${QEDIT_REQUIRE_SIGNING:-}" != 1 && -z "${WINDOWS_CERTIFICATE_BASE64:-}" ]]; then
  write_status 'unsigned' 'WINDOWS_CERTIFICATE_BASE64 and WINDOWS_CERTIFICATE_PASSWORD are not configured'
  echo "warning: Windows artifacts are unsigned; set QEDIT_REQUIRE_SIGNING=1 to make this a release-blocking error" >&2
  exit 0
fi
require_values WINDOWS_CERTIFICATE_BASE64 WINDOWS_CERTIFICATE_PASSWORD WINDOWS_TIMESTAMP_URL
command -v signtool >/dev/null 2>&1 || {
  echo 'Error: Windows signing requires signtool.exe on PATH (install the Windows SDK on the runner)' >&2
  exit 1
}
CERT_FILE="$OUT_DIR/qedit-signing.pfx"
printf '%s' "$WINDOWS_CERTIFICATE_BASE64" | base64 --decode > "$CERT_FILE"
cleanup() { rm -f "$CERT_FILE"; }
trap cleanup EXIT
for artifact in "$OUT_DIR"/*.msi "$OUT_DIR"/*.exe; do
  [[ -f "$artifact" ]] || continue
  signtool sign /fd SHA256 /f "$CERT_FILE" /p "$WINDOWS_CERTIFICATE_PASSWORD" /tr "$WINDOWS_TIMESTAMP_URL" /td SHA256 "$artifact"
  signtool verify /pa /all "$artifact"
done
write_status 'signed' 'Signed with the configured Authenticode certificate and timestamp service'
