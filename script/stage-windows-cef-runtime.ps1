param(
    [string]$TargetDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) "target\debug")
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Resolve-CefRuntimeSource {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ResolvedTargetDirectory
    )

    $candidates = Get-ChildItem -Path (Join-Path $ResolvedTargetDirectory "build") -Directory -Filter "cef-dll-sys-*" -ErrorAction SilentlyContinue |
        ForEach-Object {
            $runtimePath = Join-Path $_.FullName "out\cef_windows_x86_64"
            if (Test-Path (Join-Path $runtimePath "libcef.dll")) {
                [pscustomobject]@{
                    RuntimePath   = $runtimePath
                    LastWriteTime = (Get-Item (Join-Path $runtimePath "libcef.dll")).LastWriteTime
                }
            }
        } |
        Sort-Object LastWriteTime -Descending

    return $candidates | Select-Object -First 1 -ExpandProperty RuntimePath
}

function Sync-CefRuntimeContents {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceDirectory,
        [Parameter(Mandatory = $true)]
        [string]$DestinationDirectory
    )

    if (Test-Path $DestinationDirectory) {
        Remove-Item -LiteralPath $DestinationDirectory -Recurse -Force
    }
    New-Item -ItemType Directory -Path $DestinationDirectory -Force | Out-Null

    $directoriesToCopy = @("locales")
    $filePatternsToCopy = @("*.bin", "*.dat", "*.dll", "*.json", "*.pak")

    foreach ($directoryName in $directoriesToCopy) {
        $sourcePath = Join-Path $SourceDirectory $directoryName
        if (Test-Path $sourcePath) {
            Copy-Item -LiteralPath $sourcePath -Destination (Join-Path $DestinationDirectory $directoryName) -Recurse -Force
        }
    }

    foreach ($pattern in $filePatternsToCopy) {
        Get-ChildItem -LiteralPath $SourceDirectory -Filter $pattern -File -ErrorAction SilentlyContinue | ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $DestinationDirectory $_.Name) -Force
        }
    }
}

$targetDirectory = (Resolve-Path $TargetDirectory).Path
$cefSource = Resolve-CefRuntimeSource -ResolvedTargetDirectory $targetDirectory
if (-not $cefSource) {
    throw "Unable to locate built CEF runtime under '$targetDirectory\build'."
}

$cefRuntimeDirectory = Join-Path $targetDirectory "cef_runtime"
Sync-CefRuntimeContents -SourceDirectory $cefSource -DestinationDirectory $cefRuntimeDirectory

Write-Output $cefRuntimeDirectory
