# Capto

纯本地屏幕录制 / 截图应用——[Captura](https://github.com/MathewSachin/Captura) 的精神续作。

**栈：** Tauri 2 · Rust · React · TypeScript  
**平台：** Windows 10 1903+ 优先（架构已预留 macOS / Linux）

## 功能矩阵

### P0（MVP）

- 截图：显示器 / 窗口 / 区域
- 录屏：MP4（自动探测 NVENC / QSV / AMF / libx264）
- GIF 导出
- 光标开关、麦克风 + 系统环回混音
- 全局热键（开始 / 暂停 / 停止 / 截图）
- 托盘、录制时隐藏窗口
- 摄像头画中画进成片（dshow）
- 点击高亮 + 按键 overlay 进成片
- 本地 JSON 设置、中英 i18n、区域选择器

### P1

- Overlay：图片 / 文字静态烧录
- CLI：二进制 `capto` 经本机 HTTP 控制桌面（`status` / `record` / `shot` / `config` / `list` / `outputs` / `doctor`），默认 JSON，可自动拉起 Capto；Agent Skill 包 `capto-agent-skill`
- 热键自定义设置 UI（默认四键已注册）
- 桌面单实例（二次启动只聚焦已有进程）
- 应用内更新（`tauri-plugin-updater`，GitHub `updater` 频道；CDN/CF Worker 镜像后续）

### 明确不做

Imgur / YouTube / 上传、SharpAvi、BASS、Win7/GDI 主路径、社区周边、计时成片叠加、简易图编辑（裁剪 / 矩形 / 箭头 / 文字 / 模糊）

## 开发

```bash
# 依赖（锁定文件：apps/desktop/package-lock.json）
npm install --prefix apps/desktop

# 放置捆绑 FFmpeg（从本机已有安装复制，不联网下载）
# .\scripts\copy-ffmpeg.ps1
# → apps/desktop/src-tauri/binaries/ffmpeg.exe

# 桌面端
npm run tauri --prefix apps/desktop -- dev

# 测试
cargo test --workspace

# CLI（二进制名 `capto`；控制桌面会话，未开则自动启动）
cargo run -p capto-cli -- status
cargo run -p capto-cli -- list displays
cargo run -p capto-cli -- config get fps
cargo run -p capto-cli -- record start --source display
cargo run -p capto-cli -- record stop
cargo run -p capto-cli -- outputs recent --limit 5
# 开发时若找不到桌面：
# $env:CAPTO_APP_PATH = "D:\AIWorkspace\Capto\target\debug\capto-app.exe"
```

**文档**

| 文档 | 内容 |
|------|------|
| [AGENTS.md](AGENTS.md) | 仓库约定、agent 边界、二进制命名 |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 录制管线、单实例、控制面 |
| [docs/CLI.md](docs/CLI.md) | CLI 命令、JSON/退出码、agent 工作流 |
| [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md) | macOS / Linux 后端路线 |
| [docs/CI.md](docs/CI.md) | GitHub Actions：CI 与 Release（x64 + ARM64，FFmpeg 来自 capto-ffmpeg） |
| [packages/capto-agent-skill](packages/capto-agent-skill) | 可发布到 npm 的 Agent Skill（`capto-agent-skill`） |

## 许可

MIT（全新实现，不 fork Captura 源码）
