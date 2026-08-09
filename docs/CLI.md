# Capto CLI

<p>
  <strong>English</strong> · <a href="#zhongwen">中文</a>
</p>

Agent-oriented control surface for Capto. The CLI binary is named **`capto`**. It talks to the **running desktop app** (`capto-app` / installed Capto) over a localhost HTTP control plane. It does **not** create a second `RecordingSession`.

Full architecture: [ARCHITECTURE.md](ARCHITECTURE.md).  
Agent skill (npm): [`packages/capto-agent-skill`](../packages/capto-agent-skill).

## Mental model

```
Agent / shell
    → capto  (JSON stdout, stable exit codes)
        → 127.0.0.1:<port> + Bearer token
            → Capto desktop (single process)
                → RecordingSession / settings / outputs
```

| Rule | Detail |
|------|--------|
| CLI binary | `capto` (crate `capto-cli`) |
| Desktop binary | `capto-app` in cargo builds; product name Capto |
| Single session | One Capto process machine-wide |
| Auto-launch | If control plane is down, CLI starts desktop (unless `--no-launch`) |
| Discovery | `%APPDATA%\Capto\cli-server.json` |
| No upload | Local files only |

> **Why not both named `capto` in the same folder?** Cargo would write two `target/debug/capto.exe`. On Windows, `Capto.exe` / `capto.exe` also collide (case-insensitive). So CLI owns `capto`; desktop crate is `capto-app`. The **installer** places the CLI at `<install>\cli\capto.exe` and adds that `cli` folder to user **PATH** — not a separate Release download.

## Invoke

```bash
# After Capto install (cli\ is on PATH — open a new terminal)
capto <command> [args]

# From this repo
cargo run -p capto-cli -- <command> [args]
# → builds/runs target/debug/capto.exe
```

Global flags:

| Flag | Meaning |
|------|---------|
| (default) | JSON envelope on stdout |
| `--human` | Pretty data only (no envelope) |
| `--no-launch` | Fail if desktop control plane is down |

Dev auto-launch:

```powershell
$env:CAPTO_APP_PATH = "D:\AIWorkspace\Capto\target\debug\capto-app.exe"
```

Lookup order: `CAPTO_APP_PATH` → `Capto.exe` / `capto-app.exe` beside CLI or one level up (`cli\`) → `target/debug|release/capto-app.exe` → common install paths.

## JSON envelope (agent contract)

**Success**

```json
{ "ok": true, "data": { } }
```

**Failure**

```json
{ "ok": false, "error": { "code": "desktopUnavailable", "message": "…" } }
```

- stdout = envelope (or `--human` data)
- stderr = tracing (ignore for parsing)
- `data` fields are **camelCase**

### Exit codes

| Code | Name | When |
|------|------|------|
| 0 | ok | Success |
| 1 | usage | Bad args / unknown key |
| 2 | desktopUnavailable | No control plane / auth / launch failed |
| 3 | stateConflict | e.g. start while already recording |
| 4 | capture | Capture / device failure |
| 5 | encode | FFmpeg / encoder failure |
| 6 | configIo | Settings / outputs filesystem error |

Branch on **exit code first**, then `error.code`.

## Commands

### `doctor`

```bash
capto doctor
```

Environment / FFmpeg / control-plane readiness.

### `status`

```bash
capto status
```

States: `idle` | `starting` | `recording` | `paused` | `stopping`.

### `list`

```bash
capto list displays
capto list windows
capto list audio
capto list encoders
```

### `config`

```bash
capto config path
capto config get
capto config get fps
capto config set fps=60
capto config set --json "{\"fps\":60,\"includeCursor\":true}"
```

Keys are **camelCase**. Overlay tweaks via `--json` on `overlays`.

### `shot`

```bash
capto shot --source display
capto shot --source display --display 0
capto shot --source window --window <hwnd>
capto shot --source region --x 0 --y 0 --width 1280 --height 720
```

Returns `data.path` (absolute PNG).

### `record`

```bash
capto record start --source display
capto record start --source display --display 0 --format mp4 --fps 30
capto record pause
capto record resume
capto record stop
```

`record start` useful flags: `--source`, `--display`, `--window`, `--x/--y/--width/--height`, `--format mp4|gif|audio`, `--fps`, `--encoder`, `--mic`, `--loopback`, `--no-cursor`.

Always `record stop` when done (no duration auto-stop).

### `outputs`

```bash
capto outputs recent --limit 10
capto outputs open --last
capto outputs open --folder
```

## Agent workflows

### Record a short clip

```text
1. doctor
2. list displays
3. record start --source display
4. status          # poll
5. record stop
6. outputs recent --limit 1
```

### Screenshot

```text
1. shot --source display
2. use data.path
```

### Desktop already running

```bash
capto --no-launch status
```

## HTTP map

See [ARCHITECTURE.md](ARCHITECTURE.md). Shared types: `capto-ipc`.

## Agent skill (npm)

```bash
npm install capto-agent-skill
```

Ships `skills/capto/SKILL.md` per [Agent Skills](https://agentskills.io) + npm `skills/` convention.

## Not in CLI (yet)

- `quit` / close desktop
- Interactive pickers
- MCP server wrapper

## Safety

- Prefer `--no-launch` in headless CI
- Do not log the Bearer token from `cli-server.json`
- Do not double-`record start`
- Encode only through Capto

---

<a id="zhongwen"></a>

# Capto CLI（中文）

<p>
  <a href="#">English</a> · <strong>中文</strong>
</p>

面向 Agent 的控制面。CLI 二进制名为 **`capto`**，通过本机 HTTP 控制**已运行的桌面端**（`capto-app` / 安装版 Capto），**不会**再开第二个 `RecordingSession`。

架构见 [ARCHITECTURE.md](ARCHITECTURE.md)。  
npm Skill：[`packages/capto-agent-skill`](../packages/capto-agent-skill)。

## 心智模型

```
Agent / 终端
    → capto  （stdout JSON，稳定退出码）
        → 127.0.0.1:<port> + Bearer
            → Capto 桌面（单进程）
                → 录制会话 / 设置 / 输出
```

| 约定 | 说明 |
|------|------|
| CLI | `capto`（crate `capto-cli`） |
| 桌面 | 开发产物 `capto-app.exe`，产品名 Capto |
| 单实例 | 整机一个 Capto 进程 |
| 自动拉起 | 控制面不在时 CLI 会启动桌面（可用 `--no-launch` 禁止） |
| 发现 | `%APPDATA%\Capto\cli-server.json` |
| 不上云 | 只写本地文件 |

> 不要把桌面也命名为 `capto`：`target/debug` 会撞名；Windows 路径大小写不敏感也会冲突。安装包因此把 CLI 放到 `<安装目录>\cli\capto.exe`，并写入用户 **PATH**（新开终端可用 `capto`），**不再单独发布** CLI。

## 调用

```bash
capto <command> [args]
cargo run -p capto-cli -- <command> [args]
```

| 全局参数 | 含义 |
|----------|------|
| （默认） | stdout JSON 信封 |
| `--human` | 只打印可读数据 |
| `--no-launch` | 桌面未开则直接失败 |

开发环境：

```powershell
$env:CAPTO_APP_PATH = "…\target\debug\capto-app.exe"
```

## JSON 信封

成功：`{ "ok": true, "data": { } }`  
失败：`{ "ok": false, "error": { "code": "desktopUnavailable", "message": "…" } }`

`data` 字段为 **camelCase**。先看退出码，再看 `error.code`。

| 退出码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | 参数 / 用法错误 |
| 2 | 桌面不可用 |
| 3 | 状态冲突（如已在录制） |
| 4 | 采集失败 |
| 5 | 编码失败 |
| 6 | 配置 / 输出 IO |

## 命令速查

| 命令 | 作用 |
|------|------|
| `doctor` | 环境 / FFmpeg / 控制面 |
| `status` | 会话状态 |
| `list displays\|windows\|audio\|encoders` | 枚举设备 |
| `config get\|set\|path` | 读写设置 |
| `shot` | 截图 → `data.path` |
| `record start\|stop\|pause\|resume` | 录制 |
| `outputs recent\|open` | 最近文件 / 打开目录 |

典型录制流程：`doctor` → `list displays` → `record start` → `status` → `record stop` → `outputs recent`。

## 安全注意

- CI / 无界面环境用 `--no-launch`
- 不要把 lockfile 里的 token 打进日志
- 不要重复 `record start`
- 编码只走 Capto，不要自己起系统 FFmpeg
