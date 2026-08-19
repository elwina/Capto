# Capto privacy & data handling

Capto is a **purely local** screen recorder. This document states what Capto
stores, what it never does, and how to delete your data. It exists so agents
and maintainers change behavior with the privacy contract in mind
(PII handling).

## What Capto stores on your machine

| Location | Contents |
|----------|----------|
| `<config>/Capto/settings.json` | App settings (output dir, encoder prefs, hotkeys, feature flags). No screen content. |
| `<config>/Capto/cli-server.json` | Control-plane lock (PID / port / random bearer token). Regenerated each launch; never leaves the machine. |
| `<config>/Capto/crashes/crash-*.json` | On a crash: app version, OS, panic message, backtrace. Local only. |
| Output folder (default `Videos/Capto`) | Your recordings / screenshots. |
| Logs (stderr when `RUST_LOG` is set) | Diagnostic lines; secrets are scrubbed (see below). |

## What Capto never does

- **No uploads.** No cloud storage, embed/importer SDKs, or "share to" features
  are built in or planned (see AGENTS.md non-negotiables).
- **No telemetry to third parties.** There is no Mixpanel/PostHog/GA4/Sentry
  in the app. Analytics are **local-only**: the control plane exposes
  `GET /v1/metrics` (auth required) with process counters for agent/operator
  debugging — nothing is transmitted off-box.
- **No analytics SDKs** in the dependency tree.

## Log scrubbing

Log lines never include request bodies, query strings, or the bearer token.
`capto_ipc::redact` additionally masks `Bearer <token>` and token-like query
values in any error/URL text that reaches logs.

## Feature flags relevant to data

- `control-plane-metrics` — serve `/v1/metrics` (default enabled).
- `crash-reporting` — write `crash-*.json` reports (default enabled).

## Deleting your data

1. Close Capto.
2. Delete the output folder(s).
3. Delete `<config>/Capto` (settings, lock, crash reports).

Nothing else references Capto data; uninstalling the app removes the rest.
