# Capto — Agent Guide

Capto is a **purely local** screen capture app (Captura spiritual successor).  
Stack: **Tauri 2 + Rust + React/TypeScript**. No C#. No cloud upload.

## Non-negotiables

1. **No upload SDKs** — no Imgur, YouTube, OAuth, or remote sharing features.
2. **Encoding only via `capto-encode`** — never spawn FFmpeg ad-hoc from UI crates.
3. **New capture backends must implement `CaptureBackend`** in `capto-capture`.
4. **Windows first** — ship WGC/WASAPI path; macOS/Linux backends are stubs until implemented.
5. **UI does not process frames** — React only sends intents and renders state.

## Repo layout

| Path | Role |
|------|------|
| `apps/desktop` | Tauri shell + React UI |
| `crates/capto-core` | Session orchestration, settings |
| `crates/capto-capture` | Capture traits + platform backends |
| `crates/capto-audio` | Mic + loopback listing/mixing intents |
| `crates/capto-encode` | FFmpeg sidecar + encoder probe |
| `crates/capto-overlay` | Overlay layout / compositor helpers |
| `crates/capto-hooks` | Hotkey / input hook abstractions |
| `crates/capto-ipc` | Local CLI↔desktop control-plane types + lockfile |
| `crates/capto-cli` | CLI client for the desktop control plane |
| `docs/ARCHITECTURE.md` | Pipeline contracts |

## Cloud vs local Windows

| Task | Cloud agent | Local Windows |
|------|-------------|---------------|
| Traits, UI, i18n, CLI, unit tests | Yes | Optional |
| WGC / WASAPI / NVENC real recording | No | **Required** |
| Package installers | CI Windows runner | Verify locally |

## Dev commands

```bash
npm install --prefix apps/desktop
cargo test --workspace
npm run tauri --prefix apps/desktop -- dev
cargo run -p capto-cli -- --help
```

Place a local FFmpeg into the app bundle (no download):

```bash
# PowerShell — copies from PATH / FFMPEG_PATH / -Source
.\scripts\copy-ffmpeg.ps1
```

See `apps/desktop/src-tauri/binaries/README.md`. Runtime uses **only** that bundled binary.

## Feature matrix

See root `README.md`. P0 = MVP, P1 = next, cut list is permanent.
