# Lore

Capto is a purely local Windows screen recorder, built from scratch in about two weeks by a single maintainer, Elwina Vardal. It is a clean-room "spiritual successor" to the open-source Captura, but shares no code with it. Because one person drives the whole project, history comes in concentrated bursts: a scaffold on day one, a large desktop push a few days later, a string of releases, and one very large hardening day near the end. The commit log tells this story cleanly, and the dates below come straight from `git log --date=short`.

## Era 1: Scaffold (August 5, 2026)

Capto's first commit, "Initial commit: scaffold Capto for cloud agent development", sets up the Tauri 2 + Rust + React/TypeScript workspace skeleton and the `AGENTS.md` conventions that still govern the codebase: no upload SDKs, encoding only through `capto-encode`, new capture backends must implement `CaptureBackend`, and Windows first. The name, the crate layout, and the hard rules about staying local were all in place before any real feature existed. Nothing else was committed for the next four days, which appears to have been quiet architecture planning rather than inactivity.

## Era 2: Desktop MVP (August 9, 2026)

The project's biggest single burst, "feat: prepare Capto desktop MVP", lands capture, settings, and overlays in one day. This is where the xcap-based capture, WASAPI audio, and the FFmpeg sidecar first take shape. Also arriving this day: in-app GitHub updates and an About page, fixes for the region and window pickers on secondary monitors, and the decision to bundle the CLI inside the installer. A splash of other commits on the same day (FFmpeg attestation auth, dropping MSI installers, NSIS-only builds) show the MVP being hardened almost as quickly as it was written.

## Era 3: Agent control plane (August 9, 2026)

Directly on top of the MVP same day comes "feat: add agent-ready CLI control plane for Capto desktop", shortly followed by "feat: rename CLI to capto, add agent skill package and CI/Release workflows". This turns the desktop app into an agent-controllable surface: a localhost HTTP control plane, a JSON envelope contract, and the published `capto-agent-skill` npm package. It is a deliberate architectural bet that people (and agents) will drive Capto through a small, parseable CLI rather than clicking through a UI.

## Era 4: First releases (August 9 to 12, 2026)

The release machine spins up fast. Within a week the repo tags v0.1.0 through v0.5.0 as prereleases, then "Release Capto 1.0.0 with stable channel and FFmpeg sidecar v1.0.0" (August 11) makes the first stable cut. Two recurring obsessions run through this era. First, installer branding and PATH behavior: a saga over August 10 to 12 (release commits plus "Fix NSIS uninstall PATH hooks", then three consecutive NSIS commits on August 12) that only ends once the installer reliably places the CLI at `cli\capto.exe`, preserves long user PATH values through the EnVar plugin, and never overwrites the user's PATH on a failed read or an empty value. Second, the multi-monitor picker, fixed in "Release Capto 0.2.0 with multi-monitor selection fixes" (August 10). A root MIT LICENSE ("Add root MIT LICENSE so the README license badge resolves", August 10) and a winget-publisher parity commit round out the polished release identity.

## Era 5: Site and updater infra (August 10, 2026)

On a single day the distribution backbone appears: a Cloudflare Worker updater mirror for faster update checks and downloads (f77d1b9), the landing page hosted on Cloudflare Pages via Git integration (d03cafc), and a GitHub Actions workflow to deploy the static website to Pages (7ce5b11). Combined with the earlier in-app updates, this gives Capto a small but complete release and download pipeline, all self-hosted and local-first.

## Era 6: Agent ecosystem expansion (August 13, 2026)

A short, focused day in which the "for agents" story matures: "feat(plugin): add capto-dsh-plugin, Capto recording control for DeepSeek Harness", a README agent-install section, and bilingual (English and Chinese) social launch copy. The final commit of the day refocuses that copy ("refocus social copy on the WorkBuddy direct-control selling point"), signaling that the pitch is less "a recorder" and more "a recorder your agent can hand-control".

## Era 7: Hardening (August 19, 2026)

The largest day in the project, responsible for roughly half of all commits. It is almost entirely discipline and gates rather than new features: repo hygiene checks and single-command setup (77c43a2), ESLint and Prettier gates, Vitest unit tests, enforced V8 coverage thresholds, an npm-versus-Tauri version-drift gate, jscpd/bundle-size/timing gates, a devcontainer, Renovate, CODEOWNERS plus PR and issue templates, and opt-in local pre-commit hooks. Observability also lands: local crash breadcrumbs, profiling, usage metrics, and a PII scan (b1506d6), plus a control-plane feature set (request-id tracing, log scrubbing, crash logs, circuit breaker) in 1bca272, CI pipeline alerts with an updater canary channel (4cad94e), and a Pages fix restoring the homepage at the GitHub Pages root (f259fbe). The same day, the Factory Droid workflows arrive via merge PR "add-factory-workflows" (bb2998b), adding `droid.yml` and `droid-review.yml` backed by the `FACTORY_API_KEY` secret.

## Longest-standing features

Three ideas have survived nearly the whole project and still anchor it today. The `settings.json` model and the `RecordingSession` orchestration first show up in the August 9 desktop MVP push and have carried the most churn since, because almost every feature threads through them. The `CaptureBackend` trait, fixed in `AGENTS.md` from the scaffold, keeps abstraction over the WGC/DXGI backends stable. And the CLI JSON envelope contract, born the same day as the MVP, has proven durable enough that later agent-facing work (the skill package, the DeepSeek Harness plugin) layers on top of it instead of replacing it.

## Deprecated features

- ElapsedOverlay: marked deprecated in `capto-overlay` and kept only for settings JSON compatibility. It was never burned into recordings.
- MSI installers: dropped on August 9 ("build: ship NSIS exe installers only").
- A separate CLI release asset: retired. The CLI is now bundled into the installer at `cli\capto.exe` rather than published alone (documented in `docs/CI.md`).
- Legacy Ctrl+Shift hotkey defaults: migrated to the Alt+F5..F8 cluster by `normalize_hotkeys`.

## Major rewrites

In a two-week project there are few true rewrites, but a few notable pivots appear in the history. The preview capture rework toward DXGI Desktop Duplication appears to have been done partly to keep the cursor visible. A fast-start remux was added for immediacy in output. The hotkey default migration to the F5-F8 cluster touched settings and hooks at once. And the FFmpeg sidecar gained a verification chain (SHA256 plus GitHub attestation) so the shipped binary is trustworthy. All of these were surgical adjustments rather than throw-it-away rewrites.

## Growth trajectory

Volume is strikingly lumpy. A single commit on August 5, then nine on August 9, thirteen on August 10, two on August 11, three on August 12, four on August 13, and thirty on August 19. Roughly half of all commits landed on the final recorded day. The supporting directories tell the same story: `packages/`, `website/`, and `cloudflare/` each appear as one coherent addition during the August 9 to 10 windows, and everything stays the work of a single contributor.

## Speculation

The seven-day silence between August 13 and 19 (and the initial four days after the scaffold) likely hid planning and prep rather than idle time, given how much landed when the dust cleared. The `droid.yml` and `droid-review.yml` workflows, merged through a "add-factory-workflows" PR, appear to have been generated by a Factory workflow generator rather than hand-written, though that inference cannot be proven from the commit messages alone.

## Related pages

- [By the numbers](by-the-numbers.md) is the current quantitative snapshot
- [Design decisions](background/design-decisions.md) explains the rationale behind several eras
- [Fun facts](fun-facts.md) collects the light trivia
