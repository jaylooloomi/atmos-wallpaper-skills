; Close only this installed Atmos executable through its private native event.
; Never kill by generic Node, Python, WebView2 or Lively process name.
!macro NSIS_HOOK_PREINSTALL
  ${If} ${FileExists} "$INSTDIR\atmos-wallpaper.exe"
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --quit'
    Pop $0
    ${If} $0 != 0
      SetErrorLevel 2
      Abort "Atmos could not close safely. Exit Atmos and run the installer again."
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} ${FileExists} "$INSTDIR\atmos-wallpaper.exe"
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --quit'
    Pop $0
    ${If} $0 != 0
      SetErrorLevel 2
      Abort "Atmos could not close safely. Exit Atmos and uninstall again."
    ${EndIf}
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --autostart remove'
    Pop $0
    ${If} $0 != 0
      SetErrorLevel 2
      Abort "Atmos startup registration could not be removed."
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --autostart initialize'
  Pop $0
  ${If} $0 != 0
    SetErrorLevel 2
    Abort "Atmos startup registration could not be configured."
  ${EndIf}
!macroend
