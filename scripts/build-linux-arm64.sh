#!/usr/bin/env bash
set -euo pipefail

version="${QEDIT_VERSION:-1.0.1}"
target="aarch64-unknown-linux-gnu"
rustup target add "$target"
cargo build --release --features mimalloc --package zed --package cli --target "$target"
package="qedit_${version}_arm64"
rm -rf "$package"
mkdir -p "$package/DEBIAN" "$package/usr/bin" "$package/usr/share/icons/hicolor/512x512/apps"
install -m 0755 "target/$target/release/zed" "$package/usr/bin/qedit"
install -m 0755 "target/$target/release/cli" "$package/usr/bin/qedit-cli"
install -m 0644 icon.png "$package/usr/share/icons/hicolor/512x512/apps/qedit.png"
cat > "$package/DEBIAN/control" <<CONTROL
Package: qedit
Version: $version
Section: editors
Priority: optional
Architecture: arm64
Maintainer: Qedit contributors
Description: Qedit browser, code editor, and terminal
 Qedit is a fast development environment for focused work.
CONTROL
mkdir -p dist
dpkg-deb --build --root-owner-group "$package" "dist/Qedit-$version-linux-arm64.deb"
