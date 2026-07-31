#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-}"
if [[ "$TARGET" == -- ]]; then shift; TARGET="${1:-}"; fi
[[ "$TARGET" =~ ^(macos|windows|linux)-(arm64|x64)$ ]] || {
  echo "Usage: $0 <target>" >&2
  exit 2
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

has_all_values() {
  local name
  for name in "$@"; do
    [[ -n "${!name:-}" ]] || return 1
  done
  return 0
}

case "$TARGET" in
  linux-*)
    echo "Linux signing preparation: not applicable"
    ;;
  macos-*)
    if ! has_all_values APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; then
      if [[ "${QEDIT_REQUIRE_SIGNING:-}" != 1 ]]; then
        echo "warning: macOS signing credentials are absent or incomplete; continuing as an ad-hoc signed public preview (not notarized)"
        exit 0
      fi
      echo "Error: macOS signing is required but credentials are absent or incomplete" >&2
      exit 1
    fi
    command -v security >/dev/null 2>&1 || { echo 'Error: macOS signing requires the security command' >&2; exit 1; }
    KEYCHAIN_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
    KEYCHAIN="$KEYCHAIN_ROOT/qedit-${TARGET}-signing.keychain-db"
    CERT_FILE="$KEYCHAIN_ROOT/qedit-${TARGET}-signing.p12"
    security delete-keychain "$KEYCHAIN" >/dev/null 2>&1 || true
    printf '%s' "$APPLE_CERTIFICATE" | base64 --decode > "$CERT_FILE"
    security create-keychain -p "$APPLE_CERTIFICATE_PASSWORD" "$KEYCHAIN" >/dev/null
    security set-keychain-settings -lut 21600 "$KEYCHAIN"
    security unlock-keychain -p "$APPLE_CERTIFICATE_PASSWORD" "$KEYCHAIN"
    security import "$CERT_FILE" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign "$KEYCHAIN" >/dev/null
    security list-keychains -d user -s "$KEYCHAIN"
    security default-keychain -s "$KEYCHAIN"
    rm -f "$CERT_FILE"
    echo "Prepared macOS keychain for Tauri bundle signing"
    ;;
  windows-*)
    echo "Windows Authenticode signing runs after bundling with signtool"
    ;;
esac
