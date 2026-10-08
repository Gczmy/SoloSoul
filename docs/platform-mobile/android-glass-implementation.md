# Android 玻璃材质实施记录

## 2026-10-08 当前包预览与停绘补证

生产包 `474846…` 的 API34 两模式非空预览各 1/1，正文 / 标题、系统栏、通知占位及逐层返回通过。[预览范围与来源](../verification/fe2-current-preview-2026-10-08/README.md)。同一主包的新仪器进程采样生产首页 WebGL 提交，两模式各 1/1，静止 / 离开 / 返回 / 返回后静止为 0 / 0 / 4 / 0，最终文字数量仍可见。[绘制范围与来源](../verification/fe2-rendering-work-2026-10-08/README.md)。只是有界 JS 提交证据，不将其表述为 GPU / 功耗 / FPS 或实体性能验收。产品渲染器未变，历史设备测试保持原 APK 来源。

2026-09-15，接续 [已审核方案](android-glass-proposal.md)。

## 使用入口与范围

进入 **设置 → 主题与外观 → 玻璃材质**：

| 档位             | 表现                                                                |
| ---------------- | ------------------------------------------------------------------- |
| 关闭             | 保留 Material 3 配色、悬浮导航和固定几何，使用实色表面              |
| 局部玻璃（默认） | AppBar、悬浮底栏、React 操作弹层使用局部背景模糊                    |
| 增强玻璃         | 额外启用首页自绘液态徽记，以及设备支持时的 Android 原生玻璃快捷菜单 |

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

| 层级             | 结果                                                                                                                 |
| ---------------- | -------------------------------------------------------------------------------------------------------------------- |
| 前端             | TypeScript、ESLint、Prettier 定向检查通过；117 个测试文件、969 项 Vitest 通过；异步取消调整后 4 项请求测试复测通过   |
| 移动端浏览器     | 15 项 Android 材质与移动端冒烟通过；最终原生路由/迟到请求两项再验证通过                                              |
| Rust             | 材质入参边界 1 项、UI 偏好 3 项、命令分发 1 项通过；fmt 与 Clippy 通过                                               |
| 设置/权限一致性  | 211 个应用命令 ACL、22 个偏好 key 一致性通过                                                                         |
| Android 构建     | Kotlin 与完整 ARM64 Debug APK 构建通过，包含最终前端和原生插件                                                       |
| Android 原生测试 | Pixel_9 模拟器 API 37、WebView 145.0.7632.45、系统 blur 支持且开启；3 项 instrumentation 测试通过                    |
| 真实客户端桥接   | 安装 APK 后，在实际 WebView 验证 Rust 能力查询、插件事件注册与移除、原生菜单打开与按请求 ID 关闭；返回 `cancel` 正确 |

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

配置 Android SDK（API 36、Build Tools 35.0.0）、NDK 27.0.12077973 和 Rust ARM64 target，并通过 `JAVA_HOME` 选择 JDK 21。项目 `gradle.properties` 不指定机器上的 JDK；IDE 或个人 Gradle 设置可在本机覆盖。先在 `tauri/src-tauri/gen/android` 运行 `./gradlew --version`（Windows 为 `.\gradlew.bat --version`），确认 Launcher/Daemon JVM 为 JDK 21，不加 `-Dorg.gradle.java.home`。在 `tauri/` 运行 Debug 构建：

```bash
npx tauri android build --debug --target aarch64 --split-per-abi --apk --ci
cd src-tauri/gen/android
./gradlew :app:assembleArm64DebugAndroidTest -x :app:rustBuildArm64Debug
adb install -r app/build/outputs/apk/arm64/debug/app-arm64-debug.apk
adb install -r app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk
adb shell am instrument -w com.solosoul.app.test/androidx.test.runner.AndroidJUnitRunner
```

原生回退测试用 `UiAutomation.adoptShellPermissionIdentity` 临时授予测试进程写系统设置能力，结束后恢复并撤销；不依赖本机 API 37 模拟器存在权限异常的 `wm disable-blur` 写命令。Debug 测试 Activity 不导出，也不会进入 Release APK。

## 2026-10-07：共用桌面主题的前端重构

本节是后续实施记录，不改写上方历史验收设备或数量。设计见[移动端统一方案](MOBILE_VISUAL_ALIGNMENT_PROPOSAL_2026-10-07.md)，任务状态见[第二轮前端报告](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md)。

- Android 删除独立的绿灰背景和色板，不再用 `applyAndroidMaterial` 覆盖共享背景 / 文字 / 强调色。`--md-*` 保留为共享主题的派生兼容名，由已应用 DOM 主题数值计算。
- WebGL 用主题快照订阅更新，原生新建菜单在请求时读取同一已交付主题；都使用不透明 hex，支持同模式更换底色与自定义强调色。原生取消、失效请求、后台 / 离屏 / 静止绘制和锁定清理的生命周期不改。
- 首页概览和分类卡片使用中性正文表面，取消按索引染色。增强装饰只在右侧 42% 内；名称、数量与摘要有独立区域。普通按钮采用 primary / neutral / quiet，导航填色 90%、短弹层 92%；新建 / 编辑的长表单显式实色，平板导航实色。
- 登录页提供 90% 深底板 / 80% 浅卡片的局部层次，文字和控件不透明；不支持滤镜、关闭玻璃、减少透明度或强制色时沿用实色回退。原软键盘滚动和安全区保留。
- 外观 UI 分开模式、浅深底色方案、强调色与材质，5 个预设及自定义色均复用原账户偏好。未重置已存模式、材质、语言或动效设置；保存失败 / 迟到请求 / 账户切换有独立交互回归。

当前检查：79 项移动端浏览器回归、26 项生产包回归通过；完整前端 Vitest 2317 项通过。最终 ARM64 Debug 包在专用 API34 模拟器 `SoloSoul_RF201` 验证模糊支持 11 项、关闭模糊回退 9 项，实际 MainActivity WebView 的底色 / 文字 / 海蓝强调色与共享方案一致。原生 Card 的真实像素检查不覆盖完整首页 WebGL，原生短菜单用例也不替代全页面合成；增强首页实际 WebView 与实体设备性能、iOS 原生能力仍待补证。详见[设备与产物检查点](../verification/fe2-frontend-checkpoint-2026-10-07.json)。

独立生产 Card 夹具现在可通过 `npx tsc --project native-regression/card-surfaces/tsconfig.json` 验证类型；其白名单只增加无副作用的共享色板与对比工具，不引入账户、日志或正式插件初始化。Debug 构建资源暂存后应原样恢复本轮开始前已有的生成资源修改，不将其自动计为重构成果。

### 同日原生渲染器补证与恢复修复

最终 API34 专用模拟器支持 12/12、回退 10/10 通过，新增生产 WebGL 渲染器和概览 CSS 的独立 Debug 夹具。实际 GPU 输入、正文表面像素、静止停绘和真实上下文丢失 / 恢复通过；两组强调色是测试极值，不是新增默认色。修正生产恢复时删除失效 GPU 句柄的问题，并补全 Gradle 夹具源依赖，避免旧 HTML 进入复验。原生截图先等待当前 DOM 的 8 个真实帧，不将 GL draw 回调等同于屏幕已呈现。入口见[原生回归说明](android-native-regression.md#fe2-原生玻璃绘制入口2026-10-07)，细节见[执行报告](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md)。完整首页 / 账户合成与实体设备性能仍待验证，历史设备与 11/9 计数保持原记录。

### 2026-10-07 保存底色的原生返回兼容修复

iOS 实际生产 WKWebView 的非默认底色检查发现 `ui_get_preferences` 遗漏底色方案返回字段。Rust `UiPreferences` 补齐可选 `defaultLightTheme` / `defaultDarkTheme`；旧配置缺失时不输出默认字段，保留前端缓存兼容。修复后 iOS 专用模拟器 11 组无账户主题 / 像素检查通过，Android 当前 Debug 包与新原生测试包支持 13/13、回退 11/11 通过；新增真实 MainActivity 非默认方案与自定义强调色冷启动检查。旧测试未显式提供默认底色造成跨用例缓存干扰，已修正输入和完整色板就绪判断，未放宽最终颜色断言。该补证不替代完整首页账户合成或真实设备性能；完整失败、恢复和来源见[FE2 检查点](../verification/fe2-frontend-checkpoint-2026-10-07.json)。

### 同日真实空账户首页补证

新增独立原生首页方法，在生产 MainActivity 经引导、新建公开临时账户、外观设置和锁定 / 主密码解锁进入实际 AndroidHome。支持 / 系统 blur 关闭各 1/1，通过后的 4 张合成帧显示浅深色文字、数量和增强装饰正常，真实账户偏好 dark / enhanced 保存。截图等待 WebView 视觉提交，旧设置页截图不接受；两组都发生第一次解锁维护忙、第二次 UI 重试成功，该暂态体验未被宣称修复。专用模拟器全部 242 项私有文件内容、链接、权限及系统 blur 已恢复，私有备份不入库。新增方法不改变通用 13/11 计数；有数据的对象、账户切换、iOS 原生材质与实体设备性能仍待验。详见[执行报告](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md#2026-10-07-实际-android-首页与-macos-原生复验)。

### 同日有数据的对象与系统返回补证

生产 UI 新建 Travel / Visa 公开对象后，当前包在 API34 专用模拟器模糊支持 / 关闭两组各 1/1 通过，各 12 个阶段、11 张最终帧。对象列表 / 详情使用共享暖石实色，操作菜单为局部 CSS 玻璃，增强首页浅 / 深数量为 1 且文字和装饰可见。系统返回现在由 MainActivity 交给 Router 历史，实测详情与菜单分别关闭后保留原分类；没有改写认证或 Vault 生命周期。通知权限弹窗按真实原生界面处理，完整私有目录、根模式、系统 blur 和本应用通知权限 / 标记均恢复。

截图复核仍发现备份 Toast 遮挡底部菜单和编辑保存区，完整视觉验收保持待处理；账户切换、根返回 / 键盘、iOS 材质与实体设备性能不由这两组结果代替。历史通用 13/11 与空账户首页方法保留原来源，本次与 5 次失败另列。[执行记录](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md#2026-10-07-有数据的-android-对象与系统返回补证)、[当前检查点](../verification/fe2-frontend-checkpoint-2026-10-07.json)。

## 2026-10-07 移动端通知与局部玻璃浮层

备份等应用内 Toast 改用当前页面 / 登录壳或最高层级弹层的正常流通知槽。Android 短操作菜单仍保持局部玻璃，通知独立实色承托；对象字段、编辑和长表单继续使用稳定底色。全局通知数据、计时和操作回调保持单一来源，关闭菜单交还页面，桌面通知位置不变。320px 菜单和放大字体编辑的浏览器检查已通过；新 APK 原生复验尚未取得提醒存在的接受帧，详见本轮执行报告，不使用旧 APK 结果替代。

### 2026-10-07 真实提醒的原生验收

此前“无提醒帧”仅为失败历史；当前生产 APK 已在 API34 ARM64 专用模拟器的系统 blur 支持 / 关闭两组各 **1/1**、退出 0，取得提醒存在的编辑页与对象操作菜单合成帧。Save 与 Edit / History / Attachments / Delete 可见、可命中且未与提醒重叠。只读观察确认延迟提醒在编辑页出现；等待真实提醒后采样，没有合成注入、关闭或改写计时。各 12 阶段 / 11 PNG，242 项数据、权限、blur 和通知授权恢复。详见[原生补证](android-native-regression.md#2026-10-07-备份-toast-存在期间的原生补证)。其他浮层 / 键盘 / 账户切换、维护忙体验、iOS 能力与实体设备性能保持待验。

### 2026-10-07 嵌套浮层与预览通知

附件 / 历史滚动区增加正常流通知；文件、相册、查看器在顶栏下占位，通知高度限制为 35dvh / 240px 并可滚动。相册进入查看器与懒加载时移交宿主，打开附件属性对话框时交给更高层，退出后恢复。通知不重发、不延长计时，正文和缩放控件保持可用；桌面继续原固定位置。

修复前附件通知命中失败、两种预览缺槽；当前相关单测 124/124、全量 2321/2321，mobile / chromium 浏览器各 35/35，全部最终退出 0。独立服务器重跑前的收尾中断 / 连接拒绝历史保留。新主 APK 两组原生基线通过并完整恢复，但只覆盖原编辑 / 对象菜单路径，不是新增附件 / 历史 / 预览通知的原生证明。详见[本轮执行日志](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md#2026-10-07-附件--历史--预览通知补齐)。

### 2026-10-07 附件 / 历史真实浮层补证

在同一生产主包上新增显式 overlays 原生场景：真实备份提醒在附件和历史面板内正常流占位，属于当前 5100 层，通知及上传 / 关闭按钮可命中且无重叠。实际 Android 返回依次关闭内层、保留对象详情，再返回原 Travel 分类。专用 API34 ARM64 系统 blur 支持 / 设置关闭两组各 1/1、退出 0，各 15 阶段 / 13 PNG；242 项私有数据、权限、blur 与通知授权恢复，截图已目测。没有延长提醒或替换生产 UI / IPC。当前附件为空列表，文件、相册、查看器和上传仍待原生扩展；维护忙、键盘、账户切换及实体设备性能保持待验。[原生范围与命令](android-native-regression.md#2026-10-07-附件与历史嵌套通知场景)。

### 2026-10-07 原生软键盘避让

实际 Gboard 打开后，edge-to-edge WebView 与网页 visualViewport 未缩小，输入框落入键盘区。MainActivity 现在根据未消费的 IME insets 和父容器实际覆盖量调整 WebView 高度，键盘关闭后恢复；没有通过整页 opacity、额外遮罩或玻璃开关掩盖问题。多窗口切换的系统边界适配参数同时纠正，但真实分屏效果仍待验。

最终 Debug 包在专用 API34 ARM64 系统 blur 支持 / 设置关闭两组各 1/1、15 阶段 / 14 PNG，通过真实触屏、原生键盘高度、输入 / Save 四边命中、真实提醒同时存在、实际键盘录入与系统返回草稿保留检查。WebView 从 1920px 缩至 1048px，关闭键盘后恢复；同一暖石深色内容与玻璃偏好保持。全部测试私有数据和权限 / blur / 通知授权恢复，失败历史与原 APK 来源另存。仅证明这台模拟器的全屏编辑流程，登录键盘、旧系统、真实多窗口、预览与账户切换、iOS 和实体性能继续待验。[完整记录](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md#2026-10-07-android-真实软键盘避让修复与验收)、[原生入口](android-native-regression.md#2026-10-07-真实-ime提醒与键盘返回)。

### 2026-10-07 账户切换与极端强调色可读性

Android 主题继续来自同一账户的共享方案。无填色操作与工具图标使用独立 `--md-primary-ink`，在实际页面 / 卡片表面上至少 4.5:1；强调色过暗或过亮时只调整文字明度，保持用户填色、画布、原生菜单与保存值。首版原生账户截图发现深强调色几乎不可见、黄强调色与浅底混合，旧通过仅代表保存值隔离，不算完整视觉通过。

当前修复包的真实两账户 A → B → A，系统 blur 支持 / 关闭各 1/1、18 阶段 / 16 PNG、退出 0；保存偏好与数量隔离、实际文字对比 5.26 / 4.53、像素和命中均通过。16 项浏览器包含工具图标可读性；工具图标未在本次原生方法中逐项采样。完整 Vitest 2321、类型 / 项目 Lint通过，全部私有目录与系统设置恢复。维护忙重试、状态栏安全区、完整 macOS / iOS 原生与实体性能继续保留。[验收与边界](android-native-regression.md#2026-10-07-两账户外观隔离与无填色操作对比)。

### 2026-10-08 玻璃外壳的系统安全区

导航与页面背景仍铺到窗口边缘，控件改为消费共享安全区长度。Android 按原生窗口 / WebView 实际覆盖量交付 systemBars / cutout，CSS 与浏览器 env 取最大值，避免系统已避让后重复留白。新文档通过受能力检测的文档开始脚本交付；旧 WebView 加载完成回退未在本轮设备上验证。矮屏登录收紧留白，聚焦和键盘改变视口后将解锁区域整体带入视口，不改文字、图标和触控尺寸。

当前同一 Debug 包原生支持 / 系统 blur 关闭各 1/1、19 阶段 / 18 PNG、退出 0；首页旋转、登录真实键盘与返回、实际文档重载均避开系统栏 / IME，数据及设置完整恢复。前端 2321 单测、28 核心 mobile、33 Chromium 与 16 mobile 预览检查通过；三种照片预览的安全区消费是浏览器证明，原生非空预览仍待验。状态栏重叠和按钮细小裁剪的失败保留，维护忙与其他平台 / 实体性能不宣称完成。[完整证据与范围](../verification/fe2-safe-area-2026-10-07/README.md)。


### FE2 账户外观冷启动持久化（2026-10-08）

重启错误选择占位 Vault 根、同一 UI 配置文件的路径别名被误删、切换账户后启动缓存未更新已修复。API34 ARM64 系统模糊支持 / 关闭两组新包，实际两个进程方法各 1/1，均完成锁定登录外观与 A 深色增强 / B 浅色局部玻璃恢复；正文及操作文字保持可读。完整前端 2322 项与后端偏好 24 项通过。原生证据仅覆盖等待保存完成后的稳定帧，实体性能和启动连续帧另验，详见[公开冷启动报告](../verification/fe2-cold-2026-10-08/README.md)。

### FE2 减少动态效果持久化（2026-10-08）

登录前镜像现按 Rust 字符串契约编码 reduceMotion，账户偏好和缓存仍保留布尔。真实 UI 双账户 / 新进程测试在系统 blur 支持和关闭两组通过，减少动效时增强玻璃与文字仍可见。只证明持久化、样式交付与稳定帧，未宣称完整动画 / 真机性能；[原始失败与最终证据](../verification/fe2-motion-2026-10-08/README.md)。
## 2026-10-08 非空附件与照片预览补证

FE2-014 在专用 API34 ARM64 模拟器补真实附件 / 文本 / 图片 / 相册 / 查看器的深浅主题验收，系统模糊开启和关闭各 1/1、29 阶段 / 25 PNG、退出 0。两份公开附件经生产 IPC 加密保存；原图最终像素、文本实际文字、通知在前景正常流占位、操作安全区和逐层返回通过。每组原数据与系统设置完整恢复。本阶段只改测试采样与判定，生产材质、图片组件、通知期限及主 APK 均未改。

来源、七次失败和边界见[预览补证](../verification/fe2-preview-2026-10-08/README.md)。这不是系统文件选择器、多图翻页、PDF、连续动画或真机性能验收；macOS 和 iOS 原生剩余范围独立保留。

## 2026-10-08 旧 WebView 与预览顶栏主题修复

专用 API31 ARM64 的实际 WebView 91.0.4472.114 启动失败，确认缺少 Object.hasOwn；编译产物的范围媒体查询、color-mix / dvh 也不受支持。入口增加能力检测兼容层，UUID 回退仍使用系统安全随机源；构建目标保留传统媒体查询，视口使用 100vh / 支持时的 100dvh，玻璃色从当前共享快照交付 RGB 通道，alpha 继续由表面样式与辅助功能控制。首页文字 / 数量、编辑操作、对象 / 附件 / 照片返回在真实旧引擎完成验证，不宣称所有业务功能都已覆盖。

Android 文件 / 相册预览按应用当前主题设置系统栏；浅色顶栏与底部安全区用共享内容色。照片查看器默认白色标题在浅色顶栏上不可读，已统一按钮、标题、计数的前景色，照片区域与缩放操作仍保留深色。验收新增真实 Window flags、所有顶栏前景对比和最终截图文字 / 图标像素，不能只凭保存值或系统图标模式判断视觉正确。此前 API34 非空预览通过记录仍仅证明当时内容范围，不自动提升为新增顶栏判据通过。

逐批来源、失败及恢复记录见[旧 WebView 与预览主题证据](../verification/fe2-api31-2026-10-08/README.md)。系统窗口模糊关闭时仍保留统一配色及层级；此阶段不是实体性能、iOS 全账户、连续动画或整个旧引擎全产品兼容证明。

## 2026-10-08 API31 横屏通知安全区

旧 WebView91 的导航、登录 / IME、旋转与重载已在系统模糊开启 / 关闭两组验证；实际横屏右侧系统栏 126px，通知槽补横向安全区。规则限定已登录 Android 壳，认证壳与平板轨道已占用的安全区不再次叠加。原生 insets 现强制实际限时备份提醒及操作 / 关闭按钮安全、可点击；没有延长生产提醒。

最终两组各 1/1、19 阶段 / 18 PNG、0 跳过、退出 0，数据 / 权限 / blur 完整恢复。浏览器定向复验 4/4，Python 拒绝检查 26/26；历史范围缺少通知证据不能升级为新范围通过。新增 `verify_evidence(..., require_safe_notifications=True)` 仅用于要求新范围，当前设备驱动对 insets 强制开启；旧报告的原结论保留。浮点相邻边界误判与浏览器夹具失败、最终包 / 源码来源均见 [证据目录](../verification/fe2-api31-insets-2026-10-08/README.md)。不代表旧引擎全产品、实体性能、iOS 或 macOS 原生已完成。
