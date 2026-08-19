# PII handling

Capto is a screen recorder: by nature the recordings it produces can contain
**personal data** — a person's name on screen, an email draft, a conversation,
a face in a webcam PiP. This document states how Capto treats such data and
what guards exist so both the app **and the repo** stay PII-safe.

## What data Capto may hold, where it lives

| Data | Location | Notes |
|------|----------|-------|
| Screen / webcam content (could contain PII) | Output folder (`output_dir`) | User's own files; Capto never transmits them. |
| App settings | `settings.json` | Output dir, encoder prefs, hotkeys, feature flags; may include **device names / mic labels** (local hardware identifiers). |
| Control-plane lock | `cli-server.json` | PID / port / **random bearer token** (a credential, not PII). |
| Crash reports | `crashes/crash-*.json` | Panic subject, backtrace, breadcrumb trail, feature flags. Could embed a path or command line with a name; never bodies/query values (scrubbed). |
| Logs | stderr when `RUST_LOG`/`CAPTO_LOG` set | Scrubbed of tokens & query secrets by `capto_ipc::redact`. |

## Operating principles

1. **Local-only.** No upload, no share-to, no cloud storage or analytics —
   see AGENTS.md non-negotiables and docs/PRIVACY.md.
2. **Separation.** PII screen content is never mixed into settings, logs, or
   metrics (`/v1/metrics` exposes counts/durations only; `usage` counters carry
   no identifiers or content).
3. **Retention is user-controlled.** Deleting the output folder and
   `<config>/Capto` removes everything Capto wrote (see PRIVACY.md
   "Deleting your data").
4. **Source hygiene.** PII must not be committed to the repository.

## Source-level guard (CI)

`scripts/scan-pii.ps1` runs in the CI hygiene job and locally. It fails if any
**tracked source file** contains a well-shaped email address, US SSN, grouped
payment-card number, or PEM private-key header (using `git grep -E`, so only
tracked source is scanned; docs/screenshots are out of scope). Policy:
zero tolerance — replace real values with placeholders, never add exclusions.

Adjoining guards: `secret-scan.yml` (Gitleaks) protects against
committed secrets, and `capto_ipc::redact` masks tokens/query secrets in
logs at runtime (docs/PRIVACY.md "Log scrubbing" + `docs/crash-tracing.md`).

## When agents change code

- Never log or write request bodies, query strings, or `cli-server.json`'s
  bearer token (keep `capto_ipc::redact` applied to anything that may embed
  URLs/errors).
- Never place PII-shaped values (real emails/IDs/phones) in tests or fixtures;
  use the same placeholders the scanner expects.
- If a feature adds screen-content handling, extend this document and keep the
  data in the output folder — never in `settings.json`, metrics, or breadcrumbs.
