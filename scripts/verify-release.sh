#!/usr/bin/env bash
set -euo pipefail

ruby -ryaml -e 'workflow = YAML.load_file(".github/workflows/release-qedit.yml"); jobs = workflow.fetch("jobs"); %w[validate macos linux windows publish].each { |job| abort("missing #{job} job") unless jobs.key?(job) }; %w[macos linux windows].each { |job| abort("#{job} must depend on validate") unless jobs.fetch(job).fetch("needs") == "validate" }'
for script in scripts/*.sh; do
  bash -n "$script"
done
cargo metadata --locked --no-deps --format-version 1 >/dev/null
cargo check --locked --release --features mimalloc --package zed --package cli
cargo check --locked --package browser

if [[ "$(uname -s)" == "Darwin" ]]; then
  rustup target add x86_64-apple-darwin
  cargo fetch --locked
  cargo_home="${CARGO_HOME:-$HOME/.cargo}"
  renderer=$(find "$cargo_home/git/checkouts" -path '*/crates/gpui_metal/src/renderer.rs' -print -quit)
  test -n "$renderer"
  perl -0pi -e 's/const YES: objc::runtime::BOOL = true;/const YES: objc::runtime::BOOL = true as objc::runtime::BOOL;/; s/const NO: objc::runtime::BOOL = false;/const NO: objc::runtime::BOOL = false as objc::runtime::BOOL;/' "$renderer"
  cargo check --locked --release --features mimalloc --package zed --package cli --target x86_64-apple-darwin
  ./scripts/build-macos-arm64.sh
fi
