# 代码分析修复报告

> 最后更新：2026-08-08 01:35:00
> 当前分支：`main`
> 修复轮次：1（初始分析，按用户要求不恢复旧报告、全新生成）
> 修复进度：P001-P012 已闭环（12/45）
> 分析范围：`tauri/`（Rust 后端 + React/TS 前端）、`solosoul_cli/`；忽略 `node_modules/`、`target/`、`dist/`、`.vite/`、`*.min.js`、`*.wasm`

## 阶段 0 基线检查结果

- Git 仓库状态：干净（`main` 分支，无未提交改动）。
- `npm run check-all`（tsc → cargo fmt --check → clippy → ESLint → Vitest 570 测试）：**全部通过**。
- 结论：静态工具基线为绿，以下问题全部来自启发式人工/代理分析。

## 问题清单（按优先级 P0 > P1 > P2）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| P001 | P0 | 架构/正确性 | `tauri/src-tauri/src/commands/sync.rs:34-48`、`tauri/crates/solosoul-sync/src/types.rs:59-65`、`tauri/src/lib/ipc.ts:1-15` | 同步冲突载荷 `local_hlc.node_id` 桌面端序列化为 `number[]`、移动端为 `String`，前端只建模桌面形状，Android 上类型漂移 | `[x]` 已修复 |
| P002 | P0 | 状态一致性 | `tauri/src/stores/syncStore.ts:524-536,588-601` | 入站同步只刷新 objectStore，不同步刷新 templateStore / profileStore / trashStore，模板与回收站数据陈旧 | `[x]` 已修复 |
| P003 | P1 | 漏洞 | `tauri/src-tauri/src/attachment_import_plugin.rs:516-519,579-580` | 附件导出路径白名单被原始字符串前缀匹配绕过，解锁态可导出任意本地文件 | `[x]` 已修复 |
| P004 | P1 | 正确性/性能 | `tauri/crates/solosoul-vault/src/storage/metadata.rs:632-647` | `check_field_usage` 对加密列做 `LIKE` 匹配，结果恒为 0（功能失效）且全表扫描 | `[x]` 已修复 |
| P005 | P1 | 性能 | `tauri/src-tauri/src/commands/export_import/mod.rs:268-306` | 导出收集对象时全库双重解密 + N+1 查询 | `[x]` 已修复 |
| P006 | P1 | 安全 | `solosoul_cli/src/screens/object_detail.rs:45,87-92` | CLI `/open` 对象详情只看对象级敏感度，字段级 sensitive/critical 明文渲染，违反 P036 掩码约定 | `[x]` 已修复 |
| P007 | P1 | 安全 | `solosoul_cli/src/commands/backup.rs:218-250` | `/backup create` 明文 profile 备份以默认权限（0644）落盘，未收紧 0600 | `[x]` 已修复 |
| P008 | P1 | 正确性 | `solosoul_cli/src/commands/vault_write.rs:300-310` | `delete_page` 中子对象移入回收站失败被 `let _ =` 静默吞掉，页面照删 | `[x]` 已修复 |
| P009 | P1 | 死代码 | `tauri/src-tauri/src/commands/ocr.rs:535,547`、`export_import/export.rs:704`、`template.rs:433`、`crates/solosoul-core/src/vault_service.rs:1651` | 3 个孤儿 Tauri Commands（无前端调用）+ 1 个仅测试使用的生产函数 | `[x]` 已修复 |
| P010 | P1 | 重复代码 | `tauri/src/hooks/useUpdateChecker.ts:137-178` vs `useAppUpdate.ts:100-176` | 约 80 行 APK 下载/进度 Promise 封装近乎逐行重复，已开始各自打补丁发散 | `[x]` 已修复 |
| P011 | P1 | 重复代码 | `tauri/src/components/object/AttachmentViewer.tsx:195-217` vs `tauri/src/hooks/useAttachmentManager.ts:186-208` | `handleDownload` 逐字符级重复（含动态 import、toast 文案） | `[x]` 已修复 |
| P012 | P1 | 重复代码 | `tauri/src/pages/sync/DeviceListPanel.tsx:128-151` vs `:329-352` | 约 40 行设备卡片 JSX（含键盘可访问性）整段重复 | `[x]` 已修复 |
| P013 | P1 | 可维护性 | `AttachmentViewer.tsx`(~660 行)、`LoginPage.tsx`(~654)、`ExportImportPage.tsx`(~643)、`PageGuide.tsx`(~615)、`useObjectWorkspaceData.ts`(~524)、`PasswordVerificationDialog.tsx`(~516) | 前端 6 个 500+ 行巨型组件/Hook | `[ ]` 待修复 |
| P014 | P1 | 可维护性 | `crates/solosoul-plugin/src/host.rs:437,662,893`（530+ 行合计）、`solosoul-vault/src/storage.rs:482`、`export.rs:491` 等 | Rust 侧多个 150-300 行超长函数 | `[ ]` 待修复 |
| P015 | P2 | 安全 | `tauri/src-tauri/src/commands/llm/request.rs:214-245` | LLM base URL 允许公网 `http://`，Bearer key 与聊天内容明文传输；与 OCR 侧策略不一致 | `[x]` 已修复（非回环 host 强制 https，回环保留 http；新增 `is_loopback_host` + 单测 5 条） |
| P016 | P2 | 安全 | `tauri/src-tauri/src/commands/llm/stream.rs:388` | 流式聊天未调用 `ensure_public_llm_host`，内网主机名可绕过 SSRF 封禁 | `[x]` 已修复（`llm_send_message_stream` 补 `ensure_public_llm_host` 异步解析复核，与 chat_http 一致） |
| P017 | P2 | 安全 | `tauri/src-tauri/tauri.conf.json:30` | CSP `object-src data:` 允许加载 `data:text/html`，基线应为 `object-src 'none'` | `[ ]` 待修复 |
| P018 | P2 | 安全 | `tauri/src-tauri/src/commands/attachment.rs:96-100` | `path_within_base` canonicalize 失败兜底分支仍放行含 `..` 的路径 | `[ ]` 待修复 |
| P019 | P2 | 安全 | `tauri/src-tauri/src/attachment_import_plugin.rs:477-482,507-512` | 生产代码遗留 error 级 debug 日志，输出用户文件完整路径 | `[ ]` 待修复 |
| P020 | P2 | 性能 | `tauri/crates/solosoul-vault/src/storage/objects.rs:329-485`、`tauri/src-tauri/src/commands/object/mod.rs:459-488` | `object_list` 全量解密并经 IPC 传输全部对象完整 properties，未按注释做预览截断 | `[ ]` 待修复 |
| P021 | P2 | 性能 | `tauri/src-tauri/src/commands/search/query.rs:28-105` | 全库搜索热循环内大量临时分配（to_lowercase / format! / 全排序取最大值） | `[ ]` 待修复 |
| P022 | P2 | 性能 | `tauri/src-tauri/src/commands/llm/rag.rs:395-401` | 每次指南检索重读磁盘并重切块全部指南 markdown 仅为构建 title 映射 | `[ ]` 待修复 |
| P023 | P2 | 性能 | `tauri/src-tauri/src/commands/template.rs:22-33,121-144` | 每次模板 IPC 都执行 legacy 迁移检查（含 profile 全量加载解析），迁移完成后永久无效 | `[ ]` 待修复 |
| P024 | P2 | 性能 | `tauri/src/stores/trashStore.ts:137-155` | 回收站批量永久删除逐条 IPC，无批量端点 | `[ ]` 待修复 |
| P025 | P2 | 性能 | `tauri/crates/solosoul-vault/src/storage/objects.rs:282-327` | `list_object_attachment_ids` 为取附件 id 解密全部对象完整 properties | `[ ]` 待修复 |
| P026 | P2 | 性能 | `tauri/src/components/llm/ChatMessageList.tsx:178` | 长会话消息列表无分页/虚拟化（项目其它列表均已分页） | `[ ]` 待修复 |
| P027 | P2 | 安全 | `solosoul_cli/src/commands/settings.rs:342-397` | CLI `/debug_log` 解密审计日志默认权限落盘，未收紧 0600（与 log.rs 行为不一致） | `[ ]` 待修复 |
| P028 | P2 | 安全 | `solosoul_cli/src/commands/log.rs:56-69` | `/export_log` 先默认权限创建文件再 chmod，存在明文窗口期，且 chmod 失败被吞 | `[ ]` 待修复 |
| P029 | P2 | 安全 | `solosoul_cli/src/app.rs:776-791`、`commands/auth.rs:64-72` | 锁定/自动锁定不清理 previous_phase、chat_state、prompt，内存残留解密数据 | `[ ]` 待修复 |
| P030 | P2 | 正确性 | `solosoul_cli/src/app.rs:813-818`、`commands/llm.rs:35` | 插件成功结果与 `/model` 信息性输出误用 `error_message` 通道（渲染为红色错误） | `[ ]` 待修复 |
| P031 | P2 | 正确性 | `solosoul_cli/src/commands/history.rs:88-123` | `do_rollback` 未校验 snapshot 归属对象/账户，可把别的对象数据套到本对象 | `[ ]` 待修复 |
| P032 | P2 | 正确性 | `solosoul_cli/src/commands/mod.rs:61-68` | `update_profile_preference` 根非 object 时静默跳过仍返回 Ok，偏好写入丢失 | `[ ]` 待修复 |
| P033 | P2 | 正确性 | `solosoul_cli/src/commands/backup.rs:222-233` | 单个 profile 加载失败被静默跳过，备份可能不完整且无警告 | `[ ]` 待修复 |
| P034 | P2 | 性能 | `solosoul_cli/src/commands/search.rs:283-287` | 每条搜索结果单独 `load_object(parent_id)` 解析父页面名，N+1 模式 | `[ ]` 待修复 |
| P035 | P2 | 安全 | `solosoul_cli/src/commands/plugin.rs:102-106` | `plugin_id` 未做字符白名单校验直接拼接路径 | `[ ]` 待修复 |
| P036 | P2 | 质量 | `solosoul_cli/src/app.rs:609-636,1760,789` | 多处硬编码中文字符串绕过 `t!()` i18n 体系 | `[ ]` 待修复 |
| P037 | P2 | 质量 | `solosoul_cli/src/app.rs:2694-3016` 等 | `render()` 323 行及多个 110-150 行 key handler | `[ ]` 待修复 |
| P038 | P2 | 质量 | `solosoul_cli/src/commands/settings.rs:6,340-341`、`app.rs:364,412`、`commands/profile.rs:171-180` | 陈旧注释（引用不存在的 `trigger_debug_log_export`）、文档注释混入字面 `\n`、死分支注释误导 | `[ ]` 待修复 |
| P039 | P2 | 死代码 | `tauri/src/lib/ipc.ts:86-93` | `Profile` 接口无人引用，且字段形状与 Rust 端不符 | `[ ]` 待修复 |
| P040 | P2 | 错误处理 | `tauri/src-tauri/src/commands/sync.rs:289-294` | `parse_hlc_json` 用 `unwrap_or(0)/unwrap_or("")` 静默吞掉畸形 HLC | `[ ]` 待修复 |
| P041 | P2 | 错误处理 | `tauri/src-tauri/src/commands/settings.rs:74,81` | `remove_with_retry` 依赖读者推理的 `unwrap()` | `[ ]` 待修复 |
| P042 | P2 | 架构 | `solosoul-vault/src/lib.rs:255`、`commands/object/mod.rs:119`、`objectStore.ts:20`、`conflictFieldMeta.ts` | `type_id` 一个字段三套命名（typeId / collectionType），前端维护两套词汇 | `[ ]` 待修复 |
| P043 | P2 | 可维护性 | `tauri/src-tauri/src/sync/auto_sync_core.rs:82-130`、`PluginQuickPanel.tsx:244-270` | 控制流嵌套达 8-9 层 | `[ ]` 待修复 |
| P044 | P2 | 重复代码 | `tauri/src/components/plugin/PluginQuickPanel.tsx:249-268`、`tauri/src/pages/ai/PluginDashboardPage.tsx:~170-190` | 水印插件"先选附件"校验逻辑在两个入口重复，且插件 ID 硬编码于通用面板 | `[ ]` 待修复 |
| P045 | P2 | 质量 | `tauri/src/hooks/useRevealState.ts:90` | 全仓唯一实质 TODO（字段类型感知部分掩码），建议转为 issue 跟踪 | `[ ]` 待修复 |

## 修复进度

- 已完成：12 / 45（P001-P012）
- 当前处理：P013/P014 巨型组件拆分（长期重构项，见备注）

## 修复实施记录

### P012（P1）设备卡片 JSX 抽取共享外壳 ✅

- **改动**：新增 `tauri/src/pages/sync/DeviceCard.tsx` `DeviceCardShell`（交互容器 interactive-card-lift + 键盘可访问性 + 客户端类型图标 + 名称行），已发现设备与已知设备两张卡片改为注入 `subtitle`/`actions` 子区域，删除约 40 行重复 JSX。
- **验证**：`tsc --noEmit` ✅ / eslint ✅。

### P011（P1）附件 handleDownload 重复抽取共享入口 ✅

- **改动**：`tauri/src/lib/attachmentUtils.ts` 新增 `downloadAttachmentFile`（saveWithPause + downloadViaStage + 成功/失败 toast 统一），依赖经参数注入保持纯函数可测；`AttachmentViewer.tsx` 与 `useAttachmentManager.ts` 两处逐字符重复的 `handleDownload` 改写为单次调用。
- **测试**：新增 `attachmentUtils.test.ts` 3 用例——成功保存、取消不下载、下载失败 toast。
- **验证**：`tsc --noEmit` ✅ / eslint ✅ / `vitest attachmentUtils+updater` 7 用例全过 ✅。

### P010（P1）APK 下载逻辑抽取共享封装 ✅

- **改动**：`tauri/src/lib/updater.ts` 新增 `ensureApkDownloaded(version, url, checksum, onProgress)`——统一「检查已下载短路 → 启动事件驱动下载 → 等待 done 终态 → finally 清理 unlisten」逻辑，返回 boolean（true=本次实际下载）。`useUpdateChecker.ts` 与 `useAppUpdate.ts` 两处 80 行重复改写为单次调用。
- **测试**：新增 `updater.test.ts` 4 用例——已下载短路、成功进度回调、失败 reject + 监听清理、成功终态。
- **验证**：`tsc --noEmit` ✅ / eslint ✅ / `vitest updater.test.ts` 4 用例全过 ✅。

### P009（P1）孤儿 Tauri Commands 与仅测试函数删除 ✅

- **改动**：
  1. 删除 `ocr_get_supported_languages`（desktop/mobile 两个变体 + 单测 + `OcrPage.test.tsx` mock 引用）——前端零调用且未注册。
  2. 删除 `export_get_attachments`（前端只用 batch 版）——函数 + lib.rs 注册 + ACL 白名单条目。
  3. 删除 `template_save_from_object`（前端零调用）——函数 + lib.rs 注册 + ACL 白名单条目。
  4. 删除 `solosoul-core::VaultService::get_vault_state`（仅测试使用），测试改 `is_unlocked()` 断言。
- **验证**：`cargo check -p solo_soul -p solosoul-core` ✅ / template 30 测试 ✅ / core 12 ✅ / OcrPage 6 ✅ / tsc ✅。

### P008（P1）CLI delete_page 子对象移入回收站失败不再吞错 ✅

- **改动**：`solosoul_cli/src/commands/vault_write.rs` 的 `delete_page`——子对象 `move_to_trash` 失败由 `let _ =` 静默吞掉改为 `?` 传播，中止整页删除并报错；页面本身移入失败同样返回 Err（由调用方统一展示）。
- **验证**：`cargo test vault_write` 4 用例全过 ✅。

### P007（P1）CLI /backup create 明文落盘权限收紧 0600 ✅

- **改动**：`solosoul_cli/src/commands/backup.rs`——`fs::write` 后 `#[cfg(unix)]` 显式 `set_permissions(0o600)`，与 log.rs 审计日志导出同一约定，避免 umask 022 下生成 0644。
- **测试**：新增 `test_backup_create_sets_0600_permissions` 断言备份文件 mode == 0600。
- **验证**：`cargo test backup` 6 用例全过 ✅。

### P006（P1）CLI /open 字段级敏感度掩码 ✅

- **改动**：`solosoul_cli/src/screens/object_detail.rs`——掩码判定从对象级升级为字段级：`property_labels` 中 sensitive/critical/restricted 字段掩码、public/internal 字段照常展示；缺失时回退对象级；复用 `solosoul_core::is_protected_sensitivity`。抽纯函数 `field_display_value` 便于测试。
- **测试**：新增 4 个单元测试（critical 掩码/internal 照常/对象级回退/public 覆盖对象级/空值不掩码）。
- **验证**：`cargo test object_detail` 4 用例全过 ✅。

### P005（P1）导出收集对象批量加载——去双重解密与 N+1 ✅

- **改动**：`tauri/src-tauri/src/commands/export_import/mod.rs` 的 `collect_scope_objects`：旧实现先 `list_objects`（不解密）再对每个 id 单独 `load_object`（N+1 二次解密）。现改为：元数据筛选仍用摘要，命中 id 一次 `load_objects_batch` 批量解密加载（单条 SQL + 批量解密），结果按 id 升序保持确定性。
- **测试**：`tests.rs` 新增 `test_collect_scope_objects_batch`——include_all 全量、selected 子集、tags 过滤、空集四路径，断言 properties 已解密。
- **验证**：`cargo check -p solo_soul` ✅ / `cargo test collect_scope_objects` ✅。

### P004（P1）check_field_usage 加密列匹配失效修复 ✅

- **改动**：`tauri/crates/solosoul-vault/src/storage/metadata.rs` 的 `check_field_usage` 删除对密文的 `LIKE "%\"key\":%"` 匹配（恒为 0），改为按 `account_id` 取回 `properties`/`is_deleted` 两列、`decrypt_text_field` 逐行解密后内存判断字段 key 存在（null 值不计）。
- **测试**：`storage.rs` 新增 `test_check_field_usage_decrypts_properties`——活动对象 1 + 软删除对象 1 + 无关对象 1，断言 field-1 计 (1,1)、null 值与缺失 key 均计 0。
- **验证**：`cargo check -p solosoul-vault` ✅ / `cargo test check_field_usage` ✅。

### P003（P1）附件导出路径白名单字符串前缀绕过修复 ✅

- **改动**：
  1. `tauri/src-tauri/src/commands/attachment.rs`：`path_within_base` 提升为 `pub(crate)`（P003 注释注明）。
  2. `tauri/src-tauri/src/attachment_import_plugin.rs`：`attachment_export_content_uri` 与 `attachment_export_tree_uri` 的 `src_path.starts_with(attachments_canon.to_string_lossy())` 字符串前缀匹配改为 `path_within_base` 组件级比较（含 canonicalized 标志与 raw 双路径兜底），杜绝 `attachments_x/../../secret` 或共享前缀兄弟目录绕过。
- **验证**：`cargo check -p solo_soul` ✅ / `cargo test commands::attachment` 14 用例全过 ✅。

### P002（P0）入站同步刷新全部数据 Store ✅

- **改动**：`tauri/src/stores/syncStore.ts` 新增模块级 `refreshDataStores(accountId)` helper（对象列表+详情缓存、模板、回收站、账户偏好设置四类刷新），首事件与合并事件两个 inbound 分支在 `applied > 0` 时共用调用，替换原先仅刷新 objectStore 的逻辑。
- **验证**：`tsc --noEmit` ✅ / eslint ✅ / `vitest src/stores/syncStore.test.ts` 13 用例全过 ✅。

### P001（P0）同步冲突 HLC 载荷跨平台统一 ✅

- **commit**：`（见 git log）`
- **改动**：
  1. `tauri/src-tauri/src/commands/sync.rs`：删除移动端本地复刻 `MobileHlc`，桌面/移动共用统一 DTO `SyncConflictDto` + `ConflictHlc`（`node_id: String`，hex 编码）；桌面端 `From<&ApplyStats>` 经新增 `conflict_to_dto` 转换，移动端 `sync_with_device` 同样 hex 编码构造，两端载荷形状一致。
  2. `tauri/src/lib/ipc.ts`：`SyncConflict.local_hlc/remote_hlc.node_id` 类型 `number[]` → `string`。
  3. `tauri/src/pages/sync/SyncHistoryPanel.tsx`：`formatNodeId(bytes: number[])` 改为直接截取 hex 字符串前 8 位。
- **验证**：`cargo check -p solo_soul` ✅、`cargo check -p solo_soul --target aarch64-linux-android` ✅（仅 6 个既有 warning）、`npx tsc --noEmit` ✅。

## 修复顺序建议（阶段 2）

1. **P0 优先**：P001 → P002（均为同步链路，同一上下文连续处理）。
2. **P1 按栈分组**（减少上下文切换）：
   - Rust/安全与正确性：P003 → P004 → P008 → P005
   - CLI：P006 → P007 → P008（P008 已列于上，CLI 侧随 P006/P007 一并处理）
   - 死代码：P009（删除孤儿命令需同步清理 lib.rs 注册与 ACL）
   - 前端重复代码：P010 → P011 → P012
   - 可维护性拆分（工作量大，建议最后或按需）：P013 → P014
3. **P2**：按 Rust → CLI → 前端分批处理；P039（删死接口）可与 P009 同类合并提交原则下各自独立 commit。
4. 注意：P009、P039 涉及删除代码/文件，按流程约束属破坏性操作边缘——删除函数/接口属正常修复，无文件级删除。

---

## 详细问题描述与修复指引

### P001（P0）同步冲突载荷桌面/移动序列化分裂

- **位置**：`tauri/src-tauri/src/commands/sync.rs:34-48`、`tauri/crates/solosoul-sync/src/types.rs:59-65`、`tauri/src/lib/ipc.ts:1-15`
- **证据**：桌面端 `ConflictRecord.local_hlc` 直接嵌入原始 `Hlc`（`node_id: [u8; 16]`，serde 序列化为 `number[]`）；移动端用本地复刻 `MobileHlc`（`node_id: String`）。前端 `SyncConflict` 只建模了桌面形状（`node_id: number[]`）。另外 `Hlc.counter: u32` 与 `ConflictHlc.counter: u64`（sync.rs:270）宽度也不一致。
- **影响**：Android 上任何读取 `node_id` 的前端冲突处理逻辑都会拿到 string 而非数组，属跨平台真实 bug 隐患。
- **建议**：桌面端也走 `ConflictHlc`（String node_id）统一 DTO，或把 `Hlc` 的 serde 改为 hex string，删除移动端复刻结构。

### P002（P0）入站同步只刷新 objectStore

- **位置**：`tauri/src/stores/syncStore.ts:524-536,588-601`
- **证据**：`solosoul-sync/src/delta.rs:12` 的 `SYNC_TABLES = ["profiles", "objects", "user_templates", "trash_items"]`，对端可推入模板/profile/回收站变更；但 sync-completed 处理器在 `applied > 0` 时仅调用 `useObjectStore.getState().loadObjects(...)` 并清 `currentObjectCache`。
- **影响**：对端同步进新模板/删除模板后，本端模板列表、模板编辑器、回收站页面保持陈旧数据直到手动刷新或重启。
- **建议**：入站处理器按表粒度（或简单地全量）同时触发 `templateStore.loadTemplates()`、`trashStore` 与 profile 的重载。

### P003（P1）附件导出路径白名单字符串前缀绕过

- **位置**：`tauri/src-tauri/src/attachment_import_plugin.rs:516-519,579-580`
- **证据**：

```rust
let in_attachments = src.starts_with(&attachments_canon)
    || src_path.starts_with(attachments_canon.to_string_lossy().as_ref());
```

`src`（canonicalize 后）组件级比较安全，但 **或** 条件右侧是对用户原始字符串 `src_path` 做字面字节前缀匹配。传入 `{attachments_dir}/../../../../etc/passwd`：文件存在 → canonicalize 成功 → 组件比较失败，但原始字符串字面以 `{attachments_dir}` 开头 → 校验通过，随后导出 canonicalize 后的真实路径。
- **影响**：仅要求 Vault 解锁，即可把任意本地文件导出到攻击者指定的 content URI / SAF tree；在 webview 被 XSS 攻破的威胁模型下构成任意文件读取。
- **建议**：按 `commands/attachment.rs` 的 `path_within_base`（R2-X1 已修同款）原则统一——canonicalize 成功时只用 resolved 路径判定，删除原始字符串前缀分支；失败兜底分支显式拒绝含 `ParentDir` 组件的路径。

### P004（P1）`check_field_usage` 对加密列做 LIKE 匹配

- **位置**：`tauri/crates/solosoul-vault/src/storage/metadata.rs:632-647`（调用方 `template_check_field_usage`，`tauri/src-tauri/src/commands/template.rs:275`）
- **证据**：`objects.properties` 列存储的是 `encrypt_text_field` 产物（`"solo:" + base64(随机 nonce 密文)`），而这里执行 `SELECT COUNT(*) ... WHERE properties LIKE '%"field_key":%'`。随机 nonce 密文几乎不可能命中，结果恒为 0。
- **影响**：模板编辑器字段删除前的使用检查失效（功能 bug），且每次对全表做两次 LIKE 全扫描。
- **建议**：改为解密后内存判断（复用 `list_objects` + 解析 JSON key，与搜索路径一致）。

### P005（P1）导出收集对象全库双重解密 + N+1

- **位置**：`tauri/src-tauri/src/commands/export_import/mod.rs:268-306`
- **证据**：`collect_scope_objects` 先 `vault.list_objects(...)`（解密全部对象），`include_all` 分支对每个对象再 `vault.load_object(&summary.id)` 逐个查询并二次解密；`selected_ids` 分支同样逐个 `load_object`。crate 内已有 `list_object_records` 与 `load_objects_batch`（P110）。
- **建议**：`include_all` 直接用 `list_object_records`；选中分支用 `load_objects_batch` 替代逐条 `load_object`。

### P006（P1）CLI `/open` 字段级掩码缺失

- **位置**：`solosoul_cli/src/screens/object_detail.rs:45,87-92`
- **证据**：掩码只看对象级 `sensitivity_level`（`should_mask_level` 仅匹配 sensitive/critical/restricted），完全忽略字段级敏感度（property_labels/模板）。对象级为 public/internal 时，字段级 critical/sensitive 的值以明文渲染。对比 `widgets/field_editor.rs:188-195` 与 `commands/search.rs:228-232` 的 `collect_protected_field_keys`。
- **影响**：与项目「internal/sensitive/critical 一律掩码」收敛约定（P036）不一致，敏感字段在 CLI 明文展示。
- **建议**：复用/移植 `collect_protected_field_keys` 逻辑，详情渲染按字段级掩码。

### P007（P1）CLI 备份明文落盘且权限不符约定

- **位置**：`solosoul_cli/src/commands/backup.rs:218-250`
- **证据**：`/backup create` 把解密后的明文 profile 数据写成 JSON 备份文件，`fs::write` 默认权限（通常 0644），未像 `log.rs:68` 那样收紧 0600。
- **影响**：违反项目「文件 0600」约定；明文副本存在于 SQLite 之外，用户易误以为备份是加密的。
- **建议**：`OpenOptions::mode(0o600)` 创建时定权限（同时修复 P028 的窗口期模式）。

### P008（P1）CLI `delete_page` 吞错导致回收站不一致

- **位置**：`solosoul_cli/src/commands/vault_write.rs:300-310`
- **证据**：循环中对每个子对象 `let _ = objects::move_to_trash(...)`，任何失败被静默吞掉，随后页面照样删除并提示成功。
- **影响**：产生「页面已删但子对象未入回收站」的不一致状态，且无提示。
- **建议**：收集失败项并报告，或任一失败即中止并回滚提示。

### P009（P1）孤儿 Tauri Commands 与仅测试使用的生产函数

- **位置与证据**：
  - `commands/ocr.rs:535,547` `ocr_get_supported_languages`（desktop/mobile 两变体）：带 `#[tauri::command]` 但**未注册**进任何 `generate_handler!`（lib.rs:527-542 不含它），前端无 invoke，IPC 完全不可达。
  - `commands/export_import/export.rs:704` `export_get_attachments`：已注册且有 ACL，但前端只用批处理版 `export_get_attachments_batch`（useExportScope.ts），单对象版无调用点。
  - `commands/template.rs:433` `template_save_from_object`：已注册且有 ACL，前端全仓无 invoke（删除前建议确认产品侧是否计划做「从对象生成模板」）。
  - `crates/solosoul-core/src/vault_service.rs:1651` `pub fn get_vault_state`：生产代码无调用点，仅同文件测试模块使用。
- **建议**：删除 3 个命令时同步清理 `lib.rs` 的注册项与 ACL 字符串（lib.rs:886、:881）及对应测试；`get_vault_state` 改为 `#[cfg(test)]` 可见或删除。属删除类操作，无文件级删除。

### P010（P1）APK 下载逻辑双实现

- **位置**：`tauri/src/hooks/useUpdateChecker.ts:137-178` vs `tauri/src/hooks/useAppUpdate.ts:100-176`
- **证据**：约 80 行 `androidIsApkDownloaded` → `androidDownloadApk(url, checksum, onProgress)` → Promise 封装 → `progress.done/progress.error` 判分支近乎逐行重复；P227 注释显示 settled 标志、unlisten 清理只存在于 useAppUpdate，两套逻辑已开始发散。
- **建议**：抽取共享 `downloadApkWithProgress()`（如 `lib/updater.ts`）。

### P011（P1）附件 handleDownload 逐字符重复

- **位置**：`tauri/src/components/object/AttachmentViewer.tsx:195-217` vs `tauri/src/hooks/useAttachmentManager.ts:186-208`
- **证据**：`handleDownload` 完全相同（动态 import dialog、saveWithPause、downloadViaStage、toast 文案、注释、i18n key）。删除/恢复 handler 同样两处各自实现。
- **建议**：抽公共 `useAttachmentDownload` hook 或让 AttachmentViewer 复用 `useAttachmentManager`。

### P012（P1）设备卡片 JSX 整段重复

- **位置**：`tauri/src/pages/sync/DeviceListPanel.tsx:128-151` vs `:329-352`
- **证据**：约 40 行设备卡片 JSX（role=button、Enter/Space 键盘处理、inline style、`interactive-card-lift`）整段重复，仅数据源不同。
- **建议**：抽 `DeviceCard` 组件，键盘可访问性逻辑只维护一份。

### P013（P1）前端巨型组件/Hook

| 行数（非注释） | 位置 |
|---|---|
| ~660 | `tauri/src/components/object/AttachmentViewer.tsx:55` |
| ~654 | `tauri/src/pages/auth/LoginPage.tsx:34` |
| ~643 | `tauri/src/pages/settings/ExportImportPage.tsx:40` |
| ~615 | `tauri/src/components/guide/PageGuide.tsx:41` |
| ~524 | `tauri/src/hooks/useObjectWorkspaceData.ts:30` |
| ~516 | `tauri/src/components/forms/PasswordVerificationDialog.tsx:72`（本身是 AGENTS.md 规定的统一组件，已膨胀到 500+ 行） |

- **建议**：按「逻辑 hook + 视图子组件」模式逐步拆分；工作量大，建议拆分为多个独立 commit/多轮处理，优先级低于正确性问题。

### P014（P1）Rust 超长函数

| 行数 | 位置 |
|---|---|
| 305 | `solosoul_cli/src/app.rs:2694` `render`（另见 P037） |
| 199 | `tauri/crates/solosoul-plugin/src/host.rs:437` `register_http_fns` |
| 179 | `tauri/crates/solosoul-vault/src/storage.rs:482` `init_schema` |
| 174 | `tauri/crates/solosoul-plugin/src/host.rs:893` `register_interaction_fns` |
| 167 | `tauri/src-tauri/src/commands/export_import/export.rs:491` `export_execute` |
| 161 | `tauri/crates/solosoul-plugin/src/host.rs:662` `register_output_fns` |

另有 `biometric_check_availability`（154）、`import_attachments`（153）、`change_password`（152）、`unlock_with_kdf_upgrade`（147）。`host.rs` 三个 register_* 合计 530+ 行，闭包内联了参数读取/校验/审计/错误码映射。
- **建议**：`host.rs` 抽 `read_args_or_return(...)` 式辅助收敛；`init_schema` 按表拆函数；其余按逻辑块拆分。

### P015（P2）LLM 公网明文 HTTP

- **位置**：`tauri/src-tauri/src/commands/llm/request.rs:214-245`
- **证据**：`validate_llm_base_url` 只要求 scheme ∈ {http, https}，允许公网 `http://`；LLM 请求携带 `Authorization: Bearer <api_key>` 及私密聊天内容。OCR 侧 `validate_model_base_url` 明确拒绝非回环 http（ocr.rs:1405-1409），两侧策略不一致。
- **建议**：对齐 OCR 策略，仅允许回环地址使用 http。

### P016（P2）流式聊天缺 SSRF 主机复核

- **位置**：`tauri/src-tauri/src/commands/llm/stream.rs:388`
- **证据**：流式聊天只调 `validate_llm_base_url`，未调 `ensure_public_llm_host`；非流式 `chat_http.rs:13-15,43-45` 两者都调。已登记 provider 若用主机名（如 `nas.lan`），流式路径可命中内网解析结果。另 `ensure_public_llm_host` 解析一次后 reqwest 连接时再次解析，存在 DNS rebinding TOCTOU 窗口。
- **建议**：流式路径补齐 `ensure_public_llm_host`；TOCTOU 可考虑连接时固定解析结果（reqwest `resolve`）。

### P017（P2）CSP `object-src data:`

- **位置**：`tauri/src-tauri/tauri.conf.json:30`
- **证据**：CSP 允许 `object-src data:`，可经 `<object>/<embed>` 加载 `data:text/html` 在独立源执行脚本。`style-src 'unsafe-inline'` 为 React 内联样式所需可接受。
- **建议**：改为 `object-src 'none'`，改动后需验证 PDF/图片预览等依赖 `<object>` 的功能。

### P018（P2）`path_within_base` 兜底分支放行 `..`

- **位置**：`tauri/src-tauri/src/commands/attachment.rs:96-100`
- **证据**：canonicalize 失败（Android symlink 兜底）时退化为 `raw.starts_with(base_canon)`，朴素组件前缀匹配不解析 `..`。触发条件苛刻，但纵深防御上仍放行逃逸。
- **建议**：兜底分支显式拒绝含 `ParentDir` 组件的 raw 路径。

### P019（P2）附件导入插件调试日志残留

- **位置**：`tauri/src-tauri/src/attachment_import_plugin.rs:477-482,507-512`
- **证据**：生产代码遗留 `tracing::error!("attachment_export_content_uri debug: src_path=...")`，以 error 级输出用户文件完整路径。
- **建议**：删除或降级为 `debug!` 并脱敏。

### P020（P2）`object_list` 全量解密全量传输

- **位置**：`tauri/crates/solosoul-vault/src/storage/objects.rs:329-485`、`tauri/src-tauri/src/commands/object/mod.rs:459-488`
- **证据**：`list_objects` 对每个对象解密完整 properties/property_labels 放入 `ObjectSummary.properties` 整体返回；注释声称 "First few property key-value pairs for card previews"（lib.rs:347）但实际未截断。前端工作区无 section 过滤时拉取全账户。
- **建议**：后端对 summary properties 做预览截断（前 N 字段/每值限长），或提供 SQL 级分页；计数类调用方改用已有的 `list_object_metadata`（P111）。

### P021（P2）搜索热循环临时分配

- **位置**：`tauri/src-tauri/src/commands/search/query.rs:28,40-44,60,81,104-105`
- **证据**：对每个对象每个 key 做 `key.to_lowercase()` 分配、`format!("{}.{}", ...)` 构造 field_path（无命中也分配）、每个字符串值 `s.to_lowercase()` 完整复制；`field_matches` 整体 `sort_by` 仅取最大值。
- **建议**：field_path 用可复用 String 缓冲 push/pop；`max_by` 替代全排序。

### P022（P2）RAG 每次检索重读全部指南

- **位置**：`tauri/src-tauri/src/commands/llm/rag.rs:395-401`
- **证据**：`llm_search_guide_chunks` 每次调用都 `chunk_all_guides(&language)` 仅为构建 guide_id→title 映射，内部 `read_to_string` 遍历所有指南文件并完整跑 markdown chunking。
- **建议**：title 映射用 `OnceLock`/按 language 缓存，或随 embeddings 持久化。

### P023（P2）模板命令每次执行无效迁移检查

- **位置**：`tauri/src-tauri/src/commands/template.rs:22-33,121-144`
- **证据**：`migrate_legacy_templates_if_needed` 在每个模板命令入口调用；迁移完成后仍每次 `load_profile` + `serde_json::from_slice` 全量解析。
- **建议**：迁移成功后写 sys_config/metadata 标记，后续 O(1) 短路；或进程内缓存已迁移状态。

### P024（P2）回收站批量永久删除逐条 IPC

- **位置**：`tauri/src/stores/trashStore.ts:137-155`
- **证据**：`permanentDelete` 对每个 trashId 单独 `invoke('trash_permanent_delete')`，仅并发 8 限流；附件模块已有 `attachment_batch_delete` 批量端点先例。
- **建议**：新增 `trash_permanent_delete_batch` 端点（单事务循环删除），前端改单次调用。

### P025（P2）`list_object_attachment_ids` 全解密取 id

- **位置**：`tauri/crates/solosoul-vault/src/storage/objects.rs:282-327`
- **证据**：`SELECT id, properties FROM objects WHERE ...` 后逐行 AES 解密 + JSON 全解析，仅为提取 `__attachments` id 列表；调用方 `get_vault_stats`。
- **建议**：短期可在解密文本上子串扫描提取 `__attachments` 段；长期把附件清单拆出独立存储。

### P026（P2）聊天消息列表无分页/虚拟化

- **位置**：`tauri/src/components/llm/ChatMessageList.tsx:178`
- **证据**：`messages.map` 全量挂载所有气泡（bubble 已 memo），数百条含 markdown 消息全部在 DOM；项目其它列表（对象/日志/快照/附件）均已分页。
- **建议**：加「加载更早消息」分页或窗口化渲染。

### P027（P2）CLI `/debug_log` 明文权限未收紧

- **位置**：`solosoul_cli/src/commands/settings.rs:342-397`
- **证据**：解密后的审计日志（含操作对象名、账户 ID、数据目录路径）用 `std::fs::write` 默认权限写入 `logs/`，未做 0600 收紧（对比 `log.rs:65-68` 的显式 chmod，两处行为不一致）。
- **建议**：与 P007/P028 统一用 `OpenOptions::mode(0o600)`。

### P028（P2）CLI `/export_log` 权限窗口期

- **位置**：`solosoul_cli/src/commands/log.rs:56-69`
- **证据**：先以默认权限创建文件、写完前才 `set_permissions(0o600)`（明文窗口期），且 `let _ =` 吞掉 chmod 失败。
- **建议**：`OpenOptions::mode(0o600)` 在创建时定权限。

### P029（P2）CLI 锁定后内存残留解密数据

- **位置**：`solosoul_cli/src/app.rs:776-791`、`commands/auth.rs:64-72`
- **证据**：自动锁定与 `/lock` 只清理 `password_input` 并切 phase，不清理 `previous_phase`（可能持有解密后 ObjectRecord）、`chat_state`（LLM 对话明文）与 `prompt`（可能持有 mask 输入）。
- **建议**：锁定时统一重置上述字段，与「锁定擦除敏感状态」约定对齐。

### P030（P2）CLI 消息通道误用

- **位置**：`solosoul_cli/src/app.rs:813-818`、`commands/llm.rs:35`
- **证据**：插件运行**成功**结果与 `/model` 信息性输出通过 `self.error_message` 展示（渲染为红色「! 错误」overlay）；已有 `info_message`（app.rs:388）正是为此设计。
- **建议**：改用 `info_message`。

### P031（P2）CLI `do_rollback` 未校验快照归属

- **位置**：`solosoul_cli/src/commands/history.rs:88-123`
- **证据**：未校验 `snapshot_id` 属于 `object_id`，也未校验目标对象归属当前账户；`get_snapshot` 取到哪个快照就应用哪个。另 `history.rs:126-132` 用 `unwrap_or_default()` + `let _ = save_snapshot`，失败静默留下空快照。
- **建议**：回滚前校验快照 object_id 与账户归属；序列化/保存失败应报错。

### P032（P2）CLI `update_profile_preference` 静默丢失

- **位置**：`solosoul_cli/src/commands/mod.rs:61-68`
- **证据**：profile 数据根不是 JSON object 时 `if let Some(obj)` 静默跳过，函数仍 `save_profile` 并返回 Ok——偏好写入丢失，调用方误报成功。
- **建议**：根非 object 时返回 Err 或初始化为 object。

### P033（P2）CLI 备份静默跳过失败 profile

- **位置**：`solosoul_cli/src/commands/backup.rs:222-233`
- **证据**：`if let Ok(Some(profile)) = vault.load_profile(...)`，单个 profile 加载/解密失败被静默跳过，备份可能不完整且无警告。
- **建议**：失败时中止并报告，或在备份概要中明确标注缺失项。

### P034（P2）CLI 搜索父页名 N+1

- **位置**：`solosoul_cli/src/commands/search.rs:283-287`
- **证据**：每条搜索结果都单独 `vault.load_object(parent_id)`（DB 查询 + 解密）解析父页面名，单次搜索最多 200 次额外解密查询。
- **建议**：批量预取去重后的 parent_id 集合。

### P035（P2）CLI `plugin_id` 未校验字符

- **位置**：`solosoul_cli/src/commands/plugin.rs:102-106`
- **证据**：`plugin_id` 未做字符校验直接 `market_dir.join("plugins").join(&plugin_id)`，`../`、绝对路径不做拒绝。当前仅用于存在性检查，影响有限。
- **建议**：按插件 ID 白名单字符（如 `[a-z0-9_.-]`）校验；正向例子见 `backup.rs:80-96` `sanitize_backup_name`。

### P036（P2）CLI 硬编码中文绕过 i18n

- **位置**：`solosoul_cli/src/app.rs:609-636`（`prompt_verify_password_for_field`）、`app.rs:1760`、`app.rs:789` 等
- **证据**：硬编码中文字符串绕过全项目已铺开的 `t!()` i18n 体系。
- **建议**：补 FTL 词条，改用 `t!()`。

### P037（P2）CLI 超长函数

- **位置**：`solosoul_cli/src/app.rs:2694-3016` `render()`（323 行）；`handle_onboarding_key`（150 行）、`handle_new_object_key`（133 行）、`handle_plugin_list_key`（131 行）、`handle_command_key`（116 行）
- **建议**：多为巨型 match，可按 phase 拆函数；优先级低。

### P038（P2）CLI 陈旧注释

- **位置**：`solosoul_cli/src/commands/settings.rs:340-341`（文档注释混入字面 `\n`）、`settings.rs:6` 与 `app.rs:364,412`（引用不存在的 `trigger_debug_log_export`）、`commands/profile.rs:171-180`（`parts.is_empty()` 死分支，注释理由不准确）
- **建议**：修正注释；`parts.is_empty()` 分支可删。

### P039（P2）前端 `Profile` 死接口

- **位置**：`tauri/src/lib/ipc.ts:86-93`
- **证据**：全仓 grep 无 import；且 `createdAt/updatedAt: string` 与 Rust `profile.rs:11-19` 的 `created_at: DateTime<Utc>`（snake_case、无 rename）对不上，即便被引用也是错误形状。
- **建议**：删除，或由后端真实返回结构重新生成。

### P040（P2）`parse_hlc_json` 静默吞畸形数据

- **位置**：`tauri/src-tauri/src/commands/sync.rs:289-294`
- **证据**：`.as_u64().unwrap_or(0)` / `.as_str().unwrap_or("")` 静默吞掉畸形 HLC，冲突记录 JSON 损坏时得到 `wall_time_ms=0` 的假 HLC 并照常展示。
- **建议**：解析失败返回 Err 或至少记 warn。

### P041（P2）`remove_with_retry` 的 unwrap

- **位置**：`tauri/src-tauri/src/commands/settings.rs:74,81`
- **证据**：`last_err.as_ref().unwrap()` / `last_err.unwrap()` 逻辑上可证安全但依赖读者推理。
- **建议**：改为直接存值、循环后 `match`，消掉 unwrap。

### P042（P2）`type_id` 三套命名

- **位置**：`tauri/crates/solosoul-vault/src/lib.rs:255`（serde `rename = "typeId"`）、`tauri/src-tauri/src/commands/object/mod.rs:119`（DTO `rename = "collectionType"`）、`tauri/src/stores/objectStore.ts:20`（`collectionType`）、`conflictFieldMeta.ts`（又需认识 `typeId`）
- **证据**：同步冲突的 `local_data`/`remote_data` 透传原始 `ObjectRecord` JSON，同一份数据在不同 IPC 通道暴露不同字段名。
- **建议**：统一序列化名，或给冲突载荷也过 DTO 映射层。注意与 P001 同属同步冲突载荷问题，修复时可同轮次处理。

### P043（P2）深层嵌套

- **位置**：`tauri/src-tauri/src/sync/auto_sync_core.rs:82-130`（真实控制流 8-9 层）、`tauri/src/components/plugin/PluginQuickPanel.tsx:244-270`（JSX 回调内嵌套 8 层）
- **建议**：提前 return / 拆 `run_immediate()`、`run_periodic()`；JSX 回调逻辑移出。

### P044（P2）水印插件校验逻辑重复 + 插件 ID 硬编码

- **位置**：`tauri/src/components/plugin/PluginQuickPanel.tsx:249-268`、`tauri/src/pages/ai/PluginDashboardPage.tsx:~170-190`
- **证据**：同一段 `JSON.parse(savedParams.selectedAttachments)` + `Array.isArray` 检查 + toast，写死插件 ID `com.solosoul.official.watermark` 于两个通用 UI 入口。
- **建议**：下沉到插件参数 schema/运行时校验层。

### P045（P2）遗留 TODO 注释

- **位置**：`tauri/src/hooks/useRevealState.ts:90`
- **证据**：全仓唯一实质 TODO——「字段类型感知的部分掩码未实现（如银行卡只显后 4 位）」，是有意保留的产品决策标记，但与 AGENTS.md「禁止自行实现掩码逻辑」约定存在张力。
- **建议**：转为 issue 跟踪，代码中移除 TODO 或改写为指向 issue 的说明。

---

## 核查后确认无问题的重点面（免责说明）

- **硬编码密钥**：未发现；命中均为测试夹具与 minisign 公钥嵌入。
- **加密用法**：AES-256-GCM 全部随机 nonce，chunked 格式 v2 头部纳入 AAD；Argon2id 参数分级合理；无 ECB/固定 nonce。
- **zeroize**：密码与派生密钥在 IPC 边界即 Zeroizing 包装，lock 时显式擦除。
- **XSS**：无 `dangerouslySetInnerHTML`/`rehype-raw`；Markdown 链接有协议白名单。
- **命令注入**：仅固定参数的 `swiftc`/`icacls`，arg 数组传参，无 shell 拼接。
- **Zip slip**：导入 ZIP 用生成新 ID 落盘 + 拒绝 `/`、`\`；embed 模型解压用 `mangled_name`。
- **unsafe**：集中于平台 FFI，带 SAFETY 注释，模式正确。
- **网络层**：无禁用 TLS 校验；updater/GitHub API 均 https；Embedding/OCR 注册表有 minisign + sha256 校验。
- **crate 依赖**：无循环依赖（干净的 DAG）。
- **IPC 错误处理**：前端 `ipcClient.ts` 统一收口；commands 侧 `Result<T, String>` 模式一致，command 路径无会 panic 的 unwrap。
- **unwrap/expect**：Rust 生产代码纪律良好，绝大多数命中在 `#[cfg(test)]`。
- **数据库索引**：常用过滤列均有索引，未发现缺索引。
- **CLI 密码处理**：PasswordInput/prompt/security 链路均已正确 zeroize；日志有 crate 白名单过滤，无密码/session key 泄露。
