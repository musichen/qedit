param(
    [switch]$Run
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Require-Command {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [string]$Message
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        if ($Message) {
            throw $Message
        }
        throw "Required command not found: $Name"
    }
}

function Add-ToPathIfPresent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathEntry
    )

    if ((Test-Path $PathEntry) -and -not (($env:PATH -split ";") -contains $PathEntry)) {
        $env:PATH = "$PathEntry;$env:PATH"
    }
}

function Ensure-Ninja {
    if (Get-Command ninja -ErrorAction SilentlyContinue) {
        return
    }

    Require-Command -Name winget -Message "Ninja is required and winget is not available. Install Ninja manually and re-run this script."

    Write-Host "Installing Ninja with winget..."
    & winget install --id Ninja-build.Ninja --scope user --accept-package-agreements --accept-source-agreements --silent

    Add-ToPathIfPresent -PathEntry (Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links")

    if (-not (Get-Command ninja -ErrorAction SilentlyContinue)) {
        throw "Ninja installation completed, but ninja.exe is not visible in PATH yet. Open a new shell and re-run this script."
    }
}

function Invoke-BuildCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    if ($script:UseLocalGpuiResolved) {
        & powershell -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "cargo-gpui-local.ps1") @Arguments
    } else {
        & cargo @Arguments
    }
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

Add-ToPathIfPresent -PathEntry (Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links")

Require-Command -Name cargo -Message "Cargo is required. Install rustup and the Rust toolchain first."
Require-Command -Name cmake -Message "CMake is required. Install CMake and ensure it is on PATH."
Ensure-Ninja

$script:UseLocalGpuiResolved = $false
$defaultGpuiPath = Join-Path (Split-Path $repoRoot -Parent) "gpui"
if (Test-Path (Join-Path $defaultGpuiPath "crates\gpui\Cargo.toml")) {
    $script:UseLocalGpuiResolved = $true
    Write-Host "Using sibling GPUI checkout at '$defaultGpuiPath'."
}

Write-Host "Building companion CLI..."
Invoke-BuildCommand -Arguments @("build", "--package", "cli", "--bin", "cli")

if ($Run) {
    Write-Host "Building Qedit..."
    Invoke-BuildCommand -Arguments @("build", "-p", "zed")

    $zedBinary = Join-Path $repoRoot "target\debug\zed.exe"
    if (-not (Test-Path $zedBinary)) {
        throw "Expected Qedit binary not found at '$zedBinary'."
    }

    $cefRuntimeDirectory = & powershell -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "stage-windows-cef-runtime.ps1")
    if (-not $cefRuntimeDirectory) {
        throw "Failed to stage the Windows CEF runtime."
    }
    $cefRuntimeDirectory = $cefRuntimeDirectory.Trim()
    $env:CEF_PATH = $cefRuntimeDirectory
    Add-ToPathIfPresent -PathEntry $cefRuntimeDirectory

    Write-Host "Launching Qedit..."
    Start-Process -FilePath $zedBinary -WorkingDirectory $repoRoot | Out-Null
} else {
    Write-Host "Building Qedit..."
    Invoke-BuildCommand -Arguments @("build", "-p", "zed")
    Write-Host ""
    Write-Host "Environment is ready."
    if ($script:UseLocalGpuiResolved) {
        Write-Host "Run '.\script\cargo-gpui-local.ps1 run -p zed' or re-run this script with -Run."
    } else {
        Write-Host "Run 'cargo run -p zed' or re-run this script with -Run."
    }
}
