# Metrics

Capto keeps one in-process metrics registry and exposes it only on the loopback control plane at `GET /v1/metrics`. There is no remote aggregation and no dashboard: the endpoint returns a JSON snapshot of counters and durations for the current process lifetime, and that is all. Metrics are a local diagnostic and a local form of product analytics; nothing here implies a service-level target or a monitoring system.

## What is tracked

The registry lives in `crates/capto-core/src/metrics.rs`. A single `Metrics` instance is created at desktop startup, stored in `AppState` (`apps/desktop/src-tauri/src/lib.rs`), and shared into the control plane handlers. Snapshot JSON uses camelCase and splits into three arrays:

| Array | Entry | Meaning |
|-------|-------|---------|
| `counters` | `{ name, count }` | Lifetime monotonic counters. |
| `durations` | `{ name, count, totalMs, avgMs, maxMs }` | Aggregated latency series. |
| `usage` | `{ name, count }` | Product-usage events (the local form of analytics). |

Concrete names recorded today:

- **App lifecycle**: `app_started` counter, incremented once in `apps/desktop/src-tauri/src/lib.rs` during setup.
- **Control plane**: the `telemetry_layer` in `apps/desktop/src-tauri/src/cli_server.rs` records `http_requests_total`, `http_status_<code>` counters, and an `http_request_duration_ms` duration series on every request, plus the `x-request-id` on the breadcrumb trail.
- **Recording**: `crates/capto-core/src/session.rs` / `apps/desktop/src-tauri/src/session_svc.rs` record `recordings_started` and `recordings_stopped` counters, a `record_start_ms` duration series, and `usage` events `record.start`, `record.stop`, `record.pause`, `record.resume`, `config.patch`, and `shot`.
- **Hotkeys**: the `register_hotkeys` function in `apps/desktop/src-tauri/src/lib.rs` records `usage` events `hotkey.start_recording`, `hotkey.pause_recording`, `hotkey.stop_recording`, and `hotkey.take_screenshot`, so an agent can tell whether a feature was driven by hotkeys or the control plane.

`uptimeMs` in the snapshot is process uptime at snapshot time.

## Where it is served

`metrics_handler` in `apps/desktop/src-tauri/src/cli_server.rs` handles `GET /v1/metrics`. It requires control-plane auth (the `Authorization: Bearer <token>` check from `cli-server.json`) and is gated by the `control-plane-metrics` feature flag, default **enabled** (registry in `crates/capto-core/src/flags.rs`, documented in `docs/feature-flags.md`). When the flag is disabled the endpoint returns `404 notFound` instead of data.

## How to read it

Get the port and token from the control-plane lockfile at `%APPDATA%\Capto\cli-server.json` (the same discovery the CLI uses; the path comes from `dirs::config_dir()` in `crates/capto-ipc/src/lockfile.rs`). Then call the endpoint:

```powershell
$lock = Get-Content "$env:APPDATA\Capto\cli-server.json" | ConvertFrom-Json
curl.exe -s "http://127.0.0.1:$($lock.port)/v1/metrics" -H "Authorization: Bearer $($lock.token)"
```

The response is an envelope containing the snapshot. Keep the token out of logs, issues, and pasted output; `crates/capto-ipc/src/redact.rs` and `docs/PII.md` cover the rule. The endpoint contract and auth are also documented on [Endpoints](../api/endpoints.md).

## How to add a counter

All counters flow through the shared `Metrics` instance on `AppState`:

```rust
// counter
app.state::<AppState>().metrics.incr("my_event");

// duration series
app.state::<AppState>().metrics.observe_ms("my_call_ms", elapsed_ms);

// product-usage event
app.state::<AppState>().metrics.incr_usage("my.feature");
```

From a control-plane handler in `apps/desktop/src-tauri/src/cli_server.rs` you have the `Metrics` in `HttpState` directly (or via `st.metrics`). `Metrics` is cheap to clone (a single `Arc`), so passing it around is cheap. If an event also matters for debugging a crash, record a breadcrumb alongside it (`crates/capto-core/src/breadcrumbs.rs`); see [Crashes](crashes.md).

## No service-level targets

These counters exist to make a local recording or control-plane problem diagnosable, not to feed an SLO dashboard. Values are per-process, aggregate-only (no user identity, no timestamps beyond `uptimeMs`, no screen content), kept in memory, and gone at exit. Use them to compare a before/after change locally — for example whether `record_start_ms` and `http_status_500` moved after an encoder switch — and read `docs/analytics.md` for the intent behind the `usage` counters.
