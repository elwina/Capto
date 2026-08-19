# Fail if the desktop app version drifts between the places that must agree:
#   apps/desktop/package.json          (npm / release tooling version)
#   apps/desktop/src-tauri/tauri.conf.json  (Tauri bundled-version source)
#
# tauri-action substitutes v__VERSION__ from the Git tag, but a drift between
# these two files is exactly the kind of mismatch that produces a release
# whose installer and package.json disagree. Keep them in lockstep.
#
# Usage:
#   .\scripts\check-version-drift.ps1

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

$npmPkg = Join-Path $RepoRoot "apps/desktop/package.json"
$tauriConf = Join-Path $RepoRoot "apps/desktop/src-tauri/tauri.conf.json"

if (-not (Test-Path -LiteralPath $npmPkg)) { Write-Host "missing $npmPkg"; exit 1 }
if (-not (Test-Path -LiteralPath $tauriConf)) { Write-Host "missing $tauriConf"; exit 1 }

$npmVersion = (Get-Content -Raw -LiteralPath $npmPkg | ConvertFrom-Json).version
$tauriVersion = (Get-Content -Raw -LiteralPath $tauriConf | ConvertFrom-Json).version

if ($npmVersion -ne $tauriVersion) {
    Write-Host "Version drift: package.json=$npmVersion  tauri.conf.json=$tauriVersion"
    exit 1
}
Write-Host "Version drift check passed (version $npmVersion)."
exit 0
