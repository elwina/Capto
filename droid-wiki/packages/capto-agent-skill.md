# capto-agent-skill

Active contributors: elwina

`capto-agent-skill` is the [Agent Skills](https://agentskills.io) npm package for Capto. It ships skill documentation only, it teaches an agent to drive the local `capto` CLI through the `doctor → record → stop → outputs` loop. It contains no code, no capture logic, and no upload path. Install it wherever a host agent auto-discovers skills from npm modules.

## Purpose

Agents need a compact, loadable instruction set to control a screen recorder safely. This package provides exactly that: a `SKILL.md` body plus a `references/cli.md` cheat sheet, both describing the JSON envelope, the stable exit codes, the desktop-availability rules, and the recording/settings/discovery workflows. Because it deliberately carries no executable code, an agent can adopt it with zero build step and no risk of it doing anything beyond telling the agent how to run `capto <command>`.

## What it ships

The packaged files (see the `files` array in `packages/capto-agent-skill/package.json`) are:

- `packages/capto-agent-skill/README.md`, install instructions, requirements, and the maintainer publish checklist.
- `packages/capto-agent-skill/skills/capto/SKILL.md`, frontmatter (name, description, metadata, `npm: capto-agent-skill`) plus the rules and step-by-step workflows an agent follows.
- `packages/capto-agent-skill/skills/capto/references/cli.md`, the CLI contract: the `Agent → capto → 127.0.0.1 → desktop` model, JSON envelope, exit-code table, command table, flags, and safety notes.

The `prepublishOnly` script (`packages/capto-agent-skill/package.json`) asserts `skills/capto/SKILL.md` exists before publishing, so the package cannot go out without its body.

## Install

```bash
npm install capto-agent-skill
```

Per the agent-skills / skills-npm conventions, a host auto-discovers the skill at:

```
node_modules/capto-agent-skill/skills/capto/SKILL.md
```

That path is declared in the package's `package.json` `agentskills.skills` entry and by the `skills/capto/` directory layout.

## Requirements

- Windows Capto desktop with the bundled FFmpeg (the [installer](https://github.com/elwina/Capto/releases) embeds FFmpeg and adds `<install>\cli\` to PATH).
- `capto` on `PATH`, or in a dev checkout `cargo run -p capto-cli -- …`.
- Dev: set `CAPTO_APP_PATH` to `capto-app.exe` if the CLI cannot auto-discover the desktop.
- If a command returns exit code `2` (`desktopUnavailable`), run `capto open` or ask the user to open Capto from the Start menu.

## The taught workflow

The skill instructs agents to follow the same round trip the CLI documents on [CLI app page](../apps/cli.md): check `doctor` (ffmpegOk true, exit 2 → open/ask), optionally `list displays`, then `record start --source display`, poll `status`, `record stop`, and find the file with `outputs recent`. It adds safety rules: parse JSON stdout, never `record start` twice, always `record stop` when done, never spawn system FFmpeg for Capto outputs, and never upload.

## Publish checklist (maintainers)

The skill version is decoupled from app releases, it tracks them only when the CLI or skill contract changes together. When publishing:

1. Bump `version` in `packages/capto-agent-skill/package.json` and the `metadata.version` in `packages/capto-agent-skill/skills/capto/SKILL.md` together (only when the docs/contract change).
2. Keep `packages/capto-agent-skill/skills/capto/references/cli.md` aligned with the repo `docs/CLI.md`.
3. Dry-run from the repo root: `npm pack ./packages/capto-agent-skill` (or `npm run skill:pack`).
4. Publish: `npm publish ./packages/capto-agent-skill --access public` from the repo root (or `cd packages/capto-agent-skill && npm publish --access public`).

npm name `capto` is taken, so the package is `capto-agent-skill` (matching `metadata.npm` in the SKILL frontmatter).

## Spec compliance

The package follows the [Agent Skills specification](https://agentskills.io/specification) and the skills-inside-npm distribution proposal (`skills/` directory + `agentskills` key). See the `packages/capto-agent-skill/README.md` "Spec" section for the linked references.

## Integration points

- It wraps the `capto` CLI, the same binary the [dsh plugin](capto-dsh-plugin.md) registers tools over.
- The website promotes this package as a way for AI agents to run the capture loop (`../apps/website.md`).
- Its reference file stays in sync with `docs/CLI.md` so the contract never drifts.

## Key source files

| File | Purpose |
|------|---------|
| `packages/capto-agent-skill/package.json` | Metadata, files list, `agentskills` manifest, prepublishOnly guard |
| `packages/capto-agent-skill/README.md` | Install, requirements, and maintainer publish checklist |
| `packages/capto-agent-skill/skills/capto/SKILL.md` | The skill body: rules, workflows, desktop-recovery steps |
| `packages/capto-agent-skill/skills/capto/references/cli.md` | CLI contract reference, kept in sync with `docs/CLI.md` |
| `packages/capto-agent-skill/LICENSE` | MIT license |
