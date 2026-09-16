# Android 玻璃材质实施记录

2026-09-15，接续 [已审核方案](android-glass-proposal.md)。

## 使用入口与范围

进入 **设置 → 主题与外观 → 玻璃材质**：

| 档位 | 表现 |
| --- | --- |
| 关闭 | 保留 Material 3 配色、悬浮导航和固定几何，使用实色表面 |
| 局部玻璃（默认） | AppBar、悬浮底栏、React 操作弹层使用局部背景模糊 |
| 增强玻璃 | 额外启用首页自绘液态徽记，以及设备支持时的 Android 原生玻璃快捷菜单 |

内容正文、对象字段与输入框沿用原有清晰表面；主 Activity 保留不透明背景。平板继续使用 88px 实色导航轨道。Windows/macOS 不应用这些 Android 材质样式。

手机底栏左右各留 12px，底部安全区之外再留 6px，本体高 80px、圆角 27px；FAB 同步上移，图标和标签的位置不随材质或选中态缩放。首页欢迎语移到概览卡片上方。

## 实现

- `styles/android.css` 与 `useAndroidGlass` 统一局部材质：18px 背景模糊，弹层 24px，浅/深底色透明度 0.76/0.80。直接过滤元素背景，前景文字保持清晰，滚动弹层没有可滚走的材质伪元素。
- `androidGlass` 偏好写入现有四份设置链路：Zustand、启动缓存、明文 UI 偏好与账户偏好。旧配置缺少该项时默认 `local`，锁定后保留外观偏好。
- `AndroidGlassPlugin.kt` 持有独立原生 Dialog，只展示对象、页面、扫描三项动作。颜色、文案和一次性请求 ID 经过 Rust 校验；固定 action 返回 React 后复用原有路由和页面表单。
- 原生窗口在附着前配置色底、圆角和模糊；背景模糊 24dp、behind 4dp、圆角 30dp，初始色底 0.76/0.80。无窗口入场动画，避免首帧重复合成。
- 通过 `isCrossWindowBlurEnabled` 查询系统能力，并注册能力变化回调。已打开窗口在系统关闭 blur 时原位变为不透明、增大遮罩；下次打开使用 React 菜单回退。
- 注册 `android-glass` 内联插件 ACL，仅 Android 主窗口可注册/移除材质能力监听。菜单打开和关闭仍经三个受校验的应用命令，不直接开放 Kotlin 菜单命令。
- `androidLiquidRenderer` 使用 WebGL 圆角距离场、偏移取样、少量色散与边缘高光。背景只来自自身生成的渐变与柔和光带，不读取 DOM 截图或对象字段。文字与数量是独立 React 前景。

### 2026-09-16 前端细化

- 工具卡片统一使用中性底色、正文色和主题色图标；移除按第 1、3 项位置单独着色的规则，搜索与模板管理不再被误认为选中态。
- 首页移除密集网格和文字后的实色色块，左侧以平缓主题色保证可读性，右侧用光带折射和薄边高光呈现玻璃厚度。深色模式同步减弱高光。
- 玻璃徽记与盾牌保持同一中心，触摸仅改变光线与折射；窄屏收敛装饰宽度。WebGL 不可用或上下文丢失时，使用同位置的 CSS 渐变玻璃装饰。
- 此次前端验证覆盖 Chrome 移动视口的深浅主题、320/390/1024px 布局、WebGL 上下文恢复与离屏停绘；保留现有原生窗口实现，尚未在实体 Android 设备复测此次视觉调整。

### 跟随系统主题时的首页文字可读性修复

- Android 17 模拟器的实际 WebView 中，已解析的 `data-theme` 为 `dark`，但 `matchMedia('(prefers-color-scheme: dark)')` 返回 `false`。旧玻璃组件独立使用该媒体查询，导致着色器使用浅色底、正文使用深色主题的浅色字，文字与数量几乎不可辨认。
- 玻璃改为订阅应用已经解析的 `data-theme`，与正文共用深浅主题来源；原生主题事件、重新进入首页及 WebGL 恢复后同步更新。移除文字底板后暴露出的配色问题由此修复，无需重新增加实色底板。
- 新增回归模拟原生主题与媒体查询相反，直接校验 WebGL 配色与正文 CSS token 一致，并覆盖挂载期间切换和重新进入首页。修复前复现失败，修复后通过；类型检查、定向 ESLint 和另外两项玻璃回归通过。模拟器在修复后验证前自动锁定，本轮未重新打包安装 APK。

## 生命周期与回退

- 原生菜单拥有自身返回键；页面表单切回现有 React 浮层返回栈。请求完成只分发一次；路由改变、锁定、账户切换、进入后台后，迟到的动作失效。
- `onPause/onStop/onDestroy` 关闭原生窗口；销毁注销系统监听。关闭请求即使先于打开抵达，也阻止旧菜单再次出现。不暂停真正的后台自动锁定。
- CSS 背景模糊不可用时采用实色；增强菜单原生桥接失败时回退 React 操作菜单，保留全部三个入口。
- 用户或系统减少动效时保留静态玻璃，关闭指针随动。强制颜色或浏览器支持的减少透明度偏好使玻璃失效，不修改用户保存的档位。
- WebGL 静止时不持续申请动画帧；离屏/后台停止绘制，重新可见按需绘制。DPR 上限 1.5、画布宽度上限 900 像素。上下文丢失采用静态装饰，恢复后重建资源；锁定或离开首页释放资源。

## 验证与图像

| 层级 | 结果 |
| --- | --- |
| 前端 | TypeScript、ESLint、Prettier 定向检查通过；117 个测试文件、969 项 Vitest 通过；异步取消调整后 4 项请求测试复测通过 |
| 移动端浏览器 | 15 项 Android 材质与移动端冒烟通过；最终原生路由/迟到请求两项再验证通过 |
| Rust | 材质入参边界 1 项、UI 偏好 3 项、命令分发 1 项通过；fmt 与 Clippy 通过 |
| 设置/权限一致性 | 211 个应用命令 ACL、22 个偏好 key 一致性通过 |
| Android 构建 | Kotlin 与完整 ARM64 Debug APK 构建通过，包含最终前端和原生插件 |
| Android 原生测试 | Pixel_9 模拟器 API 37、WebView 145.0.7632.45、系统 blur 支持且开启；3 项 instrumentation 测试通过 |
| 真实客户端桥接 | 安装 APK 后，在实际 WebView 验证 Rust 能力查询、插件事件注册与移除、原生菜单打开与按请求 ID 关闭；返回 `cancel` 正确 |

原生测试采用仅存在于 Debug 的 `GlassRegressionActivity`，使用虚构 HTML，不启动 Vault。覆盖动作只返回一次、原生返回键关闭菜单且 Activity 保持、进入后台取消、取消先于打开到达、系统关闭模糊时原位提高色底。系统设置在测试后恢复原值。

- [实际 React 首页](android-glass-implemented-home.png)：Chrome 手机视口渲染，虚构测试账户。
- [原生菜单：模糊开启](android-glass-native-enabled.png)、[原生菜单：模糊关闭](android-glass-native-disabled.png)：Android 模拟器屏幕截图，底页为虚构内容。
- [原设计交互 HTML](android-glass-review.html) 保留，作为视觉设计参考，不作为原生 API 实机证据。

已验证的设备是模拟器，尚未完成 Android 9–16 实体设备、旧 WebView、中低端 GPU、高刷新屏及耗电矩阵。默认仍为局部玻璃；不将浏览器/模拟器结果视为所有设备流畅度或平台合成闪烁的保证。

## 复测命令

在 `tauri/` 执行常规前端检查与回归：

```bash
npx tsc --noEmit
npm run lint
NODE_OPTIONS=--no-experimental-webstorage npm run test
npm run check:acl
npm run check:pref-keys
SOLOSOUL_E2E_CHANNEL=chrome npx playwright test e2e/android-material.spec.ts e2e/mobile-smoke.spec.ts --project=mobile --workers=1
```

本机 macOS 的直接 Cargo 检查使用临时配置规避 Tauri 构建脚本对目标专属 feature 的误判；不改变产品的 macOS private API 配置：

```bash
TAURI_CONFIG='{"app":{"macOSPrivateApi":false}}' cargo test -p solo_soul android_glass_plugin --lib
TAURI_CONFIG='{"app":{"macOSPrivateApi":false}}' cargo test -p solo_soul test_ui_preferences --lib
TAURI_CONFIG='{"app":{"macOSPrivateApi":false}}' cargo test -p solo_soul test_dispatch_cluster_prefixes_consistent --lib
TAURI_CONFIG='{"app":{"macOSPrivateApi":false}}' cargo clippy -p solo_soul --lib -- -D warnings
cargo fmt --check
```

配置项目规定的 Android SDK/NDK 与 rustup PATH 后：

```bash
cargo tauri android build --debug --target aarch64 --split-per-abi --apk --ci
cd src-tauri/gen/android
./gradlew :app:assembleArm64DebugAndroidTest -x :app:rustBuildArm64Debug
adb install -r app/build/outputs/apk/arm64/debug/app-arm64-debug.apk
adb install -r app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk
adb shell am instrument -w com.solosoul.app.test/androidx.test.runner.AndroidJUnitRunner
```

原生回退测试用 `UiAutomation.adoptShellPermissionIdentity` 临时授予测试进程写系统设置能力，结束后恢复并撤销；不依赖本机 API 37 模拟器存在权限异常的 `wm disable-blur` 写命令。Debug 测试 Activity 不导出，也不会进入 Release APK。
