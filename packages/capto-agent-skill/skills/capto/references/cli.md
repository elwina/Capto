# Capto CLI contract

## Model

```
Agent → capto (CLI) → 127.0.0.1 + Bearer token → Capto desktop (single process)
```

Lockfile: `%APPDATA%\Capto\cli-server.json` (`pid`, `port`, `token`, `version`).

## Envelope

```json
{ "ok": true, "data": { } }
{ "ok": false, "error": { "code": "desktopUnavailable", "message": "…" } }
```

`data` fields are camelCase.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | ok |
| 1 | usage |
| 2 | desktopUnavailable |
| 3 | stateConflict |
| 4 | capture |
| 5 | encode |
| 6 | configIo |

## Commands

| Command | Purpose |
|---------|---------|
| `doctor` | FFmpeg / backend / control plane |
| `status` | Session snapshot |
| `list displays\|windows\|audio\|encoders` | Discovery |
| `config get [key]` / `set` / `path` | Settings |
| `shot` | Screenshot → `data.path` |
| `record start\|stop\|pause\|resume` | Recording |
| `outputs recent\|open` | Recent files / open |

### `record start` flags

`--source display|window|region`, `--display`, `--window`, `--x/--y/--width/--height`, `--format mp4|gif|audio`, `--fps`, `--encoder`, `--mic`, `--loopback`, `--no-cursor`.

Always `record stop` when finished (no CLI duration auto-stop).

## Repo / cargo

```bash
cargo run -p capto-cli -- status          # runs binary named `capto`
cargo build -p capto-app                  # desktop executable
```

Desktop package is `capto-app` so it does not overwrite the CLI `capto.exe` in `target/debug`.
