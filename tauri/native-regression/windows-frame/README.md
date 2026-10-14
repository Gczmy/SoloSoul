# Windows 一体顶栏的原生窗框验证（RF-1142）

独立例程使用真实 Tauri/WebView2 和生产窗口命令。它不调用正式应用 setup，不读取账户、Vault、插件或持久主题；WebView 数据目录在指定的新输出目录中。

RF-1142验证并接入原生窗框。RF-1144的正式前端在启动时登记控件避让区并显式启用；不支持时撤除并回退旧布局。独立例程无第三个参数时仍运行原来的窗框验证页面；第三个参数可加载[公开前端布局夹具](../windows-shell/README.md)。

为避免每次原生迭代链接OCR和Vault，可用轻量构建入口（保留当前D盘Cargo缓存环境）：

```powershell
python native-regression/windows-frame/build.py D:\SoloSoul\build\new-windows-frame-build
```

该入口复用工作区锁文件、冻结输入，直接编译生产窗口模块和同一例程。只将例程的库导入改为模块导入；生成的独立项目和锁文件不写回生产目录。生成入口的首行文档注释改为普通注释，功能代码保持。可在生成项目上执行`cargo test`和严格Clippy。

```powershell
cargo build --locked -p solo_soul --example windows_frame_regression --features native-perf --release
# 输出目录必须是尚不存在的绝对路径。
.\target\release\examples\windows_frame_regression.exe D:\SoloSoul\build\new-windows-frame-run
```

例程注册一个有界测试命令及两项生产窗口命令；mock上下文仅为本地单窗口放行标准拖动脚本用到的两项窗口命令。页面的 Interact、Light/Dark、Disable/Enable 按钮分别检查网页交互、生产主题同步和窗框安装/撤除；F8 读取当前窗口布局和原生命中结果。原生关闭、移动、尺寸变化另行记录，窗口最长保留15分钟。

正式接入契约：

- `set_titlebar_controls({ regions, windowsFrame: true })` 在窗口主线程安装窗框；区域使用 WebView 逻辑像素，只允许顶部52px内的有限非负矩形，最多128项。
- 不传 `windowsFrame` 的旧调用不会启用新窗框；启用后可更新已有区域。
- 显式 `windowsFrame: false` 撤除窗框并恢复原生标题栏。
- `get_window_layout` 仅在启用时增加 `windowsFrame`，包括实际DPI、最大化状态及窗口按钮保留区。未启用和其他平台的既有返回形状保持。
- 主题、布局和窗框操作均在所属窗口主线程进行；主题变化不重新安装窗框，销毁时释放该窗口的状态。
- 只对直属`WRY_WEBVIEW`宿主排除原生按钮区域，拒绝覆盖它的既有区域；尺寸变化后更新，撤除/全屏时恢复。页面的空白拖动区域仍由Tauri标准拖动标记声明。

必须检查真实窗口按钮可见且可点击、网页按钮可点击、空白拖动、边缘缩放、双击最大化/还原、最小化/恢复、启用/撤除和浅深色。原生 `WM_NCHITTEST` 结果只能证明命中处理，不能替代真实点击或Snap菜单出现的验收；当前DPI下的通过也不能替代跨DPI/跨屏矩阵。

原生方案参考：[微软自定义窗框](https://learn.microsoft.com/en-us/windows/win32/dwm/customframe)、[Snap Layout命中要求](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-snap-layout-menu)。实际验证结果另存于 `docs/verification`，不得将尚未执行的检查写成通过。

2026-10-13原生阶段验收完成：18项窗口单测、隔离及生产库严格Clippy、Rust格式、IPC/ACL通过。Windows 11/96DPI真实按钮、交互、顶部/右侧缩放、双击、最小化/恢复、主题与安装/撤除通过；拖动与Snap由用户补验，原生位移/尺寸另行核验。跨DPI/跨屏、旧系统材质回退、高对比度/关闭透明度和全屏矩阵未实测。RF-1144前端迁移及本轮原生结果另存于仓库验证记录，其他任务及goal继续暂停。
