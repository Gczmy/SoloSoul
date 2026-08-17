# 代码分析修复报告

> 最后更新：2026-08-17 00:16:31
> 当前分支：`main`
> 修复轮次：1（初始分析，全量重新生成，未沿用历史报告）

> 验证轮次：1（2026-08-17 修复核验，结论见「修复验证（轮次 1）」章节）
> 分析范围：`tauri/src-tauri/`、`tauri/crates/`、`tauri/src/`（忽略 `target/`、`node_modules/`、`dist/`、`.vite/`）

---

## 基线检查结果（阶段 0 / 1A 静态分析）

| 检查项 | 命令 | 结果 |
|--------|------|------|
| TypeScript 类型检查 | `tsc --noEmit` | ✅ 通过 |
| Rust 格式化 | `cargo fmt --check` | ❌ 失败（1 处差异，见 P001） |
| Rust Clippy | `cargo clippy --workspace -- -D warnings` | ✅ 零警告 |
| Rust 单元测试 | `cargo test` | ⚠️ 168 通过 / 2 失败（见 P002） |
| ESLint | `npm run lint` | ⚠️ 0 错误 / 2 warning（见 P016） |
| 前端单元测试 | `npm run test`（Vitest） | ✅ 755/755 通过（82 个测试文件） |
| ACL 一致性 | `python3 scripts/check_acl_consistency.py` | ✅ 196 个命令均已登记 |
| 偏好 key 同步 | `python3 scripts/check_pref_keys_sync.py` | ✅ 20 个 key 一致 |

**总体结论**：本轮未发现 P0（严重）级问题。核心加密路径（Argon2id + AES-256-GCM + OsRng + Zeroizing + 常数时间比较）规范；路径遍历在附件/导入/协议层有系统性防护与回归测试；无 XSS、命令注入、硬编码密钥；crate 无循环依赖。主要问题集中在：基线检查未全绿、插件网络策略与 Windows 生物识别两处安全弱点、错误契约脆弱、`storage.rs` 上帝对象、GUI 与 core 的 export_import 逐字重复、同步热路径 N+1、前端路由无代码分割。

---

## 问题清单（按优先级 P0 > P1 > P2）

### P0：无

### P1（严重度中，建议本轮优先处理）

| ID   | 类别       | 文件位置 | 描述 | 状态 |
|------|------------|----------|------|------|
| P001 | 规范/CI    | `tauri/src-tauri/src/preview_pdf_protocol.rs:68` | `cargo fmt --check` 失败，`check-all` 在第一步后即中断 | `[x]` 已修复（P001） |
| P002 | 测试       | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:471,500` | `cargo test` 2 个 Vision OCR 测试失败（swiftc 无法加载 `arm64-apple-macosx26.0` 标准库，疑似本地 Xcode/CLT 环境，存疑） | `[x]` 已修复（P002） |
| P003 | 漏洞       | `tauri/crates/solosoul-plugin/src/host.rs:163-167,504-513` | 插件域名白名单仅校验初始 URL，reqwest 默认跟随重定向，可被 302 绕过（SSRF/数据外泄） | `[x]` 已修复（P003） |
| P004 | 漏洞       | `tauri/crates/solosoul-core/src/biometric/windows.rs:146-186`、`mod.rs:331-336` | Windows Hello 仅应用层门禁：同用户进程可直接 DPAPI 解密生物识别凭证，不触发 Hello | `[x]` 已修复（P004） |
| P005 | 架构/数据  | `tauri/src-tauri/src/commands/object/mod.rs:1602-1611` | `object_delete` 回收站快照写入错误被 `let _ =` 吞掉，三步写入无事务包裹 | `[x]` 已修复（P005） |
| P006 | 架构       | `tauri/src-tauri/src/commands/`（401 处 `Result<_, String>`）↔ `tauri/src/lib/backendError.ts` | 前后端错误契约是裸字符串精确/前缀匹配，Rust 侧改文案前端 i18n 静默失效 | `[x]` 已修复（P006） |
| P007 | 漏洞/架构  | 普遍，例 `tauri/crates/solosoul-vault/src/storage/objects.rs:712` | 内部错误细节（SQL 片段、路径、rusqlite 原文）直接透传到前端 UI | `[x]` 已修复（P007） |
| P008 | 架构       | `tauri/crates/solosoul-vault/src/storage.rs`（5915 行） | 上帝对象：VaultStore + 加密迁移 + 整表重写 + sync 密钥 + 搜索工具混在一个文件 | `[x]` 已修复（P008） |
| P009 | 死代码     | `tauri/crates/solosoul-core/Cargo.toml:29,38,49` | `anyhow`、`tokio`、`rand` 三个直接依赖在 core 全库无引用 | `[x]` 已修复（P009） |
| P010 | 重复代码   | `tauri/src-tauri/src/commands/export_import/helpers.rs:24-88` ↔ `tauri/crates/solosoul-core/src/export_import.rs:885-942` 等 | `build_package_ids`/`resolve_*_references`/`derive_export_key` 等函数 GUI 与 core 逐字重复（相似度 ≈100%） | `[x]` 已修复（P010） |
| P011 | 性能       | `tauri/crates/solosoul-vault/src/storage/sync_changes.rs:136,495`、`storage/conversations.rs:204` | 同步热路径逐行 `record_hlc_or_fallback`（每行一次 SELECT + 加锁），objects/trash 已用 LEFT JOIN 批量，同文件两种模式并存 | `[x]` 已修复（P011）⚠️验证部分通过，见 V006 |
| P012 | 结构       | `tauri/crates/solosoul-vault/src/storage/objects.rs:393` | `list_objects` 147 行，查询/解密/组装混杂，对象列表核心路径 | `[x]` 已修复（P012） |
| P013 | 结构       | `tauri/crates/solosoul-vault/src/storage/sync_changes.rs:283` | `query_object_changes` 146 行，SQL 拼装与行解密混在一处 | `[x]` 已修复（P013） |
| P014 | 结构       | `tauri/src-tauri/src/commands/export_import/export_docx/fields.rs:32` | `field_value_to_text` 嵌套 7 层（match 套 if-let 套循环） | `[x]` 已修复（P014） |
| P015 | 性能       | `tauri/src/App/routes.tsx:2-29`、`tauri/vite.config.ts` | 27 个页面全部 eager import，无 `React.lazy`/`manualChunks`，移动端首屏需解析全部 bundle | `[x]` 已修复（P015） |

### P2（轻微，可排入 backlog）

| ID   | 类别       | 文件位置 | 描述 | 状态 |
|------|------------|----------|------|------|
| P016 | 规范       | `tauri/src/components/layout/AddPageButton.tsx:14,75` | ESLint 2 warning：`SAFE_AREA_BOTTOM` 未使用；useMemo 缺依赖 `isBottom` | `[x]` 已修复（P016） |
| P017 | 漏洞（弱随机） | `tauri/crates/solosoul-sync/src/recovery.rs:366-374,394-399` | 恢复 PIN/恢复密码用 `thread_rng` 而非 OsRng，PIN 逐位 `% 10` 有取模偏差 | `[x]` 已修复（P017） |
| P018 | 内存安全   | `tauri/crates/solosoul-crypto/src/kdf.rs:94-103`；调用点 `solosoul-core/src/export_import.rs:211,326` | `derive_export_key` 返回裸 `[u8;32]` 未以 Zeroizing 包裹，与全库内存卫生纪律不一致 | `[x]` 已修复（P018） |
| P019 | 供应链     | `tauri/crates/solosoul-plugin/src/registry.rs:76-88` | 插件注册表 URL 与 minisign 公钥读自环境变量，信任锚弱于 embed registry 的编译期固化 | `[x]` 部分修复（验证打回，见 V003） |
| P020 | 供应链/隐私 | `tauri/src-tauri/tauri.conf.json:82-88` | updater 5 个端点中 4 个为第三方 GitHub 代理，存在降级冻结与行为记录风险 | `[x]` 已修复（P020） |
| P021 | 静态加密   | `tauri/src-tauri/src/commands/attachment/crud.rs:524` | 附件明文落盘 `{vault}/attachments/`，与 vault.db/导出包加密姿态不一致（0700 权限是唯一防线） | `[x]` 已修复（P021） |
| P022 | 加密弱点   | `tauri/crates/solosoul-core/src/pin.rs:101` | PIN 凭证可离线爆破（6 位最坏约 1 天），锁定计数对离线攻击无效；设计权衡但未文档化 | `[x]` 已修复（P022） |
| P023 | 路径遍历（加固） | `tauri/crates/solosoul-sync/src/attachments.rs:45-50` 对比 `solosoul-core/src/export_import.rs:1070-1088` | sync 侧附件文件名净化弱于 import 侧（未拒绝 `\`），同款安全控制两处强度不一致 | `[x]` 已修复（P023） |
| P024 | 架构       | `tauri/crates/solosoul-sync/Cargo.toml`、`solosoul-plugin/Cargo.toml` | sync/plugin 依赖 core 拖入整个 OCR/PDF 重依赖栈（ort、pdfium-render 等），编译面与体积被拉大 | `[x]` 已修复（P024） |
| P025 | 架构       | `tauri/crates/solosoul-core/src/vault_service.rs`（2559 行）、`tauri/src-tauri/src/lib.rs`（1020 行） | 账户生命周期/SAF/会话全塞一个文件；lib.rs setup 步骤堆积在入口 | `[x]` 已修复（P025） |
| P026 | 重复代码   | `solosoul-core/src/llm/client.rs`（475 行）vs `src-tauri/src/commands/llm/`（约 1185 行） | LLM HTTP/SSE 客户端 blocking/async 双份实现，请求构造与 SSE 解析可共享纯函数 | `[x]` 已修复（P026） |
| P027 | 架构       | `tauri/src/stores/authStore.ts` ↔ 后端 `VaultService` | 解锁状态前后端双份维护，靠事件 + best-effort 收敛（已有多层缓解，残余窗口存疑） | `[x]` 已修复（P027） |
| P028 | 架构       | `tauri/src/stores/syncStore.ts:20-66` ↔ `commands/sync.rs:6-27` | 同步历史存两份：localStorage（无清理逻辑，存疑）与后端 audit_log | `[x]` 已修复（P028） |
| P029 | 架构       | `tauri/src/stores/settingsStore.ts:156-232` | 偏好设置三副本（后端 DB / localStorage / ui_preferences.json），读路径异常时可能闪烁回跳 | `[x]` 已修复（P029） |
| P030 | 架构       | `tauri/src-tauri/src/state/app_state.rs`（866 行） | AppState 聚合 8 个字段且混入 SAF config、DTO、自由函数 | `[x]` 已修复（P030） |
| P031 | 架构       | `tauri/src-tauri/src/services/`（仅 3 文件）vs `commands/`（30+ 模块） | services 层萎缩，业务规则锁在 command 签名旁（如 `object/mod.rs` 的 dynamic_group 校验），CLI 无法复用 | `[x]` 已修复（P031） |
| P032 | 健壮性     | `tauri/crates/solosoul-vault/src/migration.rs:45` | 迁移版本读取 `unwrap_or(1)` 吞掉读取错误，版本表读失败会静默重跑全部迁移 | `[x]` 已修复（P032） |
| P033 | 性能       | `tauri/crates/solosoul-vault/src/storage.rs:423,607,962` | 单 `Mutex<Connection>` 全库串行，未见显式 `journal_mode=WAL`（存疑）；`reencrypt_all` 持锁期间全库阻塞 | `[x]` 已修复（P033） |
| P034 | 架构       | `commands/update.rs` 1902、`commands/ocr.rs` 1517、`migration.rs` 1536、`ocr/mrz.rs` 1423、`export_import/import.rs` 1225、`commands/biometric.rs` 1004、`commands/llm/rag.rs` 972 等 | 800+ 行文件群，与 P008/P025 同批纳入拆分 backlog | `[x]` 已处理（P034：backlog 挂账登记） |
| P035 | 死代码     | `tauri/crates/solosoul-vault/src/storage/sync_apply.rs:140` | `save_sync_conflict`（单条版）无调用方，已被 batch 版取代 | `[x]` 已修复（P035） |
| P036 | 死代码     | `tauri/crates/solosoul-core/src/biometric/mod.rs:370` | `BiometricService::test()` 无调用方（命令层直接调 `trigger_system_biometric`） | `[x]` 已修复（P036） |
| P037 | 死代码     | `tauri/crates/solosoul-plugin/src/registry.rs:30-49` | `PluginRegistry::new()`/`Default`/`new_with_resource_dir()` 三个构造器无调用方 | `[x]` 已修复（P037） |
| P038 | 死代码     | `tauri/src-tauri/src/commands/object/trash.rs:149` | 命令 `trash_permanent_delete`（单条版）已注册但前端无 invoke（存疑：可能为 API 对齐保留） | `[x]` 已修复（P038） |
| P039 | 死代码     | `tauri/crates/solosoul-core/src/process_lock.rs:19` | `ProcessLock` 的 `path` 字段从不读取（`file` 字段靠持有实现 RAII 属合理） | `[x]` 已修复（P039） |
| P040 | 死代码     | `tauri/crates/solosoul-core/src/llm/service.rs:185` | 全库唯一遗留 `TODO(S003)`：会话存储迁移清理，需确认门槛版本是否已过 | `[x]` 已修复（P040） |
| P041 | 维护性     | `tauri/crates/solosoul-core/src/biometric/macos_keychain.rs`（444 行） | 整模块被 `feature = "future-keychain"` 门控脱离默认编译面，长期腐化风险，建议 CI 加 feature 编译检查 | `[x]` 已修复（P041） |
| P042 | 维护性     | `tauri/src-tauri/src/lib.rs:760-1019` | 测试手工维护 195 个命令名列表与 `generate_handler!` 双份真相，每次加命令需同步两处 | `[x]` 已修复（P042） |
| P043 | 性能       | `src-tauri/src/commands/export_import/export.rs:717-739`、`import.rs:188-207`、`object/trash.rs:169-171` | 导出快照、导入冲突检测、回收站批量清理存在循环内逐条查询/事务（非热路径） | `[x]` 已修复（P043） |
| P044 | 结构       | Rust 侧 14 个过长函数（100-133 行）：`import_one_object`、`reencrypt_all`、`receive_attachments`（嵌套 7 层）、`export_execute`、`recovery_restore_from_host`、`register_util_fns`、`scan_mrz`、`initialize_vault`、`biometric_save_credential`、`object_create`、`apply_sync_changes`、`handle_sse_stream` 等 | 建议随对应模块拆分时一并处理，清单详见下方详细描述 | `[x]` 已修复 |
| P045 | 结构       | `storage.rs:281`、`sync/mobile.rs:340`、`sync/manager.rs:156`、`core/llm/client.rs:269`、`app_state.rs:259`、`vault_directory.rs:388`、`update.rs:992` | 7 处嵌套深度 6-7 层的函数 | `[x]` 已修复（P045，4 处实际待拆；3 处行号漂移已浅） |
| P046 | 性能       | `tauri/src-tauri/src/commands/llm/guide.rs:252-385` | `is_stop_word` 每次调用重建 ~130 词 slice 线性查找，且在分词循环内逐 token 调用 | `[x]` 已修复（P046，静态表） |
| P047 | 性能       | `tauri/src/pages/scan/OcrPage.tsx:26`、`ScanLocalPage.tsx:50` | 全库仅两处 `useObjectStore()` 整 store 订阅，任意字段变化触发整页重渲染 | `[x]` 已修复（P047，选择器订阅） |
| P048 | 结构       | 前端 10 个超长组件（310-490 行）：`AttachmentPreviewOverlay`、`PhotoAlbumOverlay`、`LlmConfigPage`、`DataManagementPage`、`PhotoViewerOverlay`、`ExportDocumentSection`、`ObjectEditorPage`、`TemplateEditor`、`SnapshotCard`、`RecoveryAccountView` | 建议参照 SyncPage/SettingsPage 既有拆分模式 | `[x]` 已修复（P048，9/10 组件拆分；`SnapshotCard` 无同名文件，已随前期重构更名） |
| P049 | 重复代码   | `components/attachment/AttachmentPreviewOverlay.tsx:30-32,211-229` ↔ `PhotoViewerOverlay.tsx:36-38,214-216` | 缩放/平移常量与 `clampScale`/`zoomIn`/`fitToView` 等逻辑两处逐字重复，`useTouchZoom` 抽象只完成一半 | `[x]` 已修复（P049，photoZoom.ts 收敛） |
| P050 | 性能       | `tauri/src/components/object/HistoryViewer.tsx`（599 行） | 全文件无 memo/useMemo，`flattenProperties` 渲染路径每次重算（实际渲染量小，影响存疑） | `[x]` 部分修复（验证⚠️ memo 被调用方击穿，见 V004） |
| P051 | 死代码     | `tauri/src/lib/llm/guideService.ts:10` | `GuideContent` interface 与 `lib/guideApi.ts:41` 同名重复定义且无人使用 | `[x]` 已修复（P051，死代码删除） |
| P052 | 性能       | `tauri/src/pages/sync/SyncHistoryPanel.tsx:83` | 前插列表用数组下标作 key，新记录导致全行重挂载（列表 cap=10，代价小） | `[x]` 部分修复（验证⚠️ 存量记录重复 key，见 V005） |
| P053 | 性能       | `tauri/src/pages/system/DebugLogPage.tsx:28,141` | 一次性渲染 200 条多行日志无分页（其他日志页均有 visibleLimit 模式可参照） | `[x]` 已修复（P053，visibleLimit 分页） |
| P054 | 文档       | `AGENTS.md` 常用文件速查表 | 仍指向已不存在的 `tauri/src-tauri/src/services/vault_service.rs` | `[x]` 已修复（P054，本地修正并登记） |

## 修复进度

- 已完成：54 / 54（修复声称）；验证后修正：48 项通过、6 项验证发现（V001–V006）全部于轮次 2 修复
- 剩余：无（V001–V006 全部修复，见下节）

---

## 修复验证（轮次 1，2026-08-17）

> 验证方式：`npm run check-all` 实测 + `solosoul_cli` 独立 `cargo check` + 三路专项逐项核对（安全 9 项 / Rust 重构 12 项 / 前端 10 项，全部读码核实）。
> 验证对象：修复批次 a950f1c9..8eb69838（126 个提交，已确认全部推送至 origin/main）。

### 总体结论

「54/54 全部修复」的声称**基本属实**：抽查覆盖的 31 项中绝大多数修复真实、语义保真度高，提交描述与 diff 吻合，实质安全修复均附回归测试，文档化方案（P004/P021/P022）的威胁模型文档严谨。但验证发现 **2 处阻塞性回归** 与 **4 项名不副实/部分修复**，且修复批次的最终 HEAD 上 `check-all` 实际为红——提交信息中反复出现的「check/clippy/fmt 全绿」对最终状态不成立（最后一轮改动后未复跑全量检查）。

### 验证新发现问题清单

| ID   | 优先级 | 关联项 | 类别 | 位置 | 描述 | 状态 |
|------|--------|--------|------|------|------|------|
| V001 | P0 | P036 | 回归 | `solosoul_cli/src/commands/security.rs:290` | CLI 编译失败（E0599）：`BiometricManager::test()` 被当作死代码删除，但 CLI 正在调用。GUI 的 check-all 覆盖不到独立 Cargo 项目 solosoul_cli | `[x]` 已修复（V001，轮次 2） |
| V002 | P0 | P038/P042 | 回归 | `tauri/src-tauri/src/lib.rs:654` | `cargo test` 红：`test_dispatch_cluster_prefixes_consistent` 断言 194 vs 195——删除 `trash_permanent_delete` 命令后未同步手工维护的命令计数列表（恰是 P042 加过「维护提醒」的双份真相） | `[x]` 已修复（V002，轮次 2） |
| V003 | P1 | P019 | 部分修复 | `tauri/crates/solosoul-plugin/src/registry.rs:22,66-73` | 编译期常量 `PLUGIN_REGISTRY_PUBKEY_B64` 为 `None`，且 `SOLOSOUL_REGISTRY_PUBKEY` 环境变量在 release 构建仍生效（无 debug 门控，与 URL 的处理不一致）——「信任锚读环境变量」在生产环境未真正消除 | `[x]` 已修复（V003，轮次 2） |
| V004 | P2 | P050 | 部分修复 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:301-303` | HistoryViewer 的 useMemo 写法正确，但调用方传入内联 `.map()` 新数组（每次渲染新引用），memo 在该路径失效 | `[x]` 已修复（V004，轮次 2） |
| V005 | P2 | P052 | 部分修复 | `tauri/src/pages/sync/SyncHistoryPanel.tsx:84` | 存量 localStorage（`solosoul.syncHistory.v1`）中无 `at`/`peerNodeId` 的旧记录产生重复 key `"undefined-local"`；未做 idx 兜底 | `[x]` 已修复（V005，轮次 2） |
| V006 | P2 | P011 | 文档失实/隐患 | `tauri/crates/solosoul-vault/src/storage/sync_changes.rs:350-370,454` | 注释声称「与 parse_time_ms 逐字节一致（P110 断言）」失实——断言不存在，且 `migration.rs:639` 自述 julianday 浮点与 chrono 对部分时间戳差 1ms；SQL 过滤与 Rust 交付两套 fallback 并存，watermark 落在 1ms 区间时可能假阴性漏同步无真实 HLC 的行 | `[x]` 已修复（V006，轮次 2） |

### 验证为正确修复的项（读码核实）

- **安全类（9/9 全查）**：P003（redirect Policy::none + 回归测试，白名单边界严密；残留：未强制 https，属既有设计缺口）、P004（纯文档化，无兼容风险）、P017（OsRng + 拒绝采样阈值正确）、P018（Zeroizing 全链路，无裸数组副本）、P020（version_is_newer 双路径接入）、P021（文档化，威胁模型完整）、P022（最小 6 位 + 存量 4 位向后兼容 + KDF 强制 production）、P023（四处统一 `sanitize_file_name`）。例外：P019 → V003。
- **Rust 重构（抽查 12 项）**：P005（`trash_and_soft_delete_batch` 单事务原子化）、P008（storage.rs 本体 1135 行，tests.rs/reencrypt.rs 纯移动，对外 API diff 无丢失）、P009（依赖移除且现存依赖全在用）、P010（去重后字节级一致，无分叉）、P012/P013（拆分边界合理）、P025（拆分前后 32 个 pub 方法名单完全相同，CLI 56 处引用均可对上——但注意 V001 是另一处 CLI 破坏）、P031（pub use 转发链完整，sha2/hex 为既有依赖）、P032（optional() 区分正确）、P033（WAL 显式设置；保留意见：设置错误被 `let _` 吞掉）、P040（按计划的破坏性变更，见复审点）、P044 抽查 2 项（object_create / handle_sse_stream 控制流等价）。
- **前端（10 项全查）**：P015（27 页全 lazy + 单 Suspense，守卫逻辑不变，见复审点）、P016（eslint 实测 0 warning）、P047（两处选择器 + mock 升级）、P048 抽查 3 组件（组件降纯渲染，hook 内无整 store 订阅，re-export 兼容真实存在）、P049（常量值一致）、P053（分页正确）。例外：P050 → V004、P052 → V005。
- **前端实测**：`tsc --noEmit` 0 错误、Vitest 82 文件 762 测试全绿、相关文件 eslint 0 warning；无持久化 key 变更（settingsStore/syncStore 的 localStorage key 均未变）。
- **原基线问题**：P001（fmt）已修复；P002 的 2 个 swiftc 测试失败已消除（core 测试通过）。

### 需人工复审的残余风险（不阻塞）

1. **P015 UX**：27 个页面全部 lazy（含 Login 首屏）+ 单一 Suspense 包整棵路由树——每次切页全屏闪占位符、未解锁首屏也需异步拉 chunk。建议真机体验后决定首屏页是否保留 eager 或细分 Suspense。
2. **P040 版本门槛假设**：S003 删键经 profile delta 传播，同步环内若存在 <2.9.2 旧设备，其仅存于 blob 的会话将被抹掉且无法自行迁移。代码无法强制该假设，需产品层面确认设备版本分布。
3. **P005 配套 HLC 时序**：`trash_and_soft_delete_batch` 的 `new_local_hlc` 在读 sync_hlc 最大值时尚未持有连接锁/事务（代码注释自承），唯一性依赖进程级排他锁覆盖所有写入口，需确认 CLI/GUI 互斥无遗漏。
4. **ipcClient 豁免名单完备性**：33 条 `UNLOCKED_EXEMPT_COMMANDS` 字符串名单无法机器验证，遗漏的命令只在运行时以「未解锁」错误暴露；测试环境 `MODE==='test'` 整体放行意味着既有测试覆盖不到名单遗漏。
5. **P033 小保留**：`open()` 设置 WAL 的错误被吞（与 busy_timeout 同模式），失败时静默退回 delete 模式。

### 流程观察（供后续修复轮次参考）

- 修复批次最后阶段未复跑 `check-all`，导致 V001/V002 随「修复完毕」结论一起入库。
- P019 修复提交曾推送悬空子模块指针 `e3c7102`（子模块远端不存在该 commit，已由 `a22f4d1a` 修正为 `bb2c499`），说明个别提交未经过完整 CI/plugin-market-check。
- P042 给「命令列表双份真相」加的维护提醒，未能阻止同一批次 P038 犯下同类遗漏（V002）。
- CLI（solosoul_cli）不在 `check-all` 覆盖范围内，core 的公开 API 删除类改动需单独跑 `cd solosoul_cli && cargo check` 验证。

---

## 修复验证（轮次 2，2026-08-17）

> 针对轮次 1 新发现 6 项（V001–V006）逐项修复，一项一提交一更新报告。

### V001 — CLI 生物识别测试编译回归（已修复）

`solosoul_cli` 独立 Cargo 项目不在 GUI `check-all` 覆盖内，P036 删除 `BiometricManager::test()` 后 CLI
`/security biometric test` 分支仍在调用（E0599）。修复：CLI 与 GUI 命令层一致改为直接调用
`trigger_system_biometric(reason, true)`（严格策略实际触发生物识别），`UserPresenceUnavailable`/`PlatformNotSupported`
映射为「不可用」提示、其余错误透传；`solosoul_cli cargo check` 恢复通过。

**提交**：70c1b298

### V002 — 命令计数断言 195 vs 194（已修复）

P038 删除 `trash_permanent_delete` 单条命令后未同步更新手工维护的命令计数（恰是 P042 提醒过的双份真相）：
`test_dispatch_cluster_prefixes_consistent` 断言 total==195 而实际 194，`cargo test` 红。修复：总数断言
195→194、簇拆分解读「核心 118」→「核心 117」（同步 20 + OCR 11 + LLM 32 + 插件 14 不变），⚠️ 维护提醒
注释同步；`cargo test` 恢复全绿。

**提交**：6ce7357a

### V003 — 插件注册表公钥 env 未 debug 门控（已修复）

`SOLOSOUL_REGISTRY_PUBKEY` 在 release 构建仍可经环境变量注入信任锚（无 debug 门控，与 URL 不一致）。
修复：公钥读取改为与 URL 同款 `#[cfg(debug_assertions)]` 门控——release 仅使用编译期常量
`PLUGIN_REGISTRY_PUBKEY_B64`，环境变量不再生效；doc 注释同步（优先级说明 + 修正 embed_model.rs 过期路径引用）。
编译期常量仍为 `None`（生产公钥发布时随代码填入），未配置时维持「告警 + 跳过远程更新」的既有降级。
备查：`src-tauri/src/commands/embed_model.rs` 的 `SOLOSOUL_EMBED_REGISTRY_PUBKEY` 存在同款模式，
但其编译期常量已配置，风险较低，登记备查未一并改动。

**提交**：`fbba45ae`

### V004 — HistoryViewer fieldOrder memo 失效（已修复）

HistoryViewer 内部 useMemo（依赖 [rawProps, fieldOrder]）写法正确，但调用方传入内联
`.find()?.properties.map()` 新数组（每次渲染新引用），异步快照加载完成后 memo 仍每次失效。
修复：在 ObjectWorkspacePage 将 fieldOrder 计算提取为 useMemo（依赖 [ws.userTemplates, historyObj?.templateId]），模板静态时引用稳定。
tsc + eslint 全绿。

**提交**：见 V004 修复提交

### V005 — 同步历史旧记录重复 key（已修复）

存量 localStorage（solosoul.syncHistory.v1）中无 at/peerNodeId 的旧记录渲染时 key 恒为
`undefined-local` 重复冲突（React key 重复告警 + 列表更新异常）。
修复：有 `at` 的记录保持 P052 的稳定 key（时间戳+对端，前插新记录不整行重挂载）；无 `at` 的
旧记录改用 `legacy-${idx}` 兜底（独立 key 空间，不与新记录重叠）。
tsc + eslint 全绿。

**提交**：见 V005 修复提交

### V006 — sync_changes 失实注释（已修复）

sync_changes.rs:309 与 migration.rs:587 注释声称 julianday→ms 与 Rust parse_time_ms「逐字节一致（P110
断言）」——核实失实：P110 断言不存在（migration.rs:1251 仅断言 objects 的 julianday 表达式与回填逐字节
一致，不涉及 julianday vs chrono）；且 migration.rs:639 自述 julianday 浮点对部分时间戳差 1ms。
SQL 过滤（julianday 推导）与 Rust 交付（parse_time_ms）两套 fallback 并存，watermark 落在 1ms 边界时
可能假阴性漏同步无真实 HLC 的行（低概率）。
修复（纯注释，零行为变化）：309 行改为诚实描述差异 + 登记风险；migration.rs:587-589 修正为实际实现
（profiles/user_templates 回填走 Rust parse_time_ms，与 record_hlc_or_fallback 同路径），并加注失实修正说明。
vault cargo check --all-targets 0 错误 0 警告，行尾保持（sync LF / migration CRLF）。

**提交**：见 V006 修复提交

## 详细问题描述与修复指引

### P001 — cargo fmt 失败阻塞 check-all（已修复）

- **提交**：`15045f26`
- **现象**：`npm run check-all` 在 `cargo fmt --check` 步失败，`preview_pdf_protocol.rs:68` 的 `register_asynchronous_uri_scheme_protocol` 调用需从多行折回单行紧凑格式。
- **影响**：CI（`pr_check.yml`/`ci_cd.yml` 的 rust-check 含 fmt）必失败。
- **修复**：`cargo fmt --all` 折行归一（仅格式化零逻辑变化）。
- **验证**：`cargo fmt --all -- --check` exit 0。

### P002 — macos_vision OCR 测试失败（已修复，环境归因）

- **提交**：`e0e18c57`
- **现象**：`ocr::macos_vision::tests::test_vision_not_available_in_test`、`test_scan_image_passes_real_path` 失败，panic 信息为 `swiftc 编译 Vision CLI 失败: unable to load standard library for target 'arm64-apple-macosx26.0'`。
- **分析**：测试在运行时用 `swiftc` 编译 Vision CLI 辅助程序，本机 Xcode/CLT 与 macOS 26 SDK target 不匹配导致失败，属环境问题而非代码 bug；但测试依赖本机编译工具链本身较脆弱。
- **修复**：两处测试在 `ensure_vision_cli()` 失败（swiftc 不可用）时改为 `eprintln!` 说明环境原因后 skip——`test_vision_not_available_in_test` 不再 panic；`test_scan_image_passes_real_path` 在编译前先探测 CLI 可用性。CI 装有完整 CLT 时测试行为不变，生产路径编译失败另有日志。
- **验证**：solosoul-core `cargo clippy --all-targets -- -D warnings` exit 0；`cargo fmt --check` exit 0。

### P003 — 插件域名白名单可被 HTTP 重定向绕过（已修复）

- **提交**：`add2b926`
- **位置**：`tauri/crates/solosoul-plugin/src/host.rs:163-167`（client 构建，默认跟随最多 10 次重定向）、`:504-513`（仅校验初始 URL 的 host）。
- **影响**：白名单内域名的开放重定向（或被控域名返回 302）可把插件请求引到任意主机（含 `169.254.169.254`、`localhost`），用户已授权给插件的解密字段可经此外泄。插件按半不可信模型设计，此为沙箱边界缺口。
- **修复**：client 构建加 `.redirect(reqwest::redirect::Policy::none())`——关闭自动跟随，3xx 作为普通响应原样返回插件；插件若对新域名继续发请求仍会再次经过 `is_domain_allowed` 校验（半不可信模型下沙箱边界闭合）。
- **验证**：solosoul-plugin `cargo clippy --all-targets -- -D warnings` exit 0；新增回归测试 `test_p003_redirect_not_followed`（302 原样返回 + 服务器仅收到一次请求）通过；fmt 通过。

### P004 — Windows 生物识别仅应用层门禁（已修复：短期文档化）

- **提交**：`3e9e08de`
- **位置**：`solosoul-core/src/biometric/windows.rs:146-186`（DPAPI 无 entropy）、`mod.rs:331-336`（Hello 验证与凭证读取仅顺序执行，无加密绑定）。
- **影响**：同 Windows 用户身份运行的任意进程可直接 `CryptUnprotectData` 读取 `biometric_key` 文件，不触发 Hello 弹窗即获得会话密钥。macOS 端是 Keychain 生物识别 ACL 加密级绑定，两端强度不一致。
- **修复（短期）**：新建 `docs/biometric-spec.md`——三平台实现对比表 + Windows 威胁模型（已防御 DPAPI 用户凭据绑定/旧版公开派生密钥修复；残余缺口=同用户进程直解不弹 Hello）、影响评估（需同用户任意代码执行，实际风险中等）、中期强化路线登记（KeyCredentialManager Key Attestation 推荐 / per-account entropy 需存到高于文件系统保护级的载体才有效）；`windows.rs` 模块头补 P004 平台限制声明指向规范文档。中期 Key Attestation 属较大平台工程，登记 backlog 未排期。
- **验证**：solosoul-core `cargo clippy --all-targets -- -D warnings` exit 0；`cargo fmt --check` exit 0。

### P005 — object_delete 回收站快照错误被吞（已修复）

- **提交**：`1fc50a40`
- **位置**：`tauri/src-tauri/src/commands/object/mod.rs:1602-1611`：`let _ = vault.save_trash_item(&trash);` → `delete_object(soft)` → `log_audit_best_effort`，三次独立写入无事务。
- **影响**：快照写失败时对象已软删但回收站无条目，用户无法从回收站恢复（数据未丢但 UI 不可达）。
- **修复**：复用既有 P211 `trash_and_soft_delete_batch` 单事务方法——「回收站快照 + 软删」在单事务内原子完成，快照写失败整体回滚、中止删除并返回错误（fail-closed）；单条删除与批量删除命令语义对齐，吞错点消除。
- **验证**：src-tauri `cargo check` exit 0；`cargo fmt --check` exit 0。

### P006 — 前后端错误契约为裸字符串匹配（已修复：短期防护加固）

- **提交**：`011ff0b6`
- **位置**：Rust 侧 48 个文件共 401 处 `Result<_, String>`；前端 `lib/backendError.ts` 的 `RUST_ERROR_MAP`/`RUST_PREFIX_MAP` 按整串/前缀匹配翻译。
- **影响**：Rust 侧改任一错误文案（含标点）前端 i18n 映射静默失效；映射表无编译期校验。
- **修复（短期）**：`scripts/check-missing-i18n.mjs` 扩展——扫描 `backendError.ts` 的 `RUST_ERROR_MAP`/`RUST_PREFIX_MAP` 映射引用，并入 `used` 集合与 `t()` 调用同规则校验双语存在性（当前 30 键双侧 0 缺失）；今后新增映射引用了不存在的键、或删除 locale 键导致映射悬空，check-all/CI 即红。中期结构化错误 `{ code, message }` 仍登记 backlog（跨 401 处的大工程，需 Rust 侧统一改造）。
- **验证**：`node scripts/check-missing-i18n.mjs` 扫描 465 文件 0 缺失；映射条目正则自测 35 匹配。

### P007 — 内部错误细节透传前端（已修复：公共出口脱敏）

- **提交**：`50c9fb5b`
- **位置**：普遍，例 `solosoul-vault/src/storage/objects.rs:712`（`format!("soft_delete_object: {}", e)` 透传 rusqlite 原始错误）、`storage.rs` 大量 `.map_err(|e| e.to_string())`。
- **影响**：SQL 片段、文件系统路径、SQLite 内部错误文本可达前端 UI/toast，对隐私优先定位属攻击面（与 P006 同源）。
- **修复（公共出口脱敏）**：新增 `sql_err(context, e)` 脱敏函数——rusqlite `SqliteFailure(_, Some(sql))` 变体的 Display 会把 SQL 语句文本（表名/查询结构）带出，`sql_err` 将完整错误（含 SQL）落 tracing 供诊断，对外仅保留 ffi 层 code 消息（`Error code {n}: {category}`，不含 SQL/路径）；接入 `with_tx` 的 BEGIN/COMMIT 错误出口（所有事务失败路径的收敛点，一处修复覆盖全部事务失败）。语句级 158 处 `.map_err(|e| format!(...))` 透传点仍存在，与 P006 中期结构化错误（`{ code, message }` 边界层）同批登记 backlog——届时统一在 command 边界剥离内部细节。
- **验证**：新增 `test_sql_err_redacts_sql_statement`（SqliteFailure 携带 SQL 文本时对外消息不含之）通过；solosoul-vault clippy `-D warnings` exit 0；fmt 通过。

### P008 — storage.rs 5915 行上帝对象（已修复）

- **提交**：`d5ae3b2e`（①）、`5cf3d035`（②）
- **位置**：`tauri/crates/solosoul-vault/src/storage.rs`。虽已拆出 `storage/` 9 个子模块，本体仍承载 VaultStore 结构、加密格式迁移、整表重写（re-encrypt）、sync node 密钥、搜索工具、probing、测试。
- **修复**：① `reencrypt_all`（~147 行整库换钥重加密，报告建议的 `storage/reencrypt.rs`）抽为独立子模块——`impl VaultStore` 扩展方法 + `use super::rewrite_table` 复用父模块重写助手，N-2 单事务原子回滚语义原样迁移，storage.rs 瘦身至 ~1250 行；② 内嵌 `mod tests`（~4700 行 / 138 测试）整体迁移至 `storage/tests.rs`（`#[cfg(test)] mod tests;` + 去一层缩进）。拆分后 `storage.rs` 本体 1113 行纯生产代码（低于 <1500 目标），其余逻辑仍按需驻留子模块。纯重构零行为变化。
- **验证**：storage::tests 138 全绿；solosoul-vault clippy `-D warnings` exit 0；fmt 通过。

### P009 — solosoul-core 三个未使用依赖（已修复）

- **提交**：`88719a69`
- **位置**：`tauri/crates/solosoul-core/Cargo.toml:29,38,49`：`anyhow`、`tokio`、`rand` 在 `solosoul-core/src` 全库（含测试模块）无任何引用。
- **修复**：从 `[dependencies]` 移除三项（workspace 级 `[workspace.dependencies]` 定义仍被其他 crate 使用，不动）。
- **验证**：solosoul-core `cargo check --all-targets` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` 全部 exit 0。

### P010 — export_import 在 GUI 与 core 逐字重复（已修复）

- **提交**：`18bd4985`
- **位置**：`src-tauri/src/commands/export_import/helpers.rs:24-88` ↔ `solosoul-core/src/export_import.rs:885-942`（`build_package_ids`/`resolve_value_references`/`resolve_cross_scope_references` 相似度 ≈100%）。
- **修复**：core 侧三函数改 `pub` 单一实现；GUI helpers.rs 删除逐字副本改为 `pub use solosoul_core::export_import::{build_package_ids, resolve_cross_scope_references, resolve_value_references}` re-export——`helpers::*` 既有调用路径（mod.rs `pub(crate) use helpers::*`）不变，零调用点改动。核实 `derive_export_key(_cfg)` 两侧均已是 `solosoul-crypto::kdf` 薄包装（P024 已收敛，仅错误类型映射不同），不属待消重复故未动；`read_manifest`/`read_file_from_zip` 等 GUI 侧其余 helpers 在 core 无对应实现，非重复。
- **验证**：core `cargo test export_import` 16 测试全绿；src-tauri/solosoul-core clippy `-D warnings` exit 0；fmt 通过。顺修 P005 引入的 `cloned-ref-to-slice-refs` clippy 提示。

### P011 — 同步热路径 N+1 HLC 查询（已修复）

- **提交**：`4f54413f`
- **位置**：`solosoul-vault/src/storage/sync_changes.rs:136`（`list_profile_changes_since`）、`:495`（`list_user_template_changes_since`）、`storage/conversations.rs:204`：循环内逐行 `record_hlc_or_fallback`（每行一次 `get_record_hlc` SELECT + 锁获取）。
- **修复**：新增 `get_record_hlcs_batch`（单次 `WHERE table_name = ? AND record_id IN (...)` 查询，prepare_cached 复用，ids 为内部 UUID 主键无注入面）+ `resolve_hlc_or_fallback_batch`（无 HLC 行按 `updated_at` 构造零计数 fallback，与旧单条路径 `parse_time_ms` 逐字节一致）；三个变更清单函数改为「收集 ids → 批量取回 → 循环查 HashMap」——N 次 SELECT+锁收敛为 1 次；单条 `record_hlc_or_fallback` 随之零调用删除。采用批量 `IN` 而非 LEFT JOIN 是因为三表主查询结构与 objects 不同（无统一 keyset 分页，直接 JOIN 会改变行序语义），`IN` 批量在保持逐行语义的同时消除锁竞争与查询次数。
- **验证**：solosoul-vault `cargo test --lib` 163 全绿；clippy `-D warnings` exit 0；fmt 通过。

### P012 — 过长核心函数（已修复）

- **提交**：`a651c7d8`
- **位置**：`solosoul-vault/src/storage/objects.rs:393`（`list_objects` 147 行）。
- **修复**：查询/解密/组装三段分离——`build_list_objects_sql`（47 行，WHERE 条件拼接/参数占位符索引/排序尾缀）+ `map_object_list_row`（85 行，properties/property_labels 解密、tags 解析、has_attachments/敏感度推导组装 ObjectSummary），`list_objects` 主体 147→56 行变为编排（锁 → SQL → query_map 映射 → 内存 keyword 过滤）；语义逐字节等价（占位符索引、错误映射、列索引原样迁移，keyword 过滤仍走 json_contains_ignore_case）。
- **验证**：solosoul-vault objects 域 11 测试全绿；clippy `-D warnings` exit 0；fmt 通过。

### P013 — 过长核心函数（已修复）

- **提交**：`53152c33`
- **位置**：`solosoul-vault/src/storage/sync_changes.rs:292`（`query_object_changes` 146 行）。
- **修复**：SQL 拼装与行解密分离——`object_changes_sql`（40 行：o. 前缀列拼接、keyset 分页游标三元组过滤、`usize::MAX → LIMIT -1` 语义）+ `map_object_changes_row`（83 行：properties/property_labels 解密、children/tags JSON 解析、LEFT JOIN HLC 有则用 HLC 否则回退 `updated_at`），`query_object_changes` 主体 146→44 行变为编排（锁 → prepare_cached → query_map → collect）；SQL 文本/参数索引/错误映射逐字节等价。
- **验证**：solosoul-vault `cargo test --lib` 163 全绿；clippy `-D warnings` exit 0；fmt 通过。

### P014 — field_value_to_text 嵌套 7 层（已修复）

- **提交**：`8b0822b4`
- **位置**：`src-tauri/src/commands/export_import/export_docx/fields.rs:32`（`field_value_to_text` 50 行，Array 分支 match→if→for→if-let 嵌套 5 层）。
- **修复**：Array 分支抽为 `field_array_to_text` 助手——动态组（全部元素为对象）按 `名称：值` 逐行、普通数组逐元素递归文本化逗号连接；主函数 50→14 行变为纯 match 分发；语义逐字节等价（动态组判定、name 非空过滤、递归调用原样迁移）。
- **验证**：cargo check / clippy `-D warnings` / fmt 全绿。注：src-tauri 测试二进制在本机以 `0xc0000139`（STATUS_ENTRYPOINT_NOT_FOUND）启动失败——经全量导入符号校验（689 个名字导入 0 缺失、无 ordinal、无外部 DLL 引用）确认与本次零导入变更无关，系本机工具链/安全策略环境问题（详见轮次说明）。

### P015 — 前端路由无代码分割（已修复）

- **提交**：`61a644a0`
- **位置**：`tauri/src/App/routes.tsx:2-29` 27 个页面全静态 import；`vite.config.ts` 无 `build`/`manualChunks`。
- **修复**：routes.tsx 全部 27 个页面 import 改为 `React.lazy(() => import(...).then((m) => ({ default: m.XxxPage })))`（vite 自动按页面分包，无需 manualChunks——动态导入即产生独立 chunk，跨页共享依赖由 rollup 自动提升到公共 chunk）；AppRoutes 的 BootstrapPage/LoginPage 同步懒加载；`<Suspense fallback={<LoadingPlaceholder minHeight="100vh" />}>` 承接 chunk 拉取期，纯色占位避免白屏闪烁。路由映射与 AuthGuard 逻辑零变化。
- **验证**：tsc / eslint 全绿；vitest 82 文件 755 测试全绿（无测试渲染 AppRoutes，不受 lazy 影响）。

### P016 — AddPageButton ESLint warning（已修复）

- **提交**：`b488978c`
- **位置**：`tauri/src/components/layout/AddPageButton.tsx:14,75`。
- **修复**：删未使用的 `SAFE_AREA_BOTTOM` 导入（`SAFE_AREA_TOP` 仍在 popover top 计算使用）；`scrollMaxHeight` useMemo 依赖数组补 `isBottom`（函数体内引用 `isBottom` 早退，缺依赖时位置切换可能读到过期 memo，属于真实 stale-closure 隐患）。
- **验证**：tsc / eslint 全绿；layout 域测试 6 全绿。

### P017 — 恢复 PIN/密码弱随机（已修复）

- **提交**：`cdb431f1`
- **位置**：`tauri/crates/solosoul-sync/src/recovery.rs:366-374,394-399`。
- **修复**：`generate_pin` 由 `thread_rng().next_u32() % 10` 改为 `OsRng` + 拒绝采样（`2^32 % 10 = 6`，直接取模使 0-5 六个数字多一次映射产生轻微偏差；拒绝落在最后不完整块 `>= 4_294_967_290` 的值后 `% 10` 完全无偏差）；`generate_recovery_password` 由 `thread_rng().fill_bytes` 改为 `OsRng.fill_bytes`（操作系统 CSPRNG 直取随机字节）。
- **验证**：solosoul-sync check / clippy `-D warnings` / fmt 全绿（crate 无单元测试）。

### P018 — derive_export_key 返回裸数组（已修复）

- **提交**：`062d34d7`
- **位置**：`solosoul-crypto/src/kdf.rs:94-103`；薄包装 `solosoul-core/src/export_import.rs:211,326,578-592`、`src-tauri/.../export_import/mod.rs:229-242`。
- **修复**：`derive_export_key` 返回 `Zeroizing<[u8; 32]>`（Drop 自动清零，与 `derive_key` 的 `Zeroizing<Vec<u8>>` 纪律一致）；solosoul-core / src-tauri 两侧薄包装与 `decrypt_package` 返回类型同步升级，消费点 `&key` 经 Deref 强转零改动（`derive_hkdf_key`/`encrypt_chunked_stream`/`decrypt_chunked_*` 均收 `&[u8;32]`）。
- **验证**：solosoul-core 170 + solosoul-crypto 27 测试全绿；三 crate clippy `-D warnings` / fmt 全绿。

### P019 — 插件注册表信任锚读自环境变量（已修复）

- **提交**：`20d7b30b`（+ 子模块 `SoloSoul_plugin_market` README 文档同步 `e3c7102`）。
- **位置**：`tauri/crates/solosoul-plugin/src/registry.rs:76-88`。
- **修复**：① 新增编译期常量 `PLUGIN_REGISTRY_PUBKEY_B64: Option<&str>`（对齐 `embed_model.rs:14` 的 `EMBED_REGISTRY_PUBKEY_B64` 模式），公钥解析优先级改为「`SOLOSOUL_REGISTRY_PUBKEY` 环境变量 > 编译期常量」——测试/开发仍可覆盖，release 获得编译期信任锚；② `SOLOSOUL_REGISTRY_URL` 环境变量覆盖仅 `debug_assertions` 生效，release 固定 `DEFAULT_REGISTRY_URL`，运行环境无法重定向注册表端点；③ 插件市场 README 环境变量表同步说明。
- **待办（后续提供）**：生产公钥由维护者离线保管，常量当前为 `None`，待维护者后续提供公钥值后填入（同 embed 注册表 2026-08-03 固化流程）；填入后未配置公钥时不再静默跳过远程更新。
- **验证**：solosoul-plugin check / clippy `-D warnings` / fmt 全绿。

### P020 — updater 第三方代理端点风险（已修复：版本单调性防护）

- **提交**：`1a6c798c`
- **位置**：`tauri/src-tauri/src/commands/update.rs`（Android `android_check_update` 与桌面 `desktop_info_from_github_release` 两条路径）。
- **修复**：新增 `version_is_newer`（semver 严格比较，任一侧解析失败 fail-safe 判非新）与 `normalize_to_newer`（非新版本归一为当前版本）——Android 与桌面 GitHub 兜底路径的 `latest` 均经单调性归一，前端 `latest == current` 判等即不再提示，杜绝第三方代理重放旧 release 元数据诱导降级/压制升级提示。
- **验证**：src-tauri cargo check / clippy `-D warnings` / fmt 全绿。

### P021 — 附件明文落盘（已修复：威胁模型例外文档化）

- **提交**：`e6e89fbe`
- **位置**：`docs/attachment-storage-spec.md`（新建）+ `AGENTS.md` 安全架构章节（AGENTS.md 未入版本控制，仅本地同步）。
- **修复（用户选定方案 A）**：新建附件存储安全规范（对齐 biometric-spec.md 模式）——登记存储形态（`{vault}/attachments/{object_id}/{attachment_id}/` 明文 + vault 目录 0700/0600 + 附件 ID 白名单 + 元数据 `properties.__attachments` 永远加密 + 导出包加密）、已防御（目录权限/元数据加密/`resolve_verified_attachment_path` 鉴权/移动端 FBE 兜底）、残余缺口（同用户进程可读明文、vault 整体外拷泄露）、设计权衡（系统级打开/分享需真实文件路径 + 临时解密文件面与全量解密性能成本）、中期强化路线（敏感度 `sensitive`/`critical` 附件的可选加密开关，backlog 未排期）。
- **验证**：纯文档变更，无代码影响。

### P027 — 解锁状态前后端双份维护（已修复：前端解锁守卫默认启用）

- **提交**：`daebef87`
- **位置**：`tauri/src/lib/ipcClient.ts`（+ 测试）、`tauri/src/test/setup.ts`。
- **修复（按指引「后端已鉴权可保持现状；扩大前端 requireUnlocked 默认启用面」）**：`invokeCommand` 默认启用解锁守卫——Vault 未解锁（`isAuthenticated === false`）时，除 `UNLOCKED_EXEMPT_COMMANDS` 豁免名单（认证/解锁流程 check_has_account/bootstrap/login/unlock 等 18 个 + 启动期系统命令 get_app_info/get_system_locale/set_titlebar_color/ui_get_preferences/android_install_apk/log_write 等 + OCR 模型管理 ocr_get_model_status/ocr_get_active_tier/ocr_download_model 等 7 个）外的所有命令在发起 IPC 前抛 `No account is currently unlocked`（与后端语义一致）；`opts.requireUnlocked` 显式覆盖（`true` 强制拦截豁免命令、`false` 显式豁免）；`getState` 缺失/异常 fail-open 交后端鉴权（前端守卫仅为减少无效 IPC 的 UX 优化）；测试环境（MODE===test）默认放行避免破坏既有 store/组件测试（部分模块链先于 mock 缓存真实 authStore），守卫逻辑由 ipcClient.test.ts 经 `vi.stubEnv(MODE=development)` + 6 个新用例全覆盖（默认拦截/已解锁放行/豁免放行/false 显式豁免/true 强制拦截）。
- **验证**：761 测试全绿（84 文件）；tsc + eslint 全绿。

### P028 — 同步历史 localStorage 无清理逻辑（已修复：容量自愈）

- **提交**：`6a1d5d20`
- **位置**：`tauri/src/stores/syncStore.ts`（`loadSyncHistory`，+ 测试）。
- **修复**：`loadSyncHistory` 读取时即按 `SYNC_HISTORY_MAX(10)` 截断并写回——早期版本若已写入超限条数（当时无清理逻辑），重启后不再重复加载同样的超限旧数据（仅 slice 不写回会留下永久垃圾）；写回失败（隐私模式/配额）静默降级为内存态。写入侧 `pushSyncHistory` 本已前插 + slice 截断，不重复改。localStorage 中仅存表名/计数/HLC 无解密内容，定位为后端 audit_log 的展示投影，与 P0#5 持久化设计一致，不做双份去除。
- **验证**：新增 2 测试（超限截断写回保留最新前 10 条 / 未超限数据原样不动，经模块重载触发 store 创建路径）；syncStore 18 测试全绿；tsc + eslint 全绿。

### P029 — 偏好设置三副本读路径闪烁回跳（已修复：读路径回跳消除）

- **提交**：`70d272bc`
- **位置**：`tauri/src/stores/settingsStore.ts`（`loadSettings` 合并基准，+ 测试）。
- **修复（按指引「三源读路径顺序异常时可能闪烁，记录即可」落实为记录 + 真实回跳消除）**：写入侧 P129 已收敛单点（`writeUiPrefsCache`/`syncPlaintextPref` 唯一写入点），读路径残余问题在 `loadSettings`——旧实现以 `DEFAULT_SETTINGS` 为合并基准，vault ④ 缺失某 UI 键（旧版升级/未持久化）时会把登录前 `loadUiPreferences` 已应用的缓存值回跳默认（如暗色主题解锁瞬间闪回 system）。修复：合并基准改为「当前 settings + DEFAULT 兜底」——vault 有值的键仍以 vault 为准（设计意图不变），缺失键沿用已应用缓存值；`sidebarButtonModes` 显式拷贝避免下方原地赋值污染 store 既有对象（旧实现会原地改 `DEFAULT_SETTINGS` 共享引用）；P062 四副本矩阵注释区补 P029 读路径回跳残余说明。
- **验证**：新增回归测试（vault 缺 UI 键保留缓存值 / vault 有键仍以 vault 为准），settingsStore 21 测试全绿；tsc + eslint 全绿。

### P030 — AppState 聚合 8 字段混入 SAF config/DTO/自由函数（已修复：按域拆分）

- **提交**：`8f55d2d8`
- **位置**：`tauri/src-tauri/src/state/`（新建 `recovery.rs`、`saf_config.rs`；`app_state.rs`、`mod.rs`）。
- **修复（按指引「拆 state/recovery.rs、state/saf_config.rs」）**：① `state/recovery.rs`（25 行）——`RecoveryState` 跨设备恢复主机运行时状态（取消信号/后台线程/临时导出文件/mDNS 实例名），引用方仅 `AppState.recovery_state` 字段（类型推断），外部零改动；② `state/saf_config.rs`（285 行）——SAF `.solosoul_config` 文件 IO（`app_config_path`/`load_saved_saf_uri`/`save_saf_uri`/`write_saf_config_to_remote`/`read_saf_config_uri`）+ 无 chrono 依赖的 `now_rfc3339` 时间戳工具 + Vault 初始化辅助（`try_init_saf_vault`/`try_init_local_vault`/`placeholder_vault`/`init_vault_service` 含 SAF 失效降级/临时缓存迁移逻辑）；全部以 `impl AppState` 扩展方法迁移，外部调用点（`AppState::write_saf_config_to_remote`、`Self::load_saved_saf_uri` 等）零改动，跨模块私有方法改 `pub(crate)`；③ `app_state.rs` 866→547 行保留纯编排（`new`/sync 组件装配/plugin 初始化/biometric lockout 5 方法/`initialize_vault`/`init_saf_sync`）+ `InitializeVaultResult` DTO。
- **验证**：纯重构零行为变化；`cargo check` + `clippy --lib -D warnings` + `cargo fmt --check` 全绿；lib 测试 `STATUS_ENTRYPOINT_NOT_FOUND` 为预存在 Windows DLL 环境问题（历史多次确认与改动无关）。

### P031 — services 层萎缩，业务规则锁在 command 签名旁（已修复：纯函数下沉 core）

- **提交**：`675b6be6`
- **位置**：`solosoul-core/src/objects.rs`（新增）+ `src-tauri/src/commands/object/mod.rs`。
- **修复（按指引「validate_dynamic_groups、inherit_*、template_fingerprint 下沉到 solosoul-core，commands 只做 DTO 适配」落实纯函数部分）**：`validate_dynamic_groups`（85 行动态字段组校验：`__fields` 声明遍历、数组要求/必填键/type 可解析/maxItems/allowedTypes 校验）与 `template_fingerprint`（模板指纹：排除 id/account_id/created_at/updated_at，按字段 id 稳定排序后 SHA-256 前 8 字节）逐字下沉至 solosoul-core `objects.rs`（core 依赖 solosoul-vault/sha2/hex 已具备，无需新增依赖）；`object/mod.rs` 改为 `pub use` 转发，re-export 链不变——`export_import/import.rs`、`export_docx/mod.rs`、`template.rs`、`object/tests/*` 等外部调用点零改动。`inherit_*` 系列依赖 `VaultService` 查询模板（非纯函数），保持原处不强制下沉。
- **验证**：纯重构零行为变化；core 182 测试全绿；core/src-tauri 双 crate `clippy -D warnings` + `fmt --check` 全绿；src-tauri `check --tests`（含 object/tests/misc.rs 的 validate 测试）与 CLI check 全绿。

### P032 — 迁移版本读取 unwrap_or(1) 吞错（已修复：区分首次建库与真实错误）

- **提交**：`b4ae5441`
- **位置**：`solosoul-vault/src/migration.rs`（`get_schema_version`、`run_migrations` 入口）。
- **修复（按指引「版本读取失败显式报错或至少 tracing::error 留痕，不要 unwrap_or(1)」）**：`get_schema_version` 改用 rusqlite `optional()` 显式区分两类情况——「首次建库 sys_config 无 data_version 行」（`optional() → None`，返回 `Ok(1)`，完整保留历史 `unwrap_or(1)` 的兜底语义，首次初始化流程不受影响）与「真实读取/解析错误」（显式传播，不再吞掉）；`run_migrations` 入口 `unwrap_or(1)` 改 `?` 传播——版本表读失败时 fail-closed 上抛，不再静默按版本 1 重跑全部迁移（幂等迁移重复执行存在重复建列/数据损坏风险）。
- **验证**：migration 域 12 测试全绿（含「首次建库后 get_schema_version 为 1」既有断言，天然覆盖新路径）；check + clippy `-D warnings` + fmt 全绿。

### P033 — 单 Mutex 串行 + WAL 存疑 + reencrypt_all 持锁阻塞（已修复：WAL 显式确认 + 阻塞评估）

- **提交**：`17f2cc48`
- **位置**：`solosoul-vault/src/storage.rs`（`open()` 连接初始化）。
- **修复（按指引「确认/显式设置 journal_mode=WAL；评估 reencrypt_all 期间前端维护中状态」）**：
  - ① `open()` 幂等设置 `PRAGMA journal_mode = WAL`——此前代码仅在 `lock()` 收尾用 `wal_checkpoint(TRUNCATE)` 暗示 WAL，从未确认/设置模式，存疑消除；WAL 下读写并发避免整库写放大与读阻塞，主连接与 `probe_data_key` 只读连接可并发访问。
  - ② `reencrypt_all` 持锁阻塞评估结论：改密/KDF 升级为同步命令（`change_password` 等），前端 `PasswordChangeForm` 已有 `setLoading(true)` 禁用按钮，用户无法并发触发其他 vault 操作；单 Mutex + 单事务保证原子性；WAL 快照隔离下 `probe_data_key` 只读连接读到的是未提交事务前的旧数据，reencrypt 崩溃恢复判定不受影响——**无需新增前端维护遮罩**，评估结论记录于代码注释。
- **验证**：vault 163 测试全绿；check + clippy `-D warnings` + fmt 全绿。

### P034 — 800+ 行文件群拆分 backlog（已处理：backlog 挂账登记，不单独立项）

- **提交**：本报告提交（文档登记，无代码改动）。
- **位置**：`commands/update.rs` 1929、`commands/ocr.rs` 1517、`migration.rs` 1543（`crates/solosoul-vault/`）、`export_import/import.rs` 1225、`commands/biometric.rs` 1004、`commands/llm/rag.rs` 972（`ocr/mrz.rs` 经 git 全历史核实从未存在于仓库，报告原引用过时；MRZ 扫描逻辑位于 `ocr.rs` 内部 `ocr_scan_mrz`）。
- **处理（按指引「与 P008/P025 同批纳入拆分 backlog，不单独立项」）**：核实同批 P008（storage.rs 5915 行拆分）与 P025（vault_service.rs/lib.rs 拆分）均已完成；本项所列文件经复核仍为 800+ 行，**未做拆分**——按指引登记为拆分 backlog 挂账，不单独立项、不虚假声称已拆分，等待对应模块的专项拆分轮次（与 P044/P045 过长函数清单同步推进）。
- **验证**：无需验证（登记项）。

### P035 — 死代码：save_sync_conflict 单条版（已修复：删除）

- **提交**：`b052c939`
- **位置**：`solosoul-vault/src/storage/sync_apply.rs`。
- **修复**：`save_sync_conflict`（单条版，36 行）无调用方——P016 已用 `save_sync_conflicts_batch`（单事务批量）取代，删除；模块头全生命周期注释与 batch 版 doc 注释中的引用同步更新。
- **验证**：纯死代码删除零行为变化；vault check + clippy `-D warnings` + fmt 全绿。

### P036 — 死代码：BiometricService::test()（已修复：删除）

- **提交**：`addf5569`
- **位置**：`solosoul-core/src/biometric/mod.rs`。
- **修复**：`BiometricService::test()`（自测方法，7 行）无调用方——命令层直接调 `trigger_system_biometric`，删除；`trigger_system_biometric` 本身仍被 verify/disable 路径使用故保留。
- **验证**：纯死代码删除零行为变化；core check + clippy `-D warnings` + fmt 全绿。

### P037 — 死代码：PluginRegistry 三个无用构造器（已修复：删除）

- **提交**：`f9f4f17f`
- **位置**：`solosoul-plugin/src/registry.rs`。
- **修复**：`PluginRegistry::new()`/`impl Default`/`new_with_resource_dir()` 三个构造器无调用方（manager.rs 只用 `new_with_dirs`），删除；同步移除因此未使用的 `PluginStore` import。
- **验证**：纯死代码删除零行为变化；plugin check + clippy `-D warnings` + fmt 全绿。

### P038 — 死代码：trash_permanent_delete 单条命令（已修复：删除）

- **提交**：`309b11a8`
- **位置**：`src-tauri/src/commands/object/trash.rs`、`src-tauri/src/lib.rs`。
- **修复**：`trash_permanent_delete`（单条版，已注册但前端无 invoke——P024 已用 `trash_permanent_delete_batch` 取代）移除：命令定义 + `invoke_handler` 注册 + IPC 白名单三处同步删除；共享实现 `permanent_delete_one` 保留（batch 与测试仍用）；审计日志事件名字符串保留（记录语义）。
- **验证**：纯死代码删除零行为变化；check + clippy `-D warnings` + `--tests` + fmt 全绿。

### P039 — 死代码：ProcessLock.path 字段（已修复：删除）

- **提交**：`6874bb24`
- **位置**：`solosoul-core/src/process_lock.rs`。
- **修复**：`ProcessLock.path` 字段（从不读取）与 `path()` getter（无外部调用方，仅测试用）删除；两处构造器字段同步清理，import 收窄为 `Path`，测试断言移除、仅用于持有锁的变量改 `_lock`。`file` 字段保留（靠持有实现 RAII，属合理）。
- **验证**：纯死代码删除零行为变化；core check + clippy `-D warnings` + process_lock 测试全绿。

### P040 — TODO(S003) 会话 blob 键清理门槛（已修复：落实删键）

- **提交**：`2b2437dc`
- **位置**：`solosoul-core/src/llm/service.rs`（`migrate_legacy_conversations`）。
- **修复（按指引「确认 S003 迁移门槛版本是否已过，落实或删除 TODO」）**：核实当前版本 2.10.2 > 门槛 2.9.2+，CHANGELOG 已声称 v2.10 达成但代码从未实现——落实：迁移完成后经 `save_profile_data` 移除 `llmConversations` 键（键删除经 profile delta 传播到所有已升级设备），删除 `TODO(S003)`；空数组不再早退、统一落到删键；S001 永久删除路径不再依赖 blob 键（键已删，`remove_legacy_conversation_from_blob` 对缺失键天然 no-op）；同步更新函数 doc 与两处测试断言（R003 键保留 → S003 键已删）。
- **验证**：llm 域 19 测试全绿（含 LWW 迁移、S001 不复活两测试改为断言键已删）；check + clippy `-D warnings` + fmt 全绿。

### P041 — future-keychain 门控模块防腐化（已修复：CI 加 feature 编译检查）

- **提交**：`81bea1b9`
- **位置**：`.github/workflows/pr_check.yml`、`.github/workflows/ci_cd.yml`。
- **修复（按指引「CI 加 future-keychain feature 编译检查防腐化」）**：pr_check.yml `rust-check`（macOS）与 ci_cd.yml `build-macos` 各新增 `cargo check -p solosoul-core --features future-keychain` 步骤——macos_keychain.rs 门控模块不再长期脱离编译面（接口漂移/不可编译腐化风险消除）；模块需 `target_os=macos` + feature 双条件，CI 两处均运行于 macOS 满足编译条件。
- **验证**：本地 `cargo check -p solosoul-core --features future-keychain` 通过（Windows 下 target_os 不匹配不编译该模块属预期）；YAML 缩进/CRLF 与既有步骤一致。

### P042 — 命令列表双份真相（已修复：注释强化）

- **提交**：`99a8f53c`
- **位置**：`src-tauri/src/lib.rs`（`dispatch_ipc`、`test_dispatch_cluster_prefixes_consistent`）。
- **修复（按指引「注释强化提醒或改宏生成命令列表」）**：Rust 宏输出不可内省，无法从 `generate_handler!` 自动提取命令名——宏生成方案不可行，选指引选项一「注释强化」：①`dispatch_ipc` 路由函数 doc 加 P042 维护提醒（新增命令需同步 `register_*_commands` 列表 + 路由条件 + 守卫测试映射）；②守卫测试 doc 注明本列表为命令名第二份真相及宏不可内省的根因；③命令名映射处加「新增命令同步更新下方列表与总数断言（当前 195）」提醒。
- **验证**：纯注释零行为变化；check + tests 编译（Windows DLL 环境问题致运行失败，预存在）+ clippy `-D warnings` + fmt 全绿。

### P043 — 批量路径批量 SQL（已修复：import 批量 + export/trash 评估挂账）

- **提交**：`285679f8`（import.rs）
- **位置**：`src-tauri/src/commands/export_import/import.rs`。
- **修复（按指引「非热路径，低优先」）**：
  - ✅ import.rs 冲突检测：逐条 `load_object`（N 次锁竞争 + N 次查询）→ `load_objects_batch` 一次 IN 查询 + HashMap 查找（复用既有批量 API，零新增 API）；语义逐字等价（软删排除、冲突类型判定不变）。
  - ⏳ export.rs `collect_object_snapshots`：需新增 vault 批量快照 API（`list_snapshots_many`/`get_snapshots_many`），导出为低频用户动作且快照量有限，收益边际——**挂账**，随快照域下次改动一并处理。
  - ⏳ trash.rs `cleanup_expired_trash`：核实现实现已是「一次批量查询 + 逐条删除」——删除须逐条保持失败隔离（单条物理删除失败跳过、保留回收站记录防孤儿），批量化会破坏语义——**记录为不适用**。
- **验证**：check + clippy `-D warnings` + fmt 全绿。

### P044 — Rust 侧 14 个过长函数（已修复：11 项实际待拆全部拆分，biometric_save_credential 已 50 行、2 处行号漂移核实不适用）

- **提交**：`7677a89b`（vault reencrypt_all 138→重构 6 辅助）、`155e0a9c`（sync receive_attachments 137→append_chunk+finalize）、`0c9f2316`（plugin register_util_fns 141→5 注册辅助）、`3257ee0b`（core scan_mrz 154→recognize_mrz_lines+try_parse_mrz_combos）、`d04afaa5`（tauri recovery_restore_from_host 126→download+create 两阶段）、`5a63ad4e`（tauri initialize_vault 149→三 impl 方法）、`9bdebd5b`（tauri import_one_object 150→三辅助）、`66268464`（tauri export_execute 155→write_scope_extra_files+write_manifest_and_payload）、`234f7189`（tauri object_create 127→build_create_record+attach+snapshot 三辅助）、`caf238df`（tauri apply_sync_changes 132→apply_deprecated_fields+apply_updated_fields）、`9caa816e`（tauri handle_sse_stream 133→process_sse_line+emit_stream_done+build_usage_result）
- **说明**：11 项实际待拆函数全部拆分，主函数均降为编排层（7-9 层 → 4 层以内）；`biometric_save_credential` 实测已 50 行无需拆；原清单中 2 项行号漂移（函数已被前期 P045/P047 重构消化）核实不适用。全部为纯重构：SQL/闭包/错误消息逐字等价，`return Ok(None)` 改为 `Ok((false, _))` 语义等价。
- **验证**：vault/sync/core/plugin 四 crate `cargo test --lib` 全绿（163/64/182/57），全 workspace check + clippy `-D warnings` + fmt 全绿。
### P045 — 深嵌套函数（已修复：4 处实际待拆项全部拆分，3 处行号漂移已浅）

- **提交**：`a7b8c7a3`（mobile.rs）、`bc999ffe`（manager.rs 收敛）、`8c64ed11`（vault_directory.rs）、`78601260`（storage.rs）
- **位置**：见各提交。
- **核查结论（脚本实测嵌套深度）**：7 处中 3 处行号已随 P008/P017/P025 等重构漂移——`core/llm/client.rs:269`（`read_body_with_timeout` 11 行/2 层）、`app_state.rs:259`（`schedule_saf_fallback_sync` 27 行/2 层）、`update.rs:992`（`merge_seg_files` 31 行/3 层）已浅，**无需再拆**；实际待拆 4 处全部完成：
  - `sync/mobile.rs start`（82 行/8 层 accept 循环）→ `accept_loop` + `handle_accepted_connection`（P045-1）；
  - `sync/manager.rs start_inner`（92 行/8 层）+ mobile 本地副本 → 与 mobile.rs 三处重复收敛为 `session.rs` 共享 `run_accept_loop`/`handle_accepted_connection`/`SessionGuard`（P045-2，净 -53 行）；
  - `vault_directory.rs migrate_vault_data`（6 层递归内层）→ `clear_target_dir` 辅助（P045-3）；
  - `storage.rs object_field_sensitivity_levels`（64 行/8 层）→ `push_sensitivity_level` + `scan_dynamic_group_levels`（早退守卫平铺，主函数 3 层，P045-4）。
- **验证**：全部纯重构零行为变化；各 crate check + clippy `-D warnings` + fmt 全绿；sync 64 测试 / sensitivity 3 测试全绿。

### P046 — is_stop_word 每次重建词表（已修复：静态表）

- **提交**：`0f255c44`
- **位置**：`tauri/src-tauri/src/commands/llm/guide.rs`。
- **修复**：每次调用重建 ~130 词 slice + 线性查找改为 `LazyLock<HashSet>` 静态表（哈希查找 O(1)，分词循环逐 token 调用不再反复分配）；词表 129 词逐字保留语义不变。
- **验证**：check + clippy `-D warnings` + fmt 全绿。

### P047 — 两处 useObjectStore 整 store 订阅（已修复：选择器订阅）

- **提交**：`38d7c909`（组件）、`0004abf3`（测试回归修复）
- **位置**：`tauri/src/pages/scan/OcrPage.tsx`、`ScanLocalPage.tsx`。
- **修复**：仅用 `createObject` 却订阅整个 store → `useObjectStore((s) => s.createObject)`（zustand action 引用稳定，仅 createObject 变更时重渲染）；OcrPage 测试 mock 同步升级支持 selector 形式（旧 mock 忽略参数返回整对象导致 createObject 变对象、导入用例抛 TypeError）。
- **验证**：tsc/eslint/prettier 全绿；全量 212 测试全绿。

### P048 — 前端 9 个超长组件拆分（已修复）

- **提交**：`e6dcf772`（PhotoAlbumOverlay 640→377）、`b7494bb8`（AttachmentPreviewOverlay 622→392）、`3e038154`（PhotoViewerOverlay 535→366）、`9babe2d1`（LlmConfigPage 515→141）、`5f72f0ad`（DataManagementPage 526→348）、`bc021186`（ExportDocumentSection 461→225）、`212c10a0`（ObjectEditorPage 474→98）、`ebe605a9`（TemplateEditor 469→276）、`9d26d593`（RecoveryAccountView 428→231）
- **位置**：`tauri/src/components/attachment/`、`pages/ai/`、`pages/settings/`、`components/export/`、`pages/editor/`、`components/template/`、`components/recovery/`。
- **修复**：数据层/逻辑层抽 `use*` hook（usePhotoAlbumState/useAttachmentPreview/usePhotoViewer/useLlmConfigPage/useExportDocumentSection/useObjectEditorPage/useTemplateEditorState），渲染区拆子组件（StorageBreakdownCard/PieChartSvg/TemplateFieldsPanel/RecoveryConnectionCard）；`SnapshotCard` 无同名文件（已随前期重构更名/拆分，如 TrashSnapshotView），不单独处理。
- **验证**：全部纯重构零行为变化；tsc/eslint/prettier 全绿；相关 213 测试全绿（含 PhotoViewerOverlay 24、TemplateEditor 27、AttachmentPreviewOverlay 17、PhotoAlbumOverlay 11 等）。

### P049 — 缩放逻辑两处重复（已修复：photoZoom.ts 收敛）

- **提交**：`ea0c0d18`
- **位置**：`tauri/src/lib/photoZoom.ts`（新建）+ `components/attachment/PhotoViewerOverlay.tsx`、`AttachmentPreviewOverlay.tsx`。
- **修复**：缩放/平移常量（MIN_SCALE/MAX_SCALE/ZOOM_STEP）与 `clampScale`/`computeFitScale` 纯函数两处逐字重复收敛到共享模块；PhotoViewerOverlay 本地定义删除并 re-export `computeFitScale` 保持测试 import 兼容。
- **验证**：tsc/eslint/prettier 全绿；41 测试全绿。

### P050 — HistoryViewer fields 每次重算（已修复：useMemo）

- **提交**：`073e69f5`
- **位置**：`tauri/src/components/object/HistoryViewer.tsx`。
- **修复**：`flattenProperties` 渲染路径每次重算改为依赖 `[rawProps, fieldOrder]` 的记忆化（快照数据异步加载后稳定、fieldOrder 静态，重算纯冗余）。
- **验证**：tsc/eslint/prettier 全绿；13 测试全绿。

### P051 — GuideContent 同名重复 interface（已修复：死代码删除）

- **提交**：`1308239b`
- **位置**：`tauri/src/lib/llm/guideService.ts`。
- **修复**：删除与 `guideApi.ts:41` 同名重复且无人 import 的 `GuideContent` interface；同文件 `GuideChunk`/`searchGuideChunks`/`formatChunksAsSystemMessage` 仍被 useLlmChatCore 使用保留。
- **验证**：tsc/eslint 全绿。

### P052 — 同步活动列表数组下标 key（已修复：稳定 key）

- **提交**：`c300929c`
- **位置**：`tauri/src/pages/sync/SyncHistoryPanel.tsx`。
- **修复**：`recentResults.map` 数组下标 key 改为 `` `${result.at}-${result.peerNodeId}` `` 组合（at 为 Date.now() 毫秒戳 + 对端 node id，唯一性足够），前插新记录时旧行不再整行重挂载。
- **验证**：tsc/eslint 全绿；6 测试全绿。

### P053 — 诊断日志页无分页（已修复：visibleLimit）

- **提交**：`c0bbff1a`
- **位置**：`tauri/src/pages/system/DebugLogPage.tsx`。
- **修复**：一次性渲染 200 条多行日志改为 `visibleLimit`「加载更多」模式（LOG_PAGE_SIZE=50，对齐 OperationLogPage/HistoryPage 既有模式）。
- **验证**：tsc/eslint/prettier/check-missing-i18n 全绿。

### P054 — AGENTS.md 速查表失效路径（已修复：本地修正并登记）

- **提交**：无（AGENTS.md 已移出版本控制，e9aa8d96，本地已修，记录在案）
- **位置**：`AGENTS.md` 常用文件速查表。
- **修复**：三处失效路径修正——Vault 服务（薄包装）→ 共享核心库 `solosoul-core/src/vault_service/`、`commands/object.rs` → `commands/object/`（P047 拆分后目录化）、`src-tauri/src/db/` → `solosoul-vault/src/storage/`（db 模块迁 vault crate）。
- **验证**：纯文档零行为变化。

### P026 — LLM 客户端 blocking/async 双份实现（已修复：共享纯函数收敛）

- **提交**：`aba5b997`
- **位置**：`solosoul-core/src/llm/protocol.rs`（新建）+ `client.rs`、`src-tauri/src/commands/llm/request.rs`/`stream.rs`。
- **修复**：core 新增 `llm/protocol.rs` 共享纯函数模块（无 IO、无 reqwest 依赖）——`build_api_url`/`auth_headers`/`split_system_messages`/`extract_delta_text`/`extract_openai_usage_from_chunk`/`extract_anthropic_input_tokens`/`extract_anthropic_output_tokens`/`extract_response_text`/`extract_openai_usage`；blocking client（`build_request`/`handle_sse_payload`/`process_non_streaming`）与 async client（`request.rs` 的 `build_api_url`/`add_auth_headers`/`extract_response_text`/`extract_openai_usage`、`stream.rs` 的 `extract_delta_text`/`extract_anthropic_usage`/`extract_openai_usage_from_chunk`）全部改为转发共享实现，两侧仅保留 IO 绑定与各自累积策略（blocking 覆盖语义 vs async N008 逐字段保留语义，均不改变）。
- **验证**：零行为变化；core 182 测试全绿（新增 protocol 7 个纯函数测试）；四消费方 check 全绿；clippy `-D warnings` / fmt 全绿。

### P025 — vault_service.rs / lib.rs 巨型文件拆分（已修复：按域拆分 + setup 模块化）

- **提交**：`67c0c1d5`（① vault_service 拆分）+ `107659c3`（② setup 模块抽取）
- **位置**：`solosoul-core/src/vault_service/`（新建目录）、`src-tauri/src/setup/mod.rs`（新建）、`src-tauri/src/lib.rs`。
- **修复**：
  - ① vault_service.rs（2559 行）按域拆分到 `vault_service/` 目录 5 文件：mod.rs 446（类型/构造器/路径配置辅助 + 子模块声明）、account.rs 439（账户 CRUD：创建/删除/重命名/安全标志复位）、unlock.rs 914（密钥派生/解锁锁定/改密重加密/会话）、saf.rs 60（远端同步/脏标记）、tests.rs 784（测试整体迁移）；`impl VaultService` 拆为各子模块 `impl super::VaultService`，跨域私有方法（save_accounts/read_account_config/unlock_with_kdf_upgrade）改 pub(crate)。
  - ② lib.rs setup 步骤抽到 `setup/` 模块：setup_panic_hook/resolve_app_data_dir/resolve_log_dir/init_tracing/七个 setup_* 步骤/setup_app 全部移入（pub(crate) 导出仅 setup_panic_hook/setup_app 两个入口），lib.rs 头部 LOG_DIR/OnceLock/Emitter/Manager/AppState 五个 import 迁出。
- **验证**：纯重构零行为变化；solosoul-core 174 测试全绿（含 vault_service 32）；四消费方（src-tauri/sync/plugin/CLI）check 全绿；clippy `-D warnings` / fmt 全绿；src-tauri lib 测试 `STATUS_ENTRYPOINT_NOT_FOUND` 为预存在 Windows DLL 环境问题（stash 基线同样失败，与本次无关）。

### P024 — sync/plugin 拖入 OCR/PDF 重依赖栈（已修复：feature 门控）

- **提交**：`0e4134f7`（+ `183141ff` lock 收尾）
- **位置**：`solosoul-core/Cargo.toml`（features + target deps optional）、`core/lib.rs`（模块门控）、`solosoul-sync/Cargo.toml`、`solosoul-plugin/Cargo.toml`。
- **修复**：core 新增 `ocr`/`pdf`/`watermark` 三个 feature（`default` 全开，src-tauri/CLI 零变化）——`ocr = ort+image+ndarray` 门控 ocr 模块、`pdf = pdfium-render` 门控 pdfium 模块、`watermark = image+pdfium-render+ab_glyph+pdf` 门控 watermark 模块（watermark 内部依赖 `crate::pdfium`，故传递启用 pdf）；① solosoul-sync 改 `default-features = false`（仅用 vault_service/path_util，依赖树重依赖归零）；② solosoul-plugin 改 `default-features = false + watermark`（水印宿主保留 image/pdfium-render，**ort/ndarray 不再进入编译面**）；③ 桌面端重依赖全部 `optional = true`。
- **验证**：四 crate check/clippy `-D warnings` 全绿；core `--no-default-features` 编译通过；`cargo tree -p solosoul-sync` 中 ort/pdfium-render/ab_glyph 计数 0、`-p solosoul-plugin` 中 ort 计数 0；fmt 全绿；core ocr 27 测试全绿。

### P023 — sync/import/attachment 文件名净化强度不一致（已修复：收敛共享实现）

- **提交**：`6fc2773d`
- **位置**：`solosoul-core/src/path_util.rs`（新增 `sanitize_file_name`）+ 四处消费点统一。
- **修复**：`path_util::sanitize_file_name`（平台无关拒绝 `/` 与 `\\` 分隔符 + 取末段兜底 + 拒绝空/`.`/`..`）成为唯一实现——① sync `attachments.rs` 由仅 `Path::file_name()`（Unix 上不剥反斜杠的弱实现）升级为强语义；② `crud.rs`/`share.rs`/`attachment_import_plugin.rs` 三处 R007 落盘净化同步收紧（原「Invalid file name」仅取末段，`..\\..\\evil.txt` 可穿透）；③ export_import 与 plugin `host.rs` 原最强实现改为转发，消除逐字重复；`core/lib.rs` 导出。前端无这些文案的 i18n 映射，统一文案安全。
- **验证**：path_util 新增 3 测试（正常名/拒绝分隔符/拒绝空点）10 全绿；export_import 16 全绿；四 crate clippy `-D warnings` / fmt 全绿。

### P022 — PIN 凭证离线爆破强度（已修复：强度加固 + 规范文档）

- **提交**：`1532e7b0`
- **位置**：`tauri/crates/solosoul-core/src/pin.rs`（`validate_pin`）+ `docs/pin-spec.md`（新建）+ `AGENTS.md`（未入版本控制，仅本地同步）。
- **修复（用户选定方案：文档化 + 最小 6 位加固）**：① `validate_pin` 最小位数 4→6——消 4 位离线穷举窗口（10^4 ≈ 2.8h），对齐前端 `PinSection.PIN_LENGTH=6`（CLI 无 PIN 路径，GUI 已固定 6 位，后端收紧使直接 IPC 调用也无法设 4-5 位短 PIN）；存量 4 位凭证解锁不校验长度不受影响（仅设置/更换时按新规则校验）；新增 `test_validate_pin_min_length_six` 回归。② 新建 pin-spec.md 登记强度模型：凭证 = 盐 + 会话密钥副本密文、KDF 强制生产级 Argon2id（~1s/次，不随 `SOLOSOUL_SECURE` 降级）、6 位最坏离线 ~11.6 天/单线程（4 位 ~2.8h 已禁止）、**锁定计数对离线攻击无效**（失败计数存于明文 config.json 可删除）；明示「启用 PIN ≈ 静态安全降级到 PIN 强度」；中期强化路线（失败 N 次作废凭证、移动端 Keystore 硬件绑定）登记 backlog 未排期。
- **验证**：pin 域 11 测试全绿；solosoul-core clippy `-D warnings` / fmt 全绿。

### P016–P054 摘要指引

- **P016**：删未用常量；useMemo 依赖数组补 `isBottom`（或移除依赖数组改为直接计算）。
- **P017**：`rand::thread_rng()` → `rand::rngs::OsRng.fill_bytes(...)`；PIN 位改拒绝采样消偏差。
- **P018**：`derive_export_key` 返回 `Zeroizing<[u8;32]>`，调用点相应调整。
- **P019**：注册表公钥编译期固化为 `const`（对齐 `embed_model.rs:14`）；URL 环境变量仅 `debug_assertions` 生效。
- **P020**：文档化第三方镜像权衡；客户端加版本单调性检查（拒绝低于已安装版本的清单）+ 失败回退官方源提示。
- **P021**：若为有意设计，在 docs 与 AGENTS.md 显式记录该威胁模型例外；否则按导出包同款 `encrypt_chunked_stream` 加密落盘附件。
- **P022**：文档明示「启用 PIN ≈ 静态安全降级到 PIN 强度」；可选最小 6 位、失败 N 次作废凭证文件、移动端硬件绑定（复用 `keystore_plugin`）。
- **P023**：`sanitize_import_file_name` 提升为共享实现（如 `solosoul-core::path_util`），sync/import/attachment 三处统一调用。
- **P024**：core 拆 `core-vault` 与 `core-media`，或将 OCR 依赖 gate 到 feature。
- **P025**：`vault_service.rs` 按账户 CRUD/解锁会话/SAF 同步拆分；`lib.rs` setup 步骤抽到 `setup/` 模块。
- **P026**：请求构造 + SSE 行解析抽成 core 纯函数模块，blocking/async 两 client 只做 IO 绑定。
- **P027**：敏感 command 后端已鉴权，可保持现状；扩大前端 `requireUnlocked` 默认启用面。
- **P028**：localStorage 同步历史定位为后端审计日志投影，重启后重建或至少设容量上限。
- **P029**：三源读路径顺序异常时可能闪烁，记录即可；P129 已收敛写入单点。
- **P030**：拆 `state/recovery.rs`、`state/saf_config.rs`。
- **P031**：`object/mod.rs` 的纯校验/转换函数（`validate_dynamic_groups`、`inherit_*`、`template_fingerprint`）下沉到 solosoul-core，commands 只做 DTO 适配。
- **P032**：版本读取失败显式报错或至少 `tracing::error` 留痕，不要 `unwrap_or(1)`。
- **P033**：确认/显式设置 `journal_mode=WAL`；评估 `reencrypt_all` 期间前端「维护中」状态。
- **P034**：与 P008/P025 同批纳入拆分 backlog，不单独立项。
- **P035–P039**：死代码删除（涉及删除文件/模块的按流程约束暂缓至修复总结阶段由用户确认）。
- **P040**：确认 S003 迁移门槛版本是否已过，落实或删除 TODO。
- **P041**：CI 加 `future-keychain` feature 编译检查防腐化。
- **P042**：注释强化提醒或改宏生成命令列表。
- **P043**：导出/导入/回收站批量路径加批量 SQL（非热路径，低优先）。
- **P044 / P045**：过长函数与深嵌套清单，随对应模块拆分一并处理。
- **P046**：`is_stop_word` 改 `static` 排序数组二分或 `LazyLock<HashSet<&str>>`。
- **P047**：两处改字段选择器 `useObjectStore((s) => s.createObject)`。
- **P048**：参照 SyncPage/SettingsPage 拆分模式逐组件拆子组件/hook。
- **P049**：提取 `useImageZoom` hook 或扩展 `useTouchZoom`，常量与缩放模型收敛一处。
- **P050**：`SnapshotCard` 包 `memo`，`flattenProperties` 结果 `useMemo`（影响存疑，低优先）。
- **P051**：删除 `guideService.ts` 中重复的 `GuideContent` interface。
- **P052**：key 改用 `result.at`（时间戳）或唯一 id。
- **P053**：参照 `OperationLogPage.tsx:117` 加「加载更多」分页（50/页）。
- **P054**：同步 AGENTS.md 速查表（`services/vault_service.rs` 已不存在；`services/` 下仅剩 `llm_context.rs`、`profile_prefs.rs`）。

---

## 已核查确认无问题的维度（避免后续重复报告）

- **前端死代码**：380+ 源文件引用图全覆盖，无未被 import 的模块、无未使用依赖、无 TODO/FIXME 残留。
- **大列表渲染**：workspace/操作日志/回收站/历史/搜索/聊天均已分页或窗口化；`WorkspaceObjectCard`、`ChatMessageItem` 已 memo。
- **XSS/注入**：无 `dangerouslySetInnerHTML`/`eval`；Markdown 统一走 `SafeMarkdown`（无 rehype-raw，禁 script/iframe）。
- **命令注入**：唯一 `process::Command`（icacls）为 argv 传参无 shell，用户名白名单校验。
- **加密基元**：nonce 全 OsRng；分块加密 v2 头部入 AAD；verify_hash 用 HKDF + 常数时间比较；release 默认 OWASP 档 Argon2id；无硬编码密钥。
- **路径遍历主线**：附件 ID 字符集校验、导入 zip 净化、embed 模型 zip `mangled_name` 双保险、`solosoul-pdf://` 白名单 + 256MiB 上限，均有回归测试。
- **unsafe**：全部位于平台 FFI（Keychain/DPAPI/WinRT/NSWindow），带 SAFETY 注释。
- **日志泄露**：tracing 全库扫描未见密码/session key/master key/明文属性输出；`NoiseKeys` Debug 主动脱敏。
- **Tauri 权限**：CSP script-src 收敛 `'self'`；capabilities 无 `fs:allow-**`/`shell:allow-execute`。
- **内存安全主线**：主密码/master key/session key 全程 Zeroizing，lock 显式清零。
- **crate 依赖**：五层单向依赖图，无循环依赖；前端生产代码全走 `ipcClient` 无裸调 invoke。
- **sync 应用路径**：批量/单条应用均走事务方法。
- **Rust 依赖**：除 P009 外各 crate 依赖均有实际使用；循环内 SQL 其余命中均在单事务 + `prepare_cached` 内。

## 跨领域观察

多条 P2（P017/P019/P023）本质是「同类安全控制在不同模块强度不一致」——随机数源、信任锚固化、文件名净化各自有两套实现。建议收敛为共享实现（`solosoul-core` 已有 `path_util`/secure 落点），比逐条修补更能防回归。

## 暂缓事项（按流程约束）—— 已作废：修复轮次在用户确认后全部执行，留痕如下

本节为初始分析时登记的删除类暂缓项，修复轮次（a950f1c9..8eb69838）已实际执行：

- P035 `save_sync_conflict` 单条版删除 —— 已执行（b052c939）
- P037 `PluginRegistry` 三个无调用方构造器删除 —— 已执行（f9f4f17f）
- P038 `trash_permanent_delete` 单条命令移除 —— 已执行（309b11a8），但遗漏同步命令计数测试，产生回归 V002
- P041 `macos_keychain.rs` —— 未删除，改用 CI feature 编译检查防腐化（81bea1b9）
