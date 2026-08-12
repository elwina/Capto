; Capto NSIS installer hooks — put CLI on PATH so `capto` works in new terminals.
; Only `$INSTDIR\cli` is added (never `$INSTDIR`), so Capto.exe cannot shadow `capto`
; on case-insensitive Windows.
;
; This file is !include'd after StrFunc.nsh + ${StrLoc} in installer.nsi, but before
; PRODUCTNAME / UNINSTKEY / INSTALLMODE. Anything needing those defines must live in
; NSIS_HOOK_* macros (expanded later).
;
; Wired from tauri.conf.json → bundle.windows.nsis.installerHooks

!include "LogicLib.nsh"
!include "WinMessages.nsh"

; Uninstall section cannot Call StrLoc — need the un. variant.
${UnStrLoc}

Var CaptoPathIsMachine

Function CaptoReadPathVar
  ; in: CaptoPathIsMachine ; out: $0
  ${If} $CaptoPathIsMachine == "1"
    ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
  ${Else}
    ReadRegStr $0 HKCU "Environment" "Path"
  ${EndIf}
FunctionEnd

Function CaptoWritePathVar
  ; in: CaptoPathIsMachine, $0 = new Path
  ${If} $CaptoPathIsMachine == "1"
    WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" $0
  ${Else}
    WriteRegExpandStr HKCU "Environment" "Path" $0
  ${EndIf}
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
FunctionEnd

; Adds $INSTDIR\cli to PATH (idempotent).
Function CaptoAppendCliPath
  Push $0
  Push $1
  Push $2
  Push $R9

  StrCpy $R9 "$INSTDIR\cli"
  ${IfNot} ${FileExists} "$R9\capto.exe"
    DetailPrint "Capto CLI missing at $R9\capto.exe — PATH not updated"
    Goto CaptoAppendCliPath_done
  ${EndIf}

  Call CaptoReadPathVar
  ${IfErrors}
    DetailPrint "Failed to read PATH — skipping PATH update"
    Goto CaptoAppendCliPath_done
  ${EndIf}

  StrCpy $1 ";$0;"
  ${StrLoc} $2 $1 ";$R9;" ">"
  ${If} $2 != ""
    DetailPrint "Capto CLI already on PATH: $R9"
    Goto CaptoAppendCliPath_done
  ${EndIf}

  ${If} $0 == ""
    DetailPrint "PATH is empty — skipping PATH update (refusing to overwrite)"
    Goto CaptoAppendCliPath_done
  ${Else}
    StrCpy $2 $0 1 -1
    ${If} $2 == ";"
      StrCpy $0 $0 -1
    ${EndIf}
    StrCpy $0 "$0;$R9"
  ${EndIf}

  Call CaptoWritePathVar
  DetailPrint "Added Capto CLI to PATH: $R9"

CaptoAppendCliPath_done:
  Pop $R9
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function un.CaptoReadPathVar
  ${If} $CaptoPathIsMachine == "1"
    ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
  ${Else}
    ReadRegStr $0 HKCU "Environment" "Path"
  ${EndIf}
FunctionEnd

Function un.CaptoWritePathVar
  ${If} $CaptoPathIsMachine == "1"
    WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" $0
  ${Else}
    WriteRegExpandStr HKCU "Environment" "Path" $0
  ${EndIf}
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
FunctionEnd

; Removes $R9 from PATH (caller sets $R9 to the cli directory).
Function un.CaptoRemoveCliPath
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5

  ${If} $R9 == ""
    StrCpy $R9 "$INSTDIR\cli"
  ${EndIf}

  Call un.CaptoReadPathVar
  ${IfErrors}
    DetailPrint "Failed to read PATH — skipping PATH cleanup"
    Goto CaptoRemoveCliPath_done
  ${EndIf}

  ${If} $0 == ""
    Goto CaptoRemoveCliPath_done
  ${EndIf}

  StrCpy $1 ";$0;"
CaptoRemoveCliPath_loop:
  ${UnStrLoc} $3 $1 ";$R9;" ">"
  ${If} $3 == ""
    Goto CaptoRemoveCliPath_finish
  ${EndIf}
  StrCpy $4 $1 $3
  StrLen $5 ";$R9;"
  IntOp $3 $3 + $5
  StrCpy $1 $1 "" $3
  StrCpy $1 "$4;$1"
  Goto CaptoRemoveCliPath_loop

CaptoRemoveCliPath_finish:
  StrCpy $0 $1
  StrCpy $2 $0 1
  ${If} $2 == ";"
    StrCpy $0 $0 "" 1
  ${EndIf}
  StrCpy $2 $0 1 -1
  ${If} $2 == ";"
    StrCpy $0 $0 -1
  ${EndIf}

  ${If} $0 == ""
    DetailPrint "PATH is empty after cleanup — skipping write"
    Goto CaptoRemoveCliPath_done
  ${EndIf}

  Call un.CaptoWritePathVar
  DetailPrint "Removed Capto CLI from PATH: $R9"

CaptoRemoveCliPath_done:
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

!macro CaptoSetPathHive
  StrCpy $CaptoPathIsMachine "0"
  !if "${INSTALLMODE}" == "perMachine"
    StrCpy $CaptoPathIsMachine "1"
  !else if "${INSTALLMODE}" == "both"
    ${If} $MultiUser.InstallMode == "AllUsers"
      StrCpy $CaptoPathIsMachine "1"
    ${EndIf}
  !endif
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro CaptoSetPathHive
  Call CaptoAppendCliPath
  WriteRegStr SHCTX "${UNINSTKEY}" "CaptoCliPath" "$INSTDIR\cli"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CaptoSetPathHive
  ReadRegStr $R9 SHCTX "${UNINSTKEY}" "CaptoCliPath"
  ${If} $R9 == ""
    StrCpy $R9 "$INSTDIR\cli"
  ${EndIf}
  Call un.CaptoRemoveCliPath
  DeleteRegValue SHCTX "${UNINSTKEY}" "CaptoCliPath"
!macroend
