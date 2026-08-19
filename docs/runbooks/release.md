# Runbook: Releasing Capto

Capto ships Windows NSIS installers (x64 + arm64) from Git tags. A tag alone
is enough — the `Release` workflow builds, signs, and publishes.

## Before you tag (blocking checks)

1. **CI is green on `main`** for the workflows that matter:
   `rust` (test/clippy/fmt), `frontend` (lint/format/knip/test/tsc),
   `check-targets` (both arches), `hygiene`, `packages`, `secret-scan`, `codeql`.
2. Version bumped in:
   - `apps/desktop/package.json` (e.g. `1.1.0`) and `src-tauri/tauri.conf.json`
     (tauri-action substitutes `v__VERSION__` from the tag, but keep the
     package version in sync).
   - Signing keys present as Actions secrets
     (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`).
   - FFmpeg pin current: `.github/capto-ffmpeg.env`.
3. `v1.*` tags are **stable**, `v0.*` are **prerelease** — pick the right tag.

## Tag → release flow (what happens automatically)

1. `git tag vX.Y.Z && git push origin vX.Y.Z`.
2. `Release` workflow: installs deps, downloads verified FFmpeg per-arch,
   builds `capto-cli` and stages it into Tauri resources, runs
   `tauri-action` → NSIS installer per arch, signed updater artifacts
   (`latest.json`, `.sig`).
3. `publish-updater-manifest`: copies `latest.json` onto the rolling `updater`
   release so the in-app update check URL never moves.
4. `generate-changelog`: appends commit subjects since the previous
   release to the GitHub Release body (repo uses no PRs).

## After the release (verify)

1. Release page shows both `Capto-*-x64-setup.exe` and `*-arm64-setup.exe`.
2. `latest.json` on the `updater` tag points at the new version.
3. Install on a real Windows 10/11 box: launch, record (WGC/WASAPI), take a
   screenshot, and run `capto doctor` from a fresh terminal (CLI is on PATH).
4. If the in-app updater is enabled, run the update check against the new
   release.

## Rollback

Fastest: keep the previous `v*` release (updater README points at latest
versioned tag, so `latest.json` must be reverted). Edit the `updater` release
to upload the previous `latest.json` (`gh release upload updater latest-old.json
--clobber` after renaming), then cut a hotfix tag — do not re-tag history.
