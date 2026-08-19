# Known tech debt

Deliberate, tracked debt. Items here are oversized work that predate the
current quality gates; each is bundled with the validation it needs. This is
not a bug list — it is the backlog beside `scripts/scan-tech-debt.ps1`
(zero-marker policy keeps *new* ad-hoc debt out of source).

| Item | Location | Why deferred | Plan / ownership |
|------|----------|--------------|------------------|
| `MainApp` session shell | `apps/desktop/src/App.tsx` | Complexity 94; a session-orchestration monolith wiring ~30 state setters with Tauri events. Correct refactor needs live WGC/WASAPI recording to validate, which CI cannot provide. | Split per-domain loaders (`refresh`), extract event-handler bundles, cap complexity at 20 as blocks land. Owner: maintainer. |
| `refresh` hydration callback | `apps/desktop/src/App.tsx` | Complexity 22; hydrates ~15 independent settings in one async pass with per-domain fallbacks. | Split into `loadDisplays/settings/encoders` helpers. Owner: maintainer. |

Both functions carry an `eslint-disable-next-line complexity` while the rest of
the codebase is capped at 20 (`complexity: ["error", 20]` in
`apps/desktop/eslint.config.js`). New functions that exceed the cap will fail
`npm run lint`.
