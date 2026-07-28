#!/usr/bin/env bash
set -euo pipefail

# Tauri cannot cross-build complete desktop bundles from an arbitrary host:
# Linux and Windows need their native system toolchains, and macOS needs the
# matching SDK. Build the host bundle here; CI should run this command once per
# runner to produce the complete release matrix.
case "$(uname -s)" in
  Darwin) pnpm run build:mac ;;
  Linux) pnpm run build:linux ;;
  MINGW*|MSYS*|CYGWIN*) pnpm run build:win ;;
  *) echo "Unsupported build host: $(uname -s)" >&2; exit 1 ;;
esac
