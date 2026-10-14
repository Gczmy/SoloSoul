# Windows 一体顶栏公开布局夹具（RF-1144）

夹具加载生产AppShell、AppBar、SideNavigation、PrimaryNavZone、AddPageButton、SecondaryActionBar、AuthLayout、PreviewWindow、PreviewTitlebar和窗口生命周期代码。主导航、添加页和Tools使用正式布局；账户、设置、同步与正文使用合成状态，公开输入框可以任意编辑。业务IPC仅返回空结果，原生宿主仅登记窗口验证命令，不加载账户、密码、Vault、OCR模型或正式setup。

在`tauri/`运行以下命令，HTML及输出目录必须是新路径。复用D盘依赖缓存，不修改生产依赖或锁文件。

```powershell
node scripts/build-windows-shell-fixture.mjs D:\SoloSoul\build\new-shell.html
python native-regression/windows-frame/build.py D:\SoloSoul\build\new-shell-build
D:\SoloSoul\build\rf312-maintenance-20261009\cargo-target\debug\windows-frame-regression.exe D:\SoloSoul\build\new-shell-run D:\SoloSoul\build\new-shell.html
```

构建入口只输出一个内联脚本和样式的HTML，拒绝外部块、正式认证页面或正式账户/设置/同步store，并记录实际包含源码的SHA-256。原生例程默认隐藏，生产`prepareStartupWindow`显式安装窗框并显示后再操作；例程的`show_main_window`只显示隔离窗口，不读取正式窗口持久化状态，因此不覆盖正式应用的窗口状态恢复、认证或业务功能验收。

公开页面支持折叠导航、计数按钮、明暗切换、右侧导航、认证布局和附件预览。F8保存DOM控件、正文、导航和原生按钮几何；移动、缩放及原生关闭另有事件记录，窗口最长保留15分钟。截图只存公开夹具完整画面，不能保存被其他应用覆盖的截图或WebView profile。

浏览器模拟用于四种导航、窄屏与其他平台回归；真实Tauri/WebView2用于实际按钮、材质、窗口缩放和拖动。二者不能互相替代。Snap悬停和工具无法保持鼠标按下的拖动需要人工补验，并将结果与真实窗口几何结合记录。

Windows.Graphics.Capture在本机可能漏采DWM绘制的窗口按钮图标；不能仅凭截图空白认定真实按钮消失。RF-1144已结合原生按钮点击、窗口事件与用户肉眼确认验收，相关采集差异保留在验证记录中。

全局样式与正式 bootstrapApp 入口对齐，包含 animations.css 的系统/原生减少动态效果规则。侧栏验收同时覆盖普通模式、系统媒体查询减少动态效果与原生 data-reduce-motion；不能用遗漏该样式的夹具证明正式程序无二次位移。
