# Capto CLI

Agent-oriented control surface for Capto. The CLI binary is named **`capto`**. It talks to the **running desktop app** (`capto-app` / installed Capto) over a localhost HTTP control plane. It does **not** create a second `RecordingSession`.

Full architecture: [ARCHITECTURE.md](ARCHITECTURE.md).  
Agent skill (npm): [`packages/capto-agent-skill`](../packages/capto-agent-skill).

## Mental model

```
Agent / shell
    → capto  (JSON stdout, stable exit codes)
        → 127.0.0.1:<port> + Bearer token
            → Capto desktop (single process)
                → RecordingSession / settings / outputs
```

| Rule | Detail |
|------|--------|
| CLI binary | `capto` (crate `capto-cli`) |
| Desktop binary | `capto-app` in cargo builds; product name Capto |
| Single session | One Capto process machine-wide |
| Auto-launch | If control plane is down, CLI starts desktop (unless `--no-launch`) |
| Discovery | `%APPDATA%\Capto\cli-server.json` |
| No upload | Local files only |

> **Why not both named `capto`?** Cargo would write two `target/debug/capto.exe`. On Windows, same-folder `Capto.exe` / `capto.exe` also collide (case-insensitive). So CLI owns `capto`; desktop crate is `capto-app`.

## Invoke

```bash
# Prefer when installed on PATH
capto <command> [args]

# From this repo
cargo run -p capto-cli -- <command> [args]
# → builds/runs target/debug/capto.exe
```

Global flags:

| Flag | Meaning |
|------|---------|
| (default) | JSON envelope on stdout |
| `--human` | Pretty data only (no envelope) |
| `--no-launch` | Fail if desktop control plane is down |

Dev auto-launch:

```powershell
$env:CAPTO_APP_PATH = "D:\AIWorkspace\Capto\target\debug\capto-app.exe"
```

Lookup order: `CAPTO_APP_PATH` → `Capto.exe` / `capto-app.exe` beside CLI → `target/debug|release/capto-app.exe` → common install paths.

## JSON envelope (agent contract)

**Success**

```json
{ "ok": true, "data": { } }
```

**Failure**

```json
{ "ok": false, "error": { "code": "desktopUnavailable", "message": "…" } }
```

- stdout = envelope (or `--human` data)
- stderr = tracing (ignore for parsing)
- `data` fields are **camelCase**

### Exit codes

| Code | Name | When |
|------|------|------|
| 0 | ok | Success |
| 1 | usage | Bad args / unknown key |
| 2 | desktopUnavailable | No control plane / auth / launch failed |
| 3 | stateConflict | e.g. start while already recording |
| 4 | capture | Capture / device failure |
| 5 | encode | FFmpeg / encoder failure |
| 6 | configIo | Settings / outputs filesystem error |

Branch on **exit code first**, then `error.code`.

## Commands

### `doctor`

```bash
capto doctor
```

### `status`

```bash
capto status
```

States: `idle` | `starting` | `recording` | `paused` | `stopping`.

### `list`

```bash
capto list displays
capto list windows
capto list audio
capto list encoders
```

### `config`

```bash
capto config path
capto config get
capto config get fps
capto config set fps=60
capto config set --json "{\"fps\":60,\"includeCursor\":true}"
```

Keys are **camelCase**. Overlay tweaks via `--json` on `overlays`.

### `shot`

```bash
capto shot --source display
capto shot --source display --display 0
capto shot --source window --window <hwnd>
capto shot --source region --x 0 --y 0 --width 1280 --height 720
```

### `record`

```bash
capto record start --source display
capto record start --source display --display 0 --format mp4 --fps 30
capto record pause
capto record resume
capto record stop
```

Always `record stop` when done (no duration auto-stop).

### `outputs`

```bash
capto outputs recent --limit 10
capto outputs open --last
capto outputs open --folder
```

## Agent workflows

### Record a short clip

```text
1. doctor
2. list displays
3. record start --source display
4. status          # poll
5. record stop
6. outputs recent --limit 1
```

### Screenshot

```text
1. shot --source display
2. use data.path
```

### Desktop already running

```bash
capto --no-launch status
```

## HTTP map

See [ARCHITECTURE.md](ARCHITECTURE.md). Shared types: `capto-ipc`.

## Agent skill (npm)

```bash
npm install capto-agent-skill
```

Ships `skills/capto/SKILL.md` per [Agent Skills](https://agentskills.io) + npm `skills/` convention.

## Not in CLI (yet)

- `quit` / close desktop
- Interactive pickers
- MCP server wrapper

## Safety

- Prefer `--no-launch` in headless CI
- Do not log the Bearer token from `cli-server.json`
- Do not double-`record start`
- Encode only through Capto
