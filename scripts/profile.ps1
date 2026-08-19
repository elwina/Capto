<#
# Local performance-profiling helper (dev use).
#
# Capto is local-first, so profiling stays on the machine: no cloud profiler,
# no third-party APM. It covers three tiers, mirrored in docs/profiling.md:
#
#   Tier 1 - Build time  : cargo build --workspace --timings, prints the
#                          cross-crate build-duration report location.
#   Tier 2 - Runtime     : hot-path timing lines (frame pump) + /v1/metrics.
#   Tier 3 - CPU samples : Windows ETW -> WPA CPU-Usage flamegraph recipe.
#
# Usage:
#   .\scripts\profile.ps1                     # build-time profiling
#   .\scripts\profile.ps1 -RunRuntime         # record round-trip w/ timing logs
#   .\scripts\profile.ps1 -EtfGuide           # print the ETW -> WPA recipe
#   .\scripts\profile.ps1 -Package capto-cli  # profile only one package
#>
param(
    [switch]$RunRuntime,
    [switch]$EtfGuide,
    [string]$Package = "capto-app"
)
$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

$etf = @'
ETW CPU sampling (Windows, elevations required once):
  1. Open an ADMIN PowerShell and start the CPU trace:
       wpr -start GeneralProfile -filemode -start  # or: wpr -start CPU -filemode
  2. In another window exercise Capto (record a few seconds, browse the UI).
  3. Stop and convert to a WPA viewable .etl:
       wpr -stop profile.etl
  4. Open profile.etl in Windows Performance Analyzer (WPA):
       "CPU Usage (Sampled)" -> graph -> "Stack" -> right-click "Load Symbols".
     The resulting per-function CPU stack is the flamegraph view for
     capto-app.exe; sort by % and map frames to source in this repo.
  No admin? Use xperf -on Latency for a non-elevated trace (partial stacks).
'@

if ($EtfGuide) {
    Write-Host $etf
    exit 0
}

if ($RunRuntime) {
    Write-Host "Tier 2 runtime profiling - record round-trip with timing logs."
    Write-Host ""
    Write-Host "NOTE: the frame-pump timing lines are emitted by the DESKTOP process."
    Write-Host "Launch the desktop with the filter set, then run this round-trip:"
    Write-Host ""
    Write-Host "  `$env:CAPTO_LOG = 'capto_core=debug,capto=warn'"
    Write-Host "  target\debug\capto-app.exe   # or the installed Capto"
    Write-Host ""
    & .\scripts\qa-smoke.ps1 -RunRecordRoundtrip
    Write-Host ""
    Write-Host "Then grep the desktop's stderr for the frame-pump lines:"
    Write-Host "  slow ffmpeg write - capture outrunning encoder      <-- bottleneck"
    Write-Host "  frame pump finished                                <- totals"
    exit 0
}

# --- Tier 1: build-time profiling ------------------------------------------
Write-Host "Tier 1 build-time profiling (cargo build --timings) for package '$Package'..."
Push-Location $RepoRoot
cargo build -p $Package --release --timings
$exit = $LASTEXITCODE
Pop-Location
if ($exit -ne 0) { exit $exit }

$html = Get-ChildItem -Path (Join-Path $RepoRoot "target\cargo-timings") -Filter "*.html" |
    Sort-Object LastWriteTime | Select-Object -Last 1
if ($html) {
    Write-Host ""
    Write-Host "Build-duration report: $($html.FullName)"
    Write-Host "Open it in a browser; the longest crates are your compile-time hotspots."
    Write-Host "(CI also uploads target/cargo-timings/ as an artifact each run.)"
} else {
    Write-Host "No --timings report found under target\cargo-timings."
}
Write-Host ""
Write-Host "Runtime + CPU-sample guidance:"
Write-Host "  .\scripts\profile.ps1 -RunRuntime"
Write-Host "  .\scripts\profile.ps1 -EtfGuide"
