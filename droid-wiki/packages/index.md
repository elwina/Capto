# Packages

Active contributors: elwina

Capto publishes two npm packages that let AI agents drive the local `capto` CLI, rather than capturing or encoding themselves. Both wrap the same control-plane client — they ship docs or tool wrappers, not a second recording path — so the one-machine, one-session guarantee in the repo holds regardless of which package an agent uses.

## Packages

| Package | One-line summary |
|---------|------------------|
| [capto-agent-skill](capto-agent-skill.md) | An Agent Skills docs package (`skills/capto/SKILL.md` + `references/cli.md`) that teaches an agent the `doctor → record → stop → outputs` workflow over the `capto` CLI |
| [capto-dsh-plugin](capto-dsh-plugin.md) | A DeepSeek Harness (dsh) plugin registering 14 typed `capto_*` tools that each invoke the CLI and normalize its output and exit codes |

Both source trees live under `packages/`. They are not deployables; the desktop installer is the release artifact. The CLI they drive is documented on the [CLI app page](../apps/cli.md), and the control-plane contract they talk to indirectly is under [Control-plane API](../api/index.md).

## Publishing notes

- npm name `capto` is taken by an unrelated package, so the skill publishes as **`capto-agent-skill`** and the plugin as **`capto-dsh-plugin`**.
- Both are public (`publishConfig.access: "public"`), MIT, and their `package.json` files live under `packages/<name>/`, not at the repo root. Publishing from the monorepo root would hit the private `capto-workspace` package and fail, so pack and publish are run against the package subdirectory path explicitly.
- Publish from the repo root with `npm pack ./packages/<name>` (dry run) then `npm publish ./packages/<name> --access public` (see each package's README for the exact commands).
- The agent-skill package version may move independently of app releases; it was decoupled from app versioning in August 2026. The dsh plugin has its own independent `0.x` line.

## CI coverage

The repo CI (`ci.yml`, `docs/CI.md`) covers both packages. The dsh plugin runs `node test/smoke.mjs` (11 checks) with a fixture CLI, a real-CLI check on Windows when the binary is built, and `npm pack --dry-run`; the fake-only path runs on Ubuntu. Each package also has a `prepublishOnly` script that fails the publish if smoke checks or the expected files are absent.

## Integration points

- Both packages invoke `capto <command>` via the binary on PATH (or `cargo run -p capto-cli --` in a dev checkout).
- `packages/capto-agent-skill/skills/capto/references/cli.md` is kept aligned with the repo `docs/CLI.md`.
- `packages/capto-dsh-plugin` uses the same JSON envelope and exit-code contract, and its tests reuse the envelope via `test/fixtures/fake-capto.mjs`.

## Key source files

| File | Purpose |
|------|---------|
| `packages/capto-agent-skill/package.json` | Skill package metadata, files list, agentskills manifest |
| `packages/capto-agent-skill/skills/capto/SKILL.md` | The skill body agents load |
| `packages/capto-agent-skill/skills/capto/references/cli.md` | The reference contract, synced with `docs/CLI.md` |
| `packages/capto-dsh-plugin/package.json` | Plugin metadata, config schema, `dsh.bundle` manifest reference |
| `packages/capto-dsh-plugin/src/index.js` | Plugin entry exporting the Cordis contract and `tool:capto` prompt section |
| `packages/capto-dsh-plugin/src/tools.js` | The 14 tool definitions |
| `packages/capto-dsh-plugin/test/smoke.mjs` | Offline smoke suite (11 checks) |
