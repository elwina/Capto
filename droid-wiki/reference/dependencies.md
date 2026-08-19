# Dependencies

Capto is a Rust workspace plus a React frontend. Its only heavyweight external binary is the pinned FFmpeg sidecar; everything else is a normal library dependency. There are no databases, message queues, or cloud SDKs anywhere in the product.

## Rust workspace dependencies

Declared in the root `Cargo.toml` `[workspace.dependencies]` block. Twenty entries: 13 third-party crates plus 7 internal path deps (`capto-core`, `capto-capture`, `capto-audio`, `capto-encode`, `capto-overlay`, `capto-hooks`, `capto-ipc`).

| Crate | Version | Used for |
|-------|---------|----------|
| `serde` / `serde_json` | 1.x | Serialization of every data model (camelCase). |
| `thiserror` | 2.x | Error enums (for example `CaptureError`, `CoreError`). |
| `anyhow` | 1.x | CLI error plumbing. |
| `tokio` | 1.x | Async runtime (rt, sync, macros, time, process, io, fs). |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 | Structured logging; env-filter for `RUST_LOG` / `CAPTO_LOG`. |
| `chrono` | 0.4 | Local timestamps in output filenames. |
| `uuid` | 1.x | Random suffix in `capto-*` output names. |
| `axum` | 0.8 | Localhost control-plane HTTP server. |
| `tower` | 0.5 | Middleware (auth, metrics) for the control plane. |
| `http` | 1.x | HTTP types shared with axum/handlers. |
| `reqwest` | 0.12 | CLI and desktop HTTP client; `rustls-tls`, default-features off. |

### Notable per-crate dependencies

| Crate | Notable deps |
|-------|--------------|
| `capto-app` (`apps/desktop/src-tauri/Cargo.toml`) | `tauri` 2 (tray-icon) plus plugins `opener`, `global-shortcut`, `dialog`, `single-instance`, `process`, and `updater` (updater only under `cfg(any(macos, windows, linux))`). |
| `capto-capture` | `image` 0.25 (png/jpeg), `xcap` 0.0.14, and on Windows `windows` 0.58 (MediaFoundation etc.) plus `windows-capture` 2.0. |
| `capto-hooks` | `windows` 0.58 on Windows (LL mouse/keyboard hooks). |
| `capto-audio` | `cpal` 0.15, and `wasapi` 0.23 on Windows. |
| `capto-cli` | `clap` 4 (derive). |
| `capto-encode` | No heavyweight third-party deps; wraps the FFmpeg sidecar. |
| `tempfile` | Dev-dependency used by tests in `capto-core` and `capto-app`. |

## Frontend npm dependencies

`apps/desktop/package.json` (unit-test and lint tooling listed under dev).

| Kind | Packages |
|------|----------|
| Runtime | `react` 19, `react-dom`, `i18next`, `react-i18next`, `@tauri-apps/api` and plugins `dialog`, `opener`, `process`, `updater` |
| Build / dev | `vite` 7, `vitest` 4, `@vitest/coverage-v8`, `typescript` ~5.8, `eslint` 9 (`@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`), `prettier`, `jscpd`, `knip`, `globals`, `@testing-library/react` and `@testing-library/dom`, `jsdom`, `rollup-plugin-visualizer`, `@vitejs/plugin-react`, `@types/react`/`react-dom`, `@tauri-apps/cli` |

## npm packages in the repo

| Package | Runtime deps |
|---------|--------------|
| `packages/capto-agent-skill` | Zero runtime dependencies; ships an Agent Skills doc (`skills/capto/SKILL.md`) teaching agents the doctor → record → stop → outputs workflow. |
| `packages/capto-dsh-plugin` | `@deepseek-ai/dsh-tools` and `@deepseek-ai/schemastery` (DeepSeek Harness tool typing). |

## External platform dependencies

- The only heavyweight external binary is the FFmpeg sidecar, pinned via `.github/capto-ffmpeg.env` to `elwina/capto-ffmpeg` tag `v1.0.0-n9.0`. Encoding goes only through this bundle (`crates/capto-encode/`), never a system `PATH` ffmpeg. See [Updater and FFmpeg](../features/updates.md).
- Windows APIs are reached through the `windows` crate (WASAPI audio, DXGI/MediaFoundation capture, low-level input hooks); audio additionally uses `cpal`/`wasapi`. macOS and Linux capture/audio backends are stubs.
- No database, no message queue, and no cloud/upload SDKs. Crash reports, metrics, and breadcrumbs all stay on the local machine.

## Conditional and optional dependencies

| Dependency | Condition |
|------------|-----------|
| `tauri-plugin-updater` | Only under `cfg(any(target_os = "macos", windows, target_os = "linux"))` in `apps/desktop/src-tauri/Cargo.toml`. |
| `windows` crate | `cfg(windows)` in `capto-capture`, `capto-hooks`, `capto-audio`, and `capto-cli`; each enables only the features it needs. |
| `windows-capture` | `cfg(windows)` in `capto-capture`. |
| `wasapi` | `cfg(windows)` in `capto-audio`. |

## Freshness posture

- Renovate is configured in `renovate.json` with `minimumReleaseAge: "3 days"` and groups cargo patch/minor and npm devDependency updates as `chore(deps)` PRs.
- `.github/workflows` and `scripts/check-version-drift.ps1` keep `package.json` and `tauri.conf.json` versions in lockstep; the version drift gate runs in CI.
- `Cargo.lock` and `apps/desktop/package-lock.json` are committed, so the workspace resolves to pinned, reproducible builds.
