# Updater mirror

Active contributors: elwina

The updater mirror is a free Cloudflare Worker (`cloudflare/worker.js`) that proxies the Capto GitHub Release so in-app update checks and installer downloads get a faster CDN edge, especially where `github.com` is slow. The desktop's Tauri updater prefers the worker and falls back to GitHub.

## Purpose

Tauri's generated `latest.json` points each platform `url` at `api.github.com/repos/.../releases/assets/<id>`, which is rate-limited (60 requests/hour/IP, anonymous) and can be slow. Because the installer filename is fixed by the Release workflow (`Capto_<version>_<arch>-setup.exe`, tag `v<version>`), the worker rebuilds each `url` from the manifest `version` into a `github.com/.../releases/download/<tag>/<file>` browser URL. That URL has no API rate limit and redirects to the signed CDN asset.

## How it works

Routes handled in `cloudflare/worker.js`:

| Route | Behaviour |
|-------|-----------|
| `GET /updates/latest.json` | **Stable channel**: fetches the rolling `updater` release tag's `latest.json`, rewrites each platform `url` to the worker's download route, returns the JSON (cached on the CF edge). |
| `GET /updates/canary.json` | **Canary channel**: same, but reads the separate rolling `canary` release tag for opt-in testers. |
| `GET /updates/download/*` | Streams a GitHub release asset through the worker; CF caches large installers (`cache-control: immutable`, `max-age=86400`). |
| `OPTIONS` | CORS preflight so the update check can run cross-origin. |

The worker maps Tauri updater targets to installer arch tokens (`windows-x86_64(-nsis)` → `x64`, `windows-aarch64(-nsis)` → `arm64`) and derives each download URL (`rewriteReleaseJson` / `browserDownloadUrl`). The download route decodes the encoded upstream GitHub URL and streams it, following the bounce to the CDN.

### Canary staged rollout

To roll out a version early, publish an experimental build to a `canary` release tag with its own `latest.json`, and point testers or pilot agents at `.../updates/canary.json`. Once validated, promote it to stable by publishing the same version's `latest.json` to the `updater` tag, so stable users move off canary automatically. A bad canary never reaches stable users and can be pulled by deleting the `canary` tag. The full procedure is in `docs/CI.md`.

## Deployment and integration

Deploy with wrangler (`cloudflare/README.md`):

```bash
cd cloudflare
npx wrangler login
npx wrangler deploy
```

The resulting `*.workers.dev` URL goes first in `apps/desktop/src-tauri/tauri.conf.json` under `plugins.updater.endpoints`. The Tauri updater tries endpoints in order and only falls through on a non-2XX response, so the worker is listed first and `https://github.com/elwina/Capto/releases/download/updater/latest.json` second. If the worker is down, updates still work via the GitHub fallback. See [Deployment](../deployment.md) for the release pipeline that produces the updater artifacts and the rolling `updater` manifest.

## Limits

The worker runs on the Cloudflare free plan: 100,000 requests/day, with a 512 MB cache object limit per object. Installers are only tens of MB, so this is comfortable. GitHub releases are public, so no token is needed; if one ever is, set `GH_TOKEN` via `npx wrangler secret put GH_TOKEN` (`cloudflare/README.md`). Do not commit `.dev.vars`; only `.dev.vars.example` is tracked.

## Key source files

| File | Purpose |
|------|---------|
| `cloudflare/worker.js` | The entire worker: routing, URL rewriting, streaming proxy, caching |
| `cloudflare/wrangler.toml` | Worker config for deploy |
| `cloudflare/README.md` | Deploy, wire-in, local dev, and limits |
| `apps/desktop/src-tauri/tauri.conf.json` | `plugins.updater.endpoints` (worker first, GitHub second) |
