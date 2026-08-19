# Development workflow

This page walks through the day-to-day cycle: branch, code, test, push, review. It is the practical companion to [Getting started](../overview/getting-started.md), which lists every command once. Here the emphasis is on the inner loop you run while working, the local gates you use before pushing, and the release flow at the end.

## The purpose of this page

This page describes how a change moves from an idea on `main` to a merged commit and then to a release. It covers the setup script, the inner loop of edit and test, the opt-in pre-commit hooks, the QA smoke scripts, and the release flow. For the exact command list, see [Getting started](../overview/getting-started.md); for the CI and release pipeline in depth, see [Deployment](../deployment.md).

## Setting up the environment

A fresh clone becomes runnable with one command:

```powershell
.\scripts\setup-dev.ps1     # or -Local to prefer a local ffmpeg.exe
```

`setup-dev.ps1` installs the frontend dependencies, downloads the pinned FFmpeg sidecar (with attestation verification) or copies a local build, then builds and stages the CLI into the app bundle. Every working copy needs the FFmpeg sidecar and a staged `capto.exe`, because the Tauri build fails if `apps/desktop/src-tauri/binaries/capto.exe` is missing. See [Getting started](../overview/getting-started.md) for the step-by-step.

## The inner loop

While you work you bounce between Rust and TypeScript. The core commands are:

```powershell
cargo test --workspace                          # all Rust unit tests
cargo fmt --all --check                         # rustfmt gate
cargo clippy --workspace --all-targets          # clippy (warn-only in CI today)

npm run lint --prefix apps/desktop              # ESLint, max-warnings 0
npm run format:check --prefix apps/desktop      # Prettier
npm test --prefix apps/desktop                  # Vitest unit tests
```

When you change the frontend, run the full local set before pushing: `npm run lint`, `npm run format:check`, `npm run knip`, and `npm run test:coverage`. The coverage run is the strictest because it enforces the V8 thresholds in `apps/desktop/vitest.config.ts` (80% lines, 75% functions, 70% branches, 80% statements across the modules it covers).

For a manual run of the app so you can touch a real recording session:

```powershell
npm run tauri --prefix apps/desktop -- dev
```

Then drive it from the CLI control plane (`capto status`, `capto record start`, `record stop`, `outputs recent`). If you set `CAPTO_APP_PATH` to `target/debug/capto-app.exe`, the `capto` binary can auto-launch the dev desktop when the control plane is down.

## Repo-hygiene gates

Before you open a PR, run the hygiene scanners too. CI runs all of them in the `hygiene` job (`scripts/check-file-size.ps1`, `scripts/scan-tech-debt.ps1`, `scripts/scan-pii.ps1`, `scripts/validate-agents-md.ps1`, `scripts/check-version-drift.ps1`, `scripts/scan-dead-flags.ps1`). Passing them locally saves you a wasted CI run. The tools and what each fails on are listed in [Tooling](tooling.md).

The two you will hit most often while writing code are:

```powershell
.\scripts\scan-tech-debt.ps1     # fails on any TODO/FIXME/HACK/XXX in source
.\scripts\check-file-size.ps1    # fails on oversized source files
```

## Pre-commit hooks

Hooks are opt-in per clone. Install them once:

```powershell
.\scripts\install-hooks.ps1      # sets core.hooksPath to .githooks
```

The hook routes to `scripts/pre-commit.ps1` through whatever PowerShell is available (see `.githooks/pre-commit`). It is deliberately lightweight and runs only the cheap, high-frequency gates: file size, tech-debt markers, `cargo fmt --all --check`, and the frontend lint, format, and knip checks when `apps/desktop` sources are actually staged. Full suites (cargo test, Vitest, coverage) stay in CI. A failing hook blocks the commit with exit code 1; use `git commit --no-verify` to bypass once.

## QA smoke scripts

For interactive verification against a real desktop session, the repo provides scripted probes. See [Testing](testing.md) for what each covers. The quick loop is:

```powershell
.\scripts\qa-smoke.ps1                     # doctor/status/config/outputs
.\scripts\qa-smoke.ps1 -RunRecordRoundtrip # + real record -> stop -> outputs
```

CI skips the record round-trip because runners have no display session; a real Windows desktop runs it as part of validating a change.

## Release flow

Releases are tag-driven and separate from CI. Green CI does not publish; only a tag does.

1. Bump the version in lockstep. `scripts/check-version-drift.ps1` verifies that `apps/desktop/package.json` and `apps/desktop/src-tauri/tauri.conf.json` agree. This must pass before you tag, or the release build produces a split-brained version.
2. Push a `v*` tag. Stable releases are `v1.*`; the `v0.*` tags were prereleases.
3. `.github/workflows/release.yml` builds NSIS installers for x64 and arm64 with the embedded FFmpeg sidecar and staged `cli\capto.exe`, signs the updater artifacts, and mirrors `latest.json` onto the rolling `updater` tag so the in-app update URL stays stable.
4. The changelog job appends commit subjects since the previous release to the release notes.

For staged rollout, there is a canary channel: publish an experimental build to a rolling `canary` tag, point pilot agents at the worker-served `canary.json`, validate, then promote to the stable `updater` manifest. A bad canary never reaches stable users and can be pulled by deleting the tag. Full detail on the pipeline lives in [Deployment](../deployment.md).

## Related pages

- [How to contribute](index.md)
- [Testing](testing.md)
- [Debugging](debugging.md)
- [Tooling](tooling.md)
- [Getting started](../overview/getting-started.md)
- [Deployment](../deployment.md)
