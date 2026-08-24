---
title: Building Qedit for Windows
description: "Guide to building Qedit for Windows development."
---

# Building Qedit for Windows

> The commands below can be run from PowerShell or Windows Terminal.

## Repository

Clone the [Qedit repository](https://github.com/musichen/qedit).

If you are developing Qedit together with a local GPUI checkout, clone [Glass-HQ/gpui](https://github.com/Glass-HQ/gpui) as a sibling repository:

```powershell
git clone https://github.com/musichen/qedit.git
git clone https://github.com/Glass-HQ/gpui.git
```

This yields:

```text
C:\src\musichen\qedit
C:\src\Glass-HQ\gpui
```

## Dependencies

Install:

- [rustup](https://www.rust-lang.org/tools/install)
- Visual Studio or Build Tools with the C++ desktop workload
- Windows 10/11 SDK
- [CMake](https://cmake.org/download)
- `ninja`

If you install only Build Tools, launch Qedit from a developer shell so MSVC and SDK environment variables are available.

## Recommended bootstrap

Qedit includes a Windows bootstrap script that verifies the machine, installs `ninja` with `winget` when needed, stages the CEF runtime into a stable dev directory, builds the companion CLI, and launches the app.

From the repo root:

```powershell
powershell -ExecutionPolicy Bypass -File .\script\setup-windows-dev.ps1
```

To build and launch Qedit in one step:

```powershell
powershell -ExecutionPolicy Bypass -File .\script\setup-windows-dev.ps1 -Run
```

This is the intended Windows development entry point.

What the script does:

- verifies `cargo` and `cmake`
- installs `ninja` for the current user if it is missing
- uses a sibling `..\gpui` checkout automatically when one is present
- stages the Windows CEF runtime next to the development build so `zed.exe` can be launched directly
  - this step is handled by [`script/stage-windows-cef-runtime.ps1`](../../../script/stage-windows-cef-runtime.ps1)
- builds `cli.exe`, which Qedit expects in the Windows dev layout
- launches `zed.exe` directly with the staged dev runtime

## Manual local-GPUI build

If you are working with a sibling `gpui` checkout and want to drive Cargo directly instead of using the bootstrap script, use the PowerShell helper:

```powershell
powershell -ExecutionPolicy Bypass -File .\script\cargo-gpui-local.ps1 build -p cli
powershell -ExecutionPolicy Bypass -File .\script\cargo-gpui-local.ps1 build -p zed
powershell -ExecutionPolicy Bypass -File .\script\cargo-gpui-local.ps1 run -p zed
```

Building `cli` first ensures `zed.exe` can find the companion CLI binary in the expected Windows dev layout.

For day-to-day Windows development, prefer the bootstrap script:

```powershell
powershell -ExecutionPolicy Bypass -File .\script\setup-windows-dev.ps1 -Run
```

## Troubleshooting

### `cmake` cannot find Ninja

If a build fails in `cef-dll-sys` with an error saying CMake cannot find `Ninja`, install it:

```powershell
winget install --id Ninja-build.Ninja --scope user --accept-package-agreements --accept-source-agreements --silent
```

Then open a new shell and re-run the build.

### Missing sibling GPUI checkout

If you are using the local GPUI helper or the bootstrap script picks up a sibling checkout automatically, Qedit will resolve GPUI crates from `..\gpui`. In that case you must either:

- clone [Glass-HQ/gpui](https://github.com/Glass-HQ/gpui) next to the Qedit checkout, or
- move back to the pinned GPUI dependency flow once the local GPUI changes are no longer needed

### `RUSTFLAGS` breaks builds

If you set the `RUSTFLAGS` environment variable, it overrides the `rustflags` settings in `.cargo/config.toml`, which Qedit needs for Windows builds.

If you need extra Rust flags, prefer adding them in a local `.cargo/config.toml` instead of exporting `RUSTFLAGS`.

### Path too long

If dependency checkout paths exceed Windows path limits, enable long path support for both Git and Windows:

```powershell
git config --system core.longpaths true
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

Reboot after enabling Windows long paths.

### Graphics or launch failures

Qedit currently uses the same GPU stack as Zed on Windows. If the app fails to open a window, inspect the log at:

```text
C:\Users\YOU\AppData\Local\Zed\logs\Zed.log
```

If you see Vulkan or device initialization errors, update GPU drivers first.
