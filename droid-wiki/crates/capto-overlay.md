# capto-overlay

Active contributors: elwina

## Purpose

`crates/capto-overlay` is the pure configuration and layout model for screen overlays. It defines every overlay type, the anchor and position math used to place them, and the shared pixel-coordinate resolver that both the UI drag preview and the Rust frame compositor call. Because it owns no I/O and has no platform hooks, both the frontend (via serialized settings) and the backend (during encode) derive their behavior from a single `OverlayConfig` embedded in `AppSettings`, making it the single source of truth for overlay settings. It also keeps the positional math and the FFmpeg filtergraph path escaping in one testable place.

## Directory layout

| File | Role |
|------|------|
| `crates/capto-overlay/src/lib.rs` | The whole crate: every overlay type, `OverlayConfig`, `resolve_pixel_position`, `escape_filter_path`, and unit tests |

## Key abstractions

| Abstraction | Where | What it does |
|-------------|-------|--------------|
| `OverlayConfig` | `crates/capto-overlay/src/lib.rs` | The aggregate of all overlays (`mouse_clicks`, `keystrokes`, `elapsed`, `texts`, `images`, `webcam`); embedded in settings |
| `WebcamPip` | `crates/capto-overlay/src/lib.rs` | PiP compositor inputs: default `320x240`, `mirrored` on, corner radius 12, plus `device_id` / `device_label` match keys |
| `TextOverlay` | `crates/capto-overlay/src/lib.rs` | Text with `id`, `text`, `font_size`, `color`, and `position` |
| `ImageOverlay` | `crates/capto-overlay/src/lib.rs` | Image from `path`, sized by `width`/`height`, with `opacity` and `position` |
| `MouseClickOverlay` | `crates/capto-overlay/src/lib.rs` | Click circles with button colors (`#FF5252`/`#448AFF`/`#69F0AE`) and `radius` (default 18) |
| `KeystrokeOverlay` | `crates/capto-overlay/src/lib.rs` | key display with `position`, `font_size`, `color`, `background` |
| `ElapsedOverlay` | `crates/capto-overlay/src/lib.rs` | Deprecated; kept for settings JSON compatibility, not burned into recordings |
| `OverlayAnchor` | `crates/capto-overlay/src/lib.rs` | `TopLeft`, `TopRight`, `BottomLeft`, `BottomRight`, `Center`, `Custom` |
| `OverlayPosition` | `crates/capto-overlay/src/lib.rs` | `anchor` plus normalized `x`/`y`; default bottom-right at `0.85, 0.85` |
| `resolve_pixel_position` | `crates/capto-overlay/src/lib.rs` | Converts a normalized anchor+offset into concrete pixel coordinates against a frame and box size |
| `escape_filter_path` | `crates/capto-overlay/src/lib.rs` | Escapes a filesystem path for use inside an FFmpeg filtergraph option |

## How it works

All types are `Serialize`/`Deserialize` with `serde(rename_all = "camelCase")`, so the UI and the Rust compositor exchange the same shape. `OverlayConfig` is the root the settings layer stores; overlays are `Vec`s (`texts`, `images`) so users can add several of the same kind, plus one each of the singleton behaviors (`mouse_clicks`, `keystrokes`, `elapsed`, `webcam`).

`resolve_pixel_position(pos, frame_w, frame_h, box_w, box_h)` maps an `OverlayPosition` to an `(x, y)` pixel top-left. For the pre-defined anchors it places the box corner-anchored (e.g. `BottomRight` at `(fw-bw, fh-bh)`, `Center` at the midpoint of the remaining space) and then applies a fine-tune of `(pos.x - 0.5) * 40.0` / `(pos.y - 0.5) * 40.0` so the UI can nudge within ±40 px without moving the anchor. `Custom` uses the raw `pos.x * fw`, `pos.y * fh` and skips the nudge. The unit test `bottom_right_position` pins that a bottom-right placement of a `320x240` box lands in the lower-right region of a `1920x1080` frame.

`escape_filter_path` turns a Windows path into a filtergraph-safe value: backslashes to forward slashes, drive colons to `\:`, and quotes to the shell-quote form; `escape_filter_path_escapes_drive_colon` verifies `C:\Windows\Fonts\arial.ttf` becomes `C\:/Windows/Fonts/arial.ttf`.

## Integration

- Settings embed `OverlayConfig` directly in `AppSettings`; the settings load/save path in `apps/desktop/src-tauri/src/settings.rs` persists the camelCase JSON so the UI and compositor both read the same values.
- The frame compositor places the webcam PiP: `composite_webcam_pip` in `crates/capto-capture/src/composite.rs` calls `resolve_pixel_position(&pip.position, base.width, base.height, box_w, box_h)` to offset the composited camera frame (see [capto-capture](capto-capture.md)).
- The UI drag preview uses the same `resolve_pixel_position` helper so what the user drags matches what gets composited; see [overlays](../features/overlays.md) for the user-visible behavior of click, keystroke, text, image, and PiP overlays.
- Click and keystroke events reach the renderers from `crates/capto-hooks`; see [capto-hooks](capto-hooks.md).

## Entry points for modification

- **Add or change an overlay type**: edit the struct and its `Default` in `crates/capto-overlay/src/lib.rs`, and add it to `OverlayConfig`.
- **Adjust placement math**: `resolve_pixel_position` (anchor mapping, the ±40 px fine-tune, or the `Custom` branch) in `crates/capto-overlay/src/lib.rs`.
- **Change filtergraph escaping**: `escape_filter_path` and its unit tests in `crates/capto-overlay/src/lib.rs`.
- **Compositor consumption**: update the caller in `crates/capto-capture/src/composite.rs` if the layout API changes.

## Key source files

| File | What to look for |
|------|------------------|
| `crates/capto-overlay/src/lib.rs` | All overlay types, `OverlayConfig`, `OverlayAnchor`/`OverlayPosition`, `resolve_pixel_position`, `escape_filter_path`, unit tests |
| `apps/desktop/src-tauri/src/settings.rs` | `OverlayConfig` embedded in `AppSettings`, load/save |
| `crates/capto-capture/src/composite.rs` | `composite_webcam_pip` consuming `resolve_pixel_position` |
