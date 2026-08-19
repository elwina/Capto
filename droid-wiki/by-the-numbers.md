# By the numbers

Data collected on 2026-08-19.

A data snapshot of the Capto repository: line counts, activity, and complexity. This page measures the codebase and stays out of interpretation. All line counts use non-empty lines as counted by `Measure-Object -Line`. Paths were collected from the current working tree only.

## Size

Capto is a Rust-heavy monorepo. Rust makes up most of the code, followed by the TypeScript and TSX frontend, with small JavaScript, CSS, HTML, and PowerShell layers.

```mermaid
xychart-beta
    horizontal
    title "Lines of code by language"
    x-axis ["Rust", "TypeScript/TSX", "JavaScript", "CSS", "HTML", "PowerShell"]
    y-axis "Lines" 0 --> 11000
    bar [10662, 3355, 1135, 1300, 790, 974]
```

| Language | Files | Lines |
|----------|-------|-------|
| Rust | 44 | 10662 |
| TypeScript / TSX | 22 | 3355 |
| JavaScript / MJS | 7 | 1135 |
| CSS | 1 | 1300 |
| HTML | 1 | 790 |
| PowerShell | 16 | 974 |

Where the code lives:

| Area | Lines | Files |
|------|-------|-------|
| Rust crates (`crates/`) | 8161 | 37 |
| Tauri app (`apps/desktop/src-tauri/`) | 2501 | 7 |
| Frontend (`apps/desktop/src/`) | 3355 | 22 |
| Packages (`packages/*/src`, `cloudflare/`) | 1135 | 7 |
| Website (`website/`) | 2090 | 2 |
| Scripts (`scripts/`) | 974 | 16 |

Source versus test versus config, a rough split. About 91 source files carry the code above. Three are dedicated test files (the frontend `*.test.ts` and `*.test.tsx` suites), and 13 Rust files embed inline `#[cfg(test)]` modules. Config is spread across the root and per-member `Cargo.toml` files, the desktop `tauri.conf.json`, and three `package.json` files (desktop plus the two packages). Lockfiles and build output are excluded throughout.

There are 9 workspace members in `Cargo.toml`: 8 crates (`capto-audio`, `capto-capture`, `capto-cli`, `capto-core`, `capto-encode`, `capto-hooks`, `capto-ipc`, `capto-overlay`) plus the Tauri app crate `capto-app`. There are 2 npm packages.

## Activity

The repository is solo-maintained. Per-person contributor stats are omitted by policy for this wiki, so there are no contributor leaderboards here.

All history falls inside the last 90 days. There are 62 commits total.

| Milestone | Date | Event |
|-----------|------|-------|
| First commit | 2026-08-05 | Initial scaffolding of Capto for cloud agent development |
| Build-out | 2026-08-09 to 2026-08-13 | Initial feature build-out (29 commits) |
| Hardening | 2026-08-19 | Large hardening day (30 commits) |

Commits by day: Aug 5 had 1, Aug 9 had 9, Aug 10 had 13, Aug 11 had 2, Aug 12 had 3, Aug 13 had 4, and Aug 19 had 30. The Aug 19 burst was the biggest single-day push, covering observability, CI hardening, repo hygiene gates, and frontend tests.

Tags track the release line from `v0.1.0` through `v1.0.0`, with a rolling `updater` tag for the signed update mirror. The current stable version is 1.0.0.

Churn hotspots in the last 90 days (which is the whole history). Lockfiles are filtered out:

| Files | Touched |
|-------|---------|
| `AGENTS.md` | 21 |
| `docs/CI.md` | 16 |
| `.github/workflows/ci.yml` | 16 |
| `apps/desktop/package.json` | 13 |
| `apps/desktop/src-tauri/tauri.conf.json` | 12 |
| `README.md` | 12 |
| `apps/desktop/src-tauri/src/lib.rs` | 10 |
| `apps/desktop/src/App.tsx` | 8 |
| `packages/capto-agent-skill/skills/capto/SKILL.md` | 8 |

## Bot-attributed commits

Searching commit bodies for `Co-authored-by` lines containing `[bot]` finds 26 commits with bot attribution, all from `factory-droid[bot]`. This is a lower bound: the count only reflects commits that carry the trailer in their message. No `dependabot[bot]` trailers appear in history so far.

## Complexity

Average file size per directory, measured in non-empty lines:

| Directory | Files | Total lines | Average |
|-----------|-------|-------------|---------|
| `apps/desktop/src-tauri/src` | 6 | 2498 | 416 |
| `crates/capto-encode` | 2 | 630 | 315 |
| `crates/capto-capture` | 10 | 2610 | 261 |
| `crates/capto-core` | 9 | 1985 | 221 |
| `crates/capto-cli` | 5 | 1086 | 217 |
| `apps/desktop/src` | 22 | 3355 | 153 |

Largest source files:

| File | Lines |
|------|-------|
| `App.tsx` | 1355 |
| `lib.rs` (Tauri) | 1196 |
| `webcam.rs` | 811 |
| `session.rs` | 692 |
| `windows.rs` (audio) | 653 |
| `lib.rs` (encode) | 610 |
| `ffmpeg_args.rs` | 481 |
| `main.rs` (CLI) | 477 |
| `record_dxgi.rs` | 462 |
| `cli_server.rs` | 436 |

The largest files cluster in the capture and session orchestration layers, where recording and encoding logic naturally accumulates, alongside the desktop app entry point and control-plane server.

## Related pages

- [Lore](lore.md) tells the story behind these numbers
- [Cleanup opportunities](cleanup-opportunities.md) turns the complexity findings here into actionable work
- [Architecture](overview/architecture.md) shows how the pipeline these files implement is organized
