# 移动端技术调研报告：Anytype、Notion 与 SoloSoul 的移动端优化路径

> 调研日期：2026-07-16。本报告基于 Anytype / Notion 公开工程资料（官方博客、开源仓库、播客访谈）与 SoloSoul 代码库事实分析，所有外部事实均附来源链接。

## 0. 摘要

- **Anytype**：「一次编写核心，三次重写 UI」——Go 核心（anytype-heart）通过 gomobile 编译进 App，iOS（Swift/SwiftUI）与 Android（Kotlin/Compose）UI 完全独立实现，连块编辑器都是双端各自原生写的。成本极高，换来了极致的原生体验与本地优先能力。
- **Notion**：「Web 内容 + 渐进原生化」——从 Cordova WebView 壳出发，用 4 年时间把编辑器以外的一切（导航、搜索、收件箱、数据层）替换为原生；编辑器至今仍是 WebView 里的共享 Web 应用。用 11 人的小团队、度量驱动的性能工程和周发布纪律支撑数千万移动用户。
- **SoloSoul**：处于两者之间且先天有优势——Rust workspace 核心天然跨平台编译（比 Anytype 的 gomobile 绑定更省事），前端本来就是 Web 技术（Tauri Mobile 直接提供 WebView 容器，**不需要像 Anytype 那样重写编辑器，也不需要像 Notion 那样做 4 年迁移**）。Android 移植已完成 Phase 0–3，主要缺口在：发布化（签名/AAB/CI）、被桩掉的功能（OCR/embedding/sync/biometric）、响应式 UI 薄弱、以及一个疑似全平台缺失的自动锁定实现。

**核心结论：SoloSoul 的正确路线是「单一 Rust 核心 + 单一 Web 前端 + 响应式适配 + 原生能力按需桥接」，而不是效仿 Anytype 的全原生 UI。**

---

## 1. Anytype 的移动端技术栈

### 1.1 总体架构：共享 Go 核心 + 双端原生 UI

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  iOS App    │  │ Android App │  │ Desktop App │
│ Swift/SwiftUI│  │Kotlin/Compose│  │Electron + TS│
│ (+UIKit编辑器)│  │(+View编辑器) │  │             │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │ gomobile       │ gomobile       │ gRPC
       │ (in-process)   │ (in-process)   │ (localhost)
┌──────┴────────────────┴──────┐  ┌──────┴──────┐
│      anytype-heart (Go)      │  │ anytypeHelper│
│  存储/加密/CRDT/同步/块模型   │  │  (同一核心)  │
└──────────────────────────────┘  └─────────────┘
```

- **核心仓库**：`anyproto/anytype-heart`（Go 1.25），负责账户/ACL、块文档模型、对象存储、全文搜索、历史版本、聊天、导入导出、any-sync 客户端。移动端通过 **`gomobile bind`** 编译：iOS 产出 `Lib.xcframework`（`-target=ios`），Android 产出 `lib.aar`（`-target=android -androidapi 26`），以 Maven artifact `io.anyproto:anytype-heart-android` 分发。构建 tag `nogrpcserver gomobile` 表明移动端**进程内运行核心，无 gRPC server**。
- **IPC 契约是 protobuf**：命令与事件定义在 `pb/protos`；iOS 用 swift-protobuf + 自研 codegen 生成 Swift 绑定，Android 用 Square Wire 生成 Kotlin，桌面端用 ts-proto。同一套 proto，三种语言绑定，移动端是「进程内函数调用 + 事件流」，桌面端是真 gRPC。
- 来源：[anytype-heart makefiles](https://raw.githubusercontent.com/anyproto/anytype-heart/main/makefiles/ios.mk)、[Build.md](https://raw.githubusercontent.com/anyproto/anytype-heart/main/docs/Build.md)、[anytype-kotlin README](https://github.com/anyproto/anytype-kotlin)

### 1.2 iOS 端

- Swift + **SwiftUI** 为主，Combine 做响应式；架构为 **MVVM + Coordinator + Repository**，DI 用 Factory 库；SPM 多模块（`AnytypeCore`、`Services`、`ProtobufMessages`、`SecureService` 等）。要求 iOS 17+。
- **关键例外：文档编辑器是 UIKit**——`EditorCollectionView: UICollectionView` 承载块列表，每个文本块是自定义 `UITextView`（直接操作 `textStorage`/`NSAttributedString`）。即「新界面用 SwiftUI，重度文本编辑用 UIKit」的务实混合。
- App Store 体积 272.5 MB，支持 iPhone/iPad/visionOS。
- 来源：[anytype-swift CLAUDE.md](https://raw.githubusercontent.com/anyproto/anytype-swift/develop/CLAUDE.md)、[EditorCollectionView.swift](https://raw.githubusercontent.com/anyproto/anytype-swift/develop/Anytype/Sources/PresentationLayer/TextEditor/EditorPage/EditorCollectionView.swift)

### 1.3 Android 端

- Kotlin + **Jetpack Compose（Material 3）** 为主，MVVM + coroutines/Flow，Dagger 2 DI，重度多模块（`app/data/domain/presentation/core-ui/persistence/protocol/middleware/feature-*`）。minSdk 26。
- **编辑器同样走经典 View 体系**：`RecyclerView` + `BlockAdapter` + DiffUtil，每种块类型一个 ViewHolder；富文本是大量自定义 Spannable 工作（`MentionSpan`、`SlashTextWatcher`、`MarkupMapper` 等）。
- 其他：Room、DataStore、Glance（桌面小组件）、Coil3、ExoPlayer、FCM 推送、ML Kit 扫码（QR 登录）、`androidx.security:security-crypto`。
- 来源：[anytype-kotlin libs.versions.toml](https://raw.githubusercontent.com/anyproto/anytype-kotlin/main/gradle/libs.versions.toml)、[TextBlockHolder.kt](https://github.com/anyproto/anytype-kotlin/main/core-ui/src/main/java/com/anytypeio/anytype/core_ui/features/editor/TextBlockHolder.kt)

### 1.4 数据与同步

- **any-sync 协议**：每个对象是一条加密 CRDT 变更 DAG，Ed25519 签名，多头合并；冲突在设备本地解决，无需服务器共识。已生产运行约 4 年，经 Cure53 审计。
- 同步通道：局域网 **mDNS P2P**（Android 需 `NEARBY_WIFI_DEVICES` + `CHANGE_WIFI_MULTICAST_STATE` 权限）+ 官方备份节点（免费 1GB 文件备份）+ 可自建节点 + 可纯本地模式。双层加密：备份节点只见外层，内容层密钥永不出设备。
- **存储**：自研 [any-store](https://github.com/anyproto/any-store)——SQLite 单文件上的 MongoDB 风格文档库（纯 Go `modernc.org/sqlite`，无 CGO，适合 gomobile）；文件经私有 IPFS 分片加密存储；**媒体不主动同步到手机，按需拉取并缓存**（可一键清理/卸载）。全文搜索用 Tantivy（Rust 编译库）；**本地搜索索引未加密**，官方建议依赖系统盘加密。
- **密钥**：主密钥 = 12 词 BIP-39 助记词；iOS 存 Keychain（`ThisDeviceOnly`，不进 iCloud），Android 存 EncryptedSharedPreferences（主密钥在 AndroidKeyStore）。未使用 Secure Enclave。
- 来源：[any-sync overview](https://tech.anytype.io/any-sync/overview)、[安全文档](https://doc.anytype.io/anytype-docs/advanced/data-and-security/how-we-keep-your-data-safe)、[数据存储文档](https://doc.anytype.io/anytype-docs/advanced/data-and-security/data-storage-and-deletion)

### 1.5 移动端特有适配

- iOS：Share Extension、桌面小组件、**Notification Service Extension（在设备端解密推送 payload）**——代码注释明确记载：扩展有 ~24MB 内存上限，无法链接 `Lib.xcframework`（注释称 ~779MB，含全架构+符号），只能重写一个轻量解密服务。**这是「重核心嵌入移动端」的标志性代价**。
- Android：分享接收（SEND/SEND_MULTIPLE）、Glance 小组件、FCM；`allowBackup="false"`、自定义 networkSecurityConfig、锁定竖屏。
- 值得注意的「没有做」：代码搜索未发现 iOS BGTaskScheduler 或 Android WorkManager——**同步基本只在 App 前台运行时进行**；双端均未发现生物识别解锁（靠 Keychain/Keystore 里已存的助记词直接解锁，体验上等效）。
- 发布节奏极高（近乎月度功能版 + 每日 dev tag）；已知平台差异：graph view 仅桌面端、部分视图移动端降级。
- 来源：[NotificationService.swift](https://raw.githubusercontent.com/anyproto/anytype-swift/develop/AnytypeNotificationServiceExtension/NotificationService.swift)、[AndroidManifest.xml](https://raw.githubusercontent.com/anyproto/anytype-kotlin/main/app/src/main/AndroidManifest.xml)

### 1.6 Anytype 策略小结

| 决策 | 选择 | 代价/收益 |
|---|---|---|
| 核心复用 | Go 核心 gomobile 嵌入 | 收益：逻辑单点维护；代价：二进制体积、扩展场景受限 |
| UI | 双端各自原生（SwiftUI/UIKit、Compose/View） | 收益：极致体验；代价：三套 UI（含桌面）并行开发，小团队不可承受 |
| 编辑器 | 双端原生实现 | 收益：性能与系统文本能力；代价：最高的单点开发成本 |
| 同步 | 自研 CRDT + P2P/备份节点 | 本地优先最彻底，工程投入巨大 |

---

## 2. Notion 的移动端技术栈

### 2.1 演进史：WebView 壳 → 四年渐进原生化

- **2017–2018**：初版 App 是 **Apache Cordova** 壳，整体就是移动版 Web 应用的 WebView 包装；后迁到 React Native，但「不是真正的 RN」——只用 RN 的 OS hook（震动等），主体仍是单个 WebView 组件渲染 Web 应用。启动即加载整个 app.js，耗时 1.5–2 秒，被用户诟病。
- **2019**：决定原生化，初始团队仅 **4 人（2 iOS + 2 Android）**。明确不搞大爆炸重写：「我们没有重写整个 App 的奢侈条件，只能一块一块迁移」。也评估并否决了 React Native 全量方案：「不相信它能带我们走到想去的地方」。
- **~2020–2021**：**SQLite 客户端缓存层先在移动端上线**（后来才反哺桌面和浏览器 WASM SQLite），初始页面加载提速 50%。2021 年底确立愿景：「**除了编辑器，一切原生**」。
- **2022**：**Home Tab**（移动端侧栏等价物：收藏/共享/团队空间）上线——首个也是最难的原生界面，耗时约 9 个月，从零写了原生网络层/服务层，带来 **3× 感知启动提升**。
- **2023 初**：**Search Tab 原生化**，加载提速 80%+；随后 Inbox 原生，并开始把部分富文本块（代码、提及）原生渲染——首次触碰编辑器。
- **至今**：**编辑器仍是 Web**。阻碍是 collections（页内数据库）和 embeds——「在原生滚动视图里嵌多个 WebView 非常复杂」。编辑器与原生层通过 WebView **Message Ports**（JSON IPC）通信。
- 原话：「端到端，整个过程基本花了四年。」（Pragmatic Engineer 播客，2024-12）
- 来源：[Pragmatic Engineer: Notion going native](https://newsletter.pragmaticengineer.com/p/notion-going-native-on-ios-and-android)、[Android 提速两倍](https://www.notion.com/blog/notion-on-android-is-now-more-than-twice-as-fast-to-launch)、[页面加载提速](https://www.notion.com/blog/faster-page-load-navigation)

### 2.2 当前技术栈（2024-12 口径）

- **iOS**：纯 Swift；SwiftUI + Combine；SQLite 为主持久层；**约 100 个 SPM 模块**。
- **Android**：Kotlin；Jetpack Compose + Flow；Dagger DI；SQLite；**约 50 个 Gradle 模块**。
- **跨端共享不靠 KMP/RN/Rust**，靠两样：(a) WebView 里跑的共享 TypeScript Web 应用（编辑器/页面内容）；(b) **刻意镜像的双端原生架构**——服务/模块在 iOS 与 Android 一一对应，文件结构雷同，双端工程师结对同步开发。
- 混合架构的版本难题：原生包周三打包时快照一份 Web bundle，而 Web bundle 后台持续更新——Web 层最多可比原生层新一周，是显式管理的工程问题。

### 2.3 性能工程（度量驱动，官方数字）

北极星指标：`initial_home_render`（点图标→首页内容可见），生产环境盯 **P95**：

- **缓存用户会话**（用户+workspace 在两次打开间稳定）：单项带来 **~30%** 启动提升。
- **SQLite 迁移检查修复**：迁移元数据曾打成一个大 JSON 每次启动解析（215ms），改为独立整数版本号、仅在需要时加载。
- **JSON 序列化移出主线程**（WebView Message Port 流量缓冲+并行化）。
- **Baseline Profiles**（Compose AOT）：首版 **~12% P95** 提升，每个 release 在 Firebase Device Lab 真机重新生成。
- **Macrobenchmark 跑在每个 PR 上**，自定义 Trace 分段，指标周会 review。
- **磁盘不是永远更快**：低端 Android 上 SQLite 读可能慢于网络，客户端让**磁盘读与网络请求竞速**，谁先用谁。
- 成果：Android 启动比 2023 初快 2 倍以上。
- 来源：[Android 性能长文](https://www.notion.com/blog/notion-on-android-is-now-more-than-twice-as-fast-to-launch)、[WASM SQLite](https://www.notion.com/blog/how-we-sped-up-notion-in-the-browser-with-wasm-sqlite)

### 2.4 本地数据层与离线模式

- **事务模型**：编辑 = transaction，**先写本地 SQLite**，由单一 transactions API 定期 flush，sync API 拉变更——两个端点承担绝大部分流量。
- **离线模式（2025-08，v2.53）**：把「尽力而为」的缓存升级为持久层：`offline_page` + `offline_action` 两张表，记录每页离线的**原因**（手动标记/收藏继承/数据库前 50 行），原因消失才下线；重连时按 `lastDownloadedTimestamp` 增量拉取；**文本冲突走 CRDT 自动合并，非文本冲突 last-write-wins**。限制：子页面不递归下载、数据库仅首视图前 50 行、embeds/AI/表单离线不可用、下载按设备独立。
- 来源：[How we made Notion available offline](https://www.notion.com/blog/how-we-made-notion-available-offline)、[2.53 release notes](https://www.notion.com/releases/2025-08-19)、[help: offline](https://www.notion.com/help/use-pages-offline)

### 2.5 移动端 UX 适配与团队流程

- **底部 Tab 导航**（Home/Search/Inbox，2026 年已演进为四 Tab）；**小组件**（iOS 14+ / Android，含快速新建页、AI 入口）；快速捕获（长按图标新建、Siri、Spotlight、Action Button）；**移动端渲染规则**：无 hover（操作按钮持久显示）、**多列布局塌缩为单列**、部分桌面操作移动端禁用。
- 团队：移动工程师 **11 人 / 全司约 600 人**，刻意保持小而资深；**每周发布**（周三切版，周五 1% 灰度起步），TestFlight/Play Store **公开 nightly beta**；同时在线 feature flag 数以百计；四套环境（local/dev/staging/prod）以独立 App 包分发。移动用户数千万，**约一半新 workspace 在移动端创建**。
- 来源：[help: mobile widgets](https://www.notion.com/help/mobile-widgets)、[help: Notion for mobile](https://www.notion.com/help/notion-for-mobile)、[releases 2026-05-04](https://www.notion.com/releases/2026-05-04)

### 2.6 Notion 策略小结

| 决策 | 选择 | 启示 |
|---|---|---|
| 内容渲染 | 编辑器留在 WebView，共享 Web 代码 | Web 技术栈团队的最短路径 |
| 原生化 | 按「用户感知价值/实现成本」排序渐进替换 | 启动路径 > 导航 > 搜索 > 编辑器 |
| 数据层 | 本地 SQLite 先写，缓存与网络竞速 | 感知性能主要来自数据层而非渲染层 |
| 流程 | 小团队、周发布、灰度、flag | 度量先行，没有指标就没有优化 |

---

## 3. 对比分析：两条路线与 SoloSoul 的位置

| 维度 | Anytype | Notion | SoloSoul 现状 |
|---|---|---|---|
| 核心逻辑载体 | Go 核心，gomobile 嵌入 | 服务端权威 + 双端镜像原生服务层 | **Rust workspace，各平台直接编译（最省事）** |
| UI 技术 | 双端全原生（3 套 UI） | 原生壳 + WebView 编辑器（混合） | **单一 Web 前端 + Tauri WebView（1 套 UI）** |
| 编辑器 | 双端原生重写 | 至今 Web | 无需重写，WebView 直接可用 |
| 数据模型 | 本地优先 CRDT，彻底离线 | 云端权威，2025 才补离线 | **本地优先加密 SQLite，天然离线** |
| 密钥管理 | 助记词 + Keychain/Keystore | 服务端会话 | 主密码 + Argon2id，生物识别仅 macOS/Windows |
| 移动端完成度 | 全功能（少数降级） | 全功能（编辑器 Web） | **Android MVP 可用，功能桩 + 发布化未完成** |

对 SoloSoul 的关键启示：

1. **不要走 Anytype 路线**。三套原生 UI 与双端原生编辑器是 Anytype 用约 30 人级移动端投入换来的，SoloSoul 的规模不具备复制条件；且 Tauri Mobile 的 WebView 容器让「前端不重写」成为现实，编辑器零成本移植。
2. **走「Notion 数据层思想 + Tauri 壳」路线**。Notion 的经验证明：移动端感知性能的胜负手是**本地数据层**（先写本地、缓存竞速、会话缓存），而非渲染层。SoloSoul 本地优先的架构在这一点上**先天领先 Notion**（Notion 2025 年才把离线补上，SoloSoul 出生即离线）。
3. **原生桥接做薄、按需扩展**。Anytype 的重核心教训（779MB xcframework 进不了扩展进程）对应 SoloSoul 的现实问题：52MB 的 OCR/embedding 模型不能进 APK。原生层只做 WebView 做不到的事（生物识别、状态栏、content://、后台任务），其余留在 Rust + Web。
4. **度量先行**。Notion 的一切优化始于 `initial_home_render` P95。SoloSoul 应尽早定义自己的启动与交互预算并在真机测量，而不是凭感觉优化。

---

## 4. SoloSoul 现状评估（基于代码库事实）

### 4.1 已完成（Android 移植 Phase 0–3，见 `docs/android/android-port-guide.md`）

- **系统化的平台门控**：Rust 全库 `#[cfg(desktop)]`/`#[cfg(mobile)]` 分支 + 统一的 `mobile_not_supported()` 桩（`tauri/src-tauri/src/commands/mod.rs:55-64`），IPC 签名不变，前端不分叉。
- **Android 工程已生成并提交**：`tauri/src-tauri/gen/android/`，含定制 Kotlin 插件——`StatusBarPlugin`、`AttachmentImportPlugin`（content:// URI 桥）、`PdfPreviewActivity`；`MainActivity` 负责把 APK assets 中的资源复制到 dataDir（`lib.rs:237-249`）。
- **依赖选型已对移动端友好**：`rusqlite` bundled、`reqwest` rustls-tls、wasmtime 用 `pulley` 解释器（全平台）；`mobile` feature + `tauri::mobile_entry_point` + staticlib/cdylib 均已就位。
- **前端移动基础设施**：`viewport-fit=cover`、平台检测（`lib/platform.ts`、`useIsMobile`）、底部导航壳（`AppShell`/`MobileBottomNav`）、部分安全区适配、`useLongPress`、content:// 文件传输层（`lib/mobileFileTransfer.ts`）、桌面功能在移动端隐藏。
- CI 已有 `build-android.yml`（debug APK）。

### 4.2 缺口（全部为有意桩掉或未开始，非意外损坏）

- **功能桩**：OCR 10 个命令、`local_embed`、6 个 sync 命令、mDNS discovery、生物识别（macOS Keychain/Windows Hello 之外全是 stub，`crates/solosoul-core/src/biometric/mod.rs:21-25`）。
- **iOS 完全未开始**：无 `gen/apple`、无 npm script、无 iOS 原生插件；`attachment_open` 的非 Android 回退走 `opener` crate，**在 iOS 上会运行期失败**（`commands/attachment.rs:930-934`）。
- **发布化缺失**：无签名配置、无 AAB、versionCode 恒为 1、minSdk=24 与计划文档的 28 不一致、release 无 CI 产物；**mobile-gated 代码不在 CI 类型检查范围内**（fmt/clippy/test 只跑桌面 target）。
- **资源包问题**：基础 `tauri.conf.json:52-55` 仍把 30MB OCR 模型打进 bundle，移动端 overlay 只加不减——APK 会白白膨胀（对照 Anytype 的体积教训）。
- **权限遗漏**：声明了通知插件与 capability，但 `AndroidManifest.xml` 缺 `POST_NOTIFICATIONS`（Android 13+ 必需）。
- **前端 UI 缺口**：仅约 6 个样式表有 `max-width: 767px` 断点；105 条 `:hover` 规则（大量 hover-only 交互）；存在两套不一致的「mobile」判定（视口宽度的 `useIsMobile` vs 平台的 `isMobilePlatformSync`）；`SearchPopover` 硬编码 520px；弹层/Toast 无安全区适配；虚拟键盘遮挡测试未完成；`/ocr`、`/sync`、`/plugins` 路由仍可直达。
- ~~疑似安全缺口（全平台）~~ **已确认并已修复（2026-07-16）**：`autoLockTimeoutMinutes` 此前只有设置存储与 UI（`stores/settingsStore.ts:110`、`SecuritySettingsPage.tsx`），前后端均无执行代码，自动锁定形同虚设。现已实现 `useAutoLock` hook（`tauri/src/hooks/useAutoLock.ts`）：无活动超时调用 `vaultStore.lock()`，`visibilitychange` 回前台立即结算（覆盖移动端切后台与系统休眠），密码验证框打开期间经 `autoLockPauseStore` 暂停计时（与 CLI `auto_lock_paused` 语义一致），8 个单元测试覆盖（`useAutoLock.test.ts`）。

---

## 5. SoloSoul 移动端优化建议（分阶段路线图）

> 与已有 `docs/android/android-implementation-plan.md` 的 7 阶段计划对齐并补充；工期为单人量级估算。

### P0 — 正确性与安全兜底（1–2 周）
1. ~~**实现真正的自动锁定**~~（✅ 已完成 2026-07-16，见 §4.2）：前端 `useAutoLock` hook + 回前台结算 + 模态暂停，桌面与移动端同时生效。
2. 补 `POST_NOTIFICATIONS` 权限与运行时申请。
3. 移动端 bundle 排除 OCR/embedding 模型（把 `resources` 的 models 移入桌面专属配置，mobile overlay 不继承）。
4. 消除双 mobile 判定：重命名为语义明确的 `useIsNarrowViewport`（布局用）与 `isMobilePlatform`（功能门控用），全库替换。

### P1 — Android 发布化（2–3 周）
5. `signingConfigs`（env 注入 keystore）、AAB 构建、`versionCode` 管理、minSdk 决策（建议 28，与计划文档一致）。
6. CI 增补：`cargo ndk -t aarch64-linux-android check`（让 `#[cfg(mobile)]` 分支进类型检查）、release AAB 产物上传 draft release。
7. 移动端视口的 Playwright 冒烟（启动、解锁、列表、详情、新建对象）。

### P2 — 手机 UI/UX 打磨（2–4 周，借鉴 Notion 移动端渲染规则）
8. 响应式补齐，按页面优先级：首页/对象列表/对象详情/编辑器/设置/搜索（520px 硬编码先修）；原则：**多列塌缩单列、hover 操作改为持久显示或长按、触控目标 ≥44pt、弹层/Toast 全量安全区适配**。
9. 虚拟键盘遮挡回归（表单页、编辑器、命令输入）。
10. 桌面专属路由在移动端 404 化（`/ocr`、`/sync`、`/plugins`）。
11. 快速捕获入口：Android 长按图标 shortcut「新建对象」（对标 Notion quick capture，成本极低、收益高）。

### P3 — 原生能力补齐（3–6 周）
12. **生物识别**：引入 `tauri-plugin-biometric`（iOS/Android 双端官方方案），与现有 `solosoul-core::biometric` 的 macOS/Windows 实现并存；密钥存储对齐 Anytype 实践（Keychain `ThisDeviceOnly` / Keystore 绑定）。
13. **设备同步**：Android 用 NSD 替代 `mdns-sd`（或升级到支持移动端的 mDNS 方案），`solosoul-sync` 的 Noise 握手层可原样复用。
14. 通知：自动锁定提醒、备份提醒（本地通知，无需 FCM——SoloSoul 无服务端，保持零云原则）。

### P4 — OCR / Embedding 移动方案（4–8 周，可延后）
15. 优先做**模型按需下载**而非捆绑（复用现有下载流，加移动端存储预算与 Wi-Fi 仅下载选项）。
16. 推理后端评估：`ort` 的 NNAPI（Android）/ CoreML（iOS）execution provider 移动构建；若成本过高，OCR 可考虑 Android ML Kit 文本识别作为移动降级方案，embedding 可延后或仅桌面。

### P5 — iOS 启动（4–8 周，依赖 P1/P3 完成度）
17. `tauri ios init` 生成 `gen/apple`，签名与 TestFlight 流程；iOS 侧补齐 `attachment_open`（`UIDocumentInteractionController`）、状态栏插件、生物识别复用 P3 成果。

### 贯穿全程的工程纪律（Notion 经验）
- **定义并测量启动指标**：「点图标 → 解锁页可交互」「解锁 → 首页列表可见」两个 P95 预算，真机（低端 Android）测量，纳入 CI 趋势。
- **缓存竞速思想**：解锁后首页渲染不等待任何非必要 IPC，对象列表先显示缓存/骨架。
- **发布节奏**：Android 先走「每版本真机回归 + 内测渠道」，不必过早追求周发布。

---

## 6. 主要引用来源

- Anytype：[anytype-swift](https://github.com/anyproto/anytype-swift) / [anytype-kotlin](https://github.com/anyproto/anytype-kotlin) / [anytype-heart](https://github.com/anyproto/anytype-heart) / [any-sync overview](https://tech.anytype.io/any-sync/overview) / [any-store](https://github.com/anyproto/any-store) / [安全文档](https://doc.anytype.io/anytype-docs/advanced/data-and-security/how-we-keep-your-data-safe)
- Notion：[Android 提速两倍](https://www.notion.com/blog/notion-on-android-is-now-more-than-twice-as-fast-to-launch) / [离线模式工程](https://www.notion.com/blog/how-we-made-notion-available-offline) / [WASM SQLite](https://www.notion.com/blog/how-we-sped-up-notion-in-the-browser-with-wasm-sqlite) / [页面加载提速](https://www.notion.com/blog/faster-page-load-navigation) / [Pragmatic Engineer 播客](https://newsletter.pragmaticengineer.com/p/notion-going-native-on-ios-and-android) / [help: offline](https://www.notion.com/help/use-pages-offline) / [help: mobile](https://www.notion.com/help/notion-for-mobile)
- SoloSoul 内部：`docs/android/android-port-guide.md`、`docs/android/android-implementation-plan.md`、`tauri/src-tauri/`、`tauri/crates/`、`tauri/src/`
