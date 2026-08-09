---
name: capto
description: >-
  Drive Capto local screen capture via the `capto` CLI (JSON localhost control
  plane). Use when recording the screen, taking screenshots, checking Capto
  session status, changing Capto settings, listing displays/windows/audio/
  encoders, or finding recent Capto output files. Prefer `capto` over spawning
  system FFmpeg.
license: MIT
compatibility: Requires Capto desktop on Windows with bundled FFmpeg; CLI binary `capto` on PATH or via cargo run -p capto-cli
metadata:
  author: elwina
  version: "0.1.0"
  npm: capto-agent-skill
---

# Capto CLI

Capto is a **local-only** screen recorder. Agents control the **single** Capto desktop process with the `capto` CLI (loopback HTTP). Do not invent a second capture pipeline or upload anywhere.

## Rules

1. Invoke `capto <command>` when on PATH; in this repo use `cargo run -p capto-cli -- <command>` (binary name is `capto`).
2. Parse **JSON stdout**: `{ "ok": true, "data": … }` or `{ "ok": false, "error": { "code", "message" } }`. Ignore stderr traces.
3. Exit codes: `0` ok, `1` usage, `2` desktop unavailable, `3` state conflict, `4` capture, `5` encode, `6` config IO.
4. One session — if `status` is `recording`, do not `record start` again.
5. Never spawn system FFmpeg for Capto outputs; never add share/upload steps.
6. Details: [references/cli.md](references/cli.md).

## Workflows

### Screenshot

```bash
capto shot --source display
# → data.path (absolute PNG)
```

### Record

```bash
capto doctor                    # optional
capto list displays             # optional
capto record start --source display
capto status                    # poll
capto record stop
capto outputs recent --limit 1
```

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
- `--no-launch`: fail if Capto desktop is not already running
- Dev: `$env:CAPTO_APP_PATH = "<repo>/target/debug/capto-app.exe"` if auto-launch cannot find the desktop

## Out of scope

No `capto quit` yet. Desktop binary is `capto-app` / installed `Capto.exe` — not the CLI.
