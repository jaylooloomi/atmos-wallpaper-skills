; Close only this installed Atmos executable through its private native event.
; Never kill by generic Node, Python, WebView2 or Lively process name.
!macro NSIS_HOOK_PREINSTALL
  ${If} ${FileExists} "$INSTDIR\atmos-wallpaper.exe"
    ${If} ${FileExists} "$INSTDIR\Atmos.scr"
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --screensaver-stop'
    Pop $0
    ${If} $0 != 0
      SetErrorLevel 2
      Abort "Atmos screensaver could not close safely."
    ${EndIf}
    ${EndIf}
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --quit'
    Pop $0
    ${If} $0 != 0
      SetErrorLevel 2
      Abort "Atmos could not close safely. Exit Atmos and run the installer again."
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} ${FileExists} "$INSTDIR\Atmos.scr"
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --screensaver-unregister'
    Pop $0
    ${If} $0 != 0
      SetErrorLevel 2
      Abort "Atmos screensaver selection could not be restored safely."
    ${EndIf}
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\atmos-wallpaper.exe"
    ${If} ${FileExists} "$INSTDIR\Atmos.scr"
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --screensaver-stop'
    Pop $0
    ${If} $0 != 0
      SetErrorLevel 2
      Abort "Atmos screensaver could not close safely."
    ${EndIf}
    ${EndIf}
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

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\Atmos.scr"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ClearErrors
  CopyFiles /SILENT "$INSTDIR\atmos-wallpaper.exe" "$INSTDIR\Atmos.scr"
  ${If} ${Errors}
    Abort "Atmos screensaver could not be installed."
  ${EndIf}
  nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --autostart initialize'
  Pop $0
  ${If} $0 != 0
    SetErrorLevel 2
    Abort "Atmos startup registration could not be configured."
  ${EndIf}
!macroend
