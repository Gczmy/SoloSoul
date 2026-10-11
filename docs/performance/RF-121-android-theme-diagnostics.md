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

最终 10 张截图已实际查看。API31 失败截图是用例结束后的 launcher，不是深色应用像素证明；API31浅色帧有瞬时原生启动图标。API35两张浅色帧仍是原生启动屏，深色帧视觉上仍像浅色，最后浅色帧输入框边界不同。原生配置/媒体查询/变量/栏状态是实际读回，但截图与绘制同步尚未验收，也不能把这些图片视为完整深色视觉通过。下一步先核查真实 WebView 绘制完成与截图时点，再使用原严格状态用例验证旧 WebView 的主题传递修复。

[结构化证据](../verification/rf1124-android-theme-diagnostics-2026-10-11.json)及对应目录的索引/原件保留当前与失败结果。归档不含私有 TAR、旧外部快照内容、恢复 Base64 载荷、APK、JNI、模型；没有性能、物理设备或完整材质矩阵验收声明。
