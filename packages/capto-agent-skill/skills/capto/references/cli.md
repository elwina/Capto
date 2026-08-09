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

`data` fields are camelCase. Prefer exit code, then `error.code`.

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
| `status` | Session snapshot (`idle` / `recording` / `paused` / …) |
| `list displays\|windows\|audio\|encoders` | Discovery |
| `config get [key]` / `set` / `path` | Settings (camelCase keys) |
| `shot` | Screenshot → `data.path` |
| `record start\|stop\|pause\|resume` | Recording |
| `outputs recent\|open` | Recent files / open folder |

### `record start` flags

`--source display|window|region`, `--display`, `--window`, `--x/--y/--width/--height`, `--format mp4|gif|audio`, `--fps`, `--encoder`, `--mic`, `--loopback`, `--no-cursor`.

Always `record stop` when finished (no CLI duration auto-stop).

### Global flags

- Default: JSON envelope on stdout
- `--human`: pretty data only
- `--no-launch`: do not start desktop if control plane is down

## Install / path

Capto installer embeds the CLI at `<install>\cli\capto.exe` and adds that folder to the user **PATH** (open a new terminal after install). Not a separate Release download.

## Repo / cargo

```bash
cargo run -p capto-cli -- status          # binary name `capto`
cargo build -p capto-app                  # desktop executable
```

Desktop package is `capto-app` so it does not overwrite CLI `capto.exe` in `target/debug`. Installed layout uses `cli\capto.exe` because Windows cannot place `capto.exe` beside `Capto.exe`.

Dev: `$env:CAPTO_APP_PATH = "<repo>/target/debug/capto-app.exe"` when auto-launch cannot find Capto.

## Safety

- Prefer `--no-launch` in headless CI
- Never log the Bearer token
- Never double-`record start`
- Never spawn system FFmpeg for Capto outputs; never upload
