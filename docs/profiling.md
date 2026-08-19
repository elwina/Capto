# Profiling Capto (local, privacy-first)

Capto never sends profiling data anywhere. All performance instrumentation is
**local**: timing lines in logs, counts/durations exposed over the loopback
`/v1/metrics` endpoint, and on-machine ETW sampling. Nothing here requires (or
permits) a cloud APM.

There are three tiers; most days you only need Tier 2.

## Tier 1 — Build time (compile-time hotspots)

Cargo can record how long each crate takes to build:

```powershell
cargo build -p capto-app --release --timings
# report:
#   target\cargo-timings\cargo-timings-*.html
```

`.\scripts\profile.ps1` is a thin wrapper that runs the `--timings` build and
prints the report path. CI already runs `cargo build --workspace --timings`
and uploads `target/cargo-timings/` as an artifact every run, so build-duration
drift across commits is visible in Actions.

## Tier 2 — Runtime hot paths (frame pump + control plane)

The recording hot path is the **frame pump**: the DXGI capture thread hands raw
frames to this process, which writes them to the bundled FFmpeg's `stdin`
(`crates/capto-core/src/session.rs`, `attach_dxgi_pump`). The pump emits two
timing lines (debug level):

- `slow ffmpeg write - capture outrunning encoder` — a single rawvideo write
  blocked ≥ 250 ms. This is the classic cause of dropped/frozen-output
  reports: capture is producing frames faster than the encoder can drain.
- `frame pump finished` — totals at pump teardown: count of frames written,
  number of slow writes, and total loop duration.

Enable them by launching the **desktop** with the filter set (the desktop
process owns the pump, so the env var must be on that process):

```powershell
$env:CAPTO_LOG = "capto_core=debug,capto=warn"
.\target\debug\capto-app.exe        # or the installed Capto
```

Then drive a recording (CLI or UI), stop it, and inspect the desktop's stderr /
log for those lines. `.\scripts\profile.ps1 -RunRuntime` automates the
record→stop round trip via `qa-smoke.ps1`.

Control-plane call latency is already aggregated in the local metrics registry:
`/v1/metrics` exposes `http_request_duration_ms` (avg/max) and per-status
counts, guarded by the `control-plane-metrics` feature flag.

## Tier 3 — CPU sampling / flamegraph (Windows ETW)

For whole-process CPU attribution (which Rust function burns the CPU while
encoding), use the OS sampler — no third-party tooling:

```
wpr -start GeneralProfile -filemode -start   # admin shell
<exercise Capto for a few seconds>
wpr -stop profile.etl
```

Open `profile.etl` in **Windows Performance Analyzer** (WPA): *CPU Usage
(Sampled)* → *Stack* → *Load Symbols*. The stack tree is the flamegraph view
of `capto-app.exe`. `.\scripts\profile.ps1 -EtfGuide` prints these commands.

## Checklist when investigating a slowness report

1. `./scripts/profile.ps1` — build-time drift (Tier 1).
2. `-RunRuntime` — are there `slow ffmpeg write` lines in the desktop log?
   If yes, it's capture outrunning the encoder: lower capture fps / resolution
   or switch to a hardware encoder (`capto config set encoder=h264_nvenc`).
3. If both are clean, ETW-sample (Tier 3) and look for hot frames in
   `capto-capture` (DXGI) or `capto-encode` (libav).
