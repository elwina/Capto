# Capto Desktop — P2 capture backends

## Current

| OS | Screenshots / window list | Continuous recording |
|----|---------------------------|----------------------|
| Windows | Screenshots: `xcap`; **live preview: DXGI Desktop Duplication** (no GDI cursor flicker) | FFmpeg `gdigrab` + WASAPI → FFmpeg |
| macOS | `xcap` (ScreenCaptureKit path) | Stub / lavfi placeholder until avfoundation wiring |
| Linux | `xcap` | Stub until PipeWire / x11grab wiring |

## Implementing a new recording backend

1. Keep `CaptureBackend` for frame grabs (already cross-platform via xcap).
2. Add `build_record_args_*` in `capto-core/src/ffmpeg_args.rs` behind `cfg(target_os = ...)`.
3. Prefer native APIs:
   - macOS: `avfoundation` or ScreenCaptureKit → FFmpeg
   - Linux: Portal/PipeWire (`pipewire` / `xdg-desktop-portal`) 
4. Do not call FFmpeg from the UI crate.
