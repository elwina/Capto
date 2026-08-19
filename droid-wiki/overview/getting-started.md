# Getting started

This page covers prerequisites, the one-command dev setup, and the build/test/run loop for the Capto repository. Everything is Windows-oriented: recording, the CLI control plane, and packaging all assume Windows 10+.

## Prerequisites

- Windows 10 1903+ (x64 or arm64)
- Rust toolchain (edition 2021; `cargo` must be on PATH)
- Node.js 18+ (the frontend and the npm packages use modern tooling; CI runs Node 24)
- PowerShell 5.1+ for the repo scripts
- Git

Nothing else is required: FFmpeg is downloaded as a pinned sidecar during setup, and npm dependencies come from `apps/desktop/package.json`.

## One-command setup

From a fresh clone, this gets you to a runnable `tauri dev`:

```powershell
.\scripts\setup-dev.ps1     # or -Local to prefer a local ffmpeg.exe
```

`setup-dev.ps1` performs, in order (same steps as `README.md`):

```bash
npm install --prefix apps/desktop
.\scripts\download-ffmpeg.ps1   # pinned capto-ffmpeg sidecar + attestation verify (or copy-ffmpeg.ps1)
cargo build -p capto-cli --release
.\scripts\copy-cli.ps1          # stage CLI into the app bundle (required for tauri build / package)
```

## Common commands

```powershell
npm run tauri --prefix apps/desktop -- dev        # run the desktop app (capto-app)
cargo test --workspace                            # all Rust unit tests
cargo fmt --all --check                           # rustfmt gate
cargo run -p capto-cli -- status                  # talk to a running desktop
npm run lint --prefix apps/desktop                # ESLint (max-warnings 0)
npm run format:check --prefix apps/desktop        # Prettier
npm test --prefix apps/desktop                    # Vitest unit tests
npm run test:coverage --prefix apps/desktop       # unit tests + enforced coverage gate
npm run duplicate:check --prefix apps/desktop     # jscpd copy-paste gate
npm run build:analyze --prefix apps/desktop       # bundle report (dist/analyze.html)
npm run bundle-size --prefix apps/desktop         # bundle-size budget gate
```

Interactive QA against a real desktop session:

```powershell
.\scripts\qa-smoke.ps1                             # doctor/status/config/outputs loop
.\scripts\qa-smoke.ps1 -RunRecordRoundtrip         # + a real record → stop → outputs
```

See [docs/QA.md](../../docs/QA.md) for the full interactive path and [docs/CI.md](../../docs/CI.md) for what CI enforces.

## Optional environment variables

| Variable | Purpose |
|----------|---------|
| `CAPTO_APP_PATH` | Path to a prebuilt `capto-app.exe`; the CLI uses it to auto-launch the desktop when the control plane is down (dev convenience; also set by `scripts/profile.ps1`) |
| `RUST_LOG` / `CAPTO_LOG` | Tracing filter for the CLI (`warn` default) and the desktop (`capto=debug,capto_core=debug,capto_encode=debug,warn` default) |
| `RUST_BACKTRACE` | Standard Rust backtraces on panic |

The full list lives in `.env.example`.

## Release build and packaging

Packaging requires the local FFmpeg sidecar and a staged CLI in the bundle:

```powershell
.\scripts\copy-ffmpeg.ps1   # or download-ffmpeg.ps1
cargo build -p capto-cli --release
.\scripts\copy-cli.ps1
npm run tauri --prefix apps/desktop -- build
```

The NSIS installer embeds FFmpeg and places the CLI at `<install>\cli\capto.exe`, adding that folder to the user PATH via the NSIS hooks in `apps/desktop/src-tauri/windows/hooks.nsh`. See [Deployment](../deployment.md) for the CI-driven release pipeline.

## Tips for running the CLI in dev

- Point `CAPTO_APP_PATH` at `target/debug/capto-app.exe` so `capto` can auto-launch the dev desktop.
- If a command returns exit code `2` (`desktopUnavailable`), run `capto open` first, or start Capto from the Start menu.
- Always `record stop` when finished; recordings have no auto stop.
- Never log the Bearer token read from `%APPDATA%\Capto\cli-server.json`.
