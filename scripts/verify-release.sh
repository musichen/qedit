#!/usr/bin/env bash
set -euo pipefail

ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release-qedit.yml")'
for script in scripts/*.sh; do
  bash -n "$script"
done
cargo check --locked --release --features mimalloc --package zed --package cli

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
