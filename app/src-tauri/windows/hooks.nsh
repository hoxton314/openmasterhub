; NSIS hooks for the Windows installer: the bundled masterhubd service runs per user, starts
; at login (HKCU Run key) and is stopped before upgrades and on uninstall.

!define OMH_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define OMH_RUN_VALUE "OpenMasterHUB service"

!macro NSIS_HOOK_PREINSTALL
  ; Stop a running service so its executable can be replaced.
  nsExec::Exec 'taskkill /F /IM masterhubd.exe'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "${OMH_RUN_KEY}" "${OMH_RUN_VALUE}" '"$INSTDIR\masterhubd.exe"'
  Exec '"$INSTDIR\masterhubd.exe"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /IM masterhubd.exe'
  DeleteRegValue HKCU "${OMH_RUN_KEY}" "${OMH_RUN_VALUE}"
!macroend
