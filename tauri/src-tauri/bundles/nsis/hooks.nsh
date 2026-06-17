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

; 注意：欢迎页/完成页文案已移至 installer.nsi 的 inline i18n.nsh 段，
; 确保在 MUI_LANGUAGE 加载语言文件之后再定义，避免语言选择后仍显示默认中文。

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
