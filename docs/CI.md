# Capto CI / Release

## Workflows

| Workflow | File | Trigger | Purpose |
|----------|------|---------|---------|
| **CI** | [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) | push/PR → `main` | Rust test + clippy, frontend `tsc`, `cargo check` for **x64 + ARM64**, FFmpeg pin download + attestation |
| **Release** | [`.github/workflows/release.yml`](../.github/workflows/release.yml) | tag `v*` or manual | NSIS installers for both arches with **embedded** FFmpeg + CLI (`cli/capto.exe`); signed updater artifacts + rolling `updater` manifest |

CI and Release are intentionally separate: green CI does not publish; Release does not replace day-to-day checks.

## Versioning

- App / workspace version today: **0.2.0** (`Cargo.toml` workspace + `tauri.conf.json`)
- Git tags: `v0.2.0`, … — `v0.*` releases are marked **prerelease**
- First stable line: **1.0.0** (`v1.0.0`) when ready

## In-app updates (GitHub)

Desktop uses [`tauri-plugin-updater`](https://v2.tauri.app/plugin/updater/) with minisign verification.

| Piece | Location |
|-------|----------|
| Public key | `apps/desktop/src-tauri/tauri.conf.json` → `plugins.updater.pubkey` |
| Private key | GitHub Actions secret **`TAURI_SIGNING_PRIVATE_KEY`** (optional empty **`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`**) |
| Check URL | `https://github.com/elwina/Capto/releases/download/updater/latest.json` |
| Installer URLs | Inside `latest.json`, pointing at the versioned `v*` release assets |

`v0.*` tags are **prerelease**, so GitHub’s `/releases/latest` would skip them. Each Release therefore mirrors `latest.json` onto a rolling tag **`updater`** (manifest only). CDN / CF Worker mirrors can be added later as extra `endpoints` entries.

Local key (gitignored): `.secrets/capto.key` — do not commit. Rotate only if the private key is lost or leaked (existing installs cannot verify new signatures after a pubkey change unless you ship a bridge release).

Settings UI: **关于** tab — version, manual update check, bundled FFmpeg version, developer links.

## FFmpeg (security)

Sidecar comes only from [`elwina/capto-ffmpeg`](https://github.com/elwina/capto-ffmpeg) Releases — never from PATH at runtime.

Pin: [`.github/capto-ffmpeg.env`](../.github/capto-ffmpeg.env)

```powershell
.\scripts\download-ffmpeg.ps1 -TargetTriple x86_64-pc-windows-msvc -VerifyAttestation
.\scripts\download-ffmpeg.ps1 -TargetTriple aarch64-pc-windows-msvc -VerifyAttestation
```

Checks:

1. Download matching asset (`ffmpeg-windows-x86_64.exe` / `ffmpeg-windows-aarch64.exe`)
2. Verify **SHA-256** against release `SHA256SUMS`
3. In CI/Release: **`gh attestation verify`** (Sigstore / Artifact Attestations)
4. Copy to `apps/desktop/src-tauri/binaries/ffmpeg-<triple>.exe` for Tauri `externalBin`

## Architectures

| Display | Rust target (internal) | Installer | Bundled CLI |
|---------|------------------------|-----------|-------------|
| **x64** | `x86_64-pc-windows-msvc` | NSIS `.exe` | `<install>\cli\capto.exe` (+ user PATH) |
| **arm64** | `aarch64-pc-windows-msvc` | NSIS `.exe` | `<install>\cli\capto.exe` (+ user PATH) |

Job titles and release assets use **x64 / arm64**. The `*-windows-msvc` strings are only Rust target triples (Windows ABI). **MSI is not built** — NSIS setup exe only. The CLI is **not** published as a separate Release asset (`capto-windows-*.exe` retired).

## GitHub Actions versions

Aligned with Tauri’s current pipeline docs + Node 24 runners:

- `actions/checkout@v7`
- `actions/setup-node@v6` with `node-version: "24"`
- `tauri-apps/tauri-action@v1`

```powershell
cargo test --workspace
npm ci --prefix apps/desktop
npx --prefix apps/desktop tsc --noEmit -p apps/desktop
.\scripts\download-ffmpeg.ps1 -VerifyAttestation
cargo check -p capto-app -p capto-cli --target aarch64-pc-windows-msvc
```
