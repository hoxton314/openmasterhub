; NSIS hooks for the Windows installer: the bundled masterhubd service and the game-mode
; service (masterhub-games) run per user, start at login (HKCU Run key) and are stopped before
; upgrades and on uninstall. Game modes are opt-in in the app (sidebar → Game modes); while
; none is on, masterhub-games just waits for its config to change.

!define OMH_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define OMH_RUN_VALUE "OpenMasterHUB service"
!define OMH_GAMES_VALUE "OpenMasterHUB game modes"

!macro NSIS_HOOK_PREINSTALL
  ; Stop running copies so their executables can be replaced.
  nsExec::Exec 'taskkill /F /IM masterhubd.exe'
  nsExec::Exec 'taskkill /F /IM masterhub-games.exe'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "${OMH_RUN_KEY}" "${OMH_RUN_VALUE}" '"$INSTDIR\masterhubd.exe"'
  Exec '"$INSTDIR\masterhubd.exe"'
  WriteRegStr HKCU "${OMH_RUN_KEY}" "${OMH_GAMES_VALUE}" '"$INSTDIR\masterhub-games.exe"'
  Exec '"$INSTDIR\masterhub-games.exe"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /IM masterhubd.exe'
  nsExec::Exec 'taskkill /F /IM masterhub-games.exe'
  DeleteRegValue HKCU "${OMH_RUN_KEY}" "${OMH_RUN_VALUE}"
  DeleteRegValue HKCU "${OMH_RUN_KEY}" "${OMH_GAMES_VALUE}"
!macroend
