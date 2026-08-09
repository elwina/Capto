# capto-agent-skill

[Agent Skills](https://agentskills.io) package for **Capto** — a local-only Windows screen recorder.

Agents drive Capto with the `capto` CLI (JSON over a localhost control plane). This package ships skill docs only; install Capto from [Releases](https://github.com/elwina/Capto/releases) — the installer includes the CLI and adds it to PATH (`capto` in new terminals).

## Install

```bash
npm install capto-agent-skill
```

Skill entry (agentskills / skills-npm):

```
node_modules/capto-agent-skill/skills/capto/SKILL.md
```

Reference contract: `skills/capto/references/cli.md`.

## Requirements

- Windows Capto desktop with bundled FFmpeg
- `capto` on `PATH`, or in-repo: `cargo run -p capto-cli -- …`
- Dev auto-launch: set `CAPTO_APP_PATH` to `capto-app.exe` if needed

## Publish checklist (maintainers)

1. Bump `version` in `package.json` and `skills/capto/SKILL.md` metadata together.
2. Keep `references/cli.md` aligned with repo [`docs/CLI.md`](../../docs/CLI.md).
3. Dry-run pack:

```bash
npm pack ./packages/capto-agent-skill
# or from repo root:
npm run skill:pack
```

4. Publish:

```bash
npm publish ./packages/capto-agent-skill --access public
```

> npm name `capto` is taken by an unrelated package — publish as **`capto-agent-skill`**.

## Spec

- Format: [Agent Skills specification](https://agentskills.io/specification)
- Distribution: [skills inside npm `skills/`](https://github.com/antfu/skills-npm/blob/main/PROPOSAL.md)
