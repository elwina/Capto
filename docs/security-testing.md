# Security testing

Capto is a desktop app whose only network surface is the **loopback HTTP
control plane**. Security tooling is therefore targeted at that surface rather
than a deployed web service.

## Static (SAST)

- **CodeQL** (`codeql.yml`): JavaScript/TypeScript analysis on push/PR plus a
  weekly schedule.
- **Gitleaks** (`secret-scan.yml`): full-history secret scanning.

## Dynamic (DAST, control-plane layer)

The control plane is a real HTTP server, so it gets a black-box probe suite:

- `scripts/control-plane-dast.ps1` drives a **running** Capto desktop with
  adversarial requests and asserts the expected rejection:
  1. no auth → `401 unauthorized`
  2. wrong token → `401`
  3. real token → `200` + `ok:true`
  4. unknown route (with auth) → `404`
  5. malformed JSON body → `4xx`
  6. no response body leaks the bearer token

Run it against a started desktop:

```powershell
.\scripts\control-plane-dast.ps1
```

## Auth unit tests

The auth check itself is unit-tested in
`apps/desktop/src-tauri/src/cli_server.rs` (runs under
`cargo test -p capto-app` in CI): correct token accepted; missing/wrong
token, non-Bearer scheme, and value-less `Bearer` rejected.

## Runbook

If any of these fire: `docs/runbooks/security.md`.
