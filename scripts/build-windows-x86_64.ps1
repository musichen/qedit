$ErrorActionPreference = 'Stop'
$version = if ($env:QEDIT_VERSION) { $env:QEDIT_VERSION } else { '1.0.1' }

cargo build --features mimalloc --package zed --package cli
New-Item -ItemType Directory -Force -Path package, dist | Out-Null
Copy-Item target/debug/zed.exe package/Qedit.exe
Copy-Item target/debug/cli.exe package/qedit-cli.exe
Copy-Item LICENSE-GPL package/LICENSE-GPL
Compress-Archive -Path package/* -DestinationPath "dist/Qedit-$version-windows-x86_64.zip" -Force
