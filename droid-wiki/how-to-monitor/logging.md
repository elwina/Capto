# Logging

Capto logs with `tracing` (`tracing-subscriber` with an `EnvFilter`). There are no log files on disk and no log aggregator: output goes to the standard streams of the process that emits it. The desktop and the CLI each initialize their own subscriber with a different default verbosity, and each can be tuned independently.

## Where logs go

- **Desktop app** (`capto-app`). Logging is initialized in `init_tracing()` in `apps/desktop/src-tauri/src/lib.rs`. It uses `tracing_subscriber::fmt()` with `with_target(true)`, writing to the process's standard output. When you run `npm run tauri --prefix apps/desktop -- dev`, these lines appear in the terminal that owns the process. Because the desktop owns the frame pump, the encoder, and the control plane, this is the process that produces the diagnostics that matter for recording.
- **CLI** (`capto`, crate `capto-cli`). Logging is initialized in `crates/capto-cli/src/main.rs` using `tracing_subscriber::fmt()` with `with_writer(std::io::stderr)`, so CLI diagnostics go to stderr and never corrupt the JSON envelope written to stdout. The CLI is a control-plane client, so its own logging is minimal; most of what it reports comes back as structured stdout data.

The practical consequence: to inspect a recording problem you watch the desktop process's output (not a file), and to inspect a CLI run you watch stderr. There is no `logs/` directory to tail.

## Filter strings

Each binary reads its filter from the environment, with a default fallback:

- **Desktop**: `CAPTO_LOG`, default `capto=debug,capto_core=debug,capto_encode=debug,warn`. This turns on debug for the three `capto_*` modules that matter most and leaves everything else at `warn`. Set it before launching `capto-app`, for example `$env:CAPTO_LOG="capto_core=debug,capto=warn"`.
- **CLI**: standard `RUST_LOG` via `EnvFilter::try_from_default_env`, default `warn`. Because the CLI does not itself capture or encode, raising its level rarely adds recording diagnostics; it mostly shows control-plane client chatter.

Use the filter on whichever process owns the work you are watching. The frame pump belongs to the desktop, so slow-write lines only appear when `CAPTO_LOG` is set on the desktop process.

## Adding a log line

Log lines use `tracing` macros with the crate's `capto_*` target, which is what the filter selects on. Prefer structured fields over string interpolation so lines stay greppable and consistent:

```rust
tracing::info!(request_id, method, path, status = status.as_u16(), duration_ms, "control plane request");
tracing::warn!(shortcut = %binding.shortcut, ?error, "skipping unavailable hotkey");
```

The `telemetry_layer` in `apps/desktop/src-tauri/src/cli_server.rs` is a good reference: it logs a single scrubbed line carrying method, path, status, duration, and the `x-request-id` (`tracing::info!` in that function), and records the same shape into the breadcrumb trail. Match that style for new lines.

## Redaction requirement

Logs must never contain secrets. Two guards keep this true:

- **At write time**, `capto_ipc::redact` in `crates/capto-ipc/src/redact.rs` masks `Bearer <token>` values and token-like query keys (`token`, `api_key`, `secret`, `password`, and the others listed in `SECRET_QUERY_KEYS`) before strings reach `tracing`. The `telemetry_layer` records only method / path / status — never request bodies, query strings, or auth headers.
- **In the repo**, `scripts/scan-pii.ps1` scans the source and fails the hygiene job if PII leaks in; `docs/PII.md` states the policy. The Bearer token from `cli-server.json` must never be logged anywhere, and it must never be pasted into an issue or this wiki.

If you add a log line that could carry a URL, path, or command line that embeds credentials, run it through `capto_ipc::redact` (or ensure the callers already scrub) and test the line against `docs/PII.md`.

## Enabling pump diagnostics

The recording hot path lives in `crates/capto-core/src/session.rs` (`attach_dxgi_pump`). At debug level it emits two timing lines:

- `slow ffmpeg write - capture outrunning encoder` when a single rawvideo write blocks at least 250 ms (`write_ms >= 250`). This is the classic cause of dropped or frozen output: capture is producing frames faster than the encoder drains them.
- `frame pump finished` at pump teardown with the count of frames written, slow writes, and total loop duration.

Enable them by launching the desktop with a debug filter on the `capto_core` target:

```powershell
$env:CAPTO_LOG = "capto_core=debug,capto=warn"
.\target\debug\capto-app.exe
```

Then drive a recording (CLI or UI), stop it, and inspect the desktop's output for those lines. `RUST_LOG` alone does not enable them, because it governs the CLI, which does not run the pump.

## Correlating with request ids

Each control-plane request carries an `x-request-id` (the `REQUEST_ID_HEADER` constant in `crates/capto-ipc/src/redact.rs`). The `telemetry_layer` generates one if the caller did not, logs it as a structured field, and echoes it back in the response header. The same id appears as `requestId` on correlating breadcrumbs, so a single id ties a log line to a crash-report trail entry. See [Crashes](crashes.md) for the cross-reference workflow, and [Metrics](metrics.md) for the counters alongside these log lines.
