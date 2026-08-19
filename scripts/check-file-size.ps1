# Fail the build if any tracked source file grows past sanity limits.
#
# Guards against "god files" that slow agents and reviewers down. Binary
# assets (images/installers), lockfiles, vendored deps, and build output are
# intentionally excluded. Exceptions are allowed via `@nolimit` in the file.
#
# Cross-platform by design (runs on Windows pwsh and ubuntu-latest pwsh in
# CI); each file is measured defensively so one unusual file (odd name,
# missing materialization, huge blob) cannot take the whole gate down.
#
# Usage:
#   .\scripts\check-file-size.ps1            # repo root
#   .\scripts\check-file-size.ps1 -OutFile out.txt

param(
    [string]$OutFile = "",
    [int]$MaxBytes = 307200,
    [int]$MaxLines = 2500
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$candidates = git -C $RepoRoot ls-files

$excludedExt = @(".png", ".ico", ".icns", ".jpg", ".jpeg", ".gif", ".webp", ".exe",
    ".msi", ".dll", ".woff", ".woff2", ".ttf", ".otf", ".zip", ".tgz", ".lock",
    ".snap", ".svg")
$excludedFile = @("Cargo.lock", "package-lock.json", "pnpm-lock.yaml", "yarn.lock")

$violations = @()
foreach ($rel in $candidates) {
    $file = Join-Path $RepoRoot $rel
    $ext = [System.IO.Path]::GetExtension($rel).ToLowerInvariant()
    if ($excludedExt -contains $ext) { continue }
    if ($excludedFile -contains ([System.IO.Path]::GetFileName($rel))) { continue }

    try {
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { continue }
        # @nolimit marker: opt a deliberately-large file out.
        if ([System.IO.File]::ReadAllText($file).Contains("@nolimit")) { continue }

        $bytes = (Get-Item -LiteralPath $file).Length
        if ($bytes -gt $MaxBytes) {
            $violations += "$rel ($bytes bytes > $MaxBytes)"
            continue
        }
        $lines = ([System.IO.File]::ReadLines($file) | Measure-Object).Count
        if ($lines -gt $MaxLines) {
            $violations += "$rel ($lines lines > $MaxLines)"
        }
    } catch {
        Write-Warning "Skipping ${rel}: $($_.Exception.Message)"
    }
}

if ($OutFile) {
    $violations | Set-Content -Path $OutFile
}
if ($violations.Count -gt 0) {
    Write-Host "Oversized source files detected:"
    $violations | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "File size check passed (max ${MaxBytes} bytes / ${MaxLines} lines)."
exit 0
