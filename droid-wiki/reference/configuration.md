# Configuration

Capto is local-first and has exactly one configuration surface: `settings.json`, a plain JSON document in the app config directory. There is no database and no remote settings service.

## Where the file lives

The path is `config_dir()/Capto/settings.json`, computed in `AppSettings::config_path` in `crates/capto-core/src/settings.rs` using `dirs::config_dir`. On Windows that resolves to `%APPDATA%\Capto\settings.json`. `load` and `save` read and write this file (pretty-printed JSON); both go through hotkey normalization, so legacy hotkey strings are migrated on every load and save.

## Settings table

All keys use camelCase and mirror `AppSettings` in `crates/capto-core/src/settings.rs`. Defaults come from serde defaults plus the `Default` impl.

| Key | Type | Default | Meaning |
|-----|------|---------|---------|
| `outputDir` | string | `Videos/Capto` via `dirs::video_dir()` | Directory for recordings and screenshots. |
| `outputFormat` | `mp4` \| `gif` \| `audioOnly` | `mp4` | Encoded container/output kind. |
| `fps` | integer | `30` | Target capture frame rate. |
| `quality` | integer 1..100 | `60` | Captura-style perceptual quality, mapped to encoder CRF for video. |
| `includeCursor` | boolean | `true` | Whether the cursor is burned into recordings. |
| `preferredEncoder` | string \| null | `null` | `h264Nvenc`, `h264Qsv`, `h264Amf`, `libx264`, `hevcNvenc`, `hevcQsv`, `hevcAmf`, `libx265`, or `gif`. When `null`, the best H.264 encoder available is picked automatically. |
| `micDevice` | string \| null | `null` | Microphone device label. |
| `loopbackDevice` | string \| null | `null` | System-loopback audio device label. |
| `micVolume` | integer 0..200 | `100` | Microphone gain percentage. |
| `loopbackVolume` | integer 0..200 | `100` | Loopback gain percentage. |
| `defaultSource` | `display` \| `window` \| `region` | `display` | Default capture source kind. |
| `defaultDisplayId` | integer \| null | `0` | Display id used when source is a display. |
| `defaultWindowId` | integer \| null | `null` | Last picked window id (HWND), restored when present. |
| `defaultWindowTitle` | string \| null | `null` | Friendly window title used when the id is stale after restart. |
| `defaultRegion` | object \| null | `null` | Restored region (`x`, `y`, `width`, `height`). |
| `hideAppWhileRecording` | boolean | `true` | Hide the desktop window while recording. |
| `minimizeToTrayOnClose` | boolean | `true` | Minimize to tray when the window closes. |
| `showPreview` | boolean | `false` | Show the capture preview. |
| `locale` | string | `en` | UI locale tag, for example `en` or `zh-CN`. |
| `hotkeys` | array | 4 bindings | `{ action, shortcut, enabled }` items; defaults bound to `Alt+F5`..`Alt+F8`. |
| `overlays` | object | defaults | Overlay config (mouse clicks, keystrokes, texts, images, webcam). |
| `enabledFlags` | string array | `[]` | Explicitly enabled feature flags (empty means use declared defaults). |
| `disabledFlags` | string array | `[]` | Explicitly disabled feature flags (wins over `enabledFlags`). |

### Overlays

The `overlays` value is an `OverlayConfig` from `crates/capto-overlay/src/lib.rs`. Fields in brackets are defaults.

| Key | Shape |
|-----|-------|
| `overlays.mouseClicks` | `{ enabled: true, leftColor: "#FF5252", rightColor: "#448AFF", middleColor: "#69F0AE", radius: 18 }` |
| `overlays.keystrokes` | `{ enabled: true, position: { anchor: "bottomLeft", x: 0.05, y: 0.9 }, fontSize: 28, color: "#FFFFFF", background: "#000000AA" }` |
| `overlays.elapsed` | Deprecated; kept for settings JSON compatibility, not burned into recordings. Default disabled with top-right position. |
| `overlays.texts` | Array of `TextOverlay` (`id`, `text`, `fontSize`, `color`, `position`, `enabled`). |
| `overlays.images` | Array of `ImageOverlay` (`id`, `path`, `width`, `height`, `position`, `opacity`, `enabled`). |
| `overlays.webcam` | `{ enabled: false, deviceId: null, deviceLabel: null, position: { anchor: "bottomRight", x: 0.82, y: 0.78 }, width: 320, height: 240, mirrored: true, cornerRadius: 12 }` |

An `overlays.*.position` is an `OverlayPosition`: `anchor` (`topLeft`, `topRight`, `bottomLeft`, `bottomRight`, `center`, `custom`) plus normalized `x` and `y` (0..1). The default anchor is `bottomRight` at `(0.85, 0.85)`. See [capto-overlay](../crates/capto-overlay.md).

## How to edit

- Settings tab in the desktop UI.
- `capto config get` (optional key) and `capto config set key=value` (one or more pairs) or `capto config set --json '{...}'`. Keys are camelCase, matching the file; an unknown key returns a usage error. The CLI patches over the control plane (see `crates/capto-cli/src/main.rs`).
- Edit `settings.json` directly and restart the app. Optional fields tolerate omission via `#[serde(default)]`; new or unknown top-level keys are rejected (the schema sets `additionalProperties: false`).

Hotkeys are normalized on load and save by `normalize_hotkeys` in `crates/capto-hooks/src/lib.rs`, which guarantees exactly the four supported actions in order and migrates the legacy `CommandOrControl+Shift+R`-style defaults to `Alt+F5`..`Alt+F8`.

## Feature flags

Flags are read from `enabledFlags` / `disabledFlags` in `settings.json`, resolved against the registry in `crates/capto-core/src/flags.rs`. Precedence: `disabledFlags` beats `enabledFlags` beats the declared default; an unknown name resolves to `false`.

| Flag | Default | Effect |
|------|---------|--------|
| `control-plane-metrics` | `true` | Serves local metrics snapshots at `GET /v1/metrics` (localhost, auth required). |
| `crash-reporting` | `true` | Writes a structured `crash-*.json` report to the config dir on panic. |

See `docs/feature-flags.md` for the lifecycle, and [Data models](data-models.md) for the crash-report shape.

## Other files in the config dir

- `cli-server.json` is the control-plane lockfile (`crates/capto-ipc/src/lockfile.rs`): `pid`, `port`, `token`, `version`. It is app-generated and holds the bearer token the CLI uses to discover and authenticate against the desktop.
- `crashes/crash-*.json` are the local panic reports written by the desktop when `crash-reporting` is enabled.

## Environment variables

Copied from `.env.example`; all are optional. `CAPTO_LOG` is set in code (`apps/desktop/src-tauri/src/lib.rs`).

| Variable | Scope | Purpose |
|----------|-------|---------|
| `CAPTO_APP_PATH` | dev | Path to a pre-built `capto-app.exe` used by `capto` to auto-launch the desktop when the control plane is down. |
| `RUST_LOG` | CLI / desktop | tracing-subscriber env-filter for logging. |
| `CAPTO_LOG` | desktop | Overrides the default desktop log level (for example `CAPTO_LOG=capto=debug`). |
| `RUST_BACKTRACE` | dev | Print full Rust backtraces on panic. |
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | CI only | Signing keys for Tauri updater artifacts; do not set locally. |

## Validating edits

`docs/settings-schema.json` is the machine-readable JSON Schema for `settings.json`, generated to mirror the serialized camelCase `AppSettings` shape. Use it to validate or edit the file before saving.
