; NSIS hooks for the Windows installer: the bundled masterhubd service runs per user, starts
; at login (HKCU Run key) and is stopped before upgrades and on uninstall. The game modes
; (masterhub-games) are opt-in: on the first install the user is asked game by game.

!define OMH_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define OMH_RUN_VALUE "OpenMasterHUB service"
!define OMH_GAMES_VALUE "OpenMasterHUB game modes"
!define OMH_GAMES_CONFIG "$APPDATA\openmasterhub\games.toml"

; Ask whether to switch on one game's mode; `setup` finds it through Steam, else the user
; points at its folder. Nothing from the game ships with us: setup reads the look from the
; user's own install.
!macro OMH_GAME NAME TITLE
  MessageBox MB_YESNO|MB_ICONQUESTION "Turn on the ${TITLE} game mode?$\r$\n$\r$\nWhile ${TITLE} has focus, the hub shows keys for the game. Its icons are read from your own copy of the game; you can change this later with masterhub-games." /SD IDNO IDNO omh_skip_${NAME}
  nsExec::ExecToLog '"$INSTDIR\masterhub-games.exe" setup ${NAME}'
  Pop $0
  StrCmp $0 "0" omh_done_${NAME}
  nsDialogs::SelectFolderDialog "Where is ${TITLE} installed?" ""
  Pop $1
  StrCmp $1 "error" omh_skip_${NAME}
  nsExec::ExecToLog '"$INSTDIR\masterhub-games.exe" setup ${NAME} --path "$1"'
  Pop $0
  StrCmp $0 "0" omh_done_${NAME}
  MessageBox MB_OK|MB_ICONEXCLAMATION "${TITLE} could not be set up from $1.$\r$\nTry again later: masterhub-games setup ${NAME} --path <folder>" /SD IDOK
  Goto omh_skip_${NAME}
  omh_done_${NAME}:
  omh_skip_${NAME}:
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; Stop running copies so their executables can be replaced.
  nsExec::Exec 'taskkill /F /IM masterhubd.exe'
  nsExec::Exec 'taskkill /F /IM masterhub-games.exe'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "${OMH_RUN_KEY}" "${OMH_RUN_VALUE}" '"$INSTDIR\masterhubd.exe"'
  Exec '"$INSTDIR\masterhubd.exe"'
  ; Only the first install asks; later the choices live in games.toml.
  IfFileExists "${OMH_GAMES_CONFIG}" omh_games_asked
  !insertmacro OMH_GAME "cyberpunk" "Cyberpunk 2077"
  !insertmacro OMH_GAME "deadlock" "Deadlock"
  !insertmacro OMH_GAME "wow_forever" "World of Warcraft (WoW Forever)"
  !insertmacro OMH_GAME "crimson_desert" "Crimson Desert"
  IfFileExists "${OMH_GAMES_CONFIG}" omh_games_asked
  ; Every game declined: an empty config remembers the answer.
  CreateDirectory "$APPDATA\openmasterhub"
  FileOpen $2 "${OMH_GAMES_CONFIG}" w
  FileClose $2
  omh_games_asked:
  ; Exits at once while no game is switched on.
  WriteRegStr HKCU "${OMH_RUN_KEY}" "${OMH_GAMES_VALUE}" '"$INSTDIR\masterhub-games.exe"'
  Exec '"$INSTDIR\masterhub-games.exe"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /IM masterhubd.exe'
  nsExec::Exec 'taskkill /F /IM masterhub-games.exe'
  DeleteRegValue HKCU "${OMH_RUN_KEY}" "${OMH_RUN_VALUE}"
  DeleteRegValue HKCU "${OMH_RUN_KEY}" "${OMH_GAMES_VALUE}"
!macroend
