# Copy a local ffmpeg.exe into Capto's Tauri externalBin layout.
# Does NOT download anything — source must already exist on this machine.
#
# Usage:
#   .\scripts\copy-ffmpeg.ps1
#   .\scripts\copy-ffmpeg.ps1 -Source "C:\path\to\ffmpeg.exe"

param(
    [string]$Source = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$BinDir = Join-Path $RepoRoot "apps\desktop\src-tauri\binaries"

function Resolve-FfmpegSource {
    param([string]$Explicit)

    if ($Explicit) {
        $p = $Explicit.Trim()
        if (Test-Path -LiteralPath $p -PathType Leaf) { return (Resolve-Path -LiteralPath $p).Path }
        $asDir = Join-Path $p "ffmpeg.exe"
        if (Test-Path -LiteralPath $asDir -PathType Leaf) { return (Resolve-Path -LiteralPath $asDir).Path }
        throw "Source not found: $Explicit"
    }

    if ($env:FFMPEG_PATH) {
        $p = $env:FFMPEG_PATH.Trim()
        if (Test-Path -LiteralPath $p -PathType Leaf) { return (Resolve-Path -LiteralPath $p).Path }
        $asDir = Join-Path $p "ffmpeg.exe"
        if (Test-Path -LiteralPath $asDir -PathType Leaf) { return (Resolve-Path -LiteralPath $asDir).Path }
    }

    $where = & where.exe ffmpeg 2>$null | Select-Object -First 1
    if ($where -and (Test-Path -LiteralPath $where -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $where).Path
    }

    throw @"
No local ffmpeg.exe found.
Install FFmpeg on this machine (or set FFMPEG_PATH), then re-run:
  .\scripts\copy-ffmpeg.ps1
  .\scripts\copy-ffmpeg.ps1 -Source path\to\ffmpeg.exe
"@
}

$triple = (& rustc --print host-tuple 2>$null | Select-Object -First 1).Trim()
if (-not $triple) {
    throw "Could not determine Rust host triple (rustc --print host-tuple)."
}

$src = Resolve-FfmpegSource -Explicit $Source
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$destPlain = Join-Path $BinDir "ffmpeg.exe"
$destTriple = Join-Path $BinDir "ffmpeg-$triple.exe"

Copy-Item -LiteralPath $src -Destination $destPlain -Force
Copy-Item -LiteralPath $src -Destination $destTriple -Force

Write-Host "Source : $src"
Write-Host "Copied : $destPlain"
Write-Host "Copied : $destTriple"
Write-Host "Done. Capto will use only this bundled binary (no PATH fallback)."
