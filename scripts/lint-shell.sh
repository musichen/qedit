#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

# The release and native-smoke logic lives in bash, so it gets static analysis
# too. shellcheck is not an npm package, so it cannot be pinned in the lockfile;
# when it is absent this step reports that instead of failing the whole check
# pipeline on a contributor machine. Set QEDIT_REQUIRE_SHELLCHECK=1 (CI) to make
# a missing shellcheck a hard error.
if ! command -v shellcheck >/dev/null 2>&1; then
  if [[ -n "${QEDIT_REQUIRE_SHELLCHECK:-}" ]]; then
    echo "lint:shell: shellcheck is required but not installed" >&2
    exit 1
  fi
  echo "lint:shell: shellcheck not installed, skipping (brew install shellcheck)"
  exit 0
fi

shellcheck --external-sources scripts/*.sh
echo "lint:shell: all shell scripts pass shellcheck"
