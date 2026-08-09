# Sidecars (FFmpeg + CLI)

Capto ships two extras next to the installed app:

| File | Role |
|------|------|
| `ffmpeg.exe` (Tauri `externalBin`) | Encoding only — never system `PATH` |
| `cli/capto.exe` (Tauri `resources` + NSIS PATH hook) | Agent/CLI control-plane client |

`cli/capto.exe` is **not** placed beside `Capto.exe`: Windows paths are case-insensitive, so `Capto.exe` and `capto.exe` cannot share a folder.

## Developer setup

### FFmpeg

```powershell
.\scripts\download-ffmpeg.ps1
# or local binary:
.\scripts\copy-ffmpeg.ps1
```

Cross-target package:

```powershell
.\scripts\download-ffmpeg.ps1 -TargetTriple aarch64-pc-windows-msvc
```

Writes:

- `ffmpeg.exe` — `tauri dev`
- `ffmpeg-<target-triple>.exe` — `externalBin`

### CLI

```powershell
cargo build -p capto-cli --release
.\scripts\copy-cli.ps1
```

Writes `capto.exe` (and `capto-<triple>.exe` for bookkeeping). Required before `tauri build` / release packaging.

Do **not** commit these binaries (see root `.gitignore`).

## Release

GitHub Actions **Release** workflow:

1. `scripts/download-ffmpeg.ps1` per target
2. `cargo build -p capto-cli` + `scripts/copy-cli.ps1`
3. Tauri NSIS package (embeds both; `windows/hooks.nsh` adds `<install>\cli` to user PATH)

Standalone `capto-windows-*.exe` assets are **not** uploaded to GitHub Releases.

Pin / provenance for FFmpeg: `.github/capto-ffmpeg.env` + [docs/CI.md](../../../docs/CI.md).

## Suggested FFmpeg capabilities

Prefer a full GPL build with `gdigrab`, `libx264`, NVENC/QSV/AMF, `gif` + palette filters, and AAC.
