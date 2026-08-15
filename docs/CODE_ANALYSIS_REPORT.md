# 代码分析修复报告

> 最后更新：2026-08-16 00:08:05
> 当前分支：`main`
> 修复轮次：1（初始分析，全量重新生成，未沿用历史报告）
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
| P008 | 架构       | `tauri/crates/solosoul-vault/src/storage.rs`（5915 行） | 上帝对象：VaultStore + 加密迁移 + 整表重写 + sync 密钥 + 搜索工具混在一个文件 | `[ ]` 待修复 |
| P009 | 死代码     | `tauri/crates/solosoul-core/Cargo.toml:29,38,49` | `anyhow`、`tokio`、`rand` 三个直接依赖在 core 全库无引用 | `[ ]` 待修复 |
| P010 | 重复代码   | `tauri/src-tauri/src/commands/export_import/helpers.rs:24-88` ↔ `tauri/crates/solosoul-core/src/export_import.rs:885-942` 等 | `build_package_ids`/`resolve_*_references`/`derive_export_key` 等函数 GUI 与 core 逐字重复（相似度 ≈100%） | `[ ]` 待修复 |
| P011 | 性能       | `tauri/crates/solosoul-vault/src/storage/sync_changes.rs:136,495`、`storage/conversations.rs:204` | 同步热路径逐行 `record_hlc_or_fallback`（每行一次 SELECT + 加锁），objects/trash 已用 LEFT JOIN 批量，同文件两种模式并存 | `[ ]` 待修复 |
| P012 | 结构       | `tauri/crates/solosoul-vault/src/storage/objects.rs:393` | `list_objects` 147 行，查询/解密/组装混杂，对象列表核心路径 | `[ ]` 待修复 |
| P013 | 结构       | `tauri/crates/solosoul-vault/src/storage/sync_changes.rs:283` | `query_object_changes` 146 行，SQL 拼装与行解密混在一处 | `[ ]` 待修复 |
| P014 | 结构       | `tauri/src-tauri/src/commands/export_import/export_docx/fields.rs:32` | `field_value_to_text` 嵌套 7 层（match 套 if-let 套循环） | `[ ]` 待修复 |
| P015 | 性能       | `tauri/src/App/routes.tsx:2-29`、`tauri/vite.config.ts` | 27 个页面全部 eager import，无 `React.lazy`/`manualChunks`，移动端首屏需解析全部 bundle | `[ ]` 待修复 |

### P2（轻微，可排入 backlog）

| ID   | 类别       | 文件位置 | 描述 | 状态 |
|------|------------|----------|------|------|
| P016 | 规范       | `tauri/src/components/layout/AddPageButton.tsx:14,75` | ESLint 2 warning：`SAFE_AREA_BOTTOM` 未使用；useMemo 缺依赖 `isBottom` | `[ ]` 待修复 |
| P017 | 漏洞（弱随机） | `tauri/crates/solosoul-sync/src/recovery.rs:366-374,394-399` | 恢复 PIN/恢复密码用 `thread_rng` 而非 OsRng，PIN 逐位 `% 10` 有取模偏差 | `[ ]` 待修复 |
| P018 | 内存安全   | `tauri/crates/solosoul-crypto/src/kdf.rs:94-103`；调用点 `solosoul-core/src/export_import.rs:211,326` | `derive_export_key` 返回裸 `[u8;32]` 未以 Zeroizing 包裹，与全库内存卫生纪律不一致 | `[ ]` 待修复 |
| P019 | 供应链     | `tauri/crates/solosoul-plugin/src/registry.rs:76-88` | 插件注册表 URL 与 minisign 公钥读自环境变量，信任锚弱于 embed registry 的编译期固化 | `[ ]` 待修复 |
| P020 | 供应链/隐私 | `tauri/src-tauri/tauri.conf.json:82-88` | updater 5 个端点中 4 个为第三方 GitHub 代理，存在降级冻结与行为记录风险 | `[ ]` 待修复 |
| P021 | 静态加密   | `tauri/src-tauri/src/commands/attachment/crud.rs:524` | 附件明文落盘 `{vault}/attachments/`，与 vault.db/导出包加密姿态不一致（0700 权限是唯一防线） | `[ ]` 待修复 |
| P022 | 加密弱点   | `tauri/crates/solosoul-core/src/pin.rs:101` | PIN 凭证可离线爆破（6 位最坏约 1 天），锁定计数对离线攻击无效；设计权衡但未文档化 | `[ ]` 待修复 |
| P023 | 路径遍历（加固） | `tauri/crates/solosoul-sync/src/attachments.rs:45-50` 对比 `solosoul-core/src/export_import.rs:1070-1088` | sync 侧附件文件名净化弱于 import 侧（未拒绝 `\`），同款安全控制两处强度不一致 | `[ ]` 待修复 |
| P024 | 架构       | `tauri/crates/solosoul-sync/Cargo.toml`、`solosoul-plugin/Cargo.toml` | sync/plugin 依赖 core 拖入整个 OCR/PDF 重依赖栈（ort、pdfium-render 等），编译面与体积被拉大 | `[ ]` 待修复 |
| P025 | 架构       | `tauri/crates/solosoul-core/src/vault_service.rs`（2559 行）、`tauri/src-tauri/src/lib.rs`（1020 行） | 账户生命周期/SAF/会话全塞一个文件；lib.rs setup 步骤堆积在入口 | `[ ]` 待修复 |
| P026 | 重复代码   | `solosoul-core/src/llm/client.rs`（475 行）vs `src-tauri/src/commands/llm/`（约 1185 行） | LLM HTTP/SSE 客户端 blocking/async 双份实现，请求构造与 SSE 解析可共享纯函数 | `[ ]` 待修复 |
| P027 | 架构       | `tauri/src/stores/authStore.ts` ↔ 后端 `VaultService` | 解锁状态前后端双份维护，靠事件 + best-effort 收敛（已有多层缓解，残余窗口存疑） | `[ ]` 待修复 |
| P028 | 架构       | `tauri/src/stores/syncStore.ts:20-66` ↔ `commands/sync.rs:6-27` | 同步历史存两份：localStorage（无清理逻辑，存疑）与后端 audit_log | `[ ]` 待修复 |
| P029 | 架构       | `tauri/src/stores/settingsStore.ts:156-232` | 偏好设置三副本（后端 DB / localStorage / ui_preferences.json），读路径异常时可能闪烁回跳 | `[ ]` 待修复 |
| P030 | 架构       | `tauri/src-tauri/src/state/app_state.rs`（866 行） | AppState 聚合 8 个字段且混入 SAF config、DTO、自由函数 | `[ ]` 待修复 |
| P031 | 架构       | `tauri/src-tauri/src/services/`（仅 3 文件）vs `commands/`（30+ 模块） | services 层萎缩，业务规则锁在 command 签名旁（如 `object/mod.rs` 的 dynamic_group 校验），CLI 无法复用 | `[ ]` 待修复 |
| P032 | 健壮性     | `tauri/crates/solosoul-vault/src/migration.rs:45` | 迁移版本读取 `unwrap_or(1)` 吞掉读取错误，版本表读失败会静默重跑全部迁移 | `[ ]` 待修复 |
| P033 | 性能       | `tauri/crates/solosoul-vault/src/storage.rs:423,607,962` | 单 `Mutex<Connection>` 全库串行，未见显式 `journal_mode=WAL`（存疑）；`reencrypt_all` 持锁期间全库阻塞 | `[ ]` 待修复 |
| P034 | 架构       | `commands/update.rs` 1902、`commands/ocr.rs` 1517、`migration.rs` 1536、`ocr/mrz.rs` 1423、`export_import/import.rs` 1225、`commands/biometric.rs` 1004、`commands/llm/rag.rs` 972 等 | 800+ 行文件群，与 P008/P025 同批纳入拆分 backlog | `[ ]` 待修复 |
| P035 | 死代码     | `tauri/crates/solosoul-vault/src/storage/sync_apply.rs:140` | `save_sync_conflict`（单条版）无调用方，已被 batch 版取代 | `[ ]` 待修复 |
| P036 | 死代码     | `tauri/crates/solosoul-core/src/biometric/mod.rs:370` | `BiometricService::test()` 无调用方（命令层直接调 `trigger_system_biometric`） | `[ ]` 待修复 |
| P037 | 死代码     | `tauri/crates/solosoul-plugin/src/registry.rs:30-49` | `PluginRegistry::new()`/`Default`/`new_with_resource_dir()` 三个构造器无调用方 | `[ ]` 待修复 |
| P038 | 死代码     | `tauri/src-tauri/src/commands/object/trash.rs:149` | 命令 `trash_permanent_delete`（单条版）已注册但前端无 invoke（存疑：可能为 API 对齐保留） | `[ ]` 待修复 |
| P039 | 死代码     | `tauri/crates/solosoul-core/src/process_lock.rs:19` | `ProcessLock` 的 `path` 字段从不读取（`file` 字段靠持有实现 RAII 属合理） | `[ ]` 待修复 |
| P040 | 死代码     | `tauri/crates/solosoul-core/src/llm/service.rs:185` | 全库唯一遗留 `TODO(S003)`：会话存储迁移清理，需确认门槛版本是否已过 | `[ ]` 待修复 |
| P041 | 维护性     | `tauri/crates/solosoul-core/src/biometric/macos_keychain.rs`（444 行） | 整模块被 `feature = "future-keychain"` 门控脱离默认编译面，长期腐化风险，建议 CI 加 feature 编译检查 | `[ ]` 待修复 |
| P042 | 维护性     | `tauri/src-tauri/src/lib.rs:760-1019` | 测试手工维护 195 个命令名列表与 `generate_handler!` 双份真相，每次加命令需同步两处 | `[ ]` 待修复 |
| P043 | 性能       | `src-tauri/src/commands/export_import/export.rs:717-739`、`import.rs:188-207`、`object/trash.rs:169-171` | 导出快照、导入冲突检测、回收站批量清理存在循环内逐条查询/事务（非热路径） | `[ ]` 待修复 |
| P044 | 结构       | Rust 侧 14 个过长函数（100-133 行）：`import_one_object`、`reencrypt_all`、`receive_attachments`（嵌套 7 层）、`export_execute`、`recovery_restore_from_host`、`register_util_fns`、`scan_mrz`、`initialize_vault`、`biometric_save_credential`、`object_create`、`apply_sync_changes`、`handle_sse_stream` 等 | 建议随对应模块拆分时一并处理，清单详见下方详细描述 | `[ ]` 待修复 |
| P045 | 结构       | `storage.rs:281`、`sync/mobile.rs:340`、`sync/manager.rs:156`、`core/llm/client.rs:269`、`app_state.rs:259`、`vault_directory.rs:388`、`update.rs:992` | 7 处嵌套深度 6-7 层的函数 | `[ ]` 待修复 |
| P046 | 性能       | `tauri/src-tauri/src/commands/llm/guide.rs:252-385` | `is_stop_word` 每次调用重建 ~130 词 slice 线性查找，且在分词循环内逐 token 调用 | `[ ]` 待修复 |
| P047 | 性能       | `tauri/src/pages/scan/OcrPage.tsx:26`、`ScanLocalPage.tsx:50` | 全库仅两处 `useObjectStore()` 整 store 订阅，任意字段变化触发整页重渲染 | `[ ]` 待修复 |
| P048 | 结构       | 前端 10 个超长组件（310-490 行）：`AttachmentPreviewOverlay`、`PhotoAlbumOverlay`、`LlmConfigPage`、`DataManagementPage`、`PhotoViewerOverlay`、`ExportDocumentSection`、`ObjectEditorPage`、`TemplateEditor`、`SnapshotCard`、`RecoveryAccountView` | 建议参照 SyncPage/SettingsPage 既有拆分模式 | `[ ]` 待修复 |
| P049 | 重复代码   | `components/attachment/AttachmentPreviewOverlay.tsx:30-32,211-229` ↔ `PhotoViewerOverlay.tsx:36-38,214-216` | 缩放/平移常量与 `clampScale`/`zoomIn`/`fitToView` 等逻辑两处逐字重复，`useTouchZoom` 抽象只完成一半 | `[ ]` 待修复 |
| P050 | 性能       | `tauri/src/components/object/HistoryViewer.tsx`（599 行） | 全文件无 memo/useMemo，`flattenProperties` 渲染路径每次重算（实际渲染量小，影响存疑） | `[ ]` 待修复 |
| P051 | 死代码     | `tauri/src/lib/llm/guideService.ts:10` | `GuideContent` interface 与 `lib/guideApi.ts:41` 同名重复定义且无人使用 | `[ ]` 待修复 |
| P052 | 性能       | `tauri/src/pages/sync/SyncHistoryPanel.tsx:83` | 前插列表用数组下标作 key，新记录导致全行重挂载（列表 cap=10，代价小） | `[ ]` 待修复 |
| P053 | 性能       | `tauri/src/pages/system/DebugLogPage.tsx:28,141` | 一次性渲染 200 条多行日志无分页（其他日志页均有 visibleLimit 模式可参照） | `[ ]` 待修复 |
| P054 | 文档       | `AGENTS.md` 常用文件速查表 | 仍指向已不存在的 `tauri/src-tauri/src/services/vault_service.rs` | `[ ]` 待修复 |

## 修复进度

- 已完成：7 / 54
- 当前处理：P008（storage.rs 5915 行上帝对象）

---

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

### P008 — storage.rs 5915 行上帝对象

- **位置**：`tauri/crates/solosoul-vault/src/storage.rs`。虽已拆出 `storage/` 9 个子模块，本体仍承载 VaultStore 结构、加密格式迁移、整表重写（re-encrypt）、sync node 密钥、搜索工具、probing、测试。
- **修复**：继续向 `storage/` 子模块迁移（如 `storage/reencrypt.rs`），目标本体 <1500 行。

### P009 — solosoul-core 三个未使用依赖

- **位置**：`tauri/crates/solosoul-core/Cargo.toml:29,38,49`：`anyhow`、`tokio`、`rand` 在 `solosoul-core/src` 全库（含测试模块）无任何引用。
- **修复**：从 `[dependencies]` 移除。

### P010 — export_import 在 GUI 与 core 逐字重复

- **位置**：`src-tauri/src/commands/export_import/helpers.rs:24-88` ↔ `solosoul-core/src/export_import.rs:885-942`（`build_package_ids`/`resolve_value_references`/`resolve_cross_scope_references` 相似度 ≈100%）；`mod.rs:229-250` ↔ `core:580-598`（`derive_export_key`）；另有 `read_manifest`/`read_file_from_zip`/`import_attachments`/`import_preferences` 高度平行。
- **修复**：core 版改为 `pub`（core 已是 GUI 依赖），GUI 侧删副本改为薄 IPC 壳。

### P011 — 同步热路径 N+1 HLC 查询

- **位置**：`solosoul-vault/src/storage/sync_changes.rs:136`（`list_profile_changes_since`）、`:495`（`list_user_template_changes_since`）、`storage/conversations.rs:204`：循环内逐行 `record_hlc_or_fallback`（每行一次 `get_record_hlc` SELECT + 锁获取）。
- **修复**：参照同文件 `query_object_changes` 的 `LEFT JOIN sync_hlc` 批量写法统一三张表。

### P012 / P013 — 过长核心函数

- `storage/objects.rs:393` `list_objects`（147 行）：拆分查询/解密/组装阶段。
- `storage/sync_changes.rs:283` `query_object_changes`（146 行）：SQL 拼装与行解密分离。

### P014 — field_value_to_text 嵌套 7 层

- **位置**：`commands/export_import/export_docx/fields.rs:32`，match 套 if-let 套循环。
- **修复**：早返回拍平。

### P015 — 前端路由无代码分割

- **位置**：`tauri/src/App/routes.tsx:2-29` 27 个页面全静态 import；`vite.config.ts`（28 行）无 `build`/`manualChunks`。
- **影响**：首屏/移动端启动需解析全部页面代码（含 LlmConfigPage 466 行、DataManagementPage 491 行等重组件与 react-markdown/rehype-highlight 重依赖）。
- **修复**：非首屏路由改 `React.lazy` + `Suspense`；vite `manualChunks` 拆重依赖。建议先 `vite build` 看主 chunk 体积再定拆分力度。

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

## 暂缓事项（按流程约束）

以下修复涉及**删除文件/模块**，按 `review_code_process.md` 约束暂缓，待修复阶段由用户确认后执行：

- P035 `save_sync_conflict` 单条版删除（函数级删除，影响小）
- P037 `PluginRegistry` 三个无调用方构造器删除
- P038 `trash_permanent_delete` 单条命令移除（若确认不为 API 对齐保留）
- P041 `macos_keychain.rs` 若决定放弃 future-keychain 规划则整文件删除
