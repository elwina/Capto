# Capto brand assets

Source artwork for Capto (Captura-inspired mark, purple primary).

| File | Use |
|------|-----|
| `capto-mark.png` | Logo mark only (1:1) |
| `capto-lockup.png` | Logo + wordmark “CAPTO” (horizontal) |
| `capto-logo.png` | Same lockup variant (approved white-bg master) |
| `capto-app-icon.png` | App icon master (1024×1024) → feeds Tauri |

## App icons

App / shortcut icons are generated into `apps/desktop/src-tauri/icons/` via:

```bash
npm run tauri --prefix apps/desktop -- icon ../../brand/capto-app-icon.png
```

## NSIS installer branding

Welcome/Finish sidebar and page header bitmaps live in `apps/desktop/src-tauri/windows/nsis/`:

| File | Size | Role |
|------|------|------|
| `sidebar.bmp` | 164×314 | Welcome + Finish pages |
| `header.bmp` | 150×57 | Install / uninstall page headers |

Installer/uninstaller `.ico` reuse `apps/desktop/src-tauri/icons/icon.ico` (wired in `tauri.conf.json`).

Regenerate BMPs after changing the mark:

```powershell
.\scripts\gen-nsis-brand.ps1
```
