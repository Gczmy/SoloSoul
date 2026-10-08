# FE2-020：鼠标悬停与原生顶部栏补验（2026-10-08）

用户要求在客户端内仅移动鼠标即可区分滚动区域内外，不以点击、滚动或键盘焦点作为高亮条件。前两轮浏览器通过结果仍未满足客户端反馈，本轮保留历史结果，重新实现并构建两份隔离 macOS 客户端。

## 当前实现

- `useActiveScrollRegion` 只登记实际可滚动的横纵轴，监听尺寸、内容和运行平台变化；不监听鼠标、指针、滚轮或焦点事件维护颜色。
- CSS 原生 `:hover` 决定滑块颜色；每个元素独立重置变量，避免嵌套区域继承错误高亮。同方向嵌套区域、弹窗和遮罩阻止后方高亮；不同方向独立判定。浅色有效为黑，深色有效为亮，区域外为灰。
- 平台识别使用 React 挂载前已有的 `data-platform`，兼容原 `data-desktop-platform`，不依赖材质 IPC 成功。移动端不进入桌面模式。
- Safari 15.0–15.3 保留基础区域悬停回退；Safari 15.4 起支持 `:has` 的精确嵌套判定，依据 [WebKit 发布说明](https://webkit.org/blog/12445/new-webkit-features-in-safari-15-4/)。本轮测试不声称已验收旧 Safari。

## 浏览器结果

[浏览器归档](browser/README.md)包含 Chromium / WebKit 合计 **46 个用例通过、192 个实际滑块像素样本通过**。其中独立生产 hook 夹具 32 例 / 144 样本，完整应用集成组 14 例 / 48 样本；后一组还包含历史键盘和移动登录回归，不能把 14 例全部计作桌面滑块像素场景。

独立夹具在应用挂载前捕获并拦截全部 JS 鼠标 / 指针移动及进出事件，用 `page.mouse.move` 连续往返，点击和滚轮计数均为 0；验证 CSS 仍随鼠标位置切换。实际像素包含浅深模式三轮正文、侧栏、工具区和顶部往返。

嵌套滑块的 `getComputedStyle(...,'::-webkit-scrollbar-thumb')` 会被滑块自身 `:hover` 选择器误导，即使实际灰色也可能报告黑色。最终断言采用元素自身 CSS 变量与真实截图像素，不用该伪元素读数替代绘制结果。两次夹具 / 断言调试失败保留在 `browser/historical-debug/`，没有抹去失败后冒称第一次即全绿。

## macOS 原生发现与修复

第一包 `6e386d811466df6afd116c1e708e594338e2dedeb14c96183a48ebf3cafebe69` 使用新 CSS/hook、旧原生顶部拖拽带。CUA 真实客户端截图显示：正文滑块黑、侧栏灰，但鼠标放到顶部原生空白栏后仍黑，原生缺口实际复现。

[WebKit 官方实现](https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WebKit/UIProcess/mac/WebViewImpl.mm)中的 `WKMouseTrackingObserver` 会先命中窗口内容视图，并仅处理命中 WKWebView 视图树的悬停。SoloSoul 透明 `TitlebarDragView` 覆盖顶部空白栏，命中在该视图时 WebKit 跳过更新。此机制解释了本轮真实观察到的顶部残留，不能据此推断所有用户环境的其他问题。

第二包 `26cbfd224ab1c37bef6e0e9f272bba78962b41dbdb5ee818214d36dc4447a038` 仅增加公开 AppKit 修复：顶部拖拽视图 `hitTest` 在真实 `MouseMoved / MouseEntered / MouseExited / CursorUpdate` 事件返回空，让 WKWebView 获得悬停命中。鼠标按下、拖动、抬起保留现有原生处理。未调用私有 WebKit 接口、未伪造事件、未改变材质参数或窗口委托。公开 API 依据：[NSWindow.currentEvent](https://developer.apple.com/documentation/appkit/nswindow/currentevent?language=objc)、[NSView.hitTest](https://developer.apple.com/documentation/appkit/nsview/hittest%28_%3A%29?language=objc)。

两次构建都在独立临时工作区 / APFS 克隆 target 中完成；生产输入差异严格仅 `macos_titlebar.rs` 一项，9 项平台生成输入及第一包冻结产物均保持不变。[构建差异](native-build/production-input-diff.json)和各包 `source.json / report.json` 保留来源。构建报告的 `window_launched=false` 是构建阶段状态，后续 CUA 启动观察单独记录，未篡改历史构建报告。

TypeScript、相关 ESLint / Prettier、差异检查通过；原生 `cargo check -p solo_soul --lib` 和文件 rustfmt 通过。两项定向 Rust 测试在原工作区与独立 target 各通过一次，不重复计作四项：4 种悬停事件穿透，左 / 右 / 其他按钮按下、拖动、抬起及滚轮 / 键盘事件不穿透。独立执行原始结果见 [titlebar-tests.log](native-build/after-titlebar-fix/titlebar-tests.log)，2 passed / 0 failed / 783 filtered，未发生链接错误或清理正式 target。

## 原生验收边界

第二包已通过 CUA 启动、解锁原有合成账户并进入首页，正文滑块黑。随后向顶部移动验证时系统自动锁屏，CUA 要求用户手动解锁；已请求解锁，**顶部修复效果尚未完成原生复验**。完整阶段记录见 [native-observations.json](native-observations.json)。

CUA 当前原生 API 提供点击 / 拖动，没有单独鼠标移动接口。第一包观察使用点击定位指针的实际客户端截图；不能把这些操作冒称“原生纯移动、零点击”测试。纯移动零点击证据来自浏览器夹具。原生截图仅出现在 CUA 工具输出，本目录未导出其 PNG 或测量 RGB，未以浏览器像素冒充原生像素。

FE2-020 保持进行中，等待当前包原生区域切换和用户连续鼠标移动复验；其他平台的原生任务、完整窗口生命周期 / 首次后台拖拽 / 双击矩阵均不由本轮证据关闭。未提交、推送或发布。
