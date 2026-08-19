# Features

Active contributors: elwina

## Purpose

These pages describe the user- and agent-visible capabilities of Capto: what each feature does, how it works, and where to change it. They sit one level above the crate and app pages, which cover the internals. All features are local by design; nothing uploads or shares remotely.

| Feature | What it is | Page |
|---------|------------|------|
| Recording | Start, pause, resume, and stop screen recordings as MP4, GIF, or audio-only files from the UI, tray, hotkeys, or CLI | [Recording](recording.md) |
| Overlays | On-screen feedback during a recording: mouse-click highlights, keystrokes, and the recording frame | [Overlays](overlays.md) |
| Audio capture | Microphone and system-sound (loopback) capture with per-source volume, mixed into the recording | [Audio capture](audio-capture.md) |
| Webcam PiP | Picture-in-picture webcam overlay composited into the frame before encoding | [Webcam PiP](webcam-pip.md) |
| Hotkeys | Global shortcuts, default `Alt+F5` through `Alt+F8`, that start, pause, stop, and screenshot | [Hotkeys](hotkeys.md) |
| Updates | In-app updates served from the worker mirror with a signed `latest.json`, keeping the installed app current | [Updates](updates.md) |

## Where the pieces live

- [CLI](../apps/cli.md) provides the agent-facing control surface (`capto record start` / `status` / `record stop` / `outputs`).
- [Desktop app](../apps/desktop/index.md) owns the single `RecordingSession` and exposes the UI commands, tray, hotkeys, and previews.
- [Control-plane API](../api/index.md) documents the `/v1/record/*` endpoints behind the CLI.
- [capto-core](../crates/capto-core.md) holds session orchestration, settings, and FFmpeg argv building.
- [capto-capture](../crates/capto-capture.md) provides the DXGI record pump and webcam capture used by recording and preview.
- [capto-encode](../crates/capto-encode.md) wraps the bundled FFmpeg sidecar: discovery, encoder probe, and remux.
- [capto-audio](../crates/capto-audio.md) captures WASAPI microphone and loopback inputs as PCM streams.

## Feature pages vs crate pages

Feature pages describe behavior from the user's point of view and point at the specific files to touch. Crate pages describe the same machinery from the code's point of view. When both exist, read the feature page first, then follow the links into the crate pages for the details.
