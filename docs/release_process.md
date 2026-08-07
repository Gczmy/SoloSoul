# SoloSoul 发布流程（macOS + Windows + Android）

> 同时发布 macOS DMG、Windows NSIS 安装包和 Android APK，版本号严格同步。
> 当前基于 **Tauri v2** 架构。

---

## 环境要求

| 平台 | 所需环境 |
|------|----------|
| **macOS** | Node.js >= 22、Rust (stable)、npm、create-dmg（可选，用于美观 DMG） |
| **Windows** | Node.js >= 22、Rust (stable)、npm、Visual Studio 2022+（提供 MSVC 工具链） |
| **Android** | Node.js >= 22、Rust (stable)、npm、Android SDK、Android NDK、JDK 17+ |

> 注：macOS 包必须在 Mac 上编译，Windows 包必须在 Windows 上编译，Android 包建议在 Mac/Linux 上编译（Tauri Android 工具链支持交叉编译）。

### 本地资源文件

以下资源文件被 `.gitignore` 排除，不会随仓库克隆自动出现，但 `tauri.conf.json` 将其声明为打包资源。构建前必须确保它们存在且完整，否则 Tauri 打包会失败。

- **模型文件**：`tauri/src-tauri/resources/models/` 下的 `all-MiniLM-L6-v2/`、`pp-ocr-v6-small/`
- **PDFium 动态库**：`tauri/src-tauri/resources/pdfium/` 下的平台对应库（macOS 为 `libpdfium.dylib`，Windows 为 `pdfium.dll`）

若从干净仓库开始构建，可运行自动下载脚本，或从已准备好的构建机复制：

```bash
# 在 Mac/Windows 构建前检查
ls tauri/src-tauri/resources/models/all-MiniLM-L6-v2
ls tauri/src-tauri/resources/models/pp-ocr-v6-small
ls tauri/src-tauri/resources/pdfium

# 自动下载 PDFium（根据当前平台）
bash tauri/scripts/download-pdfium.sh
```

### Tauri 自动更新器签名密钥

应用内「检查更新」依赖 Tauri Updater，要求 Release 包附带 Ed25519 签名文件（`.sig`）以及 `latest.json`。构建前必须配置私钥。

本项目默认将私钥保存在本机 `/Users/zzc/SoloSoul/`（不入库）：

```
~/SoloSoul/signing/tauri-updater/secret.key
~/SoloSoul/signing/tauri-updater/secret.key.pub
```

构建脚本会自动从该位置读取（无密码），也可手动导出为环境变量：

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/SoloSoul/signing/tauri-updater/secret.key)"
```

> 私钥 **绝对不要** 提交到 Git。建议同时备份到密码管理器或 CI Secrets（GitHub Secret 名：`TAURI_SIGNING_PRIVATE_KEY`）。

若首次设置签名密钥，使用 Tauri CLI 生成：

```bash
cd tauri
npx tauri signer generate -w ~/SoloSoul/signing/tauri-updater/secret.key
```

生成后会输出公钥，将其更新到 `tauri/src-tauri/tauri.conf.json`：

```json
"plugins": {
  "updater": {
    "pubkey": "替换为生成命令输出的公钥"
  }
}
```

> 如果修改了 `pubkey`，旧版本客户端将无法再通过自动更新接收新版本，需要重新安装。仅在旧私钥丢失或泄露时更换密钥。

---

## 阶段一：准备（在 Mac 上执行一次）

### 1. 检查代码库同步状态

确认 https://github.com/Gczmy/SoloSoul.git 与本地状态相同，如有未推送的本地更新，优先推送。

```bash
git status
git push origin main
```

### 2. 确认并统一版本号

Tauri 版本号分散在 **3 个文件**中，必须保持严格一致：

| 文件 | 字段 | 示例 |
|------|------|------|
| `tauri/package.json` | `"version': "2.0.0"` | `"version': "2.1.0"` |
| `tauri/src-tauri/tauri.conf.json` | `"version': "2.0.0"` | `"version': "2.1.0"` |
| `tauri/Cargo.toml` | `workspace.package.version` | `version = "2.1.0"` |

修改以上三个文件，将版本号更新为下一个版本（遵循 [SemVer](https://semver.org/lang/zh-CN/)）。

Android 还需要同步更新 `tauri/src-tauri/gen/android/app/tauri.properties`：

```properties
tauri.android.versionName=2.1.0
tauri.android.versionCode=20100
```

> 版本号格式：`主版本.次版本.补丁`。macOS、Windows 和 Android 使用完全相同的版本号。
> Tauri 不支持 `+buildNumber` 后缀，请使用纯 SemVer 格式。

### 3. 推送版本号更新

```bash
git add tauri/package.json tauri/src-tauri/tauri.conf.json tauri/Cargo.toml tauri/src-tauri/gen/android/app/tauri.properties
git commit -m "chore: bump version to 2.1.0"
git push origin main
```

---

## 阶段二：分别编译（在三台机器/环境上并行执行）

### 4a. macOS 构建（在 Mac 上执行）

```bash
cd /path/to/SoloSoul
./docs/build_macos_release.sh
```

脚本自动从 `tauri/package.json` 读取版本号（如 `2.1.0`），产物：

```
tauri/src-tauri/target/release/bundle/
├── macos/SoloSoul.app
├── macos/SoloSoul_2.1.0_arm64.app.tar.gz     # Tauri updater 用的 macOS 更新包
├── macos/SoloSoul_2.1.0_arm64.app.tar.gz.sig # updater 签名（生成 latest.json 用，不上传）
└── dmg/SoloSoul_2.1.0_arm64.dmg              # 首次安装用的 DMG
```

> 如需覆盖版本号，可传入参数：`VERSION="2.2.0" ./docs/build_macos_release.sh`
> （注意：传入参数不会修改源文件中的版本号，仅影响产物命名）

> 构建脚本会自动为 `.app.tar.gz` 调用 `npx tauri signer sign` 生成 `.sig`，需要提前设置 `TAURI_SIGNING_PRIVATE_KEY`。
> `.sig` 文件的内容会被写入 `latest.json`，但 `.sig` 文件本身不需要上传到 GitHub Release。
> macOS 自动更新实际使用的是 `.app.tar.gz`，不是 `.dmg`。

#### 签名说明

- 默认使用 **ad-hoc 签名**（`codesign --sign -`），**无需 Apple Developer 账户**
- 首次在另一台 Mac 上运行时，需在 系统设置 > 隐私与安全性 中手动允许
- 如需使用 Apple Development 证书：
  ```bash
  APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name" ./docs/build_macos_release.sh
  ```
- **对外公开分发**前，需获取 Apple Developer ID 账户并添加公证（Notarization）步骤

### 4b. Windows 构建（在 Windows 上执行）

在 Windows PC（或 Parallels/VMware 虚拟机）的 **Git Bash**（或 MSYS2 / WSL）中：

```bash
# 1. 先拉取最新代码（确保版本号已更新）
#    Git Bash 中 /d/ 对应 Windows 的 D: 盘，请替换为实际仓库路径
cd /d/path/to/SoloSoul
git pull origin main

# 2. 运行一键构建脚本
./docs/build_windows_release.sh
```

脚本会自动安装依赖并构建，产物：

```
tauri/src-tauri/target/release/bundle/
└── nsis/SoloSoul_2.1.0_x64-setup.exe
```

> 如需覆盖版本号，可传入参数：`VERSION="2.2.0" ./docs/build_windows_release.sh`

> Windows 脚本**不生成 `.sig`**，所有更新签名统一在 macOS 本机生成，避免在 Windows 上暴露私钥。

> Windows 代码签名需另行购买证书并使用 `signtool` 签名，当前未在脚本中实现。

### 4c. Android 构建（在 Mac 或 Linux 上执行）

Android 产物为通用 APK。构建前需要：

1. 安装 Android SDK、NDK 和 JDK 17+
2. 配置签名 keystore
3. 设置必要的环境变量

#### Android 环境变量

```bash
export ANDROID_HOME=$HOME/Library/Android/sdk
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/30.0.14904198
export SOLOSOUL_KEYSTORE_PATH=<your-keystore-path>
export SOLOSOUL_KEYSTORE_PASSWORD=<your-keystore-password>
export SOLOSOUL_KEY_ALIAS=solosoul-upload
export SOLOSOUL_KEY_PASSWORD=<your-key-password>

# 构建 Android 需使用 rustup 版本，避免 Homebrew Rust 不支持交叉编译
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
```

> 注：本地 Homebrew 版 Rust 不支持 Android 交叉编译目标，构建前必须将 rustup 版 Rust 置于 PATH 优先位置。

#### Debug APK（无需签名，适合功能测试）

```bash
cd tauri
cargo tauri android build -d
```

产物路径：`tauri/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`

#### Release APK（直接安装 / 内测 / 默认产物）

```bash
cd tauri
cargo tauri android build --apk
```

产物路径：`tauri/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`

> **为什么默认构建 APK 而不是 AAB？** 当前 SoloSoul 不进入 Google Play 商店分发，APK 可直接在 Android 设备上安装，更适合 GitHub Release 侧载和内测。AAB 无法直接安装，仅作为未来上架 Play Store 时的备选格式。

#### Android 版本号规则

| 来源 | 说明 |
|------|------|
| `versionName` | 取自 `tauri/src-tauri/gen/android/app/tauri.properties` 的 `tauri.android.versionName`，应与 `tauri.conf.json` 的 `version` 保持一致。 |
| `versionCode` | 取自 `tauri.properties` 的 `tauri.android.versionCode` 基础值，并叠加 `GITHUB_RUN_NUMBER` 以保证每次 CI 构建单调递增。Play Store 要求 `versionCode` 只增不减。 |

`build.gradle.kts` 中的计算逻辑：

```kotlin
versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt() +
    (System.getenv("GITHUB_RUN_NUMBER") ?: "0").toInt()
```

发版前确保 `tauri.android.versionCode` 已按版本号规则更新（例如 `2.1.0` 可编码为 `20100` 或更高）。若基础值已高于新计算值，必须继续递增，不可下降，否则 Play Store 会拒绝上传。

#### Android 签名说明

Release APK 使用 `signingConfigs.release`，从环境变量读取 keystore 信息：

- `SOLOSOUL_KEYSTORE_PATH`
- `SOLOSOUL_KEYSTORE_PASSWORD`
- `SOLOSOUL_KEY_ALIAS`
- `SOLOSOUL_KEY_PASSWORD`

本地构建时若环境变量缺失，则回退为未签名（debug），不会报错；CI 发布时必须提供上述变量。

#### APK 校验和（SHA-256）生成与签名（P003）

Android 客户端下载 APK 后会自动验证 SHA-256 校验和。发布前需要生成校验和文件、**签名**并随 APK 一起上传到 GitHub Release。

```bash
# 为 Release APK 生成 SHA-256 校验和文件 + minisign 签名
./docs/compute-apk-checksum.sh SoloSoul-Releases/SoloSoul_2.6.1_universal-release.apk

# 产物：
#   SoloSoul_2.6.1_universal-release.apk.sha256       （64 位 hex 编码的 SHA-256 哈希）
#   SoloSoul_2.6.1_universal-release.apk.sha256.minisig （校验和文件的 minisign 签名）
```

> **P003（校验和防篡改）**：校验和不能再与 APK 同通道无条件信任——脚本会使用
> **embed 注册表专用密钥**（`~/SoloSoul/signing/embed-registry/embed-registry.key`，
> 客户端公钥已编译进 `update.rs`）对 `.sha256` 文件签名。客户端下载校验和后先验签
> （`.sha256.minisig`），验签失败即拒绝该校验和（不执行 SHA-256 校验）。
>
> `.sha256` 与 `.sha256.minisig` 文件都很小，**必须与 APK 一同上传到 GitHub Release**。
> 缺少签名文件时 Android 客户端不会进行 SHA-256 校验（不阻断下载，但失去完整性保障）。

---

## 阶段三：收集与发布（在 Mac 上执行）

### 5. 收集产物

将 macOS、Windows 和 Android 产物都传输到 Mac，统一放到同一目录：

```
/path/to/SoloSoul/SoloSoul-Releases
├── SoloSoul_2.1.0_arm64.app.tar.gz          # macOS 自动更新包（必需）
├── SoloSoul_2.1.0_arm64.dmg                 # macOS 首次安装 DMG（可选但推荐）
├── SoloSoul_2.1.0_x64-setup.exe             # Windows 安装包
├── SoloSoul_2.1.0_universal-release.apk      # Android 通用安装包
└── SoloSoul_2.1.0_universal-release.apk.sha256 # Android APK SHA-256 校验和（推荐）
```

> Android 校验和文件（`.sha256`）需在 Android 构建后通过 `./docs/compute-apk-checksum.sh` 生成。
> 如果不包含此文件，Android 客户端在下载后不会进行 SHA-256 验证，但更新功能不受影响。

### 6. 统一签名（在 Mac 上执行）

所有平台的 Tauri updater `.sig` 签名统一在 macOS 上生成：

```bash
cd /path/to/SoloSoul
./docs/sign_artifacts.sh
```

脚本会读取 `~/SoloSoul/signing/tauri-updater/secret.key`（或环境变量 `TAURI_SIGNING_PRIVATE_KEY`），为 `SoloSoul-Releases/` 中的 `.dmg`、`.exe` 和 `.AppImage` 生成同名 `.sig` 文件。

> Android AAB 签名由 Gradle 构建时完成，不需要在此步骤额外签名。

### 7. 本地验证

#### macOS
- 双击 DMG 安装，将 `SoloSoul.app` 拖入 Applications
- 首次启动若提示「无法打开，因为无法验证开发者」，前往 系统设置 > 隐私与安全性 > 安全性，点击「仍要打开」
- 验证 Vault 解锁、对象 CRUD、设置页面等基础功能

#### Windows
- 双击 `.exe` 安装包完成安装
- 从开始菜单或桌面快捷方式启动 SoloSoul
- 验证 Vault 解锁、对象 CRUD、设置页面等基础功能

#### Android
- 通过 adb 安装 APK：
  ```bash
  adb install -r tauri/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
  ```
- 或在 Android Studio 中打开 `tauri/src-tauri/gen/android` 项目，直接运行/调试
- 验证启动、Vault 解锁、对象 CRUD、设置页面等基础功能
- 对于 Release APK，确认安装后验证启动、Vault 解锁、对象 CRUD、设置页面等基础功能

### 8. 生成 latest.json

在 Mac 上执行：

```bash
cd tauri
node scripts/generate-latest-json.js \
  "$(node -p "require('./src-tauri/tauri.conf.json').version")" \
  ../SoloSoul-Releases \
  ../SoloSoul-Releases/latest.json
```

生成的 `latest.json` 包含各平台安装包下载地址与 Ed25519 签名，供应用内更新器读取。

> 注意：`latest.json` 目前仅用于桌面端（macOS + Windows）自动更新。Android 更新通过 Google Play 商店分发。

### 9. GitHub Release 发布

在 https://github.com/Gczmy/SoloSoul.git 创建 Release：

1. 点击 "Draft a new release"
2. 选择或创建标签（如 `v2.1.0`）
3. 填写 Release 标题和说明
4. **上传以下附件**（应用内更新器依赖 `latest.json` 中的签名，`.sig` 文件本身不必上传）：
   - `SoloSoul_2.1.0_arm64.app.tar.gz`             # macOS 自动更新包（必需）
   - `SoloSoul_2.1.0_arm64.dmg`                    # macOS 首次安装 DMG（推荐）
   - `SoloSoul_2.1.0_x64-setup.exe`                # Windows 安装包
   - `SoloSoul_2.1.0_universal-release.apk`         # Android 通用安装包
   - `SoloSoul_2.1.0_universal-release.apk.sha256`  # Android APK SHA-256 校验和（推荐）
   - `latest.json`
5. 点击 "Publish release"

> 通过 GitHub Releases 上传，而不是通过 git 提交。GitHub Releases 允许上传附件，这些附件不存储在 git 仓库中。

> **当前发布策略说明**：当前版本不进入 Google Play 商店或 Play Store 内部测试轨道，Android 产物以通用 APK 形式随 GitHub Release 发布，方便用户直接下载安装。如未来进入 Play Store，可额外构建 AAB 并提交 Play 商店后台。

#### 强制更新标记 `[MANDATORY]`

Android 客户端支持**强制更新**：在 Release body 中插入 `[MANDATORY]` 标记，用户打开「关于」页面后会看到不可关闭的全屏更新对话框，必须更新才能继续使用。

```markdown
## v2.7.0 安全修复

[MANDATORY]

- 修复加密库安全漏洞
- 更新依赖项
```

**行为：**
- `[MANDATORY]` 标记在 Release body 中出现任意位置均可被识别
- 标记会在返回给客户端前自动清除，用户不会看到原始标记文本
- 如果不需要强制更新，只需省略 `[MANDATORY]` 即可
- 该标记**仅对 Android 客户端生效**，桌面端不受影响

**适合使用场景：**
- 安全漏洞修复（CVE）
- 数据格式不兼容的版本
- 紧急功能修复

### 10. 更新 CHANGELOG.md

在 https://github.com/Gczmy/SoloSoul.git 的 `CHANGELOG.md` 中补充本次版本的详细变更记录（检查 commit 记录，不要遗漏），并随版本号更新一起提交到 `main` 分支。

> 本仓库 `CHANGELOG.md` 为详细版本，涵盖所有 Added / Changed / Fixed / Security / Chore 条目，作为对外发布和内部追溯的唯一变更来源。

---

## CI/CD 自动发布（备选）

Push 到 `main` 分支后，GitHub Actions 会自动：

1. `frontend-check` job：TypeScript 类型检查、Lint、单元测试
2. `rust-test` job：Rust 格式化检查、Clippy、单元测试
3. `build-macos` job：在 `macos-latest` runner 上构建 DMG（仅 main push）
4. `build-windows` job：在 `windows-latest` runner 上构建 NSIS（仅 main push）
5. `build-android` job：在 `ubuntu-latest` runner 上构建 APK（仅 main push）
6. `release` job：收集产物，统一创建并发布 GitHub Release（非 Draft、非 Pre-release），使 `releases/latest/download/latest.json` 立即对客户端可见。

详见 `.github/workflows/ci_cd.yml` 和 `.github/workflows/build-android.yml`。

---

## 附录：版本号速查

| 组件 | 文件路径 | 字段 |
|------|----------|------|
| Node.js / npm | `tauri/package.json` | `"version": "x.y.z"` |
| Tauri 配置 | `tauri/src-tauri/tauri.conf.json` | `"version": "x.y.z"` |
| Rust / Cargo | `tauri/Cargo.toml` | `[workspace.package] version = "x.y.z"` |
| Android | `tauri/src-tauri/gen/android/app/tauri.properties` | `tauri.android.versionName` / `tauri.android.versionCode` |
