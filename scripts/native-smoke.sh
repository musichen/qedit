#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

# The native build is intentionally separate from this smoke test. Keeping the
# launch step small makes it safe to run repeatedly after a build or while
# inspecting a packaged app.
if [[ "${QEDIT_PLATFORM:-$(uname -s)}" == "Darwin" ]]; then
  APP_BUNDLE=${QEDIT_APP_BUNDLE:-src-tauri/target/release/bundle/macos/qedit.app}
  EXECUTABLE="$APP_BUNDLE/Contents/MacOS/qedit"

  if [[ ! -d "$APP_BUNDLE" || ! -x "$EXECUTABLE" ]]; then
    echo "native smoke: packaged macOS app not found: $APP_BUNDLE" >&2
    exit 1
  fi

  if [[ ! -f "$APP_BUNDLE/Contents/Resources/qedit.icns" ]]; then
    echo "native smoke: packaged app is missing qedit.icns" >&2
    exit 1
  fi

  QEDIT_APP_BUNDLE="$APP_BUNDLE" pnpm exec vitest run src/__tests__/tauri-icon.test.ts
else
  EXECUTABLE=${QEDIT_BINARY:-src-tauri/target/release/qedit}
  if [[ ! -x "$EXECUTABLE" ]]; then
    echo "native smoke: release binary not found: $EXECUTABLE" >&2
    exit 1
  fi
fi

LOG_FILE=$(mktemp "${TMPDIR:-/tmp}/qedit-native-smoke.XXXXXXXXXX")
PID=""
# shellcheck disable=SC2317,SC2329 # invoked indirectly by the EXIT trap below.
# (SC2317 is the pre-0.11 code for the same "unreachable" false positive.)
cleanup() {
  if [[ -n "$PID" ]] && kill -0 "$PID" 2>/dev/null; then
    kill -TERM "$PID" 2>/dev/null || true
    wait "$PID" 2>/dev/null || true
  fi
  rm -f "$LOG_FILE"
}
trap cleanup EXIT

"$EXECUTABLE" >"$LOG_FILE" 2>&1 &
PID=$!

# A launch that exits before this window is a startup failure. The app is a GUI
# process, so there is no reliable stdout readiness marker; checking liveness
# also works for a packaged app without requiring browser automation.
for _ in {1..30}; do
  if ! kill -0 "$PID" 2>/dev/null; then
    wait "$PID" || status=$?
    status=${status:-0}
    echo "native smoke: qedit exited during startup (status $status)" >&2
    cat "$LOG_FILE" >&2
    exit 1
  fi
  sleep 0.1
done

kill -TERM "$PID" 2>/dev/null || true
for _ in {1..30}; do
  if ! kill -0 "$PID" 2>/dev/null; then
    wait "$PID" 2>/dev/null || true
    echo "native smoke: launch and shutdown passed ($EXECUTABLE)"
    exit 0
  fi
  state=$(ps -o state= -p "$PID" 2>/dev/null | tr -d ' ' || true)
  if [[ "$state" == Z* ]]; then
    wait "$PID" 2>/dev/null || true
    echo "native smoke: launch and shutdown passed ($EXECUTABLE)"
    exit 0
  fi
  sleep 0.1
done

echo "native smoke: qedit did not shut down after SIGTERM" >&2
cat "$LOG_FILE" >&2
exit 1
