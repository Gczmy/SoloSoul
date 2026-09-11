# 桌面启动与原生材质改造

执行日期：2026-09-11。范围来自已确认方案：品牌启动层、初始化兜底、macOS Liquid Glass/Vibrancy、Windows Mica、桌面导航展开及内容层级。每项验证后独立提交，完成一项再进入下一项。

| 项目 | 状态 | 验证与边界 |
|------|------|------------|
| D01 品牌启动首帧 | 已完成（DOM 层） | 静态启动层不依赖 React/IPC；账户路由绘制后淡出，系统减少动态效果时不播放过渡 |
| D02 初始化状态与异常兜底 | 已完成 | 缓存先行、应用模块后移、超时/错误/重试、启动诊断 |
| D03 macOS 原生材质 | 实现完成，原生验收待补 | macOS 26+ Liquid Glass、旧版 Vibrancy、主题与系统可访问性联动、首次显示 |
| D04 Windows 原生材质与外壳 | 实现完成，原生验收待补 | Windows 11 Mica、兼容回退、原生标题栏交互、首次显示 |
| D05 桌面导航与内容层级 | 待实施 | 可展开侧栏、现有导航位置兼容、统一登录与内容表面 |

## 验收目标

- 窗口首个可见帧已有图标，进入解锁页不闪空白，不设置人为最短启动时长。
- 初始化失败、长时间等待与重试均有反馈；账户未确认时不展示工作区。
- 系统材质透过外壳，正文/输入/敏感字段保持可读；主题更新不以实色覆盖材质。
- 保留 macOS 窗口按钮与全屏、Windows 拖动/缩放/贴靠行为。
- 浅深色、透明关闭、高对比度、减少动态效果、Windows DPI 缩放均有对应验证。
- 所有实际验证和未完成验证分别记录，不以浏览器模拟替代原生系统材质验证。

## D01 验证记录

- 图标直接引用现有 `src-tauri/icons/icon.svg`，生产构建自动打包资源（当前内联为 SVG data URL）；静态启动样式在 React 之前加载。
- 路由账户状态明确后，等待目标页面绘制再做 180ms 淡出；支持 StrictMode 清理与减少动态效果。
- TypeScript、ESLint、Vite 生产构建通过；启动层 3 个单元测试、真实 Chrome 2 个 E2E 通过（阻断主模块、缓存浅色覆盖系统深色）。
- 原生窗口隐藏/首次显示由 D03/D04 实施；D01 验证的是 WebView 首帧。
- 构建发现原有 CloudSyncPage.module.css 六处括号/变量拼写错误，已先单独修复并提交 `82664835`，恢复生产 CSS 压缩。

## D02 验证记录

- 独立 startup.js 管理准备中、失败、重试及诊断；1.5 秒显示等待提示，8 秒提供恢复入口。重新加载隔离迟到响应，错误页不会被旧请求撤下。
- 主模块仅动态加载应用入口；语言与主题读取各设 600ms 上限，偏好读取 1200ms 上限，失败沿用缓存/系统默认值。账户未确认时保持启动层。
- 缓存保存实际解析的主题色；不可用的 localStorage 不阻断语言初始化，浏览器语言不再提前写成显式偏好。
- TypeScript、定向 ESLint、Vite 生产构建通过；4 份单元测试共 49 项通过；真实 Chrome 5 项 E2E 通过，包括偏好 IPC 永不返回仍进入登录页。

## D03 验证记录

- 使用已发布 window-vibrancy 0.8.0 的公开 Regular Liquid Glass；库在 macOS 26 以下拒绝新 API，随后回退 Sidebar Vibrancy。AppKit 操作限定主线程，每窗只保留一个材质视图。
- 主题同步只更新外观，不再将玻璃覆盖为不透明背景；系统减少透明度/提高对比度时清除材质，减少动态效果时关闭启动过渡。复用现有系统主题轮询感知可访问性变化。
- macOS 主窗口默认隐藏，图标解码与 DOM 就绪后显示；关闭窗口状态插件的可见性恢复，12 秒原生兜底避免模块失败导致窗口永久隐藏。
- 当前 Windows 主机 cargo check、严格 Clippy、TypeScript、定向 ESLint、ACL 206 命令检查通过；启动/IPC 16 项单元测试及 6 项 Chrome E2E 通过。材质 E2E 使用平台响应 mock，仅验证前端透出和强制颜色回退。
- macOS 26/旧 macOS 的实际编译、材质观感、窗口按钮与辅助功能切换仍需原生机器验证，不以 Windows 编译或浏览器结果替代。
- API 依据：[window-vibrancy](https://github.com/tauri-apps/window-vibrancy) 的已发布 0.8.0 实现。

## D04 验证记录

- Windows 11 应用 Mica，Windows 10/API 不可用、透明度关闭和高对比度场景回退实色；透明度、对比度与动画偏好变化复用现有轮询通知前端。
- 原生标题栏仍负责窗口按钮、拖动、缩放、Snap 与 DPI；移除材质上方的固定标题栏颜色覆盖，WebView2 背景同步透明。
- macOS/Windows 先恢复窗口位置和大小；最大化/全屏延后至品牌首帧就绪，避免窗口状态插件提前显示空窗。
- 最终 Rust fmt 与严格 Clippy 通过；Chrome 7 项启动 E2E 通过，包含 Mica 响应与图标解码之后才请求首次显示。
- Windows 原生程序构建成功（8m44s）；随后窗口恢复时序调整通过最终严格 Clippy。原生访问工具请求应用授权超时，未获得窗口截图，也未完成实际 Mica、缩放或 Snap 验收；最终整合包仍需重建与原生验收。
- API 依据：[Microsoft Mica](https://learn.microsoft.com/en-us/windows/apps/design/style/mica) 和 [DWM 窗口属性](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)。
