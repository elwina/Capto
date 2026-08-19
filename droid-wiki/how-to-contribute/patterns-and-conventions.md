# Patterns and conventions

Capto's conventions fall out of its three non-negotiables: local-only, one session, and encoding only through the bundled sidecar. This page collects the patterns you will see across the Rust crates, the Tauri shell, the React UI, and the agent tooling.

## Repository-level rules

- **No upload SDKs.** Product code never talks to a cloud service for content. The only network touchpoints are the updater (`tauri-plugin-updater` against GitHub releases, optionally mirrored by the Cloudflare worker) and the release-side tooling in `.github/`.
- **Encoding only via `capto-encode`.** Never spawn FFmpeg ad-hoc from UI crates; the sidecar is the single encode path.
- **New capture backends implement `CaptureBackend`** in `crates/capto-capture`. Windows is first; macOS/Linux backends stay `UnsupportedCaptureBackend` stubs.
- **The UI does not process frames.** React sends intents and renders state. Native code produces JPEG previews (`Frame::preview_jpeg` in `crates/capto-capture/src/lib.rs`) and sends them to the browser only as compressed stills.
- **One session per machine.** The desktop is single-instance; the CLI is a control-plane client and never creates a second `RecordingSession`.
- **Binary naming.** `capto` = CLI crate, `capto-app` = desktop crate. Never name both `capto` (they would clash in `target/debug` and on case-insensitive Windows paths).

## Rust conventions

- Workspace layout: shared dependencies declared once in the root `Cargo.toml` `[workspace.dependencies]`; crates reference them via `{ workspace = true }`.
- Error handling: each crate defines a `thiserror` error enum plus a `pub type Result<T> = std::result::Result<T, Error>` alias. `capto_core::CoreError` wraps lower-level errors with `#[from]` (see `crates/capto-core/src/lib.rs`).
- Session state is guarded by `tokio::sync::Mutex`; snapshot-style accessors return copies (`SessionSnapshot`) rather than exposing internals.
- FFmpeg argv construction is centralized in `crates/capto-core/src/ffmpeg_args.rs`; geometry clamping and region resolution happen before spawn, with a small set of diagnostics (`slow_writes`, stderr tail capture) that flow into errors so failures are explainable.
- Logging uses `tracing` with a `capto_*` target convention; the desktop filter default is `capto=debug,capto_core=debug,capto_encode=debug,warn` (see `init_tracing` in `apps/desktop/src-tauri/src/lib.rs`), and `RUST_LOG` governs the CLI.
- Encoders, formats, and config values serialize as camelCase (`#[serde(rename_all = "camelCase")]`) everywhere, so JSON on the wire matches settings keys.

## Frontend conventions

- React 19 function components, TypeScript strict, single-page app (no router); tabs are `main` (source), `webcam`, `overlays`, `settings`, `about` in `apps/desktop/src/App.tsx`.
- All user-facing strings go through i18next (`apps/desktop/src/i18n/index.ts`) with 10 locale files under `apps/desktop/src/i18n/locales`; hard-coded UI strings fail review.
- Tauri commands are declared in `apps/desktop/src-tauri/src/lib.rs` and invoked via `@tauri-apps/api`; arguments use camelCase DTOs (e.g. `StartArgs` mirroring `RecordStartRequest`).
- Complexity cap: `complexity: ["error", 20]` in `apps/desktop/eslint.config.js`. Two grandfathered exceptions (`MainApp` shell and its `refresh` hydration callback) carry `eslint-disable-next-line complexity` and are tracked in `docs/tech-debt.md`.
- Visual logic (overlay previews, picker positioning) stays in components; layout math that Rust also needs is duplicated in `crates/capto-overlay/src/lib.rs` (`resolve_pixel_position`).

## JSON contract conventions

- CLI stdout is always the envelope `{ ok, data | error }` (`crates/capto-ipc/src/envelope.rs`); human output goes only to `--human`.
- Exit codes (0–6) are stable and documented in `docs/CLI.md`; branch on exit code first, then `error.code`.
- `data` fields are camelCase; unknown settings keys return exit `1` (`usage`).
- The Bearer token from `cli-server.json` must never be logged; `crates/capto-ipc/src/redact.rs` scrubs sensitive values from logs.

## Testing conventions

- Rust: `cargo test --workspace`; unit tests sit next to code (`#[cfg(test)] mod tests`). There are no integration tests outside the CLI smoke suite; real capture/encode needs a Windows desktop session and is covered manually via `scripts/qa-smoke.ps1` (see [QA.md](../../docs/QA.md)).
- Frontend: Vitest + Testing Library under `apps/desktop/src`; `npm run test:coverage` enforces coverage thresholds in CI.
- npm package tests use fixture CLIs speaking the envelope contract: `packages/capto-dsh-plugin/test/fixtures/fake-capto.mjs` fakes `capto` so the suite runs without a desktop.
- CI gates beyond tests: rustfmt `--check`, ESLint `--max-warnings 0`, Prettier, jscpd duplicate detection, knip unused-code check, bundle-size budget, V8 coverage thresholds.

## Repo hygiene

Scripts in `scripts/` are enforced in CI and should pass locally before pushing (see [How to contribute](index.md)):

- `scripts/check-file-size.ps1`, no oversized source files
- `scripts/scan-tech-debt.ps1`, zero TODO/FIXME/HACK/XXX markers in source
- `scripts/check-version-drift.ps1`, `package.json` and `tauri.conf.json` versions in lockstep
- `scripts/scan-dead-flags.ps1`, every declared feature flag in `crates/capto-core/src/flags.rs` is referenced by runtime code
- `scripts/scan-pii.ps1`, redaction coverage scanner
- `scripts/install-hooks.ps1`, opt-in local pre-commit hooks running the fast subset

Deliberate, tracked debt lives in `docs/tech-debt.md` with its required validation; new debt is not accepted silently.
