# Scan tracked source for likely PII / secret material (custom regex scanner).
#
# Purpose (pii_handling): Capto keeps personal data local by design, but
# screen recordings and config may still touch PII, and source must never
# accidentally embed real emails / SSNs / card numbers / private keys. This
# lightweight, dependency-free scanner guards that in CI (hygiene job) and
# locally. It is deliberately conservative to avoid false positives - it only
# flags well-shaped values, not loose number ranges, and it filters asset
# names like `icon@2x.png` (TLD is a known file extension, not a real email).
#
# It is the PII counterpart to scripts/scan-tech-debt.ps1 and follows the same
# zero-tolerance policy: any hit fails the check (fix or remove the value, or
# use a placeholder).
#
# Usage:
#   .\scripts\scan-pii.ps1            # repo root
#   .\scripts\scan-pii.ps1 -OutFile report.txt

param(
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

# Tracked source extensions the scanner considers.
$sourceExts = @(".rs", ".ts", ".tsx", ".js", ".jsx", ".ps1", ".py", ".toml", ".json")

# TLDs that are really file extensions — `icons/128x128@2x.png` must not be a hit.
$assetTlds = @(
    "png", "jpg", "jpeg", "gif", "svg", "webp", "ico", "bmp", "avif",
    "json", "html", "htm", "css", "js", "mjs", "ts", "tsx", "jsx", "rs",
    "toml", "yaml", "yml", "md", "txt", "xlsx", "docx", "pdf",
    "zip", "tar", "gz", "dll", "exe", "msi",
    "mp4", "mp3", "wav", "flac", "ogg", "lock"
)

# Keep every pattern free of literal examples so this file never flags itself.
$emailPattern = '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'
$ssnPattern = '\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b'
$cardPattern = '\b[0-9]{4}[ -][0-9]{4}[ -][0-9]{4}[ -][0-9]{4}\b'
$pemPattern = '-----BEGIN [A-Z ]*PRIVATE KEY-----'

$hits = @()
foreach ($rel in git -C $RepoRoot ls-files) {
    $ext = [System.IO.Path]::GetExtension($rel).ToLowerInvariant()
    if ($sourceExts -notcontains $ext) { continue }

    $path = Join-Path $RepoRoot $rel
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
    try {
        $len = (Get-Item -LiteralPath $path).Length
    } catch { continue }
    if ($len -gt 5MB) { continue }   # generated/asset blobs are not source

    try { $text = [System.IO.File]::ReadAllText($path) } catch { continue }

    # Email addresses, skipping asset names whose "TLD" is a file extension.
    foreach ($m in [regex]::Matches($text, $emailPattern)) {
        $tld = $m.Value.Substring($m.Value.LastIndexOf('.') + 1).ToLowerInvariant()
        if ($assetTlds -contains $tld) { continue }
        $hits += "$rel : $($m.Value)"
    }
    foreach ($m in [regex]::Matches($text, $ssnPattern)) {
        $hits += "$rel : $($m.Value)"
    }
    foreach ($m in [regex]::Matches($text, $cardPattern)) {
        $hits += "$rel : $($m.Value)"
    }
    foreach ($m in [regex]::Matches($text, $pemPattern)) {
        $hits += "$rel : $($m.Value)"
    }
}

# De-dup while preserving order.
$hits = $hits | Select-Object -Unique

if ($OutFile) {
    $hits | Set-Content -Path $OutFile
}

if ($hits.Count -gt 0) {
    Write-Host "PII / secret material found in tracked source (replace with a placeholder):"
    $hits | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "PII scan passed: no emails / SSNs / card numbers / private keys in tracked source."
exit 0
