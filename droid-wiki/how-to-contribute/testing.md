# Testing

This page explains what is covered by automated tests, how each suite runs, and what is deliberately left out of CI. Capto is a Windows desktop app, so the split matters: unit tests and logic run anywhere, while real recording and encoding only run on a Windows desktop session.

## The purpose of this page

This page maps the testing layers in the repository, from Rust unit tests to the frontend Vitest suite to the agent-facing smoke suites and the interactive QA script. It ends with the coverage gates CI enforces and the areas that are validated manually because CI cannot run a display session. The exact commands are also listed in [development-workflow.md](development-workflow.md).

## Rust: `cargo test --workspace`

Rust tests are inline unit tests living next to the code under `#[cfg(test)] mod tests`, standard for this workspace. Run them with:

```powershell
cargo test --workspace --all-targets
```

There is no integration-test harness for real capture. A Rust unit test cannot open a DXGI display surface or start a WASAPI loopback, so backends and encode paths are covered by unit-testing the parts that can run headless (argv construction in `crates/capto-core/src/ffmpeg_args.rs`, geometry clamping, config parsing). The capture and encoding backends themselves are validated manually on a real desktop.

## Frontend: Vitest + Testing Library

Frontend unit tests live under `apps/desktop/src/**/*.test.{ts,tsx}` and run in Node. The Vitest config in `apps/desktop/vitest.config.ts`:

- Runs in `environment: "node"` against pure logic (hotkey parsing, formatters, the i18n bootstrap), so no browser is required.
- Enables `globals: true` so Testing Library auto-cleanup runs between component tests.
- Retries a failed test once (`retry: 1`) as a flaky-test safety net.
- Runs files in parallel thread pools.

Run them with `npm test --prefix apps/desktop`, or `npm run test:coverage` to also enforce the coverage gate.

## Coverage gate

`npm run test:coverage` runs Vitest with V8 coverage and fails if the included modules drop below the thresholds in `apps/desktop/vitest.config.ts`:

- 80% lines, 75% functions, 70% branches, 80% statements.

Coverage is scoped deliberately to `apps/desktop/src/i18n/index.ts` and `apps/desktop/src/components/HotkeySettings.tsx`. New modules are added to that `include` list (with their tests) to bring them under the gate; un-tested views like the session shell in `apps/desktop/src/App.tsx` are not counted against you until component tests exist for them. Several other gates run alongside tests in CI and are described in [Tooling](tooling.md): jscpd duplicate detection, knip dead-code scanning, and the bundle-size budget.

## npm package tests: `capto-dsh-plugin`

The agent-facing npm packages have their own offline smoke suite. `packages/capto-dsh-plugin/test/smoke.mjs` does not need the Capto desktop or its control plane. Success paths run against a fixture, `packages/capto-dsh-plugin/test/fixtures/fake-capto.mjs` (a fake `capto` CLI that speaks the JSON envelope contract and echoes back its argv when `FAKE_CAPTO_ECHO` is set), so the suite verifies tool registration, config validation, CLI arg mapping, timeout handling, and the exit-2 `desktopUnavailable` recovery path.

When `target/debug/capto.exe` exists, the same file also runs one real-CLI check against the actual binary (in CI this exercises the `desktopUnavailable` path, since no desktop is running). CI also runs `npm pack --dry-run` for `packages/capto-agent-skill` and `packages/capto-dsh-plugin` to catch packaging problems.

## Interactive QA: `scripts/qa-smoke.ps1`

The interactive path is agent-followable and is what a real desktop validation uses. `scripts/qa-smoke.ps1` drives the loopback control plane through the `capto` CLI and asserts every step returns a JSON envelope with `ok:true`:

```powershell
.\scripts\qa-smoke.ps1                     # doctor -> status -> config path -> outputs recent
.\scripts\qa-smoke.ps1 -RunRecordRoundtrip # + record start -> stop -> outputs recent
```

The record round-trip needs a Windows desktop session, so it is off by default and is skipped in CI (runners are headless). Run it on a real machine when you change capture or encoding behavior. The full interactive narrative is in `docs/QA.md`.

## Security probe: `scripts/control-plane-dast.ps1`

The localhost HTTP control plane has a black-box security suite, `scripts/control-plane-dast.ps1`. It drives a running server with adversarial requests and asserts the expected rejection, complementing the auth unit tests in `apps/desktop/src-tauri/src/cli_server.rs`. The probes cover no-auth (401), wrong token (401), real token (200 + `ok:true`), unknown route (404), malformed JSON on POST (4xx), and that no error body leaks the bearer token. Run it against a desktop that is already started, so `cli-server.json` exists. Full detail lives in `docs/security-testing.md`.

## What CI does not cover

CI runs headless Ubuntu and Windows runners without a display session. That keeps these things out of automated coverage:

- Real WGC / WASAPI / NVENC recording and encoding. These need a Windows desktop session and are validated manually via `scripts/qa-smoke.ps1 -RunRecordRoundtrip`.
- The rustdoc for the site and the packaged installers (Release), which are separate workflows documented in [Deployment](../deployment.md).
- Certain frontend component/DOM tests, until a file opts into `environment: "jsdom"` (jsdom is already a dependency, ready to use).

Treat CI as the fast, broad safety net and the interactive QA script as the required, slow, real-desktop check before releasing.

## Related pages

- [How to contribute](index.md)
- [Development workflow](development-workflow.md)
- [Debugging](debugging.md)
- [Tooling](tooling.md)
- [Deployment](../deployment.md)
- [How to monitor](../how-to-monitor/index.md)
