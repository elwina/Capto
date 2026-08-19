# Runbook: CI failures

First read `docs/CI.md` for the workflow layout. Quick triage for each failed
job:

| Job | Failure means | First look at |
|-----|---------------|---------------|
| `rust` | fmt / clippy / unit+integration tests | `cargo fmt --all --check`, `cargo clippy --workspace`, `cargo test --workspace`. New struct-typed API? Check `capto-ipc` + `capto-dsh-plugin` smoke. |
| `frontend` | eslint / prettier / knip / vitest / tsc | `npm run lint`, `npm run format:check`, `npm run knip`, `npm test`, `npx tsc --noEmit`. Coverage gate raises if tested-file coverage drops (`npm run test:coverage`). |
| `check-targets` | cross-arch compile of app+CLI | Rust targets missing, FFmpeg asset naming, `cfg(windows)` code that broke ARM. |
| `hygiene` | oversized source files OR tech-debt markers OR AGENTS.md structure | `.\scripts\check-file-size.ps1`, `.\scripts\scan-tech-debt.ps1`, `.\scripts\validate-agents-md.ps1`. |
| `packages` | npm pack / dsh-plugin + agent-skill tests | breaking change to the CLI JSON contract → `docs/CLI.md` and `packages/capto-dsh-plugin` must be updated together. |
| `secret-scan` / `codeql` | leaked secret / static analysis alert | See `docs/runbooks/security.md`. |

## Golden rules

1. Reproduce locally before touching YAML. Every CI gate has a local
   equivalent (listed above); they run fast.
2. A fix must come with the gate re-run locally. Do not push a red workflow
   and hope.
3. If a weak or genuinely intermittent test flakes (Windows timers), fix the
   test, don't delete it. Keep `test_isolation` and parallel-friendly Vitest
   config (`pool: threads`).
4. Never disable a gate silently — that is what `docs/tech-debt.md` is for,
   and it must say why.

## Escalation

Flaky-in-CI-only, hard-to-repro failures: post the failing run URL in a PR
with the local repro output. Anything release-blocking: owners per
`CODEOWNERS` and the release runbook.
