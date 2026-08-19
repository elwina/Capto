# Validate that AGENTS.md still exposes the sections agents rely on.
#
# AGENTS.md is the canonical agent-facing guide; if a required section is
# renamed/dropped, agents lose the context they need. This is a light
# structural lint (enforced in CI via ci.yml -> hygiene job).
#
# Usage:
#   .\scripts\validate-agents-md.ps1

param([string]$RelativePath = "AGENTS.md")

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$file = Join-Path $RepoRoot $RelativePath

if (-not (Test-Path -LiteralPath $file)) {
    Write-Host "AGENTS.md not found at $RelativePath"
    exit 1
}

$content = Get-Content -Raw -LiteralPath $file
$required = @(
    "## Non-negotiables",
    "## Repo layout",
    "## Binaries",
    "## Dev commands",
    "## CI / Release"
)

$missing = $required | Where-Object { $content -notmatch [regex]::Escape($_) }
if ($missing.Count -gt 0) {
    Write-Host "AGENTS.md is missing required sections:"
    $missing | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "AGENTS.md structure valid ($($required.Count) required sections present)."
exit 0
