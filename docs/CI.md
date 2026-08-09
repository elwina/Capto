# Capto CI / Release

## Workflows

| Workflow | File | Trigger | Purpose |
|----------|------|---------|---------|
| **CI** | [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) | push/PR → `main` | Rust test + clippy, frontend `tsc`, `cargo check` for **x64 + ARM64**, FFmpeg pin download + attestation |
| **Release** | [`.github/workflows/release.yml`](../.github/workflows/release.yml) | tag `v*` or manual | NSIS installers for both arches with **embedded** FFmpeg; upload `capto` CLI assets |

CI and Release are intentionally separate: green CI does not publish; Release does not replace day-to-day checks.

## Versioning

- App / workspace version today: **0.1.0** (`Cargo.toml` workspace + `tauri.conf.json`)
- Git tags: `v0.1.0`, … — `v0.*` releases are marked **prerelease**
- First stable line: **1.0.0** (`v1.0.0`) when ready

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

| Display | Rust target (internal) | Installer | CLI asset |
|---------|------------------------|-----------|-----------|
| **x64** | `x86_64-pc-windows-msvc` | NSIS `.exe` | `capto-windows-x64.exe` |
| **arm64** | `aarch64-pc-windows-msvc` | NSIS `.exe` | `capto-windows-arm64.exe` |

Job titles and release assets use **x64 / arm64**. The `*-windows-msvc` strings are only Rust target triples (Windows ABI). **MSI is not built** — NSIS setup exe only.

## GitHub Actions versions

Aligned with Tauri’s current pipeline docs + Node 24 runners:

- `actions/checkout@v7`
- `actions/setup-node@v6` with `node-version: "24"`
- `tauri-apps/tauri-action@v1`
- `actions/upload-artifact@v5` / `actions/download-artifact@v5`

```powershell
cargo test --workspace
npm ci --prefix apps/desktop
npx --prefix apps/desktop tsc --noEmit -p apps/desktop
.\scripts\download-ffmpeg.ps1 -VerifyAttestation
cargo check -p capto-app -p capto-cli --target aarch64-pc-windows-msvc
```
