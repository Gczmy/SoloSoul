# SoloSoul Android 版本开发预研与实施计划

> 本文档基于当前 SoloSoul 代码库（Tauri v2 + React 19 + Rust workspace）的现状，制定将桌面端扩展至 Android 移动端的详细步骤。采用 **Tauri Mobile 渐进移植方案**，优先保证核心 MVP 可用，再逐步补齐高级能力。

---

## 1. 目标与范围

### 1.1 总体目标

在保持「本地优先、隐私优先」核心哲学的前提下，让 SoloSoul 能够在 Android 真机/模拟器上运行，并覆盖核心 MVP 功能。

### 1.2 MVP 功能范围

| 模块 | MVP 要求 | 备注 |
|------|---------|------|
| 账户 | 创建账户、登录、解锁、锁定、修改密码 | 与桌面端共享 Argon2id + AES-256-GCM 逻辑 |
| Vault 对象 | Page / Object 的增删改查、模板、属性编辑 | 核心数据模型复用 |
| 搜索 | 关键词搜索、多词匹配、结果列表 | 文本搜索即可，Embedding 搜索二期 |
| 附件 | 选择图片/文件、查看、删除 | 使用 Tauri dialog/fs plugin，拍照二期 |
| 设置 | 语言、主题、安全设置、审计日志 | 响应式适配 |
| 备份 | 创建/恢复/删除本地备份 | 私有目录内操作 |
| 导入导出 | `.solosoul` 加密包导入导出 | 密码验证流程 |

### 1.3 二期功能

- 本地 OCR（`ort` Android 构建 + 摄像头）
- 本地 Embedding（`ort` Android + 模型管理）
- WASM 插件运行时（等待 `wasmtime` 支持 Android 或迁移替代运行时）
- 设备同步（mDNS 替换为 Android NSD）
- 生物识别（`tauri-plugin-biometric`）
- 推送通知（`tauri-plugin-notification` 移动端）
- 自动更新（Google Play / 应用内下载）

---

## 2. 技术方案

### 2.1 推荐方案：Tauri Mobile 渐进移植

**理由：**

1. Tauri v2 已原生支持 Android/iOS 构建（`tauri android dev/build`）。
2. 现有 React 前端与 Rust 后端可最大程度复用，避免重写业务逻辑。
3. 风险分段释放：先验证核心流程，再攻克 OCR/插件/同步等复杂模块。
4. 与现有 CI/CD、发布流程（GitHub Actions、`tauri.conf.json`）最接近。

### 2.2 不采用的方案

| 方案 | 不采用原因 |
|------|-----------|
| 原生 Android（Kotlin/Jetpack Compose） | 需重写全部业务逻辑与加密层，工作量巨大 |
| Flutter | 需重建 UI 与 Rust FFI 桥接，与现有 Tauri 生态割裂 |

---

## 3. 当前代码库状态

### 3.1 已完成的预研改造

- **工程配置**：`tauri/package.json` 已添加 `tauri:android:dev` / `tauri:android:build` 脚本。
- **Cargo 条件编译**：桌面端独占依赖（`window-state`、`updater`、`wasmtime`、`ort`、`mdns-sd`、`dark-light`）已改为 `not(any(android, ios))` 条件编译。
- **移动端路径适配**：`lib.rs` 与 `state/app_state.rs` 已使用 `BaseDirectory::Data` 作为 Vault 根目录。
- **桌面端功能占位**：OCR、Embedding、插件、同步、发现服务、生物识别等命令已添加移动端占位实现。
- **前端响应式布局**：新增 `useIsMobile` hook、`MobileBottomNav` 组件、AppShell 移动端适配。
- **CI/CD**：`.github/workflows/build-android.yml` 已配置 Android Debug APK 构建。
- **文档**：`docs/android/android-port-guide.md` 与 `android-environment-setup.md` 已存在。

### 3.2 已验证的静态检查

- `cargo check --package solo_soul` ✅
- `cargo test --package solo_soul` ✅（293 单元测试 + 集成测试通过）
- `npm run lint` ✅
- `npx tsc --noEmit` ✅

### 3.3 当前阻塞

- 当前环境使用 Homebrew Rust（sysroot `/opt/homebrew/Cellar/rust/1.94.1`），未安装 Android std，无法执行 `cargo check --target aarch64-linux-android`。
- 尚未初始化 `tauri/src-tauri/gen/android/` 工程目录。
- 尚未在 Android 模拟器/真机上运行验证。

---

## 4. 详细实施步骤

### Phase 0：环境准备（预计 1–2 天）

#### 步骤 0.1：切换到 rustup 工具链

当前 Homebrew Rust 不支持 Android target，必须切换：

```bash
# 安装 rustup（如未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 设置默认工具链并安装 Android target
rustup default stable
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android
```

验证：

```bash
rustup target list --installed | grep android
rustc --print sysroot
# 应输出 ~/.rustup/toolchains/stable-aarch64-apple-darwin
```

#### 步骤 0.2：安装 Android Studio 与 SDK/NDK

1. 下载 [Android Studio](https://developer.android.com/studio)。
2. SDK Manager 中勾选：
   - **SDK Platforms**：Android 14 (API 34) 或 Android 13 (API 33)
   - **SDK Tools**：
     - Android SDK Build-Tools
     - Android SDK Platform-Tools
     - Android SDK Command-line Tools
     - NDK (Side by side) — 建议 r26b 或 r27
     - Android Emulator（如使用模拟器）

#### 步骤 0.3：配置环境变量

在 `~/.zshrc` 或 `~/.bash_profile` 中：

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"  # 替换为实际版本
export PATH="$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
```

验证：

```bash
adb --version
sdkmanager --list_installed | grep ndk
```

#### 步骤 0.4：安装项目依赖

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri
npm install
cargo check --package solo_soul
```

---

### Phase 1：初始化 Tauri Android 工程（预计 1–2 天）

#### 步骤 1.1：首次运行 Android dev 命令

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri
npm run tauri:android:dev
```

首次运行会自动生成 `tauri/src-tauri/gen/android/` 目录。按提示完成初始化。

#### 步骤 1.2：配置 AndroidManifest.xml

编辑 `tauri/src-tauri/gen/android/app/src/main/AndroidManifest.xml`：

```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.CAMERA" />

    <application
        android:label="SoloSoul"
        ... >
        <activity
            android:name=".MainActivity"
            android:configChanges="orientation|screenSize|smallestScreenSize|screenLayout"
            android:launchMode="singleTask"
            android:windowSoftInputMode="adjustResize"
            ... >
        </activity>
    </application>
</manifest>
```

> **注意**：Android 13+ 应使用细粒度媒体权限（`READ_MEDIA_IMAGES`、`READ_MEDIA_VIDEO`）替代 `READ_EXTERNAL_STORAGE`。

#### 步骤 1.3：配置 Android 主题与状态栏

- 在 `gen/android/app/src/main/res/values/themes.xml` 中设置与桌面端一致的暗色主题。
- 适配刘海屏、手势导航条（已在 CSS 中使用 `env(safe-area-inset-*)`）。

#### 步骤 1.4：验证首次编译

确保 `npm run tauri:android:dev` 能成功编译并启动到模拟器/真机，即使功能尚未完整。

---

### Phase 2：Rust 后端移动端验证与修复（预计 3–5 天）

#### 步骤 2.1：交叉编译检查

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri
cargo check --target aarch64-linux-android --package solo_soul
cargo check --target x86_64-linux-android --package solo_soul
```

修复所有平台相关编译错误。

#### 步骤 2.2：数据目录与文件权限验证

- 确认 `resolve_app_data_dir()` 在 Android 上返回 `/data/data/com.solosoul.app/` 下可写目录。
- 验证 SQLite 数据库、附件、备份、日志均能正常创建。

#### 步骤 2.3：进程锁与多实例保护

- 当前 `solosoul-core/src/process_lock.rs` 在移动端使用占位实现。
- 结合 `android:launchMode="singleTask"` 防止多实例。
- 考虑在应用启动时检测是否已有实例运行（通过文件锁或 SharedPreferences 标记）。

#### 步骤 2.4：附件路径与 Scoped Storage

- 验证 `commands/fs.rs` 中的 `allowed_fs_base()` 在移动端使用 `BaseDirectory::Data`。
- 验证附件复制、下载、删除在 Android 受限路径下工作正常。
- 对于从外部选择的文件，使用 Tauri `dialog` + `fs` plugin 返回的 URI/path，避免直接访问 `Downloads/` 等受限目录。

#### 步骤 2.5：导出/导入路径

- 验证 `~/` 在移动端映射到应用私有数据目录。
- 导出的 `.solosoul` 文件应保存到可通过系统文件管理器访问的位置（如 `Download/com.solosoul.app/`），或使用 Android Sharesheet 分享。

#### 步骤 2.6：资源文件打包

- 当前 `tauri.conf.json` 的 `bundle.resources` 包含 `resources/docs`、`resources/models`、`resources/pdfium`、插件市场目录。
- 移动端打包体积敏感：
  - `models`（OCR/Embedding）较大，建议二期按需下载，MVP 不打包。
  - `pdfium` 桌面端专用，移动端不打包。
  - 插件市场目录若体积大，可考虑不打包或精简 registry。

---

### Phase 3：前端移动端适配（预计 5–7 天）

#### 步骤 3.1：布局框架验证

- 验证 `AppShell` 在移动端正确显示底部导航。
- 验证 `MobileBottomNav` 各 Tab（Home / Search / AI Chat / Settings / Lock）路由跳转正常。

#### 步骤 3.2：核心页面响应式适配

| 页面 | 适配要点 |
|------|---------|
| LoginPage / BootstrapPage | 卡片宽度、输入框字体大小、软键盘弹出时不遮挡 |
| HomePage | 列表项增大触控区域、长按菜单 |
| ObjectListPage / ObjectDetailPage | 卡片布局、底部操作栏 |
| NewObjectPage / EditObjectPage | 表单字段垂直排列、底部保存按钮 |
| SearchResultsPage | 搜索框置顶、结果列表 |
| SettingsPage | 分组列表、开关控件放大 |
| BackupListPage / AttachmentListPage | 列表 + 操作按钮 |

#### 步骤 3.3：软键盘适配

- 测试输入框获得焦点时是否被软键盘遮挡。
- 必要时使用 `windowSoftInputMode="adjustResize"` + CSS `padding-bottom` 动态调整。
- 针对长表单页面，考虑滚动到焦点元素。

#### 步骤 3.4：触控与手势

- 按钮、列表项最小触控区域 44×44 dp。
- 返回手势与前端路由返回一致。
- 下拉刷新、上拉加载（如需要）。

#### 步骤 3.5：隐藏/禁用桌面端独占功能入口

在移动端隐藏或禁用以下入口：

- OCR 扫描
- AI Chat（若依赖本地 Embedding，可显示为占位提示）
- 插件市场
- 设备同步
- 自动更新
- 窗口状态相关设置

---

### Phase 4：MVP 功能端到端验证（预计 5–7 天）

#### 步骤 4.1：账户流程

1. 首次启动 → 进入创建账户向导。
2. 设置账户名、主密码、确认密码、提示词。
3. 创建完成后自动登录。
4. 锁定后通过密码解锁。
5. 验证密码错误提示、自动锁定（5 分钟无操作）。

#### 步骤 4.2：对象管理

1. 创建 Page。
2. 在 Page 下创建 Object。
3. 编辑对象属性。
4. 删除对象并进入回收站。
5. 从回收站恢复/永久删除。

#### 步骤 4.3：搜索

1. 输入关键词搜索对象。
2. 验证多词匹配与引号精确匹配。
3. 验证搜索结果点击跳转。

#### 步骤 4.4：附件

1. 从系统选择图片作为附件。
2. 查看附件。
3. 重命名、删除附件。

#### 步骤 4.5：设置

1. 切换语言（中文/英文）。
2. 切换主题（暗色/亮色）。
3. 修改主密码、密码提示。
4. 查看审计日志。

#### 步骤 4.6：备份

1. 创建本地备份。
2. 列出备份。
3. 恢复备份（需二次确认）。
4. 删除备份。

#### 步骤 4.7：导入导出

1. 导出当前 Vault 为 `.solosoul` 文件。
2. 从另一个 `.solosoul` 文件导入。
3. 验证导出密码与主密码不能相同。

---

### Phase 5：性能、体积与稳定性优化（预计 3–5 天）

#### 步骤 5.1：APK/AAB 体积控制

- 分析 APK 组成：`npx tauri android build --apk` 后使用 Android Studio APK Analyzer。
- 移除非必要资源：
  - 桌面端 NSIS 资源
  - PDFium 二进制
  - OCR/Embedding 模型（MVP 不打包）
- 启用 ProGuard/R8（如适用）。

#### 步骤 5.2：启动性能

- 测量从点击图标到首页可交互的时间。
- 延迟初始化非必要服务（如插件注册表刷新）。
- Rust `setup` 中已有日志记录，可据此分析启动耗时。

#### 步骤 5.3：内存与电量

- 避免后台持续轮询（桌面端主题轮询已在移动端禁用）。
- 大附件加载使用流式/分页。

#### 步骤 5.4：崩溃与异常处理

- 验证 panic hook 在 Android 上能写入日志。
- 测试低内存、磁盘空间不足等边界情况。

---

### Phase 6：CI/CD 与发布（预计 2–3 天）

#### 步骤 6.1：完善 GitHub Actions

当前 `.github/workflows/build-android.yml` 已能构建 debug APK。需补充：

1. Release AAB 构建任务。
2. 签名密钥注入（使用 GitHub Secrets）。
3. 上传产物到 Release Draft。
4. 多架构产物合并（arm64-v8a、armeabi-v7a、x86_64）。

#### 步骤 6.2：签名配置

在 `gen/android/app/build.gradle.kts` 中配置 release 签名：

```kotlin
android {
    signingConfigs {
        create("release") {
            storeFile = file(System.getenv("ANDROID_RELEASE_KEYSTORE") ?: "release.keystore")
            storePassword = System.getenv("ANDROID_RELEASE_KEYSTORE_PASSWORD")
            keyAlias = System.getenv("ANDROID_RELEASE_KEY_ALIAS")
            keyPassword = System.getenv("ANDROID_RELEASE_KEY_PASSWORD")
        }
    }
    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("release")
        }
    }
}
```

#### 步骤 6.3：版本号管理

- 保持 `tauri.conf.json` 的 `version` 与 `package.json` 一致。
- Android `versionCode` 由 Tauri 自动生成，需确认递增规则。

#### 步骤 6.4：应用商店材料

- 准备应用图标、截图、隐私政策、应用描述。
- Google Play 数据安全表单填写（说明无数据上传）。

---

### Phase 7：文档与验收（预计 2 天）

#### 步骤 7.1：更新/补充文档

| 文档 | 内容 |
|------|------|
| `docs/android/android-port-guide.md` | 维护实施进度与决策记录 |
| `docs/android/android-environment-setup.md` | 环境搭建与首次运行 |
| `docs/android/android-ui-guidelines.md` | 移动端 UI/UX 规范（新增） |
| `CHANGELOG.md` | 添加 Android 版本条目 |

#### 步骤 7.2：验收标准

完成以下检查后方可认为 MVP 通过：

- [ ] `cargo check --target aarch64-linux-android --package solo_soul` 无错误
- [ ] `npm run tauri:android:dev` 能在模拟器/真机启动
- [ ] 账户创建、登录、锁定、解锁流程正常
- [ ] Page / Object 增删改查正常
- [ ] 搜索功能正常
- [ ] 附件选择/查看/删除正常
- [ ] 语言/主题/密码设置正常
- [ ] 备份创建/恢复/删除正常
- [ ] `.solosoul` 导入导出正常
- [ ] Release AAB 构建成功
- [ ] 核心页面移动端 UI 无明显错位
- [ ] 无阻断性崩溃

---

## 5. 风险与应对

| 风险 | 影响 | 应对措施 |
|------|------|---------|
| Homebrew Rust 无法编译 Android target | 阻塞 Phase 0 | 切换到 rustup 工具链 |
| `wasmtime` 不支持 Android | 插件系统无法运行 | MVP 禁用插件；二期评估替代运行时 |
| `ort` Android 交叉编译复杂 | OCR/Embedding 延期 | MVP 禁用；提供手动导入替代 |
| Android Scoped Storage 限制 | 附件/导出路径错误 | 使用 Tauri path API + 应用私有目录 |
| APK 体积过大 | 上架困难 | 模型外置、按需下载、精简资源 |
| 多实例并发写入 Vault | SQLite 损坏 | `singleTask` launchMode + 文件锁 fallback |
| 软键盘遮挡输入框 | 体验差 | `adjustResize` + 动态 padding + 滚动到焦点 |
| 不同 Android 版本兼容性 | 崩溃 | 最低 API 28，多版本模拟器测试 |
| 移动端 UI 适配工作量大 | 延期 | 先 MVP 核心页面，其余逐步适配 |

---

## 6. 资源与时间安排

### 6.1 预估总工期

| 阶段 | 时间 |
|------|------|
| Phase 0：环境准备 | 1–2 天 |
| Phase 1：初始化 Tauri Android 工程 | 1–2 天 |
| Phase 2：Rust 后端移动端验证与修复 | 3–5 天 |
| Phase 3：前端移动端适配 | 5–7 天 |
| Phase 4：MVP 功能端到端验证 | 5–7 天 |
| Phase 5：性能、体积与稳定性优化 | 3–5 天 |
| Phase 6：CI/CD 与发布 | 2–3 天 |
| Phase 7：文档与验收 | 2 天 |
| **总计** | **约 4–6 周** |

### 6.2 关键依赖

- Android Studio + NDK r26+
- rustup stable 工具链 + Android target
- Tauri v2 CLI 移动端支持
- 真机或模拟器（推荐 Pixel 7 API 34）

### 6.3 交付物

1. 可运行的 Android MVP APK/AAB
2. 更新的 `docs/android/` 文档
3. 更新后的 `CHANGELOG.md`
4. 完善的 `.github/workflows/build-android.yml`
5. 通过 MVP 验收清单的测试报告

---

## 7. 下一步行动

1. 在本地 macOS 环境中完成 Phase 0（切换到 rustup、安装 Android Studio、配置 NDK）。
2. 执行 Phase 1.1：`npm run tauri:android:dev`，初始化 Android 工程。
3. 解决首次编译过程中的所有错误。
4. 按 Phase 2–Phase 4 逐步验证并修复 MVP 功能。
5. 每完成一个 Phase，更新 `docs/android/android-port-guide.md` 的进度清单。
