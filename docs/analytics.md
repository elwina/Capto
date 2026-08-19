# Local product analytics (privacy-first)

Capto deliberately has **no third-party analytics** — no Mixpanel/Amplitude/
PostHog/GA4, no analytics SDK in the dependency tree, nothing transmitted off
the machine (see AGENTS.md non-negotiables and docs/PRIVACY.md).

To still give agents and maintainers visibility into whether features are
actually used — and to measure the impact of a change on user behavior —
Capto keeps a **local, aggregate, in-process** usage counter:

```
GET http://127.0.0.1:<port>/v1/metrics      # requires control-plane auth
```

The snapshot contains:

| Field | What it holds |
|-------|---------------|
| `uptimeMs` | Process uptime at snapshot time. |
| `counters` | Lifetime counters (`app_started`, `http_requests_total`, per-status counts, `recordings_started/stopped`, …). |
| `durations` | Aggregated latency (`http_request_duration_ms`, `record_start_ms`) with count/avg/max. |
| **`usage`** | **Product-usage events**: how many times each feature/action was used this process lifetime. |

## `usage` event names

Recorded at each functional touchpoint (`session_svc` / `lib.rs`):

- `record.start`, `record.stop`, `record.pause`, `record.resume`
- `shot`
- `config.patch`
- `hotkey.start_recording`, `hotkey.pause_recording`,
  `hotkey.stop_recording`, `hotkey.take_screenshot` — the *input method* so
  agents can tell whether a feature is driven by hotkeys vs the control plane.

## How an agent reads it

- `capto` CLI client has no direct metrics subcommand; call the endpoint
  directly with the bearer token from `cli-server.json` (same discovery the
  CLI uses), or read the values from the desktop logs.
- Data is **per-process** (reset each launch) and **aggregate only** — no
  per-user identity, no timestamps beyond `uptimeMs`, no screen content.
- The endpoint is gated by the `control-plane-metrics` feature flag
  (default **enabled**). Disable it like any flag:

```powershell
capto config set --json '{"disabledFlags":["control-plane-metrics"]}'
```

## Why this satisfies "measure impact" without a privacy breach

- All measurements are **local aggregates**: an agent can see "record.start
  was used 42 times and shot 7 times this session" and compare `http_status_500`
  / `http_request_duration_ms` before/after a change — without any beacon,
  cookie, or off-box event.
- Nothing persists: `Metrics` lives in memory only and is gone at exit.
- `usage` is additive: no identifiers, no content, no paths.
