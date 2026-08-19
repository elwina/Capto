# capto-dsh-plugin

Active contributors: elwina

`capto-dsh-plugin` is a [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) (dsh) plugin that registers 14 typed `capto_*` tools. Each tool is a thin wrapper over the `capto` CLI control plane, so a harness agent can drive the local-only Capto screen recorder from a conversation, no shelling out, no FFmpeg, no cloud. The package is plain ESM with zero build step and works from npm, a tarball, or a git checkout as-is.

## Purpose

Agents inside a harness need structured tools, not free-form shell prompts. This plugin turns the `capto` CLI contract into first-class typed tools with strict parameter schemas, canonical JSON output, and normalized failures. Every CLI exit is mapped to an `Error` the model can branch on, so a cold desktop or a failed FFmpeg start becomes an actionable instruction rather than noise.

## The 14 tools

Tool definitions live in `packages/capto-dsh-plugin/src/tools.js`. Read-only tools are marked concurrency-safe; every tool renders its canonical JSON output.

| Tool | Purpose |
|------|---------|
| `capto_status` | Session snapshot `{ state, elapsedMs, outputPath, … }`; check before recording |
| `capto_doctor` | Environment readiness `{ ffmpegOk, controlPlane, … }` |
| `capto_open` | Open the Capto desktop window (recovery entry for exit 2) |
| `capto_list` | Enumerate `displays` / `windows` / `audio` / `encoders` |
| `capto_shot` | Screenshot → `{ path }` (absolute PNG) |
| `capto_record_start` | Start recording (source, display/window/region, format, fps, quality, encoder, mic, loopback, cursor) |
| `capto_record_stop` | Stop the current recording; returns the final snapshot with `outputPath` |
| `capto_record_pause` | Pause the current recording |
| `capto_record_resume` | Resume a paused recording |
| `capto_config_get` | Read a settings key (or the full object) |
| `capto_config_set` | Patch settings via `json` and/or `pairs` (camelCase keys) |
| `capto_config_path` | Absolute path of the Capto `settings.json` |
| `capto_outputs_recent` | Recent output files (`limit`, default 20) |
| `capto_outputs_open` | Open an output file or folder in Explorer (`path`, `last`, `folder`) |

## Installation

### npm (into a profile)

```bash
npm install --prefix ~/.dsh/profiles/web capto-dsh-plugin
```

Enable it by inserting a patch row in the profile's `cordis.patch.yml` (reproduced in `packages/capto-dsh-plugin/cordis.patch.yml`):

```yaml
- insert:
    - id: capto
      name: capto-dsh-plugin
      config:
        command: ['capto']
        timeoutMs: 120000
        noLaunch: false
        autoOpen: false
```

Verify with `dsh --profile web --dump-config`, then restart the GUI so the `capto_*` tools appear.

### dsh plugin (bundle form)

The package declares a `dsh.bundle` manifest (`packages/capto-dsh-plugin/package.json`, pointing at `cordis.patch.yml`), so the pnpm-based flow wires everything:

```bash
dsh plugin --profile web add capto-dsh-plugin
dsh --profile web --dump-config   # a "# == capto-dsh-plugin" layer appears
```

Overrides later in the layer chain replace a row's whole config (not merged), so any override must restate every key it keeps. The npm-based flow above is the equivalent for npm-managed profiles. Uninstall with `npm uninstall --prefix ~/.dsh/profiles/web capto-dsh-plugin` or `dsh plugin --profile web remove capto-dsh-plugin`, and drop the patch row.

## Configuration

Config is validated at plugin load by schemastery (`Config` in `packages/capto-dsh-plugin/src/index.js`); invalid values (empty `command`, non-positive `timeoutMs`, non-boolean flags) fail with an actionable error.

| Key | Default | Meaning |
|-----|---------|---------|
| `command` | `['capto']` | CLI argv prefix: `['capto']` (installed PATH), `['D:\\…\\capto.exe']`, or `['cargo','run','-p','capto-cli','--']` in a dev checkout |
| `timeoutMs` | `120000` | Per-call timeout in ms; must exceed the CLI's 45 s desktop auto-launch wait so a cold start can finish |
| `noLaunch` | `false` | Always pass `--no-launch`, never auto-start the Capto desktop |
| `autoOpen` | `false` | On exit 2 (`desktopUnavailable`): run `capto open`, wait ~3 s, retry once |

When `noLaunch` is true, the plugin prefixes the runtime command with `--no-launch`.

## Model experience

- **What the model sees**, the 14 tool schemas plus a short `tool:capto` prompt section (order 110) teaching the agent to check `capto_status` before starting, never start twice, recover exit 2 with `capto_open`, always end with `capto_record_stop`, and find files with `capto_outputs_recent`.
- **Failures**, every CLI exit is normalized to `Error: capto exited <code> (<errorCode>): <message>`. `desktopUnavailable` (exit 2) messages tell the model its next step (`capto_open`, wait ~3–5 s, retry; ask the user if it still fails). The normalization lives in `packages/capto-dsh-plugin/src/capto.js` (`CaptoError` and `runCapto`).
- **Cancellation**, calls observe the tool's abort signal; an aborted call raises `AbortError` (`abortError()` in `packages/capto-dsh-plugin/src/capto.js`) and never leaves an orphan CLI child.

## Development and verification

```bash
cd packages/capto-dsh-plugin
npm install            # test deps (@deepseek-ai/dsh-tools, schemastery)
node test/smoke.mjs    # 11 checks: contract, config, arg mapping, failure normalization, timeout, autoOpen recovery, real CLI
```

`packages/capto-dsh-plugin/test/fixtures/fake-capto.mjs` fakes a `capto` CLI speaking the JSON envelope contract, so the suite runs without a desktop (modes: `FAKE_CAPTO_BOOM`, `FAKE_CAPTO_SLEEP_MS`, `FAKE_CAPTO_MARKER`, `FAKE_CAPTO_ECHO`). When `target/debug/capto.exe` exists, one real-CLI check runs and accepts either control-plane state. `prepublishOnly` runs the smoke suite so a broken plugin cannot be published.

### CI matrix

CI runs the suite against the freshly built CLI on Windows and fake-only on Ubuntu, plus `npm pack --dry-run` (`docs/CI.md`).

## Known limitations

- **Windows-only by design**, Capto is a Windows screen recorder; other platforms gain nothing.
- **One desktop session**, the tools drive the single Capto desktop process; there is no second capture pipeline (a Capto guarantee enforced by its control plane).
- **No image preview inside results**, `capto_shot` returns a path; image rendering in tool results is not yet available in the harness.
- **`autoOpen` is best-effort**, it retries once after `capto open`; a wedged desktop (stale process) still needs a manual kill and relaunch per the Capto skill docs.

## Integration points

- It wraps the same CLI binary as [capto-agent-skill](capto-agent-skill.md), the [CLI app page](../apps/cli.md) documents the contract both use.
- The loopback control plane is described under [Control-plane API](../api/index.md).
- The [website](../apps/website.md) promotes the agent integrations this plugin enables.

## Key source files

| File | Purpose |
|------|---------|
| `packages/capto-dsh-plugin/package.json` | Metadata, exports, `dsh.bundle` manifest, scripts |
| `packages/capto-dsh-plugin/cordis.patch.yml` | Bundle layering rules and neutral default config |
| `packages/capto-dsh-plugin/src/index.js` | Plugin entry: `{ name, inject, Config, apply }`, `tool:capto` prompt section |
| `packages/capto-dsh-plugin/src/tools.js` | The 14 `capto_*` tool definitions and arg mapping |
| `packages/capto-dsh-plugin/src/capto.js` | CLI runner, `CaptoError`, `abortError`, autoOpen recovery |
| `packages/capto-dsh-plugin/test/smoke.mjs` | Offline smoke suite (11 checks) |
| `packages/capto-dsh-plugin/test/fixtures/fake-capto.mjs` | Fake `capto` CLI speaking the envelope contract |
