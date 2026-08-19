# Design decisions

Capto is a purely local Windows screen recorder built as a Tauri 2 + Rust + React monorepo. This page records the architectural choices that shape the product and the reasoning behind each one, grouped by theme. Every decision lists concrete evidence in the code or docs so a reader can verify it against the current tree.

The shaping of the recording pipeline itself is covered in [Architecture](../overview/architecture.md); this page focuses on why the decisions were made, not how they connect.

## Local-first product positioning

### Purely local, no upload SDKs

**Decision.** Capto ships no upload, embedding, or sharing SDKs and no remote telemetry.

**Why.** Capto positions itself as a privacy-respecting successor to Captura. Staying local-only is the defining product boundary, so no cloud, OAuth, or "share to" feature is built in or planned, and no third-party analytics SDK sits in the dependency tree.

**Evidence.** `AGENTS.md` non-negotiables (no Imgur/YouTube/OAuth); `docs/PRIVACY.md` documents "no uploads" and "no telemetry to third parties"; `docs/analytics.md` states there are no Mixpanel/Amplitude/PostHog/GA4 packages.

### No database: settings.json only

**Decision.** There is no database and no ORM. State is either in-memory during a session or single small files on disk.

**Why.** A tiny, user-editable settings file keeps the app install-free and trivially portable. Session state is short-lived and serialized on demand; control-plane discovery is a one-process lock; recordings are files, not rows. The ADR argues that a database would be absurd for this data shape.

**Evidence.** The no-db architecture decision record is `docs/data.md` (added by commit 77fff77); the `AppSettings` serde type is the source of truth in `crates/capto-core/src/settings.rs`; `docs/settings-schema.json` mirrors its serialized shape.

### Privacy-first observability

**Decision.** Diagnostics stay on the box: a local metrics endpoint, local crash reports with a capped breadcrumb trail, and scrubbed logs.

**Why.** Agents and maintainers still need visibility into a purely local app, but nothing should ever leave the machine. The control plane serves `GET /v1/metrics` under auth, crashes write `crash-*.json` with a context trail, and `capto_ipc::redact` scrubs tokens from logs.

**Evidence.** `docs/PRIVACY.md`, `docs/analytics.md`, `docs/crash-tracing.md`, and `apps/desktop/src-tauri/src/lib.rs` (metrics registry + crash writer).

### Feature flags as a local declarative registry

**Decision.** Flags are a local declarative registry in `crates/capto-core/src/flags.rs`, resolved from `settings.json`, rather than remote kill switches.

**Why.** Agents can ship a behavior behind a toggle that defaults to safe, flip it via `capto config set`, and disable it without a release. Keeping the registry and locks local is consistent with the privacy-first positioning, since a remote switch would require a network call and an account of entitled toggles.

**Evidence.** `crates/capto-core/src/flags.rs` (registry + `is_enabled` resolution), `docs/feature-flags.md`, `scripts/scan-dead-flags.ps1`.

## Control plane and session ownership

### One machine-wide session owned by the desktop

**Decision.** The desktop owns a single `RecordingSession`; the `capto` CLI is a control-plane client and never owns a session; the app is single-process via `tauri-plugin-single-instance`.

**Why.** Two independent capture pipelines would compete for the same display and audio devices. A single session owned by the desktop, driven through a localhost control plane, avoids that. The single-instance plugin also means the CLI's spawn-on-demand cannot create a second session.

**Evidence.** `apps/desktop/src-tauri/src/lib.rs` registers the single-instance plugin first; `docs/ARCHITECTURE.md` describes the one-session invariant and the CLI→desktop loopback contract.

### Distinct binary names: capto vs capto-app

**Decision.** The CLI binary is `capto` (crate `capto-cli`) and the desktop binary is `capto-app` (crate `capto-app`).

**Why.** Naming both `capto` would collide in `target/debug` and on case-insensitive Windows paths, so the two binaries are kept distinct. The installer ships the CLI at `<install>/cli/capto.exe` and adds that folder to PATH.

**Evidence.** `AGENTS.md` binaries table, `docs/CLI.md`, `docs/ARCHITECTURE.md` ("CLI vs desktop binaries stay distinct").

## Encoding pipeline

### Encoding only through a pinned, attested bundled FFmpeg

**Decision.** All encoding goes through a bundled FFmpeg sidecar, resolved only from `binaries/` or next to the app exe, never the system PATH. The FFmpeg build is pinned to a known release.

**Why.** Reproducibility and supply-chain trust matter more than convenience. A pinned build means every machine encodes identically and CI can attest the exact binary, instead of depending on whatever `ffmpeg` happens to be installed.

**Evidence.** `crates/capto-encode/src/lib.rs` (`FfmpegNotFound` and sidecar-only resolution), `.github/capto-ffmpeg.env` (pins `elwina/capto-ffmpeg` tag `v1.0.0-n9.0`), `docs/CI.md`.

### Encoder fallback chain with boot-failure fallback

**Decision.** MP4 prefers `h264_nvenc` → `h264_qsv` → `h264_amf` → `libx264`, and a hardware encoder that fails at boot falls back to `libx264` rather than aborting the recording.

**Why.** Hardware encoders keep CPU low but are not available on every machine, so a chain maximizes hardware use while guaranteeing a software fallback. Because encoder failures manifest at pipeline boot, the fallback is triggered from the boot path, not at a later retry.

**Evidence.** `crates/capto-core/src/session.rs` (`pick_best_h264` and the boot-failure fallback branch), `crates/capto-encode/src/lib.rs` (encoder list).

### Faststart remux on clean stop

**Decision.** On a clean stop of an MP4, the fragmented file is remuxed into a progressive MP4 with `+faststart`.

**Why.** Fragmented MP4 was not reliably openable in common players, so the recording is rewritten to a progressive, faststart layout on stop when FFmpeg exits cleanly. If the remux fails, the fragmented file is kept so the user still has something.

**Evidence.** `remux_frag_to_faststart` in `crates/capto-core/src/session.rs` (doc comment cites Movies & TV, QuickTime).

### Pause stops the producers

**Decision.** Pause is implemented by stopping the video pump and audio session (`pump.set_paused(true)` / `audio.set_paused(true)`) rather than by pausing the output muxer.

**Why.** Stopping the producers makes the encode timeline skip the paused wall time, so the elapsed clock and the output do not include the pause. Resume reverses the two and accumulates the pause duration into the elapsed calculation.

**Evidence.** `RecordingSession::pause` / `resume` in `crates/capto-core/src/session.rs`.

## Capture and audio backends

### DXGI Desktop Duplication for the preview

**Decision.** The live preview uses DXGI Desktop Duplication rather than GDI BitBlt.

**Why.** GDI `BitBlt` of the desktop DC makes Windows briefly hide the system cursor on every grab, which reads as mouse jitter. DXGI (and WGC) do not.

**Evidence.** The module doc comment in `crates/capto-capture/src/preview.rs`.

### WASAPI normalized to 48 kHz stereo f32le

**Decision.** Windows audio sources (mic capture, render loopback) are normalized to 48 kHz stereo `f32le`, paced against wall-clock time with silence insertion, and streamed over localhost TCP.

**Why.** Normalizing both sources to one format lets FFmpeg mix and mux the PCM inputs without ever touching Windows audio devices, and TCP gives each source independent backpressure while keeping FFmpeg's stdin free.

**Evidence.** `crates/capto-audio/src/windows.rs` (constants `SAMPLE_RATE: u32 = 48_000`, two channels, `f32` samples; TCP transports), `docs/ARCHITECTURE.md`.

## Distribution and resilience

### Updater mirror worker plus canary channel

**Decision.** A Cloudflare Worker mirrors the GitHub release and exposes stable (`latest.json`) and canary (`canary.json`) channels, listed before GitHub in the update endpoints.

**Why.** In regions where `github.com` is slow, a nearer CDN edge keeps update checks fast, and rebuilding download URLs avoids GitHub's API rate limit on asset links. The canary channel lets pilot users adopt a version before stable promotion. Worker-first / GitHub-second order gives an automatic fallback if the mirror is down.

**Evidence.** `cloudflare/README.md`, `apps/updater-mirror.md`, `docs/CI.md`.

### NSIS-only installers

**Decision.** The project ships NSIS exe installers only; MSI was dropped.

**Why.** Shipping one installer format keeps the signing and support matrix simpler than maintaining both an MSI and an NSIS path.

**Evidence.** Commit 7252d0e ("ship NSIS exe installers only (drop MSI)"); NSIS hook at `windows/hooks.nsh`.

### Minimize-to-tray and hide-while-recording defaults

**Decision.** On close the app minimizes to tray, and the app window hides while recording, both enabled by default in settings.

**Why.** These defaults match the expected behavior of a background screen recorder: reduce presence during capture and avoid accidentally closing the app. They are user configurable.

**Evidence.** `AppSettings::default` in `crates/capto-core/src/settings.rs` sets `hide_app_while_recording: true` and `minimize_to_tray_on_close: true`.
