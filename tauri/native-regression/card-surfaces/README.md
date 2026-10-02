# macOS 独立 Card 原生验收入口（RF-121）

只引入生产 Card、CardGrid 及表面样式，不启动正式客户端的账户、日志或插件初始化。所有内容均为合成数据。构建脚本检查引用边界，不允许新增其他生产 TS 模块。测试页只调用独立例程的 `card_surface_report`，不能调用正式账户或文件命令。

在 `tauri/` 使用项目支持的 Node：

```sh
node scripts/build-card-surface-fixture.mjs /tmp/card-surfaces.html
```

输出文件必须尚不存在；不会覆盖已有文件。生成的单文件 HTML 只能交给独立原生例程，不能将浏览器打开的结果当作原生通过。

`src-tauri/examples/macos_window_appearance.rs` 支持 `--card-fixture /tmp/card-surfaces.html`，只接受小于 2 MiB 的 UTF-8 普通文件。运行此例程需图形会话，以及 Cargo manifest 中 Tauri 的 `macos-private-api` feature；仅传 Cargo 命令行 feature 不能满足当前 build.rs 的配置校验。本轮临时启用此 feature，并在每次运行结束后恢复原始 manifest 字节，未提交构建配置变化。

例程保持原生窗口、隐藏恢复、缩放和全屏几何检查，再按浅/深主题测量真实 WKWebView 的两个 Card。系统材质和辅助功能属性来自当前宿主的原生命令，不伪造高对比度或减少透明度设置。颜色采样必须等待实际绘制帧及主题过渡；无绘制帧、测量缺失或断言失败均以 exit1 结束。`--unthrottled` 只供已有诊断模式使用，其结果必须标明，不能冒充生产后台节流策略的通过。

本轮独立例程全屏恢复后曾出现 `visibility=hidden`、0 绘制帧；最终诊断为窗口可见、WebView 未隐藏、应用未激活，文档 `visibility=visible` 但仅收到 1 帧，2.5 秒仍未完成主题过渡采样。两者均未通过，尚不能判定正式客户端有相同缺陷。窗口几何通过不等于 Card 绘制通过，更不证明恢复过程中没有黑帧。详见[本轮证据](../../../docs/verification/rf121-macos-native-checkpoint-2026-10-02.json)。RF-121 的三平台、辅助功能及像素合成矩阵继续待验。
