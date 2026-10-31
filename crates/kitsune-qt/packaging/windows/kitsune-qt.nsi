; NSIS installer for the Qt build of Kitsune.
;
; Built by .github/workflows/qt-release.yml, which passes every path in on the
; command line with /D so nothing here is specific to one machine or version:
;
;   makensis /DVERSION=0.2.0 /DSRCDIR=<windeployqt output> \
;            /DOUTFILE=Kitsune-Qt_0.2.0_x64-setup.exe kitsune-qt.nsi
;
; The payload is the whole windeployqt tree: kitsune-qt.exe, the Qt DLLs, the
; platform/imageformat/QML plugin directories, and libmpv-2.dll. Getting that
; tree right is windeployqt's job, not this script's -- this script only puts
; it on disk, registers the deep-link scheme, and writes the uninstaller.

Unicode true
ManifestDPIAware true

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SRCDIR
  !error "SRCDIR must be passed with /DSRCDIR=<windeployqt output directory>"
!endif
!ifndef OUTFILE
  !define OUTFILE "Kitsune-Qt_x64-setup.exe"
!endif

!define APP_NAME "Kitsune"
!define APP_ID "com.kitsune.app"
!define APP_EXE "kitsune-qt.exe"
!define PUBLISHER "Kitsune"
; Per-user install by default: no UAC prompt, and HKCU is where the deep-link
; registration goes anyway. Selecting "all users" is left out deliberately --
; it would need HKLM plus elevation, and the app has no machine-wide state.
!define INSTALL_DIR "$LOCALAPPDATA\Programs\Kitsune-Qt"

Name "${APP_NAME} ${VERSION}"
OutFile "${OUTFILE}"
InstallDir "${INSTALL_DIR}"
InstallDirRegKey HKCU "Software\Kitsune\Qt" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "MIT"

!include "MUI2.nsh"
!define MUI_ICON "${SRCDIR}\kitsune-qt.exe"
!define MUI_UNICON "${SRCDIR}\kitsune-qt.exe"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Kitsune" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"

  ; The full windeployqt tree, recursively. `File /r` preserves the plugin
  ; subdirectories (platforms\, imageformats\, qml\, ...) that Qt resolves at
  ; runtime relative to the executable.
  File /r "${SRCDIR}\*.*"

  ; Record where we installed, so an upgrade reuses the same directory.
  WriteRegStr HKCU "Software\Kitsune\Qt" "InstallDir" "$INSTDIR"

  ; ---- Uninstall entry (Add/Remove Programs) -------------------------------
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt" \
    "DisplayName" "${APP_NAME} (Qt)"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt" \
    "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt" \
    "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt" \
    "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt" \
    "NoRepair" 1

  ; ---- Deep-link scheme ----------------------------------------------------
  ; AniList redirects to kitsune://auth#access_token=... . Windows launches the
  ; registered command with the URL as %1, which is exactly what
  ; single_instance::auth_url_from_args looks for.
  ;
  ; HKCU (not HKLM) so this works without elevation, and because the Tauri
  ; installer -- if it is installed too -- owns the HKLM/classes entry.
  WriteRegStr HKCU "Software\Classes\kitsune" "" "URL:Kitsune"
  WriteRegStr HKCU "Software\Classes\kitsune" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\kitsune\DefaultIcon" "" "$INSTDIR\${APP_EXE},0"
  WriteRegStr HKCU "Software\Classes\kitsune\shell\open\command" "" '"$INSTDIR\${APP_EXE}" "%1"'

  ; Tell Explorer the association changed, or the first click on a kitsune://
  ; link may still go to whatever was registered before.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'

  ; ---- Start menu shortcut -------------------------------------------------
  CreateDirectory "$SMPROGRAMS\Kitsune"
  CreateShortcut "$SMPROGRAMS\Kitsune\Kitsune.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$SMPROGRAMS\Kitsune\Uninstall.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  ; Remove the scheme first: leaving a handler pointing at a deleted exe makes
  ; every kitsune:// click pop an error dialog.
  DeleteRegKey HKCU "Software\Classes\kitsune"
  DeleteRegKey HKCU "Software\Kitsune\Qt"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KitsuneQt"

  Delete "$SMPROGRAMS\Kitsune\Kitsune.lnk"
  Delete "$SMPROGRAMS\Kitsune\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Kitsune"

  ; windeployqt's tree, including its subdirectories. `RMDir /r` would also
  ; take user-created files with it, so remove the known subtrees then the dir.
  RMDir /r "$INSTDIR\platforms"
  RMDir /r "$INSTDIR\imageformats"
  RMDir /r "$INSTDIR\iconengines"
  RMDir /r "$INSTDIR\styles"
  RMDir /r "$INSTDIR\tls"
  RMDir /r "$INSTDIR\networkinformation"
  RMDir /r "$INSTDIR\qml"
  RMDir /r "$INSTDIR\translations"
  Delete "$INSTDIR\*.dll"
  Delete "$INSTDIR\*.exe"
  Delete "$INSTDIR\*.txt"
  RMDir "$INSTDIR"
SectionEnd