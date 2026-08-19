# Scan tracked source for tech-debt markers (TODO/FIXME/HACK/XXX).
#
# This repo keeps a zero-debt policy: the CI job fails if any marker is added.
# There is no exemption comment — fix the debt.
#
# Usage:
#   .\scripts\scan-tech-debt.ps1            # repo root
#   .\scripts\scan-tech-debt.ps1 -OutFile report.txt

param(
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

$hits = git -C $RepoRoot grep -n -I -E "TODO|FIXME|HACK|XXX" -- `
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
Write-Host "Tech-debt scan passed: no TODO/FIXME/HACK/XXX markers in source."
exit 0
