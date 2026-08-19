# Interactive QA: how an agent brings Capto up and verifies it works

Capto is a **Windows desktop** app, so the end-to-end interactive QA path is:
build/launch the desktop, then drive it through the **`capto` CLI control
plane** (a loopback HTTP server owned by the desktop). This path is fully
agent-followable and is what `scripts/qa-smoke.ps1` automates.

## 0. Prerequisites (fresh clone)

```powershell
.\scripts\setup-dev.ps1     # npm deps + FFmpeg sidecar + staged CLI (single command)
```

## 1. Launch the desktop into an interactive state

Option A — dev build (needs a Windows desktop session):

```powershell
npm run tauri --prefix apps/desktop -- dev
```

Option B — already-installed app: start **Capto** from the Start menu, or let
the CLI auto-launch it (`capto` launches the desktop when the control plane is
down and `CAPTO_APP_PATH` points at a built `capto-app.exe`).

There is **no login/account gate**: the loopback control plane uses a random
per-process bearer token written to `cli-server.json`; the CLI reads it
automatically, so an agent never needs credentials.

## 2. Verify the control plane

```powershell
capto doctor       # readiness probe: desktop + FFmpeg present, ok:true
capto status       # current session state, ok:true
cargo run -p capto-cli -- status   # same, from a dev checkout
```

Exit code `2` (desktop unavailable) means the desktop isn't running; start it
and retry. See `docs/CLI.md` for the exit-code contract.

## 3. Drive a meaningful interaction

```powershell
capto record start --source display --format mp4
capto status                      # observe state=Recording
capto record stop
capto outputs recent --limit 1    # last output written, ok:true
```

## 4. Automated smoke (what CI/harnesses use)

```powershell
.\scripts\qa-smoke.ps1                     # doctor/status/config/outputs
.\scripts\qa-smoke.ps1 -RunRecordRoundtrip # + real record/stop (needs a desktop session)
```

Every step must return a JSON envelope with `ok:true`; the script exits
non-zero otherwise.

## Headless notes

CI runners have no display session, so the record round-trip is skipped there;
agents on a real Windows desktop should run `-RunRecordRoundtrip`. The security
probe suite lives at `scripts/control-plane-dast.ps1`
(see `docs/security-testing.md`).
