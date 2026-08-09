<p align="center">
  <img src="apps/desktop/public/capto-mark.png" alt="Capto" width="220" />
</p>

<h1 align="center">Capto</h1>

<p align="center">
  <strong>超轻量 Windows 屏幕录制</strong><br />
  <a href="https://github.com/MathewSachin/Captura">Captura</a> 的精神续作。
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="website/index.html">Website</a> ·
  <a href="https://github.com/elwina/Capto/releases">Releases</a>
</p>

## 为什么选 Capto

| | |
|:---:|:---|
| 🪟 | **模式 + Windows 特调** — 显示器 / 窗口 / 区域；Windows 优先采集与音频路径，而不是泛平台凑合实现。 |
| 🎬 | **MP4 与 GIF** — 录制 MP4（自动 NVENC / QSV / AMF / libx264）或导出 GIF。 |
| ✨ | **Overlay** — 鼠标点击高亮、按键显示、摄像头画中画，以及光标开关与实时预览。 |
| 🎞️ | **自研捆绑 FFmpeg（`capto-ffmpeg`）** — Capto 自有、版本钉死、可验证的 FFmpeg 侧车；编码只走这一份，不碰系统 PATH。 |
| 🤖 | **CLI + Skill，面向 AI** — 完备的 `capto` 本机控制面与 [`capto-agent-skill`](https://www.npmjs.com/package/capto-agent-skill)。 |
| 🔒 | **开源 · 本地 · 安全** — MIT；无上传 SDK；文件只留在本机。 |

## 安装

从 [Releases](https://github.com/elwina/Capto/releases) 下载 x64 / arm64 NSIS 安装包。内嵌经验证的 FFmpeg（[`capto-ffmpeg`](https://github.com/elwina/capto-ffmpeg)）以及 CLI：`<安装目录>\cli\capto.exe`；安装时会把该 `cli` 目录写入用户 **PATH**，新开终端即可直接运行 `capto`（不再单独发布 CLI）。

## 开发者

**Elwina Vardal** · [elwina.work](https://www.elwina.work) · [GitHub](https://github.com/elwina)

## 许可

MIT（全新实现，不 fork Captura 源码）
