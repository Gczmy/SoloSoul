# macOS / Windows / Android 独立 Card 原生验收入口（RF-121）

只引入生产 Card、CardGrid 及表面样式，不启动正式客户端的账户、日志或插件初始化。所有内容均为合成数据。构建脚本检查引用边界，只额外允许无 IPC 的 Android 材质 token helper。测试页只调用独立例程的 `card_surface_report` 或 Debug Activity 的单向报告桥，不能调用正式账户或文件命令。

在 `tauri/` 使用项目支持的 Node：

```sh
node scripts/build-card-surface-fixture.mjs /tmp/card-surfaces.html
```

输出文件必须尚不存在；不会覆盖已有文件。生成的单文件 HTML 只能交给独立原生例程，不能将浏览器打开的结果当作原生通过。

`src-tauri/examples/macos_window_appearance.rs` 支持 `--card-fixture /tmp/card-surfaces.html`，只接受小于 2 MiB 的 UTF-8 普通文件。运行此例程需图形会话，以及 Cargo manifest 中 Tauri 的 `macos-private-api` feature；仅传 Cargo 命令行 feature 不能满足当前 build.rs 的配置校验。本轮临时启用此 feature，并在每次运行结束后恢复原始 manifest 字节，未提交构建配置变化。

例程保持原生窗口、隐藏恢复、缩放和全屏几何检查，再按浅/深主题测量真实 WKWebView 的两个 Card。系统材质和辅助功能属性来自当前宿主的原生命令，不伪造高对比度或减少透明度设置。颜色采样必须等待实际绘制帧及主题过渡；无绘制帧、测量缺失或断言失败均以 exit1 结束。`--unthrottled` 只供已有诊断模式使用，其结果必须标明，不能冒充生产后台节流策略的通过。

命令行直接启动本轮独立例程时，曾出现 `visibility=hidden`、0 绘制帧；最终诊断为窗口可见、WebView 未隐藏、应用未激活，文档 `visibility=visible` 但仅收到 1 帧，2.5 秒仍未完成主题过渡采样。此路径未通过。随后将同一个二进制置于专用临时 `.app` 包，通过 LaunchServices 前台启动：应用激活，正常节流下浅/深色 Card 的严格绘制帧检查通过。启动方式的对照支持环境原因，不据此前失败判定正式客户端有相同缺陷。窗口几何通过不等于 Card 绘制通过，更不证明恢复过程中没有黑帧。详见[本轮证据](../../../docs/verification/rf121-macos-native-checkpoint-2026-10-02.json)。RF-121 的三平台、辅助功能及像素合成矩阵继续待验。

图形验收应把同一个编译产物放入专用临时应用包（本轮为 `/tmp/.../CardSurfaceRegression.app`），Info.plist 使用 `com.solosoul.appearance-regression`、APPL、NSApplication、专用 executable，并用 `open -n -W -a <包路径> --stdout <日志> --stderr <日志> --args --card-fixture <HTML>` 前台启动。无需安装到 Applications。`open` 的退出码只证明启动状态：必须同时核对应用内最终 PASS、没有 FAIL/panic，以及实际 light/dark 两份报告；不能将 launcher exit0 冒充应用进程退出码。运行后仅清理此专用包。

## Android Debug 验收

Debug `merge*DebugAssets` 自动运行构建脚本，生成专用 `src/debug/assets/rf121-card-surfaces.html`。`--replace-debug-asset` 只允许此精确路径且已有文件必须含生成器标记；普通目的文件仍禁止覆盖，符号链接和非标记文件拒绝。Release 任务图不生成此资源，合并 manifest 不包含验收 Activity。

`CardSurfaceRegressionActivity` 仅加载这个内联页，禁用网络、文件/内容访问和导航，无 Tauri 初始化或真实账户。instrumented 测试核对默认系统对比度与正常动画，测试浅深色 default/floating Card 的真实帧、长文本溢出、颜色和截图背景像素；使用生产 Android token，不把中性 Card 当作原生玻璃弹窗。

设备驱动两组各加入该用例，总计支持11/回退9项。API34 ARM64 专用模拟器20项通过；默认场景、背景像素不代表辅助功能、全部文本或多端矩阵完成。详见[Android补证](../../../docs/verification/rf121-android-native-checkpoint-2026-10-02.json)。

## Windows 独立验收

`src-tauri/examples/windows_card_surfaces.rs` 复用生产 Windows 窗口配置与材质命令，仅注册合成页的测量报告。独立应用标识、窗口标签和新建输出目录中的 WebView2 profile 与正式客户端分离，不运行正式 setup，不访问账户、日志或插件。使用已有、默认关闭的 `native-perf` SDK 依赖，不新增依赖。

在 Windows 图形会话的 `tauri/` 下，按顺序执行（HTML 和输出目录必须是尚不存在的绝对路径）：

```powershell
node scripts/build-card-surface-fixture.mjs C:/Temp/rf121-card-surfaces.html
cargo test --locked -p solo_soul --features native-perf --example windows_card_surfaces
cargo build --locked -p solo_soul --features native-perf --example windows_card_surfaces
./target/debug/examples/windows_card_surfaces.exe --card-fixture C:/Temp/rf121-card-surfaces.html --output C:/Temp/rf121-windows-current --expect mica
```

系统当前确实关闭透明效果或启用高对比度时，将 `--expect` 分别改为 `transparency-off` 或 `high-contrast`，使用新的输出目录。例程只读取系统设置，不切换全局辅助功能；预期与真实设置不符即失败。未实测这些设置时不能记为通过。

每次在浅/深主题下检查首次显示、隐藏恢复、调整尺寸、最小化恢复，共8份报告；断言实际绘制帧、Card token、长文本和页面横向布局、客户区几何。读回 DWM 背景类型、Tauri 窗口主题、系统 caption 和 WebView2 背景 alpha；Mica 必须是实际类型2和透明 WebView，实色回退必须是不透明 WebView。

`CAPTION_COLOR` 和 `USE_IMMERSIVE_DARK_MODE` 在[官方 DWM 属性契约](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)中只支持 Set，不声称读回或验过标题栏颜色；第一次属性查询失败记录保留。SDK 截图采样两个生产 Card 的背景像素。PNG 仅含 WebView 内容，不含 DWM/非客户区合成；合成导航区不是正式 Sidebar/AppBar，也不证明全部文字像素、恢复中的瞬间黑帧或完整辅助功能矩阵。接受结果需应用 exit0、`result.json` 的 success=true、8份实际 DOM/原生报告与对应 PNG；超时、缺帧、旧报告、断言失败或提前关闭均以 exit1 结束。

运行结束后确认独立例程及其专用 WebView2 子进程已经退出；只清理输出目录内的测试 profile。保留结果、截图及失败记录用于审查，不安装或启动正式客户端。

2026-10-03 Windows11 Enterprise LTSC26100 x64、WebView2 154.0.4258.48 默认辅助功能设置完成8场景，实际 Mica类型2/背景alpha0与16个 Card 背景采样一致。两轮例程开发失败、最终4项回归和完整记录见[Windows补证](../../../docs/verification/rf121-windows-native-checkpoint-2026-10-03.json)。RF-121继续待验多端辅助功能及完整原生合成，不解除后续迁移依赖。
