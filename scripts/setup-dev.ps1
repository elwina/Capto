# One-command dev environment bootstrap: fresh clone -> runnable `tauri dev`.
#
# Installs the desktop frontend deps, provisions the FFmpeg sidecar, and
# builds + stages the `capto` CLI so `tauri dev` and the control plane work.
#
# Usage:
#   .\scripts\setup-dev.ps1                 # download FFmpeg sidecar (CI-style, needs network)
#   .\scripts\setup-dev.ps1 -Local          # use a locally installed ffmpeg.exe

param(
    [switch]$Local
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

function Invoke-Step {
    param([string]$Title, [scriptblock]$Body)
    Write-Host ""
    Write-Host "==> $Title"
    & $Body
    if ($LASTEXITCODE -ne 0) { throw "Failed step: $Title" }
}

Invoke-Step "npm install --prefix apps/desktop" { npm install --prefix apps/desktop }

Invoke-Step "Provision FFmpeg sidecar into Tauri externalBin" {
    if ($Local -or -not $env:GH_TOKEN) {
        # No token / explicit local: try a local ffmpeg first, then download.
        try {
            & .\scripts\copy-ffmpeg.ps1
        } catch {
            & .\scripts\download-ffmpeg.ps1
        }
    } else {
        & .\scripts\download-ffmpeg.ps1
    }
}

Invoke-Step "cargo build -p capto-cli (debug)" { cargo build -p capto-cli }
Invoke-Step "Stage CLI into Tauri resources (debug)" {
    & .\scripts\copy-cli.ps1 -Profile debug
}

Write-Host ""
Write-Host "Dev environment ready. Start the app with:"
Write-Host "  npm run tauri --prefix apps/desktop -- dev"
