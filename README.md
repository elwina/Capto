<p align="center">
  <img src="apps/desktop/public/capto-mark.png" alt="Capto" width="220" />
</p>

<h1 align="center">Capto</h1>

<p align="center">
  <strong>Ultra-light Windows screen recorder</strong><br />
  Spiritual successor to <a href="https://github.com/MathewSachin/Captura">Captura</a>.
</p>

<p align="center">
  <a href="https://github.com/elwina/Capto/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/elwina/Capto/ci.yml?branch=main&style=flat-square&label=CI" alt="CI" /></a>
  <a href="https://github.com/elwina/Capto/releases/latest"><img src="https://img.shields.io/github/v/release/elwina/Capto?include_prereleases&style=flat-square&label=release" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License" /></a>
  <a href="https://github.com/elwina/Capto/releases"><img src="https://img.shields.io/github/downloads/elwina/Capto/total?style=flat-square" alt="Downloads" /></a>
  <a href="https://www.npmjs.com/package/capto-agent-skill"><img src="https://img.shields.io/npm/v/capto-agent-skill?style=flat-square&label=capto-agent-skill" alt="npm" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2B-0078D4?style=flat-square&logo=windows&logoColor=white" alt="Windows" />
  <img src="https://img.shields.io/badge/stack-Tauri%202%20%7C%20Rust%20%7C%20React-9B7BFF?style=flat-square" alt="Stack" />
</p>

<p align="center">
  <a href="README.zh-CN.md">中文</a> ·
  <a href="https://elwina.github.io/Capto/">Website</a> ·
  <a href="https://github.com/elwina/Capto/releases">Releases</a>
</p>

## Why Capto

| | |
|:---:|:---|
| 🪟 | **Capture modes + Windows-tuned stack** — Display / window / region, with a Windows-first path (DXGI / WASAPI and friends) instead of a generic lowest-common-denominator backend. |
| 🎬 | **MP4 & GIF** — Record to MP4 (auto NVENC / QSV / AMF / libx264) or export GIF. |
| ✨ | **Overlays** — Mouse-click highlights, keystroke overlay, webcam PiP, include-cursor, and live preview. |
| 🎞️ | **Rebuilt FFmpeg sidecar (`capto-ffmpeg`)** — Capto-owned, pinned, attested FFmpeg embedded in the app. Encoding goes only through this bundle — not whatever is on `PATH`. |
| 🤖 | **CLI + Agent Skill for AI** — Full `capto` control plane (JSON, stable exit codes) and [`capto-agent-skill`](https://www.npmjs.com/package/capto-agent-skill) so agents can doctor → record → stop → collect outputs. |
| 🔒 | **Open source · local · safe** — MIT, no upload SDKs, files stay on your machine. |

## Install

Download the NSIS setup for your CPU (**x64** / **arm64**) from [Releases](https://github.com/elwina/Capto/releases). Installers embed verified FFmpeg from [`capto-ffmpeg`](https://github.com/elwina/capto-ffmpeg) and the `capto` CLI at `<install>\cli\capto.exe`, and add that `cli` folder to your user **PATH** so `capto` works in new terminals (not a separate download).

## Develop

```bash
npm install --prefix apps/desktop
.\scripts\download-ffmpeg.ps1   # or copy-ffmpeg.ps1
cargo build -p capto-cli --release
.\scripts\copy-cli.ps1          # required for tauri build / package
npm run tauri --prefix apps/desktop -- dev
cargo test --workspace
cargo run -p capto-cli -- status
```

If the CLI cannot find the desktop in dev:

```powershell
$env:CAPTO_APP_PATH = "D:\AIWorkspace\Capto\target\debug\capto-app.exe"
```

## Author

**Elwina Vardal** · [elwina.work](https://www.elwina.work) · [GitHub](https://github.com/elwina)

## License

MIT (clean-room implementation — not a Captura fork)
