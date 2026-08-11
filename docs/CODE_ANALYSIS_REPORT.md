# 代码分析修复报告

> 最后更新：2026-08-11（P023 修复完成）
> 当前分支：`main`
> 修复轮次：2（按用户指令逐项修复，一项一提交）

## 基线检查（阶段 0）

`npm run check-all` **未通过**：TypeScript 与 fmt 已通过，**Rust Clippy 报 2 个 error 后中止**，后续 lint / 单元测试未执行（见 P001）。

Git 状态：工作树除本报告文件重建外干净（旧报告已删除，本轮按要求全新生成）。

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|------|--------|------|----------|------|------|
| P001 | P0 | 规范/构建 | `tauri/src-tauri/src/commands/attachment.rs:1158,1192` 等 | Clippy error 致 check-all 中止（详见下方修复记录） | `[x]` 已完成 |
| P002 | P1 | 漏洞 | `tauri/src-tauri/src/commands/update.rs:449-463` | `android_download_apk` 信任前端回传的 URL 与 checksum（可传空跳过校验），验签成果未绑定到 IPC 通道 | `[x]` 已完成 |
| P003 | P1 | 性能 | `tauri/src-tauri/src/commands/export_import/mod.rs:290`、`export.rs:183` | 导出 selected 分支全库解密两遍（`list_objects` + `load_objects_batch`），且随导出页每次勾选变更（500ms 防抖）触发 | `[x]` 已完成 |
| P004 | P1 | 性能/架构 | `tauri/src-tauri/src/commands/llm/conversation.rs:13-57` | LLM 会话整体存加密 preferences blob，每次保存 = 全量解密+深克隆+序列化+加密+写盘；每条聊天消息都触发 | `[x]` 已完成 |
| P005 | P1 | 性能 | `solosoul_cli/src/commands/search.rs:181-184,206-209` | CLI `/search` 用 `list_objects(...).len()` 统计子对象数，对每个命中页面全量解密仅为取计数（GUI 已有 `count_objects` 先例） | `[x]` 已完成 |
| P006 | P1 | 架构 | `tauri/src-tauri/src/commands/object/mod.rs:439`、`tauri/src/stores/objectStore.ts:56,185`、`ObjectDetailModal.tsx:470` | 类型漂移：Rust `ObjectData` 无 `tags`，TS 声明 `tags?` 永为 undefined；`updateObject` 用 undefined 覆盖摘要 tags，详情页标签成死渲染路径 | `[x]` 已完成 |
| P007 | P1 | 架构 | `tauri/src-tauri/src/commands/export_import/export_docx.rs:200` | `flatten_object_fields` 6–7 层控制流嵌套，动态字段组展平逻辑难读难测 | `[x]` 已完成 |
| P008 | P1 | 架构 | `tauri/src-tauri/src/commands/search/query.rs:16` | `search_properties_for_matches` 133 行递归函数承担 4 种职责，`__fields` 分支 5–6 层嵌套 | `[x]` 已完成 |
| P009 | P1 | 架构 | `tauri/src-tauri/src/commands/attachment.rs:1130` | `attachment_share` ~161 行，macOS/Windows 两个 `#[cfg]` 块各重复约 40 行「复制→主线程调度→oneshot」骨架 | `[x]` 已完成 |
| P010 | P1 | 规范 | `tauri/src/components/layout/AddPageButton.tsx:162-198` ↔ `NavButton.tsx:37-73` | 悬停卡片 portal 定位逻辑 ~37 行逐字符复制（注释自认 same pattern），应抽共享 hook | `[x]` 已完成 |
| P011 | P1 | 规范 | `tauri/src/components/layout/AddPageButton.tsx:438-520` ↔ `CustomPageEditPopover.tsx:322-409` | 图标分类选择器 ~44 行 + 分类数组两处复制，应抽 `IconCategoryPicker` 共享组件 | `[x]` 已完成 |
| P012 | P2 | 漏洞 | `tauri/src-tauri/src/commands/update.rs:238-246` | Release 资产匹配过宽（`contains("sha256")` 会命中 `.minisig`），完整性校验可能静默失效（需人工确认 assets 顺序） | `[x]` 已完成 |
| P013 | P2 | 漏洞 | `tauri/src-tauri/src/commands/export_import/import.rs:9,59,183` | 导入命令 `file_path` 无白名单（与 fs 命令 P107 收窄策略不一致），构成有限任意文件探测原语 | `[x]` 已完成 |
| P014 | P2 | 漏洞 | `tauri/src-tauri/src/commands/attachment.rs:445-482` | `attachment_copy_to_vault` 兜底分支字面 `starts_with` 且未拒绝 `..` 组件，Android symlink 场景可绕过 allowed-dir（需人工确认可达性） | `[x]` 已完成 |
| P015 | P2 | 漏洞 | `tauri/src-tauri/src/commands/export_import/export.rs:337-340` | 导出路径白名单为空时 fail-open 放行任意路径（`attachment_download:911` 同款），应 fail-closed 或至少 warn | `[x]` 已完成 |
| P016 | P2 | 规范 | `tauri/src-tauri/src/commands/pin.rs:42-59,103-113`、`vault.rs:34-45` | `pin_setup`/`pin_disable`/`pin_unlock`/`change_password` 的密码参数未在 IPC 边界 Zeroizing 包装（P031 模式不一致，需确认 PinManager 内部清零） | `[x]` 已完成 |
| P017 | P2 | 漏洞 | `tauri/src-tauri/src/commands/fs.rs:133-142` | `resolve_within` 校验后返回未规范化路径，存在符号链接 TOCTOU 窗口（威胁模型较低） | `[x]` 已完成 |
| P018 | P2 | 规范 | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:281-298` | OCR debug 日志记录用户扫描图片完整路径（GUI 侧 tracing 落盘位置需确认） | `[x]` 已完成 |
| P019 | P2 | 架构 | `tauri/src-tauri/src/commands/log.rs:65-76` | `log_write` 允许前端写任意 `action_type` 审计条目，无枚举白名单，审计日志作为安全证据的可信度受限（需确认设计意图） | `[x]` 已完成 |
| P020 | P2 | 架构 | `tauri/src-tauri/src/commands/object/mod.rs:604` 等 | object 系命令信任客户端 `account_id`（template/ocr 等已统一 `current_account` 服务端派生，两种约定并存，需确认触发路径） | `[x]` 已完成 |
| P021 | P2 | 架构 | `tauri/src/types/llmProvider.ts:5` vs `tauri/crates/solosoul-core/src/llm/config.rs:33` | 潜伏类型漂移：TS `ProviderConfig` 缺 `embeddingModel`，重构保存逻辑时会静默重置该字段 | `[x]` 已完成 |
| P022 | P2 | 死代码 | `tauri/crates/solosoul-core/src/vault_service.rs:493-518`、`tauri/src-tauri/src/commands/auth.rs:12-19` | 登录/账户列表把 `salt`、`verifyHash` 序列化给前端但前端零消费，扩大 WebView 攻击面且误导开发 | `[x]` 已完成 |
| P023 | P2 | 规范 | `tauri/src-tauri/src/commands/auth.rs:12` vs `solosoul-core/src/vault_service.rs:195` | `AccountInfo`/`AccountSummary` 重复 DTO 字段重叠但可选性不同，易演进漂移 | `[x]` 已完成 |
| P024 | P2 | 架构 | `tauri/src-tauri/src/commands/export_import/mod.rs:238` vs `solosoul-core/src/export_import.rs` | `derive_export_key_cfg` 密码学逻辑双份实现，仅靠注释约束一致性（安全敏感路径） | `[ ]` 待修复 |
| P025 | P2 | 架构 | `tauri/Cargo.toml:73` | release 全局 `panic = "abort"`，命令 handler 内 panic = 整进程崩溃且 Vault 未走正常锁定清理 | `[ ]` 待修复 |
| P026 | P2 | 规范 | `template.rs:171`、`object/mod.rs:604`、`settings.rs:346` | 命令参数普遍缺长度/格式校验（名称无上限、properties/preferences 载荷无大小限制、偏好 key 无白名单） | `[ ]` 待修复 |
| P027 | P2 | 死代码 | `tauri/src-tauri/src/commands/object/trash.rs:147`、`lib.rs:389,870,996` | `trash_permanent_delete` 已注册但前端从不调用（P024 批量改造后走 batch），删除需同步守卫测试与总数断言 | `[ ]` 待修复 |
| P028 | P2 | 死代码 | `tauri/crates/solosoul-vault/src/profile.rs:56` | `VersionedProfileData` 及 3 个方法全 workspace 零调用（约 25 行） | `[ ]` 待修复 |
| P029 | P2 | 死代码 | `tauri/crates/solosoul-plugin/src/manifest.rs:119` | `PluginTier::label()` 零调用 | `[ ]` 待修复 |
| P030 | P2 | 死代码 | `tauri/src-tauri/src/fs/vault_file_system.rs:1-6`、`fs.rs:4` | 死 re-export shim，全 workspace 无 import，制造双 import 路径混淆 | `[ ]` 待修复 |
| P031 | P2 | 死代码 | `tauri/src/stores/pluginStore.ts:95` | `DEFAULT_ENABLED_TIERS` 导出但仅本文件使用 | `[ ]` 待修复 |
| P032 | P2 | 规范 | `tauri/src-tauri/src/lib.rs:643-644` | 过时注释：描述的 legacy XOR 迁移遥测代码已不存在（迁移窗口已关闭） | `[ ]` 待修复 |
| P033 | P2 | 死代码 | `tauri/src-tauri/src/lib.rs:304-316`（调用于 :667） | `setup_detect_locale()` 结果仅用于一行 debug 日志，前端实际走 `get_system_locale` IPC（需人工确认是否保留诊断） | `[ ]` 待修复 |
| P034 | P2 | 架构 | `tauri/src-tauri/src/commands/llm/stream.rs:92` | `handle_sse_stream` 123 行，usage 提取分支 5 层嵌套，SSE 解析与 token 统计耦合 | `[ ]` 待修复 |
| P035 | P2 | 架构 | `tauri/src-tauri/src/commands/export_import/import.rs:624` | `rebuild_imported_templates` 模板 ID 去重三分支内联约 5 层嵌套 | `[ ]` 待修复 |
| P036 | P2 | 架构 | `tauri/src-tauri/src/commands/object/snapshot.rs:179` | `build_preview_properties` ~141 行，4 个编号阶段混在一个 else 分支 | `[ ]` 待修复 |
| P037 | P2 | 架构 | `tauri/src-tauri/src/commands/export_import/export.rs:6` | `export_get_scope_tree` ~133 行承担 5 个阶段（敏感度合并/孤儿过滤等应抽函数） | `[ ]` 待修复 |
| P038 | P2 | 架构 | `tauri/src-tauri/src/commands/object/mod.rs:1021` | `collect_updated_fields` ~122 行，类型变更与 6 项元数据对比混在一个循环体 | `[ ]` 待修复 |
| P039 | P2 | 规范 | `tauri/src-tauri/src/commands/export_import/export.rs:85-103,136-146` | 6 个系统分区字符串在数组/映射/集合中重复书写 3 遍，新增分区需同步改 3 处 | `[ ]` 待修复 |
| P040 | P2 | 规范 | `PasswordVerificationDialog.tsx:444-505` ↔ `pages/auth/LoginPinView.tsx:27-82` | PIN 输入卡片 ~26 行近乎复制（AGENTS.md 禁止多处复制对话框的同类问题） | `[ ]` 待修复 |
| P041 | P2 | 规范 | `pages/scan/OcrScanSettingsPanel.tsx:79-140` ↔ `pages/settings/OcrSettingsPage.tsx:95-160` | OCR tier 状态行 ~30 行两处复制 | `[ ]` 待修复 |
| P042 | P2 | 架构 | `sync/SyncScanQrDialog.tsx:90-137` ↔ `SyncShowQrDialog.tsx:181-234` | 手写模态外壳 ~29 行复制，项目已有共享 `Dialog` 组件 | `[ ]` 待修复 |
| P043 | P2 | 规范 | `pages/system/MandatoryUpdateOverlay.tsx:157-188` ↔ `UpdateInfoCard.tsx:225-257` | 下载进度条 ~24 行复制 | `[ ]` 待修复 |
| P044 | P2 | 规范 | `tauri/src-tauri/src/commands/discovery.rs:180-192,344-356` | desktop/mobile 两个 `mdns_discover` 中 client_type 解析逻辑近似复制 ~15 行（cfg 分支语义需人工确认） | `[ ]` 待修复 |
| P045 | P2 | 规范 | `HistoryViewer.tsx:391`、`TrashSnapshotView.tsx:408`、`WorkspaceObjectCard.tsx:389` | 动态字段组快照行渲染疑似三份拷贝（各 ~16–22 行，需人工核对） | `[ ]` 待修复 |
| P046 | P2 | 架构 | 前端 10 个巨型组件（汇总） | 单组件非注释行 > 300：`AttachmentViewer`(~550)、`LoginPage`(~501)、`PasswordVerificationDialog`(~447)、`TemplateFieldRow`(~436)、`DeviceListPanel`(~424)、`TrashPage`(~422)、`ObjectDetailModal`(~422)、`ExportImportPage`(~417)、`ImportSection`(~409)、`ExportSection`(~405)，建议按「数据 hook + 展示子组件」拆分 | `[ ]` 待修复 |
| P047 | P2 | 架构 | Rust 巨型文件（汇总） | `attachment.rs` 2057 行、`object/tests.rs` 2353 行、`export_docx.rs` 1989 行，文件级拆分作为后续架构项 | `[ ]` 待修复 |

## 修复进度

- 已完成：23 / 47
- 当前处理：P024（按建议顺序推进）

## 详细问题描述与修复指引

### P001（P0）Clippy 基线失败 — 已完成（commit 51135565）

**原问题**：

```
error: unused import: `NSView`
  --> src-tauri/src/commands/attachment.rs:1158:53
error: deref which would be done by auto-deref
  --> src-tauri/src/commands/attachment.rs:1192:64
```

**修复记录**（共 5 处，覆盖 Windows 与 macOS 双平台 CI）：

1. `attachment.rs:1158`：删除未使用的 `NSView` import（macOS 块）。
2. `attachment.rs:1192`：`&*dest.to_string_lossy()` → `&dest.to_string_lossy()`（explicit_auto_deref）。
3. `window.rs:12`：`calculate_luminance` 仅 macOS 分支调用，Windows/Linux 非 test 构建为死代码，加 `#[cfg_attr(not(target_os = "macos"), allow(dead_code))]`。
4. `solosoul-core/vault_service.rs:33`：`use std::fs;` 仅 unix 函数与 test 使用，按 `#[cfg(any(unix, test))]` 门控。
5. `solosoul-core/biometric/windows.rs:308`：`if let Err(e) = ensure_mta() { return Err(e); }` → `ensure_mta()?;`（clippy::question_mark）。

**验证**：`cargo clippy --workspace -- -D warnings` 全绿；`cargo fmt --check` ✅；`tsc --noEmit` ✅；`eslint src` ✅；`check_acl_consistency.py` OK（194 命令）；`vitest` 619 用例全过。

### P002（P1）APK 下载信任前端回传的 URL 与校验和 — 已完成（commit fe582841）

**修复**（采用报告的方案二：下载命令内按 version 重新拉取元数据，不接受前端传入）：

1. `android_download_apk` 命令签名改为仅接收 `version`（删除 `download_url`/`expected_checksum` 参数）。
2. 命令内部按 `releases/tags/v{version}` 重新拉取 GitHub Release 元数据，提取 APK 资产 URL；校验和复用 `resolve_verified_checksum`（下载 `.sha256` + `.sha256.minisig` 并以编译期公钥验签）。
3. **fail-closed**：release 缺失、APK 资产缺失或校验和验签失败 → 直接拒绝下载（不再降级为无校验下载）。
4. 提取共享 helper：`find_apk_asset`（返回克隆值，避免借用阻塞字段移动）、`resolve_verified_checksum`、`fetch_github_release_by_tag`/`fetch_github_release_url`，`android_check_update` 与下载命令复用同一套资产解析逻辑。
5. 前端 `androidDownloadApk`/`ensureApkDownloaded` 移除 URL/checksum 参数，两个 hook（useAppUpdate/useUpdateChecker）调用点同步；新增测试断言 invoke 仅携带 `version`。

**验证**：cargo check --tests ✅；updater.test.ts 4/4 ✅；vitest 619 全过 ✅；ACL 194 命令一致 ✅。

### P003（P1）导出 selected 分支全库双重解密 — 已完成（commit 6f947313）

**修复**：

1. **筛 id 阶段不再解密**：`collect_scope_objects` selected 分支改用新增的 `list_object_metadata_with_tags`（`list_object_metadata_impl(with_tags=true)`）——纯 SQL SELECT 元数据列 + 明文 `tags_json`，不触碰加密的 `properties`/`property_labels`。命中对象才经 `load_objects_batch` 解密一次。
2. **估算不再序列化**：`export_estimate_size` 对象 payload 体积改用新增的 `objects_size_batch`（纯 SQL `SUM(LENGTH(properties))`，参照 `snapshots_size_batch` 先例）估算，不再对已解密 properties 做 `serde_json::to_vec` 往返。

**验证**：cargo check --tests ✅；clippy --workspace 全绿 ✅。

### P004（P1）LLM 会话整 blob 重写 — 已完成（commit f2f8c58b）

**原问题**：全部会话（每条最多 500 消息）存于加密 profile preferences blob；任何 save/list/rename/delete 都整 blob 解密→修改→加密→写盘，且 `save_conversations` 先 `to_vec()` 深克隆全部会话（:45）。每条聊天消息流式结束都触发 `llm_save_conversation`，开销随历史线性增长。

**修复**（采用报告中期方案：会话改存独立 SQLite 表，行级存储）：

1. **migration v26**：新增 `llm_conversations` 表（id PK、account_id、data 加密 BLOB、updated_at），建表 + `idx_llm_conversations_account` 索引；幂等采用 v24/v25 同款「表存在性守卫 + INSERT OR IGNORE」模式（兼容 v23 测试降级重跑场景）。
2. **storage 层**（`solosoul-vault/storage/conversations.rs`）：行级 CRUD——`save_conversation`（upsert 单行 + HLC）、`load_conversation`（仅解密目标行）、`list_conversations`、`delete_conversation`（记墓碑）、`conversations_size`（纯 SQL SUM(LENGTH(data)) 统计，替代 vault stats 的 blob 读取）。
3. **sync 集成**：`list_conversation_changes_since`/`apply_conversation_sync_record_tx` 接入 sync_changes/sync_apply 分发，`SYNC_TABLES` 常量新增 `llm_conversations`；同步记录随行携带 `accountId`（与 ObjectRecord 自带 account_id 同理），本地按该账户 upsert，与行过滤一致。
4. **命令层**（`commands/llm/conversation.rs` 重写）：从 blob 存储切换行级 API，旧 blob 数据懒迁移到新表后清除；流式热路径（`stream.rs`）改为单行读改存；`llm/tests.rs` 测试同步更新。
5. **一致性**：`solosoul-core/llm/service.rs` 的会话方法（无跨 crate 调用方）改为委托 vault 行级 API，消除双写路径；`vault.rs` 存储统计改用 `conversations_size`。

**验证**：vault 159 测试全过（含新增行级 CRUD + sync apply + v26 建表测试）✅；clippy --workspace 全绿 ✅；src-tauri `cargo check --tests` ✅；CLI 编译 ✅。注：src-tauri 测试二进制运行报 `STATUS_ENTRYPOINT_NOT_FOUND`（Windows 本机 DLL 环境问题，旧二进制同样无法运行，非本次改动引入）。

### P005（P1）CLI 搜索计数全量解密 — 已完成（commit 29a9d8ec）

**原问题**：`search_pages` 用 `list_objects(...).map(|v| v.len())` 统计子对象数，`list_objects` 对每行 AES-GCM 解密 + JSON 解析仅为取计数，命中多个分区时相当于多次全库解密。

**修复**：`search.rs` 两处计数（自定义页子对象数 :182、系统分区对象数 :207）改用 `VaultStore::count_objects`（纯 SQL `COUNT(*)`，不解密），语义等价（`count_objects` 与 `list_objects(kw=None, include_deleted=false)` 均按 `account_id + is_deleted=0` + type/parent 过滤）。

**验证**：CLI 编译 ✅；`commands::search` 测试 10/10 ✅。（注：CLI 全量测试中 backup/ocr/profile 等 14 项失败为 Windows 本机预先存在问题，经 stash 对比确认与本次改动无关。）

### P006（P1）ObjectData tags 类型漂移 — 已完成（commit 91290b0b）

**原问题**：Rust `record_to_data` 不返回 `tags`，TS `ObjectData.tags?` 永为 undefined；`updateObject`（objectStore.ts:185）将 `tags: obj.tags`（undefined）显式覆盖进摘要列表项，抹掉 `object_list` 摘要已有的 tags；详情弹窗 `obj.tags`（ObjectDetailModal.tsx:470）是死渲染路径。

**修复**：Rust `ObjectData` 补齐 `tags: Vec<String>`（`#[serde(default, skip_serializing_if = "Vec::is_empty")]`），`record_to_data` 透传 `record.tags_json`。TS `ObjectData` 本就声明 `tags?: string[]`，无需改动——`object_get`/`object_update` 现在返回真实 tags，`updateObject` 摘要合并恢复正确（tags 与摘要同源同一行 `tags_json`，无覆盖错位），详情弹窗 `ObjectDetailTags` 渲染成为活路径。`test_record_to_data_conversion` 增补 tags 断言，serde roundtrip 测试改为非空 tags 覆盖序列化路径。

**验证**：`cargo check -p solo_soul --tests` ✅；tsc ✅；eslint ✅；objectStore + ObjectDetailModal 前端测试 15/15 ✅；代码审查确认无回归（tags 与摘要同源一致性、skip_serializing_if 空值场景无可见影响）。

### P007（P1）flatten_object_fields 拆分 — 已完成（commit b234236c）

**原问题**：`flatten_object_fields`（export_docx.rs:200）60 行内混 3 种职责——`__fields` 元信息提取、dynamic_group 展平、普通字段输出，6–7 层控制流嵌套，动态字段组展平逻辑难读难测。

**修复**：抽三个独立函数，主循环只做分发——`build_field_meta`（提取 `__fields` 的 name/type 元信息，回退键名）、`flatten_dynamic_group`（dynamic_group 子字段展开，name/value 均非空才收集）、`flatten_object_fields`（跳过 `__` 键 → dynamic_group 分发 → 普通字段输出）。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅。现有测试 `test_flatten_basic_fields`（普通字段 label/原始值）与 `test_flatten_dynamic_group_expands_children`（子字段展开 + 无占位符条目）覆盖两条路径，行为等价（纯提取重构）。（注：src-tauri 测试二进制在本机 Windows 无法运行，为预先存在的 DLL 环境问题。）

### P008（P1）search_properties_for_matches 拆分 — 已完成（commit 875ab0b8）

**原问题**：`search_properties_for_matches`（query.rs:16）133+ 行递归函数承担 4 种职责（元数据键跳过、`__fields` 定义名匹配、字段名匹配、值匹配打分/截断），`__fields` 分支 5–6 层嵌套。

**修复**：抽两个辅助函数，主循环只做分发——`match_field_defs`（`__fields` 定义中的 `name` 匹配，精确/包含打分）、`push_value_match`（字符串值匹配：内部占位 token 排除、searchable 校验、精确/包含打分、超长截断）。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅（修复一处 doc 注释接续列表项导致的 doc_list_item lint）。主函数 157→102 行，`search/tests.rs` 22 个测试直接覆盖（含 `__fields`/dynamic_group/数组/嵌套路径），纯提取行为等价。

### P009（P1）attachment_share 按平台拆分 — 已完成（commit 2f688fd9）

**原问题**：`attachment_share`（attachment.rs:1130）~161 行，macOS/Windows 两个 `#[cfg]` 块各重复约 40 行「复制→主线程调度→oneshot」骨架。

**修复**：调度器只保留公共前置（resolve 路径 + 平台分发）；按平台拆 `share_macos`/`share_windows`/`share_linux` 子函数，抽两个共享模板——`copy_to_share_dir_async`（spawn_blocking 复制到分享目录）与 `run_on_main_thread_oneshot`（`run_on_main_thread` 调度 + oneshot 回传，泛型闭包参数接收 AppHandle 克隆）。Android（插件 share_file）与 iOS（显式不支持）保持内联。各平台函数体内 `use` 与 SAFETY 注释原样保留。

**验证**：`cargo check -p solo_soul --tests` ✅（Windows 目标，macos/android 分支 cfg 排除但代码逐字保留）；clippy --workspace 全绿 ✅。重构为纯提取，平台行为不变（修复一处 helper 末尾漏 `Ok(())` 的编译错误）。

### P010（P1）useHoverCardPosition 共享 hook — 已完成（commit 9c3112e7）

**原问题**：`AddPageButton.tsx:162-198` 与 `NavButton.tsx:37-73` 悬停名称卡片定位逻辑双份复制（约 40 行/份：rect 定位计算、supportsHover 守卫、scroll/resize 跟随）。

**修复**：抽共享 hook `useHoverCardPosition(wrapperRef, {isHorizontal, isBottom, isRight})`（`src/hooks/useHoverCardPosition.ts`），返回 `cardStyle`/`isHovered`/`handleMouseEnter`/`handleMouseLeave`。两组件改用 hook，删除各自复制块；NavButton 保留独立的 `updateIndicator`（激活指示条，非共享逻辑）。

**验证**：tsc ✅；eslint ✅；全量 vitest 619/619 ✅（纯提取，无行为变化）。

### P011（P1）IconCategoryPicker 共享组件 — 已完成（commit cb5d43bf）

**原问题**：`AddPageButton.tsx:438-520` 与 `CustomPageEditPopover.tsx:322-409` 图标分类选择器 ~44 行 + 12 项分类数组两处复制。

**修复**：分类数组提为 `ICON_CATEGORY_ORDER` 常量导出（`lib/pageIcons.ts`）；抽 `IconCategoryPicker` 共享组件（`components/layout/IconCategoryPicker.tsx`）渲染「分类名 + 6 列图标网格」，`variant` 支持 `module`（CSS module 按钮，AddPageButton 弹出面板）与 `inline`（内联瓦片，编辑弹窗）两种样式；点击行为由调用方 `onSelect` 决定（保持打开 / 关闭选择器）。

**验证**：tsc ✅；eslint ✅；全量 vitest 619/619 ✅（纯提取，两处渲染与行为不变）。

### P012（P2）APK 校验和资产匹配与验签警告 — 已完成（commit e33d4753）

**原问题**：`resolve_verified_checksum` 资产匹配用 `contains("sha256")`，会误匹配 `.sha256.minisig` 签名文件；验签失败/资产缺失仅 `tracing::warn` 静默返回空串，前端无感知。

**修复**：① 校验和资产匹配收紧为 `ends_with(".sha256") && !ends_with(".minisig")`；② `resolve_verified_checksum` 返回 `(Option<String>, Option<String>)`（校验和 + 不可用原因），三类失败（资产缺失/签名缺失或验签失败/文件格式异常）分别给出中文原因；③ `AndroidUpdateInfo` 新增 `checksum_warning` 字段，check 命令透传给前端；④ 前端 `AndroidUpdateInfo`/`VersionInfo` 补 `checksumWarning`，UpdateInfoCard 在可用更新区以 AlertTriangle 警告条展示；⑤ download 命令仍 fail-closed（`.0.ok_or_else`），不因警告放宽。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅；tsc ✅；eslint ✅；全量 vitest 619/619 ✅。
### P013（P2）导入命令 file_path 白名单校验 — 已完成（commit cf43f114）

**原问题**：`import_parse_package`/`import_decrypt_preview`/`import_execute_advanced` 桌面端直接信任前端传入的 `file_path` 读取任意路径，构成有限任意文件探测原语。

**修复**：`fs.rs` 的 `resolve_allowed_path` 改 `pub(crate)`；import.rs 新增 `validate_import_path` helper（桌面端：`resolve_allowed_path` 白名单校验，越界即拒绝；Android/iOS：SAF/应用内路径跳过），三个导入命令入口各加 `AppHandle<R>` 参数并先校验（`import_execute_advanced` 校验 `req.source_path`）。校验只在命令边界，`import_execute_internal` 保持不动——恢复备份（recovery.rs）复用内部函数读应用内备份路径，不受影响。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅；ACL 194 命令一致 ✅；前端 invoke 参数不变（AppHandle 由 Tauri 注入）。
### P014（P2）attachment_copy_to_vault 拒绝 .. 组件 — 已完成（commit a388bc53）

**原问题**：`attachment_copy_to_vault`（attachment.rs:445-482）兜底分支（canonicalize 失败但文件存在，Android symlink 场景）用字面路径做 `starts_with` 前缀判定，未拒绝 `..` 组件——`..` 可让字面前缀通过检查却解析到白名单外。

**修复**：入口处（`src_raw` 建立后）与 `attachment_download` 对齐，拒绝 `Component::ParentDir`（`Source path must not contain '..'`）。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅。
### P015（P2）路径白名单为空 fail-closed — 已完成（commit 34dbce73）

**原问题**：`validate_export_dest`（export.rs:337-340）、`attachment_download`（attachment.rs:911 同款）白名单为空时 fail-open 放行任意路径。

**修复**：三处（`attachment_copy_to_vault`/`attachment_download`/`validate_export_dest`）在白名单为空时改为拒绝 + `tracing::warn`（fail-closed），错误文案说明 Desktop/Documents/Downloads 与 SOLOSOUL_FS_BASE 均不可解析。移动端不受影响（`allowed_fs_bases` 含应用缓存目录，实际恒非空）。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅。
### P016（P2）PIN/密码命令 Zeroizing 包装 — 已完成（commit 3a5f68c0）

**原问题**：`pin_setup`/`pin_disable`/`pin_unlock`/`change_password` 的 password/pin 参数为普通 `String`，明文残留堆内存（PinManager 以 `&str` 处理，内部派生密钥已由 solosoul-crypto 层 Zeroizing，但命令边界字符串未清零）。

**修复**：四命令入口处用 `zeroize::Zeroizing::new(...)` 包装 password/pin（P015 `import_execute_advanced` 同款模式），所有权转移后 drop 即清零；命令签名保持 `String`（`Zeroizing<String>` 直接作 CommandArg 需 zeroize `unstable` feature，避免改动 workspace 依赖）。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅。
### P017（P2）resolve_within 返回 canonical 路径 — 已完成（commit 0869e4e1）

**原问题**：`resolve_within`（fs.rs:133）已用 `target_canon` 做越界校验，但返回字面路径 `abs`——校验与后续文件操作使用不同路径，符号链接场景存在 TOCTOU 竞态窗口。

**修复**：改返回 `target_canon`（canonicalize_existing 已解析全部现存组件并追加不存在的尾段），校验与文件操作使用同一已解析路径；同步更新 `resolve_allowed_path` 文档注释（不再「保持与旧行为一致」返回原路径）。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅。
### P018（P2）OCR debug 日志脱敏 — 已完成（commit 0a8de3c7）

**原问题**：`macos_vision.rs` 的 debug 日志记录用户扫描图片完整路径（含用户目录名，如 `/Users/<name>/...`），GUI tracing 落盘后构成敏感信息泄露面。

**修复**：debug 日志只记 `image_path.file_name()`（尾部组件），无文件名时记 `<unknown>`；Vision CLI 内部二进制路径保留（非用户数据，诊断仍可用）。排查确认 `commands/ocr.rs` 与 `mobile_ocr_plugin.rs` 无其他用户路径入日志（仅模型下载重试 warn 记录错误信息，无路径）。

**验证**：`cargo check -p solosoul-core --tests` ✅；clippy --workspace 全绿 ✅。
### P019（P2）log_write action_type 白名单 — 已完成（commit ac613f87）

**原问题**：`log_write`（log.rs:65-76）允许前端写任意 `action_type` 审计条目，可伪造登录/导出/备份等系统级动作，审计日志作为安全证据的可信度受限。

**修复**：新增 `FRONTEND_LOG_ACTION_TYPES` 枚举白名单（仅 `critical_field_login`/`critical_field_pin`/`critical_field_touch_id`/`critical_field_windows_hello`/`critical_field_face_id`——前端两个调用点（HistoryViewer/ObjectDetailModal 关键字段查看）实际使用的 5 类），白名单外 action_type 直接拒绝。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅。

### P020–P026（P2，架构/规范类）

### P020（P2）object 写命令服务端派生账户 — 已完成（commit 11ec4c31）

**原问题**：`object_create` 用客户端 `input.account_id` 写对象、`page_delete` 用客户端 account_id 过滤删除目标——陈旧 accountId（切换账户后前端残留）会把写入/删除对准错误账户；template/ocr 已统一 `current_account(&state)?` 服务端派生约定。

**修复**：`object_create` 在命令入口派生 `current_account(&state)?` 并用于 `record.account_id`（忽略客户端值）；`page_delete` 同样派生并忽略客户端参数（参数改名 `_account_id` 保持前端 invoke 兼容）。`object_update`/`object_delete` 本就按 object_id 操作（记录自带 account_id），无需改动。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅；tsc ✅（前端 invoke 参数不变）。
### P021（P2）TS ProviderConfig 补 embeddingModel — 已完成（commit 59d7feed）

**原问题**：Rust `ProviderConfig`/`ProviderWithKey` 均有 `embedding_model`（serde camelCase），TS `ProviderConfig` 缺 `embeddingModel`——类型未声明的字段在重构保存逻辑（对象重建/字面量构造）时会静默丢失。

**修复**：TS `ProviderConfig` 补 `embeddingModel?: string`，与 `llm_get_providers` 实际返回的 `ProviderWithKey` IPC 载荷对齐（`LlmConfigPage` 用 `invoke<ProviderConfig[]>` 接收的就是该载荷）。保存路径 `llm_save_provider` 整体覆盖语义保持不变——前端编辑时 spread 保留运行时字段，类型补全后任何新构造路径都携带该字段。

**验证**：tsc ✅；eslint ✅（无 ProviderConfig 相关测试）。
### P022（P2）DTO 移除 salt/verifyHash — 已完成（commit 91835380）

**原问题**：`auth.rs::AccountInfo` 与 `solosoul-core::AccountSummary` 把 `salt`、`verifyHash` 序列化给前端但前端零消费（`ipc.ts` 声明后无任何读取），扩大 WebView 攻击面且误导开发。

**修复**：两个 DTO 移除 salt/verify_hash 字段（`list_accounts` 的解构元组同步简化）；TS `AccountInfo` 删除 `salt?`/`verifyHash?`；auth.rs 的 `test_account_info_serialization` 更新为不含两字段的断言。CLI 使用 AccountSummary 但不读 salt/verify_hash，无需改动。

**验证**：`cargo check -p solosoul-core -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅；CLI 编译 ✅；tsc ✅；eslint ✅。
### P023（P2）收敛 core 单一 AccountSummary — 已完成（commit 78cae3be）

**原问题**：`auth.rs::AccountInfo` 与 `solosoul-core::AccountSummary` 字段重叠（id/name/password_hint/created_at）但可选性与字段集不同（AccountInfo 缺 has_biometric_history/has_pin_history），两处易演进漂移。TS `ipc.ts::AccountInfo` 已与 AccountSummary 对齐，重复的只有 Rust 端。

**修复**：删除 `auth.rs::AccountInfo`，`bootstrap` 返回类型改为 core 的 `AccountSummary`（新账户标志位 false）；`test_account_summary_serialization` 改用 AccountSummary 并补两个布尔字段断言。CLI/e2e 无 AccountInfo 引用，无需改动。

**验证**：`cargo check -p solo_soul --tests` ✅；clippy --workspace 全绿 ✅；全仓 grep 无 AccountInfo 残留 ✅。
- **P024**：`solosoul-crypto` 提供与错误类型无关的核心 KDF 函数，两端薄包装各自映射错误类型。
- **P025**：评估 `panic = "unwind"`；若保留 abort，clippy 禁非测试代码新增 `unwrap/expect`。
- **P026**：command 边界统一校验（名称 ≤ N 字符、载荷 ≤ M MB、偏好 key 白名单/前缀约束）。

### P027–P033（P2，死代码类）

- **P027**：删除 `trash_permanent_delete` 函数 + `lib.rs:389` 注册 + `:870` 测试清单，守卫测试总数断言 194→193；清理 `commands/object/tests.rs:508` 注释引用。**注意：删除即破坏 API 完备性，按流程约束属「删除文件/代码」类，需用户确认后执行。**
- **P028–P031**：删除对应定义/re-export/export；若属预留 API 则加 `#[allow(dead_code)]` + 注释说明意图。
- **P032**：删除 `lib.rs:643-644` 过时注释块。
- **P033**：`setup_detect_locale` 结果仅用于一行 debug 日志，需人工确认是否保留诊断；建议删除该步骤。

### P034–P039（P2，结构优化类）

- **P034**：usage 提取按 Anthropic/OpenAI 各抽函数（`extract_delta_text` 是现成好例子）。
- **P035**：内层「解析 local_id」抽纯函数 `resolve_template_id(vault, tpl, hash, now)`。
- **P036**：对象分支按阶段拆 `collect_field_defs()`/`resolve_field_order()`/`resolve_sensitivity()`。
- **P037**：敏感度合并（:27-45）与孤儿过滤（:136+）各抽函数。
- **P038**：元数据逐项对比抽 `collect_metadata_diff(old_def, prop)`。
- **P039**：定义单个 `const SYSTEM_SECTIONS: &[(&str, &str)]`，数组/映射/集合均由它派生。

### P040–P045（P2，前端重复代码类）

- **P040**：抽 `PinEntryCard` 共享组件。
- **P041**：抽 `OcrTierStatusRow` 组件。
- **P042**：改用共享 `Dialog` 或抽 `QrModalShell`。
- **P043**：抽 `DownloadProgressBar` 小组件。
- **P044**：抽 `resolve_peer_client_type(vault_result, txt, node_id)` 共享函数（抽取前确认两端语义）。
- **P045**：人工核对三处快照行渲染后抽共享组件。

### P046–P047（P2，巨型组件/文件汇总）

- **P046**：10 个 >300 行前端组件按「数据 hook + 展示子组件」逐步拆分（可拆为多个独立修复任务）。
- **P047**：`attachment.rs`(2057)、`object/tests.rs`(2353)、`export_docx.rs`(1989) 文件级拆分，作为后续架构项。

## 已核查确认无问题的领域（排除误报）

- **XSS**：全库无 `dangerouslySetInnerHTML`/`eval`/`srcDoc`；markdown 统一收口 `SafeMarkdown`（无 rehype-raw）；URL 协议白名单正确。
- **命令注入**：仅 3 处 `Command::new`，参数均为程序生成路径，无用户输入拼接；macOS Vision CLI 有 sha256+权限+执行前哈希校验防 TOCTOU。
- **路径遍历核心路径**：fs.rs 全部命令经 `resolve_allowed_path`；`attachment_download` 双向校验完备；插件输出文件 canonical 前缀校验；无 zip-slip。
- **硬编码密钥/弱加密**：无；PIN/恢复密码用 CSPRNG；恢复 PIN 校验常数时间比较。
- **SQL 注入**：全部参数化；唯二 `format!` SQL 用编译期常量。
- **IPC ACL**：194 条命令与权限白名单逐条一致（有防回归单测）。
- **供应链完整性**：embed 注册表 minisign、插件注册表签名 + WASM sha256、OCR 模型 pinned sha256、updater Tauri 签名通道。
- **Crate 依赖图**：单向 DAG 无循环依赖。
- **Zustand/后端一致性**：`vault-locked` 事件清理链完整；searchCache TTL + 写失效；settingsStore 写入矩阵文档化。
- **性能已优化领域**：附件/导出分块加密、缩略图 LRU 缓存、全表解密 spawn_blocking、前端大列表分页 + memo、批量计数 IPC 等（P005/P020/P114/P210 等历史专项成果）。
- **serde untagged**：零命中。unsafe：均为平台 FFI 且合理。全库 TODO/FIXME 注释为 0。

## 备注

- 多处「需人工确认」项已在表格与详情中标注，修复前请先确认前提（GitHub assets 排序、Android 兜底路径可达性、PinManager 内部清零、tracing 落盘位置、log_write 设计意图、陈旧 accountId 触发路径等）。
- 按流程约束，涉及删除文件/代码的项（P027–P031 等）执行前需用户确认。
- 待用户指令后开始阶段 3 迭代修复（建议顺序：P001 → P002 → P003–P005（Rust 性能）→ P006–P009（Rust 架构）→ P010–P011（前端重复）→ P2 批次）。
