; Removes GiENEMY's sign-in task (and stops the running copy) before the
; files are deleted. The exe asks for administrator approval itself.
!macro NSIS_HOOK_PREUNINSTALL
  ExecWait '"$INSTDIR\GiENEMY.exe" --uninstall'
!macroend
