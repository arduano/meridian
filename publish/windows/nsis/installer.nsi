!include "MUI2.nsh"

Name "Meridian"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\Meridian"
RequestExecutionLevel user

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "${STAGE_DIR}\*.*"
  CreateDirectory "$SMPROGRAMS\Meridian"
  CreateShortcut "$SMPROGRAMS\Meridian\Meridian CLI.lnk" "$INSTDIR\meridian.exe"
  IfFileExists "$INSTDIR\meridian-ui.exe" 0 +2
  CreateShortcut "$SMPROGRAMS\Meridian\Meridian UI.lnk" "$INSTDIR\meridian-ui.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\Meridian\Meridian CLI.lnk"
  Delete "$SMPROGRAMS\Meridian\Meridian UI.lnk"
  RMDir "$SMPROGRAMS\Meridian"
  Delete "$INSTDIR\meridian.exe"
  Delete "$INSTDIR\meridian-ui.exe"
  Delete "$INSTDIR\meridian.ico"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\KNOWN_ISSUES.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
