; Ganchos del instalador NSIS de rFirma (ADR-0035).

!define RFIRMA_SCHEME_KEY "Software\Classes\afirma"
!define RFIRMA_COMMAND "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""

; La carpeta de instalación en el PATH del usuario, para que `rfirma` resuelva a rfirma.com en una
; consola nueva (ADR-0041). El PATH no pasa nunca por una variable de NSIS, cuyas cadenas se
; truncan a 1024 caracteres: lo lee y lo escribe PowerShell en HKCU\Environment, sin expandir y
; con su tipo de valor, y la carpeta le llega por el entorno, no citada en la orden. Una entrada
; cuenta como la de rFirma si, expandida y sin la barra final, es la carpeta de instalación, sin
; distinguir mayúsculas: así no se duplica ni se quita otra.
!define RFIRMA_HWND_BROADCAST 0xFFFF
!define RFIRMA_WM_SETTINGCHANGE 0x001A
!define RFIRMA_POWERSHELL `"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command`
!define RFIRMA_PATH_READ `$$f={[Environment]::ExpandEnvironmentVariables($$args[0]).TrimEnd('\')};$$d=&$$f $$env:RFIRMA_INSTDIR;$$k=[Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment');$$p=[string]$$k.GetValue('Path','','DoNotExpandEnvironmentNames');$$t=if($$k.GetValueNames() -contains 'Path'){$$k.GetValueKind('Path')}else{'ExpandString'};$$e=$$p.Split(';');`
!define RFIRMA_PATH_ADD `if(-not($$e|?{(&$$f $$_) -eq $$d})){$$s=if($$p -eq '' -or $$p.EndsWith(';')){''}else{';'};$$k.SetValue('Path',$$p+$$s+$$env:RFIRMA_INSTDIR,$$t)}`
!define RFIRMA_PATH_REMOVE `$$r=@($$e|?{(&$$f $$_) -ne $$d});if($$r.Count -lt $$e.Count){if($$r -join ''){$$k.SetValue('Path',($$r -join ';'),$$t)}else{$$k.DeleteValue('Path')}}`

!macro RFIRMA_USER_PATH CHANGE
  System::Call 'Kernel32::SetEnvironmentVariable(t "RFIRMA_INSTDIR", t "$INSTDIR") i'
  nsExec::ExecToLog `${RFIRMA_POWERSHELL} "${RFIRMA_PATH_READ}${CHANGE}"`
  Pop $0
  ; Las consolas que se abran a partir de ahora heredan el PATH nuevo del Explorador.
  SendMessage ${RFIRMA_HWND_BROADCAST} ${RFIRMA_WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

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
  ; También al actualizar: si alguien quitó la carpeta del PATH, vuelve; si sigue, no se repite.
  !insertmacro RFIRMA_USER_PATH "${RFIRMA_PATH_ADD}"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    ReadRegStr $0 HKCU "${RFIRMA_SCHEME_KEY}\shell\open\command" ""
    ${If} $0 == "${RFIRMA_COMMAND}"
      DeleteRegKey HKCU "${RFIRMA_SCHEME_KEY}"
    ${EndIf}
    nsExec::ExecToLog '"$SYSDIR\certutil.exe" -user -delstore Root "rFirma CA local"'
    Pop $0
    ; Solo en una desinstalación de verdad: la actualización desinstala la versión anterior con
    ; $UpdateMode = 1, y la carpeta tiene que seguir en el PATH.
    !insertmacro RFIRMA_USER_PATH "${RFIRMA_PATH_REMOVE}"
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
