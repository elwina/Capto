# FFmpeg sidecar

Capto ships a **bundled** `ffmpeg` next to the app (Tauri `externalBin`).
At runtime it uses **only** that copy — never system `PATH` / WinGet.

## Developer setup

Download the pinned Capto FFmpeg Release for the current Rust target:

```powershell
.\scripts\download-ffmpeg.ps1
```

For a cross-target package build, pass the exact Tauri/Rust target triple:

```powershell
.\scripts\download-ffmpeg.ps1 -TargetTriple aarch64-pc-windows-msvc
```

The script selects the matching Release asset and verifies it against that
Release's `SHA256SUMS` before copying it into this folder.

For a custom local FFmpeg build, copy an existing `ffmpeg.exe` instead:

```powershell
.\scripts\copy-ffmpeg.ps1
# or
.\scripts\copy-ffmpeg.ps1 -Source "C:\path\to\ffmpeg.exe"
```

Both scripts write:

- `ffmpeg.exe` — used in `tauri dev`
- `ffmpeg-<target-triple>.exe` — required by Tauri `externalBin`

Do **not** commit the binary (see root `.gitignore`).

## Release

GitHub Actions must run `scripts/download-ffmpeg.ps1` for the build target before
Tauri packages the app. The installer embeds that verified sidecar; end users do
not install FFmpeg separately.

## Suggested capabilities

Prefer a full GPL build with `gdigrab`, `libx264`, NVENC/QSV/AMF, `gif` + palette filters, and AAC.
