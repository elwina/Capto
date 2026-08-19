# Capto runbooks

Operational responses for alerts and recurring maintenance. Read the relevant
runbook before acting on a signal. These complement the CI and Release
workflows: if a workflow is failing, start here.

- [Security incidents](security.md) — secret scanning / code scanning alerts, leaked keys, supply-chain signals.
- [Releasing Capto](release.md) — how a tag becomes an installer, what to verify before/after tag.
- [CI failures](ci.md) — triage steps for the CI workflow matrix, and when owners get involved.

See `docs/ARCHITECTURE.md` for the codebase itself and `docs/CI.md` for the
workflow layout.
