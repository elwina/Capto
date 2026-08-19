# Reference

Reference material that documents the persistent shape of a Capto install: the settings file, the core data-model types those settings and the runtime produce, and the dependency graph that builds and runs the app.

## Sub-pages

| Page | Summary |
|------|---------|
| [Configuration](configuration.md) | `settings.json`, the only config surface: every key with its default, feature flags, the CLI `config get/set` workflow, and the other files in the config dir. |
| [Data models](data-models.md) | The serde (camelCase) Rust types that travel through the app and the control plane, plus output-naming and crash-report shapes. |
| [Dependencies](dependencies.md) | Rust workspace and frontend npm dependencies, the pinned FFmpeg sidecar, and external platform requirements. |

## Canonical source documents

These maintain authoritative contract details:

- `docs/settings-schema.json` is the machine-readable JSON Schema for `settings.json`; use it to validate or edit the file. The mapping and defaults live in `crates/capto-core/src/settings.rs`.
- `docs/CLI.md` documents the CLI/envelope/exit-code JSON contract used by `capto` and agents (English and 中文).
- `docs/ARCHITECTURE.md` describes the recording pipeline and the control-plane contracts between the desktop, the CLI, and agents.

See [Glossary](../overview/glossary.md) for the vocabulary these pages use (`RecordingSession`, control plane, DXGI pump, feature flag, and so on). Crate-level detail that overlaps these references is expanded in [capto-core](../crates/capto-core.md), [capto-capture](../crates/capto-capture.md), [capto-ipc](../crates/capto-ipc.md), and [capto-overlay](../crates/capto-overlay.md).
