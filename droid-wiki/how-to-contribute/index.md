# How to contribute

Capto is a solo-maintained, local-only project. Its Git history is a straight line on `main` and PRs are written, reviewed, and merged by the same maintainer. That does not mean review is skipped: the CI pipeline treats every change equally and is the real gatekeeper. This page explains how to find work, what a pull request looks like, and what "done" means. If you want to contribute a change, start here.

## The purpose of this page

This is the onboarding entry point for contributors. It covers three things: where to find work, how a pull request flows, and the exact checks a change must pass before it is accepted. Process and tooling details live on the sibling pages in this section, so this page stays short and points at them instead of repeating their lists.

## Finding work

Because the project is a monorepo with a single maintainer, work is tracked in a few well-known places rather than a sprawling board.

- GitHub issues. Bug reports and feature requests use the templates in `.github/ISSUE_TEMPLATE/` (`bug_report.md` and `feature_request.md`). The templates keep each issue actionable with environment and repro fields.
- The feature matrix in `README.md`. It marks P0 (MVP) and P1 (next) features, and its cut list records things deliberately left out. Before proposing a feature, check the cut list so you do not re-litigate a permanent decision.
- The tracked backlog in `docs/tech-debt.md`. This is not a bug list. It records deliberate, oversized debt (for example the `MainApp` session shell in `apps/desktop/src/App.tsx`) bundled with the validation each item needs before it can be split up. These items are large; agree the approach with the maintainer before starting one.

If you are unsure whether an idea fits, read `docs/ARCHITECTURE.md` and the non-negotiables in `AGENTS.md` first. Capto is strictly local, encodes only through the bundled sidecar, and the React UI never processes frames. Anything that violates these needs to be argued before it is written.

## The pull request process

The flow is the standard branch-push-PR loop. There is no forked contribution model and no trunk-based ceremony beyond it.

1. Pick an issue or backlog item and create a branch from `main`.
2. Make a focused change. Keep the PR small enough that the CI gates and a human review can both come to a clear conclusion.
3. Fill out the template in `.github/pull_request_template.md`. It asks for a description, a bullet list of changes, the exact commands you ran to verify, and checkboxes for the core gates (`cargo test --workspace`, `cargo fmt --all --check`, `npm test`, `npm run lint`).
4. Push and open the pull request. `CODEOWNERS` assigns review to `@elwina`, and the Droid workflows (`.github/workflows/droid.yml` and `droid-review.yml`) run an automated code and security review when the `FACTORY_API_KEY` secret is present.

Nothing is required to succeed in CI for the PR to be mergeable in the sense of branch protection: there are no required checks configured. In practice the maintainer treats green CI as the definition of progress, so treat the gates as mandatory regardless of what branch protection formally requires.

## Review expectations

The maintainer reviews for the same quality the tooling enforces, plus a few things automation cannot check.

- All CI gates are green. That means rustfmt and clippy for Rust, the frontend lint, format, coverage, and bundle-size gates, and the full workspace test suite. The exact commands live in `development-workflow.md`.
- No new tech debt. `scripts/scan-tech-debt.ps1` fails on any `TODO`, `FIXME`, `HACK`, or `XXX` in source, so ad-hoc markers are not accepted. If a problem really must be deferred, it is tracked in `docs/tech-debt.md` with its validation, not left as a comment.
- No hard-coded UI strings. All user-facing text goes through the i18n layer under `apps/desktop/src/i18n/`; a string dropped directly into a component fails review.
- The change stays within the architectural rules in `AGENTS.md`, especially the "no upload" rule and the single encode path through `capto-encode`.

## Definition of done

A contribution is done when all of the following hold:

- Rust and frontend test suites pass locally, and CI reports green on `cargo test --workspace`, `cargo test --all` checks, and the frontend `npm test` and coverage runs.
- `cargo fmt --all --check` passes and `npm run lint --prefix apps/desktop` passes with zero warnings.
- The repo-hygiene gates pass: no `TODO`/`FIXME`/`HACK`/`XXX` markers, no oversized source files, and no unexplained version drift between `apps/desktop/package.json` and `apps/desktop/src-tauri/tauri.conf.json`.
- Documentation is updated where the change touches behavior. Claims about the codebase cite real repo-root file paths, and user-visible or CLI-facing changes update `docs/CLI.md`.
- Feature flags stay consistent: every flag declared in `crates/capto-core/src/flags.rs` is still referenced, so `scripts/scan-dead-flags.ps1` passes.

If a change also alters the app or workspace version, run `scripts/check-version-drift.ps1` before opening the PR so the npm version and the Tauri config version stay in lockstep.

## Related pages

- [Development workflow](development-workflow.md)
- [Testing](testing.md)
- [Debugging](debugging.md)
- [Tooling](tooling.md)
- [Patterns and conventions](patterns-and-conventions.md)
- [Getting started](../overview/getting-started.md)
- [Deployment](../deployment.md)
