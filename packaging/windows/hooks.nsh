; Ganchos del instalador NSIS de rFirma (ADR-0035).

!define RFIRMA_SCHEME_KEY "Software\Classes\afirma"
!define RFIRMA_COMMAND "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "${RFIRMA_SCHEME_KEY}\shell\open\command" ""
  ReadRegStr $1 HKLM "${RFIRMA_SCHEME_KEY}\shell\open\command" ""
  ${If} $0 == ""
  ${AndIf} $1 == ""
    WriteRegStr HKCU "${RFIRMA_SCHEME_KEY}" "" "URL:afirma"
    WriteRegStr HKCU "${RFIRMA_SCHEME_KEY}" "URL Protocol" ""
    WriteRegStr HKCU "${RFIRMA_SCHEME_KEY}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
    WriteRegStr HKCU "${RFIRMA_SCHEME_KEY}\shell\open\command" "" "${RFIRMA_COMMAND}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    ReadRegStr $0 HKCU "${RFIRMA_SCHEME_KEY}\shell\open\command" ""
    ${If} $0 == "${RFIRMA_COMMAND}"
      DeleteRegKey HKCU "${RFIRMA_SCHEME_KEY}"
    ${EndIf}
    nsExec::ExecToLog '"$SYSDIR\certutil.exe" -user -delstore Root "rFirma CA local"'
    Pop $0
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegKey HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"
  ${EndIf}
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RMDir /r "$APPDATA\rfirma"
    RMDir /r "$LOCALAPPDATA\rfirma"
  ${EndIf}
!macroend
