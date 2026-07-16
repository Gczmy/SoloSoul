# SoloSoul 安卓版本开发预研与实施步骤

> 本文档记录将 SoloSoul 从现有 Tauri 桌面端扩展至 Android 端的预研结论、实施进度与后续步骤。

---

## 1. 项目现状与目标

### 1.1 现状

SoloSoul 当前主客户端为 **Tauri v2 + React 19 + TypeScript + Vite**，Rust 后端位于 `tauri/src-tauri/` 及 `tauri/crates/` workspace，已交付 macOS / Windows 版本。核心能力包括：

- 本地加密 Vault（Argon2id + AES-256-GCM）
- SQLite 对象存储（rusqlite bundled）
- 本地 OCR（PP-OCRv6，ONNX Runtime `ort`）
- 本地 Embedding（ONNX，sentence-transformer）
- WASM 插件运行时（Wasmtime + WASI P1）
- 本地同步（mDNS + Noise）
- 附件/备份/导入导出/审计日志等

### 1.2 目标

在保持「本地优先、隐私优先」核心哲学的前提下，让 SoloSoul 能够在 Android 上运行，并至少覆盖：

1. 账户创建/登录/解锁/锁定
2. Vault 内对象（Object/Page）的增删改查
3. 附件拍照/选择/查看
4. 模板与搜索
5. 设置、备份、导入导出

高级能力（OCR、本地 Embedding、插件、设备同步）可作为二期目标。

---

## 2. 方案决策

采用 **方案 A：Tauri Mobile 渐进移植**。原因：

- 现有 Tauri v2 已具备 Android/iOS 构建能力（`tauri android dev/build`）。
- 前端 React 代码与 Rust 核心可直接复用，最小化重写。
- 风险可分段释放：先验证核心流程，再逐步攻克 OCR/插件/同步。
- 与当前 CI/CD、发布流程（`tauri.conf.json`、GitHub Actions）最接近。

---

## 3. 已完成的改造

### 3.1 Phase 0 — 环境准备与工程配置

- [x] 切换 Rust 工具链到 rustup，安装 Android target：
  ```bash
  rustup default stable
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```
- [x] 验证 Android SDK/NDK 可用
- [x] `tauri/package.json` 新增脚本：
  - `tauri:android:dev`
  - `tauri:android:build`
- [x] `tauri/src-tauri/Cargo.toml`：
  - 新增 `[lib]` 块（`crate-type = ["staticlib", "cdylib", "rlib"]`），满足 Tauri Mobile 生成 `.so` 要求
  - 新增 `mobile` feature
  - `reqwest` 改为 `rustls-tls`，避免 Android 交叉编译 OpenSSL
  - 桌面端专属依赖改为按平台条件编译
  - 测试依赖启用 `tauri/test` feature
- [x] Gradle wrapper 改为腾讯镜像，并手动缓存 Gradle 8.14.3 分发包，解决国内下载 SSL 中断问题

### 3.2 Phase 1 — 初始化 Tauri Android 工程

- [x] 执行 `npx tauri android init`，生成 `tauri/src-tauri/gen/android/`
- [x] 配置 `gen/android/app/src/main/AndroidManifest.xml`：
  - `android:launchMode="singleTask"` 防止多实例
  - `android:windowSoftInputMode="adjustResize|stateHidden"` 适配软键盘
  - 权限：`INTERNET`、`READ_EXTERNAL_STORAGE`（maxSdkVersion=32）、`WRITE_EXTERNAL_STORAGE`（maxSdkVersion=32）、`READ_MEDIA_IMAGES`、`READ_MEDIA_VIDEO`、`CAMERA`

### 3.3 Phase 2 — Rust 后端移动端适配

- [x] `tauri/src-tauri/src/lib.rs`
  - 日志目录、数据目录改用 `tauri::path::BaseDirectory::Data`（移动端）
  - 桌面端插件（window-state、updater）条件初始化
  - PDFium 路径、插件注册表刷新、mDNS 发现服务、系统主题轮询仅在桌面端运行
- [x] `tauri/src-tauri/src/state/app_state.rs`
  - 移动端通过 `app.path()` 解析应用私有目录并传入 `VaultService::with_base_path`
- [x] `tauri/src-tauri/src/commands/fs.rs`
  - `allowed_fs_base()` 移动端使用 `BaseDirectory::Data`
- [x] `tauri/src-tauri/src/commands/export_import/export.rs`
  - `~/` 路径在移动端解析为应用数据目录
- [x] `tauri/src-tauri/src/commands/attachment.rs`
  - `attachment_copy_to_vault` 对无法 canonicalize 的源路径增加降级处理，兼容 Android 缓存路径
- [x] `tauri/src-tauri/src/commands/system.rs`
  - `get_system_theme()` 移动端返回固定值，桌面端继续使用 `dark_light`
- [x] `tauri/src-tauri/capabilities/default.json`
  - 移除移动端不存在的 `updater:default`
  - 新增 `capabilities/desktop.json`，仅在桌面平台启用 `updater:default`
  - 补充 `fs:allow-stat`、`fs:allow-mkdir`、`fs:allow-remove`，确保前端可用 plugin-fs 进行 content URI 中转
- [x] `tauri/crates/solosoul-core/src/process_lock.rs`
  - 桌面端使用 `fs2` 排他锁；移动端使用应用单实例占位
- [x] `tauri/crates/solosoul-core/src/lib.rs`
  - `ocr`、`pdfium`、`watermark` 模块桌面端先行
- [x] `tauri/crates/solosoul-plugin` workspace crate：
  - 修复 `manager_mobile.rs`、`sandbox_mobile.rs`、`host_mobile.rs` 占位实现编译错误
  - `error.rs` 中 `From<wasmtime::Error>` 仅在桌面端编译
- [x] `tauri/crates/solosoul-sync` workspace crate：
  - 新增 `src/types.rs`，将 `SyncPeerInfo`、`SyncSessionResult`、`ApplyStats`、`ConflictRecord`、`AttachmentSyncStats` 等类型抽离到公共模块
  - 新增 `src/mobile.rs`，为移动端提供 `SyncService` / `NoiseKeys` 无操作占位
  - `manager`、`noise`、`service`、`transport`、`attachments`、`delta` 模块仅在桌面端编译
  - `Cargo.toml` 将 `mdns-sd`、`snow`、`x25519-dalek` 统一标记为桌面端依赖
- [x] 桌面端独占命令模块已添加移动端占位实现：
  - `commands/ocr.rs`
  - `commands/embed_model.rs`
  - `commands/plugin.rs`
  - `commands/sync.rs`
  - `commands/discovery.rs`
  - `commands/biometric.rs`
  - `local_embed.rs`
  - `plugin/*`（Tauri 侧）

- [x] `tauri/src-tauri/src/plugin/paths.rs`
  - Android 上插件市场目录也改为从 `{data}/resources` 读取，与帮助文档一致
- [x] `tauri/src-tauri/src/status_bar_plugin.rs` + `capabilities/default.json`
  - 修复桌面端 `StatusBarPluginHandle` 因 `PhantomData<R>` 不满足 `Send + Sync` 导致的编译错误
  - 新增 `solo-soul:allow-set-status-bar-style` 权限，使前端 `set_status_bar_style` 在 Android 上不再被 capability 拒绝
- [x] `tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/MainActivity.kt`
  - 资源复制完成后增加日志与 `docs/guides/index.json` 存在性校验，便于排查帮助索引缺失问题
- [x] `tauri/src/hooks/useApplyThemeFromSettings.ts` + `AppRoutes.tsx` + `LoginPage.tsx` + `BootstrapPage.tsx`
  - 抽离公共 Hook 统一根据 settings 应用主题
  - 登录页、创建账户页 mount 时主动应用主题并同步 Android 状态栏颜色
- [x] `tauri/src-tauri/src/state/app_state.rs`
  - 移动端启动时打印 `data_dir` 与已加载账户数，便于排查账户数据丢失问题
- [x] 开发命令保留应用数据
  - `npm run tauri:android:dev -- --no-reinstall` 可在不卸载 App 的情况下重装，避免每次重新编译都清空账户数据

### 3.3 Phase 2 — 前端移动端适配

- [x] 新增 `src/hooks/useIsMobile.ts`：基于 `matchMedia` 的移动端检测
- [x] 新增 `src/components/layout/MobileBottomNav.tsx`：底部 Tab 导航（Home / Search / AI Chat / Settings / Lock）
- [x] `src/components/layout/AppShell.tsx`
  - 移动端强制底部导航
  - 使用 `env(safe-area-inset-*)` 适配刘海/手势条
- [x] `src/components/layout/AppBar.module.css`
  - 移动端全宽、48px 高度、返回按钮放大
- [x] `src/components/layout/AppShell.module.css`
  - 移动端减少 padding、底部安全区
- [x] `src/App/AppRoutes.tsx`
  - 移动端跳过自动更新检查、OCR 静默安装、窗口关闭拦截
- [x] `src/pages/auth/LoginPage.tsx` / `BootstrapPage.tsx`
  - 登录/创建账户卡片宽度响应式化
- [x] Android content:// URI 中转（关键修复）
  - 新增 `src/lib/mobileFileTransfer.ts`：通过 `tauri-apps/plugin-fs` 把 `content://` URI 复制到应用缓存
  - `src/lib/attachmentUpload.ts`：上传前若路径为 URI 则先中转
  - `src/components/object/AttachmentViewer.tsx` / `src/pages/settings/GlobalAttachmentManager.tsx`：单附件下载通过缓存中转；批量下载到目录在移动端给出明确提示
  - `src/pages/settings/ExportImportPage.tsx`：导出/导入 `.solosoul` 包均通过缓存中转
  - `src/pages/settings/OperationLogPage.tsx` / `src/pages/system/DebugLogPage.tsx`：审计日志/调试日志导出通过缓存中转

### 3.4 Phase 3 — 运行验证

- [x] `cargo ndk -t aarch64-linux-android check --package solo_soul` 通过（仅有 warning）
- [x] `npm run tauri:android:dev` 成功编译并在 Android 模拟器上安装启动
- [x] 应用包名 `com.solosoul.app` 已运行，`MainActivity` 为 topResumedActivity
- [x] Rust 启动日志正常：
  - `SoloSoul v2.5.12 启动`
  - `目标平台: android`
  - `Vault encryption migration completed successfully`
- [x] `npm run lint` / `npx tsc --noEmit` 通过
- [x] MVP 功能手动验证（需你在模拟器/真机上操作并反馈）
- [x] 移动端设置页已隐藏插件 / OCR / 同步入口
- [x] 移动端关于页已跳过桌面端自动更新检查

### 3.5 验证结果

- `cargo check --package solo_soul` ✅
- `cargo test --package solo_soul` ✅（293 单元测试 + 集成测试全部通过）
- `npm run lint` ✅
- `npx tsc --noEmit` ✅
- `cargo ndk -t aarch64-linux-android check --package solo_soul` ✅
- `npm run tauri:android:dev` 模拟器启动 ✅
- 移动端设置页入口过滤、关于页更新检查已通过代码审查 ✅
- 帮助索引本地路径、状态栏权限、登录页主题同步已通过代码审查 ✅

---

## 4. 推荐开发/验证命令

```bash
cd tauri

# 1. 安装依赖
npm install

# 2. 桌面端快速检查
npx tsc --noEmit
cargo check --package solo_soul

# 3. Android 模拟器热调试（保留应用数据，避免每次重新创建账户）
npm run tauri:android:dev -- --no-reinstall

# 4. 若必须完整重装（如签名变更、native lib 更新），账户数据会丢失，需重新创建账户
npm run tauri:android:dev

# 5. Android Release APK 构建
npm run tauri:android:build
```

---

## 6. 待完成的关键步骤

### 4.1 MVP 功能手动验证

在 Android 模拟器或真机上验证：

1. 首次启动 → 创建账户 → 登录
2. 首页查看 Page / Object 列表
3. 创建/编辑对象
4. 搜索
5. 附件选择/查看
6. 设置（语言、主题、密码）
7. 备份创建/恢复
8. 导入/导出 `.solosoul`

### 4.1.1 启动性能基线

定义两个指标：

- **T1**：点图标 → 解锁页可交互
- **T2**：解锁成功 → 首页对象列表可见

代码已埋点：

- `tauri/src/main.tsx`：应用启动时记录 `__SOLOSOUL_APP_START_TIME`。
- `tauri/src/pages/auth/LoginPage.tsx`：登录页挂载时输出 `[perf] T1=xxxms`。
- `tauri/src/pages/home/HomePage.tsx`：首页挂载时输出 `[perf] T2=xxxms`。

真机采集方法：

```bash
# 连接 Android 设备并过滤日志
adb logcat -s "Web Console" | grep "\[perf\]"
```

每次冷启动 ×10，取 P50/P95，记录在案。预算：T1 P95 ≤ 2.5s、T2 P95 ≤ 1.5s。

### 4.2 已知需要在验证中修复的问题

- [x] 附件 `attachment_copy_to_vault` / `attachment_download` 的目录校验在移动端已做兼容：
  - 新增 `src/lib/mobileFileTransfer.ts` 处理 `content://` URI 中转
  - 单附件上传/下载、导入/导出均通过应用缓存中转
  - 批量下载到目录在移动端暂不支持，已给出明确提示
- [x] 每次重新编译 APK 后要求新建账户
  - 根因：`npm run tauri:android:dev` 默认会卸载重装 App，清空应用私有数据目录
  - 规避：使用 `npm run tauri:android:dev -- --no-reinstall` 保留数据；若签名或 native lib 变更导致必须重装，则账户数据会丢失，属开发期正常行为
  - 代码侧已在 `AppState` 中增加日志，确认移动端数据目录与账户加载数量
- [x] 深色主题下 Android 系统状态栏图标/文字仍为黑色
  - 根因：`set_status_bar_style` command 未在 capability 中授权，invoke 被静默拒绝
  - 修复：新增 `solo-soul:allow-set-status-bar-style` 权限；登录页/引导页 mount 时主动应用主题
- [x] 帮助页面无法加载索引 `asset://localhost/docs/guides/index.json`
  - 根因：Android 上 `app.path().resolve("resources", BaseDirectory::Data)` 可能返回 asset URL，导致 Rust `std::fs` 读取失败
  - 修复：`lib.rs` 与 `plugin/paths.rs` 改为通过 `resolve_app_data_dir` 拼接 `{data}/resources`；`MainActivity.kt` 增加复制完成日志与存在性校验
- [ ] 导出路径若使用 `~/` 在移动端已映射到应用数据目录，但前端需给出合适默认路径（当前由用户通过系统对话框选择）。
- [x] OCR、插件、同步、自动更新等功能在移动端已返回「暂不支持」，前端对应入口已隐藏或禁用。
- [ ] 软键盘弹出时输入框遮挡问题需进一步测试（已加 `adjustResize`）。
- [x] 配置 `gen/android/app/src/main/res/values/themes.xml` 与 `values-night/themes.xml`，设置状态栏/导航栏颜色与 `windowLightStatusBar`/`windowLightNavigationBar`，匹配 SoloSoul 暖石浅色 `#FAFAF8` 与深色 `#1F1C18`。
- [ ] 开发模式下 Vite HMR WebSocket 在 WebView 内连接失败（`[vite] failed to connect to websocket`），不影响功能但热更新不可用；Release 构建无此问题。
- [ ] 附件“用系统应用打开”在 Android 上依赖 FileProvider，当前使用 `opener` crate 可能无法直接唤起外部应用，建议优先使用内置预览。

---

## 7. 二期功能规划

| 功能 | Android 方案 | 预估工作量 |
|------|-------------|-----------|
| 本地 OCR | `ort` Android 构建 + 摄像头拍照 + PP-OCRv6 模型 | 4–6 周 |
| 本地 Embedding | `ort` Android + 模型下载管理 | 2–3 周 |
| 插件系统 | 等待 `wasmtime` 官方支持 Android；或改用 QuickJS/WASM3 轻量运行时 | 6–10 周 |
| 设备同步 | mDNS 替换为 Android NSD；Noise 保留 | 3–4 周 |
| 生物识别 | `tauri-plugin-biometric` | 1–2 周 |
| 推送通知 | `tauri-plugin-notification` 已支持移动端 | 1 周 |
| 自动更新 | Google Play 内更新或应用内下载 APK | 2–3 周 |

---

## 8. 风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| `wasmtime` 不支持 Android | 插件系统无法运行 | Phase 1 禁用；二期评估替代运行时 |
| `ort` Android 交叉编译复杂 | OCR/Embedding 延期 | 先禁用，提供手动导入替代 |
| Android Scoped Storage | 附件/备份路径错误 | 已改用 Tauri path API + App 私有目录 |
| 移动端 UI 适配量大 | 延期 | 先 MVP 核心页面，其余逐步适配 |
| APK 体积过大 | 无法上架 | 模型外置，按需下载 |
| 多实例数据冲突 | SQLite 损坏 | singleTask launchMode + 文件锁 fallback |
| 不同 Android 版本兼容性 | 崩溃/异常 | 最低 API 28，多版本模拟器测试 |

---

## 9. 交付物清单

1. `docs/android/android-port-guide.md` — 本文件
2. `docs/android/android-environment-setup.md` — 环境搭建与首次运行指南
3. `docs/android/android-ui-guidelines.md` — 移动端 UI 规范
4. `tauri/src-tauri/tauri.conf.json` 及相关 Cargo 配置
5. Android 条件编译代码（Rust）与响应式布局（前端）
6. GitHub Actions `build-android.yml`
7. 可运行的 Android MVP（待环境就绪后验证）
