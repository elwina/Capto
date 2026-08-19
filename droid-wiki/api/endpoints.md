# Endpoints

Active contributors: elwina

The full reference for the localhost control plane, one route per section. All requests go to `http://127.0.0.1:<port>` with the `Authorization: Bearer <token>` header from the lockfile, and every response is an `Envelope`. Types come from `crates/capto-ipc/src/types.rs`, `crates/capto-core/src/session.rs`, and `crates/capto-core/src/settings.rs`. The router and handlers are defined in `apps/desktop/src-tauri/src/cli_server.rs`.

## Status

- Method/path: `GET /v1/status`
- Request: none
- Response data: `SessionSnapshot` from `crates/capto-core/src/session.rs` (`{ state, elapsedMs, outputPath, lastError, encoder, hideApp }`)
- Errors: no application-level errors; unauthenticated `unauthorized` → 401

```json
{ "ok": true, "data": {
  "state": "Idle",
  "elapsedMs": 0,
  "outputPath": null,
  "lastError": null,
  "encoder": null,
  "hideApp": false
} }
```

`state` is one of `Idle`, `Starting`, `Recording`, `Paused`, `Stopping`.

## Doctor

- Method/path: `GET /v1/doctor`
- Request: none
- Response data: `DoctorInfo` (OS, capture backend, FFmpeg path/presence, control-plane status, pid, port, preferred encoder) from `crates/capto-ipc/src/types.rs`
- Errors: `unauthorized` → 401

```json
{ "ok": true, "data": {
  "os": "windows",
  "captureBackend": "dxgi",
  "ffmpegPath": "C:\\Capto\\ffmpeg.exe",
  "ffmpegOk": true,
  "controlPlane": true,
  "pid": 4812,
  "port": 51023,
  "preferredEncoder": "h264_nvenc"
} }
```

`ffmpegOk` is a real `-version` probe, not just file presence, so a wedged process still reports `false` (`session_svc::doctor`).

## Config

### `GET /v1/config`

- Request: none
- Response data: full `AppSettings` from `crates/capto-core/src/settings.rs` (camelCase)
- Errors: `unauthorized` → 401

### `PATCH /v1/config`

- Method/path: `PATCH /v1/config`
- Request: a JSON object of partial settings; only the supplied keys are merged
- Response data: the merged full `AppSettings`
- Errors: `configIo` → 404 on filesystem failure; `unauthorized` → 401
- Side effect: hotkey re-registration. If the patch changes `hotkeys`, `patch_settings` in `apps/desktop/src-tauri/src/session_svc.rs` calls `register_hotkeys` and stores any conflict results in app state. It also emits a `settings://changed` event to the UI.

```json
{ "ok": true, "data": { "fps": 60, "quality": 80 } }
```

### `GET /v1/config/path`

- Method/path: `GET /v1/config/path`
- Request: none
- Response data: `ConfigPathInfo` (`{ path }`) pointing at `settings.json`
- Errors: `unauthorized` → 401

## Record

### `POST /v1/record/start`

- Request: `RecordStartRequest` from `crates/capto-ipc/src/types.rs`, camelCase, with all fields optional (`#[serde(default)]`) except `source`
- Response data: `SessionSnapshot`
- Errors: classified by the `record_start` handler in `apps/desktop/src-tauri/src/cli_server.rs`:
  - `stateConflict` (409) when the message contains `invalid state` or `already`, for example starting while already recording
  - `encode` (500) when the message contains `ffmpeg` or `encode`
  - `capture` (500) for everything else

```json
{
  "source": "display",
  "displayId": 0,
  "includeCursor": true,
  "format": "mp4",
  "fps": 60,
  "quality": 80
}
```

### `POST /v1/record/stop`

- Request: none
- Response data: `SessionSnapshot` (returns to `Idle`, with the final `outputPath`)
- Errors: `stateConflict` (409) when nothing is recording

### `POST /v1/record/pause`

- Request: none
- Response data: `SessionSnapshot` with `state: "Paused"`
- Errors: `stateConflict` (409) when not recording or already paused

### `POST /v1/record/resume`

- Request: none
- Response data: `SessionSnapshot` with `state: "Recording"`
- Errors: `stateConflict` (409) when not paused

## Shot

### `POST /v1/shot`

- Request: `ShotRequest` (`source` plus optional `displayId`/`windowId`/`region`)
- Response data: `{ "path": "<absolute png path>" }`
- Errors: `capture` (500) on capture or region failure; window-gone returns `capture`

```json
{ "ok": true, "data": { "path": "C:\\Users\\x\\Videos\\Capto\\capto-shot-20260812-104523-a1b2c3d4.png" } }
```

## List

All four listing endpoints are `GET`, return raw vendor JSON in `data`, and require no body.

| Endpoint | Returns | Error code |
|----------|---------|------------|
| `GET /v1/list/displays` | capture displays | `capture` |
| `GET /v1/list/windows` | capture windows | `capture` |
| `GET /v1/list/audio` | mic + loopback devices | `capture` |
| `GET /v1/list/encoders` | FFmpeg probe result | `encode` |

`/v1/list/encoders` calls `refresh_encoder` and probes the bundled FFmpeg, so it reports `encode` when FFmpeg is missing.

## Outputs

### `GET /v1/outputs/recent`

- Query: optional `limit` (positive integer; default 20 in the handler, capped at a minimum of 1)
- Request body: none
- Response data: `OutputsList` (`{ outputDir, items: [ { path, name, bytes, modifiedMs } ] }`), newest first, filtered to `capto-*`/`capto_*` files plus common media extensions
- Errors: `configIo` (500) on directory read failure

```json
{ "ok": true, "data": {
  "outputDir": "C:\\Users\\x\\Videos\\Capto",
  "items": [
    { "path": "C:\\Users\\x\\Videos\\Capto\\capto-20260812...mp4",
      "name": "capto-20260812....mp4",
      "bytes": 2143548,
      "modifiedMs": 1755012345678 }
  ]
} }
```

### `POST /v1/outputs/open`

- Request: `OpenOutputsRequest` (`{ path?, folder?, last? }`) from `crates/capto-ipc/src/types.rs`
- Response data: `{ opened, kind }` where `kind` is `folder` or `file`
- Errors: `notFound` (404) when a path does not exist or no output is found
- Behaviors (`app_services::open_outputs` in `session_svc.rs`):
  - `folder: true` → opens (and creates if needed) the output directory
  - `path` set → opens that file
  - `last: true` → resolves the most recent output via `outputs_recent(limit 1)`
  - none of path/folder/last → error

```json
{ "request": { "last": true, "folder": false },
  "response": { "ok": true, "data": { "opened": "C:\\...\\capto.mp4", "kind": "file" } } }
```

## Metrics

### `GET /v1/metrics`

- Request: none
- Response data: a `Metrics` snapshot (request counters, duration histogram, per-status counters)
- Errors: `notFound` (404) when the `CONTROL_PLANE_METRICS` feature flag is disabled; `unauthorized` → 401

Metrics are recorded by the `telemetry_layer` middleware on every control-plane call (`apps/desktop/src-tauri/src/cli_server.rs`).

## Common errors

Every handler returns `unauthorized` (HTTP 401) with code `unauthorized` when the bearer token is missing or wrong. Route dispatch failures and unknown paths return HTTP 404. The server converts internal codes to HTTP status in `map_err` (`apps/desktop/src-tauri/src/cli_server.rs`).

## CLI command mapping

Every CLI command maps 1:1 to an endpoint (`crates/capto-cli/src/main.rs`), so the endpoint list and the CLI surface stay in lockstep.

| CLI command | Endpoint |
|-------------|----------|
| `capto status` | `GET /v1/status` |
| `capto doctor` | `GET /v1/doctor` |
| `capto config get` | `GET /v1/config` |
| `capto config set` | `PATCH /v1/config` |
| `capto config path` | `GET /v1/config/path` |
| `capto record start` | `POST /v1/record/start` |
| `capto record stop` | `POST /v1/record/stop` |
| `capto record pause` | `POST /v1/record/pause` |
| `capto record resume` | `POST /v1/record/resume` |
| `capto shot` | `POST /v1/shot` |
| `capto list displays` | `GET /v1/list/displays` |
| `capto list windows` | `GET /v1/list/windows` |
| `capto list audio` | `GET /v1/list/audio` |
| `capto list encoders` | `GET /v1/list/encoders` |
| `capto outputs recent` | `GET /v1/outputs/recent?limit=N` |
| `capto outputs open` | `POST /v1/outputs/open` |
| `capto open` | no HTTP, launches the desktop only |

See [capto CLI](../apps/cli.md) for the exit-code table and auto-launch behavior.

## Key source files

| File | Role |
|------|------|
| `apps/desktop/src-tauri/src/cli_server.rs` | Router, handlers, auth, error classification, telemetry |
| `apps/desktop/src-tauri/src/session_svc.rs` | Shared implementation behind every handler |
| `crates/capto-ipc/src/types.rs` | Request and info types (`RecordStartRequest`, `ShotRequest`, `OutputsList`, ...) |
| `crates/capto-ipc/src/envelope.rs` | `Envelope` and error serialization |
| `crates/capto-core/src/session.rs` | `SessionSnapshot` |
| `crates/capto-core/src/settings.rs` | `AppSettings` |
| `crates/capto-cli/src/main.rs` | CLI command surface and exit-code mapping |

## Related pages

- [Control-plane API](../api/index.md), envelope, auth, versioning, mental model
- [capto CLI](../apps/cli.md), client commands, exit codes, discovery
- [capto-ipc](../crates/capto-ipc.md), envelope, lockfile, shared types
- [capto-core](../crates/capto-core.md), `SessionSnapshot` / `AppSettings`
- [Security](../security.md), token handling and redaction
- [Recording](../features/recording.md), record workflow semantics
