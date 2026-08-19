# Dead feature-flag scan.
#
# Fails CI when a *declared* feature flag is never referenced by runtime code
# outside its definition in crates/capto-core/src/flags.rs. This keeps flag
# churn visible: a flag that is no longer wired up is dead weight waiting to
# accumulate (dead_feature_flag_detection).
#
# The list below must stay in sync with `flags::all()` in
# crates/capto-core/src/flags.rs — each entry is the Rust CONST name of the
# flag. Reference count is measured with `git grep` over tracked sources;
# a flag counts as alive when it appears at least twice (its definition plus
# one or more runtime references such as `flags::CONTROL_PLANE_METRICS`).
#
# Usage:  .\scripts\scan-dead-flags.ps1

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

# Keep in sync with crates/capto-core/src/flags.rs.
$declared = @(
    "CONTROL_PLANE_METRICS",
    "CRASH_REPORTING"
)

$fail = $false
foreach ($flag in $declared) {
    # Count references across tracked AND untracked source files (git grep
    # only sees tracked files, which would under-count brand-new modules).
    $files = git -C $RepoRoot ls-files -co --exclude-standard -- "*.rs" "*.ts" "*.tsx" "*.js" "*.jsx"
    $count = 0
    foreach ($rel in $files) {
        $path = Join-Path $RepoRoot $rel
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $count += @(Select-String -LiteralPath $path -Pattern "\b$flag\b" -AllMatches).Count
        }
    }
    if ($count -lt 2) {
        Write-Host "DEAD FEATURE FLAG: '$flag' has only $count reference(s) (need definition + >=1 runtime use)."
        Write-Host "  Remove it from flags.rs + this script, or wire it into runtime code."
        $fail = $true
    } else {
        Write-Host "OK: '$flag' referenced $count time(s)."
    }
}

if ($fail) {
    Write-Host "Dead feature-flag scan FAILED."
    exit 1
}
Write-Host "Dead feature-flag scan passed ($($declared.Count) declared flags all alive)."
exit 0
