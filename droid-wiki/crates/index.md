# Crates

Active contributors: elwina

## Purpose

The Rust code that records, encodes, and controls Capto lives in the `crates/` workspace. Eight crates are documented here, one per page, plus the Tauri shell (`apps/desktop/src-tauri/`), which is covered under [Apps](../apps/index.md). Each crate owns one slice of the pipeline: capture, audio, encoding, session orchestration, overlay geometry, hotkeys, shared control-plane types, and the CLI client. Dependency flow is one-way: `crates/capto-core/` orchestrates the capture, audio, and encode crates, and produces the types that `crates/capto-ipc/` and the desktop shell consume.

| Crate | One-line summary |
|-------|------------------|
| [capto-core](capto-core.md) | Session orchestration: the `RecordingSession` state machine, settings persistence, FFmpeg argv building, feature flags, and observability primitives |
| [capto-capture](capto-capture.md) | Capture backends: the `CaptureBackend` trait, the Windows DXGI record pump, frame buffer and previews, webcam capture and PiP compositing |
| [capto-encode](capto-encode.md) | The FFmpeg sidecar wrapper: sidecar discovery, encoder probe (`pick_best_h264`), video-encoder args, remux and one-shot runs |
| [capto-audio](capto-audio.md) | WASAPI audio sessions: mic and loopback devices, PCM input specs and mixing intents, live level metering for the UI |
| [capto-overlay](capto-overlay.md) | Overlay layout config and compositor helpers: mouse-click highlights, keystrokes, webcam PiP, cursor, and shared position math |
| [capto-hooks](capto-hooks.md) | Hotkey and input-hook abstractions: `HotkeyBinding`, the `Alt+F5..F8` default cluster, and legacy migration |
| [capto-ipc](capto-ipc.md) | Local control-plane types shared between the desktop shell and the CLI: the JSON envelope, `cli-server.json` lockfile, request/response types, and redaction |
| [capto-cli](../apps/cli.md) | The `capto` binary: a control-plane client over localhost HTTP; its code is covered on the [CLI app page](../apps/cli.md), not here |

## Directory layout

| Path | Crate | Role |
|------|-------|------|
| `crates/capto-core/` | capto-core | Recording session orchestration, settings, flags, observability |
| `crates/capto-capture/` | capto-capture | Capture backends, DXGI pump, webcam, previews |
| `crates/capto-encode/` | capto-encode | FFmpeg sidecar discovery and invocation |
| `crates/capto-audio/` | capto-audio | WASAPI microphone and loopback sessions |
| `crates/capto-overlay/` | capto-overlay | Overlay layout and compositor helpers |
| `crates/capto-hooks/` | capto-hooks | Hotkey and input hook abstractions |
| `crates/capto-ipc/` | capto-ipc | CLI and desktop control-plane types, envelope, lockfile |
| `crates/capto-cli/` | capto-cli | The `capto` CLI client binary |

## Workspace notes

- The root `Cargo.toml` lists nine workspace members: the eight crates above plus `apps/desktop/src-tauri` (the `capto-app` binary, documented under [Apps](../apps/index.md)).
- Shared dependencies are declared once in the root workspace table and referenced with `{ workspace = true }`; all crates share version 1.0.0 and the MIT license from `[workspace.package]`.
- Windows-first: capture and audio backends are implemented for Windows, with macOS/Linux staying as stubs.
- Encoding goes only through the bundled FFmpeg sidecar via `crates/capto-encode/`; no crate spawns FFmpeg ad-hoc (see [Patterns and conventions](../how-to-contribute/patterns-and-conventions.md)).
