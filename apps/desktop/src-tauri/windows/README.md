# Windows NSIS packaging

| File | Role |
|------|------|
| `hooks.nsh` | PATH install/uninstall hooks for `cli\` |
| `installer.nsi` | Custom Tauri NSIS template (forked from tauri-v2.11.0) |
| `nsis/header.bmp` / `nsis/sidebar.bmp` | Installer branding |

## Fixed install directory

Capto always installs to `%LOCALAPPDATA%\Capto` (`installMode: currentUser`). The directory chooser page is skipped so CLI auto-launch, bundled FFmpeg, and updater stay on one path.

When rebasing `installer.nsi` onto a newer Tauri template, keep:

1. Directory page `MUI_PAGE_CUSTOMFUNCTION_PRE Skip`
2. Forced `StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"` for `currentUser` (no `RestorePreviousInstallLocation`)
