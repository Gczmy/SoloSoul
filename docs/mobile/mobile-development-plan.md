# SoloSoul 移动端开发实施文档

> **读者**：负责移动端开发的工程师。本文档是任务书，可直接按编号领任务开发。
> **配套阅读**：
> - `docs/mobile/mobile-adaptation-report.md` — 调研背景（Anytype/Notion 策略对比、路线选择依据）
> - `docs/android/android-port-guide.md` — Android 移植进度与已完成的 Phase 0–3 改造
> - `docs/android/android-implementation-plan.md` — 原始 7 阶段计划（工期估算来源）
> - `docs/android/android-environment-setup.md` — 开发环境搭建
>
> 文档版本：v1.0（2026-07-16）。基于 commit `32975dc4` 的代码事实编写。

---

## 0. 开发约定（所有任务通用）

### 0.1 任务编号与优先级

- 编号格式 `MOB-Px-nn`，`x` 为里程碑（P0 最高，P5 最低），同里程碑内按编号顺序执行。
- 每个任务包含：**目标 / 现状 / 实现步骤 / 验收标准 / 预估**。预估单位为「1 名熟悉本项目的工程师 · 天」。
- 任务间依赖在各任务头部标注；无标注即只依赖 P0 完成。

### 0.2 分支与提交

- 从 `master` 切功能分支：`git checkout -b feat/mobile-p1-signing`（示例）。
- 提交信息遵循仓库现有规范：`<type>(<scope>): <中文描述>`，scope 用 `android` / `ios` / `mobile` / `security` 等，例如 `feat(android): 配置 release 签名与 AAB 构建`。
- PR 触发 `pr_check.yml`（tsc + lint + vitest + fmt + clippy + cargo test），**必须全绿才可合并**。

### 0.3 提交前本地验证清单

```bash
cd tauri
npm run check-all            # tsc + fmt + clippy + lint + vitest（桌面目标）
# 涉及 Rust 改动且可能影响移动端时，额外执行：
cargo ndk -t aarch64-linux-android check   # 让 #[cfg(mobile)] 分支参与类型检查（需 cargo-ndk）
# 涉及 Android 工程改动时：
npx tauri android build --apk --debug      # 验证 Android 端可构建
```

### 0.4 代码风格

- 代码注释以中文为主，技术术语保留英文（与现有代码一致）。
- Rust 移动端适配沿用现有模式：`#[cfg(desktop)]` / `#[cfg(mobile)]` 门控 + `mobile_not_supported()` 桩（参考 `tauri/src-tauri/src/commands/mod.rs:55-64`），**IPC 签名不得因平台分叉**。
- 前端功能门控用 `isMobilePlatformSync()`（平台判定），布局响应用视口判定 hook（见 MOB-P0-04）。
- **禁止提交**：keystore 文件、签名密码、`local.properties`、任何真实密钥。`gen/android/` 是已纳入版本控制的生成工程，允许修改但禁止全量重新生成覆盖（会丢定制代码，见各任务说明）。

### 0.5 真机验证要求

凡标注「需真机」的任务，至少在 1 台 API 28+ 的 Android 真机（或模拟器，注明时）上验证通过才算完成。低端机（≤4GB RAM）优先用于性能类验收。

---

## 1. 里程碑总览

| 里程碑 | 目标 | 任务数 | 预估总计 |
|---|---|---|---|
| P0 | 正确性与安全兜底 | 4（1 已完成） | 3–4 天 |
| P1 | Android 发布化 | 7 | 10–14 天 |
| P2 | 手机 UI/UX 打磨 | 7 | 10–15 天 |
| P3 | 原生能力补齐（生物识别/同步/通知） | 3 | 15–25 天 |
| P4 | OCR / Embedding 移动方案 | 2 | 15–30 天（可延后） |
| P5 | iOS 启动 | 4 | 20–30 天（依赖 P1/P3） |

| 编号 | 任务 | 预估 | 依赖 |
|---|---|---|---|
| MOB-P0-01 | 自动锁定（✅ 已完成） | — | — |
| MOB-P0-02 | 补 POST_NOTIFICATIONS 权限 | 0.5 天 | — |
| MOB-P0-03 | 移动端 bundle 排除 OCR 模型 | 1 天 | — |
| MOB-P0-04 | 统一 mobile 判定语义 | 1.5 天 | — |
| MOB-P1-01 | minSdk 决策与调整 | 0.5 天 | — |
| MOB-P1-02 | release 签名配置 | 1 天 | P1-01 |
| MOB-P1-03 | AAB 构建与 versionCode 管理 | 1 天 | P1-02 |
| MOB-P1-04 | CI：移动端 Rust 交叉检查 | 1 天 | — |
| MOB-P1-05 | CI：release AAB 上传 draft release | 1 天 | P1-03 |
| MOB-P1-06 | 移动视口 E2E 冒烟测试 | 2–3 天 | P0-04 |
| MOB-P1-07 | 启动性能基线测量 | 2 天 | — |
| MOB-P2-01 | 核心页面响应式补齐 | 4–6 天 | P0-04 |
| MOB-P2-02 | 弹层自适应宽度与全量安全区 | 1.5 天 | P0-04 |
| MOB-P2-03 | hover 依赖消除 | 2–3 天 | P2-01 |
| MOB-P2-04 | 触控目标尺寸 ≥44pt | 1 天 | P2-01 |
| MOB-P2-05 | 虚拟键盘遮挡回归 | 1 天（需真机） | P2-01 |
| MOB-P2-06 | 桌面专属路由移动端守卫 | 1 天 | P0-04 |
| MOB-P2-07 | Android 快捷方式「新建对象」 | 1.5 天 | — |
| MOB-P3-01 | 生物识别（tauri-plugin-biometric） | 5–8 天 | P1 |
| MOB-P3-02 | 设备同步（Android NSD 桥） | 8–12 天 | P1、P2-06 |
| MOB-P3-03 | 本地通知场景（锁定/备份提醒） | 2–3 天 | P0-02 |
| MOB-P4-01 | OCR/embedding 模型按需下载 | 5–8 天 | P0-03 |
| MOB-P4-02 | 移动端推理后端（ort/ML Kit） | 10–20 天 | P4-01 |
| MOB-P5-01 | `tauri ios init` 与工程提交 | 2–3 天 | P1 |
| MOB-P5-02 | iOS `attachment_open` 实现 | 3–5 天 | P5-01 |
| MOB-P5-03 | iOS 状态栏/生物识别补齐 | 3–5 天 | P5-01、P3-01 |
| MOB-P5-04 | Apple 签名与 TestFlight | 3–5 天（需真机） | P5-01 |

---

## 2. P0 — 正确性与安全兜底

### MOB-P0-01 自动锁定（✅ 已完成，无需开发）

- **状态**：已于 2026-07-16 完成并推送（commit `32975dc4`）。
- **实现**：`tauri/src/hooks/useAutoLock.ts`（闲置超时调用 `vaultStore.lock()`，`visibilitychange` 回前台立即结算，覆盖移动端切后台与系统休眠）；`tauri/src/stores/autoLockPauseStore.ts`（暂停计数）；`PasswordVerificationDialog.tsx` 打开期间暂停计时；8 个单元测试 `useAutoLock.test.ts`。
- **后续任务引用**：P2-05 键盘回归时需验证密码验证框打开超时不被锁定；P3-03 的锁定提醒通知基于此功能。

### MOB-P0-02 补 POST_NOTIFICATIONS 权限

- **预估**：0.5 天（需真机或 API 33+ 模拟器）｜ **依赖**：无

**现状**：前端 `tauri/src/lib/notification.ts:44-48` 已调用 `isPermissionGranted()/requestPermission()`，`tauri/src-tauri/capabilities/` 已声明 `notification:default`，但 `tauri/src-tauri/gen/android/app/src/main/AndroidManifest.xml` 缺少 `POST_NOTIFICATIONS` 权限声明。Android 13（API 33）起通知属于运行时权限，**未声明即静默收不到通知**（AI 回复完成提醒在 Android 13+ 失效）。

**实现步骤**：

1. 编辑 `tauri/src-tauri/gen/android/app/src/main/AndroidManifest.xml`，在 `<manifest>` 节点内、现有 `<uses-permission>` 区块（第 3–8 行）追加：
   ```xml
   <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
   ```
2. 确认 `tauri-plugin-notification` 在 Android 端的权限请求由 `requestPermission()` 自动触发（插件内部调用 `ActivityCompat.requestPermissions`），无需额外 Kotlin 代码。
3. 构建 debug APK：`cd tauri && npx tauri android build --apk --debug`，用 `aapt dump permissions tauri/src-tauri/gen/android/app/build/outputs/apk/debug/*.apk | grep POST_NOTIFICATIONS` 确认权限已进入 APK。
4. 真机/模拟器（API ≥ 33）验证：触发一次 AI 流式回复完成 → 首次弹出系统通知权限请求 → 允许后收到系统通知；拒绝后应用内 toast 仍正常（`notification.ts:72-76` 的兜底逻辑）。

**验收标准**：

- [ ] manifest 含 `POST_NOTIFICATIONS`，APK 权限清单可检出。
- [ ] API 33+ 设备首次触发时弹出权限请求，允许后能收到「AI 已完成回复」系统通知。
- [ ] API < 33 设备行为不变（权限自动授予）。
- [ ] `npm run check-all` 通过（本任务不涉及代码，仅确认无回归）。

### MOB-P0-03 移动端 bundle 排除 OCR 模型

- **预估**：1 天 ｜ **依赖**：无

**现状**：基础配置 `tauri/src-tauri/tauri.conf.json:52-55` 的 `bundle.resources` 包含 `resources/models/pp-ocr-v6-small`（约 30MB）。Tauri 2 构建时会把平台配置文件（`tauri.android.conf.json` / `tauri.ios.conf.json`）与基础配置**按键合并**，平台 overlay 只能增加资源、不能删除，导致 Android APK 白白打进 30MB OCR 模型——而移动端 OCR 命令全部是桩（`tauri/src-tauri/src/commands/ocr.rs` 的 `#[cfg(mobile)]` 分支），模型完全不会被使用。参照 Anytype「重核心进不了扩展进程」的教训，移动端包体必须精打细算。

**实现步骤**：

1. 修改 `tauri/src-tauri/tauri.conf.json`，将 `bundle.resources` 缩减为仅保留文档：
   ```json
   "resources": {
     "resources/docs": "docs"
   }
   ```
2. 把模型资源迁移到三个桌面平台 overlay（文件均已存在于 `tauri/src-tauri/`），分别在 `tauri.macos.conf.json`、`tauri.windows.conf.json`、`tauri.linux.conf.json` 的 `bundle` 节点内加入：
   ```json
   "resources": {
     "resources/models/pp-ocr-v6-small": "models/pp-ocr-v6-small"
   }
   ```
   注意保留各 overlay 已有内容，只做合并添加。
3. 桌面回归：macOS 执行 `npm run tauri build`，检查 `src-tauri/target/release/bundle/macos/SoloSoul.app/Contents/Resources/models/pp-ocr-v6-small/` 存在；Windows/Linux 同样验证（可在 CI 产物上检查）。
4. Android 验证：`npx tauri android build --apk --debug`，然后 `unzip -l tauri/src-tauri/gen/android/app/build/outputs/apk/debug/*.apk | grep -i models` 应无结果；对比任务前后 APK 体积，应下降约 30MB。
5. 确认 `tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/MainActivity.kt` 的 `extractAssetsToDataDir()` 不依赖 models 资源（它只复制 `docs` 与 `SoloSoul_plugin_market`，无需改动）。
6. 确认移动端 OCR 桩页面（`/settings/ocr` 在移动端已隐藏入口）无「模型未找到」类报错路径——OCR 命令在移动端均为桩，不会触碰资源目录。

**验收标准**：

- [ ] Android APK assets 中无 `models/` 目录，APK 体积下降 ≥ 25MB。
- [ ] macOS/Windows/Linux 桌面 bundle 中 `models/pp-ocr-v6-small` 仍在，桌面 OCR 功能无回归。
- [ ] 桌面与 Android 构建均成功，`npm run check-all` 通过。

### MOB-P0-04 统一 mobile 判定语义

- **预估**：1.5 天 ｜ **依赖**：无 ｜ **阻塞**：P1-06、P2-01、P2-02、P2-06

**现状**：代码库存在两套「mobile」判定，命名容易误用：

- `tauri/src/hooks/useIsMobile.ts` — 视口宽度 < 768px（窄桌面窗口也会命中），共 9 个业务文件使用（`AppRoutes.tsx`、`AppShell.tsx`、`HomePage.tsx`、`AboutPage.tsx`、`GlobalAttachmentManager.tsx`、`AppearanceSettingsPage.tsx`、`SettingsPage.tsx`、`AttachmentPreviewOverlay.tsx`、`AttachmentViewer.tsx`）。
- `tauri/src/lib/platform.ts` — `isMobilePlatform()/isMobilePlatformSync()`（真实平台判定，Android/iOS）。

现状下「窄窗口桌面」会被当作移动端布局，「平板宽屏 Android」会被当作桌面布局，功能门控与布局判定语义混杂。

**实现步骤**：

1. 重命名 `tauri/src/hooks/useIsMobile.ts` → `tauri/src/hooks/useIsNarrowViewport.ts`，导出 hook 改名 `useIsNarrowViewport`，保留 `MOBILE_BREAKPOINT = 768`，更新文件头注释为「窄视口判定（布局用途）」。
2. 全库替换引用：`grep -rn "useIsMobile" tauri/src/` 逐文件更新 import 与调用；调用处局部变量 `isMobile` 改为 `isNarrowViewport`（涉及 `AppRoutes.tsx:34` 等）。
3. **逐处审查语义**（关键步骤，不是机械替换）：
   - 纯布局用途（底部导航、卡片栅格、字号）→ 保持 `useIsNarrowViewport`。
   - 功能门控用途（如 `AboutPage.tsx:51-54` 跳过更新检查、`SettingsPage.tsx:113-144` 隐藏桌面设置项、`AppRoutes.tsx:55` 跳过桌面更新器）→ 改为 `isMobilePlatformSync()`；注意 `isMobilePlatformSync` 依赖缓存，须确认 `initPlatform()` 已在应用启动早期调用（检查 `main.tsx`/`App.tsx`，若未调用则补上）。
4. 更新受影响测试（`grep -rn "useIsMobile" tauri/src --include="*.test.*"`），并补充一条 lint 层无法保证、需在 code review 中检查的团队约定：**布局用视口、功能用平台**。

**验收标准**：

- [ ] `grep -rn "useIsMobile" tauri/src/` 无任何结果。
- [ ] 上述功能门控调用点全部改为平台判定；桌面窄窗口（<768px）下不再隐藏桌面专属功能入口。
- [ ] `npm run check-all` 通过；相关单测更新后全绿。

---

## 3. P1 — Android 发布化

### MOB-P1-01 minSdk 决策与调整

- **预估**：0.5 天 ｜ **依赖**：无

**现状**：`tauri/src-tauri/gen/android/app/build.gradle.kts:22` 为 `minSdk = 24`，而 `docs/android/android-implementation-plan.md:457` 建议 API 28。API 24–27 设备存量已极低，且后续生物识别（BiometricPrompt 在 API 28+ 行为才完整）、WebView 新特性在旧版本上兼容性差。

**实现步骤**：

1. `build.gradle.kts` 中 `minSdk = 24` 改为 `minSdk = 28`。
2. `npx tauri android build --apk --debug` 构建通过。
3. 在 API 28、33、35 模拟器（或真机）各做一次启动 + 解锁 + 列表冒烟。

**验收标准**：

- [ ] minSdk=28；三个 API 级别冒烟通过；无 `minSdkVersion` 相关 lint 报错。

### MOB-P1-02 release 签名配置

- **预估**：1 天 ｜ **依赖**：P1-01

**现状**：`build.gradle.kts` 的 `buildTypes.release` 无 `signingConfig`，release 包未签名无法安装/上架。

**实现步骤**：

1. 生成 upload keystore（**本地一次性操作，产物绝不入库**）：
   ```bash
   keytool -genkey -v -keystore solosoul-upload.jks -keyalg RSA -keysize 4096 -validity 10950 -alias solosoul-upload
   ```
   将 jks 文件与密码存入团队密码库；同时 `base64 solosoul-upload.jks` 存入 GitHub Secrets（见步骤 4）。
2. 修改 `build.gradle.kts`，在 `android {}` 块内、`buildTypes` 之前加入：
   ```kotlin
   signingConfigs {
       create("release") {
           val path = System.getenv("SOLOSOUL_KEYSTORE_PATH")
           if (path != null) {
               storeFile = file(path)
               storePassword = System.getenv("SOLOSOUL_KEYSTORE_PASSWORD")
               keyAlias = System.getenv("SOLOSOUL_KEY_ALIAS") ?: "solosoul-upload"
               keyPassword = System.getenv("SOLOSOUL_KEY_PASSWORD")
           }
       }
   }
   ```
   并在 `getByName("release") { ... }` 内加 `signingConfig = signingConfigs.getByName("release")`（环境变量缺失时回退为未签名，保证本地 release 编译不炸，但 CI 必须提供变量）。
3. 本地验证：`SOLOSOUL_KEYSTORE_PATH=... npx tauri android build --apk`，`apksigner verify --print-certs *.apk` 确认签名链。
4. GitHub Secrets 配置：`SOLOSOUL_KEYSTORE_BASE64`、`SOLOSOUL_KEYSTORE_PASSWORD`、`SOLOSOUL_KEY_ALIAS`、`SOLOSOUL_KEY_PASSWORD`。CI 中使用方式见 P1-05。

**验收标准**：

- [ ] release APK 签名验证通过；仓库中搜索不到 keystore 文件与明文密码（`git ls-files | grep -i jks` 为空）。
- [ ] `.gitignore` 确认包含 `*.jks`、`*.keystore`、`local.properties`（若无则补上）。

### MOB-P1-03 AAB 构建与 versionCode 管理

- **预估**：1 天 ｜ **依赖**：P1-02

**现状**：`build.gradle.kts:24-25` 的 `versionCode`/`versionName` 从 `tauri.properties` 读取，默认 `1`/`1.0`，从未递增；Play Store 要求 AAB 格式且 versionCode 单调递增。

**实现步骤**：

1. versionName 策略：以 `tauri.conf.json` 的 `version`（当前 2.5.12）为唯一来源，构建后检查产物 versionName 是否一致（`aapt dump badging *.apk | grep versionName`）；若仍为 `tauri.properties` 的默认值 `1.0`，则手工对齐 `tauri.properties` 的 `tauri.android.versionName` 并固化到发布脚本。
2. versionCode 策略：采用 `major*10000 + minor*100 + patch` 起步（2.5.12 → 20512），后续每次发布 +1；在 CI 中用 `GITHUB_RUN_NUMBER` 叠加避免重复：`versionCode = 20512 + runNumber`。实现位置：`build.gradle.kts` 的 `defaultConfig` 改为：
   ```kotlin
   versionCode = (
       tauriProperties.getProperty("tauri.android.versionCode", "1").toInt() +
       (System.getenv("GITHUB_RUN_NUMBER") ?: "0").toInt()
   )
   ```
   并在 `tauri.conf.json` 同级规划：发版时由发布脚本更新基础 versionCode（可复用 `docs/release_process.md` 流程，给该文档补一节 Android 版本规则）。
3. 构建 AAB：`npx tauri android build --aab`，产物位于 `gen/android/app/build/outputs/bundle/release/*.aab`。
4. 用 `bundletool build-apks --bundle=app-release.aab --ks=...` 本地转成 APK 安装真机验证（bundletool 从 Google 官方获取）。

**验收标准**：

- [ ] CI 与本地均可产出已签名 AAB；versionCode 按规则递增；bundletool 转出的 APK 真机安装运行正常。

### MOB-P1-04 CI：移动端 Rust 交叉检查

- **预估**：1 天 ｜ **依赖**：无

**现状**：`.github/workflows/build-android.yml` 只构建 debug APK；`pr_check.yml`/`ci_cd.yml` 的 fmt/clippy/test 全部只跑桌面 target，`#[cfg(mobile)]` 分支（如 `commands/ocr.rs`、`commands/sync.rs`、`crates/solosoul-sync/src/mobile.rs` 的大量移动端代码）**不参与任何 CI 类型检查**，移动端代码腐烂无警报。

**实现步骤**：

1. 在 `.github/workflows/build-android.yml` 的 `Install NDK` 步骤之后、`Build Android debug APK` 之前插入：
   ```yaml
      - name: Install cargo-ndk
        run: cargo install cargo-ndk --locked

      - name: Cargo check (Android target)
        run: cargo ndk -t aarch64-linux-android check
        env:
          ANDROID_HOME: ${{ env.ANDROID_HOME }}
          NDK_HOME: ${{ env.NDK_HOME }}
   ```
   （`build-android.yml` 已对 PR 触发，此步骤天然覆盖 PR 检查。）
2. 可选加固：同步骤追加 `cargo ndk -t aarch64-linux-android clippy -- -D warnings`（若现有代码在 mobile cfg 下有告警，先修再开，或本任务只开 `check`、clippy 另立任务）。
3. 触发一次 PR 验证步骤真实执行且通过。

**验收标准**：

- [ ] PR 与 master push 均执行 `cargo ndk check`；故意在 `#[cfg(mobile)]` 分支制造类型错误时 CI 变红（验证后回滚该测试改动）。

### MOB-P1-05 CI：release AAB 上传 draft release

- **预估**：1 天 ｜ **依赖**：P1-03

**现状**：`ci_cd.yml:159-247` 的 release 流程只覆盖 macOS DMG 与 Windows NSIS；`build-android.yml` 只产 debug APK。

**实现步骤**：

1. `build-android.yml` 新增 job `build-android-release`，`if: github.event_name == 'push' && github.ref == 'refs/heads/master'`：
   - 检出（含子模块）、Node/Rust/JDK/SDK/NDK 步骤复用现有 job。
   - 解码 keystore：
     ```yaml
     - name: Decode keystore
       run: echo "${{ secrets.SOLOSOUL_KEYSTORE_BASE64 }}" | base64 -d > /tmp/solosoul-upload.jks
     ```
   - 构建：`npx tauri android build --aab`，env 注入 `SOLOSOUL_KEYSTORE_PATH=/tmp/solosoul-upload.jks` 及三个密码变量。
   - 上传 artifact：`actions/upload-artifact@v4`，路径 `tauri/src-tauri/gen/android/app/build/outputs/bundle/release/*.aab`。
2. `ci_cd.yml` 的 `release` job 增加对 Android artifact 的下载与附加（`actions/download-artifact` + 追加到现有 draft pre-release 资产列表），命名规范对齐现有产物：`SoloSoul_<version>_android.aab`。
3. 更新 `docs/release_process.md`：补充 Android 发版步骤与 versionCode 规则（与 P1-03 呼应）。

**验收标准**：

- [ ] master push 后 draft release 中出现已签名 AAB；下载后用 bundletool 验证可安装。

### MOB-P1-06 移动视口 E2E 冒烟测试

- **预估**：2–3 天 ｜ **依赖**：P0-04

**现状**：`tauri/e2e/` 的 Playwright 用例全部按桌面视口编写；`e2e/fixtures/tauriMock.js` 已提供 Tauri IPC mock（含 `autoLockTimeoutMinutes` 等设置项），无移动端项目配置。

**实现步骤**：

1. `tauri/playwright.config.ts` 新增 project：
   ```ts
   { name: 'mobile', use: { viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true } }
   ```
2. 扩展 `tauriMock.js`：mock `@tauri-apps/plugin-os` 的 `platform()` 返回 `'android'`（增加开关，如 `window.__MOCK_PLATFORM__`，桌面用例不受影响）。
3. 编写冒烟用例（复用现有页面对象/fixture 模式）：启动 → 创建账户/解锁 → 首页底部导航渲染 → 对象列表 → 对象详情 → 新建对象保存 → 设置页打开。
4. `package.json` 增加脚本 `"test:e2e:mobile": "playwright test --project=mobile"`；在 `build-android.yml` 或 `pr_check.yml` 中接入（跑在 ubuntu + webkit/chromium 即可，无需 Android 环境）。

**验收标准**：

- [ ] mobile project 冒烟全绿并接入 CI；桌面 project 无回归。

### MOB-P1-07 启动性能基线测量

- **预估**：2 天（需真机）｜ **依赖**：无

**现状**：Notion 的经验是「度量先行」——先定义北极星指标再谈优化。SoloSoul 目前无任何性能埋点。

**实现步骤**：

1. 定义两个指标：**T1 = 点图标 → 解锁页可交互**；**T2 = 解锁成功 → 首页对象列表可见**。
2. 埋点（轻量、不上报）：`main.tsx` 记录 `performance.now()` 起点；登录页可交互（首个输入框 focus 事件）打 T1；首页列表首个对象渲染完成打 T2。用 `tracing`/console 输出即可，格式 `[perf] T1=xxxms T2=xxxms`。
3. 真机采集：至少 1 台低端 Android（≤4GB RAM），冷启动 ×10 取 P50/P95，记录在 `docs/android/android-port-guide.md` 新增「性能基线」一节。
4. 建立预算：T1 P95 ≤ 2.5s、T2 P95 ≤ 1.5s（参考 Notion 启动 1.5–2s 为「偏慢」的历史数据），超标后续任务再议优化。

**验收标准**：

- [ ] 基线数据落盘文档；后续每次移动端大版本发布前复测对比。

---

## 4. P2 — 手机 UI/UX 打磨

> 通用原则（源自 Notion 移动端渲染规则）：**多列塌缩单列、hover 操作改为持久可见或长按、触控目标 ≥44pt、弹层全量安全区适配**。布局判定一律使用 MOB-P0-04 后的 `useIsNarrowViewport` 与 CSS 断点 `max-width: 767px`。

### MOB-P2-01 核心页面响应式补齐

- **预估**：4–6 天（需真机走查）｜ **依赖**：P0-04

**现状**：全库仅约 6 个样式表含 `max-width: 767px` 断点（`AppShell.module.css:28`、`AppBar.module.css:80`、`Button.module.css:113`、`WorkspaceObjectCard.module.css:83`、`PluginDashboardPage.module.css:367` 等），其余 28 个保护路由页面（`tauri/src/App/routes.tsx:43-72`）只有流式布局，窄屏下普遍横向溢出或拥挤。

**实现步骤**（按优先级排序，逐页处理）：

1. **首页** `tauri/src/pages/home/HomePage.tsx` + 对应 `.module.css`：自定义页面卡片栅格改为窄屏 1–2 列；标题/操作区不折行溢出。
2. **对象列表** `tauri/src/pages/workspace/ObjectWorkspacePage.tsx`：搜索框+筛选区竖向堆叠；列表卡片化；批量操作栏改为底部固定操作条。
3. **对象详情** `tauri/src/components/object/ObjectDetailModal.tsx`：窄屏改为全屏页式展示；属性表格键值对改上下排列；敏感字段掩码组件（`SensitiveValueWidget`）按钮区不溢出。
4. **编辑器** `tauri/src/pages/editor/ObjectEditorPage.tsx`：工具栏图标化换行；编辑区 padding 收窄；预览/编辑切换在窄屏下可用。
5. **设置各页**（`SettingsPage`、`SecuritySettingsPage`、`AppearanceSettingsPage`、`ExportImportPage`、`DataManagementPage`、`OperationLogPage`、`BackupConfigPage`、`GlobalAttachmentManager`、`TemplateManagerPage`）：表单控件全宽；长文本省略号。
6. **搜索** `tauri/src/pages/search/SearchPage.tsx` 与历史 `HistoryPage`：结果项多行截断；筛选器横向滚动。
7. 每页完成后用 Chrome DevTools 390×844 与真机各走查一遍（截图存档到 PR）。

**验收标准**：

- [ ] 上述页面在 390px 视口无横向滚动条、无内容遮挡、无文字溢出；真机走查通过。
- [ ] 桌面 ≥768px 布局零变化（截图对比）。

### MOB-P2-02 弹层自适应宽度与全量安全区

- **预估**：1.5 天 ｜ **依赖**：P0-04

**现状**：`tauri/src/components/layout/SearchPopover.module.css:12` 硬编码 `width: 520px`；安全区适配只做了外壳三处（`AppShell.module.css:36-37`、`AppBar.module.css:84`、`MobileBottomNav.module.css`），弹层与 Toast 均未处理 `env(safe-area-inset-*)`。

**实现步骤**：

1. `SearchPopover.module.css:12` 改为 `width: min(520px, calc(100vw - 32px));`，同步检查 `AiQuickChatPopover`、`OcrQuickScanPopover`、`PluginQuickPanel`、`CustomPageEditPopover`、`AddPageButton` 弹层的固定宽度（`grep -rn "width: [0-9]\{3,\}px" tauri/src/**/*.module.css`）。
2. 全局弹层安全区：给 `Dialog`（`tauri/src/components/ui/Dialog.tsx` 及其样式）、所有 popover、Toast 容器加 `padding-bottom: env(safe-area-inset-bottom, 0px);`，顶部贴边的加 `env(safe-area-inset-top, 0px)`。
3. iOS 相关 `viewport-fit=cover` 已存在于 `tauri/index.html:5`，无需改动；Android edge-to-edge 已启用（`MainActivity.kt:14`），验证弹层不被手势条遮挡。

**验收标准**：

- [ ] 全部弹层在 390px 视口完整可见；带手势条的真机上底部按钮不被遮挡。

### MOB-P2-03 hover 依赖消除

- **预估**：2–3 天 ｜ **依赖**：P2-01

**现状**：34 个 CSS 文件共 105 条 `:hover` 规则；触屏无 hover，操作入口（如卡片操作按钮）hover 才显示即等于不可见。已有替代基建：`tauri/src/hooks/useLongPress.ts`（`HomePage.tsx:150` 已用于自定义页面卡片长按编辑）。

**实现步骤**：

1. 分类：`grep -rn ":hover" tauri/src/ --include="*.module.css"`，把每条规则标为「纯视觉增强」（背景/边框高亮，触屏可忽略）或「功能入口」（按钮/菜单仅 hover 显示）。
2. 纯视觉增强：用 `@media (hover: hover) and (pointer: fine) { ... }` 包裹，避免触屏点按后残留「卡住的高亮」。
3. 功能入口：移动端（窄视口）改为持久可见图标按钮，或接入 `useLongPress` 长按弹出操作菜单（与 HomePage 既有模式一致）。
4. 桌面端行为保持不变。

**验收标准**：

- [ ] 窄视口下所有功能入口无需 hover 即可达；触屏点击无高亮残留；桌面 hover 行为不变。

### MOB-P2-04 触控目标尺寸 ≥44pt

- **预估**：1 天 ｜ **依赖**：P2-01

**实现步骤**：

1. 在全局样式（如 `tauri/src/styles/` 下的基础样式或 `AppShell.module.css` 的 767px 断点块）加窄视口规则：按钮、图标按钮、列表项、导航项 `min-height: 44px; min-width: 44px;`（图标按钮可用 padding 达成，避免放大图标本身）。
2. 重点核查 `MobileBottomNav` 项、`ObjectWorkspacePage` 列表项、`ObjectDetailModal` 属性行操作按钮、`Button.module.css` 的 `sm` 尺寸。
3. 真机拇指操作走查：常用路径（列表 → 详情 → 编辑 → 保存 → 删除确认）无误触。

**验收标准**：

- [ ] 窄视口可点击元素均 ≥44×44px（DevTools 审查抽查 + 真机走查）。

### MOB-P2-05 虚拟键盘遮挡回归

- **预估**：1 天（需真机）｜ **依赖**：P2-01

**现状**：`AndroidManifest.xml:39` 已设 `windowSoftInputMode="adjustResize|stateHidden"`，WebView 应随键盘压缩视口；`docs/android/android-port-guide.md:238` 将键盘遮挡测试列为未完成的已知问题。

**实现步骤**：

1. 真机逐页回归：登录页（密码框）、`PasswordVerificationDialog`、对象编辑器（各字段类型）、搜索页/搜索弹层、导出导入页（密码输入）、设置各表单页、`PasswordChangeForm`、PIN 输入。
2. 每页操作：聚焦输入框 → 键盘弹出 → 确认输入框不被遮挡、可滚动到、确认按钮可见；收起键盘后布局还原。
3. 发现问题优先用 CSS 解决（容器 `overflow-y: auto`、底部操作栏 `position: sticky`），不要轻易动 `adjustResize`（全局影响大）。
4. 顺带验证 MOB-P0-01 的语义：密码验证框打开期间自动锁定暂停计时。

**验收标准**：

- [ ] 上述页面键盘场景全部通过；问题清单清零或登记为后续任务。

### MOB-P2-06 桌面专属路由移动端守卫

- **预估**：1 天 ｜ **依赖**：P0-04

**现状**：`/ocr`、`/sync`、`/plugins` 等路由在 `tauri/src/App/routes.tsx:56,65-66` 无条件注册；入口虽已在移动端导航隐藏（`useNavigationItems.ts:202-220`、`SettingsPage.tsx:113-144`），但直接输入路径或深链仍可打开，而这些页面的后端命令在移动端是桩。

**实现步骤**：

1. 在 `tauri/src/App/routes.tsx` 新增 `DesktopOnlyGuard` 组件：
   ```tsx
   function DesktopOnlyGuard({ children }: { children: ReactNode }) {
     const [blocked, setBlocked] = useState<boolean | null>(null);
     useEffect(() => {
       isMobilePlatform().then(setBlocked);
     }, []);
     if (blocked === null) return null;
     if (blocked) return <Navigate to="/" replace />;
     return <>{children}</>;
   }
   ```
   （用异步 `isMobilePlatform()` 而非 `isMobilePlatformSync()`，避免缓存未命中误判。）
2. 包裹路由：`/ocr`、`/sync`、`/plugins`、`/settings/ocr`。后续随功能开放逐个解除（如 P3-02 完成后解除 `/sync`）。
3. 可选：被拦截时 toast 提示「该功能暂不支持移动端」（复用 `useUiStore.showToast`）。

**验收标准**：

- [ ] 移动端直接访问 `/ocr`、`/sync`、`/plugins` 被重定向首页；桌面端路由行为不变；E2E mobile project 加一条断言用例。

### MOB-P2-07 Android 快捷方式「新建对象」

- **预估**：1.5 天（需真机）｜ **依赖**：无

**现状**：无快速捕获入口。对标 Notion 的长按图标新建页——成本极低、收益高。

**实现步骤**：

1. 新建 `gen/android/app/src/main/res/xml/shortcuts.xml`，定义静态 shortcut（`shortcutId="new_object"`，intent action `android.intent.action.VIEW`，targetClass `MainActivity`，携带 extra `shortcut_action=new_object`），图标复用启动器图标或新增 `drawable/ic_shortcut_add`。
2. `AndroidManifest.xml` 的 `MainActivity` 节点内加 `<meta-data android:name="android.app.shortcuts" android:resource="@xml/shortcuts" />`。
3. `MainActivity.kt`：`onCreate` 与 `onNewIntent` 中读取 `intent.getStringExtra("shortcut_action")`，通过 Tauri event 转发到前端（参照 `StatusBarPlugin.kt`/`AttachmentImportPlugin.kt` 与 Rust 的桥接模式，在 Rust 侧 `lib.rs` emit 一个 `quick-capture` 事件，或直接在 Kotlin 调 `webView.evaluateJavascript` 注入自定义 DOM 事件——优先 Tauri event，与现有架构一致）。
4. 前端：`AppRoutes.tsx` 监听 `quick-capture` 事件 → 若未解锁先正常进登录页（登录后再触发），已解锁则 `navigate('/editor?new=1')` 或调起新建对象流程（复用 `AddPageButton`/新建对象的既有入口逻辑）。
5. 冷启动场景验证：App 未运行时点击 shortcut 也能在启动后进入新建流程（事件可能早于前端就绪，前端监听器注册前先缓存事件标志，如写入 `sessionStorage` 再由路由消费）。

**验收标准**：

- [ ] 长按桌面图标出现「新建对象」；热启动与冷启动两种路径均正确到达新建界面；未解锁时不绕过登录。

---

## 5. P3 — 原生能力补齐

### MOB-P3-01 生物识别（tauri-plugin-biometric）

- **预估**：5–8 天（需真机）｜ **依赖**：P1

**现状**：`tauri/src-tauri/src/commands/biometric.rs` 移动端全部返回 `mobile_not_supported()`（如 :88-95、:153-164、:215-225、:266-277）；`tauri/crates/solosoul-core/src/biometric/mod.rs:21-25,136-140` 仅有 macOS Keychain（`security-framework`）与 Windows Hello（`windows` crate）实现，其余平台是 stub。前端 `BiometricSection`（`tauri/src/components/settings/BiometricSection.tsx`）在移动端隐藏。

**实现步骤**：

1. 引入官方插件：`tauri/src-tauri/Cargo.toml` 在 `[target.'cfg(mobile)'.dependencies]` 加 `tauri-plugin-biometric = "2"`；`lib.rs` 注册 `#[cfg(mobile)] { .plugin(tauri_plugin_biometric::init()) }`。
2. 替换桩实现：`commands/biometric.rs` 的 `#[cfg(mobile)]` 分支改为调用插件 API（`status()` 查询可用性、`authenticate()` 发起验证），保持既有 IPC 命令签名不变（`biometric_status`、`biometric_enable`、`biometric_verify` 等），前端零改动。
3. 密钥材料存储（对齐 Anytype 实践）：解锁后的会话密钥/主密码等价物，Android 存 Keystore 加密的 SharedPreferences、iOS 存 Keychain `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`（iOS 部分在 P5-03 验证）。注意：**不存主密码明文**，只存经生物识别门禁保护的派生密钥，锁定/登出时清除。
4. capabilities：在 `tauri/src-tauri/capabilities/default.json`（全平台生效）或新建 `mobile.json`（platform-scoped 到 android/ios，参照现有 `desktop.json` 的写法）中加入 `biometric:default`。
5. 前端：`BiometricSection` 在移动端放开入口；生物识别解锁入口加入登录页（参照桌面 `LoginPage` 的生物识别路径，若桌面未做则在登录页新增按钮，调用 `biometric_verify` 成功后走既有解锁流程）。
6. 边界处理：生物识别硬件变更（新增/删除指纹）后已存密钥失效 → 回退主密码；连续失败锁定 → 系统 BiometricPrompt 自带；无生物硬件 → 设置项隐藏。

**验收标准**：

- [ ] Android 真机指纹/人脸可解锁 Vault；硬件变更后安全回退主密码；`cargo ndk check` 与桌面 `check-all` 全绿。

### MOB-P3-02 设备同步（Android NSD 桥）

- **预估**：8–12 天（需 2 台设备）｜ **依赖**：P1、P2-06（完成后解除 `/sync` 守卫）

**现状**：`tauri/crates/solosoul-sync/src/mobile.rs` 全部为 no-op；`mdns-sd`、`snow`、`x25519-dalek` 均为桌面-only 依赖；6 个 sync 命令与 mDNS discovery 在移动端是桩（`commands/sync.rs:117-210`、`commands/discovery.rs:20-47`）。`mdns-sd` crate 在 Android 上需要 `MulticastLock` 且原生支持不佳。

**实现步骤**：

1. **方案选型（已定为 A）**：A = Kotlin 插件桥接 Android NSD（`NsdManager`），复用 `AttachmentImportPlugin.kt` 的插件模式；B = fork `mdns-sd` 加 MulticastLock 支持（维护成本高，弃用）。
2. 新建 `NsdPlugin.kt`：服务注册（`_solosoul._tcp.`）、发现、解析；结果经插件事件桥到 Rust。
3. Rust 侧：`solosoul-sync` 新增 `#[cfg(target_os = "android")]` 传输/发现实现，**Noise 握手与加密层（`snow`）原样复用**——把 `snow`/`x25519-dalek` 依赖从桌面-only 改为全平台（纯 Rust，无移植障碍），仅 mDNS 发现分叉。
4. 权限：`AndroidManifest.xml` 加 `CHANGE_WIFI_MULTICAST_STATE`、`ACCESS_FINE_LOCATION`（NSD 在 API ≤ 32 需要）与 `NEARBY_WIFI_DEVICES`（API 33+，`usesPermissionFlags="neverForLocation"`）。
5. 同步协议层不变（与桌面互同步是核心验收场景）；前台运行期间工作即可，后台同步不做（与 Anytype 现状一致，见调研报告 §1.5）。
6. UI：解除 `/sync` 路由守卫；`SyncPage` 移动端布局适配（并入 P2-01 清单）。

**验收标准**：

- [ ] Android 真机与桌面端同网互发现并完成一次对象同步；锁屏/切后台恢复后同步可继续；权限拒绝时 UI 有明确提示。

### MOB-P3-03 本地通知场景（锁定/备份提醒）

- **预估**：2–3 天 ｜ **依赖**：P0-02

**现状**：通知基建已通（`notification.ts`），目前仅用于 AI 回复完成。SoloSoul 无服务端，全部使用本地通知，不引入 FCM（保持零云原则）。

**实现步骤**：

1. **自动锁定提醒**（可选，默认关）：`useAutoLock` 锁定触发时发本地通知「Vault 已自动锁定」。注意锁定时前端即将跳转登录页，通知用 `sendNotification` 即发即弃，无需常驻。
2. **备份提醒**：设置项 `backupReminderDays`（默认 7 天）；检查时机为「每次解锁后」——读最近一次备份时间（复用 `commands/backup.rs` 的列表能力），超期则应用内 toast + 系统通知引导到 `/settings/backup`。不做闹钟式定时通知（移动端后台受限，收益低）。
3. 权限申请 UX：不要在启动时弹权限；在各功能首次触发点申请（AI 回复、备份提醒首次启用时）。

**验收标准**：

- [ ] 超期未备份时解锁后出现提醒并可跳转备份页；通知权限拒绝时应用内提示兜底；无后台/推送依赖。

---

## 6. P4 — OCR / Embedding 移动方案（可整体延后）

### MOB-P4-01 模型按需下载

- **预估**：5–8 天（需真机）｜ **依赖**：P0-03

**现状**：桌面端 OCR 模型随包捆绑（30MB），另有首次安装下载流（`OcrInstallBanner`、`ocrInstallStore`、`commands/ocr.rs` 内的下载命令，走 `reqwest`）。embedding 模型（MiniLM，约 23MB）桌面端走下载。移动端两者均为桩。

**实现步骤**：

1. 将 `commands/ocr.rs` 中「模型下载/状态查询」命令的 `#[cfg(mobile)]` 桩替换为真实实现（复用桌面下载逻辑，落盘到 `BaseDirectory::Data` 下 `models/`，移动端可写）。
2. 存储预算与提示：下载前显示模型大小；设置页 `OcrSettingsPage` 移动端显示已用存储与「删除模型」；可选「仅 Wi-Fi 下载」开关（通过 `@tauri-apps/plugin-os` + 简单网络状态判断，或退化为用户确认弹窗）。
3. `MainActivity.extractAssetsToDataDir` 不变（模型不再来自 assets）。
4. 本任务只解决「模型到设备」，不解决推理；OCR 页面入口保持守卫（P2-06）直到 P4-02 完成。

**验收标准**：

- [ ] Android 真机可下载/删除 OCR 与 embedding 模型，断点续传或失败重试可用；存储占用可见。

### MOB-P4-02 移动端推理后端

- **预估**：10–20 天（含 2–3 天 spike）｜ **依赖**：P4-01

**现状**：`ort`、`tokenizers`、`ndarray`、`image`、`pdfium-render` 均为桌面-only 依赖（`src-tauri/Cargo.toml:79-89`、`crates/solosoul-core/Cargo.toml`），`local_embed.rs:36-39,188-205` 移动端桩。

**实现步骤**：

1. **Spike（2–3 天，先做）**：验证 `ort` crate Android 构建（onnxruntime mobile 预编译库 + NNAPI execution provider），用 PP-OCRv6 small 模型在真机跑通一次识别并计时。产出可行性结论，决定主线方案。
2. **主线 A（ort 跨平台）**：若 spike 通过——桌面/移动共享同一套 OCR/embedding Rust 代码，仅依赖按 cfg 区分；iOS 侧用 CoreML EP（P5 阶段验证）。
3. **主线 B（ML Kit 降级）**：若 ort 成本过高——OCR 改用 Kotlin 插件接 ML Kit Text Recognition v2（中文支持良好），结果经桥接回 Rust 走既有导入流程；embedding 降级为桌面-only，移动端搜索退化为关键词匹配。
4. 性能验收：中端真机识别单页 ≤ 3s；识别过程不阻塞 UI（异步 + 进度事件）。
5. 完成后解除 `/ocr`、`/settings/ocr` 路由守卫并做 P2 布局适配。

**验收标准**：

- [ ] spike 报告归档 `docs/android/`；选定主线的识别功能真机可用，或明确的降级方案落地。

---

## 7. P5 — iOS 启动

> 前置：Apple Developer 账号（99 USD/年）、一台 Mac、一台 iPhone 真机（最后验证必需）。

### MOB-P5-01 `tauri ios init` 与工程提交

- **预估**：2–3 天 ｜ **依赖**：P1（Android 发布化经验与 CI 模式复用）

**实现步骤**：

1. `cd tauri && npx tauri ios init` 生成 `src-tauri/gen/apple/`；参考 `gen/android` 的提交策略，将 gen/apple 纳入版本控制并记录定制点。
2. `tauri.ios.conf.json` 检查资源 overlay（与 P0-03 对齐，iOS 同样不打包 OCR 模型）。
3. `npx tauri ios dev` 模拟器启动；修复首批编译问题（常见问题：`fs2`/文件锁、路径权限、`#[cfg(target_os = "ios")]` 缺失——现有代码多用 `#[cfg(mobile)]` 已覆盖 iOS，预期问题集中在沙盒路径与 Keychain API）。
4. `package.json` 增加 `tauri:ios:dev`、`tauri:ios:build` 脚本。
5. CI 增补 iOS `cargo check`（需 macos runner）：`cargo check --target aarch64-apple-ios`。

**验收标准**：

- [ ] 模拟器完成启动 → 创建账户 → 解锁 → 列表/详情/新建对象闭环。

### MOB-P5-02 iOS `attachment_open` 实现

- **预估**：3–5 天（需真机）｜ **依赖**：P5-01

**现状**：`tauri/src-tauri/src/commands/attachment.rs:916-928` 仅 Android 走 Kotlin 插件；`:930-934` 的 `#[cfg(not(target_os = "android"))]` 回退用 `opener` crate——**在 iOS 上会运行期失败**。

**实现步骤**：

1. 新增 iOS 原生插件（Swift）：用 `QLPreviewController`（预览）与 `UIDocumentInteractionController`（分享/打开方式）展示附件，参照 Android `PdfPreviewActivity` + FileProvider 的等价物。
2. `commands/attachment.rs` 增加 `#[cfg(target_os = "ios")]` 分支调用该插件；`opener` 回退限定为桌面。
3. 附件导入（`AttachmentImportPlugin.kt` 的 iOS 对应物）：`UIDocumentPickerViewController` 选文件 → 拷贝进沙盒 → 返回路径，接 `lib/mobileFileTransfer.ts` 既有抽象。
4. 附件导出/分享：系统分享面板。

**验收标准**：

- [ ] iOS 真机附件导入、预览、外部打开、导出分享全链路可用。

### MOB-P5-03 iOS 状态栏/生物识别补齐

- **预估**：3–5 天（需真机）｜ **依赖**：P5-01、P3-01

**实现步骤**：

1. 状态栏：`tauri/src-tauri/src/status_bar_plugin.rs:72-80` 目前 Android-only，新增 iOS 实现（`UIStatusBarStyle` 切换，跟随前端主题，对齐 `useApplyThemeFromSettings.ts` 的既有调用）。
2. 生物识别：P3-01 引入的 `tauri-plugin-biometric` 本身支持 iOS（LocalAuthentication），本任务验证 Face ID/Touch ID + Keychain 存储路径，补齐 `solosoul-core::biometric` 的 iOS cfg 分支。
3. 安全区与键盘：iOS WebView 的 `safe-area-inset` 已由 P2-02 覆盖；键盘遮挡用 iOS 模拟器+真机按 P2-05 清单回归（iOS 的 `adjustResize` 行为不同，重点看编辑器与密码框）。

**验收标准**：

- [ ] Face ID 解锁 Vault；状态栏风格随主题切换；键盘场景无遮挡。

### MOB-P5-04 Apple 签名与 TestFlight

- **预估**：3–5 天（需真机）｜ **依赖**：P5-01

**实现步骤**：

1. Xcode 工程配置：Bundle ID `com.solosoul.app`（与 Android applicationId 对齐）、签名团队、Provisioning Profile（先开发后分发）。
2. `npx tauri ios build` 产出 `.ipa`；本地真机安装调试。
3. App Store Connect 建应用、TestFlight 内测组；上传构建（`xcodebuild -exportArchive` 或 Xcode Organizer）。
4. CI（macos runner）：fastlane 或 xcodebuild 自动构建 + 上传 TestFlight，证书/描述文件用 GitHub Secrets + `match` 类方案管理（参照 Android P1-02/P1-05 的密钥管理模式）。
5. 更新 `docs/release_process.md` 补 iOS 发版流程。

**验收标准**：

- [ ] 内测用户经 TestFlight 安装并跑通核心流程；崩溃-free 会话 ≥ 95%（TestFlight 崩溃报告）。

---

## 8. 总验收清单（DoD）

- [ ] **P0**：通知权限可用；APK 无冗余模型；mobile 判定语义统一。
- [ ] **P1**：CI 全链路（ndsk check / 签名 AAB / release 上传 / mobile E2E）；性能基线落盘。
- [ ] **P2**：390px 视口全页面走查通过；hover/触控/键盘/安全区/路由守卫问题清零。
- [ ] **P3**：指纹解锁、双端同步、备份提醒真机可用。
- [ ] **P4**（可选）：移动端 OCR 可用或明确的降级方案落地。
- [ ] **P5**：TestFlight 内测可装可用。
- [ ] 全程：`npm run check-all` 与 `cargo ndk check` 常绿；文档（port-guide、release_process）同步更新。
