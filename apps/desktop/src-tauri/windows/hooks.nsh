; Capto NSIS installer hooks — put CLI on PATH so `capto` works in new terminals.
; Only `$INSTDIR\cli` is added (never `$INSTDIR`), so Capto.exe cannot shadow `capto`
; on case-insensitive Windows.
;
; PATH manipulation uses the EnVar NSIS plugin. EnVar reads/writes the registry
; through the Win32 API, so it is not limited by NSIS_MAX_STRLEN (1024 bytes) and
; will never truncate or overwrite a long user PATH. It also de-dupes on add and
; removes exactly on delete.
;
; The plugin DLL lives at `nsis\plugins\x86-unicode\EnVar.dll` next to this file.
; `!addplugindir` must appear before any EnVar command in the generated installer,
; so it goes here at the top (Tauri's installer.nsi includes this file via an
; absolute path and already calls `!addplugindir` for its own plugins).
;
; This file is !include'd after StrFunc.nsh + ${StrLoc} in installer.nsi, but before
; PRODUCTNAME / UNINSTKEY / INSTALLMODE. Anything needing those defines must live in
; NSIS_HOOK_* macros (expanded later).
;
; Wired from tauri.conf.json → bundle.windows.nsis.installerHooks

!addplugindir "${__FILEDIR__}\nsis\plugins\x86-unicode"

!include "LogicLib.nsh"
!include "WinMessages.nsh"

Var /GLOBAL CaptoPathIsMachine

!macro CaptoSetPathHive
  StrCpy $CaptoPathIsMachine "0"
  !if "${INSTALLMODE}" == "perMachine"
    StrCpy $CaptoPathIsMachine "1"
  !else if "${INSTALLMODE}" == "both"
    ${If} $MultiUser.InstallMode == "AllUsers"
      StrCpy $CaptoPathIsMachine "1"
    ${EndIf}
  !endif
  ${If} $CaptoPathIsMachine == "1"
    EnVar::SetHKLM
  ${Else}
    EnVar::SetHKCU
  ${EndIf}
!macroend

; Adds $INSTDIR\cli to PATH (idempotent).
!macro NSIS_HOOK_POSTINSTALL
  !insertmacro CaptoSetPathHive
  EnVar::AddValue "Path" "$INSTDIR\cli"
  Pop $0
  ${If} $0 == 0
    DetailPrint "Added Capto CLI to PATH: $INSTDIR\cli"
  ${Else}
    DetailPrint "EnVar::AddValue for $INSTDIR\cli returned $0"
  ${EndIf}
  WriteRegStr SHCTX "${UNINSTKEY}" "CaptoCliPath" "$INSTDIR\cli"
!macroend

; Removes $INSTDIR\cli from PATH.
!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CaptoSetPathHive
  ReadRegStr $R9 SHCTX "${UNINSTKEY}" "CaptoCliPath"
  ${If} $R9 == ""
    StrCpy $R9 "$INSTDIR\cli"
  ${EndIf}
  EnVar::DeleteValue "Path" $R9
  Pop $0
  ${If} $0 == 0
    DetailPrint "Removed Capto CLI from PATH: $R9"
  ${Else}
    DetailPrint "EnVar::DeleteValue for $R9 returned $0"
  ${EndIf}
  DeleteRegValue SHCTX "${UNINSTKEY}" "CaptoCliPath"
!macroend
