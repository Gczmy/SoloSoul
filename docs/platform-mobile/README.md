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

### 移动端已知限制

- 批量下载到系统目录暂不支持（已有明确 UI 提示）
- 开发模式下 Vite HMR WebSocket 在 WebView 内连接失败，不影响 Release 功能
- 附件「用系统应用打开」在 Android 依赖 FileProvider，建议优先使用内置预览

---

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
