# Stage the Capto CLI (`capto.exe`) for the desktop installer.
# Bundled as <install>/cli/capto.exe (not beside Capto.exe — Windows is
# case-insensitive, so Capto.exe and capto.exe cannot share a folder).
# Not published as a separate GitHub Release asset.
#
# Usage:
#   cargo build -p capto-cli --release
#   .\scripts\copy-cli.ps1
#   .\scripts\copy-cli.ps1 -TargetTriple aarch64-pc-windows-msvc
#   .\scripts\copy-cli.ps1 -Source "D:\path\to\capto.exe"

param(
    [string]$TargetTriple = "",
    [string]$Source = "",
    [ValidateSet("release", "debug")]
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$BinDir = Join-Path $RepoRoot "apps\desktop\src-tauri\binaries"
$TargetRoot = Join-Path $RepoRoot "target"

function Resolve-Triple {
    param([string]$Explicit)
    if ($Explicit) { return $Explicit.Trim() }
    $t = (& rustc --print host-tuple 2>$null | Select-Object -First 1)
    if (-not $t) { throw "Could not determine Rust host triple (rustc --print host-tuple)." }
    return $t.Trim()
}

function Resolve-CliSource {
    param([string]$Explicit, [string]$Triple, [string]$BuildProfile)

    if ($Explicit) {
        $p = $Explicit.Trim()
        if (Test-Path -LiteralPath $p -PathType Leaf) { return (Resolve-Path -LiteralPath $p).Path }
        throw "Source not found: $Explicit"
    }

    $candidates = @(
        (Join-Path $TargetRoot "$Triple\$BuildProfile\capto.exe"),
        (Join-Path $TargetRoot "$BuildProfile\capto.exe")
    )
    foreach ($c in $candidates) {
        if (Test-Path -LiteralPath $c -PathType Leaf) {
            return (Resolve-Path -LiteralPath $c).Path
        }
    }

    throw @"
No capto.exe found under target/. Build first, then re-run:
  cargo build -p capto-cli --$BuildProfile --target $Triple
  .\scripts\copy-cli.ps1 -TargetTriple $Triple -Profile $BuildProfile
"@
}

$triple = Resolve-Triple -Explicit $TargetTriple
$src = Resolve-CliSource -Explicit $Source -Triple $triple -BuildProfile $Profile

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$destPlain = Join-Path $BinDir "capto.exe"
$destTriple = Join-Path $BinDir "capto-$triple.exe"

Copy-Item -LiteralPath $src -Destination $destPlain -Force
Copy-Item -LiteralPath $src -Destination $destTriple -Force

Write-Host "Copied CLI:"
Write-Host "  $src"
Write-Host "  -> $destPlain"
Write-Host "  -> $destTriple"
