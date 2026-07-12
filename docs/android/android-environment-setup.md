# SoloSoul Android 开发环境搭建指南

本指南说明如何在 macOS 上搭建 SoloSoul Android 客户端的开发、构建与调试环境。

---

## 1. 前置要求

- macOS 12+（Apple Silicon 或 Intel）
- Node.js 22+（与桌面端一致）
- Rust 工具链（**强烈建议使用 rustup**，而非 Homebrew Rust）
- Android Studio 2023.1+（含 SDK、NDK、模拟器）

---

## 2. 安装 Rust 与 Android target

### 2.1 切换到 rustup stable（推荐）

当前项目环境检测到 Homebrew Rust（sysroot 在 `/opt/homebrew/Cellar/rust/1.94.1`），其未携带 Android std，会导致 `cargo check --target aarch64-linux-android` 失败。建议：

```bash
# 安装 rustup（如未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 使用 stable 工具链
rustup default stable

# 安装 Android target
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android
```

### 2.2 验证

```bash
rustc --print sysroot
# 应输出类似 /Users/zzc/.rustup/toolchains/stable-aarch64-apple-darwin

rustup target list --installed
# 应包含 aarch64-linux-android
```

---

## 3. 安装 Android Studio

1. 下载并安装 [Android Studio](https://developer.android.com/studio)
2. 打开 Android Studio → SDK Manager：
   - **SDK Platforms**：勾选 Android 14 (API 34) 或 Android 13 (API 33)
   - **SDK Tools**：
     - Android SDK Build-Tools
     - Android SDK Platform-Tools
     - Android SDK Command-line Tools
     - NDK (Side by side) — 建议 r26b 或 r27
     - Android Emulator（如使用模拟器）
3. 记录 NDK 实际路径，例如：
   ```
   /Users/zzc/Library/Android/sdk/ndk/26.2.11394342
   ```

---

## 4. 配置环境变量

在 `~/.zshrc` 或 `~/.bash_profile` 中添加：

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/26.2.11394342"  # 替换为你的 NDK 版本目录
export PATH="$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
```

重新加载：

```bash
source ~/.zshrc
```

验证：

```bash
echo $ANDROID_HOME
echo $NDK_HOME
adb --version
```

---

## 5. 安装项目依赖

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri

# 前端依赖
npm install

# Rust workspace 依赖（桌面端检查）
cargo check --package solo_soul
```

---

## 6. 初始化 Tauri Android 工程

首次运行 Android 开发服务器时，Tauri CLI 会自动初始化 Android 工程：

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri
npm run tauri:android:dev
```

按提示完成初始化，会生成 `tauri/src-tauri/gen/android/` 目录。

### 6.1 配置 AndroidManifest.xml

打开 `tauri/src-tauri/gen/android/app/src/main/AndroidManifest.xml`，在 `<application>` 标签内添加：

```xml
<activity
    android:name=".MainActivity"
    android:configChanges="orientation|screenSize|smallestScreenSize|screenLayout"
    android:launchMode="singleTask"
    android:windowSoftInputMode="adjustResize"
    ... >
```

在 `<manifest>` 内按需申请权限：

```xml
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
<uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />
<uses-permission android:name="android.permission.CAMERA" />
<uses-permission android:name="android.permission.RECORD_AUDIO" />
```

注意：Android 13+ 应使用更细粒度的媒体权限替代 `READ_EXTERNAL_STORAGE`。

---

## 7. 运行与调试

### 7.1 连接真机或启动模拟器

```bash
# 列出设备
adb devices

# 启动模拟器（示例）
emulator -avd Pixel_7_API_34
```

### 7.2 开发模式

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri
npm run tauri:android:dev
```

首次编译可能耗时较长（需编译 Rust + Android）。

### 7.3 构建 APK / AAB

```bash
# 调试 APK
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri
npx tauri android build --apk

# Release AAB
npx tauri android build --aab
```

构建产物位于：

```
tauri/src-tauri/gen/android/app/build/outputs/apk/
tauri/src-tauri/gen/android/app/build/outputs/bundle/
```

---

## 8. 常见问题

### Q1: `cargo check --target aarch64-linux-android` 报 `can't find crate for core`

原因：当前使用的是 Homebrew Rust，未安装 Android std。  
解决：切换到 rustup 并安装对应 target（见 2.1）。

### Q2: `ANDROID_HOME` 或 `NDK_HOME` 未设置

解决：检查环境变量是否正确导出，并确保 `NDK_HOME` 指向具体 NDK 版本目录（不是 `ndk/` 父目录）。

### Q3: 编译提示缺少 `libclang`

解决：安装 Xcode Command Line Tools：

```bash
xcode-select --install
```

### Q4: 模拟器上中文显示乱码

解决：确保模拟器系统语言包含中文，或在应用内通过 `/language` 设置。

---

## 9. 下一步

环境就绪后，按 `docs/android/android-port-guide.md` 中「待完成的关键步骤」进行 MVP 功能验证与修复。
