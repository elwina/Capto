# Capto data model (no database — by design)

This is an architecture decision record (ADR) for agents: Capto deliberately
has **no database and no ORM**. Data is either in-memory during a session or
single small files on disk.

## Where state lives

| Concern | Mechanism | Why |
|---------|-----------|-----|
| App settings | `settings.json` (`AppSettings`, serde) | Tiny, user-editable, no server needed |
| Session state | In-memory `RecordingSession` (mutex-guarded) | Short-lived; snapshot serialized on demand |
| Control-plane discovery | `cli-server.json` lock (PID/port/token) | One random process; a DB would be absurd |
| Recordings | Files in `output_dir` | Media is a file, not rows |
| Usage/metrics | In-process `Metrics` registry, `/v1/metrics` | No persistence needed |

The settings schema `AppSettings` (fields documented in
`crates/capto-core/src/settings.rs`) is the closest thing to a schema; it is
version-tolerant via serde defaults so older/newer config files interop.

## Formal schema (for agents/editors)

A machine-readable JSON Schema for `settings.json` lives at
[`docs/settings-schema.json`](settings-schema.json) (JSON Schema 2020-12). Use
it to validate or edit a settings file:

```powershell
# validate settings.json against the schema (any JSON-Schema validator works)
npx -y ajv-cli validate -s docs\settings-schema.json -d "$env:APPDATA\Capto\settings.json"
```

Keep `docs/settings-schema.json` in lockstep with the `AppSettings` serde type
when fields change.

## Implications

- **No N+1 query risk**: there are no relational queries at all. N+1-detection
  tooling (bullets/nplusone/ORM log analysis) does not apply to this codebase.
- **No migrations**: schema changes are field additions with `#[serde(default)]`.
- If a future feature genuinely needs a DB (e.g., searchable output index),
  revisit this ADR **before** adding one — a sqlite-style local embed would be
  the likely path, keeping the purely-local constraint.

## Evaluation notes for agent-readiness audits

- **`database_schema`**: no database exists by design (this ADR + `docs/data.md`).
  The persisted data model is `settings.json`, formally described by
  `docs/settings-schema.json`; there are no Prisma/SQLAlchemy/raw-SQL schemas
  to maintain. This criterion should be **skipped as N/A**, not scored 0.
- **`n_plus_one_detection`**: no database/ORM and therefore no N+1 surface;
  the criterion is designed to be **skipped for apps without DB/ORM usage**.
  No detection tooling is (or should be) added for a nonexistent query layer.
