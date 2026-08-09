# Capto brand assets

Source artwork for Capto (Captura-inspired mark, purple primary).

| File | Use |
|------|-----|
| `capto-mark.png` | Logo mark only (1:1) |
| `capto-lockup.png` | Logo + wordmark “CAPTO” (horizontal) |
| `capto-logo.png` | Same lockup variant (approved white-bg master) |
| `capto-app-icon.png` | App icon master (1024×1024) → feeds Tauri |

App / installer icons are generated into `apps/desktop/src-tauri/icons/` via:

```bash
npm run tauri --prefix apps/desktop -- icon brand/capto-app-icon.png
```
