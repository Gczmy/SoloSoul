# Windows 一体顶栏公开布局夹具（RF-1144）

夹具加载生产AppShell、AppBar、SideNavigation、PrimaryNavZone、AddPageButton、SecondaryActionBar、AuthLayout、PreviewWindow、PreviewTitlebar和窗口生命周期代码。主导航、添加页和Tools使用正式布局；账户、设置、同步与正文数据使用合成状态，公开输入框可以任意编辑。业务IPC仅提供内存中的公开示例或空结果，原生宿主仅登记窗口验证命令，不加载账户、密码、Vault、OCR模型或正式setup。

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

## 右侧对象资料面板验收

从 2026-10-21 起默认进入第一个公开对象的正式 `ObjectEditorPage`，经正式 `PageShell` / `ShellLayout` 登记对象上下文。原生窗口初始为 1280×800，保留生产 52px 顶栏、56px 功能栏以及 340px 右侧停靠面板。

- 第一个对象：48 条历史、44 个文本附件；第二个对象：17 条历史、13 个附件；第三个对象：空历史和空附件。
- 正式历史组件自行倒序排列，展开一条才加载该公开版本；附件列表自行倒序，点击可打开正式附件预览。预览正文由内存生成，`public-fixture://` 路径不映射到实际磁盘。
- 主编辑区有 14 个公开字段，可与右侧列表分别滚动。右栏随左侧对象选择同步，顶部按钮控制显示/隐藏；缩小窗口到 800px 可验证左右面板的自动避让。
- **F6** 切换浅色/深色，**F8** 保存公开布局记录。窗口最多保留 15 分钟。
- 宿主没有账户、Vault 或文件业务命令；生产认证 store、认证页面与验证守卫不允许进入构建图。验证对话框替换为空实现，关键字段验证固定拒绝，不提供登录、密码或真实文件访问。

此夹具验证的是正式组件在原生 WebView2 中的布局与交互，业务数据均为合成内容。它不覆盖真实 Vault 历史读取、附件文件权限或系统主题通知验收；本次兼容独立宿主时，未安装的系统主题通知入口明确返回不可用。
