# Crashes

When the desktop process panics, Capto writes a structured crash report instead of uploading anything. The report is local, contextual, and deterministic enough to turn a one-line panic into a concrete code path. This page covers how reports are produced, what is in them, and how to collect one for a bug report.

## Panic hook and report location

At startup, `apps/desktop/src-tauri/src/lib.rs` calls `crashlog::install_panic_hook()` before anything else runs. That hook, in `apps/desktop/src-tauri/src/crashlog.rs`, installs a `std::panic::set_hook` that writes a `crash-<epoch-ms>.json` report and also prints the panic subject to stderr for console users.

`crash_dir()` computes the destination folder. Note a discrepancy worth flagging:

- **What the code does**: `crash_dir()` returns `AppSettings::config_path().parent().join("crashes")`, which on Windows resolves to `%APPDATA%\crashes`, outside the `Capto` config folder, because `config_path()` already returns `%APPDATA%\Capto` and the `.parent()` strips it.
- **What the comments and docs claim**: the doc comment in `crashlog.rs`, `docs/crash-tracing.md`, and `docs/PII.md` all say reports live at `<config>/Capto/crashes`.

The safest way to find the report regardless of which holds true on your install is to search from `%APPDATA%`:

```powershell
Get-ChildItem "$env:APPDATA" -Recurse -Filter "crash-*.json" -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -Last 1
```

If you touch this code, reconcile `crash_dir()` with its documentation, because the two currently disagree about where the `Capto` segment goes.

## What a report contains

The `CrashReport` struct (serialized camelCase) carries:

| Field | Meaning |
|-------|---------|
| `subject` | The `PanicInfo` text, the message and usually the panic site. |
| `panicLocation` | Exact `file:line:col` of the panic, when available. The fastest anchor to the throwing line. |
| `backtrace` | Full captured stack. `RUST_BACKTRACE=1` (or `=full`) enriches symbols. |
| `pid`, `uptimeMs` | Process id and uptime at crash time. |
| `featureFlags` | Feature flags active at crash time, from `settings.json`. |
| `lastRequestId` | Most recent control-plane `x-request-id`, for matching against logs. |
| `breadcrumbs` | The capped, scrubbed event trail leading up to the panic. |
| `app`, `version`, `os`, `timestampMs` | Identity and timestamp of the report. |

Writing is best-effort and never blocks the panic handler: if the file cannot be written, the hook logs an error rather than stalling the process.

## Breadcrumbs

The trail is a fixed-capacity ring buffer in `crates/capto-core/src/breadcrumbs.rs` (`DEFAULT_CAPACITY = 64`). Events carry a `category`, a scrubbed `message`, an optional `request_id`, a relative `rel_ms`, and a wall-clock `at_ms`. The categories in use are `lifecycle`, `control-plane`, `session`, and `hotkey`.

- `record(category, message)` records an event with no request id, for example `record("lifecycle", "app_started")` and the hotkey lines in `apps/desktop/src-tauri/src/lib.rs`.
- `record_with_request(category, message, request_id)` tags a control-plane event with its `x-request-id`; the `telemetry_layer` in `apps/desktop/src-tauri/src/cli_server.rs` uses it and records method / path / status only.
- `try_current_context()` / `breadcrumbs::try_snapshot()` produce the `CrashContext` the panic hook embeds. They use a non-blocking lock (`try_lock`), so if another thread holds the lock at panic time the report simply omits the trail instead of deadlocking. That is intentional.

Only scrubbed descriptions are ever recorded, method / path / status and action names, never request bodies, query strings, or the bearer token. `crates/capto-ipc/src/redact.rs` masks anything that slips in.

## Collecting a report for a bug report

1. **Find the newest report** with the discovery command above, or open `docs/crash-tracing.md`, which gives the PowerShell `Get-ChildItem` pipelines and the full walkthrough for reverting a PanicInfo to code.
2. **Read `panicLocation` and `subject`** first. If the subject names a crate and line, that is almost certainly the throw site; confirm with the top of `backtrace`.
3. **Read the breadcrumbs from the end.** The most recent events are the ones that likely triggered the panic. A trail ending `record start ok → hotkey take_screenshot pressed` with a screenshot/encoding panic location points at a reproducible race you can turn into a regression test.
4. **Correlate with logs.** Match `lastRequestId` (and each breadcrumb `requestId`) against the `tracing` lines emitted by the `telemetry_layer`, per [Logging](logging.md). Run with `RUST_BACKTRACE=full` if symbols are missing.
5. **Reproduce** the breadcrumb sequence on a dev build (`npm run tauri --prefix apps/desktop -- dev`) and attach the steps, the crash file, and the version to the issue.

## Capturing and disabling crash reports

The `crash-reporting` feature flag (default **enabled**) gates report generation. Disable it like any flag:

```powershell
capto config set --json '{"disabledFlags":["crash-reporting"]}'
```

The flag is declared in `crates/capto-core/src/flags.rs` and covered in `docs/feature-flags.md`. To remove all existing reports, delete the `crashes` folder.

## Related diagnostics

A bad recording does not always crash. If it pauses, freezes, or produces a short file, capture is more often outrunning the encoder than panicking, look at the desktop stderr for `slow ffmpeg write - capture outrunning encoder` and `frame pump finished` tail lines from `crates/capto-core/src/session.rs` (`CAPTO_LOG=capto_core=debug`, per [Logging](logging.md)), and see [Metrics](metrics.md) for the duration counters. Crash reporting also feeds the canary validation loop in [Updates](../features/updates.md), and the whole local-first design is explained under [Privacy and security](../security.md).
