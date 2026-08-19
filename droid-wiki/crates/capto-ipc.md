# capto-ipc

Active contributors: elwina

## Purpose

`crates/capto-ipc/` is the shared wire contract for the local Capto control plane. It defines the request and response types the desktop HTTP server speaks, the JSON envelope every response is wrapped in, the lockfile the CLI uses to discover the desktop, and the log-redaction helper that keeps `capto_*` logs free of credentials. It has no business logic of its own, only the shapes and constants that the desktop and the CLI both lean on.

Three consumers sit on this crate: the desktop control plane (writes the lockfile, builds every response with `Envelope`), the CLI client (parses envelopes, maps errors to exit codes, reads the lockfile), and agent tooling that drives the desktop over `docs/CLI.md`. Anything one side serializes, another side deserializes.

## Directory layout

| Path | Role |
|------|------|
| `crates/capto-ipc/src/lib.rs` | Module declarations, re-exports, `API_PREFIX` = `/v1` |
| `crates/capto-ipc/src/types.rs` | Request and info types (`RecordStartRequest`, `ShotRequest`, output listing, `DoctorInfo`) |
| `crates/capto-ipc/src/envelope.rs` | `Envelope<T>`, `ApiError`, and the stable `ExitCode` enum |
| `crates/capto-ipc/src/lockfile.rs` | `ServerLock`, write/read/clear helpers, `is_pid_alive`, `LOCK_VERSION` |
| `crates/capto-ipc/src/redact.rs` | Secret scrubbing (`redact`) and `REQUEST_ID_HEADER` |

## Key abstractions

| Abstraction | File | What it is |
|-------------|------|------------|
| `Envelope<T>` | `crates/capto-ipc/src/envelope.rs` | The generic response wrapper `{ ok, data, error }`. Constructors `ok(data)`, `err(code, message)`, `from_result`. `data` and `error` serialize only when present. |
| `ApiError` | `crates/capto-ipc/src/envelope.rs` | `{ code, message }` carried inside the error envelope. |
| `ExitCode` | `crates/capto-ipc/src/envelope.rs` | The stable `i32` codes `0..6`: `Ok`, `Usage`, `DesktopUnavailable`, `StateConflict`, `Capture`, `Encode`, `ConfigIo`. The CLI maps these from HTTP errors. |
| `RecordStartRequest` | `crates/capto-ipc/src/types.rs` | Body for `POST /v1/record/start`; camelCase and mirrors the desktop `StartArgs`. |
| `ShotRequest` | `crates/capto-ipc/src/types.rs` | Body for `POST /v1/shot`; source plus optional display/window/region. |
| `OutputEntry` / `OutputsList` | `crates/capto-ipc/src/types.rs` | One output file (path, name, bytes, modified timestamp) and the full listing with `output_dir`. |
| `OpenOutputsRequest` | `crates/capto-ipc/src/types.rs` | Mutable-folder open request: optional path, open-folder flag, or "open newest". |
| `DoctorInfo` | `crates/capto-ipc/src/types.rs` | `doctor` report: OS, capture backend, FFmpeg presence/health, control-plane pid and port, preferred encoder. |
| `ServerLock` | `crates/capto-ipc/src/lockfile.rs` | `{ pid, port, token, version }` discovery record written by the desktop. |
| `write_server_lock` / `clear_server_lock` / `read_server_lock` | `crates/capto-ipc/src/lockfile.rs` | Lifecycle helpers around the lockfile; `read_server_lock_at` takes an explicit path for tests. |
| `redact` / `REQUEST_ID_HEADER` | `crates/capto-ipc/src/redact.rs` | Scrubbing function and the `x-request-id` HTTP header constant. |

The `LOCK_VERSION` constant lives in `crates/capto-ipc/src/lockfile.rs` (currently `1`) and is stamped into every `ServerLock` so a stale lock from an older desktop is detectable.

## How it works

### Envelope contract

Every control-plane response is a single generic shape. A successful call:

```json
{
  "ok": true,
  "data": { "state": "Recording", "elapsedMs": 1200 }
}
```

A failed call, always with a machine-readable `code` and a human `message`:

```json
{
  "ok": false,
  "error": { "code": "stateConflict", "message": "already recording" }
}
```

`ok` is the discriminator. The CLI reads `envelope.ok` first; if false it maps the `error.code` onto an `ExitCode` (for example an HTTP 409 with code `stateConflict` maps to `ExitCode::StateConflict`). The envelope is defined in `crates/capto-ipc/src/envelope.rs` and used as the return body of every handler in `apps/desktop/src-tauri/src/cli_server.rs`.

### Lockfile lifecycle

Discovery is a small file, not a fixed port. When the desktop starts its control plane, `start_control_plane` in `apps/desktop/src-tauri/src/cli_server.rs` binds `127.0.0.1:0`, generates a fresh v4 UUID as the bearer token, and writes a `ServerLock` (pid, bound port, token, `LOCK_VERSION`) to `%APPDATA%\Capto\cli-server.json` via `write_server_lock`. When the spawned axum server exits, `clear_server_lock` removes it; `shutdown_control_plane` clears it on a graceful shutdown too.

The CLI reverse path is `try_existing` in `crates/capto-cli/src/client.rs`: it calls `read_server_lock`, uses `is_pid_alive` to drop a stale lock (and clears it), then probes `GET /v1/status` with the read token to confirm the desktop is really there. Only after that does it construct a client from the lock's `port` and `token`.

### Redaction approach

`redact` in `crates/capto-ipc/src/redact.rs` scrubs two known patterns from arbitrary text: `Bearer <token>` becomes `Bearer ***`, and the values of a fixed list of query keys (`token`, `api_key`, `secret`, `password`, `auth`, `session`, and others) become `***` when they appear as `key=value` segments in a URL-like string. It is deliberately conservative, matching only those patterns and preserving everything else byte-for-byte so error text stays readable. The desktop routes anything that could carry credentials through it before it reaches `tracing` output, so `capto_*` logs only ever show scrubbed strings. This scrubbing is documented in `docs/PRIVACY.md`. The CLI also wraps user-facing error strings with `redact` before returning them (`crates/capto-cli/src/client.rs`).

### Request-id header

`REQUEST_ID_HEADER` (`x-request-id`) is defined in `crates/capto-ipc/src/redact.rs`. The CLI generates a v4 UUID per request and sends it as that header. On the desktop, the `telemetry_layer` axum middleware in `apps/desktop/src-tauri/src/cli_server.rs` reads or generates it, echoes it back in the response, records it on metrics counters and breadcrumbs, and includes it in a structured log line with only method, path, status, and duration. Bodies, query strings, and auth material never reach the log.

## Integration points

- `apps/desktop/src-tauri/src/cli_server.rs` builds the axum router, wraps every response in `Envelope`, rejects bad auth with an error envelope, writes/clears the lockfile on control-plane start and stop, and runs the request-id telemetry middleware.
- `apps/desktop/src-tauri/src/session_svc.rs` is called by those handlers to turn `RecordStartRequest` and `ShotRequest` into real session work.
- `crates/capto-cli/src/client.rs` parses envelopes, maps error codes to `ExitCode`, reads the lockfile for discovery, sends `x-request-id`, and applies `redact` to returned messages. `crates/capto-cli/src/main.rs` turns the resulting `ExitCode` into the process exit status.
- The IPC request types mirror the desktop's own internal start and shot arguments in `apps/desktop/src-tauri/src/lib.rs`: `RecordStartRequest` and `ShotRequest` in `crates/capto-ipc/src/types.rs` carry the same `source`/`displayId`/`windowId`/`region` intent that the UI sends, and `RecordStartRequest` mirrors `RecordRequest` in `crates/capto-core/src/ffmpeg_args.rs` (shared types come from `crates/capto-core/` and `crates/capto-encode/`).

## Entry points for modification

- Add an endpoint field: extend the matching type in `crates/capto-ipc/src/types.rs`, use it in the handler in `apps/desktop/src-tauri/src/cli_server.rs`, and pass it through `crates/capto-cli/src/client.rs` plus the CLI command. Keep `#[serde(default)]` on any new optional field so older clients and servers stay wire-compatible.
- Change the envelope: since `docs/CLI.md` documents the contract, any change must keep `ok`/`data`/`error` shape and the error `code` strings backwards compatible; old desktops must still be drivable by a newer CLI and vice versa.
- Add a redaction pattern: extend `SECRET_QUERY_KEYS` and add a `mask_*` step in `crates/capto-ipc/src/redact.rs`, with tests alongside it.
- Bump the lockfile: change `LOCK_VERSION` and add a migration path in `read_server_lock` so an old lock cannot be mistaken for a live control plane.

## Key source files

| File | Role |
|------|------|
| `crates/capto-ipc/src/lib.rs` | Re-exports, `API_PREFIX` |
| `crates/capto-ipc/src/types.rs` | Request and info types |
| `crates/capto-ipc/src/envelope.rs` | `Envelope<T>`, `ApiError`, `ExitCode` |
| `crates/capto-ipc/src/lockfile.rs` | `ServerLock`, lock lifecycle, `is_pid_alive`, `LOCK_VERSION` |
| `crates/capto-ipc/src/redact.rs` | `redact` scrubbing and `REQUEST_ID_HEADER` |

`redact` is used beyond the crate: the `capto_*` log scrubbing it powers is documented in `docs/PRIVACY.md`, and the CLI applies it to surfaced error strings in `crates/capto-cli/src/client.rs`.

## Related pages

- [Control-plane API](../api/index.md), the endpoint contract built on these types
- [capto CLI](../apps/cli.md), the client consuming exit codes and the envelope
- [Desktop app](../apps/desktop/index.md), the control-plane server (`cli_server.rs`)
- [capto-core](../crates/capto-core.md), `RecordStartRequest` mirrors `RecordRequest`
- [Security](../security.md), token storage and redaction rationale
