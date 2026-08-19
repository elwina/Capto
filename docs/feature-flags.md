# Feature flags

Capto uses a **local** declarative feature-flag system — no remote service.
Flags are resolved from `settings.json` (`enabledFlags` / `disabledFlags`)
against the registry in `crates/capto-core/src/flags.rs`.

## Why

Feature flags let agents ship a behavior behind a toggle that defaults to
safe, then flip it via `capto config set` without a code change; if it misbehaves,
agents can disable it without a release.

## Registry (`crates/capto-core/src/flags.rs`)

| Flag (const) | JSON name | Default | Effect when enabled |
|--------------|-----------|---------|---------------------|
| `CONTROL_PLANE_METRICS` | `control-plane-metrics` | `true` | Serve `GET /v1/metrics` on the control plane (localhost, auth required) |
| `CRASH_REPORTING` | `crash-reporting` | `true` | Write structured `crash-*.json` reports on panic |

`flags::is_enabled(settings, name)` resolves: `disabledFlags` > `enabledFlags` >
default.

## Lifecycle

1. **Add** a flag: declare the const + `FeatureFlag` in `flags.rs`, gate the
   runtime behavior, document it here.
2. **Flip** it at runtime: `capto config set --json '{"enabledFlags":["<name>"]}'`
   (or remove it to return to default).
3. **Remove** a flag once the behavior is always-on (or always-off): delete the
   const + registry entry, undo the gating, update this doc.

## Dead-flag detection

`scripts/scan-dead-flags.ps1` runs in CI and fails if a declared flag is never
referenced by runtime code outside its definition — it must stay in sync with
`flags.rs`. This prevents stale flags from accumulating.
