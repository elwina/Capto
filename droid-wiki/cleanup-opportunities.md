# Cleanup opportunities

This page records concrete hotspots that are candidates for cleanup and the debt that is deliberately carried. It is the flip side of [Design decisions](background/design-decisions.md): that page documents why things are built a certain way, and this one tracks where the code is heavy and how new debt is kept out. For the raw size and complexity numbers, see [By the numbers](by-the-numbers.md).

The tracked, deliberate refactor backlog lives in `docs/tech-debt.md`. Anything there predates the current quality gates and is bundled with the validation it needs.

## Complexity hotspots

These are the files where complexity concentrates. They are tracked, not free-floating: both entry points below carry grandfathered `eslint-disable-next-line complexity` annotations while the rest of the frontend is capped at 20 by `complexity: ["error", 20]` in `apps/desktop/eslint.config.js`.

| File | Size | What is heavy |
|------|------|---------------|
| `apps/desktop/src/App.tsx` | 1355 non-empty lines | `MainApp` session shell at complexity 94 wiring ~30 state setters to Tauri events; the `refresh` hydration callback at complexity 22 hydrates ~15 settings in one async pass. |
| `apps/desktop/src-tauri/src/lib.rs` | 1196 lines | ~36 `#[tauri::command]` handlers plus tray, hotkeys, crash reporting, and session services in one entry crate; could split handlers per domain. |
| `crates/capto-capture/src/webcam.rs` | 811 lines | Media Foundation plumbing for webcam capture and PiP. |
| `crates/capto-core/src/session.rs` | 692 lines | The boot and stop paths for a recording pipeline, plus fallback logic and the faststart remux. |

The planned fixes are recorded in `docs/tech-debt.md`: split `MainApp` into per-domain loaders (extract event-handler bundles, cap complexity at 20 as blocks land) and split `refresh` into `loadDisplays` / `settings` / `encoders` helpers. Both are deferred because a correct refactor needs live WGC/WASAPI recording to validate, which CI cannot provide.

## Dead ends: already prevented

Unused exports and dead paths are mostly guarded by tooling rather than found by hand:

- TypeScript unused exports are covered by **knip**, which runs as a frontend CI guard.
- Rust dead code is covered by **`cargo clippy` with `deny(warnings)`** in CI.
- The repo enforces a **zero-marker policy**: no `TODO`, `FIXME`, `HACK`, or `XXX` may exist in tracked source. `scripts/scan-tech-debt.ps1` greps source for those markers (written split so the scanner never flags itself) and fails CI if any appear. There is no exemption comment; the fix is to resolve the debt.
- Feature flags are **scan-guarded**: `scripts/scan-dead-flags.ps1` fails CI if a declared flag in `crates/capto-core/src/flags.rs` is never referenced by runtime code, so stale toggles cannot accumulate.

So while the four files above are heavy, undeclared dead code and ad-hoc markers are actively blocked at merge rather than left for cleanup.

## Dependency freshness

Dependency updates are driven by **Renovate** with a 3-day minimum release age (`minimumReleaseAge: "3 days"` in `renovate.json`) so freshly published releases stabilize before being proposed. Updates are grouped by manager and type (`chore(deps)`, Rust workspace patch/minor, npm devDependency patch/minor), and `lockFileMaintenance` is enabled (no automerge). Renovate uses `Asia/Shanghai` as its timezone.

Freshness pressure is low because the stack is recent:

- Tauri 2 ecosystem and React 19 (`"react": "^19.1.0"`, `"@tauri-apps/api": "^2"` in `apps/desktop/package.json`).
- axum 0.8 (lockfile pins `axum` 0.8.9) for the localhost control plane.
- Both `Cargo.lock` and `package-lock.json` are kept in sync by the same update flow.

One note: the FFmpeg sidecar is pinned externally at `elwina/capto-ffmpeg` tag `v1.0.0-n9.0` (`.github/capto-ffmpeg.env`) and updates on its own release cadence, independent of app version bumps. It is a deliberate, pinned dependency, not a freshness gap.

For the dependency landscape, see the crate pages under [Crates](crates/index.md) and the workspace Cargo.toml.

## Tracked debt

`docs/tech-debt.md` keeps two deliberate entries, each bundled with the validation it needs:

1. **`MainApp` session shell** (`apps/desktop/src/App.tsx`) - complexity 94, a session-orchestration monolith. Deferred because correct refactoring needs live recording to validate. Plan: split per-domain loaders, extract event-handler bundles.
2. **`refresh` hydration callback** (`apps/desktop/src/App.tsx`) - complexity 22, hydrates ~15 independent settings in one async pass. Plan: split into `loadDisplays` / `settings` / `encoders` helpers.

Both carry a grandfathered complexity disable while the rest of `apps/desktop/src` is capped at 20, and new functions exceeding the cap will fail `npm run lint`.

## What CI already prevents

New code that would normally add to this list is blocked at merge by the repo-hygiene gates:

- Zero-debt marker policy (`scripts/scan-tech-debt.ps1`) - no `TODO` / `FIXME` / `HACK` / `XXX`.
- File-size gate (`scripts/check-file-size.ps1`) - no oversized source files.
- Dead-flag gate (`scripts/scan-dead-flags.ps1`) - every declared flag still used.
- Duplicate gate (`npm run duplicate:check`, jscpd) and bundle-size budget gate (`npm run bundle-size`) for the frontend.
- Version-drift guard (`scripts/check-version-drift.ps1`) keeping `package.json` and `tauri.conf.json` in lockstep.

So the hotspots above are a fixed, tracked set, and the expectations for new contributions are that they do not recreate them.
