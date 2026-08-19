# Tooling

This page catalogs the tools that keep Capto green: the build system, the linters and quality gates, the repo-hygiene scanners, the CI workflows, the dependency updater, and the dev container. It is the reference counterpart to the process pages in this section.

## The purpose of this page

This page is a directory of the toolchain. It explains what each tool does and where it is configured, with the assumption that you have already read [index.md](index.md) and [development-workflow.md](development-workflow.md) for the actual commands and the inner loop. Nothing here replaces those pages; it gives you the map instead.

## Build system

Capto is a Cargo workspace plus a Vite frontend, glued together by the Tauri CLI.

- Rust crates live under `crates/` (and the `apps/desktop/src-tauri` crate for the desktop). Shared dependencies are declared once in the root `Cargo.toml` `[workspace.dependencies]` and referenced with `{ workspace = true }`.
- The frontend in `apps/desktop` builds with Vite; `npm run build` runs `tsc` then `vite build`. Topics are the `dist/` output and the bundle report.
- `npm run tauri --prefix apps/desktop` runs the Tauri CLI, which orchestrates building the Rust side, bundling the FFmpeg external binary and the staged CLI, and producing the NSIS installer. Packaging requires `apps/desktop/src-tauri/binaries/ffmpeg-<triple>.exe` and `apps/desktop/src-tauri/binaries/capto.exe`, which is why the setup script stages them.

## Linters and formatters

| Tool | What it guards | Configuration |
|------|----------------|---------------|
| `rustfmt` | Rust formatting; `cargo fmt --all --check` is a CI gate | project defaults, `rustfmt.toml` if present |
| `clippy` | Rust lints; run with `cargo clippy --workspace --all-targets` | warn-only in CI today, per `.github/workflows/ci.yml` |
| ESLint | Frontend lint with `--max-warnings 0`; includes the cyclomatic-complexity cap of 20 (`complexity: ["error", 20]`), a blanket `no-explicit-any` error, and react-hooks rules | `apps/desktop/eslint.config.js` |
| Prettier | Frontend formatting; `npm run format:check` is the CI gate | `.prettierrc` under `apps/desktop` |
| knip | Dead code and unused dependencies in the frontend | `npm run knip`, on by default |

The complexity cap is worth calling out. `apps/desktop/src/App.tsx` has two grandfathered functions (`MainApp` and its `refresh` hydration callback) that carry `eslint-disable-next-line complexity` and are tracked in `docs/tech-debt.md`. Any function you add that exceeds 20 will fail `npm run lint`. Knock complexity down with small helpers instead of suppression.

## Quality gates

Beyond linting and tests, CI enforces three frontend-specific gates:

- **jscpd** (`npm run duplicate:check`) flags copy-pasted code; the `jscpd.json` at `apps/desktop/jscpd.json` configures thresholds and excludes.
- **Bundle-size budget** (`npm run build:analyze` + `npm run bundle-size`). `apps/desktop/scripts/bundle-size.mjs` reads the rollup raw-data report (`dist/stats.json`), sums the emitted asset sizes, and fails if they exceed the `MAX_BUDGET_BYTES` baseline. The visualizer HTML in `dist/analyze.html` shows which dependency dominates, so a regression is easy to attribute.
- **Coverage** (`npm run test:coverage`) enforces the V8 thresholds in `apps/desktop/vitest.config.ts` (80% lines, 75% functions, 70% branches, 80% statements) over the modules it includes. See [testing.md](testing.md).

## Repo-hygiene scanners

The scripts in `scripts/` run in the CI `hygiene` job and should pass locally before pushing (enforced; the same run happens via the pre-commit hook for the cheap subset). They are PowerShell, not bash.

| Script | Fails when |
|--------|-----------|
| `scripts/check-file-size.ps1` | a source file is oversized |
| `scripts/scan-tech-debt.ps1` | a `TODO`, `FIXME`, `HACK`, or `XXX` marker appears in source |
| `scripts/scan-pii.ps1` | emails, SSNs, card numbers, or private keys slip into committed text |
| `scripts/validate-agents-md.ps1` | `AGENTS.md` breaks its expected structure |
| `scripts/check-version-drift.ps1` | `apps/desktop/package.json` and `apps/desktop/src-tauri/tauri.conf.json` versions diverge |
| `scripts/scan-dead-flags.ps1` | a feature flag declared in `crates/capto-core/src/flags.rs` is no longer referenced by runtime code |
| `scripts/install-hooks.ps1` | (installer, not a scan) sets `core.hooksPath` to `.githooks` |

## CI workflows

The GitHub Actions workflows live in `.github/workflows/`. Summary of the main ones; full detail is in [Deployment](../deployment.md).

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | push/PR to `main` | Rust test + clippy + build timings, frontend tsc/lint/jscpd/knip/bundle-size/coverage, `cargo check` for x64 and arm64 with attestation-verified FFmpeg, packages tests, devcontainer build |
| `release.yml` | tag `v*` or manual | NSIS installers for both arches, signed updater artifacts, rolling `latest.json` |
| `pages.yml` | push to `main` | deploy `website/` + rustdoc as one Pages artifact |
| `droid.yml`, `droid-review.yml` | `@droid` / PRs | Factory Droid code + security review (needs `FACTORY_API_KEY`) |
| `codeql.yml`, `secret-scan.yml` | push/PR | static security scanning |
| `ci-alert.yml` | `workflow_run` | open/update a `[build-health]` issue when CI/Release/Pages fails |

CI and release are deliberately separate: green CI never publishes; only a `v*` tag triggers a release.

## Dependencies

`renovate.json` configures Renovate with `config:recommended`, a 3-day `minimumReleaseAge`, an `Asia/Shanghai` timezone, and `chore(deps)` commit messages. Rust workspace patch/minor updates are grouped under one label as are npm patch/minor `devDependencies`, and lockfile maintenance is enabled but never auto-merged. The dependency landscape (what each dependency is for) lives in [Dependencies](../reference/dependencies.md).

## Dev container

`.devcontainer/devcontainer.json` provides a Linux development environment (Ubuntu base with Rust, Node 24, and PowerShell features plus VS Code extensions for Rust, ESLint, Prettier, and GitHub Actions). It is a real, continuously verified build: CI builds the container and smoke-tests the toolchain (`cargo`, `node`, `pwsh` versions). Note the container is for editing and tooling only; compiling the native capture crates needs OS audio/display headers it cannot provide, so Windows-specific verification still happens on a Windows machine.

## Windows-first scripts

All repository scripts are PowerShell (`*.ps1`), not bash. The pre-commit hook at `.githooks/pre-commit` is a thin shim that resolves to `scripts/pre-commit.ps1` through whatever PowerShell is available (pwsh, or `powershell.exe` under Git Bash). If you add automation to the repo, keep it in PowerShell and follow the same convention so it runs on the Windows machines that matter for capture and packaging.

## Related pages

- [How to contribute](index.md)
- [Development workflow](development-workflow.md)
- [Testing](testing.md)
- [Debugging](debugging.md)
- [Deployment](../deployment.md)
- [Dependencies](../reference/dependencies.md)
