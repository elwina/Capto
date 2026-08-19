# Scan tracked source for tech-debt markers.
#
# This repo keeps a zero-debt policy: the CI job fails if any marker is added
# to source. There is no exemption comment — fix the debt.
# Note: the marker keywords are intentionally never written literally in this
# file so the scanner cannot flag itself.
#
# Usage:
#   .\scripts\scan-tech-debt.ps1            # repo root
#   .\scripts\scan-tech-debt.ps1 -OutFile report.txt

param(
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

$mark = "T" + "O" + "D" + "O|F" + "IX" + "M" + "E|H" + "AC" + "K|X" + "XX"

$hits = git -C $RepoRoot grep -n -I -E $mark -- `
    "*.rs" "*.ts" "*.tsx" "*.js" "*.jsx" "*.ps1" "*.py" 2>$null
if ($LASTEXITCODE -eq 1) {
    # git grep exits 1 when nothing matches.
    $hits = @()
}

if ($OutFile) {
    $hits | Set-Content -Path $OutFile
}

if ($hits.Count -gt 0) {
    Write-Host "Tech-debt markers found in source (fix or remove them):"
    $hits | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "Tech-debt scan passed: zero debt markers in tracked source."
exit 0
