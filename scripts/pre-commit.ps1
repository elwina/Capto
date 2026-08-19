# Fast local pre-commit gate (runs before every commit once hooks are installed).
#
# Deliberately lightweight: full test suites (cargo test, vitest, coverage)
# still run in CI only. This catches the cheap, high-frequency breakage
# classes locally so you never push a commit that instantly burns a CI run:
#   1. oversized source files          (scripts/check-file-size.ps1)
#   2. new debt markers                (scripts/scan-tech-debt.ps1)
#   3. rustfmt drift                   (cargo fmt --all --check)
#   4. frontend gates (lint/format/knip) only when apps/desktop sources change
#
# Exit code 1 blocks the commit. Disable for a "quick" commit with:
#   git commit --no-verify

param()

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Push-Location $Root
$fail = $false

function Invoke-Gate([string]$Title, [scriptblock]$Body) {
    Write-Host ""
    Write-Host "==> [pre-commit] $Title"
    & $Body
    if ($LASTEXITCODE -ne 0) { $script:fail = $true }
}

# --- cheap repo-wide gates -------------------------------------------
Invoke-Gate "file-size check (over)large source" { & .\scripts\check-file-size.ps1 }
Invoke-Gate "tech-debt marker scan"             { & .\scripts\scan-tech-debt.ps1 }
Invoke-Gate "cargo fmt --all --check"            { cargo fmt --all --check }

# --- frontend gates, only when the frontend actually changed ---------
$staged = git diff --cached --name-only --diff-filter=ACMR
$frontend = @($staged) | Where-Object { $_ -match '^apps/desktop/(src/|package\.json$|package-lock\.json$|eslint\.config\.js$|\.prettierrc|vitest\.config\.ts$)' }

if ($frontend.Count -gt 0) {
    Push-Location (Join-Path $Root "apps\desktop")
    Invoke-Gate "npm run lint"        { npm run lint }
    Invoke-Gate "npm run format:check" { npm run format:check }
    Invoke-Gate "npm run knip"        { npm run knip }
    Pop-Location
} else {
    Write-Host "==> [pre-commit] frontend gates skipped (no apps/desktop source staged)"
}

Pop-Location
if ($fail) {
    Write-Host ""
    Write-Host "[pre-commit] FAILED - see errors above. Use 'git commit --no-verify' to bypass once."
    exit 1
}
Write-Host "[pre-commit] OK"
exit 0
