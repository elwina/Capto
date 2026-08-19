# Updates

Active contributors: elwina

Capto ships self-updates through the Tauri updater plugin (`tauri-plugin-updater`) with minisign signature verification. In-app checks hit the Cloudflare Worker mirror first and fall back to the raw GitHub release manifest, so the app updates reliably even when a single host is down. Releases are signed by the maintainer and verified locally against a pinned public key.

## Purpose

Installed users need a way to move to new versions without downloading installers by hand. The updater gives the desktop app a check-and-install loop: it fetches a `latest.json` manifest, compares it to the running version, downloads the signed installer, installs passively, and relaunches. The whole flow is local-first in the sense that nothing is required to work, if both endpoints fail, the app simply keeps running and the user checks again later.

## How it works

The updater config lives in `apps/desktop/src-tauri/tauri.conf.json` under `plugins.updater`. It declares a minisign public key and an ordered list of check endpoints:

1. `https://capto-update-proxy.elwina-vardal.workers.dev/updates/latest.json` (Cloudflare Worker)
2. `https://github.com/elwina/Capto/releases/download/updater/latest.json` (GitHub)

The Tauri updater tries endpoints in order and only falls through on a non-2XX response. Listing the worker first means updates ride Cloudflare's CDN by default; the GitHub URL is the fallback if the worker is down. Each manifest's installer `url`s point at the versioned `v*` release assets on GitHub. The Cloudflare worker itself is covered on [Updater mirror](../apps/updater-mirror.md).

### Signing

Every `latest.json` is signed with minisign. The public key is baked into `apps/desktop/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). The matching private key is the GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY` (with an optional empty `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`); a gitignored local copy lives at `.secrets/capto.key` and must never be committed. The desktop refuses an unsigned or wrongly signed update, which is what makes the whole channel safe to mirror through a third-party CDN.

### Rolling `updater` tag

The Release workflow mirrors each release's `latest.json` onto a rolling Git tag named `updater`. Only the manifest is published to that tag, so the in-app check URL stays stable across releases: users always ask the same endpoint and get whatever the newest published version is. This is described in `docs/CI.md` under "In-app updates (GitHub)".

### Canary staged rollout

New versions can be rolled out in stages before hitting all users. The procedure, documented in `docs/CI.md`, is:

1. Publish an experimental build to a rolling `canary` release tag with its own `latest.json` (`gh release create canary --prerelease` plus upload `latest.json`).
2. Point testers or pilot agents at `https://capto-update-proxy.elwina-vardal.workers.dev/updates/canary.json`, which the worker serves from the `canary` tag.
3. Validate the canary (record/QA round trips plus crash reports), then promote: publish that same version's `latest.json` to the `updater` tag so stable users move off canary automatically, and close the canary tag once superseded.

A bad canary never reaches stable users and can be pulled by deleting the `canary` tag or manifest.

### Versioning

Current version is `1.0.0` (stable). `v1.*` releases are stable; `v0.*` were prerelease. The updater targets the same versions, canary and stable builds follow the versioning rules set out in [Deployment](../deployment.md).

## Configuration

The relevant keys in `apps/desktop/src-tauri/tauri.conf.json`:

| Key | Value |
|-----|-------|
| `plugins.updater.pubkey` | Minisign public key (line-based key body) |
| `plugins.updater.endpoints[0]` | Cloudflare Worker `latest.json` |
| `plugins.updater.endpoints[1]` | GitHub `updater` tag `latest.json` |
| `plugins.updater.windows.installMode` | `passive` (installs without interactive prompts), then relaunch |

## UI surface

The About tab exposes the update flow in the React UI:

- `apps/desktop/src/components/UpdateSettings.tsx`, a manual "check for updates" button plus an install button when an update is available. It reports checking / up-to-date / available / downloading (with percent) / installing / error phases, and calls `relaunch()` after download-and-install.
- `apps/desktop/src/components/AboutPanel.tsx`, embeds `UpdateSettings`, shows the app version via the Tauri app API, and displays the bundled FFmpeg bundle version (from the `get_ffmpeg_info` command) plus the FFmpeg binary version, path, and source repository link. It also links the project source, license, and developer pages.

Settings UI: the **About** tab shows version, a manual update check, the bundled FFmpeg version, and developer links (per `docs/CI.md`).

## Key rotation

If the minisign private key is lost or leaked, the public key in `apps/desktop/src-tauri/tauri.conf.json` must be rotated in the same release. After a public-key change, existing installs cannot verify new signatures unless you also ship a bridge release, so rotations should be rare and deliberate. The full guidance lives in `docs/CI.md`.

## Integration points

- [Updater mirror](../apps/updater-mirror.md), the Cloudflare Worker that serves the worker endpoint and mirrors the `canary.json` route.
- [Deployment](../deployment.md), the Release workflow that produces signed updater artifacts and the rolling `updater` manifest.
- The About panel and update settings both live in `apps/desktop/src/` and call the Tauri updater and `get_ffmpeg_info` commands.

## Entry points for modification

- To change check endpoints, edit `plugins.updater.endpoints` in `apps/desktop/src-tauri/tauri.conf.json`.
- To change the update UI, edit `apps/desktop/src/components/UpdateSettings.tsx` and `apps/desktop/src/components/AboutPanel.tsx`.
- To change the rollout procedure or signing steps, edit `docs/CI.md` and `.github/workflows/release.yml`.

## Key source files

| File | Purpose |
|------|---------|
| `apps/desktop/src-tauri/tauri.conf.json` | Updater plugin: pubkey, ordered endpoints, Windows install mode |
| `apps/desktop/src/components/UpdateSettings.tsx` | Manual check + install/relaunch UI with progress states |
| `apps/desktop/src/components/AboutPanel.tsx` | App version, FFmpeg bundle info, developer links, embeds UpdateSettings |
| `.github/workflows/release.yml` | Builds signed updater artifacts and mirrors `latest.json` onto the rolling `updater` tag |
| `.secrets/capto.key` | Gitignored local minisign private key (never committed) |
| `docs/CI.md` | Updater, canary staged rollout, and key-rotation guidance |
