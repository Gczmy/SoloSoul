# 代码分析修复报告

> 最后更新：2026-08-19 17:47:35
> 当前分支：`main`
> 修复轮次：1（初始分析，全新生成，未沿用历史报告）
> 本轮范围：仅分析并生成报告，**未执行任何修复**（应用户要求）。

---

## 阶段 0：基线检查结果

| 检查项 | 命令 | 结果 |
|--------|------|------|
| TypeScript 类型检查 | `npx tsc --noEmit` | ✅ 通过 |
| ESLint | `npm run lint` | ✅ 通过 |
| 前端单元测试 | `npm run test`（Vitest） | ✅ 99 个测试文件 / 832 个测试全部通过 |
| Rust 单元测试 | `cargo test` | ✅ 通过（含 solosoul-vault 163 个测试，0 失败） |
| Markdown chunk 边界 | `node scripts/check-markdown-chunk-boundary.mjs` | ✅ 通过 |
| ACL 一致性 | `python3 scripts/check_acl_consistency.py` | ✅ 通过 |
| Pref keys 同步 | `python3 scripts/check_pref_keys_sync.py` | ✅ 通过 |
| Rust 格式化 | `cargo fmt --check` | ❌ **失败**（见 P008） |
| Rust Clippy（CI 配置） | `cargo clippy -- -D warnings` | ❌ **失败**（见 P009） |

> 结论：`npm run check-all` 当前会在 fmt / clippy 两步失败，CI 基线为红。
> Git 状态：仅两个未跟踪的 Android bugreport zip（`tauri/bugreport-*.zip`），无未提交的代码改动。

---

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别       | 文件位置 | 描述 | 状态 |
|------|--------|------------|----------|------|------|
| P001 | P1 | 安全 | `tauri/crates/solosoul-core/src/objects.rs:1043-1045` | Vault 附件以明文落盘，仅导出/同步时才加密，与零知识定位不符 | `[x]` 已修复（附件加密落盘，SOLC 头 + HKDF 派生密钥，写入/读取全链路覆盖） |
| P002 | P1 | 前端缺陷 | `tauri/src/stores/objectStore.ts:194-196` | `updateObject` 吞错不抛出，编辑保存失败被误报「保存成功」并退出页面，数据静默丢失 | `[x]` 已修复（f585f43f） |
| P003 | P1 | 前端缺陷 | `tauri/src/stores/settingsStore.ts:491-523` | `addCustomPage` 失败仍无条件 `return newPage`，调用方导航到后端不存在的页面 | `[x]` 已修复（d8648b3f） |
| P004 | P1 | 前端缺陷 | `tauri/src/pages/ai/useLlmConfigPage.ts:190-228` | 本地 Embedding 开关/选模型 invoke 无 try/catch，失败后前后端状态漂移 | `[x]` 已修复（76cffe3d） |
| P005 | P1 | 性能 | `tauri/src-tauri/src/commands/object/snapshot.rs:466-484` | 回收站子对象列表循环内逐条 `get_trash_item`（每次附带整条 data 解密），而 summary 已含 `original_id`，属纯浪费 | `[x]` 已修复（54b02ac8） |
| P006 | P1 | 性能 | `tauri/src/pages/home/HomePage.tsx:235-255` + `tauri/src-tauri/src/commands/attachment/tree.rs:55` | 首页角标每次返回都调用 `attachment_list_all` 全表解密，仅为渲染两个计数 | `[x]` 已修复（df464e69） |
| P007 | P1 | 代码质量 | `tauri/src-tauri/src/commands/attachment/crud.rs:47-69` ↔ `tauri/crates/solosoul-core/src/export_import.rs:64-84` | `AttachmentMeta` 结构体双定义，序列化契约靠注释维持，存在漂移风险 | `[x]` 已修复（8824c261） |
| P008 | P1 | 规范 | `tauri/crates/solosoul-core/src/vault_service/account.rs:91,118` | `cargo fmt --check` 失败（2 处 tracing 宏格式），CI 基线红 | `[x]` 已修复（9054d0b1） |
| P009 | P1 | 规范 | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:334-335`；`vault_service/tests.rs:26` | `cargo clippy -- -D warnings` 失败：2 处 `needless_borrows_for_generic_args`；`--all-targets` 下另有 1 处 unused variable | `[x]` 已修复（346d7563） |
| P010 | P2 | 安全 | `tauri/src-tauri/src/commands/attachment/share.rs:33-41` | 分享副本明文残留 `temp_dir()/solosoul_share/`，永不清理 | `[x]` 已修复（b95b4ace） |
| P011 | P2 | 安全 | `tauri/src-tauri/src/commands/vault.rs:7-18`（注册于 `lib.rs:55`） | 遗留 `unlock` IPC 命令 `password: String` 未 `Zeroizing` 包装；前端已无调用（仅测试 mock 引用） | `[x]` 已修复（686d807c） |
| P012 | P2 | 安全 | `tauri/src-tauri/src/commands/auth.rs:159-176` | `verify_password` 不计失败、不触发阶梯锁定，构成无限速密码验证 oracle | `[x]` 已修复（937446b7） |
| P013 | P2 | 性能 | `tauri/src-tauri/src/commands/export_import/export.rs:751-761` | 导出时快照收集为 N+M 嵌套查询（每对象 1 次 list + 每快照 1 次 get） | `[ ]` 待修复 |
| P014 | P2 | 性能 | `tauri/src/pages/settings/useTrashPage.tsx:145-168` | 回收站批量恢复逐项串行 IPC，与批量删除的批量入参不一致 | `[ ]` 待修复 |
| P015 | P2 | 代码质量 | `tauri/src/pages/ai/useLlmConfigPage.ts:369,413`；`tauri/src/components/llm-config/ProviderManagerPanel.tsx:280` | API key 哨兵 `'••••••••'` 字面量硬编码三处，与 `lib/masking.ts:14` 的 `MASK_PLACEHOLDER` 脱钩 | `[ ]` 待修复 |
| P016 | P2 | 代码质量 | `tauri/src/hooks/useAttachmentManagerBatchOps.ts:105-107` | 批量附件下载 catch-all 将任意异常误判为「用户取消」，无日志 | `[ ]` 待修复 |
| P017 | P2 | 死代码 | `tauri/crates/solosoul-core/src/export_import.rs:129-131` | `ExportError::Crypto` 变体从未被构造 | `[ ]` 待修复 |
| P018 | P2 | 死代码 | `tauri/scripts/tokenize-fonts.mjs`、`tokenize-icons.mjs`、`fix_invoke_keys.cjs`、`revert_invoke_keys.cjs` | 4 个一次性 codemod 脚本残留，package.json/CI/文档均无引用 | `[x]` 已修复（094e75b8，用户确认删除） |
| P019 | P2 | 重复代码 | `tauri/src-tauri/src/commands/llm/provider.rs:25-55` ↔ `llm/unified_chat.rs:30-59` | LLM provider 合并逻辑跨文件复制（~20 行，>80% 相似） | `[ ]` 待修复 |
| P020 | P2 | 重复代码 | `tauri/crates/solosoul-vault/src/storage/metadata.rs:535-560` ↔ `sync_changes.rs:475-500` | `user_templates` 行解密映射代码几乎逐字重复 | `[ ]` 待修复 |
| P021 | P2 | 重复代码 | `tauri/src-tauri/src/commands/export_import/export_docx/docx.rs:110-129` ↔ `text.rs:29-48` | 导出文档「元信息段」构建块逐字相同 | `[ ]` 待修复 |
| P022 | P2 | 可维护性 | 见下文 Top 10 表 | 超长函数/组件 10 个（357–391 行） | `[ ]` 待修复 |
| P023 | P2 | 可维护性 | `tauri/src/hooks/useDragToAttach.ts:190-234` 等 | 深层嵌套热点（控制流 ≥5 层，JSX brace 深度最高 11） | `[ ]` 待修复 |

## 修复进度

- 已完成：13 / 23（P008、P009、P002、P003、P004、P018、P001、P005、P006、P007、P010、P011、P012）
- 当前处理：P013

---

## 详细问题描述与修复指引

### P001（P1 安全）Vault 附件明文落盘（已完成）

**修复方案（用户确认：完整加密落盘）**：附件以 `encrypt_chunked_stream`（SOLC magic 头）加密落盘，读取时检测 magic——SOLC 密文流式解密、旧明文直读（零迁移兼容）。密钥 = `HKDF(session_key, b"solosoul:attachments:at-rest", b"solosoul:attachments:at-rest:v1")`（与数据库密钥域分离，同密码跨设备派生同一密钥，同步无需分发）。

**改动**（12 文件 + 1 新模块）：
1. `attachment_crypto.rs`（新）— 密钥派生 / 流式加密 / 明文兼容解密 / magic 检测，+6 单测；
2. `unlock.rs` — `VaultService::attachment_encryption_key()`（+2 单测）；`change_password` 改密后附件目录递归重加密（+1 集成单测）；
3. 写入点：`crud.rs`（copy_to_vault 加密）、`attachment_import_plugin.rs`（Android importContentUri 复制后就地加密）、`import.rs`（导入落盘先解密 ZIP → 临时明文 → 加密写盘）、`objects.rs add_attachments`/`copy_file_to_vault`（CLI 用，`attachment_key: Option<&[u8; 32]>`）；
4. 读取点：`mod.rs`（download/open 解密）、`share.rs`（分享解密）、`attachment_import_plugin.rs`（export_content_uri / export_tree_uri 先解密到临时明文再交 Kotlin）、`fs.rs`（三个预览命令 SOLC 自动解密，图片改内存解码）、`preview_pdf_protocol.rs`（PDF 协议解密）、`export.rs`（导出先解密源再加密进 ZIP）、`solosoul-plugin`（FieldResolver 注入密钥，插件工作区复制前解密）；
5. CLI 同步：`solosoul_cli/commands/attachment.rs`（add_attachments 传密钥）、`plugin.rs`（run 传密钥）；solosoul-core `export_vault` 检测到密文附件时明确报错（CLI 无密钥，防双重加密损坏包）。

**验证**：workspace 测试全过（src-tauri 444 / core 194 / vault 163 / plugin 57…），clippy --all-targets 无警告，fmt 干净，solosoul-cli 编译通过。Kotlin 侧零改动（Rust 传临时明文路径）。

### P002（P1 前端缺陷）`updateObject` 吞错 → 假成功提示

`src/stores/objectStore.ts:194-196`：

```ts
} catch (err) {
  set({ error: String(err), isLoading: false });   // 无 throw
}
```

同文件 `createObject`（:161-164）catch 后会 `throw err`。唯一调用方 `useObjectEditorPage.ts:422-428` 依赖异常进入 onError 分支；当前 `object_update` 失败时仍执行 `onSuccess(t('common:object_saved'))` 并 `navigate(-1)`，编辑内容静默丢失。
**修复建议**：catch 中补 `throw err`，与 `createObject` 对齐。

**修复记录（f585f43f）**：`updateObject` catch 补 `throw err`（注释说明调用方依赖），+1 单测验证失败抛错且 store.error 置位。objectStore 11 测试全过。

### P003（P1 前端缺陷）`addCustomPage` 失败仍返回成功值

`settingsStore.ts:491-523`：catch 里回滚了乐观更新（:520），但 `return newPage` 无条件执行（:522）且无 toast。调用方 `AddPageButton.tsx:125-130` 的 `.then((page) => onCreate(page))` 在失败时同样触发，UI 进入后端不存在的页面（刷新后消失）。
**修复建议**：失败时 throw 或返回 `null`；`AddPageButton` 加 `.catch`/空值判断 + 错误 toast。

**修复记录（d8648b3f）**：store catch 回滚后 `throw e`；`AddPageButton` 接 `.catch` + `useToastError.onError` 提示「创建页面失败」，不再触发 `onCreate` 导航；失败测试改为断言 `rejects.toThrow`。settingsStore 21 测试全过。

### P004（P1 前端缺陷）LLM 本地 Embedding 设置无错误处理

`useLlmConfigPage.ts:190-216`（`handleToggleLocalEmbedding`）、:218-228（`handleSelectLocalModel`）的 `invoke('llm_set_local_embedding', ...)` 无 try/catch，且 `handleSelectLocalModel` 先改前端状态再 invoke，失败留下「前端已选模型 A、后端未生效」的漂移。同文件 `handleRebuildEmbeddings`（:233-243）等均有 try/catch + onError + 回滚，此处明显遗漏。
**修复建议**：两处包 try/catch + onError；改为先 invoke 成功再 `setLocalModelId`，或失败回滚。

**修复记录（76cffe3d）**：`handleToggleLocalEmbedding` / `handleSelectLocalModel` 的 invoke 均包 try/catch + `onError`；两处失败回滚 `setLocalModelId(prevModelId)`，开关失败不回改 `useLocalEmbedding`。tsc/eslint/prettier 通过。

### P005（P1 性能）回收站子对象 N+1 冗余解密（已完成）

`snapshot.rs:466-484`：`list_trash_items` 返回的 `TrashItemSummary` **已包含 `original_id`**（`solosoul-vault/src/lib.rs:216`），但代码仍对每个子对象调 `get_trash_item`，每次附带整条 data blob 的 AES 解密（`trash.rs:193-194`），仅为取 `original_id`。

**修复记录（54b02ac8）**：`fetch_trash_child_items` 删除 `get_trash_item` 循环调用，直接用 summary 的 `original_id`；`list_trash_items` 加 `item_type: Some("object")` 让对象过滤在 SQL 层完成（减少扫描量）；`commands/object/tests/trash.rs` 复制的同款逻辑同步对齐。相关 trash 测试 16 个全过，clippy/fmt 干净。

### P006（P1 性能）首页角标全量解密换计数（已完成）

`HomePage.tsx:235-255` 每次回到首页（`location.pathname === '/'`）触发 `loadCounts()` → `attachment_list_all`（`tree.rs:55` 第一步即 `vault.list_objects(...)` 全表解密），只为渲染「照片数」「附件数」两个角标。

**修复记录（df464e69）**：新增 `attachment_count_stats` 轻量命令——`vault.count_active_attachment_stats` 单 SQL（`SELECT properties ... WHERE is_deleted = 0`）解密 + P025 子串扫描统计活跃附件总数与照片数，免附件树分组/模板解析/文件存在性探测；照片判定与前端 `previewItemByMime` 对齐（mimeType `image/` 前缀或扩展名 ∈ {png,jpg,jpeg,gif,webp,svg}）。`HomePage.loadCounts` 改用新命令；P025 扫描抽出完整数组版（`extract_attachments_array_from_json_text`）供 id 提取与计数共用。含 vault 单测 1 条 + 前端测试 3 条同步更新；clippy/fmt/eslint/prettier/tsc 全绿。

### P007（P1 代码质量）`AttachmentMeta` 双定义（已完成）

`crud.rs:47-69` 与 `export_import.rs:64-84` 字段逐字段相同（含相同 serde 属性），靠注释维持隐式契约，任一侧加字段忘同步即产生导出/导入格式漂移。

**修复记录（8824c261）**：删除 `crud.rs` 本地 `AttachmentMeta` struct，改为 `pub use solosoul_core::export_import::AttachmentMeta;`（序列化契约单一维护）；同时移除 crud.rs 变 unused 的 serde `Serialize/Deserialize` import。附件相关测试 27 个全过，clippy/fmt 干净。

### P008（P1 规范）`cargo fmt --check` 失败

`crates/solosoul-core/src/vault_service/account.rs:91,118` 两处 `tracing::info!/warn!` 宏格式不符。**修复建议**：`cargo fmt` 即可。

**修复记录（9054d0b1）**：`cargo fmt` 自动格式化两处 tracing 宏（91 行 warn 压缩为单行、118 行 info 展开为多行），`cargo fmt --check` 恢复通过，仅改动 account.rs 1 文件。

### P009（P1 规范）`cargo clippy -- -D warnings` 失败

- `crates/solosoul-core/src/ocr/macos_vision.rs:334-335`：`needless_borrows_for_generic_args`（`.arg(&x.to_string_lossy().as_ref())` 应去掉 `&`）。
- 另 `cargo clippy --all-targets` 下 `vault_service/tests.rs:26` 有 unused variable `account_id`（建议改 `_account_id`）。

**修复建议**：按 clippy 提示修改，两行级修复。

**修复记录（346d7563）**：macos_vision.rs 两处 `.arg(&x.to_string_lossy().as_ref())` 去掉多余 `&`；tests.rs `account_id` → `_account_id`。`cargo clippy --all-targets -- -D warnings` 恢复通过，solosoul-core 186 测试全过。

### P010（P2 安全）分享副本残留临时目录（已完成）

`share.rs:33-41` 桌面端分享前将附件明文复制到 `temp_dir()/solosoul_share/`，注释自认「跨会话残留但不自动清理」，全仓库无清理逻辑。

**修复记录（b95b4ace）**：分享前清理旧副本——桌面端 `copy_to_share_dir` 复制前 `cleanup_share_dir` 清掉 `solosoul_share/` 内旧文件（上次分享必然已完成，无保留价值；目录本身保留供 `copy_into_dir` 复用，仅删平铺文件不递归）；Android 分支同样在解密复制前清理 `solosoul_share_{object_id}/` 旧副本。新增 cleanup 单测 1 条（旧明文删除 + 子目录保留）。附件测试 22 个全过，clippy/fmt 干净。

### P011（P2 安全）遗留 `unlock` IPC 未 Zeroizing（已完成）

`vault.rs:7-18` 的 `unlock` 命令 `password: String` 直接传递、用后不清零，仍注册于 `lib.rs:55`。前端已改用 `auth.rs` 的 `login`（Zeroizing 包装），grep 确认前端仅 `ipc.test.ts` mock 中引用 `unlock`。

**修复记录（686d807c）**：删除命令定义、`lib.rs` 注册与 ACL 列表、`permissions/default.toml` 白名单条目、前端 P027 豁免名单 `'unlock'` 条目及 `ipc.test.ts` 对应 mock 测试；解锁统一走 `auth::unlock_with_password`（Zeroizing 包装）。编译/clippy/fmt/eslint/prettier/tsc 全绿，ipc 测试 11 个全过。

### P012（P2 安全）`verify_password` 无限速（已完成）

主密码解锁路径有阶梯锁定（`record_password_failure`），但 `verify_password`（`auth.rs:159-176`）不计失败、不触发锁定，可被无限次调用验证主密码。Argon2id 高参数使在线爆破成本高，风险有限，但与解锁路径限流策略不一致。

**修复记录（937446b7）**：新增 `VaultService::verify_password_with_lockout`——与 `unlock` 完全同款语义（锁定预检先于昂贵 KDF、失败经 `record_password_failure` 递增计数触发阶梯锁定、成功经 `clear_password_failures` 归零）；`verify_password` IPC 改走该方法并 `spawn_blocking`（验证含 Argon2id KDF 防阻塞 tokio）。错误密码仍返回 `false` 不抛异常（前端 P123「异常≠密码错误」语义不变），锁定期间返回与 unlock 一致的 `MASTER_PASSWORD_LOCKED_ERR`（前端 `backendError.ts` 已映射 `common:password_locked` 文案）。新增 core 限流单测 1 条；clippy/fmt 全绿。

### P013（P2 性能）导出快照 N+M 查询

`export.rs:751-761`：每对象 1 次 `list_snapshots` + 每快照 1 次 `get_snapshot`。1000 对象 × 5 快照 ≈ 6000 次独立查询。
**修复建议**：`solosoul-vault` 加 `list_snapshots_batch(object_ids)`（`WHERE object_id IN (...)`），消掉内层 get。

### P014（P2 性能）回收站批量恢复串行 IPC

`useTrashPage.tsx:145-168`：逐项 `await restoreItem(id)`，N 项 N 次串行往返；同文件 `permanentDelete(ids)` 已是批量入参。
**修复建议**：后端加 `trash_restore_batch(ids)`，前端一次调用。

### P015（P2 代码质量）掩码哨兵字面量三处

`'••••••••'` 硬编码于 `useLlmConfigPage.ts:369,413` 与 `ProviderManagerPanel.tsx:280`，而 `lib/masking.ts:14` 已导出 `MASK_PLACEHOLDER`。常量一旦调整，三处静默断链（占位符被当真实 key 发往后端）。
**修复建议**：三处统一 `import { MASK_PLACEHOLDER } from '@/lib/masking'`。

### P016（P2 代码质量）批量下载 catch-all 吞错

`useAttachmentManagerBatchOps.ts:105-107`：`catch { // dialog cancelled }` 吞掉 try 块内任意异常（含 dialog 插件错误），无任何日志。
**修复建议**：`catch (e) { logger.warn(...) }` 留痕。

### P017（P2 死代码）`ExportError::Crypto` 从未构造

全库（含 src-tauri、solosoul_cli）搜索仅命中定义行。**修复建议**：删除该变体。

### P018（P2 死代码）一次性 codemod 脚本残留

`tokenize-fonts.mjs`（文件头自述一次性）、`tokenize-icons.mjs`、`fix_invoke_keys.cjs`、`revert_invoke_keys.cjs` 均无引用。
**修复建议**：确认迁移已落地后删除，或移入 `scripts/archive/` 并注明。⚠️ 涉及删除文件，按流程约束暂缓，需用户确认后执行。

**修复记录（094e75b8）**：用户确认删除，4 个脚本直接移除（净 -780 行）。

### P019–P021（P2 重复代码）

- **P019**：`provider.rs:25-55` 与 `unified_chat.rs:30-59` 的 provider 合并循环，仅差掩码步骤。建议提取 `merge_saved_providers`，掩码作为调用方后置步骤。
- **P020**：`metadata.rs:535-560` 与 `sync_changes.rs:475-500` 同一 SQL + 同一解密映射。建议提取 `map_user_template_row`。
- **P021**：`docx.rs:110-129` 与 `text.rs:29-48` 元信息段构建逐字相同。建议提取 `build_meta_lines`。

### P022（P2 可维护性）超长函数/组件 Top 10

| 行数 | 位置 | 函数/组件 |
|---|---|---|
| 391 | `src/hooks/useLlmChatCore.ts:63` | useLlmChatCore |
| 388 | `src/components/attachment/AttachmentPreviewOverlay.tsx:34` | AttachmentPreviewOverlay |
| 388 | `src/hooks/useRecoveryReceive.ts:28` | useRecoveryReceive |
| 386 | `src/pages/ai/PluginDashboardPage.tsx:35` | PluginDashboardPage |
| 376 | `src/components/layout/AddPageButton.tsx:24` | AddPageButton |
| 374 | `src/pages/ai/useLlmConfigPage.ts:48` | useLlmConfigPage |
| 369 | `src/components/settings/PinSection.tsx:20` | PinSection |
| 369 | `src/pages/settings/VaultDirectorySection.tsx:27` | VaultDirectorySection |
| 362 | `src/components/sync/RecoveryQrContent.tsx:19` | RecoveryQrContent |
| 357 | `src/components/attachment/PhotoAlbumOverlay.tsx:33` | PhotoAlbumOverlay |

项目已有拆分先例（W005、P046），建议按同模式拆子组件/子 hook。无功能 bug 证据。

### P023（P2 可维护性）深层嵌套热点

- `useDragToAttach.ts:190-234`：drop 分支约 6 层嵌套，函数整体 276 行，建议抽独立函数。
- `useAttachmentManagerBatchOps.ts:73-81`：try 内三重 for + if，建议 `flatMap`。
- 5 层边界：`useExportScope.ts:251-262`、`useTouchZoom.ts:184`、`propertyFlatten.ts:86`、`useExportImportPage.tsx:247`、`settingsStore.ts:446`。
- JSX：`DeviceListKnownCard.tsx:93-106` 三元 + Fragment 嵌套 brace 深度 11，建议抽子组件。

---

## 需人工确认的疑似问题（不计入清单）

1. **生物识别将主密码写入 OS 凭据存储**（`biometric.rs:357`，Windows DPAPI / macOS Keychain / Android Keystore）：通行设计，但与「主密码从不存储」的宣称语义有出入，建议在文档中限定承诺范围。
2. **DPAPI key 文件 ACL**（`biometric/windows.rs:126-143`）：`write_dpapi_key_file` 直接 `fs::write`，未见 `icacls`；若父目录已由 `set_private_dir` 收紧则无碍，需确认 Windows ACL 继承状态。
3. **CSP `style-src 'unsafe-inline'`**（`tauri.conf.json:30`）：React 内联样式所需；`script-src` 未放宽，无实际 XSS 放大效应，仅提示。
4. **`shell:allow-open` 允许任意 http/https/mailto/tel**（`capabilities/default.json:13-19`）：当前无 XSS 入口，风险低。
5. **settingsStore 未纳入入站同步刷新**（`syncStore.ts:52-71` `refreshDataStores` 不含 settingsStore）：对端改主题/语言后本端要等下次解锁才生效，需确认是否为有意设计。
6. **同步历史 localStorage key 不按账户隔离**（`syncStore.ts:21`）：多账户同机时互相可见（仅表名/计数/HLC，无明文），需确认多账户并存场景。
7. **`ImportStrategy::Merge` 静默降级为覆盖**（`export_import.rs:106-107`）：CLI 仍接受 `"merge"` 输入但按 Overwrite 处理；GUI 不暴露该选项。需确认是最终决定还是未完成 feature。
8. **约 30 个 Rust `pub fn` 仅文件内自用**（如 `local_embed.rs:209`、`auto_sync.rs:228`、`rag.rs:460-520`）：非死代码但可见性过宽，可逐批收紧为 `pub(crate)`。
9. **`strip_bookkeeping` 冲突消解深拷贝整个对象 JSON**（`solosoul-sync/src/delta.rs:34-45`）：仅冲突路径调用，需 profiling 确认是否热点。
10. **`GlobalAttachmentManager` 编辑后整树刷新**：低频可接受，批量编辑描述时反复全量解密，可改为只刷新受影响节点。

## 已审计确认无问题的类别

- 硬编码密钥/token：未发现（LLM key 为用户配置，加密存储，回传前掩码）。
- 命令注入：`Command` 仅用于 `xcrun`/`swiftc`/`icacls`，参数均为常量或经白名单校验，无 shell 拼接。
- 路径遍历：`sanitize_file_name` 被导入/插件/同步/分享统一复用；fs 命令走 `resolve_allowed_path` 白名单 + canonicalize；ZIP 条目手工校验。
- 反序列化：全库无 `#[serde(untagged)]`。
- 前端 XSS：无 `dangerouslySetInnerHTML`/`innerHTML`/`eval`；Markdown 经 `SafeMarkdown` 且不引入 `rehype-raw`。
- 日志敏感信息：tracing 日志为状态/错误码级；`NoiseKeys` Debug 仅暴露公钥指纹。
- 加密强度：Argon2id（release 64 MiB/3 iter）+ AES-256-GCM 随机 nonce + HKDF 域分离 + 常数时间比较 + 全程 `Zeroizing`，无自造加密。
- TODO/FIXME/HACK/XXX 注释：零命中；无大段注释掉的代码；无未被 import 的 TS 模块；无定义未注册的 Tauri command。
- 裸调 plugin-dialog：全部走 `src/lib/dialog.ts` 封装；掩码逻辑统一走 `useRevealState`/`lib/masking.ts`。
- 大文件加密已分块（1 MiB/chunk）；全表解密路径均在 `spawn_blocking`；前端列表均有分页 + memo。

## 方法学说明（可信度边界）

- 静态分析基于本次实跑的 `tsc` / `eslint` / `vitest` / `cargo test` / `cargo fmt --check` / `cargo clippy`（含 `--all-targets`）输出。
- 启发式分析由 4 路并行审查完成（安全 / 性能 / 死代码 / 前端架构），P001–P005、P011 等关键条目已由主审查流程逐一打开源码核实；其余条目证据来自子审查报告，修复前建议再核对行号（代码演进可能导致行号偏移）。
- TS 重复代码用 10 行完全一致窗口检测，70–99% 相似的非逐字重复只抽查了代表样本。
- 本报告按用户要求**只生成、不修复**；修复阶段请从 P008/P009（恢复 CI 基线）开始，再按 P002 → P003 → P004（数据丢失风险）→ P001/P005/P006 的顺序推进。
