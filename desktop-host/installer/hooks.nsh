; Only close this installed executable and retire its legacy screensaver selection.
!macro NSIS_HOOK_PREINSTALL
  ${If} ${FileExists} "$INSTDIR\atmos-wallpaper.exe"
    ${If} ${FileExists} "$INSTDIR\Atmos.scr"
      nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --screensaver-stop'
      Pop $0
      ${If} $0 != 0
        Abort "Legacy Atmos screensaver could not close safely."
      ${EndIf}
      nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --screensaver-unregister'
      Pop $0
      ${If} $0 != 0
        Abort "Legacy Atmos screensaver selection could not be restored safely."
      ${EndIf}
    ${EndIf}
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --quit'
    Pop $0
    ${If} $0 != 0
      Abort "Atmos could not close safely. Exit Atmos and run the installer again."
    ${EndIf}
  ${EndIf}
!macroend
!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --retire-node-payload'
  Pop $0
  ${If} $0 != 0
    Abort "Legacy Node payload could not be retired safely."
  ${EndIf}
  nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --retire-legacy-screensaver'
  Pop $0
  ${If} $0 != 0
    Abort "Legacy Atmos screensaver could not be retired safely."
  ${EndIf}
  nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --autostart initialize'
  Pop $0
  ${If} $0 != 0
    Abort "Atmos startup registration could not be configured."
  ${EndIf}
!macroend
!macro NSIS_HOOK_PREUNINSTALL
  ${If} ${FileExists} "$INSTDIR\atmos-wallpaper.exe"
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --retire-legacy-screensaver'
    Pop $0
    ${If} $0 != 0
      Abort "Legacy Atmos screensaver could not be retired safely."
    ${EndIf}
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --quit'
    Pop $0
    ${If} $0 != 0
      Abort "Atmos could not close safely. Exit Atmos and uninstall again."
    ${EndIf}
    nsExec::ExecToLog '"$INSTDIR\atmos-wallpaper.exe" --autostart remove'
    Pop $0
    ${If} $0 != 0
      Abort "Atmos startup registration could not be removed."
    ${EndIf}
  ${EndIf}
!macroend
!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\Atmos.scr"
!macroend
