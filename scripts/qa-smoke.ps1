# Interactive QA smoke for the Capto control plane.
#
# The agent-followable end-to-end check documented in docs/QA.md. Requires a
# running Capto desktop (or a dev checkout that can auto-launch it). Exercises
# the loopback control plane through the real `capto` CLI binary:
#
#   doctor -> status -> config path -> outputs recent
#
# A full record/stop round-trip needs a Windows desktop session; enable it
# explicitly with `-RunRecordRoundtrip` (CI runners are headless, so it stays
# off by default).
#
# Usage:
#   .\scripts\qa-smoke.ps1
#   .\scripts\qa-smoke.ps1 -CaptoPath C:\path\to\capto.exe
#   .\scripts\qa-smoke.ps1 -RunRecordRoundtrip
#
# Exits 0 when every step returned a JSON envelope with ok:true.

param(
    [string]$CaptoPath = "",
    [switch]$RunRecordRoundtrip
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

if ($CaptoPath -eq "") {
    # Prefer a staged/built CLI, else fall back to `cargo run`.
    $candidates = @(
        (Join-Path $RepoRoot "target\debug\capto.exe"),
        (Join-Path $RepoRoot "target\release\capto.exe")
    )
    $built = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    $CaptoPath = if ($built) { $built } else { "cargo" }
}

function Invoke-CaptoJson([string]$Arguments) {
    Write-Host "==> capto $Arguments"
    if ($CaptoPath -eq "cargo") {
        $out = Push-Location $RepoRoot; cargo run -q -p capto-cli -- $Arguments 2>$null | Out-String; Pop-Location
    } else {
        $out = & $CaptoPath $Arguments 2>$null | Out-String
    }
    $trimmed = $out.Trim()
    if (-not $trimmed) {
        Write-Host "    produced no output"
        return $null
    }
    try {
        $parsed = $trimmed | ConvertFrom-Json
    } catch {
        Write-Host "    non-JSON output: $trimmed"
        return $null
    }
    if (-not $parsed.ok) {
        Write-Host "    envelope ok=false: $($parsed.error.code) $($parsed.error.message)"
        return $null
    }
    Write-Host "    ok:true"
    return $parsed
}

$failed = $false
$steps = @(
    "doctor",
    "status",
    "config path",
    "outputs recent --limit 3"
)

foreach ($step in $steps) {
    $result = Invoke-CaptoJson $step
    if ($null -eq $result) {
        $failed = $true
        Write-Host "    FAILED: $step"
    }
}

if ($RunRecordRoundtrip) {
    Write-Host "==> record round-trip (requires a desktop session)"
    $started = Invoke-CaptoJson "record start --source display"
    if ($null -eq $started) { $failed = $true }
    Start-Sleep -Seconds 2
    $stopped = Invoke-CaptoJson "record stop"
    if ($null -eq $stopped) { $failed = $true }
    $recent = Invoke-CaptoJson "outputs recent --limit 1"
    if ($null -eq $recent) { $failed = $true }
}

if ($failed) {
    Write-Host ""
    Write-Host "QA smoke FAILED. See docs/QA.md. If the desktop is not running, launch it first (npm run tauri --prefix apps/desktop -- dev) or pass CAPTO_APP_PATH."
    exit 1
}

Write-Host ""
Write-Host "QA smoke passed: all control-plane steps returned ok:true."
exit 0
