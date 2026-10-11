# RF-1124 Android 系统主题分层诊断

本项只补齐原生失败证据；生产主题逻辑未修改，RF-121/312 保持未验收。基线为 `c714a7196`。同一冻结的 RF-1121 默认生产 APK 与本项最终测试 APK，分别在专用 API31/API35 模拟器的 supported/fallback 模式运行原 `systemThemeFollowsLightDarkLight`。

| 深色检查点 | API31 / WebView91 | API35 / WebView124 |
| --- | --- | --- |
| 系统 nightMode | 2 | 2 |
| Activity / Application night mask | 32 / 32 | 32 / 32 |
| Android / AppCompat isLightTheme | false / false | false / false |
| `prefers-color-scheme: dark` | false | true |
| 前端 theme | light | dark |
| 现有严格状态用例 | 两组均超时失败 | 两组均通过 light→dark→light |

API31 provider 为 `com.android.webview 91.0.4472.114`，API35 为 `124.0.6367.219`；两边 `forceDark=1`（AUTO），页面无 color-scheme meta。API35 `algorithmicDarkeningAllowed=false`，API31 不读取不受平台支持的该设置。原 45 秒预算、实际媒体查询、完整主题变量和系统栏断言保持不变，不模拟媒体信号，不把诊断采集成功当作用例通过。

已确定失败出现在原生主题配置向 WebView 媒体查询传递的环节。尚未通过反事实运行唯一证明旧 provider、legacy ForceDark 策略或缺少 meta 中哪项是直接原因；API版本和provider同时不同，不能仅凭这两组对照证明唯一因果。[Android 官方说明](https://developer.android.com/develop/ui/views/layout/webapps/dark-theme)描述了 isLightTheme、内容自身深色样式及旧 ForceDark 策略的区别；这为后续功能检测和兼容策略提供线索，不能替代真实红绿测。

10 项 Kotlin 单元测试实际通过，0 失败/错误/跳过。重试构建中既有单元任务为 UP-TO-DATE，保留首次 XML 和复用事实；最终测试源码重新编译、APK包名和签名验证实际通过。冻结 1837 份源码，默认生产 APK/JNI未重编译：前一阶段变化仅 Windows 非默认原生观察器及其测试/Node读者，不进入 Android 默认业务逻辑。Android production、依赖和锁文件未改变。

首次驱动的 supported 组完整采集真实超时；收尾用 adb push 恢复四个旧证据文件字节，却令所有者变成 shell，第二组发生 EACCES，原失败保留。最终诊断改为每轮独立文件名，不覆盖旧证据。受限 androidTest 恢复入口以真实应用 UID 重建四个既有证据文件，原 SHA、UID 和私有目录全部核验；专用恢复用例通过。API31 重试两组均完整诊断但主题仍失败；API35 两组状态用例通过。每次应用私有文件、链接、权限、夜间模式和模糊设置均恢复；两台精确身份绑定的自有模拟器正常退出，没有强制终止用户进程。

最终 10 张截图已实际查看。API31 失败截图是用例结束后的 launcher，不是深色应用像素证明；API31浅色帧及API35首次浅色帧有启动标志，深色帧视觉上仍像浅色，最后浅色帧输入框边界不同。此前称启动标志来自“原生启动屏”的证据不足：`tauri/index.html` 也包含相同标志的 HTML 启动覆盖层，旧截图没有绑定其来源。原生配置/媒体查询/变量/栏状态是实际读回，但这批截图与绘制同步尚未验收，不能视为完整深色视觉通过。RF-1125单独补齐真实 WebView 绘制与截图像素绑定，旧原件保持不变。

[结构化证据](../verification/rf1124-android-theme-diagnostics-2026-10-11.json)及对应目录的索引/原件保留当前与失败结果。归档不含私有 TAR、旧外部快照内容、恢复 Base64 载荷、APK、JNI、模型；没有性能、物理设备或完整材质矩阵验收声明。


## RF-1125 绘制与像素补验

主题用例现在等待实际WebView的visual callback及其后一次onDraw返回，重新核对相同视图、真实主题变量、媒体查询和系统栏。按实际卡片CSS及几何读取PNG内两个无遮挡内点，容差沿用既有不透明卡片测试的每通道6；像素不匹配即用例失败。原45秒主题预算与切换命令保持。

同一生产APK在API35两种模糊配置均通过原用例，6帧/12像素匹配；API31两种配置的首次浅色2帧/4像素匹配，原深色媒体查询超时仍失败。8张合格卡片原图已实际查看，独立编码PNG读者每API拒绝6种篡改。这个结果只证明本次bootstrap不透明卡片的截图可信；生产主题未修复，API31深色、物理设备及完整玻璃材质/平台矩阵未验收。

API31首次启动超时、第一次来源检查的Cargo换行差异拒绝均保留，未削弱应用断言。四组私有数据和原设置完整恢复，专用模拟器正常退出；24个确切备份/旧快照临时文件已清理。[RF-1125证据](../verification/rf1125-android-theme-rendered-pixels-2026-10-11.json)包含最终来源、原通过/失败、事件绑定及原图SHA。


## RF-1126 旧provider兼容修复

旧实现的系统主题传递现在按WebViewFeature能力处理。缺少ALGORITHMIC_DARKENING、但支持FORCE_DARK和FORCE_DARK_STRATEGY时，在WebView创建和真实配置变化后选择WEB_THEME_DARKENING_ONLY并随系统切换ON/OFF；页面仍由自己的CSS及显式偏好控制配色。支持新机制时保持既有自动主题行为。[Android官方说明](https://developer.android.com/develop/ui/views/layout/webapps/dark-theme)区分了网页配色和自动反色，并说明旧模式需要跟随配置变化。

本轮同一新生产APK在API31/WebView91、API35/WebView124上，以两种模糊配置分别执行原四个主题方法：16通过、0失败/跳过，48帧/96个编码PNG采样像素匹配。显式浅色、显式深色、保存色板及自定义强调色全部保持；原45秒、媒体、系统栏和实际绘制断言不变。此前同API31原失败保留作为修复前证据。每API实际审阅四个supported深色检查点，其余帧由独立像素读者核验。

诊断字段forceDarkStrategy保留公开getter的真实返回，但不是实际策略读回：当前1.14 AAR字节码显示它委托getForceDark，原读者期待1而失败。记录中保留0/2及这一限制，未更改生产依赖；[AndroidX源码](https://raw.githubusercontent.com/androidx/androidx/androidx-main/webkit/webkit/src/main/java/androidx/webkit/WebSettingsCompat.java)提供交叉核对，当前安装版本以本地AAR/JAR哈希和javap输出为准。主题验收仍依据真实媒体、CSS、系统栏及像素，不能据此声称读取策略1。

生产Kotlin、测试APK和10项既有单元实际通过；ZIP中全部资源和JNI与原件SHA一致。16次私有文件和设置恢复，专用模拟器正常退出；97个明确临时文件已删除。[本项证据](../verification/rf1126-legacy-webview-theme-2026-10-11.json)保留来源、原红灯、读者失败与最终结果。这个修复不代表物理设备、其他provider或完整跨平台材质/辅助功能/性能矩阵验收。
