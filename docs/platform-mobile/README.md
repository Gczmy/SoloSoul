# SoloSoul 移动端（Mobile）文档

> 当前版本：v2.8.6 · 更新日期：2026-08-07
>
> 本目录是移动端（Android / iOS）的唯一文档入口，涵盖平台现状、开发环境与构建发布。

---

## 目录结构

| 文件 | 说明 |
|------|------|
| `README.md` | 本文档——移动端当前状态总览与功能实现清单 |
| `android-environment-setup.md` | Android 开发环境搭建（macOS + rustup + Android Studio） |

> 历史文档（Android 移植调研、P0–P5 开发计划、7 月审查报告、7 阶段实施计划、OCR spike、移植进度记录）已完成使命并删除，需要追溯时见 git 历史。

---

## 平台现状

### Android — ✅ 已发布

- **构建与签名**：Release APK 签名发布（keystore 见 AGENTS.md），产物路径 `tauri/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`
- **minSdk = 28**（Android 9.0+），targetSdk 随 Tauri v2 基线
- **CI**：`.github/workflows/build-android.yml` 在 `tauri/**` 或 `docs/platform-mobile/**` 变更时构建 debug APK；main 分支额外构建签名 release AAB（keystore 经 secrets 注入）
- **iOS 工程**：`tauri/src-tauri/gen/apple/` 已初始化（含 `solo_soul.xcodeproj`、Podfile、Assets），但 **无发布计划**（P5-02/03/04 暂缓，无 Apple 开发者账号签名流程）

### 功能实现状态（对照代码核实）

| 功能 | 状态 | 实现位置 |
|------|------|----------|
| 设备同步（NSD 发现 + Noise 握手 + SAF 目录） | ✅ 已实现 | `src-tauri/src/commands/discovery.rs`、`sync.rs`、`vault_directory.rs`；原生侧 `SafSyncHelper.kt` |
| 生物识别解锁 | ✅ 已实现 | 原生侧 `BiometricKeystorePlugin.kt`（Android Keystore + BiometricPrompt） |
| OCR（ML Kit 中文识别） | ✅ 已实现 | `src-tauri/src/mobile_ocr_plugin.rs` + 原生 `MobileOcrPlugin.kt`；`ocr_scan_image` 移动端路由到插件 |
| 附件 content:// 中转 | ✅ 已实现 | `src/lib/mobileFileTransfer.ts`，单附件上传/下载/导入/导出经应用缓存中转 |
| 自动更新（APK 下载/校验/安装） | ✅ 已实现 | `src-tauri/src/commands/update.rs`（仅 Android，桌面走 `plugin-updater`） |
| 通知权限 | ✅ 已申请 | `POST_NOTIFICATIONS`（Android 13+），`tauri-plugin-notification` |
| 帮助文档资源复制 | ✅ 已实现 | `MainActivity.kt` 将 APK assets 复制到 dataDir |
| 状态栏/导航栏主题适配 | ✅ 已实现 | `themes.xml` + `values-night/themes.xml`（暖石浅色 `#FAFAF8` / 深色 `#1F1C18`） |
| 虚拟键盘遮挡 | ⚠️ 基本可用 | `windowSoftInputMode="adjustResize"`，真机冒烟待补 |
| 批量下载到目录 | ❌ 不支持 | 移动端明确提示（系统目录访问受限） |

### 应用内更新平台边界（RF-202，2026-09-27）

APK 缓存、版本检查、下载和安装四个入口仅供 Android 使用；iOS 不调用 Android 或桌面更新命令，关于页显示不支持应用内更新并提示使用原安装分发渠道。此状态不表示网络失败或已是最新版，也不显示更新横幅和安装按钮。实现与验证边界见[构建发布与自动更新规范 §5.1](../design_map/22_构建发布_签名_自动更新.md#51-平台入口rf-2022026-09-27)。

### 前端统一与 iOS 原生补证（FE2，2026-10-08）

移动端共用桌面主题的底色、强调色和文字层级；Android 保留局部玻璃与原生短操作菜单，正文使用稳定表面。完整实施和未验范围以[本轮执行报告](../REFACTOR_FRONTEND_EXECUTION_REPORT_2026-10-07.md)为准，不以本页早期平台基线替代当前原生证据。

iOS 的中文字体回退同时覆盖正文与表单控件：仅在 iOS 作用域优先 PingFang SC，控件规则采用零特异性，组件显式字体继续优先。生产 WKWebView 曾出现“正文正常、按钮中文缺字”的差异，不能只检查可访问名称或浏览器截图。当前无签名模拟器包已通过真实创建、锁定、首次解锁、冷启动首次解锁和账户外观设置，实际导航与按钮字形已复核。

专用 iPhone17 / iOS26.3.1 实际系统浅→深事件、系统深色下固定浅色和固定深色的共享色板及状态栏时钟前景已补证。iOS 状态栏 IPC 仍为无操作；当前所采帧实际配色正确，没有据此覆盖整个 UIKit 窗口外观或更换根控制器。此结果不证明旧 iOS、原生预览、Liquid Glass、完整生命周期或真机性能，也不代表已有签名发布能力。[包、源码、失败及原生截图](../verification/fe2-ios-control-fonts-2026-10-08/README.md)。

后续修复iOS编辑聚焦导致整窗上移：采用实际可视视口高度/偏移、固定顶栏与键盘避让，并用公开原生scrollView API避免重复inset。最新98280597包真实姓名输入、保存、详情关闭留原页、展开导航与键盘下添加页面取消原生1通过；顶部栏y=0、操作位于键盘上方，旧失败完整保留。Android最新9c6395包正文Save/Cancel和真实IME、Back草稿/历史及详情返回，在系统模糊支持/关闭两组各原生1通过、16阶段/15PNG，两组242项数据/权限恢复独立核对。只接受这些实际范围，不将旧包字体、API31、预览或生命周期矩阵升级为新包全量验收。[iOS视口证据](../verification/fe2-ios-root-viewport-2026-10-08/README.md)、[Android当前编辑证据](../verification/fe2-current-editor-android-2026-10-08/README.md)。

### 移动端已知限制

- 批量下载到系统目录暂不支持（已有明确 UI 提示）
- 开发模式下 Vite HMR WebSocket 在 WebView 内连接失败，不影响 Release 功能
- 附件「用系统应用打开」在 Android 依赖 FileProvider，建议优先使用内置预览

---

iOS真实系统选择器补验发现Library/Caches中转被原生附件源路径白名单拒绝，现仅iOS改用应用tempDir，未扩大后端权限。新无签名Simulator包2e4006a7…在iPhone17/iOS26.3.1实际TXT/PNG导入、浅深色文本/照片集/查看器及逐层返回原工作区，原生1通过、0失败、0跳过；45组截图/层级归档，四个稳定预览帧内容、工具栏和原生时钟检查通过。定向10项单测、类型和ESLint通过，106份构建/执行来源一致、9组平台输入恢复，99项数据记录（含根目录）实际恢复核对一致。旧包证据保留历史来源；其他iOS版本/PDF/系统分享/完整生命周期与实体性能继续待验。[当前iOS文件选择器与预览证据](../verification/fe2-ios-native-file-picker-2026-10-08/README.md)。

同一2e4006a7…生产包补验两张公开图片：浅深主题下真实系统选择器导入、横向触控翻页、上一个/下一个、放大/缩小/适应窗口和逐层返回，原生1通过/0失败/0跳过，59组PNG/层级保留。12个稳定帧实际像素、计数、按钮几何和图像缩放核对通过，当前106份来源一致，99项数据记录实际恢复核对一致。只接受这些动作与稳定帧，不代表连续动画或真机性能。[多图原生证据](../verification/fe2-ios-multiphoto-2026-10-08/README.md)。

iOS当前PDF附件可经真实系统选择器导入，但打开链路没有iOS原生桥接，实际系统opener失败。当前2e4006a7包已补浅深主题错误提示的真实文字/安全区/返回验收，串行原生与源码保护通过，数据恢复完整；这不是PDF打开或渲染成功。首次与桌面构建重叠的源码保护拒绝保留原文，后续必须串行保护共享构建输入。[明确能力边界与反馈证据](../verification/fe2-ios-pdf-feedback-2026-10-08/README.md)。

## 关键构建命令

```bash
cd tauri

# 开发模式（真机/模拟器）
npm run tauri:android:dev

# Debug APK（免签名，适合功能测试）
npm run tauri:android:build

# Release APK（需签名环境变量，见 AGENTS.md）
ANDROID_HOME=$HOME/Library/Android/sdk \
ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/30.0.14904198 \
cargo tauri android build
```

> ⚠️ Rust 工具链注意：本地 Homebrew Rust 不支持 Android 交叉编译，构建前需将 rustup 工具链置于 PATH 优先（详见 `android-environment-setup.md` §2）。
