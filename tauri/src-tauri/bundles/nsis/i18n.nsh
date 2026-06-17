; i18n.nsh — 覆盖 Tauri 默认 NSIS 语言字符串，确保安装/卸载全过程中英文正确显示。
; 本文件在 installer.nsi 的 {{#each language_files}} 之后引入，因此会覆盖 Tauri 自动生成的默认值。
; 键名必须与 Tauri 默认 NSIS 模板使用的 LangString 名称保持一致（包括已知的拼写错误 choowHowToInstall）。

; ── 已安装版本检测 / 重新安装页面 ────────────────────────────────────────────
LangString addOrReinstall ${LANG_ENGLISH} "Add/Reinstall components"
LangString addOrReinstall ${LANG_SIMPCHINESE} "添加/重新安装组件"

LangString alreadyInstalled ${LANG_ENGLISH} "Already Installed"
LangString alreadyInstalled ${LANG_SIMPCHINESE} "已安装"

LangString alreadyInstalledLong ${LANG_ENGLISH} "${PRODUCTNAME} ${VERSION} is already installed. Select the operation you want to perform and click Next to continue."
LangString alreadyInstalledLong ${LANG_SIMPCHINESE} "${PRODUCTNAME} ${VERSION} 已经安装。选择要执行的操作后点击下一步继续。"

LangString older ${LANG_ENGLISH} "older"
LangString older ${LANG_SIMPCHINESE} "旧的"

LangString unknown ${LANG_ENGLISH} "unknown"
LangString unknown ${LANG_SIMPCHINESE} "未知"

LangString olderOrUnknownVersionInstalled ${LANG_ENGLISH} "An $R4 version of ${PRODUCTNAME} is installed on your system. It's recommended that you uninstall the current version before installing. Select the operation you want to perform and click Next to continue."
LangString olderOrUnknownVersionInstalled ${LANG_SIMPCHINESE} "系统中已存在 $R4 版本的 ${PRODUCTNAME}。建议先卸载当前版本再继续安装。选择要执行的操作后点击下一步继续。"

LangString newerVersionInstalled ${LANG_ENGLISH} "A newer version of ${PRODUCTNAME} is already installed! It is not recommended that you install an older version. If you really want to install this older version, it's better to uninstall the current version first. Select the operation you want to perform and click Next to continue."
LangString newerVersionInstalled ${LANG_SIMPCHINESE} "已安装更新版本的 ${PRODUCTNAME}！不建议安装旧版本。如果确实需要安装此旧版本，建议先卸载当前版本。选择要执行的操作后点击下一步继续。"

LangString uninstallBeforeInstalling ${LANG_ENGLISH} "Uninstall before installing"
LangString uninstallBeforeInstalling ${LANG_SIMPCHINESE} "安装前卸载"

LangString dontUninstall ${LANG_ENGLISH} "Do not uninstall"
LangString dontUninstall ${LANG_SIMPCHINESE} "不要卸载"

LangString dontUninstallDowngrade ${LANG_ENGLISH} "Do not uninstall (Downgrading without uninstall is disabled for this installer)"
LangString dontUninstallDowngrade ${LANG_SIMPCHINESE} "不要卸载（此安装程序禁止未卸载就降级）"

LangString uninstallApp ${LANG_ENGLISH} "Uninstall ${PRODUCTNAME}"
LangString uninstallApp ${LANG_SIMPCHINESE} "卸载 ${PRODUCTNAME}"

LangString chooseMaintenanceOption ${LANG_ENGLISH} "Choose the maintenance option to perform."
LangString chooseMaintenanceOption ${LANG_SIMPCHINESE} "选择要执行的维护操作。"

; 注意：Tauri 默认模板中此处拼写为 choowHowToInstall，必须保持同名才能覆盖。
LangString choowHowToInstall ${LANG_ENGLISH} "Choose how you want to install ${PRODUCTNAME}."
LangString choowHowToInstall ${LANG_SIMPCHINESE} "选择如何安装 ${PRODUCTNAME}。"

; ── 应用运行检测 ──────────────────────────────────────────────────────────────
LangString appRunning ${LANG_ENGLISH} "${PRODUCTNAME} is running! Please close it first then try again."
LangString appRunning ${LANG_SIMPCHINESE} "${PRODUCTNAME} 正在运行！请先关闭后再试。"

LangString appRunningOkKill ${LANG_ENGLISH} "${PRODUCTNAME} is running!$\r$\nClick OK to close it."
LangString appRunningOkKill ${LANG_SIMPCHINESE} "${PRODUCTNAME} 正在运行！$\r$\n点击确定关闭它。"

LangString failedToKillApp ${LANG_ENGLISH} "Failed to close ${PRODUCTNAME}. Please close it first then try again."
LangString failedToKillApp ${LANG_SIMPCHINESE} "无法关闭 ${PRODUCTNAME}。请先手动关闭后再试。"

LangString unableToUninstall ${LANG_ENGLISH} "Unable to uninstall!"
LangString unableToUninstall ${LANG_SIMPCHINESE} "无法卸载！"

; ── 快捷方式与卸载选项 ────────────────────────────────────────────────────────
LangString createDesktop ${LANG_ENGLISH} "Create desktop shortcut"
LangString createDesktop ${LANG_SIMPCHINESE} "创建桌面快捷方式"

LangString deleteAppData ${LANG_ENGLISH} "Delete the application data"
LangString deleteAppData ${LANG_SIMPCHINESE} "删除应用程序数据"

; ── 安静安装降级提示 ──────────────────────────────────────────────────────────
LangString silentDowngrades ${LANG_ENGLISH} "Downgrades are disabled for this installer. Cannot proceed with the silent installer; please use the graphical installer instead.$\r$\n"
LangString silentDowngrades ${LANG_SIMPCHINESE} "此安装程序已禁用降级。无法继续静默安装，请使用图形界面安装程序。$\r$\n"

; ── WebView2 安装提示 ─────────────────────────────────────────────────────────
LangString webview2Downloading ${LANG_ENGLISH} "Downloading WebView2 bootstrapper..."
LangString webview2Downloading ${LANG_SIMPCHINESE} "正在下载 WebView2 引导程序..."

LangString webview2DownloadSuccess ${LANG_ENGLISH} "WebView2 bootstrapper downloaded successfully"
LangString webview2DownloadSuccess ${LANG_SIMPCHINESE} "WebView2 引导程序下载成功"

LangString webview2DownloadError ${LANG_ENGLISH} "Error: Downloading WebView2 failed - $0"
LangString webview2DownloadError ${LANG_SIMPCHINESE} "错误：下载 WebView2 失败 - $0"

LangString webview2AbortError ${LANG_ENGLISH} "Failed to install WebView2! The app can't run without it. Try restarting the installer."
LangString webview2AbortError ${LANG_SIMPCHINESE} "无法安装 WebView2！没有它应用无法运行。请尝试重启安装程序。"

LangString installingWebview2 ${LANG_ENGLISH} "Installing WebView2..."
LangString installingWebview2 ${LANG_SIMPCHINESE} "正在安装 WebView2..."

LangString webview2InstallSuccess ${LANG_ENGLISH} "WebView2 installed successfully"
LangString webview2InstallSuccess ${LANG_SIMPCHINESE} "WebView2 安装成功"

LangString webview2InstallError ${LANG_ENGLISH} "Error: Installing WebView2 failed with exit code $1"
LangString webview2InstallError ${LANG_SIMPCHINESE} "错误：安装 WebView2 失败，退出码 $1"
