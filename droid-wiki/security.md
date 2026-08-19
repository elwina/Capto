# Security

Active contributors: elwina

Capto's security story follows from its design: a **purely local** Windows screen recorder. There are no upload SDKs, no accounts, and no remote endpoints in product code, so the threat model is small. The only live network surface is a **loopback HTTP control plane** that lets a local CLI drive the running desktop. Security work therefore concentrates on that one surface, on the FFmpeg supply chain, and on keeping secrets out of the repo, not on defending a deployed web service.

This page documents the trust boundaries honestly, including the accepted local threat model, and maps each control to a file you can cite. See `docs/PRIVACY.md`, `docs/security-testing.md`, and `docs/runbooks/security.md` for the operating detail.

## Local-only product surface

- **No undergrading surface to the network**: no upload/storage/embed SDKs, no sharing features, no accounts (AGENTS.md non-negotiables).
- **Outputs stay in user-controlled directories**: recordings and screenshots go to the output folder (default `Videos/Capto`); settings, the control-plane lock, and crash reports live in `<config>/Capto`. Nothing transmits off-box.
- Feature flags gate the only two data-producing surfaces, and both are local: `/v1/metrics` (`control-plane-metrics`) and crash reports (`crash-reporting`). There is no remote telemetry. See [Reference configuration](reference/configuration.md), until written, `docs/feature-flags.md` and `docs/PRIVACY.md`.

## Control-plane security

The desktop exposes a small HTTP server on loopback for the CLI and agent control plane (see [Control-plane API](api/index.md) and [api/endpoints](api/endpoints.md)):

- **Loopback only**: binds `127.0.0.1` on an **ephemeral port** (`TcpListener::bind("127.0.0.1:0")`), never on a routable interface.
- **Bearer token, per run**: a UUID token is generated each launch and written, along with PID and the bound port, to the lock file `<config>/Capto\cli-server.json`. It is regenerated on every run and never leaves the machine.
- **Auth middleware**: every `/v1/*` route calls `check_auth` in `apps/desktop/src-tauri/src/cli_server.rs`, which requires an exact `Authorization: Bearer <token>` match. Unit tests there cover correct token, wrong token, missing token, non-Bearer scheme, and value-less `Bearer`.
- **Token never logged**: logs scrub the token. `crates/capto-ipc/src/redact.rs` masks `Bearer <token>` and token-like query values in any error/URL text reaching logs; `docs/PRIVACY.md` documents the log-scrubbing contract.
- **Request-id tracing**: the server propagates/echoes an `x-request-id`, and the recorded breadcrumb trail keeps method / path / status / request-id only, never bodies, query strings, or the token.

### Honest threat model

Documented limitations that are accepted by design (see `docs/PRIVACY.md`):

- **Any local process that can read the config directory can read the token** from `cli-server.json` and drive the control plane. This is the accepted local threat model, Capto assumes the machine and its user are reasonably trusted; it defends against casual wireshark/curl mistakes, not a fully compromised local attacker.
- **Control plane is unencrypted HTTP** over loopback. It never crosses a host boundary, so transport encryption would add little attacker value while complicating agent tooling (the API pages document the plain contract).
- **Single-instance app**: the desktop owns the lock and one `RecordingSession`; the CLI is a client over the control plane, never a second session owner.

## Input validation

- **Settings patch** is JSON-typed through `serde` (`AppSettings`); unknown keys are rejected (the CLI raises a usage error), and missing fields fall back to typed defaults. See [capto-capture/engine internals in crates](crates/index.md) and `docs/ARCHITECTURE.md` for session state.
- **Record/shot regions** are validated against the virtual screen: `clamp_rect` in `crates/capto-capture/src/desktop.rs` clamps out-of-bounds requests instead of trusting client coordinates.
- **Config values** are typed (`AppSettings` serde defaults for missing fields) rather than free-form strings, so a malformed or malicious config cannot inject unexpected values.

## Sidecar supply chain

The bundling pipeline in [Deployment](deployment.md) protects the encoder binary at every stage:

- **FFmpeg is pinned**: only `elwina/capto-ffmpeg` Releases, pinned in `.github/capto-ffmpeg.env`. It is **never** pulled from system PATH at runtime.
- **Verified download**: `scripts/download-ffmpeg.ps1` checks **SHA-256** against the release `SHA256SUMS` and, in CI/Release, **`gh attestation verify`** (Sigstore / Artifact Attestations) via `-VerifyAttestation`.
- **Updater is signed**: `latest.json` is signed with minisign (public key in `apps/desktop/src-tauri/tauri.conf.json` → `plugins.updater.pubkey`; private key is the `TAURI_SIGNING_PRIVATE_KEY` GitHub secret). A gitignored local copy lives at `.secrets/capto.key` and must never be committed. Key rotation is rare and deliberate (existing installs stop verifying after a pubkey change unless a bridge release is shipped), see [Updates](features/updates.md) and `docs/CI.md`.

## Hook surface

Global hotkeys are Windows low-level input hooks (`WH_MOUSE_LL` / `WH_KEYBOARD_LL`) in `crates/capto-hooks/src/lib.rs`. They are only installed while enabled in settings and are torn down when the feature is disabled or the app exits, the app does not globally hook the input system by default.

## Tooling

Static, dynamic, and repo-hygiene checks run automatically (see [Deployment](deployment.md) for the workflow table):

- **CodeQL** (`.github/workflows/codeql.yml`), JavaScript/TypeScript semantic analysis on push/PR plus a weekly schedule. Rust support is still beta and is left out.
- **Secret scanning** (`.github/workflows/secret-scan.yml`), Gitleaks over full repo history (deep `fetch-depth: 0`), uploading redacted SARIF to the Security tab. This catches hardcoded keys that GitHub's own provider-pattern scanning would miss.
- **Control-plane DAST**, `scripts/control-plane-dast.ps1` is a black-box suite against a *running* desktop. It asserts: no auth → `401`; wrong token → `401`; real token → `200 ok:true`; unknown route → `404`; malformed JSON on POST → `4xx`; and no response body leaks the bearer token. See [How to monitor](how-to-monitor/index.md) (`docs/security-testing.md`).
- **PII / scanner**, `scripts/scan-pii.ps1` scans for emails, SSNs, card numbers, and private keys in CI hygiene.
- **No secrets in the repo**: `FACTORY_API_KEY` and `TAURI_SIGNING_PRIVATE_KEY` are Actions secrets only. A missing Droid key fails those jobs without blocking merges (they are not required checks).

## Secrets runbook pointers

- **Incident response**: `docs/runbooks/security.md`, steps if CodeQL, Gitleaks, or the DAST probes fire.
- **Data handling**: `docs/PII.md`, `docs/data.md`, `docs/PRIVACY.md`.
- **Local privacy observability**: see [How to monitor](how-to-monitor/index.md) for how the local-first `/v1/metrics` and crash reports relate to the privacy contract.

## Repo defense

- **Branch protection & review**: per `docs/CI.md`, there are no required checks on merge; Droid review is advisory, not a gate.
- **CODEOWNERS** pins the sole maintainer (`@elwina`) as required reviewer for changes.
- **PR template** asks contributors to consider security implications of a change, keeping it a fast-fail human gate before automated review runs.

## Integration points

- [Deployment](deployment.md), the CI/Release pipeline that runs CodeQL, secret scan, the FFmpeg attestation, and the DAST probe.
- [Control-plane API](api/index.md) and [api/endpoints](api/endpoints.md), the auth header and endpoint contract.
- [capto-ipc](crates/capto-ipc.md), token redaction and the control-plane lockfile.
- [Updates](features/updates.md), minisign signing and key rotation.
- [Reference configuration](reference/configuration.md), feature-flag gating (`control-plane-metrics`, `crash-reporting`).
- [Glossary](overview/glossary.md), project terminology (control plane, token, lockfile, when used).
