# FE2-020 第二次补验（2026-10-08）

用户再次报告区域内/外高亮不正确，FE2-020 重新打开，等待当前 macOS 开发客户端复验。保留前次证据；本次不复用旧原生包作为当前源码证明。

## 修复与范围

- 侧栏工具区使用生产 SideNavigation CSS Modules 验证。非 auto 标准宽度覆盖旧 WebKit 颜色规则；改为 auto，原有 4px 实际宽度不变。隐藏的导航滚动条不改。
- 指针/滚轮改为捕获阶段监听，子元素拦截冒泡不阻断区域更新；同步清除旧区域，下一帧按当前几何复核。
- 兼容 MouseEvent 路径，收到鼠标/滚轮输入后解除失焦暂停；排除触摸合成的兼容鼠标事件。保留横纵方向与遮罩边界。

样式机制依据：[CSS Scrollbars 标准交互规则](https://drafts.csswg.org/css-scrollbars/#interaction)，[WebKit 标准滚动条属性支持](https://webkit.org/blog/17640/webkit-features-for-safari-26-2/)。

## 证据

`before.log/json`：三类新用例在两引擎共 6 项均失败。标准宽度用例是样式机制断言，并不是原生窗口截图。

`final.log/json`：Chromium / WebKit 共 38 项通过，包含既有对象/历史/密码保护回归。

`pixels.py/json`：直接读取 PNG 滑块像素。独立生产 hook + 生产工具区样式夹具连续三轮正文/侧栏/工具/顶栏往返，浅深主题、两引擎共 144 个像素检查；完整 AppShell + macOS/Windows 样式 + Mock IPC 连续三轮正文/侧栏往返，浅深主题、两引擎共 48 个像素检查。共 192 个样本。图片及采样坐标保存在 `screenshots/`，文件路径已转为本目录相对路径。

`fixture-layout-failure.log/json`：工具嵌套区加入后，旧鼠标坐标进入新的工具区、Chromium 自动将无按钮滚动容器加入 Tab 顺序，旧夹具 5 项失败。调整旧测试的侧栏空白命中坐标和仅夹具容器的 Tab 顺序后全部通过。

`pixels-initial-sampling-failure.json`：完整 App 初始采样点位于顶端留白或滑块圆角外侧，18 个像素检查失败。截图可见滑块自正文 padding 下方开始；改为正文起点下 60px 的滑块内部后重跑，不改产品样式掩盖采样错误。

`types.log`、`lint.log`、`format.log`：TypeScript、针对 hook 的 ESLint 与相关文件格式检查通过。

## 原生限制

本次未编译新原生包。通过 CUA 检查应用列表，当前命令行启动的开发客户端未暴露为可绑定原生应用；不能读取其窗口做原生鼠标往返验证。窗口失焦的测试是 DOM 事件，不等同真实后台激活测试。当前客户端目测结果待用户回复，不能将浏览器 38 项或 192 个像素通过作为真实 WKWebView 验收完成。
