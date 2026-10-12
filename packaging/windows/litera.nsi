; Литера installer for Windows 10/11 (NSIS 3, Unicode). Built by package-nsis.sh:
;   makensis -DVERSION=0.0.2 -DBINDIR=<dir with litera.exe> -DICON=<.ico> -DOUTFILE=<setup.exe> litera.nsi
Unicode true
ManifestDPIAware true
RequestExecutionLevel admin
SetCompressor /SOLID lzma
!include "MUI2.nsh"
!include "x64.nsh"

!define APPNAME "Литера"
!define REGKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Litera"
!define CAPS "Software\Litera\Capabilities"

Name "${APPNAME} ${VERSION}"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\Litera"
InstallDirRegKey HKLM "Software\Litera" "InstallDir"
BrandingText "${APPNAME} ${VERSION} — nookbat.ru"
VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=0 "ProductName" "${APPNAME}"
VIAddVersionKey /LANG=0 "FileDescription" "${APPNAME} — установка"
VIAddVersionKey /LANG=0 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "CompanyName" "Горшков Сергей Владимирович"
VIAddVersionKey /LANG=0 "LegalCopyright" "MIT OR Apache-2.0"

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\litera.exe"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "Литере нужна 64-разрядная Windows 10 или 11."
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

!macro Assoc EXT PROGID DESC
  WriteRegStr HKLM "Software\Classes\${PROGID}" "" "${DESC}"
  WriteRegStr HKLM "Software\Classes\${PROGID}\DefaultIcon" "" '"$INSTDIR\litera.exe",0'
  WriteRegStr HKLM "Software\Classes\${PROGID}\shell\open\command" "" '"$INSTDIR\litera.exe" "%1"'
  WriteRegStr HKLM "Software\Classes\${EXT}\OpenWithProgids" "${PROGID}" ""
  WriteRegStr HKLM "Software\Classes\Applications\litera.exe\SupportedTypes" "${EXT}" ""
  WriteRegStr HKLM "${CAPS}\FileAssociations" "${EXT}" "${PROGID}"
!macroend
!macro UnAssoc EXT PROGID
  DeleteRegKey HKLM "Software\Classes\${PROGID}"
  DeleteRegValue HKLM "Software\Classes\${EXT}\OpenWithProgids" "${PROGID}"
!macroend

Section "Литера" Main
  SectionIn RO
  SetShellVarContext all
  SetOutPath "$INSTDIR"
  File "${BINDIR}\litera.exe"
  File "${BINDIR}\litera-cli.exe"
  File /nonfatal "${BINDIR}\LICENSE-MIT"
  File /nonfatal "${BINDIR}\LICENSE-APACHE"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\Литера.lnk" "$INSTDIR\litera.exe"
  CreateShortcut "$DESKTOP\Литера.lnk" "$INSTDIR\litera.exe"

  WriteRegStr HKLM "Software\Litera" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\litera.exe" "" "$INSTDIR\litera.exe"
  WriteRegStr HKLM "Software\Classes\Applications\litera.exe" "FriendlyAppName" "${APPNAME}"
  WriteRegStr HKLM "Software\Classes\Applications\litera.exe\shell\open\command" "" '"$INSTDIR\litera.exe" "%1"'
  WriteRegStr HKLM "Software\RegisteredApplications" "Litera" "${CAPS}"
  WriteRegStr HKLM "${CAPS}" "ApplicationName" "${APPNAME}"
  WriteRegStr HKLM "${CAPS}" "ApplicationDescription" "Бесплатная программа для создания и редактирования документов."
  WriteRegStr HKLM "${CAPS}" "ApplicationIcon" '"$INSTDIR\litera.exe",0'
  !insertmacro Assoc ".docx" "Litera.Document" "Документ Word"
  !insertmacro Assoc ".docm" "Litera.DocumentMacroEnabled" "Документ Word с макросами"
  !insertmacro Assoc ".dotx" "Litera.Template" "Шаблон Word"
  !insertmacro Assoc ".dotm" "Litera.TemplateMacroEnabled" "Шаблон Word с макросами"
  !insertmacro Assoc ".doc" "Litera.LegacyDocument" "Документ Word 97–2003"
  !insertmacro Assoc ".dot" "Litera.LegacyTemplate" "Шаблон Word 97–2003"
  !insertmacro Assoc ".rtf" "Litera.RichText" "Документ RTF"
  !insertmacro Assoc ".odt" "Litera.OpenDocumentText" "Текст OpenDocument"
  !insertmacro Assoc ".md" "Litera.Markdown" "Документ Markdown"
  !insertmacro Assoc ".txt" "Litera.PlainText" "Текстовый документ"

  WriteRegStr HKLM "${REGKEY}" "DisplayName" "${APPNAME}"
  WriteRegStr HKLM "${REGKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${REGKEY}" "DisplayIcon" '"$INSTDIR\litera.exe",0'
  WriteRegStr HKLM "${REGKEY}" "Publisher" "Горшков Сергей Владимирович"
  WriteRegStr HKLM "${REGKEY}" "URLInfoAbout" "https://nookbat.ru"
  WriteRegStr HKLM "${REGKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${REGKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${REGKEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${REGKEY}" "NoModify" 1
  WriteRegDWORD HKLM "${REGKEY}" "NoRepair" 1
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

Section "Uninstall"
  SetShellVarContext all
  Delete "$INSTDIR\litera.exe"
  Delete "$INSTDIR\litera-cli.exe"
  Delete "$INSTDIR\LICENSE-MIT"
  Delete "$INSTDIR\LICENSE-APACHE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Литера.lnk"
  Delete "$DESKTOP\Литера.lnk"
  !insertmacro UnAssoc ".docx" "Litera.Document"
  !insertmacro UnAssoc ".docm" "Litera.DocumentMacroEnabled"
  !insertmacro UnAssoc ".dotx" "Litera.Template"
  !insertmacro UnAssoc ".dotm" "Litera.TemplateMacroEnabled"
  !insertmacro UnAssoc ".doc" "Litera.LegacyDocument"
  !insertmacro UnAssoc ".dot" "Litera.LegacyTemplate"
  !insertmacro UnAssoc ".rtf" "Litera.RichText"
  !insertmacro UnAssoc ".odt" "Litera.OpenDocumentText"
  !insertmacro UnAssoc ".md" "Litera.Markdown"
  !insertmacro UnAssoc ".txt" "Litera.PlainText"
  DeleteRegKey HKLM "Software\Classes\Applications\litera.exe"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\litera.exe"
  DeleteRegValue HKLM "Software\RegisteredApplications" "Litera"
  DeleteRegKey HKLM "Software\Litera"
  DeleteRegKey HKLM "${REGKEY}"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
