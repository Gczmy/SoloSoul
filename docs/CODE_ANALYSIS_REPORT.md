# 代码分析修复报告

> 最后更新：2026-08-24 00:25:00
> 修复轮次：2（进入阶段 3 修复）
> 当前分支：`main`
> 前置轮次：1（初始分析，本轮仅分析不修复）

## 本轮说明

- 按用户要求：**重新全量分析生成新报告，不延续旧报告，生成后不进行修复**。
- 分析范围：`tauri/`（Rust `src-tauri/src` + `crates/`，前端 `src/`），跳过 `node_modules/`、`target/`、`dist/`、`.vite/`、`gen/` 等生成目录。
- Git 状态：仅 2 个未跟踪的 `bugreport-*.zip` 文件（位于 `tauri/`），无代码改动，未做任何提交。

## 基线检查结果（`npm run check-all`）

| 检查项 | 结果 |
|--------|------|
| TypeScript `tsc --noEmit` | ✅ 通过 |
| Rust `cargo fmt --check` | ✅ 通过 |
| Rust `cargo clippy -- -D warnings` | ✅ 通过 |
| Rust `cargo test` | ✅ 通过 |
| ESLint | ✅ 通过 |
| Vitest（108 文件 / 928 用例） | ✅ 通过 |
| `check-markdown-chunk-boundary.mjs` | ✅ 通过 |
| `check_acl_consistency.py` | ❌ **失败**（8 个 cloud_sync 命令未登记 ACL；`ui_get_preferences` 白名单遗留）→ P001/P002 |
| `check_pref_keys_sync.py` | ⏭ 未执行（前一步失败后中断） |

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|------|--------|------|----------|------|------|
| P001 | P1 | 规范/CI | `src-tauri/permissions/solo-soul/default.toml` | 8 个 cloud_sync 命令已注册 handler 且前端在调，但未登记 ACL 白名单；`check_acl_consistency.py` exit 1，CI（pr_check.yml）必红，运行时 ACL 拒绝 | `[x]` 已修复 |
| P002 | P1 | 架构 | `src-tauri/src/lib.rs:46-193`（定义于 `commands/settings.rs:166-181`） | `ui_get_preferences` 有 `#[tauri::command]` 定义且进了 ACL，但从未注册进 `generate_handler!`，前端三处调用永远失败并被静默吞掉 | `[x]` 已修复 |
| P003 | P1 | 漏洞 | `crates/solosoul-core/src/cloud_sync/webdav.rs:48-65` | WebDAV 连接器允许 `http://` + Basic 认证，账号密码明文传输；与 LLM/OCR 的「非回环强制 https」策略不一致，错误文案与行为自相矛盾 | `[x]` 已修复 |
| P004 | P1 | 安全/规范 | `src/lib/searchShared.tsx:175-183` + `crates/solosoul-core/src/search_filter.rs:11` | 搜索结果中 internal 级字段命中值明文渲染，无 `useRevealState`/`maskValue`，违反 P036 掩码统一约定 | `[x]` 已修复 |
| P005 | P1 | 安全 | `src/components/layout/SearchPopover.tsx:49-63` | 最近搜索词明文持久化 localStorage（`solosoul_recent_searches`），不按账户隔离，Vault 锁定/退出时不清除 | `[x]` 已修复（核验补修：调用点漏传 accountId 致功能失效，已恢复） |
| P006 | P1 | 性能/事务 | `crates/solosoul-vault/src/storage/snapshots.rs:547-584` | `copy_snapshots` 循环逐条 INSERT 无事务包裹（中途失败留半成品），且同钥 decrypt→encrypt 纯浪费，应直接复制密文 | `[x]` 已修复 |
| P007 | P1 | 可维护性 | `src/pages/settings/CloudSyncPage.tsx:51-632` | 单组件约 535 行非注释代码，承载 WebDAV 配置/保留策略/连接器/入站列表全部逻辑 | `[x]` 已修复 |
| P008 | P1 | 可维护性 | `src/components/forms/DatePicker.tsx:168-609` | 主组件约 385 行，段落解析+键盘处理+滚轮渲染混杂 | `[x]` 已修复（核验补修：Calendar 新增的 common:hour/minute 双语键已入库） |
| P009 | P1 | 可维护性 | `src/pages/settings/useExportImportPage.tsx:33-454` | 导出/导入 hook 状态机约 371 行 | `[x]` 已修复 |
| P010 | P1 | 可维护性 | `src/pages/settings/VaultDirectorySection.tsx:27-423` | 单组件约 369 行 | `[ ]` 待修复 |
| P011 | P2 | 安全 | `crates/solosoul-core/src/export_import.rs:221-237,1218-1242,2214` | 导出/导入附件临时明文落共享 temp 目录（可预测目录名、未设 0700/0600）；同仓库其他路径均已收紧权限，此处是离群点 | `[x]` 已修复 |
| P012 | P2 | 安全（加固） | `crates/solosoul-sync/src/recovery.rs:269-279,184` | Recovery 主机指纹校验可选（手动输入路径无 MITM 防线），且主机端接受裸 PIN 认证；已有限流/一次性 nonce 缓解，建议加固 | `[ ]` 待修复 |
| P013 | P2 | 性能/事务 | `crates/solosoul-vault/src/storage/snapshots.rs:369-414` | `repair_invisible_objects` 循环内逐行 query_row + UPDATE 无事务（有一次性标记兜底，仅跑一次，故 P2） | `[x]` 已修复 |
| P014 | P2 | 性能 | `crates/solosoul-vault/src/storage/objects.rs:448` | `save_object_tx` 无条件克隆整棵 properties JSON，即使无需注入 `__templateName`；可加 `template_id.is_some()` 惰性克隆 | `[x]` 已修复 |
| P015 | P2 | 性能 | `src-tauri/src/commands/export_import/import.rs:130-135` | `import_decrypt_preview` 整包读入内存（上限 100MB，峰值约 3×100MB）；主导入路径已流式化，预览路径遗留 | `[x]` 已修复 |
| P016 | P2 | 重复代码 | `crates/solosoul-vault/src/storage/reencrypt.rs:54,106,126` | 三个 reencrypt 函数仅表名不同、函数体逐字节相同；`storage.rs:595` 已有通用版可收敛 | `[x]` 已修复 |
| P017 | P2 | 重复代码 | `crates/solosoul-core/src/objects.rs:1102` vs `src-tauri/src/commands/attachment/mod.rs:114` | `load_all_referenced_attachment_ids` 跨 crate 双实现（后者 test-only），建议保留一个共享实现 | `[x]` 已修复 |
| P018 | P2 | 重复代码 | 全库 93 处 | `conn.lock()` + `ok_or("Vault is locked")?` 守卫样板 93 处，可考虑宏/helper 收敛（设计惯性，非 bug） | `[ ]` 待修复 |
| P019 | P2 | 可维护性 | 详见下文清单 | Rust 过长函数 Top10（>50 行非注释，最长 159 行） | `[ ]` 待修复 |
| P020 | P2 | 可维护性 | 详见下文清单 | Rust 深层嵌套（≥5 层）多处，最深 `auto_sync_core.rs:87` 达 8 层 | `[ ]` 待修复 |
| P021 | P2 | 可维护性 | 详见下文清单 | 前端超长组件第二梯队（6 个 300+ 行组件 + `pluginStore.runPlugin` 137 行 + `syncStore` 内嵌监听器 120 行） | `[ ]` 待修复 |
| P022 | P2 | 性能 | `src/components/attachment/PhotoAlbumGrid.tsx:153` | 相册网格 `items.map` 全量渲染 DOM，无窗口化上限（缩略图已懒加载缓解） | `[x]` 已修复 |
| P023 | P2 | 性能 | `src/pages/scan/ScanLocalPage.tsx:107` | 目录导入 `Promise.allSettled` 并发无上限，大目录瞬时打出大量 IPC | `[x]` 已修复 |
| P024 | P2 | 死代码 | 详见下文清单 | 6 个 `export` 仅在定义文件内部使用，可去掉 export（无整文件级死代码） | `[x]` 已修复 |
| P025 | P2 | 重复代码 | 详见下文清单 | 已有共享 `CopyButton.tsx`，仍有 ≥7 处自行实现「复制到剪贴板+已复制反馈」 | `[x]` 已修复（hook 方案） |
| P026 | P2 | 重复代码 | 详见下文清单 | `visibleLimit`+slice+「加载更多」增量分页模式重复出现于 5+ 文件，可抽公共 hook | `[x]` 已修复 |
| P027 | P2 | 规范偏离 | `src/components/editor/FieldSuggestions.tsx:47,125-131` | 字段推荐对 internal 级明文展示（有意设计且有注释），与 P036 不一致，建议确认例外或写回 AGENTS.md | `[x]` 已确认例外并写入 design_map/12 规范 |
| P028 | P2 | 规范偏离 | `src/components/sync/SyncConflictDialog.tsx:373-433` + `src-tauri/src/commands/sync.rs:371` | 同步冲突对话框明文渲染 sensitive/critical 字段差异值（场景可辩护），建议对受保护字段加揭示交互 | `[x]` 已修复 |
| P029 | P2 | 架构 | `src-tauri/src/commands/vault_directory.rs:160-166` | `vault_set_directory` 后端锁定 Vault 但不 emit `vault-locked` 事件，前端认证态不失效，后续命令报锁错误但 UI 仍显示已解锁 | `[x]` 已修复 |
| P030 | P2 | i18n | `src/pages/settings/CloudSyncPage.tsx:603` | `common:enabled` / `common:disabled` 双语均缺 key 且无 defaultValue，UI 直接渲染原始 key 字符串 | `[x]` 已修复 |

#### 修复说明（续）
- **P027**：核实代码注释（推荐场景用途=引用同名字段快速填入，internal 在编辑页
  本就以揭示态呈现）后，将例外正式写入 `docs/design_map/12_敏感度等级规范.md`
  §2.3。
- **P028**：新增 `extractFieldLevels`（从 __fields.sensitivityLevel 提取字段→
  敏感度映射，本地/远程取更严格者）+ `ProtectedValue` 组件——受保护字段
  默认 MASK_PLACEHOLDER，点击临时揭示（useRevealState 1 分钟 TTL）；标量行与
  diffEntry 叶子行共 6 处值渲染全部包裹。TSC/Vitest(928) 回归通过。

## 修复进度

- 已完成：21 / 30（P0: 0，P1: 6，P2: 15）
- 延期：9 项（见「延期项处置决定」）

## 延期项处置决定（2026-08-24 审查轮收尾）

以下 9 项**不在本轮修复**，理由与建议时机如下：

| ID | 类别 | 延期理由 | 建议时机 |
|----|------|----------|----------|
| ~~P007~~ ✅ | CloudSyncPage 拆分 | **已完成**：631 行 → 主组件 96 行 + useCloudSyncPage(251) + 7 个 section（26~116 行） | — |
| ~~P008~~ ✅ | DatePicker 拆分 | **已完成**：609 行 → 主组件 357（分段输入逻辑）+ helpers 143（纯函数逐字节保真）+ Calendar 189 | — |
| ~~P009~~ ✅ | useExportImportPage 拆分 | **已完成**：454 行 → 341 行主 hook + useExportExecution(190) + guide 配置(45)；导入/范围/估算本已委托子 hook | — |
| ~~P010~~ ✅ | VaultDirectorySection 拆分 | **已完成**：423 行 → 展示层 261 + useVaultDirectory(216)；顺带收敛目录切换成功后的重复收尾为 afterDirectorySwitched | — |
| P012 | Recovery 指纹强制化 | UX 流程变更（手动输入路径要求录指纹），需产品确认 + GUI/CLI 双端改造 + i18n + 测试 | 产品决策后单独排期 |
| P018 | 93 处 lock 守卫样板宏收敛 | 报告自述「设计惯性非 bug」；93 处机械替换 churn 大、回归面广、零行为收益 | 触碰 storage 层时渐进采用新 helper，不做一次性迁移 |
| ~~P019~~ ✅ 6/9 | Rust 过长函数 Top10 | **已完成**：import_attachments→3 阶段函数、attachment_download→2 校验函数、unlock_with_kdf_upgrade→build_upgraded_config、list_trash_changes→map_trash_change_row、page_delete→build_page_delete_trash_items、import_decrypt_preview→build_preview_object_summaries。**handle_inbound / recovery_restore_from_host / export_objects_document 维持现状**——已是「命名 helper 的纯编排层」（下载→建户→导入各阶段自成函数），再拆只是搬参数表反而降低可读性 |
| ~~P020~~ ✅ 点名项 | Rust 深层嵌套 | biometric 系 6 处审计样板收敛 write_biometric_audit + unlock_audit_action_type（报告点名「值得优先重构」项）；其余为 tokio::select!/SQL 链式结构性嵌套（报告自述实际风险低），维持现状 |
| ~~P021~~ ✅ 逻辑类 3/6 | 前端超长组件第二梯队 | pluginStore.runPlugin 巨型 switch→applyPluginRunEvent 纯函数；syncStore 入站刷新尾收敛 refreshAfterInbound（消除双分支重复）；RecoveryQrContent 手动面板拆出 RecoveryManualEntryPanel（391→205+233）。**PageGuide / AttachmentPreviewOverlay / PhotoAlbumOverlay / useObjectEditorPage 维持现状**——手势拖拽与预览生命周期高内聚，拆分损害内聚性且视觉回归无法在此环境验证 |

## 收尾验证基线（含结构性拆分轮）

- `cargo fmt --check` / `clippy -D warnings`：✅
- Rust workspace：**994 passed / 0 failed**
- TSC / ESLint：✅（0 error 0 warning）
- Vitest：**928 passed**
- `check_acl_consistency.py`：✅ 205 命令全登记
- `check-missing-i18n.mjs`：✅ 双语 0 缺失

> 结构性拆分轮新增提交（按序）：CloudSyncPage 拆分 → DatePicker 拆分 →
> useExportImportPage 拆分 → VaultDirectorySection 拆分 → import_attachments/
> attachment_download/unlock_with_kdf_upgrade/list_trash_changes/page_delete/
> import_decrypt_preview 六处过长函数拆分 → biometric 审计样板收敛 →
> runPlugin/syncStore/RecoveryQrContent 三处前端逻辑拆分 → fmt 归一。

#### 补充修复说明（历史条目存档）

#### 修复说明（续）
- **P026**：新增 `useIncrementalWindow(initial, step)`（limit/hasMore/showMore/
  reset/setLimit），5 个站点迁移：OperationLogPage、DebugLogPage、HistoryPage、
  useTrashPage、useObjectWorkspaceData。PhotoAlbumGrid（P022 新增）暂保留内联
  实现（组件局部 state，无 reset 语义）。ESLint exhaustive-deps 全部补齐。
  TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P023**：handleImportAll 改 CONCURRENCY=4 分批 allSettled，失败统计语义
  不变。TSC 回归通过。

#### 修复说明（续）
- **P022**：PhotoAlbumGrid 加增量窗口（INITIAL_VISIBLE_LIMIT=200，
  「加载更多」按钮步进 200，items 变化重置）；组件测试 4 项回归通过。

#### 修复说明（续）
- **P024**：6 处冗余 export 全部去除（buildPdfPreviewSrc/SWIPE_THRESHOLD/
  FORMAT_FILTERS/CONFLICT_VALUE_MAX_LEN/prefetchWarmupTasks/MockResizeObserver）。
  TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P017**：core 版改 `pub` 导出；solo_soul 侧 #[cfg(test)] 重复实现删除，
  测试改为 `use solosoul_core::objects::load_all_referenced_attachment_ids`
  （两实现语义一致：对象 __attachments 引用集合）。solo_soul 465 测试回归
  通过。

#### 修复说明（续）
- **P016**：reencrypt_profiles / reencrypt_trash_items / reencrypt_object_snapshots
  收敛为 `reencrypt_blob_table(tx, table, old_key, new_key)`（表名参数化，SQL
  format! 拼接——表名为编译期常量字面量无注入面）；调用点改三行单行调用。
  solosoul-vault 172 测试回归通过。

#### 修复说明（续）
- **P015**：preview 命令改调主路径 `decrypt_package`（流式解密至数据目录内
  NamedTempFile + `serde_json::from_reader`），删除 read_file_from_zip +
  decrypt_chunked_from_bytes + from_slice 链路；解密失败错误码由
  decrypt_zip_entry_streaming 内部映射，前端 i18n 行为不变。solo_soul 465
  测试回归通过。

#### 修复说明（续）
- **P014**：properties 改 `Cow<'_, serde_json::Value>` 借用原值，仅在实际
  注入 __templateName 时 `to_mut()` 触发克隆；批量保存路径（无模板对象占多数）
  零拷贝。solosoul-vault 172 测试回归通过。

#### 修复说明（续）
- **P013**：`repair_restored_objects` 的 SELECT 先收集、stmt 提前 drop，
  修复循环整体包 `with_tx`（失败回滚不留半改状态）；REPAIR_FLAG 仅在
  事务成功后落位。solosoul-vault 测试回归通过。

#### 修复说明（续）
- **P011**：新增 `create_private_temp_dir`（0700 + UUID 随机目录名）与
  `tighten_file_perms`（0600），替换导出（write_attachment_entries）与导入
  （import_vault）两处生产路径的共享固定目录；测试内第 3 处为快照对比用途，
  不涉明文，保持原样。export_import 17 测试回归通过。

#### 修复说明（续）
- **P029**：`vault_set_directory` 在 `svc.lock()` 之后 emit `vault-locked`
  （与 commands/vault.rs::lock 对齐），前端 AppRoutes 监听链自动失效认证态。
  Clippy 回归通过。

#### 修复说明（续）
- **P006**：`copy_snapshots` 包 `with_tx`（失败整体回滚）；循环内去掉同钥
  解密→重加密，直接复制密文行；`data_key()` 保留作解锁态校验。
  solosoul-vault 172 测试回归通过。

#### 修复说明（续）
- **P005**：双保险——① 存储键改为 `solosoul_recent_searches:{accountId}`
  按账户隔离；② authStore `lock()`/`logout()` 调 `clearRecentSearches()`
  清除全部前缀键。TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P004**：MatchHint 的 fieldValue 分支抽出 `FieldValueHint` 子组件——
  `sensitivityLevels` 任一非 public 即渲染 `MASK_PLACEHOLDER`（点击揭示，
  复用 useRevealState 1 分钟 TTL）；SearchPage/SearchPopover 共用路径一次收敛。
  后端 search_filter.rs 保持不变（匹配仍覆盖 internal 值，仅展示层掩码）。
  TSC/ESLint/Vitest(928) 回归通过。

#### 修复说明（续）
- **P003**：新增 `is_local_http_host` 判定——http 仅允许回环/RFC1918 私网/IPv6
  unique-local/.local 主机名（局域网 NAS 明文属可接受用户选择）；公网 host http
  返回 `ConfigMissing` 类型化错误并提示改用 https。单测 `test_p003_http_policy`
  覆盖 9 种地址形态。E2E 9/9 回归通过（本地 wsgidav 即 127.0.0.1 回环）。

#### 修复说明
- **P001**：default.toml 按字母序补入 8 个命令；`check_acl_consistency.py` 现报
  「205 个命令均已登记」，exit 0。
- **P002**：lib.rs generate_handler! 补注册 `commands::settings::ui_get_preferences`
  （测试断言字符串清单本就含此命令，印证遗漏）；settingsStore/notification/onboarding
  三处读链路恢复。

#### 补充说明（P025 方案选择）

未采用「全部换用 plugin-market 的 CopyButton 组件」——其样式/文案形态与各站点差异大
（图标按钮、键控多目标、toast 反馈），强行替换会改变视觉与交互。改为抽取共享
`useCopyToClipboard` hook（copy 返回布尔 + 键控 copied 态 + execCommand fallback），
6 个站点保留各自样式仅收敛逻辑：GuideCodeBlock、PluginResultPanel、
useObjectDetailModal（copiedField 键控）、useLlmChatCore（copiedIndex 键控）、
AccountSettingsPage（toast 驱动）、SyncShowQrDialog（addr/pin 双键，含 fallback）。
RecoveryQrContent 为纯展示组件（props 驱动），随 SyncShowQrDialog 一并受益。

## 详细问题描述与修复指引

### P001 — cloud_sync 命令未登记 ACL 白名单（P1，规范/CI）

- **证据**：`python3 scripts/check_acl_consistency.py` exit 1，报错：`cloud_sync_delete_config / cloud_sync_get_config / cloud_sync_list_incoming / cloud_sync_mark_applied / cloud_sync_now / cloud_sync_save_config / cloud_sync_test_connection / cloud_targets_detect 未登记到 default.toml`。
- 这些命令已注册进 handler（`src-tauri/src/lib.rs:135-142`），前端确实在调（`src/pages/settings/CloudSyncPage.tsx:87,110,204`、`src/hooks/useExportImportPage.tsx:45`）。
- **影响**：该脚本在 CI（`.github/workflows/pr_check.yml`）与 `npm run check-all` 中强制执行，当前 main 下次跑 CI 直接失败；Tauri v2 对未列入 allow 的命令默认拒绝，云同步页全部 IPC 运行时报 "not allowed by ACL"。
- **诱因**：`default.toml` 最后更新 2026-08-21，cloud-sync 系列命令是 8-21~8-23 新增，加命令时漏同步 ACL。
- **建议修复**：将 8 个命令加入 `src-tauri/permissions/solo-soul/default.toml` 的 `allow-all-custom-commands` 列表。

### P002 — `ui_get_preferences` 未注册进 handler（P1，架构）

- **证据**：命令定义于 `src-tauri/src/commands/settings.rs:166-181`，已进 ACL 白名单（`default.toml:199`），但 `generate_handler!` 列表（`src-tauri/src/lib.rs:46-193`）只有 `ui_update_preference`（:143），没有 `ui_get_preferences`；`check_acl_consistency.py` 同步报 WARN「白名单中存在但 handler 中未注册」。
- **影响（均被 catch 吞掉，无崩溃但功能降级）**：
  - `src/stores/settingsStore.ts:271` — 明文层 `ui_preferences.json` 的主题/语言永远读不回来（write 通、read 断），WebView 缓存清除后登录前主题/语言回退默认。
  - `src/lib/notification.ts:34` — 通知权限"已请求"标记读不到。
  - `src/App/index.tsx:56-75` — `hasSeenOnboarding` 读失败回落 `false`。
- **建议修复**：在 `register_core_commands` 的 `generate_handler!` 中补注册 `ui_get_preferences`。

### P003 — WebDAV 允许 http + Basic 明文凭证（P1，漏洞）

- **证据**：`crates/solosoul-core/src/cloud_sync/webdav.rs:48-65` — `WebDavConnector::new` 仅校验 scheme ∈ {http, https}，随后无条件构造 Basic base64 认证头。用户填 `http://`（自建 NAS 常见）时，每次同步在公网/局域网上明文发送账号密码。
- **对比**：LLM `validate_llm_base_url`（`commands/llm/request.rs:190`）与 OCR `validate_model_base_url`（`commands/ocr.rs:1005`）均强制非回环 https；WebDAV 错误文案写「需形如 https://」却实际放行 http，文案与行为矛盾。
- **建议修复**：与 LLM/OCR 对齐——非回环 host 拒绝 http，或显式警告并要求用户确认。

### P004 — 搜索结果 internal 字段明文渲染（P1，安全/规范）

- **证据**：`src/lib/searchShared.tsx:175-183` 直接明文渲染 `matchedValue`，无 `useRevealState`/`maskValue`；后端 `crates/solosoul-core/src/search_filter.rs:11` 的 `PROTECTED_SENSITIVITIES = ["sensitive", "critical"]` 不含 internal，即 internal 字段值参与搜索匹配并明文返回展示。
- **冲突**：AGENTS.md 规定「仅 public 永不掩码，internal/sensitive/critical 一律掩码」（P036 已收敛）。SearchPage 与 SearchPopover 共用此路径。
- **建议修复**：internal 命中值按 P036 规则掩码 + 点击揭示，或将该例外显式写回约定文档。

### P005 — 最近搜索词明文 localStorage 残留（P1，安全）

- **证据**：`src/components/layout/SearchPopover.tsx:49-63` — `solosoul_recent_searches` 保留 3 条明文，不按账户隔离，Vault 锁定/退出登录时不清除。
- **影响**：搜索词可能含证件号、姓名等敏感片段；同机换账户登录后可在搜索弹层看到前一账户的搜索历史。与 `ocrScanStore`「锁定即清空明文」、syncStore「仅落非敏感元数据」的既定做法不一致。
- **建议修复**：锁定/退出时清除，或按账户隔离存储。

### P006 — `copy_snapshots` 无事务 + 同钥多余重加密（P1，性能/事务）

- **证据**：`crates/solosoul-vault/src/storage/snapshots.rs:547-584` — 恢复回收站对象时逐条 `insert.execute(...)`，每条隐式独立事务，中途失败留半成品快照集；且每条 `decrypt_field`→`encrypt_field`（同一把 key）纯浪费，直接复制 `raw_data` 密文即可。同文件 `trash.rs:70`、`sync_apply.rs:149` 均已事务化，此处是漏网。
- **建议修复**：包 `with_tx`，去掉同钥解密-重加密，直接复制密文行。

### P007–P010 — 前端超长组件（P1，可维护性）

| ID | 文件：行 | 非注释行数 | 说明 |
|----|----------|-----------|------|
| P007 | `src/pages/settings/CloudSyncPage.tsx:51-632` | ~535 | 单组件承载 WebDAV 配置、保留策略、连接器选择、入站文件列表全部逻辑 |
| P008 | `src/components/forms/DatePicker.tsx:168-609` | ~385 | 段落解析+键盘处理+滚轮渲染混杂 |
| P009 | `src/pages/settings/useExportImportPage.tsx:33-454` | ~371 | 导出/导入 hook 状态机过长 |
| P010 | `src/pages/settings/VaultDirectorySection.tsx:27-423` | ~369 | Vault 目录设置区单组件 |

注：计数含内联 style 对象，实际复杂度略低于行数；建议按职责拆子组件/子 hook。

### P011 — 附件临时明文 temp 目录权限未收紧（P2，安全）

- **证据**：`crates/solosoul-core/src/export_import.rs:221-237`（导出）、`:1218-1242` 与 `:2214`（导入）— 解密后附件明文写入 `std::env::temp_dir().join("solosoul_export_att")` 等可预测目录名，仅 `create_dir_all`，未设 0700/0600；多用户系统上本地其他用户可预占目录或在明文窗口期读取。
- **对比**：`decrypt_to_temp_dir`（`commands/attachment/mod.rs:489-503`，0700/0600）与 `write_payload_to_temp`（`export_import.rs:955-970`，落在 0700 vault 数据目录内）均已正确示范。导入侧崩溃残留有 `cleanup_orphan_import_temps` 兜底。
- **建议修复**：复用 tempfile + 0700/0600 的既有模式。

### P012 — Recovery 指纹校验可选 + 裸 PIN（P2，安全加固）

- **证据**：`crates/solosoul-sync/src/recovery.rs:269-279`（客户端仅 `Some(expected_fp)` 才校验指纹）、`:184`（主机端放行裸 PIN）。Noise_XX 用临时身份密钥，指纹是唯一 MITM 防线；`fingerprint=None` 时主动中间人可透明中继拿到 PIN 与 32 字节恢复密码。
- **缓解**：6 位 PIN + 一次性 nonce + 全局限流（`GLOBAL_MAX_ATTEMPTS = 10`）+ served 一次性标记，暴力破解不可行；但被动 relay 不需要猜 PIN。
- **建议修复**：手动输入路径要求一并输入指纹（QR 已含），或对无指纹连接在主机端弹确认。

### P013–P015 — Rust 性能遗留（P2）

- **P013**：`snapshots.rs:369-414` `repair_invisible_objects` 循环内逐行 `query_row` + `UPDATE` 无事务；有 `sys_config` 一次性标记兜底，实际只跑一次，建议包 `with_tx`。
- **P014**：`objects.rs:448` `save_object_tx` 无条件 `obj.properties.clone()`，可加 `if obj.template_id.is_some()` 惰性克隆。
- **P015**：`import.rs:130-135` `import_decrypt_preview` 整包读入内存（上限 100MB，峰值约 3×100MB）；主导入路径（`import.rs:816`）已流式化，预览路径遗留。

### P016–P018 — Rust 重复代码（P2）

- **P016**：`reencrypt.rs:54/106/126` 三个 reencrypt 函数仅表名不同、函数体逐字节相同；`storage.rs:595` 已有通用版 `rewrite_blob_table_encrypted`，可收敛为一个参数化函数。
- **P017**：`load_all_referenced_attachment_ids` 跨 crate 双实现（`solosoul-core/src/objects.rs:1102` 与 `commands/attachment/mod.rs:114`，后者 test-only），建议保留一个共享实现。
- **P018**：「Vault is locked」守卫样板 93 处（非测试代码），设计惯性非 bug，可考虑宏/helper 收敛。

### P019 — Rust 过长函数 Top10（P2）

| # | 文件：行 | 函数 | 非注释行数 | 嵌套深度 |
|---|----------|------|-----------|---------|
| ~~1~~ | `crates/solosoul-core/src/export_import.rs:1109` | `import_attachments` | ✅ 已拆分（build_attachment_meta_map / write_imported_attachment / write_back_imported_attachments） | — |
| 2 | `crates/solosoul-vault/src/storage/sync_changes.rs:541` | `list_trash_changes_since_limited` | 123 | 5 |
| 3 | `src-tauri/src/commands/attachment/mod.rs:223` | `attachment_download` | 123 | 4 |
| 4 | `crates/solosoul-core/src/vault_service/unlock.rs:428` | `unlock_with_kdf_upgrade` | 118 | 4 |
| 5 | `crates/solosoul-sync/src/session.rs:305` | `handle_inbound` | 116 | 3 |
| 6 | `src-tauri/src/commands/biometric.rs:383` | `biometric_save_credential` | 112 | 4 |
| 7 | `src-tauri/src/commands/export_import/import.rs:117` | `import_decrypt_preview` | 110 | 5 |
| 8 | `src-tauri/src/commands/object/trash.rs:271` | `page_delete` | 107 | 5 |
| 9 | `src-tauri/src/commands/recovery.rs:302` | `recovery_restore_from_host` | 106 | 4 |
| 10 | `src-tauri/src/commands/export_import/export_docx/mod.rs:274` | `export_objects_document` | 105 | 4 |

另：`storage.rs:808 create_schema_tables`（149 行纯 DDL）与 `lib.rs:44 register_core_commands`（133 行纯注册样板）属线性样板，未计入。

### P020 — Rust 深层嵌套（P2）

- 深度 8：`src-tauri/src/sync/auto_sync_core.rs:87` `spawn_scheduler`（主要为 `tokio::select!` 分支结构）。
- 深度 7：`commands/search/query.rs:88`、`solosoul-sync/src/attachments.rs:383`、`solosoul-sync/src/manager.rs:321`、`solosoul-vault/src/storage/objects.rs:669`、`solosoul-core/src/export_import.rs:482`、`solosoul-core/src/objects.rs:954`、`commands/export_import/helpers.rs:197`。
- 深度 6（代表性）：`commands/biometric.rs:573 biometric_unlock`（两个 `.map_err` 闭包几乎逐字重复，值得优先重构）、`solosoul-sync/src/delta.rs:124`、`commands/object/mod.rs:1120`、`sync/cloud_auto_sync.rs:628`。

### P021 — 前端超长组件第二梯队（P2）

- `src/components/attachment/AttachmentPreviewOverlay.tsx:34-444`（~388 行，0 处 memo）
- `src/pages/ai/PluginDashboardPage.tsx:35-451`（~386 行）
- `src/components/sync/RecoveryQrContent.tsx:19-391`（~362 行，几乎全静态 markup）
- `src/components/attachment/PhotoAlbumOverlay.tsx:33-396`（~357 行）
- `src/pages/editor/useObjectEditorPage.ts:65-480`（~352 行）
- `src/components/guide/PageGuide.tsx:43-432`（~350 行）
- `src/stores/pluginStore.ts:203` `runPlugin` action 约 137 行
- `src/stores/syncStore.ts:587-707` `initSyncCompletedListener` 内嵌事件处理器约 120 行

### P022–P023 — 前端性能（P2）

- **P022**：`PhotoAlbumGrid.tsx:153` 相册网格全量渲染 DOM 节点，无窗口化上限；缩略图已由 IntersectionObserver 懒加载缓解。主要列表（对象工作区、历史、回收站、日志、聊天）均已窗口化或后端限量，**无系统性问题**。
- **P023**：`ScanLocalPage.tsx:107` 目录导入 `Promise.allSettled(files.map(handleImport))` 并发无上限，建议加并发上限。全库无「循环内顺序 await invoke」模式（批量操作均已 Promise.all/分批并行化）。

### P024 — 冗余 export（P2，死代码边缘）

无整文件级死代码（431/540 模块被引用，其余为测试/入口/CSS）。以下 6 个 `export` 仅在定义文件内部使用，可去掉 export：

- `src/components/attachment/useAttachmentPreview.ts:33` — `buildPdfPreviewSrc`
- `src/components/attachment/usePhotoViewer.ts:12` — `SWIPE_THRESHOLD`
- `src/components/export/useExportDocumentSection.ts:21` — `FORMAT_FILTERS`
- `src/lib/conflictFieldMeta.ts:330` — `CONFLICT_VALUE_MAX_LEN`
- `src/lib/prefetch/warmup.ts:36` — `prefetchWarmupTasks`
- `src/test/setup.ts:58` — `MockResizeObserver`（测试基建）

TODO/FIXME 注释：全库 **零**。

### P025–P026 — 前端重复模式（P2）

- **P025**：已有共享组件 `src/components/plugin/shared/CopyButton.tsx`，仍有 ≥7 处自行实现 `navigator.clipboard.writeText` + copied 状态：`GuideCodeBlock.tsx:18`、`PluginResultPanel.tsx:202`、`useObjectDetailModal.tsx:191`、`SyncShowQrDialog.tsx:157`、`useLlmChatCore.ts:254`、`AccountSettingsPage.tsx:38`、`RecoveryQrContent.tsx`（双份）。
- **P026**：`visibleLimit` + `slice(0, visibleLimit)` + 「加载更多」增量分页重复出现于 5+ 文件：`useObjectWorkspaceData.ts`、`OperationLogPage.tsx:56`、`useTrashPage.tsx`、`DebugLogPage.tsx`、`HistoryPage.tsx:97`，各约 15 行，可抽公共 hook（如 `useIncrementalWindow`）。

### P027–P028 — 掩码约定偏离（P2，有意设计待确认）

- **P027**：`FieldSuggestions.tsx:47,125-131` 字段推荐对 internal 级明文展示（注释自述「内部级在推荐场景与公开同权」），建议确认例外是否写回 AGENTS.md 或收敛。
- **P028**：`SyncConflictDialog.tsx:373-433` + `commands/sync.rs:371` 冲突对话框明文渲染本地/远端字段差异（含 sensitive/critical），冲突解决需看清差异属可辩护设计，建议至少对受保护字段加揭示交互。

### P029 — `vault_set_directory` 锁定不发事件（P2，架构）

- **证据**：`commands/vault_directory.rs:160-166` 目录切换前 `svc.lock()` 但不 emit `vault-locked`（对比 `commands/vault.rs:13-15` 会 emit，`AppRoutes.tsx:250` 有完整监听清理链）。前端 `VaultDirectorySection.tsx:97-107` 仅显示重启提示，不重置 `isAuthenticated`，用户可离开本页继续操作，后续命令报「No account is currently unlocked」但 UI 仍显示已解锁。
- **缓解**：有重启提示 UI 与各命令错误 toast，不会静默损坏数据。
- **建议修复**：后端补 emit `vault-locked`，或前端成功后重置认证态。

### P030 — i18n 缺键（P2）

- **证据**：`node scripts/check-missing-i18n.mjs` 实测 zh-CN、en-US 各缺 `common:enabled`、`common:disabled`；使用点 `CloudSyncPage.tsx:603` 无 defaultValue，缺失时直接渲染原始 key 字符串。
- **建议修复**：补两份语言的 key。

## 已核查无发现的维度（留档）

- **Rust 安全**：无不安全 `unsafe`（4 处均为必要平台 FFI）；无命令注入（Command 全分离参数 + 用户名白名单）；路径遍历防护完整有测试；无硬编码密钥；无 `serde(untagged)`；加密无误用（nonce 唯一、KDF 参数正确、ct 比较、Zeroizing 贯穿）；无 SQL 注入（全参数化）。
- **Rust 死代码**：无确凿发现（全部非 command 非测试函数均有真实调用点；唯一 `#[allow(dead_code)]` 是 RAII 锁句柄的有意保留）。
- **前端安全**：无 `dangerouslySetInnerHTML`/`eval`/`new Function`；Markdown 统一经 `SafeMarkdown` 消毒；无敏感数据写日志；文件对话框全部经 `lib/dialog.ts` 封装（18 个调用方无裸调）。
- **架构**：crates 依赖为单向 DAG 无循环；capabilities 无过度授权（fs/shell 均最小权限）；IPC 统一走 `ipcClient.ts` 不吞错；Zustand↔Rust 事件同步主链路完整（唯一缺口即 P029）。

## 备注

- 按用户要求，本轮**不进入阶段 3 修复流程**，所有问题保持 `[ ]` 待修复状态。
- P001/P002 为 CI 阻断项，建议优先处理；P003 为唯一安全策略不一致项，建议紧随其后。
