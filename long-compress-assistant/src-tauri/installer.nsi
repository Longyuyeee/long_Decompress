; Based on the official Tauri CLI 1.6.3 NSIS template.
; Source: https://github.com/tauri-apps/tauri/blob/tauri-cli-v1.6.3/tooling/bundler/src/bundle/windows/templates/installer.nsi
; Upstream SHA-256: cb1f407c21ec29ea83ddb4de405c6b8c3a641f929ac842664be10745092587db
; Local change: remove Explorer context-menu registrations during uninstall.
Unicode true
; Set the compression algorithm. Default is LZMA.
!if "{{compression}}" == ""
  SetCompressor /SOLID lzma
!else
  SetCompressor /SOLID "{{compression}}"
!endif

!include MUI2.nsh
!include FileFunc.nsh
!include x64.nsh
!include WordFunc.nsh
!include "StrFunc.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
${StrCase}
${StrLoc}

!define MANUFACTURER "{{manufacturer}}"
!define PRODUCTNAME "{{product_name}}"
!define LEGACY_PRODUCTNAME "胧解压·方便助手"
!define VERSION "{{version}}"
!define VERSIONWITHBUILD "{{version_with_build}}"
!define INSTALLMODE "{{install_mode}}"
!define LICENSE "{{license}}"
!define INSTALLERICON "{{installer_icon}}"
!define SIDEBARIMAGE "{{sidebar_image}}"
!define HEADERIMAGE "{{header_image}}"
!define MAINBINARYNAME "{{main_binary_name}}"
!define MAINBINARYSRCPATH "{{main_binary_path}}"
!define BUNDLEID "{{bundle_id}}"
!define COPYRIGHT "{{copyright}}"
!define OUTFILE "{{out_file}}"
!define ARCH "{{arch}}"
!define PLUGINSPATH "{{additional_plugins_path}}"
!define ALLOWDOWNGRADES "{{allow_downgrades}}"
!define DISPLAYLANGUAGESELECTOR "{{display_language_selector}}"
!define INSTALLWEBVIEW2MODE "{{install_webview2_mode}}"
!define WEBVIEW2INSTALLERARGS "{{webview2_installer_args}}"
!define WEBVIEW2BOOTSTRAPPERPATH "{{webview2_bootstrapper_path}}"
!define WEBVIEW2INSTALLERPATH "{{webview2_installer_path}}"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUPRODUCTKEY "Software\${MANUFACTURER}\${PRODUCTNAME}"
!define LEGACY_UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${LEGACY_PRODUCTNAME}"
!define LEGACY_MANUPRODUCTKEY "Software\${MANUFACTURER}\${LEGACY_PRODUCTNAME}"
!define UNINSTALLERSIGNCOMMAND "{{uninstaller_sign_cmd}}"
!define ESTIMATEDSIZE "{{estimated_size}}"

Name "${PRODUCTNAME}"
BrandingText "${COPYRIGHT}"
OutFile "${OUTFILE}"

VIProductVersion "${VERSIONWITHBUILD}"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"

; Plugins path, currently exists for linux only
!if "${PLUGINSPATH}" != ""
    !addplugindir "${PLUGINSPATH}"
!endif

!if "${UNINSTALLERSIGNCOMMAND}" != ""
  !uninstfinalize '${UNINSTALLERSIGNCOMMAND}'
!endif

; Handle install mode, `perUser`, `perMachine` or `both`
!if "${INSTALLMODE}" == "perMachine"
  RequestExecutionLevel highest
!endif

!if "${INSTALLMODE}" == "currentUser"
  RequestExecutionLevel user
!endif

!if "${INSTALLMODE}" == "both"
  !define MULTIUSER_MUI
  !define MULTIUSER_INSTALLMODE_INSTDIR "${PRODUCTNAME}"
  !define MULTIUSER_INSTALLMODE_COMMANDLINE
  !if "${ARCH}" == "x64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !else if "${ARCH}" == "arm64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !endif
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_KEY "${UNINSTKEY}"
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_VALUENAME "CurrentUser"
  !define MULTIUSER_INSTALLMODEPAGE_SHOWUSERNAME
  !define MULTIUSER_INSTALLMODE_FUNCTION RestorePreviousInstallLocation
  !define MULTIUSER_EXECUTIONLEVEL Highest
  !include MultiUser.nsh
!endif

; installer icon
!if "${INSTALLERICON}" != ""
  !define MUI_ICON "${INSTALLERICON}"
!endif

; installer sidebar image
!if "${SIDEBARIMAGE}" != ""
  !define MUI_WELCOMEFINISHPAGE_BITMAP "${SIDEBARIMAGE}"
!endif

; installer header image
!if "${HEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE
  !define MUI_HEADERIMAGE_BITMAP  "${HEADERIMAGE}"
!endif

; Define registry key to store installer language
!define MUI_LANGDLL_REGISTRY_ROOT "HKCU"
!define MUI_LANGDLL_REGISTRY_KEY "${MANUPRODUCTKEY}"
!define MUI_LANGDLL_REGISTRY_VALUENAME "Installer Language"

; Installer pages, must be ordered as they appear
; 1. Welcome Page
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_WELCOME

; 2. License Page (if defined)
!if "${LICENSE}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MUI_PAGE_LICENSE "${LICENSE}"
!endif

; 3. Install mode (if it is set to `both`)
!if "${INSTALLMODE}" == "both"
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MULTIUSER_PAGE_INSTALLMODE
!endif


; 4. Custom page to ask user if he wants to reinstall/uninstall
;    only if a previous installtion was detected
Var ReinstallPageCheck
Page custom PageReinstall PageLeaveReinstall
Function PageReinstall
  ; Uninstall previous WiX installation if exists.
  ;
  ; A WiX installer stores the isntallation info in registry
  ; using a UUID and so we have to loop through all keys under
  ; `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
  ; and check if `DisplayName` and `Publisher` keys match ${PRODUCTNAME} and ${MANUFACTURER}
  ;
  ; This has a potentional issue that there maybe another installation that matches
  ; our ${PRODUCTNAME} and ${MANUFACTURER} but wasn't installed by our WiX installer,
  ; however, this should be fine since the user will have to confirm the uninstallation
  ; and they can chose to abort it if doesn't make sense.
  StrCpy $0 0
  wix_loop:
    EnumRegKey $1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" $0
    StrCmp $1 "" wix_done ; Exit loop if there is no more keys to loop on
    IntOp $0 $0 + 1
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "DisplayName"
    ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "Publisher"
    StrCmp "$R0$R1" "${PRODUCTNAME}${MANUFACTURER}" 0 wix_loop
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "UninstallString"
    ${StrCase} $R1 $R0 "L"
    ${StrLoc} $R0 $R1 "msiexec" ">"
    StrCmp $R0 0 0 wix_done
    StrCpy $R7 "wix"
    StrCpy $R6 "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1"
    Goto compare_version
  wix_done:

  ; Check if there is an existing installation, if not, abort the reinstall page
  ReadRegStr $R0 SHCTX "${UNINSTKEY}" ""
  ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
  ${IfThen} "$R0$R1" == "" ${|} Abort ${|}

  ; Compare this installar version with the existing installation
  ; and modify the messages presented to the user accordingly
  compare_version:
  StrCpy $R4 "$(older)"
  ${If} $R7 == "wix"
    ReadRegStr $R0 HKLM "$R6" "DisplayVersion"
  ${Else}
    ReadRegStr $R0 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${EndIf}
  ${IfThen} $R0 == "" ${|} StrCpy $R4 "$(unknown)" ${|}

  nsis_tauri_utils::SemverCompare "${VERSION}" $R0
  Pop $R0
  ; Reinstalling the same version
  ${If} $R0 == 0
    StrCpy $R1 "$(alreadyInstalledLong)"
    StrCpy $R2 "$(addOrReinstall)"
    StrCpy $R3 "$(uninstallApp)"
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$(chooseMaintenanceOption)"
    StrCpy $R5 "2"
  ; Upgrading
  ${ElseIf} $R0 == 1
    StrCpy $R1 "$(olderOrUnknownVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    StrCpy $R3 "$(dontUninstall)"
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$(choowHowToInstall)"
    StrCpy $R5 "1"
  ; Downgrading
  ${ElseIf} $R0 == -1
    StrCpy $R1 "$(newerVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    !if "${ALLOWDOWNGRADES}" == "true"
      StrCpy $R3 "$(dontUninstall)"
    !else
      StrCpy $R3 "$(dontUninstallDowngrade)"
    !endif
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$(choowHowToInstall)"
    StrCpy $R5 "1"
  ${Else}
    Abort
  ${EndIf}

  Call SkipIfPassive

  nsDialogs::Create 1018
  Pop $R4
  ${IfThen} $(^RTL) == 1 ${|} nsDialogs::SetRTL $(^RTL) ${|}

  ${NSD_CreateLabel} 0 0 100% 24u $R1
  Pop $R1

  ${NSD_CreateRadioButton} 30u 50u -30u 8u $R2
  Pop $R2
  ${NSD_OnClick} $R2 PageReinstallUpdateSelection

  ${NSD_CreateRadioButton} 30u 70u -30u 8u $R3
  Pop $R3
  ; disable this radio button if downgrading and downgrades are disabled
  !if "${ALLOWDOWNGRADES}" == "false"
    ${IfThen} $R0 == -1 ${|} EnableWindow $R3 0 ${|}
  !endif
  ${NSD_OnClick} $R3 PageReinstallUpdateSelection

  ; Check the first radio button if this the first time
  ; we enter this page or if the second button wasn't
  ; selected the last time we were on this page
  ${If} $ReinstallPageCheck != 2
    SendMessage $R2 ${BM_SETCHECK} ${BST_CHECKED} 0
  ${Else}
    SendMessage $R3 ${BM_SETCHECK} ${BST_CHECKED} 0
  ${EndIf}

  ${NSD_SetFocus} $R2
  nsDialogs::Show
FunctionEnd
Function PageReinstallUpdateSelection
  ${NSD_GetState} $R2 $R1
  ${If} $R1 == ${BST_CHECKED}
    StrCpy $ReinstallPageCheck 1
  ${Else}
    StrCpy $ReinstallPageCheck 2
  ${EndIf}
FunctionEnd
Function PageLeaveReinstall
  ${NSD_GetState} $R2 $R1

  ; $R5 holds whether we are reinstalling the same version or not
  ; $R5 == "1" -> different versions
  ; $R5 == "2" -> same version
  ;
  ; $R1 holds the radio buttons state. its meaning is dependant on the context
  StrCmp $R5 "1" 0 +2 ; Existing install is not the same version?
    StrCmp $R1 "1" reinst_uninstall reinst_done ; $R1 == "1", then user chose to uninstall existing version, otherwise skip uninstalling
  StrCmp $R1 "1" reinst_done ; Same version? skip uninstalling

  reinst_uninstall:
    HideWindow
    ClearErrors

    ${If} $R7 == "wix"
      ReadRegStr $R1 HKLM "$R6" "UninstallString"
      ExecWait '$R1' $0
    ${Else}
      ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
      ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
      ExecWait '$R1 /P _?=$4' $0
    ${EndIf}

    BringToFront

    ${IfThen} ${Errors} ${|} StrCpy $0 2 ${|} ; ExecWait failed, set fake exit code

    ${If} $0 <> 0
    ${OrIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
      ${If} $0 = 1 ; User aborted uninstaller?
        StrCmp $R5 "2" 0 +2 ; Is the existing install the same version?
          Quit ; ...yes, already installed, we are done
        Abort
      ${EndIf}
      MessageBox MB_ICONEXCLAMATION "$(unableToUninstall)"
      Abort
    ${Else}
      StrCpy $0 $R1 1
      ${IfThen} $0 == '"' ${|} StrCpy $R1 $R1 -1 1 ${|} ; Strip quotes from UninstallString
      Delete $R1
      RMDir $INSTDIR
    ${EndIf}
  reinst_done:
FunctionEnd

; 5. Choose install directoy page
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_DIRECTORY

; 6. Start menu shortcut page
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
Var AppStartMenuFolder
!insertmacro MUI_PAGE_STARTMENU Application $AppStartMenuFolder

; 7. Installation page
!insertmacro MUI_PAGE_INSTFILES

; 8. Finish page
;
; Don't auto jump to finish page after installation page,
; because the installation page has useful info that can be used debug any issues with the installer.
!define MUI_FINISHPAGE_NOAUTOCLOSE
; Use show readme button in the finish page as a button create a desktop shortcut
!define MUI_FINISHPAGE_SHOWREADME
!define MUI_FINISHPAGE_SHOWREADME_TEXT "$(createDesktop)"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateDesktopShortcut
; Show run app after installation.
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION RunMainBinary
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_FINISH

Function RunMainBinary
  nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
FunctionEnd

; Uninstaller Pages
; 1. Confirm uninstall page
Var DeleteAppDataCheckbox
Var DeleteAppDataCheckboxState
!define /ifndef WS_EX_LAYOUTRTL         0x00400000
!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.ConfirmShow
Function un.ConfirmShow
    FindWindow $1 "#32770" "" $HWNDPARENT ; Find inner dialog
    ${If} $(^RTL) == 1
      System::Call 'USER32::CreateWindowEx(i${__NSD_CheckBox_EXSTYLE}|${WS_EX_LAYOUTRTL},t"${__NSD_CheckBox_CLASS}",t "$(deleteAppData)",i${__NSD_CheckBox_STYLE},i 50,i 100,i 400, i 25,i$1,i0,i0,i0)i.s'
    ${Else}
      System::Call 'USER32::CreateWindowEx(i${__NSD_CheckBox_EXSTYLE},t"${__NSD_CheckBox_CLASS}",t "$(deleteAppData)",i${__NSD_CheckBox_STYLE},i 0,i 100,i 400, i 25,i$1,i0,i0,i0)i.s'
    ${EndIf}
    Pop $DeleteAppDataCheckbox
    SendMessage $HWNDPARENT ${WM_GETFONT} 0 0 $1
    SendMessage $DeleteAppDataCheckbox ${WM_SETFONT} $1 1
FunctionEnd
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.ConfirmLeave
Function un.ConfirmLeave
    SendMessage $DeleteAppDataCheckbox ${BM_GETCHECK} 0 0 $DeleteAppDataCheckboxState
FunctionEnd
!insertmacro MUI_UNPAGE_CONFIRM

; 2. Uninstalling Page
!insertmacro MUI_UNPAGE_INSTFILES

;Languages
{{#each languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}
!insertmacro MUI_RESERVEFILE_LANGDLL
{{#each language_files}}
  !include "{{this}}"
{{/each}}

!macro SetContext
  !if "${INSTALLMODE}" == "currentUser"
    SetShellVarContext current
  !else if "${INSTALLMODE}" == "perMachine"
    SetShellVarContext all
  !endif

  ${If} ${RunningX64}
    !if "${ARCH}" == "x64"
      SetRegView 64
    !else if "${ARCH}" == "arm64"
      SetRegView 64
    !else
      SetRegView 32
    !endif
  ${EndIf}
!macroend

Var PassiveMode
Function .onInit
  ${GetOptions} $CMDLINE "/P" $PassiveMode
  IfErrors +2 0
    StrCpy $PassiveMode 1

  !if "${DISPLAYLANGUAGESELECTOR}" == "true"
    !insertmacro MUI_LANGDLL_DISPLAY
  !endif

  !insertmacro SetContext

  ${If} $INSTDIR == ""
    ; Set default install location
    !if "${INSTALLMODE}" == "perMachine"
      ${If} ${RunningX64}
        !if "${ARCH}" == "x64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else if "${ARCH}" == "arm64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else
          StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
        !endif
      ${Else}
        StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
      ${EndIf}
    !else if "${INSTALLMODE}" == "currentUser"
      StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"
    !endif

    Call RestorePreviousInstallLocation
  ${EndIf}


  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_INIT
  !endif
FunctionEnd


Section EarlyChecks
  ; Abort silent installer if downgrades is disabled
  !if "${ALLOWDOWNGRADES}" == "false"
  IfSilent 0 silent_downgrades_done
    ; If downgrading
    ${If} $R0 == -1
      System::Call 'kernel32::AttachConsole(i -1)i.r0'
      ${If} $0 != 0
        System::Call 'kernel32::GetStdHandle(i -11)i.r0'
        System::call 'kernel32::SetConsoleTextAttribute(i r0, i 0x0004)' ; set red color
        FileWrite $0 "$(silentDowngrades)"
      ${EndIf}
      Abort
    ${EndIf}
  silent_downgrades_done:
  !endif

SectionEnd

Section WebView2
  ; Check if Webview2 is already installed and skip this section
  ${If} ${RunningX64}
    ReadRegStr $4 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${Else}
    ReadRegStr $4 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${EndIf}
  ReadRegStr $5 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"

  StrCmp $4 "" 0 webview2_done
  StrCmp $5 "" 0 webview2_done

  ; Webview2 install modes
  !if "${INSTALLWEBVIEW2MODE}" == "downloadBootstrapper"
    Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    DetailPrint "$(webview2Downloading)"
    NSISdl::download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    Pop $0
    ${If} $0 == "success"
      DetailPrint "$(webview2DownloadSuccess)"
    ${Else}
      DetailPrint "$(webview2DownloadError)"
      Abort "$(webview2AbortError)"
    ${EndIf}
    StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    Goto install_webview2
  !endif

  !if "${INSTALLWEBVIEW2MODE}" == "embedBootstrapper"
    Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    File "/oname=$TEMP\MicrosoftEdgeWebview2Setup.exe" "${WEBVIEW2BOOTSTRAPPERPATH}"
    DetailPrint "$(installingWebview2)"
    StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    Goto install_webview2
  !endif

  !if "${INSTALLWEBVIEW2MODE}" == "offlineInstaller"
    Delete "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
    File "/oname=$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe" "${WEBVIEW2INSTALLERPATH}"
    DetailPrint "$(installingWebview2)"
    StrCpy $6 "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
    Goto install_webview2
  !endif

  Goto webview2_done

  install_webview2:
    DetailPrint "$(installingWebview2)"
    ; $6 holds the path to the webview2 installer
    ExecWait "$6 ${WEBVIEW2INSTALLERARGS} /install" $1
    ${If} $1 == 0
      DetailPrint "$(webview2InstallSuccess)"
    ${Else}
      DetailPrint "$(webview2InstallError)"
      Abort "$(webview2AbortError)"
    ${EndIf}
  webview2_done:
SectionEnd

!macro CheckIfAppIsRunning
  !if "${INSTALLMODE}" == "currentUser"
    nsis_tauri_utils::FindProcessCurrentUser "${MAINBINARYNAME}.exe"
  !else
    nsis_tauri_utils::FindProcess "${MAINBINARYNAME}.exe"
  !endif
  Pop $R0
  ${If} $R0 = 0
      IfSilent kill 0
      ${IfThen} $PassiveMode != 1 ${|} MessageBox MB_OKCANCEL "$(appRunningOkKill)" IDOK kill IDCANCEL cancel ${|}
      kill:
        !if "${INSTALLMODE}" == "currentUser"
          nsis_tauri_utils::KillProcessCurrentUser "${MAINBINARYNAME}.exe"
        !else
          nsis_tauri_utils::KillProcess "${MAINBINARYNAME}.exe"
        !endif
        Pop $R0
        Sleep 500
        ${If} $R0 = 0
          Goto app_check_done
        ${Else}
          IfSilent silent ui
          silent:
            System::Call 'kernel32::AttachConsole(i -1)i.r0'
            ${If} $0 != 0
              System::Call 'kernel32::GetStdHandle(i -11)i.r0'
              System::call 'kernel32::SetConsoleTextAttribute(i r0, i 0x0004)' ; set red color
              FileWrite $0 "$(appRunning)$\n"
            ${EndIf}
            Abort
          ui:
            Abort "$(failedToKillApp)"
        ${EndIf}
      cancel:
        Abort "$(appRunning)"
  ${EndIf}
  app_check_done:
!macroend

Section Install
  SetOutPath $INSTDIR

  !insertmacro CheckIfAppIsRunning

  ; Keep the current opt-in value across overlay updates. The application only
  ; creates it after an explicit settings-page click and never rewrites it at
  ; startup. Old brand values are still removed as a one-way migration.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LongDecompress"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${LEGACY_PRODUCTNAME}"

  ; Remove versioned Explorer integration from the previous installation
  ; before copying this release. Otherwise every overlay upgrade leaves an
  ; obsolete shell DLL behind, and an unsigned build can accidentally reuse
  ; stale signed identity packages.
  IfFileExists "$INSTDIR\resources\long_compress_context_menu_registration.ps1" 0 context_menu_upgrade_identity_removed
    ExecWait '$"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe$" -NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File $"$INSTDIR\resources\long_compress_context_menu_registration.ps1$" -Action Uninstall' $0
  context_menu_upgrade_identity_removed:
  Delete "$INSTDIR\resources\long_compress_shell_extension_*.dll"
  Delete "$INSTDIR\resources\long_compress_context_menu_extract.msix"
  Delete "$INSTDIR\resources\long_compress_context_menu_pack.msix"
  Delete "$INSTDIR\resources\long_compress_context_menu.msix"

  ; Copy main executable
  File "${MAINBINARYSRCPATH}"

  ; Copy resources
  {{#each resources_dirs}}
    CreateDirectory "$INSTDIR\\{{this}}"
  {{/each}}
  {{#each resources}}
    File /a "/oname={{this.[1]}}" "{{unescape-dollar-sign @key}}"
  {{/each}}

  ; Copy external binaries
  {{#each binaries}}
    File /a "/oname={{this}}" "{{unescape-dollar-sign @key}}"
  {{/each}}

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Save $INSTDIR in registry for future installations
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR

  !if "${INSTALLMODE}" == "both"
    ; Save install mode to be selected by default for the next installation such as updating
    ; or when uninstalling
    WriteRegStr SHCTX "${UNINSTKEY}" $MultiUser.InstallMode 1
  !endif

  ; Save current MAINBINARYNAME for future updates from v2 updater
  WriteRegStr SHCTX "${UNINSTKEY}" "MainBinaryName" "${MAINBINARYNAME}.exe"

  ; Registry information for add/remove programs
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr SHCTX "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr SHCTX "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoModify" "1"
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoRepair" "1"
  WriteRegDWORD SHCTX "${UNINSTKEY}" "EstimatedSize" "${ESTIMATEDSIZE}"

  ; Migrate the v1.0.9 and earlier brand identity without touching user data.
  Delete "$INSTDIR\${LEGACY_PRODUCTNAME}.exe"
  Delete "$DESKTOP\${LEGACY_PRODUCTNAME}.lnk"
  Delete "$SMPROGRAMS\${LEGACY_PRODUCTNAME}\${LEGACY_PRODUCTNAME}.lnk"
  RmDir "$SMPROGRAMS\${LEGACY_PRODUCTNAME}"
  DeleteRegKey SHCTX "${LEGACY_UNINSTKEY}"
  DeleteRegKey SHCTX "${LEGACY_MANUPRODUCTKEY}"

  ; Create start menu shortcut (GUI)
  !insertmacro MUI_STARTMENU_WRITE_BEGIN Application
    Call CreateStartMenuShortcut
  !insertmacro MUI_STARTMENU_WRITE_END

  ; Create shortcuts for silent and passive installers, which
  ; can be disabled by passing `/NS` flag
  ; GUI installer has buttons for users to control creating them
  IfSilent check_ns_flag 0
  ${IfThen} $PassiveMode == 1 ${|} Goto check_ns_flag ${|}
  Goto shortcuts_done
  check_ns_flag:
    ${GetOptions} $CMDLINE "/NS" $R0
    IfErrors 0 shortcuts_done
      Call CreateDesktopShortcut
      Call CreateStartMenuShortcut
  shortcuts_done:

  ; Auto close this page for passive mode
  ${IfThen} $PassiveMode == 1 ${|} SetAutoClose true ${|}
SectionEnd

Function .onInstSuccess
  ; The Tauri updater uses passive mode without adding `/R`. Restart by
  ; default after a passive update, while allowing validation and maintenance
  ; installs to opt out explicitly with `/NR`.
  ${If} $PassiveMode == 1
    ${GetOptions} $CMDLINE "/NR" $R0
    IfErrors restart_passive run_done
    restart_passive:
      nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
      Goto run_done
  ${EndIf}

  ; Silent installs retain their explicit `/R` opt-in behavior.
  IfSilent check_r_flag run_done
  check_r_flag:
    ${GetOptions} $CMDLINE "/R" $R0
    IfErrors run_done 0
      ${GetOptions} $CMDLINE "/ARGS" $R0
      nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" "$R0"
  run_done:
FunctionEnd

Function un.onInit
  !insertmacro SetContext

  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_UNINIT
  !endif

  !insertmacro MUI_UNGETLANGUAGE
FunctionEnd

!macro DeleteAppUserModelId
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_DestinationList} ${IID_ICustomDestinationList} r1 ""
  ${If} $1 P<> 0
    ${ICustomDestinationList::DeleteList} $1 '("${BUNDLEID}")'
    ${IUnknown::Release} $1 ""
  ${EndIf}
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ApplicationDestinations} ${IID_IApplicationDestinations} r1 ""
  ${If} $1 P<> 0
    ${IApplicationDestinations::SetAppID} $1 '("${BUNDLEID}")i.r0'
    ${If} $0 >= 0
      ${IApplicationDestinations::RemoveAllDestinations} $1 ''
    ${EndIf}
    ${IUnknown::Release} $1 ""
  ${EndIf}
!macroend

; From https://stackoverflow.com/a/42816728/16993372
!macro UnpinShortcut shortcut
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_StartMenuPin} ${IID_IStartMenuPinnedList} r0 ""
  ${If} $0 P<> 0
      System::Call 'SHELL32::SHCreateItemFromParsingName(ws, p0, g "${IID_IShellItem}", *p0r1)' "${shortcut}"
      ${If} $1 P<> 0
          ${IStartMenuPinnedList::RemoveFromList} $0 '(r1)'
          ${IUnknown::Release} $1 ""
      ${EndIf}
      ${IUnknown::Release} $0 ""
  ${EndIf}
!macroend

!macro RemoveLongDecompressContextMenu
  ; Remove the sparse package before deleting its externally referenced DLL.
  IfFileExists "$INSTDIR\resources\long_compress_context_menu_registration.ps1" 0 context_menu_identity_removed
    ExecWait '$"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe$" -NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File $"$INSTDIR\resources\long_compress_context_menu_registration.ps1$" -Action Uninstall' $0
  context_menu_identity_removed:

  ; Remove classic cascade roots and direct high-frequency actions.
  DeleteRegKey HKCU "Software\Classes\*\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\*\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\*\shell\LongDecompressQuickPack"
  DeleteRegKey HKCU "Software\Classes\*\shell\LongDecompressNative"
  DeleteRegKey HKCU "Software\Classes\*\shell\LongDecompressNativeQuickExtract"
  DeleteRegKey HKCU "Software\Classes\*\shell\LongDecompressNativeQuickPack"
  DeleteRegKey HKCU "Software\Classes\directory\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\directory\shell\LongDecompressQuickPack"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\LongDecompressNative"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\LongDecompressNativeQuickPack"
  DeleteRegKey HKCU "Software\Classes\directory\Background\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\directory\Background\shell\LongDecompressQuickPack"

  ; Remove the Windows 11 IExplorerCommand COM registration.
  DeleteRegKey HKCU "Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}"
  DeleteRegKey HKCU "Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4B}"
  DeleteRegKey HKCU "Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C}"

  ; Remove all CommandStore verbs owned by the application.
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.open"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.quickExtract"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.extractHere"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.extractTo"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.testArchive"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.compressZip"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.compress7z"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.compressCustom"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.compressZipHere"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.compress7zHere"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.compressCustomHere"

  ; Archive-specific cascades cannot be removed with a registry wildcard.
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zip\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zipx\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.7z\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.rar\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tar\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.gz\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.gzip\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.bz2\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.bzip2\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.xz\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zst\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zstd\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.lzma\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tgz\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tpz\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tbz\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tbz2\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.txz\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tzst\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.iso\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.img\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.dmg\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.wim\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vhd\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vhdx\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.cab\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.msi\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.deb\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.rpm\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.lzh\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.lha\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.arj\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.chm\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.xar\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.cpio\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.squashfs\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.sfs\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.udf\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.jar\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.xpi\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.odt\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ods\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.docx\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.xlsx\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.pptx\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.epub\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ipa\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.apk\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.appx\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ova\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.aes\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.qcow\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.qcow2\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.qcow2c\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vdi\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vmdk\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.udeb\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.msp\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.msm\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.nsis\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.apfs\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext2\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext3\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext4\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.gpt\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.mbr\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.uefif\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.cramfs\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.fat\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ntfs\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.hfs\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.hfsx\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ar\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.a\shell\LongDecompress"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ihex\shell\LongDecompress"

  ; Remove direct quick-extract verbs for every currently supported archive type.
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zip\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zipx\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.7z\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.rar\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.wim\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tar\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ova\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.gz\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.gzip\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tgz\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tpz\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.bz2\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.bzip2\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tbz\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tbz2\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.xz\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.txz\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zst\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zstd\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tzst\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.aes\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.lzma\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.jar\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.xpi\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ipa\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.apk\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.appx\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.iso\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.img\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.dmg\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vhd\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vhdx\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.qcow\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.qcow2\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.qcow2c\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vdi\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.vmdk\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.cab\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.deb\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.udeb\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.rpm\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.msi\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.msp\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.msm\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.nsis\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.apfs\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext2\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext3\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ext4\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.gpt\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.mbr\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.uefif\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.cramfs\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.fat\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ntfs\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.hfs\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.hfsx\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ar\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.a\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.lzh\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.lha\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.squashfs\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.sfs\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.xar\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.cpio\shell\LongDecompressQuickExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ihex\shell\LongDecompressQuickExtract"

  ; Refresh Explorer's association cache without restarting Explorer.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

Section Uninstall
  !insertmacro CheckIfAppIsRunning
  !insertmacro RemoveLongDecompressContextMenu

  ; Remove every startup value owned by current or legacy product identities.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LongDecompress"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${LEGACY_PRODUCTNAME}"

  ; Delete the app directory and its content from disk
  ; Copy main executable
  Delete "$INSTDIR\${MAINBINARYNAME}.exe"

  ; Delete resources
  {{#each resources}}
    Delete /REBOOTOK "$INSTDIR\\{{this.[1]}}"
  {{/each}}

  ; Delete external binaries
  {{#each binaries}}
    Delete "$INSTDIR\\{{this}}"
  {{/each}}

  ; Delete uninstaller
  Delete "$INSTDIR\uninstall.exe"

  {{#each resources_ancestors}}
  RMDir /REBOOTOK "$INSTDIR\\{{this}}"
  {{/each}}
  RMDir "$INSTDIR"

  !insertmacro DeleteAppUserModelId
  !insertmacro UnpinShortcut "$SMPROGRAMS\$AppStartMenuFolder\${MAINBINARYNAME}.lnk"
  !insertmacro UnpinShortcut "$DESKTOP\${MAINBINARYNAME}.lnk"

  ; Remove start menu shortcut
  !insertmacro MUI_STARTMENU_GETFOLDER Application $AppStartMenuFolder
  Delete "$SMPROGRAMS\$AppStartMenuFolder\${MAINBINARYNAME}.lnk"
  RMDir "$SMPROGRAMS\$AppStartMenuFolder"

  ; Remove desktop shortcuts
  Delete "$DESKTOP\${MAINBINARYNAME}.lnk"

  ; Remove registry information for add/remove programs
  !if "${INSTALLMODE}" == "both"
    DeleteRegKey SHCTX "${UNINSTKEY}"
  !else if "${INSTALLMODE}" == "perMachine"
    DeleteRegKey HKLM "${UNINSTKEY}"
  !else
    DeleteRegKey HKCU "${UNINSTKEY}"
  !endif

  DeleteRegValue HKCU "${MANUPRODUCTKEY}" "Installer Language"

  ; Delete app data
  ${If} $DeleteAppDataCheckboxState == 1
    SetShellVarContext current
    RmDir /r "$APPDATA\${BUNDLEID}"
    RmDir /r "$LOCALAPPDATA\${BUNDLEID}"
  ${EndIf}

  ${GetOptions} $CMDLINE "/P" $R0
  IfErrors +2 0
    SetAutoClose true
SectionEnd

Function RestorePreviousInstallLocation
  ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
  StrCmp $4 "" 0 restore_install_location
  ReadRegStr $4 SHCTX "${LEGACY_MANUPRODUCTKEY}" ""
  restore_install_location:
  StrCmp $4 "" restore_install_location_done 0
  StrCpy $INSTDIR $4
  restore_install_location_done:
FunctionEnd

Function SkipIfPassive
  ${IfThen} $PassiveMode == 1  ${|} Abort ${|}
FunctionEnd

!macro SetLnkAppUserModelId shortcut
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P<> 0
    ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
    ${If} $1 P<> 0
      ${IPersistFile::Load} $1 '("${shortcut}", ${STGM_READWRITE})'
      ${IUnknown::QueryInterface} $0 '("${IID_IPropertyStore}",.r2)'
      ${If} $2 P<> 0
        System::Call 'Oleaut32::SysAllocString(w "${BUNDLEID}") i.r3'
        System::Call '*${SYSSTRUCT_PROPERTYKEY}(${PKEY_AppUserModel_ID})p.r4'
        System::Call '*${SYSSTRUCT_PROPVARIANT}(${VT_BSTR},,&i4 $3)p.r5'
        ${IPropertyStore::SetValue} $2 '($4,$5)'

        System::Call 'Oleaut32::SysFreeString($3)'
        System::Free $4
        System::Free $5
        ${IPropertyStore::Commit} $2 ""
        ${IUnknown::Release} $2 ""
        ${IPersistFile::Save} $1 '("${shortcut}",1)'
      ${EndIf}
      ${IUnknown::Release} $1 ""
    ${EndIf}
    ${IUnknown::Release} $0 ""
  ${EndIf}
!macroend

Function CreateDesktopShortcut
  CreateShortcut "$DESKTOP\${MAINBINARYNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${MAINBINARYNAME}.lnk"
FunctionEnd

Function CreateStartMenuShortcut
  CreateDirectory "$SMPROGRAMS\$AppStartMenuFolder"
  CreateShortcut "$SMPROGRAMS\$AppStartMenuFolder\${MAINBINARYNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\$AppStartMenuFolder\${MAINBINARYNAME}.lnk"
FunctionEnd
