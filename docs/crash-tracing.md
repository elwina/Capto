# Tracing a Capto crash back to code

Capto never uploads telemetry. When the desktop process panics, it writes a
**contextual local crash report** to:

```
<config>/Capto/crashes/crash-<epoch-ms>.json
```

(`<config>` is the folder that also holds `settings.json`; on Windows
`%APPDATA%\com.elwina.capto` or similar — see `capto-core::settings::config_path`.)

This document explains the report schema and the exact steps an agent (
or maintainer) follows to turn a report into a concrete code path. It pairs
with `apps/desktop/src-tauri/src/crashlog.rs` (reporter) and
`crates/capto-core/src/breadcrumbs.rs` (local breadcrumb trail).

## Report schema

```json
{
  "app": "Capto",
  "version": "1.0.0",
  "os": "windows",
  "timestampMs": 1750000000000,
  "subject": "called `Option::unwrap()` on a `None` value at ...",
  "backtrace": "  0: std::backtrace::Backtrace::create\n  1: ...\n",
  "panicLocation": "crates/capto-encode/src/lib.rs:314:26",
  "pid": 42013,
  "uptimeMs": 81234,
  "featureFlags": ["control-plane-metrics", "crash-reporting"],
  "lastRequestId": "4f0a...# req-...",
  "breadcrumbs": [
    { "category": "lifecycle",      "message": "app_started", "relMs": 8,    "atMs": 1749912345678 },
    { "category": "lifecycle",      "message": "control plane started on 127.0.0.1:44123", "relMs": 120, "atMs": 1749912345790 },
    { "category": "control-plane",  "message": "GET /v1/status -> 200", "requestId": "req-7f2c", "relMs": 9000,  "atMs": 1749912354678 },
    { "category": "session",        "message": "record start ok", "relMs": 9200, "atMs": 1749912354878 },
    { "category": "hotkey",         "message": "take_screenshot pressed", "relMs": 81100, "atMs": 1749912426778 }
  ]
}
```

| Field | Meaning |
|-------|---------|
| `subject` | `PanicInfo` text — the message and (usually) the panic site. |
| `panicLocation` | Exact `file:line:col` of the `panic!`/unwrap call when available. This is the fastest anchor to the code. |
| `backtrace` | Full captured stack. `RUST_BACKTRACE=1`/`=full` enriches symbols. |
| `pid`, `uptimeMs` | Process id and uptime at crash time. |
| `featureFlags` | Feature flags active at crash time (from `settings.json`). |
| `lastRequestId` | Most recent control-plane `x-request-id`, for correlating with `RUST_LOG` output. |
| `breadcrumbs` | Capped, scrubbed trail (oldest→newest) of the events that preceded the panic. |

## Localization workflow

1. **Open the newest report** in the `crashes/` folder (`.` for current dir,
   PowerShell: `Get-ChildItem $env:APPDATA\*\crashes\crash-*.json | Sort-Object Name | Select-Object -Last 1`).
2. **Jump to the panic site** via `panicLocation` and `subject`. If the
   `subject` names a crate and line, open that file — that is the throw site
   (`Backtrace` top frames will confirm it).
3. **Read the breadcrumbs from the end**. The most recent events are the ones
   that likely triggered the panic. Example: if the trail ends
   `record start ok → hotkey take_screenshot pressed` and the panic location
   is in the screenshot/encoding path, the trigger was a screenshot raced
   against an active recording — a reproducible input to a regression test.
4. **Correlate with logs** if `RUST_LOG`/`CAPTO_LOG` was set: match
   `lastRequestId` (and each breadcrumb `requestId`/`atMs`) against the
   control-plane `tracing` lines logged by `cli_server::telemetry_layer`.
5. **Reproduce** the breadcrumb sequence on a dev build
   (`npm run tauri --prefix apps/desktop -- dev`) and add a regression test
   once the trigger is isolated.

## Why this is safe / privacy-first

- The trail is capped (`capto_core::breadcrumbs::DEFAULT_CAPACITY = 64` events)
  and lives in memory only until a crash is written.
- Only scrubbed descriptions are recorded: method / path / status and action
  names. Request bodies, query strings, auth headers, and the bearer token
  never enter the trail (`cli_server` records nothing past `method path -> status`).
- The panic hook never blocks: breadcrumb reads use `try_snapshot`, so a
  contended lock yields a report without the trail rather than a deadlocked
  process.
- Delete `<config>/Capto/crashes` to remove all crash reports.

## Flagging

`crash-reporting` (default **enabled**) gates writing these reports. Disable in
settings (`capto config set --json '{"disabledFlags":["crash-reporting"]}'`) to
stop crash report generation entirely.
