---
name: capto
description: >-
  Drive Capto local screen capture via the `capto` CLI (JSON localhost control
  plane). Use when recording the screen, taking screenshots, checking Capto
  session status, changing Capto settings, listing displays/windows/audio/
  encoders, or finding recent Capto output files. Prefer `capto` over spawning
  system FFmpeg.
license: MIT
compatibility: Requires Capto desktop on Windows (installer embeds FFmpeg + CLI on PATH); or cargo run -p capto-cli in a dev checkout
metadata:
  author: elwina
  version: "0.5.0"
  npm: capto-agent-skill
---

# Capto CLI

Capto is a **local-only** screen recorder. Agents control the **single** Capto desktop process with the `capto` CLI (loopback HTTP). Do not invent a second capture pipeline or upload anywhere.

## Rules

1. Invoke `capto <command>` after Capto install (PATH); in this repo use `cargo run -p capto-cli -- <command>` (binary name is `capto`).
2. Parse **JSON stdout**: `{ "ok": true, "data": … }` or `{ "ok": false, "error": { "code", "message" } }`. Ignore stderr traces.
3. Exit codes: `0` ok, `1` usage, `2` desktop unavailable, `3` state conflict, `4` capture, `5` encode, `6` config IO.
4. One session — if `status` is `recording`, do not `record start` again.
5. Never spawn system FFmpeg for Capto outputs; never add share/upload steps.
6. Details: [references/cli.md](references/cli.md).

## Desktop must be running

Most commands talk to the Capto **desktop** control plane. If it is down:

1. Run `capto open` (opens installed / discovered `capto-app.exe`).
2. Wait ~3–5s, then `capto status` (or retry the original command).
3. If still exit `2` / `desktopUnavailable`: **ask the user to open Capto** (Start menu → Capto), wait until the window is up, then retry. Do not loop forever.

Optional Windows fallback (same idea as `capto open`):

```powershell
Start-Process "$env:LOCALAPPDATA\Capto\capto-app.exe"
```

`--no-launch` skips auto-start (use in CI). Default commands may auto-open the desktop; prefer explicit `capto open` + user prompt when that fails.

## Workflows

### Screenshot

```bash
capto status || capto open
capto shot --source display
# → data.path (absolute PNG)
```

### Record

```bash
capto doctor                    # optional; exit 2 → open / ask user; ffmpegOk must be true
capto list displays             # optional
capto record start --source display
capto status                    # poll
capto record stop
capto outputs recent --limit 1
```

If `record start` / `list encoders` fails with empty FFmpeg stderr: **fully quit Capto** (tray Quit), run `capto open`, then retry. A wedged desktop process can still answer `status` while failing to spawn FFmpeg.

### Settings

```bash
capto config get fps
capto config set fps=30
capto config set --json "{\"includeCursor\":true}"
```

Keys are camelCase (`outputDir`, `micDevice`, `overlays`, …).

### Discovery

```bash
capto list displays|windows|audio|encoders
capto outputs recent --limit 10
```

## Flags

- Default: JSON envelope on stdout
- `capto open`: start desktop only (no control-plane wait)
- Auto-launch (default on other commands): opens desktop if control plane is down
- `--no-launch`: fail if Capto desktop is not already running
- Dev: `$env:CAPTO_APP_PATH = "<repo>\target\debug\capto-app.exe"` if discovery fails (plain path; no `\\?\`)

## Out of scope

No `capto quit` yet. Desktop binary is `capto-app.exe` (product name Capto) — not the CLI `capto.exe`.
