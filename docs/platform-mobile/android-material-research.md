# Android 材质与液态玻璃可行性调研

调研日期：2026-09-15。范围：SoloSoul 当前源码、平台官方文档、上游库和公开 issue。此次未修改应用实现，未进行 Android 真机视觉或性能验证。

## 结论

Android 可以实现实时毛玻璃，也可以通过自定义渲染实现折射、色散和边缘高光等液态玻璃效果。对 SoloSoul，关键是选择与 React + Android WebView 匹配的渲染路径。

建议先落地 **应用内局部玻璃**，用于 AppBar、底部导航、搜索浮层及底部弹层；再以单个组件验证更强的液态折射。正文保持清晰的阅读表面。第一阶段不需要迁移原生 UI，也不需要把最低 Android 版本提高到 12 或 13。

macOS Liquid Glass、Windows Mica 属于不同平台的窗口材质。项目使用的 `window-vibrancy` 支持矩阵没有 Android 后端，无法把现有调用原样移植到安卓。[window-vibrancy 官方支持矩阵](https://github.com/tauri-apps/window-vibrancy)

## 当前代码检查

| 检查项 | 当前实现 | 对材质的影响 |
| --- | --- | --- |
| 原生材质分发 | `tauri/src-tauri/src/commands/window.rs:139`，非 macOS/Windows 返回 `material: "solid"`、`platform: "other"` | Android 当前没有原生材质实现 |
| macOS | `commands/window/macos.rs:159` 调用 `apply_liquid_glass`，失败后尝试 Vibrancy | 已接入平台原生效果 |
| Windows | `commands/window/windows.rs:91` 调用 `apply_mica` | 已接入平台原生效果 |
| Android 构建 | `gen/android/app/build.gradle.kts:29`、`:45`：compile/target SDK 36，min SDK 28 | 仍需覆盖 Android 9 起的设备；新原生 API 必须做版本判断 |
| Android UI | `MainActivity.kt` 继承 `TauriActivity`；Gradle 没有 Compose 配置 | 主界面是 React/WebView，添加 Compose 库不会自动改变网页组件 |
| Android 主题 | `res/values/themes.xml`、`values-night/themes.xml` 使用 `Theme.MaterialComponents.DayNight.NoActionBar`，提供浅/深色背景 | 没有配置透明窗口和跨窗口模糊 |
| 系统栏 | `MainActivity.kt:39` 开启 edge-to-edge，多窗口另行处理 inset | 内容可延伸至系统栏；此设置本身不产生玻璃 |
| 移动顶部栏 | `tauri/src/components/layout/AppBar.module.css:98` 的移动断点覆盖为不透明 `--bg-elevated`，未设置 backdrop blur | 移动顶部栏当前为实色；通用规则的半透明 `--bg-toolbar` 在该断点不生效 |
| 移动底部栏 | `MobileBottomNav.module.css:10`、功能面板使用半透明的 `--bg-toolbar`，未设置 backdrop blur | 已有着色透明度，缺少实时背景模糊 |
| 已有网页材质 | `SideNavigation.module.css:228` 等浮层已有 `backdrop-filter` | 可提取统一材质样式，但仍需移动端验证 |
| 滚动与导航 | `AppShell.module.css:47` 为固定栏预留滚动内边距；AppBar、MobileBottomNav 使用 fixed 定位 | 适合探索内容从局部玻璃后方滚过的效果，需验证首尾留白和 inset |

上述 `commands/`、`gen/android/` 简写路径均相对 `tauri/src-tauri/`。

`androidx.webkit:webkit:1.14.0` 是 AndroidX 支持库版本，不能作为设备实际 Chromium/WebView 版本。应读取当前 WebView provider 及其版本，前端再进行 CSS 能力检测。[Android WebView Version API](https://developer.android.com/develop/ui/views/layout/webapps/managing-webview#version-api)

## 各条实现路径

### 1. Material 3 Expressive：适合作为 Android 设计方向

Material 3 Expressive 涵盖组件、形状、色彩和运动设计。它可以用于现代化 Android UI，但不能理解为给整个应用开启玻璃的系统开关。Google 的原生实现面向 Compose；SoloSoul 可以在 React 中采用相应设计原则，直接升级 Android XML 主题或 SDK 不会改造网页内容。[Android 官方设计示例](https://android-developers.googleblog.com/2025/05/androidify-building-delightful-ui-with-compose.html)

Google 已在系统通知面板使用背景模糊，但系统界面的效果不等于第三方 WebView 自动获得同样材质。[Google Material 3 Expressive 介绍](https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/)

### 2. CSS 背景模糊：最适合当前架构的第一阶段

使用半透明背景、`backdrop-filter: blur(...) saturate(...)` 和边缘高光，可以实时模糊组件背后的网页内容，文字和图标放在清晰的前景层。Chrome 自 76 起支持基本 backdrop-filter，但应按实际 WebView 检测并实测。[Chrome 团队说明](https://web.dev/articles/backdrop-filter)

这条路径提供真实的页面内背景模糊；单独使用 `blur()` 不会获得液态玻璃的光学折射。它也不会让网页直接采样桌面壁纸或其他应用。

工程判断：适合先覆盖少量固定导航和浮层。玻璃后面必须确实有内容；如果后方只是纯色，增加模糊半径也不会变得有层次。避免给全部对象卡片增加持续模糊。

### 3. Android 跨窗口模糊：用于独立浮动窗口更合适

Android 12/API 31 起提供 `Window.setBackgroundBlurRadius`、`FLAG_BLUR_BEHIND` 等公开能力，模糊的是当前窗口后面的窗口。背景模糊要求窗口透明，且支持情况取决于设备；省电等状态可能在运行时关闭效果，应通过 `isCrossWindowBlurEnabled` 和监听器适配。[AOSP Window blurs](https://source.android.com/docs/core/display/window-blurs)

对 SoloSoul 的推论：它适合独立原生 Dialog 或浮动窗口实验，无法直接让同一 WebView 内的底部导航模糊正文。把整个 Activity 透明化也不等于获得桌面版材质体验，因此不建议作为主界面首选路线。

### 4. 原生 Shader：能做液态折射，但需要渲染集成

Android 13/API 33 起可使用 AGSL `RuntimeShader` 自定义图形效果。它需要相应的绘制内容或输入图像，不能仅靠一个窗口配置将 HTML 元素变为原生玻璃。[Android AGSL 文档](https://developer.android.com/develop/ui/views/graphics/agsl)

接入现有项目需要建立原生背景来源、玻璃视图和前景控件的层级，并同步位置、裁剪、滚动和生命周期。不能把模糊直接套在整个 WebView 上，否则正文也会受到影响。原生路线的成本主要来自这部分集成，不能按“增加一个依赖”估算。

## 可以参考的现有项目

| 项目 | 已提供的能力 | SoloSoul 适配判断 |
| --- | --- | --- |
| [Kyant0 / AndroidLiquidGlass（Backdrop）](https://github.com/Kyant0/AndroidLiquidGlass) | Compose Multiplatform 的背景采样、模糊与透镜效果；Apache-2.0 | 适合参考原生折射或制作原生实验组件，需要额外接入 WebView 背景来源 |
| [Haze](https://github.com/chrisbanes/haze) | 当前 README 的 Haze 2 为 beta，包含 Blur 和实验性 Glass；Apache-2.0 | 可评估原生材质抽象；仍是 Compose 组件体系，不能直接包裹 React DOM |
| [rdev / liquid-glass-react](https://github.com/rdev/liquid-glass-react) | React 的折射、色散、高光与弹性效果；README 明确 Safari/Firefox 的位移效果受限 | 与前端技术更接近，适合单组件实验；需要审查布局、触摸和实际 Android WebView 表现 |
| [Cap-go / capacitor-native-navigation](https://github.com/Cap-go/capacitor-native-navigation) | Android 原生导航条采样 WebView 背景并模糊，旧系统退化为着色表面 | 说明混合应用有可参考的集成路径；这是 Capacitor 插件，不能直接安装进 Tauri，且其 Android “LiquidGlass” 说明主要是模糊效果 |

Backdrop 文档区分 Android 12+ 的 RenderEffect 与 Android 13+ 的 Lens/RuntimeShader。因此“支持 Android”不能等同于“最低版本设备拥有全部折射效果”。[Backdrop 效果说明](https://kyant.gitbook.io/backdrop/api/backdrop-effects)

上述判断基于上游文档及当前架构，没有把作者演示或性能声明视为 SoloSoul 真机验证结果。

## 相关 issue 与边界

| 记录 | 检索时状态/内容 | 对本项目的意义 |
| --- | --- | --- |
| [Android #527376569 对应官方修复记录](https://developer.android.com/about/versions/17/qpr2/release-notes) | 官方列在 QPR2 Beta 1 修复项：窗口模糊不显示，开发者开关重启后重置 | 说明系统材质也有版本相关问题；不能据此判断所有 Android 版本都存在此问题 |
| [AndroidLiquidGlass #82](https://github.com/Kyant0/AndroidLiquidGlass/issues/82) | Open；报告原生视频叠加玻璃出现闪烁，作者随后描述 TextureView 等处理后稳定 | 原生视图混合采样必须验证；这是视频场景报告，不能推断普通 WebView 必然同样闪烁 |
| [liquid-glass-react #20](https://github.com/rdev/liquid-glass-react/issues/20) | Open；报告强制居中变换影响正常布局定位，环境包含 React 19 + Vite | 直接替换导航组件可能引入位置偏移；应让布局与光学渲染分离 |

本轮没有找到可直接启用 Tauri Android 全窗口 Liquid Glass、并保证跨设备无闪烁的上游方案。macOS 先前通过微量背景色解决的恢复闪黑，不能直接视为 Android 合成问题的通用修复。

## 建议的实施顺序与验收

以下是针对 SoloSoul 的工程建议，尚未实施。

1. **建立 Android 材质能力描述。** 将系统窗口材质与页面内玻璃分开管理，前者目前仍为 solid，后者由实际 WebView 能力决定。不要为了启用 CSS 玻璃而伪报 `nativeMaterial=liquid-glass`，以免触发根容器透明样式。保留老设备支持，不为装饰效果提高 minSdk。
2. **实现统一的局部玻璃表面。** 先选择移动底部导航及一个搜索浮层，使用适度背景着色、模糊和高光；布局尺寸、按钮位置不随材质变化。原生窗口提供稳定底色，应用内容在玻璃后方滚动。验证通过后扩展到 AppBar 和底部弹层。
3. **单独验证液态折射。** 先比较 SVG 背景滤镜方案与普通模糊，验证实际像素效果，不能只依赖 `CSS.supports()` 返回 true。若必须追求更强的原生折射，再评估 AGSL/Backdrop 与 Tauri 的桥接成本。若引入页面捕获，锁定后需要及时释放含页面内容的捕获资源。
4. **完成真机验收后再扩大覆盖。** 至少覆盖项目最低支持版本、Android 12、Android 13+ 和当前目标设备；同时记录 WebView provider/版本。比较长列表滚动、前后台恢复、键盘开合、横竖屏、分屏、主题切换和省电状态。测试正常及大字号下的可读性与点击目标。

验收应包括：无闪黑、首帧位置稳定、滚动背景持续更新、文字图标不被模糊、弹层不被裁剪、底部操作不被系统手势区遮挡；同时比较关闭与开启效果时的帧耗时和内存。60 Hz 与 120 Hz 的每帧总预算分别约为 16.7 ms 与 8.3 ms，材质只能占其中一部分。桌面浏览器模拟移动尺寸不能替代这一步。

推荐决策：**局部 CSS 玻璃进入实施候选；液态折射进入实验候选；全窗口跨应用模糊暂不作为 Android 主界面方案。**
