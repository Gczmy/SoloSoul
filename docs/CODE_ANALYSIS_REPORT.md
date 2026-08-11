# 代码分析修复报告

> 最后更新：2026-08-11（P002 修复完成）
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
| P003 | P1 | 性能 | `tauri/src-tauri/src/commands/export_import/mod.rs:290`、`export.rs:183` | 导出 selected 分支全库解密两遍（`list_objects` + `load_objects_batch`），且随导出页每次勾选变更（500ms 防抖）触发 | `[ ]` 待修复 |
| P004 | P1 | 性能/架构 | `tauri/src-tauri/src/commands/llm/conversation.rs:13-57` | LLM 会话整体存加密 preferences blob，每次保存 = 全量解密+深克隆+序列化+加密+写盘；每条聊天消息都触发 | `[ ]` 待修复 |
| P005 | P1 | 性能 | `solosoul_cli/src/commands/search.rs:181-184,206-209` | CLI `/search` 用 `list_objects(...).len()` 统计子对象数，对每个命中页面全量解密仅为取计数（GUI 已有 `count_objects` 先例） | `[ ]` 待修复 |
| P006 | P1 | 架构 | `tauri/src-tauri/src/commands/object/mod.rs:439`、`tauri/src/stores/objectStore.ts:56,185`、`ObjectDetailModal.tsx:470` | 类型漂移：Rust `ObjectData` 无 `tags`，TS 声明 `tags?` 永为 undefined；`updateObject` 用 undefined 覆盖摘要 tags，详情页标签成死渲染路径 | `[ ]` 待修复 |
| P007 | P1 | 架构 | `tauri/src-tauri/src/commands/export_import/export_docx.rs:200` | `flatten_object_fields` 6–7 层控制流嵌套，动态字段组展平逻辑难读难测 | `[ ]` 待修复 |
| P008 | P1 | 架构 | `tauri/src-tauri/src/commands/search/query.rs:16` | `search_properties_for_matches` 133 行递归函数承担 4 种职责，`__fields` 分支 5–6 层嵌套 | `[ ]` 待修复 |
| P009 | P1 | 架构 | `tauri/src-tauri/src/commands/attachment.rs:1130` | `attachment_share` ~161 行，macOS/Windows 两个 `#[cfg]` 块各重复约 40 行「复制→主线程调度→oneshot」骨架 | `[ ]` 待修复 |
| P010 | P1 | 规范 | `tauri/src/components/layout/AddPageButton.tsx:162-198` ↔ `NavButton.tsx:37-73` | 悬停卡片 portal 定位逻辑 ~37 行逐字符复制（注释自认 same pattern），应抽共享 hook | `[ ]` 待修复 |
| P011 | P1 | 规范 | `tauri/src/components/layout/AddPageButton.tsx:438-520` ↔ `CustomPageEditPopover.tsx:322-409` | 图标分类选择器 ~44 行 + 分类数组两处复制，应抽 `IconCategoryPicker` 共享组件 | `[ ]` 待修复 |
| P012 | P2 | 漏洞 | `tauri/src-tauri/src/commands/update.rs:238-246` | Release 资产匹配过宽（`contains("sha256")` 会命中 `.minisig`），完整性校验可能静默失效（需人工确认 assets 顺序） | `[ ]` 待修复 |
| P013 | P2 | 漏洞 | `tauri/src-tauri/src/commands/export_import/import.rs:9,59,183` | 导入命令 `file_path` 无白名单（与 fs 命令 P107 收窄策略不一致），构成有限任意文件探测原语 | `[ ]` 待修复 |
| P014 | P2 | 漏洞 | `tauri/src-tauri/src/commands/attachment.rs:445-482` | `attachment_copy_to_vault` 兜底分支字面 `starts_with` 且未拒绝 `..` 组件，Android symlink 场景可绕过 allowed-dir（需人工确认可达性） | `[ ]` 待修复 |
| P015 | P2 | 漏洞 | `tauri/src-tauri/src/commands/export_import/export.rs:337-340` | 导出路径白名单为空时 fail-open 放行任意路径（`attachment_download:911` 同款），应 fail-closed 或至少 warn | `[ ]` 待修复 |
| P016 | P2 | 规范 | `tauri/src-tauri/src/commands/pin.rs:42-59,103-113`、`vault.rs:34-45` | `pin_setup`/`pin_disable`/`pin_unlock`/`change_password` 的密码参数未在 IPC 边界 Zeroizing 包装（P031 模式不一致，需确认 PinManager 内部清零） | `[ ]` 待修复 |
| P017 | P2 | 漏洞 | `tauri/src-tauri/src/commands/fs.rs:133-142` | `resolve_within` 校验后返回未规范化路径，存在符号链接 TOCTOU 窗口（威胁模型较低） | `[ ]` 待修复 |
| P018 | P2 | 规范 | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:281-298` | OCR debug 日志记录用户扫描图片完整路径（GUI 侧 tracing 落盘位置需确认） | `[ ]` 待修复 |
| P019 | P2 | 架构 | `tauri/src-tauri/src/commands/log.rs:65-76` | `log_write` 允许前端写任意 `action_type` 审计条目，无枚举白名单，审计日志作为安全证据的可信度受限（需确认设计意图） | `[ ]` 待修复 |
| P020 | P2 | 架构 | `tauri/src-tauri/src/commands/object/mod.rs:604` 等 | object 系命令信任客户端 `account_id`（template/ocr 等已统一 `current_account` 服务端派生，两种约定并存，需确认触发路径） | `[ ]` 待修复 |
| P021 | P2 | 架构 | `tauri/src/types/llmProvider.ts:5` vs `tauri/crates/solosoul-core/src/llm/config.rs:33` | 潜伏类型漂移：TS `ProviderConfig` 缺 `embeddingModel`，重构保存逻辑时会静默重置该字段 | `[ ]` 待修复 |
| P022 | P2 | 死代码 | `tauri/crates/solosoul-core/src/vault_service.rs:493-518`、`tauri/src-tauri/src/commands/auth.rs:12-19` | 登录/账户列表把 `salt`、`verifyHash` 序列化给前端但前端零消费，扩大 WebView 攻击面且误导开发 | `[ ]` 待修复 |
| P023 | P2 | 规范 | `tauri/src-tauri/src/commands/auth.rs:12` vs `solosoul-core/src/vault_service.rs:195` | `AccountInfo`/`AccountSummary` 重复 DTO 字段重叠但可选性不同，易演进漂移 | `[ ]` 待修复 |
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

- 已完成：2 / 47
- 当前处理：P003（按建议顺序推进）

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

### P003（P1）导出 selected 分支全库双重解密

`collect_scope_objects` selected 分支 `list_objects(account_id, None...)` 对全账户逐行完整解密仅为筛 id，随后 `load_objects_batch` 对命中对象再解密一次；`export_estimate_size` 随导出页每次勾选变更（500ms 防抖）触发该路径。P005 只修了 include_all 分支。
**修复**：页面/标签过滤均可下推 SQL（`section_type`、`tags_json` 列未加密），无需解密 properties；估算字节数参照 `snapshots_size_batch` 先例用 `SUM(LENGTH(properties))` 纯 SQL。

### P004（P1）LLM 会话整 blob 重写

全部会话（每条最多 500 消息）存于加密 profile preferences blob；任何 save/list/rename/delete 都整 blob 解密→修改→加密→写盘，且 `save_conversations` 先 `to_vec()` 深克隆全部会话（:45）。每条聊天消息流式结束都触发 `llm_save_conversation`（`useLlmChatCore.ts:263`），开销随历史线性增长。
**修复**：短期——按 id 定位仅克隆/trim 目标会话；中期——会话改存独立 SQLite 表（按 conversation_id 行存储），与 objects/audit_log 存储方式对齐。

### P005（P1）CLI 搜索计数全量解密

`search_pages` 用 `list_objects(...).map(|v| v.len())` 统计子对象数，`list_objects` 对每行 AES-GCM 解密 + JSON 解析仅为取计数，命中多个分区时相当于多次全库解密。
**修复**：改用 GUI 已在用的 `VaultStore::count_objects`（`solosoul-vault/src/storage/objects.rs:715`，纯 SQL COUNT 不解密），改动极小。

### P006（P1）ObjectData tags 类型漂移

Rust `record_to_data` 不返回 `tags`，TS `ObjectData.tags?` 永为 undefined；`updateObject`（objectStore.ts:185）将 `tags: obj.tags`（undefined）显式覆盖进摘要列表项，抹掉 `object_list` 摘要已有的 tags；详情弹窗 `obj.tags`（ObjectDetailModal.tsx:470）是死渲染路径。
**修复**：Rust `ObjectData` 增加 `tags`（或 TS 删除该字段改为仅从 `ObjectSummary` 读）；`updateObject` 摘要同步只合并后端实际返回的字段。

### P007–P009（P1）深嵌套/过长函数拆分

- `flatten_object_fields`（export_docx.rs:200）：`__` 元数据键处理与 dynamic_group 展平各抽独立函数，主循环只做分发。
- `search_properties_for_matches`（query.rs:16）：`__fields` 分支抽 `match_field_defs()`；值匹配打分/截断抽 `push_value_match()`。
- `attachment_share`（attachment.rs:1130）：按平台拆 `share_macos`/`share_windows`/`share_android` 子函数，共享「复制到分享目录 + 主线程调度 + oneshot」模板。

### P010–P011（P1）布局组件复制

- `AddPageButton.tsx:162-198` ↔ `NavButton.tsx:37-73`：抽共享 hook `useHoverCardPosition(wrapperRef, {isHorizontal, isBottom, isRight})`。
- `AddPageButton.tsx:438-520` ↔ `CustomPageEditPopover.tsx:322-409`：抽 `IconCategoryPicker` 共享组件，分类数组移到 `CUSTOM_ICON_MAP` 旁导出常量。

### P012–P019（P2，安全类）

- **P012**：资产匹配改 `ends_with(".sha256") && !ends_with(".minisig")`；验签失败向前端返回可感知警告。
- **P013**：导入命令桌面端对 `file_path` 复用 `resolve_allowed_path`（Desktop/Documents/Downloads + SOLOSOUL_FS_BASE）。
- **P014**：与 `attachment_download` 对齐，入口处拒绝 `Component::ParentDir`。
- **P015**：桌面端白名单为空改为拒绝（fail-closed），或至少 `tracing::warn`。
- **P016**：`pin_setup`/`pin_disable`/`pin_unlock`/`change_password` 统一套用 P031 Zeroizing 模式。
- **P017**：`resolve_within` 返回 `target_canon` 或注释说明接受的残余风险。
- **P018**：OCR debug 日志只记文件名尾部组件或哈希。
- **P019**：`log_write` 对 `action_type` 加枚举白名单。

### P020–P026（P2，架构/规范类）

- **P020**：object 写命令统一 `current_account(&state)?` 派生，忽略/校验客户端 account_id（先人工确认陈旧 accountId 触发路径）。
- **P021**：TS `ProviderConfig` 补 `embeddingModel?: string`，或 `llm_save_provider` 合并旧值而非整体覆盖。
- **P022**：Rust 两个 DTO 去掉 salt/verify_hash 字段（或 `skip_serializing`），TS 同步删除。
- **P023**：收敛为 core 单一 `AccountSummary`，auth.rs 复用。
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
