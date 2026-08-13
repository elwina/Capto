# Capto — Social Launch Copy (EN / 中文)

Sizes verified against the **v1.0.0** release assets (GitHub API):
`Capto_1.0.0_x64-setup.exe` = **10.14 MB** · `Capto_1.0.0_arm64-setup.exe` = **8.66 MB**
(installer embeds the pinned FFmpeg sidecar + the `capto` CLI on PATH).

Targets: **X (Twitter)** · LinkedIn · 微博/即刻 · Mastodon. Every post ships in **English + 中文**.

---

## 1. X — main post（主帖）

**EN**

> Meet **Capto** — a **10 MB** Windows screen recorder. 🪟
>
> • Display / window / region capture, MP4 + GIF
> • Auto NVENC / QSV / AMF / libx264
> • Click & keystroke overlays, cursor toggle
> • Bundled FFmpeg, zero cloud, MIT
>
> 10 MB installer. Everything included.
> 👉 https://github.com/elwina/Capto
> #Windows #ScreenRecorder #OpenSource

**中文**

> Capto —— 只有 **10 MB** 的 Windows 录屏软件 🪟
>
> • 显示器 / 窗口 / 区域录制，MP4 + GIF
> • 自动 NVENC / QSV / AMF / libx264 硬编码
> • 点击与按键 Overlay、光标开关
> • 内置 FFmpeg，纯本地，MIT 开源
>
> 10 MB 安装包，开箱即用。
> 👉 https://github.com/elwina/Capto
> #Windows #录屏 #开源

---

## 2. X — thread reply 1（跟帖 1：Agent 原生）

**EN**

> Capto is **agent-native** 🤖
>
> A `capto` CLI (JSON, stable exit codes) over a localhost control plane means any AI agent can
> `doctor → record → stop → collect outputs` in seconds.
>
> • **Agent Skill** (SKILL.md, Agent Skills standard) — import into WorkBuddy / CodeBuddy / Claude Code / Cursor
> • **DeepSeek Harness plugin** — 14 first-class `capto_*` tools in your dsh profile
>
> npm: `capto-agent-skill` + `capto-dsh-plugin`
> #AI #AgentSkills #DeepSeek #WorkBuddy

**中文**

> Capto 是 **Agent 原生** 的 🤖
>
> `capto` CLI（JSON 稳定退出码）+ 本机控制面，任何 AI agent 都能
> `doctor → record → stop → 取输出` 一键走完。
>
> • **Agent Skill**（SKILL.md 标准）——可直接导入 WorkBuddy / CodeBuddy / Claude Code / Cursor
> • **DeepSeek Harness 插件** —— dsh profile 里的一等公民 `capto_*` 工具（14 个）
>
> npm：`capto-agent-skill` + `capto-dsh-plugin`
> #AI #AgentSkills #DeepSeek #WorkBuddy

---

## 3. X — thread reply 2（跟帖 2：为什么 10 MB）

**EN**

> 10 MB is the *whole* deal: a custom-built, pinned, attestation-verified FFmpeg sidecar + the agent
> CLI, inside one NSIS installer. No runtime, no bloat — a clean-room MIT spiritual successor to Captura.
>
> ```bash
> capto doctor      # env check
> capto record start --source display
> capto record stop
> ```

**中文**

> 10 MB 就是全部：自研、钉死版本、可验证的 FFmpeg 侧车 + agent CLI，一个 NSIS 安装包搞定。
> 无运行时、无冗余。Captura 的干净房 MIT 精神续作。
>
> ```bash
> capto doctor      # 环境检查
> capto record start --source display
> capto record stop
> ```

---

## 4. LinkedIn — post（领英）

**EN**

> Capto v1.0 is out — a **10 MB Windows screen recorder** that fits entirely inside one installer,
> FFmpeg and agent CLI included.
>
> It captures display / window / region to MP4 or GIF with automatic hardware encoding
> (NVENC / QSV / AMF / libx264), click & keystroke overlays, cursor control, and a pinned,
> attestation-verified FFmpeg sidecar. Local-only, MIT, no cloud, no telemetry.
>
> What makes it interesting for the AI crowd: Capto is **agent-native**. A `capto` CLI with a stable
> JSON contract lets any agent drive the desktop recorder — and we ship both an Agent Skills package
> (importable into WorkBuddy / CodeBuddy / Claude Code / Cursor) and a DeepSeek Harness plugin with
> 14 typed `capto_*` tools.
>
> Install: https://github.com/elwina/Capto/releases
> Skill: https://www.npmjs.com/package/capto-agent-skill
> DSH plugin: https://www.npmjs.com/package/capto-dsh-plugin
>
> #Windows #ScreenRecorder #OpenSource #AI #AgentSkills #DeepSeekHarness

**中文**

> Capto v1.0 发布了 —— 一个只有 **10 MB** 的 Windows 录屏软件，安装包自带 FFmpeg 与 agent CLI，
> 全部塞进一个安装程序里。
>
> 支持显示器 / 窗口 / 区域录制，输出 MP4 或 GIF，自动硬件编码（NVENC / QSV / AMF / libx264），
> 带点击与按键 Overlay、光标开关，以及一份版本钉死、可验证的 FFmpeg 侧车。纯本地、MIT、
> 无云、无遥测。
>
> 对 AI 生态更有意思的是：Capto 是 **Agent 原生**的。`capto` CLI 提供稳定的 JSON 契约，
> 任何 agent 都能驱动桌面录制；我们还同时提供了 Agent Skills 包（可导入 WorkBuddy /
> CodeBuddy / Claude Code / Cursor）和 DeepSeek Harness 插件（14 个类型化 `capto_*` 工具）。
>
> 安装：https://github.com/elwina/Capto/releases
> Skill：https://www.npmjs.com/package/capto-agent-skill
> DSH 插件：https://www.npmjs.com/package/capto-dsh-plugin
>
> #Windows #录屏 #开源 #AI #AgentSkills #DeepSeekHarness

---

## 5. 微博 / 即刻 — post（含 EN 备用）

**中文（主）**

> Capto —— 一个只有 **10 MB** 的 Windows 录屏软件 🪟
>
> • 显示器 / 窗口 / 区域录制，MP4 + GIF
> • 自动 NVENC / QSV / AMF / libx264 硬编码
> • 点击与按键 Overlay、光标开关
> • 内置 FFmpeg，纯本地，MIT 开源
>
> 10 MB 安装包，全部打包好，无需额外运行时。
> 👉 https://github.com/elwina/Capto
> #Windows #录屏 #开源

**EN（备用）**

> Capto — a **10 MB** Windows screen recorder 🪟
> Display / window / region capture, MP4 + GIF, hardware encoding, overlays, bundled FFmpeg.
> Local-only, MIT, agent-native (`capto` CLI + Agent Skill + DeepSeek Harness plugin).
> 👉 https://github.com/elwina/Capto

---

## Visual suggestions / 配图建议

- **Card 1**: app logo + "10 MB" stat + "Windows screen recorder"（用 `apps/desktop/public/capto-mark.png`）
- **Card 2**: a real recording screenshot（可用 `Videos\Capto` 里的输出）展示窗口/屏幕采集质量
- **Card 3**: terminal snippet（`capto doctor` → `record start` → `stop`）突出 agent-native 卖点
- Alt text: "Capto: a 10 MB local-only Windows screen recorder with agent integration."

## Posting checklist / 发布清单

- [ ] 确认网站 `https://elwina.github.io/Capto/` 可访问
- [ ] X 主帖发布后置顶 24 h
- [ ] 回复评论时带上 skill / plugin 的 npm 链接
- [ ] 中文版同日同步到微博 / 即刻
- [ ] LinkedIn 发布后顺手关注评论区互动
