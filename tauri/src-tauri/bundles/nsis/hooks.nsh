; hooks.nsh — SoloSoul Windows NSIS 安装程序自定义 hooks 与本地化文案
; 本文件由 tauri.conf.json 中的 installerHooks 配置引入。
; 仅包含展示层文案与无状态提示，不修改安装/卸载逻辑。

; ── 安装生命周期 hooks ───────────────────────────────────────────────────────
!macro NSIS_HOOK_PREINSTALL
  DetailPrint "$(preparingInstall)"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "$(installComplete)"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "$(preparingUninstall)"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "$(uninstallComplete)"
!macroend

; Force the uninstaller to exit cleanly after a successful uninstall.
; Called right before the wizard closes, both when the user clicks "Close"
; on the INSTFILES page and during silent/unattended runs.
Function un.onUninstSuccess
  Quit
FunctionEnd

; ── 自定义页面文案（由 installer.nsi 中的 MUI_WELCOMEPAGE_TITLE / TEXT 引用）────
LangString WELCOME_TITLE ${LANG_ENGLISH} "Welcome to SoloSoul"
LangString WELCOME_TEXT ${LANG_ENGLISH} "SoloSoul is a local-first, privacy-first personal digital twin.$\r$\n$\r$\nAll your data is encrypted and stored locally on this device. No cloud, no compromise.$\r$\n$\r$\nClick Next to continue."
LangString FINISH_TITLE ${LANG_ENGLISH} "Installation Complete"
LangString FINISH_TEXT ${LANG_ENGLISH} "SoloSoul has been installed successfully.$\r$\n$\r$\nClick Finish to launch the app and start building your personal digital twin."

LangString WELCOME_TITLE ${LANG_SIMPCHINESE} "欢迎使用 SoloSoul（独灵）"
LangString WELCOME_TEXT ${LANG_SIMPCHINESE} "SoloSoul 是一款本地优先、隐私优先的个人数字孪生应用。$\r$\n$\r$\n你的所有数据都将在本机加密存储，无需上传云端，也无需担心泄露。$\r$\n$\r$\n点击“下一步”继续。"
LangString FINISH_TITLE ${LANG_SIMPCHINESE} "安装完成"
LangString FINISH_TEXT ${LANG_SIMPCHINESE} "SoloSoul 已成功安装。$\r$\n$\r$\n点击“完成”启动应用，开始构建你的个人数字孪生。"

; ── 安装过程状态提示（在 INSTFILES 页面通过 DetailPrint 使用）────────────────
LangString preparingInstall ${LANG_ENGLISH} "Preparing installation..."
LangString installingFiles ${LANG_ENGLISH} "Installing files..."
LangString creatingShortcuts ${LANG_ENGLISH} "Creating shortcuts..."
LangString installComplete ${LANG_ENGLISH} "Installation complete."

LangString preparingInstall ${LANG_SIMPCHINESE} "正在准备安装..."
LangString installingFiles ${LANG_SIMPCHINESE} "正在安装文件..."
LangString creatingShortcuts ${LANG_SIMPCHINESE} "正在创建快捷方式..."
LangString installComplete ${LANG_SIMPCHINESE} "安装完成。"

; ── 卸载过程状态提示 ─────────────────────────────────────────────────────────
LangString preparingUninstall ${LANG_ENGLISH} "Preparing uninstallation..."
LangString removingFiles ${LANG_ENGLISH} "Removing files..."
LangString removingShortcuts ${LANG_ENGLISH} "Removing shortcuts..."
LangString uninstallComplete ${LANG_ENGLISH} "Uninstallation complete."

LangString preparingUninstall ${LANG_SIMPCHINESE} "正在准备卸载..."
LangString removingFiles ${LANG_SIMPCHINESE} "正在移除文件..."
LangString removingShortcuts ${LANG_SIMPCHINESE} "正在移除快捷方式..."
LangString uninstallComplete ${LANG_SIMPCHINESE} "卸载完成。"
