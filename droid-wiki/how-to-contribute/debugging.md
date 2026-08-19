# Debugging

Capto is privacy-first, so almost all debugging is local: structured logs, contextual crash reports written to disk, a loopback metrics endpoint, and an ETW sampling path. There is no cloud telemetry to query. This page collects the techniques and the common failure modes, from a silent CLI exit to a recording that drops frames.

## The purpose of this page

This page is the first stop when something does not work: how to read logs, how to turn a crash report back into a code path, how to recover when the control plane is unreachable, and how to approach recorder failures. For the deeper observability details (metrics, breadcrumbs, crash schema) see [How to monitor](../how-to-monitor/index.md).

## Logs

Logging uses `tracing` with a `capto_*` target convention. Two environment variables control output:

- `CAPTO_LOG` governs the desktop process. Its default filter is `capto=debug,capto_core=debug,capto_encode=debug,warn` (see `init_tracing` in `apps/desktop/src-tauri/src/lib.rs`). Raise a crate's level to get more detail, for example `capto_core=debug,capto=warn`.
- `RUST_LOG` governs the CLI. It is `warn` by default.

The CLI writes the JSON envelope to stdout and human-readable log lines to stderr, so when a `capto` command misbehaves, capture stderr rather than stdout. The full env-var list is in `.env.example`.

### Redaction

Logs and error strings pass through `crates/capto-ipc/src/redact.rs`, which masks known secret patterns by construction. It rewrites `Bearer <token>` to `Bearer ***` and replaces the value of secret-looking query parameters (`token`, `api_key`, `password`, and the others listed in the file's `SECRET_QUERY_KEYS`). Two rules follow from this:

- Never log the bearer token yourself. It is read from `%APPDATA%\Capto\cli-server.json`, and the whole point of `redact` is that it never appears in tracing output.
- Treat redaction as best-effort masking of known patterns, not full PII redaction. If you add a new endpoint or log line, route text through `capto_ipc::redact`.

## Crash reports

When the desktop process panics, the panic hook at `apps/desktop/src-tauri/src/crashlog.rs` writes a contextual report to `<config>/Capto/crashes/crash-<epoch-ms>.json`. The most useful fields for debugging are:

- `panicLocation`, the exact `file:line:col` of the panic, which is the fastest anchor to the code.
- `subject`, the panic message, and `backtrace`, the captured stack (`RUST_BACKTRACE=1` enriches symbols).
- `breadcrumbs`, a capped, in-memory trail of the events that preceded the panic (control-plane requests, session transitions, lifecycle and hotkey markers), held in `crates/capto-core/src/breadcrumbs.rs`.
- `lastRequestId`, the most recent control-plane `x-request-id`, for correlating the report with log output.

Read a crash from the end: the most recent breadcrumbs are the likely trigger. To reproduce, replay the breadcrumb sequence on a dev build and add a regression test once the trigger is isolated. The full walkthrough and report schema are in `docs/crash-tracing.md`. Reports are local only; delete `<config>/Capto/crashes` to remove them.

## Control plane trouble

Most CLI friction comes from the control plane, which is a loopback HTTP server owned by the desktop process.

- **Exit code 2 (`desktopUnavailable`).** The desktop is not running. Start it first: run `capto open`, or launch Capto from the Start menu, or set `CAPTO_APP_PATH` to a built `capto-app.exe` so the CLI auto-launches the dev desktop. Merge requests that return 404 with a fresh token usually mean the lockfile is stale (see gotchas).
- **Lockfile.** The desktop writes `%APPDATA%\Capto\cli-server.json` with the port and bearer token. If you hand-edit or copy it, the CLI's token may no longer match and every authenticated call returns 401. Delete the stale lockfile and restart the desktop.
- **One-session rule.** Only one desktop instance writes the lockfile. A second `capto-app` instance does not take over the control plane and can confuse which process owns the server; run a single instance when debugging.

## Recorder failures

Recording and encoding go through a single sidecar path (`capto-encode` and the frame pump in `crates/capto-core/src/session.rs`). A few signals help when output is missing or frozen:

- FFmpeg's stderr tail is captured into error messages, so a failed encode includes the relevant sidecar output.
- Slow-write instrumentation. The frame pump logs debug-level timing lines. A `slow ffmpeg write - capture outrunning encoder` line means a single rawvideo write blocked 250 ms or more, the classic cause of dropped or frozen output. Set `CAPTO_LOG="capto_core=debug,capto=warn"` on the desktop process, drive a recording, then inspect the desktop's stderr. This and the ETW path are documented in `docs/profiling.md`.
- `/v1/metrics` exposes local usage counters, including per-status request counts and `http_request_duration_ms`, behind authentication and the `control-plane-metrics` feature flag.

## Common gotchas

- **Missing FFmpeg sidecar.** The encode path needs `apps/desktop/src-tauri/binaries/ffmpeg-<triple>.exe` (bundled via Tauri `externalBin`). If `capto doctor` reports FFmpeg missing, re-run `.\scripts\download-ffmpeg.ps1` (or `copy-ffmpeg.ps1` for a local build) before building. The sidecar comes only from the pinned `elwina/capto-ffmpeg` release, never from PATH.
- **Stale `cli-server.json`.** Leftover from a previous run can point at a dead port and return mismatched-token 401s; clear it and restart the desktop.
- **Second desktop instance.** Keep the single-instance rule in mind; it owns the control plane and the lockfile.

## Related pages

- [How to contribute](index.md)
- [Development workflow](development-workflow.md)
- [Testing](testing.md)
- [How to monitor](../how-to-monitor/index.md)
