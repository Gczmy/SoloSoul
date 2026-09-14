# macOS 内容背景延伸至原生标题栏：调研与实施方案

日期：2026-09-15。代码基线：`09ff2aa1`。目标：右侧内容背景延伸到窗口最上沿，侧边栏保持原生玻璃，交通灯仍使用系统按钮。

## 结论

可以实现，当前框架已经具备所需能力。SoloSoul 顶部整条玻璃的直接原因是项目自己的 WebView 布局约束，并非 Tauri 无法在原生交通灯下绘制网页。

建议保留 `Overlay`，让 WKWebView 贴齐完整 `contentView`，由网页背景覆盖右侧顶部；交互控件另行避让交通灯。继续使用已有的原生玻璃层与 alpha=0.001 窗口底色。

已按此方案修改客户端，并完成原生窗口及浏览器回归。下文“当前为什么会割裂”描述修改前的基线；实施结果见文末。实际 Dock / 台前调度恢复的瞬时画面及手动拖拽仍需本机目测验收。

## 当前为什么会割裂

本项目锁定 Tauri / tauri-runtime-wry 2.11.2、Wry 0.55.1、Tao 0.35.3；应用直接使用 window-vibrancy 0.8.0。以下判断基于这些已安装源码，而非仅查看最新版本文档。

1. [macOS 配置](/Users/zzc/PycharmProjects/SoloSoul/tauri/src-tauri/tauri.macos.conf.json:26)已经使用 `titleBarStyle: Overlay`，并保留 `decorations: true`、隐藏文字标题。
2. [constrain_webview](/Users/zzc/PycharmProjects/SoloSoul/tauri/src-tauri/src/commands/window/macos.rs:46)把 WKWebView 四边约束到 `NSWindow.contentLayoutGuide`。该指南对应标题栏/工具栏不遮挡的可用区域，因此网页不能覆盖原生标题栏。
3. 原生玻璃覆盖完整窗口，而 [AppShell 内容背景](/Users/zzc/PycharmProjects/SoloSoul/tauri/src/components/layout/AppShell.module.css:52)只存在于网页内部。网页内的 `top: 0` 是 WebView 顶部，CSS 的 `z-index` 或负外边距不能使它画到 WebView 之外。
4. 先前的[原生回归程序](/Users/zzc/PycharmProjects/SoloSoul/tauri/src-tauri/examples/macos_window_appearance.rs:72)还明确断言“WebView 必须留在标题栏下方”。实施时必须更新这项几何约定，同时保留材质稳定性断言。

Tauri 官方区分了两种样式：`Transparent` 显露窗口底色，`Overlay` 才允许窗口内容进入标题栏区域。当前不是少配了 Overlay，而是后续约束又缩小了网页区域。[TitleBarStyle 文档](https://docs.rs/tauri/latest/x86_64-apple-darwin/tauri/enum.TitleBarStyle.html)

Apple SDK 的 `NSWindow.h` 也说明，`FullSizeContentView` 使内容视图覆盖整个窗口；`contentLayoutRect` / `contentLayoutGuide` 用于定位不被标题栏遮挡的内容。本次核查了本机 Xcode SDK 头文件；Apple 网页正文要求 JavaScript，其 Markdown 链接访问失败，因此未把网页摘要作为唯一证据。[contentLayoutGuide 官方入口](https://developer.apple.com/documentation/appkit/nswindow/contentlayoutguide)

## 已有 issue 及与本项目的关系

以下状态于调研日通过 GitHub API 核对；“Open”不表示本项目必然复现，也不等于没有规避方式。

| 官方记录 | 状态 | 对方案的影响 |
| --- | --- | --- |
| [Tauri #2663](https://github.com/tauri-apps/tauri/issues/2663)，保留系统按钮的标题栏样式 | Closed / completed，2022-09-30 关闭 | 对应能力已经交付；无需重画交通灯或改成无边框窗口。 |
| [Wry #1747](https://github.com/tauri-apps/wry/issues/1747)，布局后交通灯自定义位置重置 | Open | 报告针对 `trafficLightPosition` / 自定义 inset。Wry 0.55.1 本地源码仍在 `drawRect:` 重设自定义位置；本方案优先保留系统位置，避免引入这条额外路径。 |
| [Tauri #13044](https://github.com/tauri-apps/tauri/issues/13044)，更改窗口标题使交通灯位置重置 | Open | 同样与自定义交通灯位置相关，不能据此推断网页不能覆盖标题栏。 |
| [Tauri #9503](https://github.com/tauri-apps/tauri/issues/9503)，Overlay 后无法拖拽 | Open | 维护者要求确认是否设置拖拽区；仅启用 Overlay 不会自动给网页元素补上拖拽行为。 |
| [Tauri #4316](https://github.com/tauri-apps/tauri/issues/4316)，局部 `acceptsFirstMouse` | Open | 全局开启首次点击会改变其他按钮的后台点击行为，不能无条件当作完整解决方案。 |
| [Tauri #15623](https://github.com/tauri-apps/tauri/issues/15623)，新窗口在不同焦点状态下拖拽异常 | Open，报告使用 Wry 0.55.1 | 表明仍需在当前实际构建中验证聚焦/失焦拖拽，不能因静态布局通过就宣称交互完成。 |

官方有窗口自定义指南，包括拖拽区及权限配置。其“透明标题栏 + 窗口底色”示例适用于整窗统一底色，不能单独表达本次左右两块不同背景。[窗口自定义指南](https://v2.tauri.app/learn/window-customization/)

## 方案比较

| 方案 | 能否实现目标 | 取舍 |
| --- | --- | --- |
| **完整 WebView + 网页分区背景，推荐** | 能让内容背景自然延伸到窗口上沿，并随侧栏边界变化 | 需补齐交通灯避让、拖拽区和布局测试；继续使用稳定的原生约束。 |
| 原生标题栏下增加局部主题色视图 | 能消除这条背景色带，网页仍在标题栏下方 | 需同步主题、左右侧栏位置、展开宽度及登录态；维护两套背景边界。适合作为保留旧网页几何的备选。 |
| 仅更改 `NSWindow.backgroundColor` / 切回 Transparent | 不能完整表达左侧玻璃、右侧实体内容 | 改为整窗不透明底色会改变已验证的玻璃路径，不适合本次目标。 |
| `decorations: false` 后重画交通灯 | 可自行绘制外观 | 额外承担原生按钮交互和窗口行为维护，不推荐。 |

推荐方案的视图结构保持简单：同一个原生窗口、同一个玻璃背景视图、同一个 WKWebView。网页的右侧不透明背景遮住其下玻璃，左侧透明部分继续显示玻璃；系统按钮保持原生层级。

## 推荐实施步骤

1. **调整原生约束的目标。** 首次安装时，把 WKWebView 四边绑定到完整 `contentView`。保留单次安装和原有父视图关系，避免每次聚焦、主题同步或缩放时重新挂载 WebView / 玻璃。
2. **分开处理背景覆盖与控件避让。** 右侧背景从窗口顶部开始；交通灯实际占用范围通过公开 AppKit 按钮几何和 `contentLayoutRect` 测量，并转成网页坐标。标题、返回按钮、品牌区、登录卡片等使用安全距离，不能把整个 WebView 再次下移。测量需响应缩放、全屏和显示器变化，不应把本机 32 点写成所有系统的固定值。
3. **覆盖已有导航布局。** 左侧展开 232px 时交通灯在侧栏内；折叠到 48px 时不能假设三个按钮都在侧栏内。右侧导航以及上/下方导航也需处理左上控件避让。同步检查 `--shell-content-top`、尺标、浮层与滚动定位，避免再次产生坐标偏移。
4. **补齐拖拽。** 当前 AppBar 没有拖拽区标记，默认 capability 也未显式允许 `core:window:allow-start-dragging`。应在空白标题区域设置拖拽，按钮、输入框等保持交互。若要求后台首次按下即可拖动，可评估局部原生拖拽区域；全局 `acceptFirstMouse` 会改变所有网页按钮行为，需要作为明确的交互取舍。沿用 Tauri 现有双击处理并验证系统偏好。
5. **保留闪黑修复。** 继续使用白色 alpha=0.001 底色、同一玻璃视图、原生阴影和系统圆角；不引入聚焦后重建材质、改动私有标题栏容器等操作。此前通过的恢复测试仍需重新验收，因为网页覆盖范围发生了变化。

## 本机布局对照结果

已用独立 AppKit + WKWebView 程序创建两组离屏窗口，保留原生玻璃与微量底色；不显示窗口、不读取用户数据。程序：[macos-titlebar-layout-probe.swift](/Users/zzc/PycharmProjects/SoloSoul/tauri/scripts/macos-titlebar-layout-probe.swift)。

| 约束目标 | 窗口内容视图高度 | WebView 高度 | 顶部未覆盖高度 |
| --- | ---: | ---: | ---: |
| contentLayoutGuide | 800 | 768 | 32 |
| 完整 contentView | 800 | 800 | 0 |
| contentLayoutGuide，缩放后 | 620 | 588 | 32 |
| 完整 contentView，缩放后 | 620 | 620 | 0 |

单位为本机 AppKit 逻辑点。两种约束下关闭按钮位置一致；程序也检查三个系统按钮与 WKWebView 身份、父视图、窗口微量底色在缩放后保持。完整约束下 WebView 覆盖了交通灯所在的几何区域。

复现命令（项目根目录）：

```bash
xcrun swiftc -module-cache-path /private/tmp/solosoul-swift-module-cache \
  -framework AppKit -framework WebKit tauri/scripts/macos-titlebar-layout-probe.swift \
  -o /private/tmp/solosoul-titlebar-layout-probe
/private/tmp/solosoul-titlebar-layout-probe
```

这项结果只证明公开 API 的几何布局可行，不证明 Tauri 集成后的最终像素、拖拽、恢复过程无闪烁。完整客户端应再验收登录/解锁、侧栏展开/折叠及换边、主题切换、全屏往返、尺寸变化、Dock/台前调度恢复，并检查右上圆角与顶部背景衔接。

## 实施结果（2026-09-15）

- WKWebView 四边绑定完整 `contentView`，右侧内容背景从窗口顶部开始；既有玻璃视图和白色 alpha=0.001 底色保持。
- `set_titlebar_color` 返回 `titlebarHeight`，由 `contentLayoutRect` 与内容视图坐标计算。前端通过共享状态/CSS 变量为 AppBar、侧栏、横向导航、登录卡片及快捷卡片留出空间；窗口 resize 时重新同步，原生实测全屏后为 0、退出后恢复。
- 顶部空白带安装一个透明原生 `NSView`，由公开 AppKit 事件处理拖拽，并局部接收后台首次鼠标按下。双击支持系统 Minimize / None 偏好，其他值执行 `performZoom`。AppBar 下方空白区沿用 Tauri 拖拽区域；按钮和 actions 排除拖拽。原生交通灯保持系统位置。
- 原生回归覆盖整个 WebView 和材质范围、微量底色、主题切换、隐藏/显示、缩放、全屏往返，以及 WebView / 材质 / 拖拽视图 / 交通灯身份稳定。交通灯命中断言在普通窗口和退出全屏后执行；全屏中它们由 AppKit 的隐藏工具栏管理，不能套用普通窗口命中坐标。全屏不显示普通窗口阴影也属系统行为。
- 同轮修复侧栏卡片遮挡：搜索原层级为 300，AI / OCR / 插件为 200，低于 AppBar 的 1000。统一为 `--z-nav-popover: 2000`，覆盖正文和顶部栏，仍低于确认/认证弹窗；新建/编辑页面浮层也统一使用此层级。搜索另行保留原生标题栏避让。回归在左右侧栏分别打开四类卡片，使用 `elementFromPoint` 检查顶部、正文和下沿的真实命中，以及搜索输入与 Esc 关闭。
- TypeScript、目标文件 ESLint、Rust Clippy（`-D warnings`）、Rust fmt 通过；原生窗口回归通过；原生外观单元测试 7 项通过；Chromium 和 WebKit 的桌面导航、标题栏、尺标场景共 18 项/引擎通过，侧栏顶部避让在最终修改后再次验证。

新增的左右侧栏四类快捷卡片命中回归在 Chromium / WebKit 均通过。macOS Debug 应用已构建到 `tauri/target/debug/bundle/macos/SoloSoul.app`，包含本次标题栏和卡片层级修复。

验收边界：浏览器测试使用模拟 IPC，原生测试使用独立窗口，不读取 Vault。自动回归证明几何和对象稳定性，不证明每一帧无闪黑。图形工具恢复后已查看真实应用登录页，卡片圆角和上下留白正常；应用仍在锁定态，尚未完成真实应用的手动后台首次拖动、双击偏好及 Dock / 台前调度恢复目测。
