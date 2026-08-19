# Capto overview

Capto is an ultra-light, purely local screen recorder for Windows 10+, written as a clean-room spiritual successor to [Captura](https://github.com/MathewSachin/Captura). It records display, window, or region to MP4 (NVENC/QSV/AMF/libx264) or GIF, with mouse-click and keystroke overlays, webcam picture-in-picture, global hotkeys, and a full agent-facing CLI. Files stay on your machine: there are no upload SDKs, no accounts, and no cloud features in product code.

The project is a Rust + TypeScript monorepo. The desktop app is a Tauri 2 shell (`apps/desktop`) whose React UI sends intents over Tauri commands; all frame processing happens in Rust crates under `crates/`. The `capto` CLI (`crates/capto-cli`) controls the running desktop over a localhost HTTP control plane rather than owning its own recording pipeline, which guarantees one recording session per machine. Two npm packages (`packages/capto-agent-skill`, `packages/capto-dsh-plugin`) let AI agents drive the same CLI.

The wiki maps to the codebase as follows:

- [Architecture](architecture.md), the recording pipeline, session state machine, and control plane
- [Getting started](getting-started.md), prerequisites, setup, build, test, run
- [Glossary](glossary.md), project vocabulary
- [Apps](../apps/index.md), desktop app, CLI, website, updater mirror worker
- [Crates](../crates/index.md), the Rust workspace packages
- [Packages](../packages/index.md), the npm agent packages
- [Features](../features/index.md), recording, overlays, audio, webcam PiP, hotkeys, updates
- [Control-plane API](../api/index.md), the localhost HTTP contract agents talk to
- [Deployment](../deployment.md), CI, release, and hosting pipelines
- [Security](../security.md), trust boundaries and privacy controls
- [How to contribute](../how-to-contribute/index.md), working in this repo

## Key facts

- Version 1.0.0 (stable), MIT license, Windows first (macOS/Linux capture backends are stubs)
- Stack: Tauri 2, Rust (workspace of 9 crates), React 19 + TypeScript + Vite, FFmpeg sidecar from [`elwina/capto-ffmpeg`](https://github.com/elwina/capto-ffmpeg)
- Encoding goes only through the bundled FFmpeg sidecar, never a system/PATH FFmpeg
- One `RecordingSession` per machine, owned by the desktop process; the CLI is a client, not a second recorder
- Repo hygiene gates (`scripts/scan-tech-debt.ps1` and friends) keep the source free of TODO/FIXME markers and oversized files
