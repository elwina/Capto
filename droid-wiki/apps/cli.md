# CLI

Active contributors: elwina

The `capto` CLI (crate `capto-cli`) is the control-plane client for the Capto desktop app. It is a thin, agent-friendly wrapper that sends HTTP requests to the running desktop over `127.0.0.1` and prints JSON. It never records by itself: it drives the single desktop `RecordingSession`, so one machine always has at most one live capture. The canonical reference is `docs/CLI.md`.

## Purpose

Agents and scripts need a stable, parseable way to control a screen recorder. The CLI provides one: it reads the desktop's connection details from the lockfile, authenticates with a Bearer token, and returns a JSON envelope on stdout with predictable exit codes. It also auto-launches the desktop when the control plane is not running, so a bare `capto status` mostly just works.

## Command surface

Commands are defined in `crates/capto-cli/src/main.rs` (a clap `Command`/`Subcommand`). Two global flags apply to most commands:

- `--human`, print readable data instead of the JSON envelope
- `--no-launch`, fail if the desktop control plane is down (don't auto-start it)

| Command | Notes |
|---------|-------|
| `open` | Start the desktop only; does not wait for the control plane |
| `status` | Session snapshot (`idle`/`starting`/`recording`/`paused`/`stopping`) |
| `doctor` | Environment / FFmpeg / control-plane readiness |
| `record start\|stop\|pause\|resume` | Recording controls |
| `shot` | Screenshot; returns `data.path` |
| `config get\|set\|path` | Read / patch settings |
| `list displays\|windows\|audio\|encoders` | Enumerate devices |
| `outputs recent\|open` | Recent files / open in Explorer |

`record start` and `shot` share source-selection flags: `--source display|window|region`, plus `--display`, `--window`, and `--x/--y/--width/--height` for a region. `record start` also takes `--format mp4|gif|audio`, `--fps`, `--quality`, `--encoder`, `--mic`, `--loopback`, and `--no-cursor`. `config set` accepts either `key=value` pairs (camelCase keys, e.g. `fps=60`) or `--json '{...}'`.

## JSON envelope and exit codes

On success the CLI prints `{ "ok": true, "data": ... }`; on failure `{ "ok": false, "error": { "code", "message" } }` (see `emit_ok` and the `main` error path in `crates/capto-cli/src/main.rs`). With `--human` it prints only the data or a plain message. `data` fields and settings keys are camelCase.

Exit codes are the stable part of the contract; branch on the code first, then `error.code`:

| Code | Name | When |
|------|------|------|
| 0 | ok | Success |
| 1 | usage | Bad args / unknown settings key |
| 2 | desktopUnavailable | No control plane / auth / launch failed |
| 3 | stateConflict | e.g. start while already recording |
| 4 | capture | Capture / device failure |
| 5 | encode | FFmpeg / encoder failure |
| 6 | configIo | Settings / outputs filesystem error |

## Auto-launch and discovery

The CLI finds the desktop before connecting (`crates/capto-cli/src/client.rs::connect`). It first tries an existing control plane; if the plane is down it spawns the desktop (unless `--no-launch`) and polls for readiness.

The lookup order for the desktop executable lives in `crates/capto-cli/src/launch.rs::find_capto_exe`:

1. `CAPTO_APP_PATH` env var (must point at `capto-app.exe`, not the CLI; a bare `\\?\` verbatim prefix is stripped)
2. `capto-app.exe` or `Capto.exe` next to the CLI binary and one level up (covers the `<install>\cli\` layout)
3. `target/debug|release/capto-app.exe` relative to the crate manifest, and the Tauri target directories
4. `%LOCALAPPDATA%\Capto\` and `Program Files\Capto\`

It deliberately skips any candidate that resolves to the CLI's own binary, so it never re-opens `capto.exe`. Launch uses `ShellExecuteW` on Windows so the desktop does not inherit the CLI's redirected stdout/stderr, which matters when agents capture JSON through pipes.

## Resilience and retries

The CLI is defensive about the control-plane channel flapping while the desktop starts or quits. `crates/capto-cli/src/resilience.rs` implements a circuit breaker: it opens after the first 3 consecutive failures, fails fast while open, and half-opens after a 5-second cooldown to probe again. `client.rs` retries with exponential backoff (250ms, 500ms, ...) but only for idempotent `GET` requests, retrying a mutating `POST` that already reached the server could double-record.

## Error mapping

`crates/capto-cli/src/main.rs::map_http` converts the error `code` from the server into the exit-code table above: `unauthorized`/`desktopUnavailable` → 2, `stateConflict` → 3, `capture` → 4, `encode` → 5, `configIo` → 6, `badRequest`/`usage` → 1, and anything else → 2. The lockfile, envelope, and request types it uses come from [capto-ipc](../crates/capto-ipc.md) (`crates/capto-ipc/src/types.rs`).

## Agent workflow loop

Agents follow a fixed round trip:

```text
1. doctor                      # exit 2 → open desktop / ask user; ffmpegOk must be true
2. list displays               # optional
3. record start --source display
4. status                      # poll
5. record stop
6. outputs recent --limit 1
```

The screenshots workflow is `status`/`open` → `shot --source display` → use `data.path`. Agents should check `status` before starting, never `record start` twice, always `record stop` when done, and prefer `--no-launch` in headless CI.

## Npm packages wrap the CLI

The CLI is the backbone of both agent npm packages, see [capto-agent-skill](../packages/capto-agent-skill.md) and [capto-dsh-plugin](../packages/capto-dsh-plugin.md). The skill ships `skills/capto/SKILL.md` plus `references/cli.md` (kept in sync with `docs/CLI.md`) and tells an agent to call `capto <command>`; the DeepSeek Harness plugin registers 14 typed `capto_*` tools that each invoke the CLI and normalize its output and exit codes. Both wrap the same binary rather than reimplementing anything.
