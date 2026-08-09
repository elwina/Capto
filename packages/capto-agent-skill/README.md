# capto-agent-skill

[Agent Skills](https://agentskills.io) package for **Capto** — a local-only screen recorder.

Agents use the `capto` CLI (JSON over a localhost control plane) to record, screenshot, read status, and manage settings. This package does **not** embed the Capto binary; install/build Capto separately.

## Install

```bash
npm install capto-agent-skill
# or discover via skills-npm after install:
# npx skills-npm
```

Skill path (agentskills / skills-npm convention):

```
node_modules/capto-agent-skill/skills/capto/SKILL.md
```

## Publish (maintainers)

```bash
npm publish ./packages/capto-agent-skill --access public
# or from repo root:
npm run skill:pack
```

> npm name `capto` is already taken by an unrelated package — we publish as `capto-agent-skill`.

## Spec

- Format: [Agent Skills specification](https://agentskills.io/specification)
- Distribution: [skills inside npm `skills/`](https://github.com/antfu/skills-npm/blob/main/PROPOSAL.md)
