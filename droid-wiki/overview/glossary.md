# Glossary

Project-specific vocabulary you will meet while reading the code or the docs. File paths point at where each term is defined or used.

| Term | Meaning |
|------|---------|
| `capto-app` | The desktop binary (crate name `capto-app` in `apps/desktop/src-tauri/Cargo.toml`). Product name is Capto. Distinct from the CLI binary to avoid `target/debug` collisions on case-insensitive Windows. |
| Control plane | The localhost HTTP server the desktop runs (`apps/desktop/src-tauri/src/cli_server.rs`) so the CLI and agents can drive the session. Bound to `127.0.0.1`, protected by a Bearer token from `cli-server.json`. |
| `cli-server.json` | Lockfile written by the control plane (`crates/capto-ipc/src/lockfile.rs`) with `pid`, `port`, `token`, `version`. The CLI's discovery mechanism. |
| Envelope | The CLI stdout contract `{ "ok": true, "data": … }` or `{ "ok": false, "error": { code, message } }` (`crates/capto-ipc/src/envelope.rs`). Exit codes 0–6 map to `ok`, `usage`, `desktopUnavailable`, `stateConflict`, `capture`, `encode`, `configIo`. |
| RecordingSession | The single machine-wide recording orchestration owned by the desktop process (`crates/capto-core/src/session.rs`). States: `Idle → Starting → Recording ⇄ Paused → Stopping → Idle`. |
| `CaptureBackend` | Trait for platform capture (`crates/capto-capture/src/backend.rs`): `list_displays`, `list_windows`, `capture_frame`, `platform_name`. Windows uses WGC/DXGI-oriented implementations; macOS/Linux ship `UnsupportedCaptureBackend` stubs. |
| Sidecar / bundled FFmpeg | The pinned, attested FFmpeg binary from [`elwina/capto-ffmpeg`](https://github.com/elwina/capto-ffmpeg) embedded in the app (`crates/capto-encode/src/lib.rs`). Encoding only goes through this bundle, never a system `PATH` ffmpeg. The pin lives in `.github/capto-ffmpeg.env`. |
| DXGI pump | `DxgiRecordPump` in `crates/capto-capture/src/record_dxgi.rs`, which captures frames via Desktop Duplication and pushes rawvideo to the FFmpeg child's stdin. |
| WASAPI | Windows audio API used by `crates/capto-audio/src/windows.rs` (capture endpoint for mic, render endpoint with loopback for system sound). Both sources are normalized to 48 kHz stereo `f32le`. |
| Faststart remux | Rewriting the fragmented MP4 into a progressive file with `+movflags +faststart` after a clean stop (`remux_frag_to_faststart` in `crates/capto-core/src/session.rs`), so standard players open it reliably. |
| PiP | Webcam picture-in-picture: the webcam stream composited over the capture in-process by `capto_capture::composite_webcam_pip` before encoding. |
| Feature flag | Locally declared toggle resolved from `settings.json` (`enabledFlags` / `disabledFlags`) against the registry in `crates/capto-core/src/flags.rs`. Examples: `control-plane-metrics`, `crash-reporting`. |
| Breadcrumbs | Ring buffer of lifecycle events (`crates/capto-core/src/breadcrumbs.rs`) attached to crash reports so a panic is debuggable without external telemetry. |
| Redaction | `crates/capto-ipc/src/redact.rs` scrubs paths, keys, and other sensitive values out of logs before they are written. |
| Encoder chain | `h264_nvenc → h264_qsv → h264_amf → libx264` (with HEVC equivalents), probed by `FfmpegEncoder::probe_encoders` / `pick_best_h264`. GIF uses palettegen/paletteuse. |
| `capto` (CLI) | The agent/CLI control-plane client binary (crate `capto-cli`). Installed at `<install>\cli\capto.exe` with that folder on the user PATH. |
| `capto-agent-skill` | npm package shipping an Agent Skills doc (`skills/capto/SKILL.md`) teaching agents the doctor → record → stop → outputs workflow. |
| `capto-dsh-plugin` | npm package registering 14 typed `capto_*` tools for DeepSeek Harness (dsh). |
| Captura | The original open-source screen recorder this project succeeds. Capto is a clean-room implementation, not a fork. |
| dsh | DeepSeek Harness, an agent runtime; `capto-dsh-plugin` plugs into its Cordis-based profile system. |
| gdigrab / rawvideo | FFmpeg input concepts used in `crates/capto-core/src/ffmpeg_args.rs`: the rawvideo pipe carries capture frames; gdigrab crop geometry is used for region capture resolution. |

## Related pages

- [Architecture](architecture.md) puts these terms in context
- [Reference configuration](../reference/configuration.md) covers the settings model
- [Control-plane API](../api/index.md) uses the envelope and lockfile terms
