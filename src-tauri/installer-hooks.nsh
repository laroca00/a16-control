; The installer already runs elevated (per-machine install into Program
; Files), so it registers the WMI classes and the sign-in task here and the
; app never has to ask for administrator approval itself.
!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"$INSTDIR\GiENEMY.exe" --install'
!macroend

; Hands the fans back to the firmware, stops the running copy and removes
; the sign-in task before the files are deleted.
!macro NSIS_HOOK_PREUNINSTALL
  ExecWait '"$INSTDIR\GiENEMY.exe" --uninstall'
!macroend
