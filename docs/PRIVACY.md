# Capto privacy & data handling

Capto is a **purely local** screen recorder. This document states what Capto
stores, what it never does, and how to delete your data. It exists so agents
and maintainers change behavior with the privacy contract in mind
(PII handling). For how personal data (e.g. screen content) is treated and
guarded, see [docs/PII.md](PII.md); for local analytics, see
[docs/analytics.md](analytics.md); for crash-report tracing, see
[docs/crash-tracing.md](crash-tracing.md).

## What Capto stores on your machine

| Location | Contents |
|----------|----------|
| `<config>/Capto/settings.json` | App settings (output dir, encoder prefs, hotkeys, feature flags). No screen content. |
| `<config>/Capto/cli-server.json` | Control-plane lock (PID / port / random bearer token). Regenerated each launch; never leaves the machine. |
| `<config>/Capto/crashes/crash-*.json` | On a crash: panic subject, exact `file:line` location, captured backtrace, pid/uptime, active feature flags, and a capped trail of recent events (control-plane calls, session transitions, hotkeys) plus the last `x-request-id`. Local only. See [docs/crash-tracing.md](crash-tracing.md). |
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

## Crash reports (contextual, local)

On a panic Capto writes `crash-*.json` (feature flag `crash-reporting`) with
the panic subject, exact `file:line` location, a captured backtrace,
pid/uptime, active feature flags, and a capped **breadcrumb trail** of the
events that led up to the crash — control-plane calls (`method path -> status`),
session transitions, hotkey and lifecycle markers — plus the last
`x-request-id` for log correlation. The trail is scrubbed by construction:
only method/path/status and action names are recorded, never request bodies,
query strings, or the bearer token. Nothing is transmitted anywhere; delete
the `crashes/` folder at any time.

See [docs/crash-tracing.md](crash-tracing.md) for the report schema and how an
agent turns a crash report into a concrete code path.

## Feature flags relevant to data

- `control-plane-metrics` — serve `/v1/metrics` (default enabled).
- `crash-reporting` — write `crash-*.json` reports (default enabled).

## Deleting your data

1. Close Capto.
2. Delete the output folder(s).
3. Delete `<config>/Capto` (settings, lock, crash reports).

Nothing else references Capto data; uninstalling the app removes the rest.
