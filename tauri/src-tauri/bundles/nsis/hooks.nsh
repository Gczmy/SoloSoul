!macro NSIS_HOOK_PREINSTALL
  DetailPrint "SoloSoul installer preparing..."
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "SoloSoul installation complete."
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "SoloSoul uninstaller preparing..."
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "SoloSoul uninstallation complete."
!macroend

; Force the uninstaller to exit cleanly after a successful uninstall.
; Called right before the wizard closes, both when the user clicks "Close"
; on the INSTFILES page and during silent/unattended runs.
Function un.onUninstSuccess
  Quit
FunctionEnd

; Custom localized messages for the welcome/finish pages
LangString WELCOME_TITLE ${LANG_ENGLISH} "Welcome to SoloSoul"
LangString WELCOME_TEXT ${LANG_ENGLISH} "SoloSoul is a local-first, privacy-first personal digital twin. All data is encrypted and stored locally."
LangString FINISH_TITLE ${LANG_ENGLISH} "Installation Complete"
LangString FINISH_TEXT ${LANG_ENGLISH} "SoloSoul has been installed. Click Finish to launch the app."

LangString WELCOME_TITLE ${LANG_SIMPCHINESE} "欢迎使用 SoloSoul（独灵）"
LangString WELCOME_TEXT ${LANG_SIMPCHINESE} "SoloSoul 是一款本地优先、隐私优先的个人数字孪生应用。所有数据均在本地加密存储。"
LangString FINISH_TITLE ${LANG_SIMPCHINESE} "安装完成"
LangString FINISH_TEXT ${LANG_SIMPCHINESE} "SoloSoul 已安装完成。点击完成以启动应用。"
