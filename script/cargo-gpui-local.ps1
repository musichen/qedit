param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

if (-not $CargoArgs -or $CargoArgs.Count -eq 0) {
    throw "Usage: script/cargo-gpui-local.ps1 <cargo args...>"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$gpuiRoot = if ($env:QEDIT_GPUI_PATH) { $env:QEDIT_GPUI_PATH } else { Join-Path (Split-Path $repoRoot -Parent) "gpui" }
$lockFile = Join-Path $repoRoot "Cargo.lock"

if (-not (Test-Path (Join-Path $gpuiRoot "crates\gpui\Cargo.toml"))) {
    throw "Local GPUI checkout not found at '$gpuiRoot'. Set QEDIT_GPUI_PATH or clone Glass-HQ/gpui next to the Qedit checkout."
}

$gpuiRootNormalized = $gpuiRoot.Replace("\", "/")
$configFile = [System.IO.Path]::GetTempFileName()
$lockFileBytes = $null
$lockFileExists = Test-Path $lockFile

try {
    if ($lockFileExists) {
        $lockFileBytes = [System.IO.File]::ReadAllBytes($lockFile)
    }

    $configContents = @"
[patch."https://github.com/Glass-HQ/gpui.git"]
collections = { path = "$gpuiRootNormalized/crates/collections" }
derive_refineable = { path = "$gpuiRootNormalized/crates/refineable/derive_refineable" }
gpui = { path = "$gpuiRootNormalized/crates/gpui" }
gpui_macos = { path = "$gpuiRootNormalized/crates/gpui_macos" }
gpui_macros = { path = "$gpuiRootNormalized/crates/gpui_macros" }
gpui_metal = { path = "$gpuiRootNormalized/crates/gpui_metal" }
gpui_platform = { path = "$gpuiRootNormalized/crates/gpui_platform" }
gpui_tokio = { path = "$gpuiRootNormalized/crates/gpui_tokio" }
gpui_util = { path = "$gpuiRootNormalized/crates/gpui_util" }
http_client = { path = "$gpuiRootNormalized/crates/http_client" }
http_client_tls = { path = "$gpuiRootNormalized/crates/http_client_tls" }
media = { path = "$gpuiRootNormalized/crates/media" }
refineable = { path = "$gpuiRootNormalized/crates/refineable" }
scheduler = { path = "$gpuiRootNormalized/crates/scheduler" }
sum_tree = { path = "$gpuiRootNormalized/crates/sum_tree" }
util = { path = "$gpuiRootNormalized/crates/util" }
"@
    Set-Content -LiteralPath $configFile -Value $configContents -NoNewline

    Set-Location $repoRoot
    $cargoInvocationArgs = @("--config", $configFile)
    if (-not $env:QEDIT_CARGO_VERBOSE) {
        $cargoInvocationArgs += "--quiet"
    }
    $cargoInvocationArgs += $CargoArgs

    & cargo @cargoInvocationArgs
}
finally {
    if ($lockFileExists -and $null -ne $lockFileBytes) {
        [System.IO.File]::WriteAllBytes($lockFile, $lockFileBytes)
    }

    Remove-Item -LiteralPath $configFile -ErrorAction SilentlyContinue
}
