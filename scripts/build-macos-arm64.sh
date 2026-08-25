#!/usr/bin/env bash
set -euo pipefail

version="${QEDIT_VERSION:-1.0.1}"
cargo build --release --features mimalloc --package zed --package cli
target_dir="target/release"
app="$target_dir/Qedit.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" dist
cp "$target_dir/zed" "$app/Contents/MacOS/Qedit"
cp crates/zed/resources/Qedit.icns "$app/Contents/Resources/Qedit.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Qedit</string>
<key>CFBundleDisplayName</key><string>Qedit</string>
<key>CFBundleExecutable</key><string>Qedit</string>
<key>CFBundleIdentifier</key><string>dev.qedit.Qedit</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleIconFile</key><string>Qedit.icns</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
</dict></plist>
PLIST
stage=$(mktemp -d)
cp -R "$app" "$stage/Qedit.app"
ln -s /Applications "$stage/Applications"
hdiutil create -volname Qedit -srcfolder "$stage" -ov -format UDZO "dist/Qedit-$version-macos-arm64.dmg"
rm -rf "$stage"
