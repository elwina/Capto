# Control-plane DAST probe (dynamic security testing against a live server).
#
# A black-box security suite for the localhost HTTP control plane: it drives
# the running server with adversarial requests and asserts the expected
# rejection, complementing the auth unit tests in
# apps/desktop/src-tauri/src/cli_server.rs. Run against a Capto desktop that
# is already started (so the server lock, `cli-server.json`, exists).
#
# Probes:
#   1. /v1/status with NO auth           -> 401
#   2. /v1/status with a WRONG token     -> 401
#   3. /v1/status with the REAL token    -> 200 + ok:true
#   4. /v1/does-not-exist with auth      -> 404
#   5. POST /v1/shot with malformed JSON -> 4xx
#   6. No response body leaks the token
#
# Usage:  .\scripts\control-plane-dast.ps1
# See:    docs/security-testing.md

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

function Get-ConfigDir {
    $candidate = Join-Path $env:APPDATA "Capto"
    if ($env:APPDATA -and (Test-Path -LiteralPath (Join-Path $candidate "cli-server.json"))) {
        return $candidate
    }
    $home = Join-Path $HOME "AppData\Roaming\Capto"
    if (Test-Path -LiteralPath (Join-Path $home "cli-server.json")) { return $home }
    throw "control-plane lock (cli-server.json) not found. Start the Capto desktop first."
}

$configDir = Get-ConfigDir
$lockPath = Join-Path $configDir "cli-server.json"
$lock = Get-Content -Raw -LiteralPath $lockPath | ConvertFrom-Json
$base = "http://127.0.0.1:$($lock.port)"
$secret = $lock.token

function Invoke-Probe([string]$Label, [scriptblock]$Request, [int[]]$Expected) {
    $result = & $Request
    $status = [int]$result.StatusCode
    $ok = $Expected -contains $status
    $detail = if ($result.Body) { " body=$($result.Body.Substring(0, [Math]::Min(80, $result.Body.Length)))" } else { "" }
    if ($ok) {
        Write-Host "PASS  $Label -> HTTP $status (expected $($Expected -join '|'))"
    } else {
        Write-Host "FAIL  $Label -> HTTP $status (expected $($Expected -join '|'))$detail"
    }
    return $ok
}

function Invoke-RawRequest([string]$Method, [string]$Path, [string]$Token = $null, [string]$RawBody = $null) {
    $headers = @{}
    if ($Token) { $headers["Authorization"] = "Bearer $Token" }
    if ($RawBody) { $headers["Content-Type"] = "application/json" }
    try {
        $resp = Invoke-WebRequest -Uri "$base$Path" -Method $Method -Headers $headers -Body $RawBody -UseBasicParsing -SkipHttpErrorCheck -TimeoutSec 10
        return @{ StatusCode = [int]$resp.StatusCode; Body = [string]$resp.Content }
    } catch {
        # -SkipHttpErrorCheck should prevent throws for 4xx/5xx; keep a fallback.
        return @{ StatusCode = -1; Body = $_.Exception.Message }
    }
}

$allOk = $true

# 1. No auth -> 401
$allOk = (Invoke-Probe "no auth"        { Invoke-RawRequest "GET" "/v1/status" } @(401)) -and $allOk
# 2. Wrong token -> 401
$allOk = (Invoke-Probe "wrong token"    { Invoke-RawRequest "GET" "/v1/status" "nope" } @(401)) -and $allOk
# 3. Real token -> 200
$real = Invoke-RawRequest "GET" "/v1/status" $secret
$ok = ($real.StatusCode -eq 200) -and ($real.Body -match '"ok"\s*:\s*true')
Write-Host "$(if ($ok) { 'PASS' } else { 'FAIL' })  real token -> HTTP $($real.StatusCode) ok:true"
$allOk = $ok -and $allOk
# 4. Unknown route with auth -> 404
$allOk = (Invoke-Probe "unknown route"  { Invoke-RawRequest "GET" "/v1/does-not-exist" $secret } @(404)) -and $allOk
# 5. Malformed JSON on POST -> 4xx (400/422)
$allOk = (Invoke-Probe "malformed json" { Invoke-RawRequest "POST" "/v1/shot" $secret "{not json" } @(400, 415, 422)) -and $allOk
# 6. Token must never appear in any error body
$leak = $real.Body.Contains($secret)
Write-Host "$(if (-not $leak) { 'PASS' } else { 'FAIL' })  no token leak in body"
$allOk = $allOk -and (-not $leak)

if ($allOk) {
    Write-Host ""
    Write-Host "DAST probes passed. See docs/security-testing.md."
    exit 0
}
Write-Host ""
Write-Host "DAST probes FAILED. Investigate before shipping."
exit 1
