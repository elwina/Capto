# Enable the committed local pre-commit hooks for this repository.
#
# Hooks live in the tracked .githooks/ directory (so every clone has them);
# each developer opts in per-clone by pointing core.hooksPath at it. No files
# are copied into .git — pull the hooks out of git, not from a hidden copy.
#
# Usage:
#   .\scripts\install-hooks.ps1

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

git -C $RepoRoot config core.hooksPath .githooks
if ($LASTEXITCODE -ne 0) { throw "failed to set core.hooksPath" }

$current = git -C $RepoRoot config --get core.hooksPath
Write-Host "core.hooksPath -> $current"
if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot ".githooks\pre-commit"))) {
    throw ".githooks\pre-commit missing"
}
Write-Host "Pre-commit hooks installed. On every commit: file-size + tech-debt + rustfmt checks, and frontend lint/format/knip when apps/desktop changes."
Write-Host "Bypass once: git commit --no-verify"
