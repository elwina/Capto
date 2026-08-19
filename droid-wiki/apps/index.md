# Apps

Active contributors: elwina

Capto ships as four separate deployables: the Tauri desktop app, the `capto` CLI, a static website, and a Cloudflare Worker that mirrors updates. They share the same monorepo but build, run, and deploy independently.

- [Desktop app](desktop/index.md), `capto-app`, the Tauri 2 shell that owns the recording session
- [CLI](cli.md), the `capto` control-plane client for agents and shells
- [Website](website.md), the static landing page
- [Updater mirror](updater-mirror.md), the Cloudflare Worker that proxies GitHub release metadata and downloads

## Purpose

The desktop app is the single process that records the screen; everything else is an interface to it. The CLI drives that one process over a localhost HTTP control plane instead of starting a second recorder. The website points potential users at downloads and agent integrations. The updater mirror keeps in-app update checks and installer downloads fast by riding Cloudflare's CDN and avoiding `api.github.com` rate limits.

The CLI and the desktop avoid sharing the name `capto`, because cargo would write two `target/debug/capto.exe` files and Windows paths are case-insensitive (see `docs/CLI.md`). The CLI owns the `capto` name; the desktop crate is `capto-app`.

## Deployables

| Deployable | Binary | Crate / package | Role | Deploy target |
|------------|--------|-----------------|------|---------------|
| Desktop app | `capto-app.exe` | `capto-app` | Recording session + local control plane | NSIS installer (`%LOCALAPPDATA%\Capto`) |
| CLI | `capto.exe` | `capto-cli` | Agent/shell client of the control plane | Bundled in installer at `<install>\cli\`, added to user PATH |
| Website | static HTML | `website/` (no crate) | Product landing page | GitHub Pages + Cloudflare Pages |
| Updater mirror |, | `cloudflare/` worker | Mirrors release metadata + installer assets | Cloudflare Workers |

The npm agent packages are not deployables themselves; they wrap the CLI. See [capto-agent-skill](../packages/capto-agent-skill.md) and [capto-dsh-plugin](../packages/capto-dsh-plugin.md).

## Directory layout

```
droid-wiki/apps/
index.md            This index
desktop/            Desktop app (Tauri + React)
cli.md              The capto CLI
website.md          The landing page
updater-mirror.md   The Cloudflare update proxy
```

Source for each lives in the repo at `apps/desktop/`, `crates/capto-cli/`, `website/`, and `cloudflare/`, respectively.

## Integration points

- The CLI calls the desktop's control-plane endpoints (`/v1/*`) described fully in [Control-plane API](../api/index.md); shared envelope and lockfile types live in [capto-ipc](../crates/capto-ipc.md).
- The updater config in `apps/desktop/src-tauri/tauri.conf.json` lists the worker endpoint first and the GitHub endpoint second, so the worker can be down without breaking updates.
- The release pipeline ([Deployment](../deployment.md)) embeds the CLI into the installer and publishes the updater artifacts the worker mirrors.
