# 代码分析修复报告

> 最后更新：2026-08-01 01:20:00
> 当前分支：`main`
> 修复轮次：3 + 复核轮次 1（58 项修复声明已独立核验）+ 决策轮次（剩余 5 项方案与用户决策已记录，见各条目「决策记录」）
> 分析范围：`tauri/`（Rust 后端 `src-tauri/` + `crates/`，React/TS 前端 `src/`）；`solosoul_cli/` 不在本轮范围

## 基线检查结果（阶段 0，全部通过）

| 检查项 | 结果 |
|--------|------|
| `cargo fmt --check` | ✅ 通过 |
| `cargo clippy -- -D warnings` | ✅ 通过，0 警告 |
| `cargo test` | ✅ 通过（含 vault 99 项等全部套件，0 失败） |
| `npx tsc --noEmit` | ✅ 通过 |
| `npm run lint`（ESLint） | ✅ 通过 |
| `npm run test`（Vitest） | ✅ 45 个测试文件 / 423 个测试全部通过 |

## 问题清单（按优先级 P0 > P1 > P2）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| P001 | P0 | 漏洞 | `tauri/src-tauri/src/commands/discovery.rs:239-243`、`tauri/crates/solosoul-sync/src/recovery.rs:172-187` | 恢复凭证（PIN+nonce）经 mDNS TXT 明文广播，局域网攻击者可窃取整个 Vault | `[x]` 已修复 |
| P002 | P1 | 漏洞 | `tauri/crates/solosoul-core/src/export_import.rs:811-833` | 加密导入包附件路径遍历（Windows 可任意目录写） | `[x]` 已修复 |
| P003 | P1 | 漏洞 | `tauri/crates/solosoul-core/src/export_import.rs:836-855`、`tauri/crates/solosoul-plugin/src/host.rs:1284-1285` | 导入时附件元数据 `file_name` 未净化，形成存储型路径遍历 | `[x]` 已修复（复核补改：GUI 导入路径净化对齐 core 并写回 safe_name） |
| P004 | P1 | 漏洞 | `tauri/src/components/plugin/PluginResultPanel.tsx:331-337` | 前端用插件提供的路径直接 shell open，可打开/执行任意本地文件 | `[x]` 已修复（复核补改：host 侧盖章真实 output_dir 闭环信任锚） |
| P005 | P1 | 漏洞 | `tauri/crates/solosoul-core/src/pin.rs:188-197,336-350` | PIN 解锁将 Vault 安全性降为 6 位离线爆破，锁定计数可被绕过 | `[x]` 已修复 |
| P006 | P1 | 性能 | `tauri/src-tauri/src/commands/search/query.rs:155-168,238,261` | 搜索分页计数用 `list_objects` 全量解密后取长度，N+1 次 AES 解密 | `[x]` 已修复 |
| P007 | P1 | 性能 | `tauri/src-tauri/src/commands/search/commands.rs:305-346` | 模板命中时第二次全表解密扫描，单次搜索最多 2 次全表解密 | `[x]` 已修复 |
| P008 | P1 | 性能 | `tauri/src-tauri/src/commands/llm/rag.rs:358-363` | top_k 循环内反复 `chunk_all_guides`，每条聊天消息把指南文件完整读 3 遍 | `[x]` 已修复 |
| P009 | P1 | 性能 | `tauri/src-tauri/src/commands/ocr.rs:258,356` | 每次 OCR 命令重新加载 ONNX 引擎（数百 ms），无缓存 | `[x]` 已修复 |
| P010 | P1 | 性能 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:148-158` | `useObjectStore()` 无 selector 整店订阅，store 任何变化触发整页重渲染 | `[x]` 已修复（复核补改：`useObjectWorkspaceData.ts:141` 残留的 templateStore 裸订阅已分字段化） |
| P011 | P1 | 性能 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:803`、`tauri/src/pages/settings/GlobalAttachmentManager.tsx:1077` | 大列表无虚拟滚动/分页，对象数百+ 时首屏与重渲染成本高 | `[x]` 已修复（复核补改：GlobalAttachmentManager 顶层页面列表补「加载更多」分页） |
| P012 | P1 | 架构/重复 | `tauri/src-tauri/src/plugin/` vs `tauri/crates/solosoul-plugin/src/` | 插件运行时双份平行实现：GUI 用本地版（功能超集），CLI 用 crate 版（见详情事实修正） | `[>]` 已决策待执行（方向 B：统一到 crate，6 步计划见详情） |
| P013 | P1 | 架构 | `tauri/src-tauri/src/commands/discovery.rs:30`、`tauri/crates/solosoul-sync/src/manager.rs:168` | mDNS ServiceDaemon 在两个层各起一个实例，可同时存活导致结果不一致 | `[x]` 已修复 |
| P014 | P1 | 架构 | `tauri/src/stores/authStore.ts:154-155` | `logout` invoke 失败时前端认证状态永不重置，出现半认证僵尸态 | `[x]` 已修复 |
| P015 | P1 | 架构 | `tauri/src/stores/vaultStore.ts:15-44`、`tauri/src/stores/authStore.ts` | 认证/锁定状态双 store 平行维护，`vaultState` 只写不读，存在三种写入路径 | `[x]` 已修复（复核补改：LoginPage PIN/生物识别两条裸 setState 收敛为 authStore.completeUnlock） |
| P016 | P1 | 规范 | `tauri/src/pages/scan/ScanLocalPage.tsx:60`、`tauri/src/components/plugin/WatermarkPluginConfig.tsx:192`、`tauri/src/components/layout/OcrQuickScanPopover.tsx:139` | 三处裸调 plugin-dialog `open`，违反 dialog.ts 封装约定，可致自动锁定误触发 | `[x]` 已修复 |
| P017 | P1 | 死代码 | `tauri/src/components/liquid-glass/`、`tauri/src/styles/liquid-glass.css` | 整套玻璃拟态组件与样式零引用 | `[x]` 已删除（2026-08-01） |
| P018 | P1 | 死代码 | `tauri/src/stores/index.ts`、`tauri/src/components/guide/index.ts` | 两个 barrel 文件无任何导入方 | `[x]` 已删除（2026-08-01） |
| P019 | P1 | 死代码 | `ocrInstallStore.ts:27`、`trash/types.ts:60`、`ipc.ts:77`、`templateSync.ts:8`、`exportImport.ts:71`、`template.ts:63` | 6 个零引用导出符号（OCR_STORAGE_KEY、TrashItem、ProfileSummary、TemplateSyncStatus、ImportSelection、UserTemplateRaw） | `[x]` 已修复 |
| P020 | P1 | 死代码 | 详见下文清单 | 18 个已注册但前端从未 invoke 的死 Tauri Commands（缩小 IPC 攻击面） | `[x]` 已修复 |
| P021 | P1 | 死代码 | 详见下文清单 | 7 个零调用点 Rust 函数（含注释声称已接线但实际未调用的 `ensure_guide_embeddings_built`） | `[x]` 已修复 |
| P022 | P1 | 死代码 | `tauri/package.json:28,38,47` | 3 个未被 import 的 npm 依赖（react-hook-form、@hookform/resolvers、plugin-window-state） | `[x]` 已修复 |
| P023 | P1 | 结构 | `tauri/src-tauri/src/commands/export_import/import.rs:234` | `import_execute_internal` 单函数 431 行，多阶段逻辑混杂 | `[x]` 已修复 |
| P024 | P1 | 结构 | `tauri/src/pages/settings/GlobalAttachmentManager.tsx:116` | 组件 1202 行（全项目最大），过滤/预览/批量操作全塞一个函数体 | `[x]` 已修复 |
| P025 | P1 | 结构 | `tauri/src/components/recovery/RecoveryReceiveDialog.tsx:57` | 组件 1001 行，QR 扫描+状态机+渲染一体 | `[x]` 已修复 |
| P026 | P1 | 结构 | `tauri/src/components/onboarding/OnboardingDialog.tsx:38` | 组件 989 行，多步向导+手写 hover 混杂 | `[x]` 已修复 |
| P027 | P1 | 结构 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:57` | 组件 881 行，数据加载/模板同步/拖拽/渲染全职责 | `[x]` 已修复 |
| P028 | P1 | 重复 | `llm/mod.rs:210-231`、`llm/stats.rs:154-173`、`llm/conversation.rs:49-64`、`services/llm_context.rs:339-353`、`commands/settings.rs:260+` | 「写 profile preferences」约 20 行整块复制 6 处（报告原列 snapshot.rs 为过时行号，该文件只有读无写；llm/mod.rs 实为 2 处） | `[x]` 已修复 |
| P029 | P2 | 漏洞 | `tauri/crates/solosoul-sync/src/recovery.rs:175` | 恢复 PIN+nonce 使用非常数时间字符串比较（违反项目安全约定） | `[x]` 已修复 |
| P030 | P2 | 漏洞 | `tauri/src-tauri/capabilities/default.json` | capabilities 授权面过大（fs `**`、shell open `**`、allow-all-custom-commands） | `[x]` 已修复 |
| P031 | P2 | 漏洞 | `tauri/src-tauri/src/commands/auth.rs:37-46,77-89` | GUI 登录/建库密码以普通 String 经 IPC，未用 Zeroizing（CLI 已对齐） | `[x]` 已修复 |
| P032 | P2 | 漏洞 | `tauri/src-tauri/tauri.conf.json:78` | shell.open 正则允许 `file://` 与绝对路径 `/…` | `[x]` 已修复 |
| P033 | P2 | 死代码 | 详见下文清单 | 9 个"命令注册多余"的 Tauri Commands（有内部调用或仅测试用，建议取消注册） | `[x]` 已修复 |
| P034 | P2 | 死代码 | `tauri/src-tauri/src/commands/object/mod.rs:710,745` | `object_backfill_property_labels`/`object_backfill_property_fields` 疑似一次性迁移工具，待确认后删除 | `[x]` 已修复 |
| P035 | P2 | 死代码 | `commands/mod.rs:65`、`services/llm_context.rs:86`、`crates/solosoul-sync/src/manager.rs:225`、`tauri/package.json:32` | P2 死代码组：mobile_not_supported_with、clear_cache 未接线确认、set_active_sessions_for_test 应 cfg(test)、plugin-http npm 依赖 | `[x]` 已修复 |
| P036 | P2 | 死代码 | `tauri/src-tauri/src/sync/device_auto_sync.rs:148` | `trigger_periodic` 零调用，Periodic 事件分支不可达 | `[x]` 已修复 |
| P037 | P2 | 死代码 | `lib/plugin.ts`（13 处）、`useNavigationItems.ts`（6 处）、`lib/updater.ts`（5 处）等 | 约 60 处"仅本文件内使用、export 多余"的类型导出 | `[x]` 已修复（热点批次 27 处） |
| P038 | P2 | 结构 | `tauri/src/pages/auth/LoginPage.tsx:32,111-120,220-229` | 组件 846 行；biometryType→显示名 if-else 链同文件重复两份 | `[x]` 已修复 |
| P039 | P2 | 结构 | `tauri/src/components/import/ImportSection.tsx:71`、`tauri/src/components/import/ExportSection.tsx:88` | ImportSection 848 行 / ExportSection 751 行，结构高度对称可共享抽取 | `[x]` 已修复 |
| P040 | P2 | 结构 | `tauri/src/pages/system/AboutPage.tsx:50` | 组件 834 行，双平台更新下载流程应抽 hook | `[x]` 已修复 |
| P041 | P2 | 结构 | `TemplateEditor.tsx:81`(789)、`ObjectDetailModal.tsx:120`(783)、`AttachmentViewer.tsx:59`(763)、`SyncPage.tsx:91`(759) | 4 个 750+ 行大组件需拆分 | `[x]` 已修复（复核补改：ObjectDetailModal/SyncPage 继续拆分至阈值以下） |
| P042 | P2 | 结构 | `tauri/crates/solosoul-vault/src/migration.rs:31` | `run_migrations` 464 行，每版本重复「查列是否存在」样板 | `[x]` 已修复 |
| P043 | P2 | 结构 | `tauri/src-tauri/src/lib.rs:135` | `run` 441 行，启动初始化 10+ 步全内联在一个 setup 闭包 | `[x]` 已修复 |
| P044 | P2 | 结构 | `export.rs:217`(258)、`object/snapshot.rs:210`(254)、`llm/stream.rs:76`(227) | 3 个 220+ 行多阶段函数需按阶段抽取 | `[x]` 已修复 |
| P045 | P2 | 结构 | `export_import/helpers.rs:173-232`、`import.rs:394-412`、`profile.rs:191-205`、`plugin.rs:80-107`、`llm/stream.rs:257-302` | 5 处 5-6 层深层嵌套 | `[x]` 已修复 |
| P046 | P2 | 重复 | `tauri/src-tauri/src/commands/attachment.rs:983-1027` vs `tauri/crates/solosoul-core/src/objects.rs:945-999` | `cleanup_orphan_attachments` GUI 端整体复制 core 实现 | `[x]` 已修复 |
| P047 | P2 | 重复 | `tauri/src-tauri/src/plugin/host/register.rs:763-812,838-890` | 两个 watermark 注册闭包除一行外完全相同；read_string 校验样板 20+ 次 | `[>]` 已决策待执行（并入 P012 方向 B 第③步，在统一后的 crate host.rs 上只做一次） |
| P048 | P2 | 重复 | 54 文件 119 处 onMouseEnter、57 文件 128 处 onMouseLeave（内联样式改写 471 处/49 文件） | 手写 onMouseEnter/Leave hover，应统一迁移到 `ui/Button` 或视觉等价 CSS hover | `[~]` 部分完成（第一批 3 文件 19 处已迁移，2026-08-01） |
| P049 | P2 | 重复 | `TrashPage.tsx:254-345`（×2）、`OperationLogPage.tsx:280-323`、`TemplateManagerPage.tsx:597+`、`SampleTemplateGallery.tsx:282+` | 筛选 chip 按钮块（约 45 行）重复 5 处，可抽 FilterChipGroup | `[x]` 已修复 |
| P050 | P2 | 结构 | `tauri/src/pages/editor/ObjectEditorPage.tsx:300-360` | 动态组校验 switch 6 个 case 同一模式，应表驱动化（含 5 层嵌套） | `[x]` 已修复 |
| P051 | P2 | 性能 | `tauri/src-tauri/src/commands/llm/rag.rs:794-808,942` | 重建 embedding 逐条 `save_guide_embedding`，每条独立事务+fsync | `[x]` 已修复 |
| P052 | P2 | 性能 | `tauri/src/stores/trashStore.ts:137-140` | `permanentDelete` 循环内串行 await 逐条 IPC | `[x]` 已修复 |
| P053 | P2 | 性能 | `tauri/src/stores/settingsStore.ts:328-355` | `loadCustomPages` 对每个自定义页单独 `object_get`（N+1 IPC） | `[x]` 已修复 |
| P054 | P2 | 性能 | `GlobalAttachmentManager.tsx:419-425`、`AttachmentViewer.tsx:343-361` | 附件批量下载逐条串行 IPC+文件拷贝 | `[x]` 已修复 |
| P055 | P2 | 性能 | `ObjectEditorPage.tsx:36`、`TrashPage.tsx:79`、`TemplateManagerPage.tsx:75`、`ObjectDetailModal.tsx:136` | 多处 `useXxxStore()` 整店订阅（负载较小，同类于 P010） | `[x]` 已修复 |
| P056 | P2 | 架构 | `tauri/src/stores/objectStore.ts:184-220` | objectStore 的 trash 切片是死代码，与 trashStore 双轨调用不同后端命令 | `[x]` 已修复 |
| P057 | P2 | 架构 | `tauri/src/stores/objectStore.ts:158-169` | `updateObject` 只更新缓存不同步 `objects` 摘要列表（潜伏性不一致） | `[x]` 已修复 |
| P058 | P2 | 架构 | `tauri/src/stores/profileStore.ts:75-98` | `loadSection`/`updateField` 无调用方死代码，且 updateField 写后不同步本地 | `[x]` 已修复 |
| P059 | P2 | 架构 | `HistoryPage.tsx:36-38`、`HistoryViewer.tsx:485-487`、`vaultStore.ts:40-43` | 3 处前端 invoke 链缺 `.catch`，锁定等场景下 unhandled rejection 且无提示 | `[x]` 已修复 |
| P060 | P2 | 规范 | `tauri/src/components/plugin/PluginResultPanel.tsx:344,360` | UI 组件直接调 plugin-fs `copyFile` 且静默吞错，违反 IPC 封装约定 | `[x]` 已修复 |
| P061 | P2 | 架构 | `tauri/src/pages/settings/GlobalAttachmentManager.tsx`（12 处直接 invoke） | 页面组件承载附件树遍历/聚合/批量编排等重业务逻辑，应下沉 store/lib（与 P024 关联） | `[x]` 已修复 |
| P062 | P2 | 架构 | `tauri/src/stores/settingsStore.ts:154-307` | 主题/语言设置四副本（zustand+localStorage+ui_preferences.json+vault），需补写入路径矩阵注释或收敛 | `[x]` 已修复 |
| P063 | P2 | 架构 | `tauri/Cargo.toml:73` | release profile `panic = "abort"`：未来新增生产代码引入 unwrap 时代价大，保留认知即可 | `[x]` 已处理（标记为「设计如此」，无需改动） |

## 修复进度

- 已完成：60 / 63（经 2026-08-01 复核确认通过的项）
- 部分完成 `[~]`：1 项——P048（第一批 3 文件 19 处已迁移，2026-08-01）
- 已决策待执行 `[>]`：2 项——P012（方向 B 统一到 crate，P047 并入）
- 当前处理：P048 第一批已完成（2026-08-01），待后续批次

## 静态基线之外已检查且无发现的维度（误报排除记录）

- 硬编码密钥/密码/token：无真实发现（命中均为测试夹具）。
- XSS：前端无 `dangerouslySetInnerHTML`/`innerHTML`/`eval`；Markdown 统一走 `SafeMarkdown.tsx`。
- 命令注入：3 处 `Command` 调用均无用户输入拼接。
- serde untagged：未使用。
- unsafe 块：均为平台 FFI 且有 SAFETY 注释。
- 加密误用：AES-256-GCM nonce 每次随机、模型下载强制 SHA256、registry minisign 验签、同步 Noise_XX，无问题。
- 敏感信息写日志：未发现密码/session key/解密内容输出。
- TODO/FIXME/大段注释代码/不可达分支：无发现。
- Crate 循环依赖：无环（crypto←vault←core←sync/plugin←src-tauri 严格单向）。
- Rust Command 裸 panic：全部命令返回 `Result<T, String>`，锁中毒处理到位。
- 生产路径 unwrap/expect：仅 `settings.rs:73,80` 两处且可证明非空，无滥用。
- 大文件加解密分块、Embedding 模型缓存、指南索引缓存：均已正确实现。

## 详细问题描述与修复指引

### P0

**P001 | 恢复凭证经 mDNS 明文广播（安全）**
`recovery_advertise`（`discovery.rs:239-243`）把 `pin`、`nonce`、`fp`、`addr` 全部写入 mDNS TXT 明文广播；主机端唯一认证闸门是 `recovery.rs:172-187` 的 `auth != expected`（nonce:pin）。局域网内任何人浏览 `_solosoul_recovery._tcp.local.` 即得 addr+PIN+nonce+指纹，直接连接真实主机通过认证后拿到恢复密码与加密导出包 = 完整 Vault 失陷；指纹同信道广播使 MITM 校验形同虚设。
修复方案：mDNS TXT 只放 addr+指纹+服务名，PIN/nonce 仅经 QR 码/手动输入带外传递；长期可改 PAKE（SPAKE2）。修复成本低。

### P1 安全

**P002 | 导入包附件路径遍历**
`export_import.rs:811-833`：附件 ZIP 条目按 `/` 切分校验 `parts.len()==2` 后，`obj_id` 未经字符校验直接 `join("attachments").join(obj_id)`；`obj_id` 来自攻击者构造的解密 payload（`:353,:430` 无格式校验）。Windows 上对象 ID 设为 `..\..\..\evil` 可写 Vault 目录之外。
修复方案：复用 `attachment.rs:19` 的 `validate_attachment_id`（ASCII 字母数字+`-_`）校验导入对象/附件 ID；`resolve_path`+`is_path_under_workspace` 断言落点在 `attachments/` 下。与 P003 同源可一并修复。

**复核注记（2026-08-01，判定通过）**：白名单校验已落实——core 新增 `validate_import_id`（`export_import.rs:38-51`，`:843-844` 校验 obj_id 与 old_att_id），GUI 用等价 `validate_export_id`（`import.rs:678`/`mod.rs:61-71`）；字符集排除 `.`，遍历在源头不可能。**但 commit/修复说明声称包含「落点断言」（resolve_path+is_path_under_workspace），代码中并不存在该断言**（core/GUI 两条导入路径均无）。白名单已闭环，断言属冗余纵深，不影响安全判定，但说明与代码不符，特此记录。

**P003 | 附件元数据 file_name 存储型路径遍历**
`export_import.rs:836-855`：落盘用 `safe_name`（取 `Path::file_name()`），但元数据写回原始 `file_name`；后续插件主机 `copy_attachment_to_workspace`（`host.rs:1284`）直接 `dst_dir.join(file_name)`。恶意导入包设 `file_name=../../evil.txt`，用户用插件处理该附件时发生遍历写。
修复方案：导入时将 `safe_name` 写回元数据；`copy_attachment_to_workspace` join 前加末段净化兜底。

**复核发现（2026-08-01，状态降为 `[~]`）**：core 路径已修（`export_import.rs:862` 净化 + `:877` 写回 `file_name: safe_name`），host 兜底双份已加（`crates/solosoul-plugin/src/host.rs:1296-1309` 与 `src-tauri/src/plugin/host/register.rs:1113-1126` 的 `sanitize_attachment_file_name` 拒绝 `/` 与 `\`）。**但 GUI 导入路径 `tauri/src-tauri/src/commands/export_import/import.rs:734` 仍写回 `file_name: old_meta.file_name.clone()` 原始值**，未写回 `:714-718` 算出的 `safe_name`——源头净化只落实了 core（CLI）一半。当前不可利用的唯一原因是消费点 `copy_attachment_to_workspace` 已有兜底净化；按纵深防御原则仍应补齐 GUI 路径的写回。次要差异：GUI 落盘净化 `:714` 仅用 `Path::file_name()`，不像 core 版显式拒绝 `\`（Windows 上会被解析，Unix 上 `\` 非分隔符，实际风险低）。

**修复记录（2026-08-01，状态恢复 `[x]`）**：GUI 导入路径 `import.rs` 补齐——`import_attachments` 净化逻辑对齐 core `sanitize_import_file_name`（显式拒绝含正斜杠/反斜杠的原始名、再取末段组件、拒绝空/`.`/`..`），元数据 `file_name` 写回 `safe_name.clone()`。core/GUI 双导入路径与 host 兜底三层净化全部闭环。

**P004 | 插件提供路径直接 shell open**
`PluginResultPanel.tsx:331-337`：`new URL(path.replace(/\\/g,'/'),'file://').href` 后 `open(fileUrl)`，`path` 来自插件返回值，无校验；叠加 `capabilities/default.json` 的 `shell:allow-open url=**` 与 `tauri.conf.json:78` 允许 `file://`/绝对路径，恶意插件可诱导用户打开 `.app`/脚本，等同逃逸 WASM 沙箱获得本机执行能力。
修复方案：仅允许打开经工作区/插件输出目录校验的路径；或改"在文件夹中显示"；配合 P032 收紧 open 正则。

**复核发现（2026-08-01，状态降为 `[~]`）**：前端已改 `invoke('plugin_open_output_file')`（`PluginResultPanel.tsx:331-343`），Rust 侧 `plugin.rs:233-253` 的 `resolve_output_file` 有 canonical 化 + `starts_with` 包含校验。**但包含校验的基准目录 `outputDir` 本身来自插件可控数据**——`payload.outputDir` 取自插件自行构造的 `watermark_result` 结果 JSON（host 透传不盖章，`pluginStore.ts:44-45` 仅校验形状）。恶意插件上报 `outputDir: "/"` + `outputPath: "/Applications/xxx.app"` 时，`canonical(path).starts_with("/")` 恒真，校验形同虚设，`opener::open` 仍可打开/执行任意本地文件，原始「沙箱逃逸获得本机执行」威胁对恶意插件**依然成立**。且 `opener` crate 直接调系统打开，不经 plugin-shell，P032 的正则收缩对此路径无防御作用。P060 新增的 `plugin_copy_output_file` 共用 `resolve_output_file`，同一缺陷同样存在。**修复建议**：host 侧在收集结果时用运行参数中宿主已知的真实 `output_dir` 覆写/盖章结果 payload（或前端从运行上下文而非插件 payload 取 `outputDir`），使校验基准不受插件控制。

**修复记录（2026-08-01，状态恢复 `[x]`）**：host 侧盖章已落实——`register.rs` 的 `solosoul_result` 函数对 `watermark_result` 载荷用宿主已知的 run param `outputDir`（`WatermarkPluginConfig` 用户所选目录）覆写 `outputDir` 字段；宿主无真实目录（未配置/空串）时写空串使 `resolve_output_file` 对空串 canonicalize 失败而安全拒绝，绝不透传插件自报值。盖章发生在结果存入 `host.results` 与事件发送前的同一处，`plugin_open_output_file`/`plugin_copy_output_file` 的校验基准（`plugin.rs` `resolve_output_file`）不再受插件控制，恶意插件上报 `outputDir:"/"` 的绕过路径已闭合。注记：`crates/solosoul-plugin/src/host.rs` 平行死实现（P012 待删）存在同一漏洞，随 P012 一并消除。

**P005 | PIN 离线爆破短路主密码**
`pin.rs:188-197`：`derive_key(pin,...)` 派生 KEK 加密会话密钥写入 `pin_credential`；防爆破的 `pin_failed_attempts`/`pin_locked_until`（`:336-350`）只是数据目录 JSON 字段，攻击者拿到数据目录副本后可离线爆破 6 位 PIN（10⁶ 组合，开发 KDF 参数下约数小时）解开全库。
修复方案：PIN 凭证强制生产级 KDF 参数（不随 SOLOSOUL_SECURE 降级）；UI 明确风险提示；考虑 PIN 绑定设备密钥（Keychain/Keystore 包裹）使离线副本无法单独爆破。

**复核注记（2026-08-01，判定通过）**：核心修复已落实——`pin.rs:106-108` `pin_kdf_config()` 恒返回 `KdfConfig::production()`（64MiB/3iter），存量凭证解锁时经 `decrypt_session_key_with_fallback` 回退旧参数并 `upgrade_credential` 就地升级，逻辑正确。**附属项「UI 明确风险提示」未见实现**（grep 无相关文案），「PIN 绑定设备密钥」原文为「考虑」可接受。如需完整闭环，UI 提示可作后续小项。

### P1 性能

**P006 | 搜索计数 N+1 全量解密**
`query.rs:155-168`：`count_page_objects`/`count_section_objects` 调 `list_objects` 对全部对象 properties 做 AES-GCM 逐行解密只为取 `.len()`；`search_pages` 对每个命中页面+最多 4 个系统分区各调一次。
修复方案：改 `SELECT COUNT(*) FROM objects WHERE account_id=?1 AND parent_id=?2 AND is_deleted=0`。

**P007 | 模板命中二次全表解密**
`commands.rs:305-346`：`search_advanced_impl` 已解密全部对象一次，模板匹配时又在 :306 再次 `list_objects(None,None,None)` 全表解密。
修复方案：复用已解密 records 做模板归属过滤，合并为一次扫描。与 P006 同区域可一并修复。

**P008 | 指南分块循环内重复读盘**
`rag.rs:358-363`：top_k（默认 3）循环内每次调 `chunk_all_guides`（`rag.rs:425-434` 全量 `read_to_string`+分块），每条 LLM 聊天消息把所有指南文件完整读 3 遍只为取 `guide_title`。
修复方案：调用提升出循环，先建一次 `guide_id→title` HashMap。

**P009 | OCR 引擎无缓存**
`ocr.rs:258,356`：每次 OCR 命令重新 `OcrEngine::load`（det+rec 两个 ONNX session，数百 ms）。
修复方案：照搬 `local_embed.rs:20` 的 `EMBEDDER_CACHE` 模式，按 `OcrModelTier` 全局缓存 `Mutex<Option<Arc<OcrEngine>>>`。

**复核注记（2026-08-01，P006–P009 均判定通过）**：四处修复均已核验落实——P006 改纯 SQL `count_objects`（`storage.rs:2527-2554`，零解密）；P007 模板分支复用 `all_records` 单次全表解密（`commands.rs:42,304-343`）；P008 title HashMap 循环外一次构建（`rag.rs:357-365`）；P009 新增 `OCR_ENGINE_CACHE`（`ocr.rs:34,41-62`），缓存 key 含 tier（语言由档位模型决定，key 完备），安装/下载/删除模型后均清缓存（`:619,661,706,748`）。**两点报告维护问题**：① P006–P009 正文缺「修复说明」段落，仅表格标 `[x]`，无法比对说明与代码一致性；② 四项无独立 commit，改动混入 `61271ba7`（message 只写 P001-P005 与死代码清理），message 未提及 P006–P009，违反「一项一提交」的追溯原则。建议补记修复说明。

**P010 | 工作区整店订阅**
`ObjectWorkspacePage.tsx:148-158`：`useObjectStore()` 无 selector，store 每个 action 翻转 `isLoading` 两次并重建 `currentObjectCache`，任何变化触发整页（含 803 行全量卡片 map）重渲染。
修复方案：分字段 selector（`s=>s.objects`、`s=>s.isLoading` 等）。P055 为同类低负载页面。

**复核发现（2026-08-01，状态降为 `[~]`）**：objectStore 已分字段化——P027 拆分后订阅收敛到 `hooks/useObjectWorkspaceData.ts:121-129`，9 个分字段 selector，页面已无裸 `useObjectStore()`。**但同一 hook 的 `:141` 仍残留 `const { templates: userTemplates, loadTemplates: loadUserTemplates } = useTemplateStore();` 裸整店订阅**，templateStore 任何变化（如 `isLoading` 翻转）仍触发整个工作区页重渲染，与本项要消除的问题同类同处。对照 P055 中 ObjectDetailModal 的 templateStore 都已改为分字段 selector，此处属漏改。

**修复记录（2026-08-01，状态恢复 `[x]`）**：`useObjectWorkspaceData.ts` 的 `useTemplateStore()` 裸订阅已拆为 `useTemplateStore((s) => s.templates)` 与 `useTemplateStore((s) => s.loadTemplates)` 两个分字段 selector——templates 值字段按引用比较、loadTemplates 函数引用稳定，templateStore 的 `isLoading`/`error` 翻转不再触发工作区页重渲染，与同 hook 中 objectStore 的分字段化完全对齐。tsc/lint/Vitest 415 全绿。

**P011 | 大列表无虚拟化**
全项目无 windowing 库；工作区一次挂载全部对象卡片，附件管理器全量渲染对象×附件树。
修复方案：引入 `@tanstack/react-virtual` 或分页/「加载更多」。

**修复记录（2026-08-01，状态恢复 `[x]`）**：GlobalAttachmentManager 顶层页面列表补「加载更多」分页——模块级 `VISIBLE_PAGE_SIZE = 20`，`visiblePages = displayPages.slice(0, visiblePageLimit)`，搜索词/回收站视图切换时重置游标，列表尾部超出时显示 `Button`「加载更多」逐批展开（与工作区 ObjectWorkspacePage 既有 P011 模式一致）。折叠状态（expandedPages/expandedObjects）与选中状态按 key 记录，slice 后未渲染页的状态保留无行为差异。tsc/lint/Vitest 415 全绿，两轮审查通过。

### P1 架构 / 规范

**P012 | 插件运行时双份平行实现（最高优先级架构债）**
`src-tauri/src/plugin/`（manager.rs 689 行、host/ 1596 行、registry.rs 265）与 `crates/solosoul-plugin/src/`（manager.rs 394、sandbox.rs 129、registry.rs 205、host.rs 1585）各一套；`register_host_functions` 两处各 914 行约 95% 相同，sandbox.rs diff 为 0。`app_state.rs:4` 实际用本地版本，crate 侧约 3000 行无人调用（grep 全 workspace 无 `solosoul_plugin::{manager,sandbox,registry}` 引用）；已观察到两处行为分歧（consent 超时 `Ok(Ok(Ok(..)))` vs `Ok(Ok(..))`、`write_u32` vs `write_handle`），双份代码开始漂移；wasmtime feature 声明也不一致。
修复方案：二选一——删除 crate 侧死实现，或让 src-tauri 全面改用 crate 版本。属大改动，建议单独一轮处理并全量回归插件功能。

**决策记录（2026-08-01，用户已确认：方向 B 统一到 crate）**

*事实修正（深度调查后）*：原报告「crate 侧约 3000 行无人调用」**不成立**——`solosoul_cli/Cargo.toml:17` 依赖 `solosoul-plugin`，CLI 的 12 个插件命令消费 crate 侧 `PluginManager`/`PluginEvent`/`PluginEventSink`/`PluginManifest`。真实格局：**一份运行时实现两次，GUI 消费 src-tauri 本地版（功能超集），CLI 消费 crate 版（架构更好）**。两侧的主要分歧：

- manager：crate 版同步、仅读 bundled；本地版 async、远程下载+bundled 回退+hash 短路+降级安装+effective_roles 回填（多 ~240 行）
- registry：crate 单路径（Release 下远程更新必写失败）；本地版 bundled+cache 双路径、原子写缓存、pubkey 缺失优雅跳过
- event：crate 多 `custom_type` 字段与 `PluginEventSink` trait（Tauri 解耦关键）；本地用 `tauri::ipc::Channel`
- host：`write_handle` vs `write_u32` 同功能异名；consent 阻塞形态不同但行为等价
- paths：本地版含 Android `asset://` 规避分支
- wasmtime features：crate `async,cranelift,pulley` vs 本地 `cranelift,runtime,component-model,pulley`，两侧代码均只用同步 API，`async`/`component-model` 均无代码使用
- 两侧 orphan mobile 文件（host_mobile/manager_mobile/sandbox_mobile，共 511 行）未被 mod 声明、未参与编译，属纯死文件
- `version.rs` 逻辑存在 3 份（crate + 本地内联 ×2）

*方向 A（删 crate 侧）已排除*：会砸掉 CLI 12 个命令或制造第三份拷贝。

*方向 B 执行计划（6 步）*：
1. **零风险清理**：删除两侧 6 个 orphan mobile 文件（511 行）；`version.rs` 三份去重。
2. **功能移植进 crate**：本地 manager 的 async install/远程下载/bundled 回退/hash 短路/降级安装/locale 启动消息、registry 缓存双路径与 pubkey 容错；新增 `new_with_dirs(market_dir, data_dir)` 显式注入目录（app_state 负责 AppHandle 解析含 Android 分支，crate 不反向依赖 tauri）；`install_from_registry`/`update` 改 async（CLI 调用点同步适配）。
3. **host 对齐 + P047**：统一 `write_handle`/`write_u32` 命名；抽 watermark 公共闭包与 `read_string` 样板，只在 crate `host.rs` 做一次（先在本地做 P047 会在第 4 步被丢弃，故并入此步）。
4. **GUI 切换**：src-tauri 加 ~25 行 `TauriChannelSink(Channel<PluginEvent>)` 适配器；`app_state.rs:355-373` 构造方式改显式注入；删除本地 6 模块 2894 行，`plugin/mod.rs` 改纯 re-export；适配集成测试 `plugin_sandbox.rs`/`plugin_address_fmt.rs`（`SoloHostFunctions` 签名变为 `Arc<dyn PluginEventSink>`）；确认前端 `pluginStore.typeGuards` 容忍事件 JSON 多出 `"customType": null`。
5. **依赖收敛**：两侧 wasmtime features 统一为 `cranelift, pulley`。
6. **全量回归**：`cargo test`（workspace + CLI）→ 4 个插件集成测试 → 前端 pluginStore 测试 → 手工回归清单：市场列表/搜索 → 安装（远程/bundled 回退/hash 重装三路径）→ 更新 → 卸载 → 运行 hello_world/address-fmt/phone-fmt → 水印插件（图片+PDF）→ consent 通过/拒绝/300s 超时 → host HTTP → 审计日志/会话 TTL → Android 端插件运行 → CLI 全部 12 个插件命令。

*净效果*：删 ~2900 行、crate 增 ~350 行，全项目只剩一份插件运行时。注意：crate 内 `env!("CARGO_PKG_VERSION")` 语义在统一后会改变（当前数值巧合一致），移植时需显式处理。

**P013 | mDNS 双 daemon**
sync crate manager（`manager.rs:168`）`sync_enable` 时自建 `ServiceDaemon`；`discovery.rs:30,55-60` 另有 app 生命周期常驻 `SharedDaemon`。前端 `syncStore.enable` 成功后立即 `discoverDevices`（`syncStore.ts:140-141`），两个 daemon 同时运行，缓存各自为政。
修复方案：进程内只保留一个 ServiceDaemon（discovery 命令复用 sync crate 实例或反之）。运行时端口冲突未做动态验证，修复时实测。

**P014 | logout 状态分歧**
`authStore.ts:154-155`：`logout` 先 `await invoke('logout')` 后 `set(...)`，无 try/catch；`AppRoutes.tsx:474` 在 `vault-locked` 事件里 fire-and-forget 调用——后端已锁定，invoke 若 reject 则 `set` 被跳过，AuthGuard（`routes.tsx:33`）继续放行受保护路由，UI 进入半认证僵尸态。
修复方案：状态重置放 `finally`（先清前端状态再尽力通知后端）。

**P015 | 认证状态双 store**
路由守卫只读 `authStore.isAuthenticated`；`vaultStore.vaultState` 全应用无人读取（仅测试引用）；解锁有三种写入路径（authStore.login、LoginPage 直接 setState:303/:357、vaultStore.unlock），同步依赖 `vault-locked` 事件不丢失。
修复方案：删除 `vaultState` 死状态，lock/unlock 收敛为 authStore action，vaultStore 降级为薄封装或删除。与 P014 一并处理。

**复核发现（2026-08-01，状态降为 `[~]`）**：主体完成属实——`vaultStore.ts` 已删除、全仓无代码残留；`lock` 收敛为 `authStore.lock`（自带 try/catch），导航/自动锁定全部改用它。**但原问题点名的「三种写入路径」中，LoginPage 两条直接 setState 路径仍然存在**：PIN 解锁 `LoginPage.tsx:293` 与生物识别解锁 `LoginPage.tsx:347` 仍 `useAuthStore.setState({ isAuthenticated: true, ... })`，解锁写入路径仍有 3 条（`authStore.login` + 2 处裸 setState），未收敛为 authStore action。当前两条路径写入内容一致、暂无实际分歧，但与「收敛写入路径」的修复目标不符，修复说明也未记录此残留。建议补 `authStore.completeUnlock(acc)` 统一入口。

**修复记录（2026-08-01，状态恢复 `[x]`）**：新增 `authStore.completeUnlock(account, accounts?)` action（set isAuthenticated/currentAccount、accounts ?? 保留现有、error=null、isLoading=false），LoginPage PIN 解锁与生物识别解锁两条路径改 `useAuthStore.getState().completeUnlock(...)`——解锁写入路径收敛为 `authStore.login` + `authStore.completeUnlock` 两个 action，再无裸 setState。语义与原 setState 等价（PIN 路径 accounts 保留、生物识别路径显式传入 accs）；error/isLoading 清零属合理卫生处理（登录页错误走组件本地 state）。tsc/lint/Vitest 415 全绿，审查通过。

**P016 | 裸调 plugin-dialog（违反明确约定）**
`lib/dialog.ts:9` 明文禁止裸调，但 `ScanLocalPage.tsx:60`、`WatermarkPluginConfig.tsx:192`、`OcrQuickScanPopover.tsx:139` 三处直接 `open()`；开启「切后台锁定」时文件选择器触发 `visibilitychange:hidden` → Vault 被误锁，选完文件后流程失败。
修复方案：三处全部改 `openWithPause`。修复成本极低，建议优先处理。

### P1 死代码

**P017 | liquid-glass 整套死样式**
`components/liquid-glass/GlassCard.tsx` + `GlassCard.module.css` + `styles/liquid-glass.css` 全文件零引用（`main.tsx` 只导入 tokens/global/themes/animations）。**（2026-08-01 复核仍零引用，用户已确认删除：整个目录 + CSS 文件，独立 commit）**

**修复记录（2026-08-01，状态 `[x]`）**：三个文件已删除（`GlassCard.tsx`、`GlassCard.module.css`、`liquid-glass.css`）。删除前 grep 确认 `src/`+`main.tsx`+`App.tsx` 全零引用（含 CSS import），删除后 tsc/lint/Vitest 415 全绿。

**P018 | 死 barrel ×2**：`stores/index.ts`、`components/guide/index.ts` 零导入方。**（2026-08-01 复核仍零导入方，用户已确认删除，两个文件各一 commit 或与 P017 合并为「删除死文件」一组）**

**修复记录（2026-08-01，状态 `[x]`）**：两个文件已删除。内容为纯 re-export（stores/index 导出 5 个 store、guide/index 导出 6 个指南组件），删除不触碰底层模块（`@/stores/authStore` 等直接导入路径不受影响）。删除后 tsc/lint/Vitest 415 全绿（tsc 通过即证明无 `@/stores`/`@/components/guide` 目录导入残留）。

**P019 | 死导出符号 ×6**：`OCR_STORAGE_KEY`（ocrInstallStore.ts:27，遗留别名）、`TrashItem`（trash/types.ts:60）、`ProfileSummary`（ipc.ts:77）、`TemplateSyncStatus`（templateSync.ts:8）、`ImportSelection`（types/exportImport.ts:71）、`UserTemplateRaw`（types/template.ts:63）。删除对应行即可。

**P020 | 死 Tauri Commands ×18**（已注册进 `generate_handler!`，前端含 e2e/mock 之外零 invoke，Rust 侧无内部调用）：
`attachment_cleanup_orphans`（attachment.rs:963）、`get_current_account`（auth.rs:191）、`llm_chat`（llm/unified_chat.rs:24）、`llm_send_message`（llm/chat_http.rs:56）、`llm_find_guides`（llm/guide.rs:523）、`llm_persist_stats`（llm/stats.rs:243，连同 helper `persist_stats`:233）、`llm_delete_conversation`（llm/conversation.rs:159）、`guide_load_search_index`（llm/guide.rs:635）、`import_get_password_hint`（export_import/import.rs:71）、`inspect_backup`（fs.rs:111）、`mdns_advertise`（discovery.rs:414）、`search_advanced`（search/commands.rs:164）、`trash_get_retention`/`trash_set_retention`（object/snapshot.rs:108,128）、`object_get_template_sync_status`（object/mod.rs:1312）、`profile_save`/`profile_list`/`profile_delete`（profile.rs:38,86,103）。
修复方案：删除函数与注册项，一项一 commit 或按模块分组 commit。直接收益：缩小 IPC 攻击面。

**P021 | 死 Rust 函数 ×7**：`ensure_guide_embeddings_built`（llm/rag.rs:879，注释声称 "Called from app setup" 但实际未接线——需确认是遗漏还是有意）、`apk_downloaded_size`/`delete_apk_cache`（update.rs:149,161）、`PluginRegistry::from_path`（plugin/registry.rs:60）、`sync_to_remote_with_progress`（state/app_state.rs:409）、`persist_stats`（llm/stats.rs:233，随 P020）、`llm_context.clear_cache` 见 P035。
`[x]` 已修复：删除 `ensure_guide_embeddings_built`（功能被已注册命令 `llm_rebuild_guide_embeddings` 覆盖）、`apk_downloaded_size`、`delete_apk_cache`、`sync_to_remote_with_progress`；`persist_stats` 已随 P020 删除；`PluginRegistry::from_path` 实为集成测试 `tests/plugin_registry_update.rs` 使用，保留（报告误报）；`llm_context.clear_cache` 归入 P035。

**P022 | 未使用 npm 依赖 ×3**：`react-hook-form`、`@hookform/resolvers`（src/e2e 零 import）、`@tauri-apps/plugin-window-state`（窗口持久化全由 Rust 侧完成）。从 package.json 移除并 `npm install` 更新锁文件。
`[x]` 已修复：三依赖从 package.json 移除，`npm install` 后锁文件零残留；tsc/lint/Vitest 418 全部通过。

### P1 结构 / 重复

**P023 | `import_execute_internal` 431 行**（import.rs:234）：单函数混合策略解析、ID 映射、模板继承、附件还原、审计等多阶段。按阶段抽 `resolve_strategy`/`inherit_template`/`rewrite_ids`/`build_record` 等私有函数。
`[x]` 已修复：拆分为编排主函数 + 5 个阶段化私有函数——`decrypt_package`（阶段1 解密）、`rebuild_imported_templates`（阶段2 模板重建）、`build_import_record`（阶段4.1 记录构建，含 KeepBoth ID 重写）、`import_attachments`（阶段5 附件流式解密）、`import_preferences`（阶段6 偏好导入）；阶段3 KeepBoth 预映射与冲突检查保留在主函数并简化 match→if 条件（三分支语义等价）。新增 `use super::helpers::ManifestData` 导入。clippy/fmt/322 测试全部通过。

**P024 | `GlobalAttachmentManager` 1202 行**：全项目最大组件。拆工具栏/列表项/预览面板子组件，状态逻辑抽 hook（与 P061 一并设计）。

**P025 | `RecoveryReceiveDialog` 1001 行**：状态机抽 `useRecoveryReceive` hook，视图按阶段拆子组件。

**P026 | `OnboardingDialog` 989 行**：每步一个子组件 + 共享按钮组件（与 P048 hover 统一联动）。

**P027 | `ObjectWorkspacePage` 881 行**：抽 `useWorkspaceData` hook + 列表/详情子组件（与 P010/P011 同文件，建议同一轮处理）。

**P028 | profile preferences 写入块 ×6**：「load_profile→不存在则 new_with_id→解析 data→entry(preferences)→写 key→序列化→version+=1→save」约 20 行整块复制 6 份。抽共享函数 `update_profile_prefs(vault, account_id, |prefs| ...)` 于 `services/profile_prefs.rs`，全部替换完成。
`[x]` 已修复：新建 `services/profile_prefs.rs`，6 处调用点改为闭包写法（llm/mod.rs `save_config`+`save_api_key`、llm/stats.rs、llm/conversation.rs、llm_context.rs、settings.rs）。行为等价；附带改进：原 5 处 LLM 站点 `prefs["key"]=...`（IndexMut）在 preferences 为非对象时会 panic，现先替换为空对象消除潜在 panic。`template.rs::cleanup_legacy_json` 为条件保存（仅移除键才写），不纳入。clippy/fmt/322 测试全部通过。

### P2 安全

**P029 | 非常数时间比较**：`recovery.rs:175` `String != String` 提前退出构成理论计时侧信道，违反项目自身安全约定。改 `subtle::ConstantTimeCompare` 字节比较。随 P001 一并修复。

**P030 | capabilities 授权面过大**：`fs:allow-copy-file/stat/mkdir/remove` 全 `path:**`、`shell:allow-open url:**`、`allow-all-custom-commands`；叠加 `fs.rs:40-43` 桌面端 fs base 为整个 `$HOME`。收缩 fs 到数据目录+Desktop/Documents/Downloads，自定义命令按模块拆权限。

**复核注记（2026-08-01，判定通过）**：`shell:allow-open` 已收敛为 https/http/mailto/tel；`fs:allow-copy-file`/`stat` 收敛为 `$APPCACHE/$TEMP/$DESKTOP/$DOCUMENT/$DOWNLOAD`，`mkdir`/`remove` 收敛为 `$APPCACHE/$TEMP`。**偏差（已说明、可接受）**：`allow-all-custom-commands` 保留——实质是 `permissions/solo-soul/default.toml` 逐条枚举 205 个命令的白名单而非字面全允许，且有 `check_acl_consistency.py` 守一致性；commit `53523050` 已记录保留理由。「按模块拆权限」未做，属已说明的合理偏差。

**P031 | GUI 密码未 Zeroizing**：`auth.rs:37-46,77-89` 密码以普通 String 经 IPC；CLI 已用 `Zeroizing<String>`+`unlock_secure`，GUI 未对齐。改用 `unlock_secure` 并在命令层以 Zeroizing 接收。

**P032 | shell open 正则过宽**：`tauri.conf.json:78` 允许 `file://` 与 `/.+`。剔除本地路径项，本地文件预览走自定义命令+路径白名单（与 P004 联动）。

### P2 死代码

**P033 | 注册多余命令 ×9**（有内部调用方或仅测试引用，建议取消 `generate_handler!` 注册、函数保留或降级）：`sync_discover`（sync.rs:146）、`import_execute`（import.rs:191，recovery.rs:317 内部调用）、`schedule_saf_fallback_sync`/`cancel_saf_fallback_sync`（vault_directory.rs:341,348）、`encrypt_with_key`/`decrypt_with_key`/`generate_salt`/`constant_time_compare`（crypto.rs:40,56,106,119，仅自测）、`ocr_get_supported_languages`（ocr.rs:407）。
`[x]` 已修复：9 个命令从 `generate_handler!` 取消注册（函数保留供内部调用/测试）；`schedule_saf_fallback_sync`/`cancel_saf_fallback_sync` 两个命令包装函数取消注册后零调用，已连同删除（AppState 方法保留）。ACL 白名单同步移除 9 项。

**P034 | 一次性迁移工具**：`object_backfill_property_labels`/`object_backfill_property_fields`（object/mod.rs:710,745），确认迁移窗口已过后删除。
`[x]` 已修复：两命令前端零 invoke、Rust 内部仅注册引用，依赖 helper 仍被 `object_create`/`object_update`/`import.rs` 使用故无连带死代码，已删函数定义与注册项。**顺带修复 ACL 一致性**：`plugin_open_output_file`（P004 活命令）此前未登记白名单会被运行时 ACL 拒绝，已补登；并清理 P020/P034 已删命令的 20 条白名单遗留项（`desktop_check_update` 等 4 条实为 `check_acl_consistency.py` 正则遇 `#[cfg(...)]` 提前截断的误报，脚本已改平衡括号匹配修复）。ACL 检查零 ERROR 零 WARN，205 命令全部登记。

**P035 | P2 死代码组**：`mobile_not_supported_with`（commands/mod.rs:65，cfg(mobile) 零调用）删除；`llm_context.clear_cache`（services/llm_context.rs:86，注释称锁定时调用但实际未接线——需人工确认补接线或删除）；`set_active_sessions_for_test`（sync/manager.rs:225）加 `#[cfg(test)]`；`@tauri-apps/plugin-http` npm 包零 import，移除并确认 Rust 插件注册是否一并清理。
`[x]` 已修复：删除 `mobile_not_supported_with`；`clear_cache` 接线到 `vault.rs` 的 `lock` 命令（锁定时清 LLM 系统提示缓存）；`set_active_sessions_for_test` 已带 `#[cfg(test)]`（核验确认）；plugin-http 完整移除（npm 依赖 + Cargo.toml + lib.rs 注册 + capabilities `http:default` + ACL），锁文件零残留。

**P036 | Periodic 事件链路不可达**：`device_auto_sync.rs:148` `trigger_periodic` 零调用，`DeviceSyncEvent::Periodic` 分支（:173/:185）不可达。确认周期同步是否规划中功能，否则删触发器与事件变体。
`[x]` 已修复：核验确认**周期同步机制本身是活的**（Idle 状态 `interval.tick()` 直接触发 `Running(Periodic)`，不走事件通道），故保留 interval 机制与 `DeviceSyncSource::Periodic`；仅删除冗余的 `trigger_periodic` 方法、`DeviceSyncEvent::Periodic` 事件变体，并将两处 `Foreground | Periodic` match 分支简化为 `Foreground`。全仓库零残留引用，322 项测试全部通过。

**P037 | 约 60 处多余 export**：类型仅本文件使用但带 `export`（热点：`lib/plugin.ts` 13 处、`useNavigationItems.ts` 6 处、`lib/updater.ts` 5 处）。低优先级批量收敛，非功能性问题。

### P2 结构 / 重复

**P038 | LoginPage 846 行 + 查表重复**：三种登录路径+更新检查混一处；`biometryType→显示名` if-else 链 :111-120 与 :220-229 重复两份。按登录方式拆子组件，显示名改 `Record<string,string>` 查表。

**P039 | ImportSection/ExportSection 对称重复**：848/751 行，文件选择、加密设置、选项区块结构对称，抽共享子组件。

**P040 | AboutPage 834 行**：桌面/Android 双更新下载流程抽 `useUpdateChecker` hook。

**P041 | 4 个 750+ 行组件**：TemplateEditor(789)、ObjectDetailModal(783)、AttachmentViewer(763)、SyncPage(759)，分别按字段编辑行/标签页/附件类型/设备同步 hook 拆分。

**复核发现（2026-08-01，状态降为 `[~]`）**：实测行数——TemplateEditor 475 行 ✅、AttachmentViewer 732 行 ✅（均降至阈值以下）；**ObjectDetailModal 现 870 行（主组件函数体 754 行）、SyncPage 现 790 行（主组件函数体 749 行）**，虽分别抽了 ObjectDetailFieldsList 与 useSyncPage hook，主组件仍为 750+ 行单函数大组件，且两文件均超过报告基线行数（中间功能提交曾涨到 955/882）。「750+ 行大组件」问题在这两处未解决，需继续按标签页/阶段拆视图子组件。

**修复记录（2026-08-01，状态恢复 `[x]`）**：两处继续拆分到位——① ObjectDetailModal：指南数据提取为模块级纯函数 `buildDetailGuidePages`（-77 行）、删除确认对话框提取为 `ObjectDetailDeleteDialog` 子组件（-49 行），主组件函数体降至 **632 行**；② SyncPage：同步状态卡片（开关+自动同步+指纹+端口，约 180 行）提取为 `SyncStatusCard` 子组件，主组件函数体降至 **596 行**。两文件主组件均低于 750 行阈值。tsc/lint/Vitest 415 全绿，两轮审查通过。

**P042 | run_migrations 464 行**（solosoul-vault/migration.rs:31）：11+ 版本迁移顺序堆叠，每段重复 `pragma_table_info` 查列样板。抽 `has_column()` 帮助函数 + 每版本 `migrate_vN()`，主函数改版本列表驱动。

**P043 | lib.rs run 441 行**：setup 闭包内联 10+ 初始化步骤。拆 `init_logging`/`init_data_dir`/`init_state` 等步骤函数。

**P044 | 3 个 220+ 行函数**：`export_execute`（export.rs:217，258 行）按阶段抽私有函数；`trash_get_detail`（object/snapshot.rs:210，254 行）按 item_type 抽 preview 构建；`send_chat_stream`（llm/stream.rs:76，227 行）抽 `parse_sse_chunk`/`extract_delta` 纯函数。
`[x]` 已修复：
- **export.rs**：`export_execute`（原 315 行）拆为 6 个阶段函数——`validate_export_password`、`resolve_zip_path`、`collect_attachment_entries`（新增私有 `ExportAttachmentEntry` 结构，去掉未用的 file_name 字段）、`write_attachment_entries`（空 entries 早退，HKDF key 仅非空时派生）、`write_encrypted_extra`（统一 prefs/behavioral 两块加密附加文件写入）、`build_manifest_json`（`&ExportScope` 借代 + json! 引用序列化，返回 Value 而非永不失败的 Result）；`key` 参数类型按 `derive_hkdf_key` 签名修正为 `&[u8; 32]`。
- **snapshot.rs**：`trash_get_detail`（原 286 行）拆为 6 个阶段函数——`trash_remaining_days`、`trash_original_location`、`build_preview_properties`（template/object 双分支闭包原样搬迁）、`parse_trash_attachments`、`fetch_trash_child_items`（非 page 提前 return）、`extract_trash_metadata`；辅助函数引用 `&VaultStore`/`&TrashItem`。
- **stream.rs**：`send_chat_stream`（原 270 行）拆为 `extract_delta_text` 纯函数（消除循环与尾部两处 delta 提取重复）+ `handle_sse_stream`/`handle_json_response` 两个处理器，主函数降为请求构建+Content-Type 分发；`conversation_id` 由 `String` 改 `&str`（`clone()`→`to_string()`），`use futures::StreamExt` 移入 SSE 处理器。
行为等价；fmt/clippy/322 测试全部通过，两轮审查通过。

**P045 | 深层嵌套 ×5**：`helpers.rs:173-232` rewrite_id_references 三连 if-let（抽 `rewrite_str_ref`）；`import.rs:394-412` 模板 labels 合并 5 层（提前 return/and_then）；`profile.rs:191-205` 6 层（`iter_mut().find()` 组合子）；`plugin.rs:80-107` 5 层（抽 `migrate_seed_bindings` 用 `?`）；`llm/stream.rs:257-302` SSE 尾部 6 层（抽子函数+提前 continue）。
`[x]` 已修复：
- **helpers.rs**：`rewrite_id_references` 三连 if-let 抽 `rewrite_str_ref`（let-else 双早退）+ `rewrite_str_array_ref`（let-else + continue），主函数改 `match key.as_str()` 三分支，RelationProperty targetId/id/objectId 循环语义保留。
- **import.rs**：模板 labels 合并 5 层抽 `merge_labels_into(tpl, existing)` 助手（`if let (Some, Some)` 双模式）；match 简化为三臂。
- **profile.rs**：`profile_update_field` 5 层 if/for 改两层 `iter_mut().find()` 组合子（sections→section→fields→field），find 停止语义与 break 等价。
- **plugin.rs**：`plugin_install` 5 层 if-let 链抽 `migrate_seed_bindings(state, plugin_id)`，let-else 链保持原告警不阻断语义。
- **stream.rs**：SSE 尾部 6 层抽 `handle_remaining_data` 助手，`[DONE]`/解析失败早退镜像原条件跳过。
行为等价；fmt/clippy/322 测试全部通过，审查通过。

**P046 | cleanup_orphan_attachments 双份**：`attachment.rs:983-1027` 整体复制 `objects.rs:945-999`，仅多 audit/auto_sync 触发。GUI 改调 core 实现后再做日志与同步；附带的 `map(|mut d| d.next().is_none()).unwrap_or(false)` 可顺手改 `is_ok_and`。
`[x]` 已修复：GUI 端重复实现整体已在 **P020** 连带删除（死命令 `attachment_cleanup_orphans` 删除时移除，附件 `attachment.rs:983-1027` 已不存在）；core 版本仍被 CLI `solosoul_cli/src/commands/attachment.rs:308` 使用，非死代码。本轮补齐报告附带的 `is_ok_and` 改进：`objects.rs:981` 空目录判断 `map(|mut d| d.next().is_none()).unwrap_or(false)` → `is_ok_and(|mut d| d.next().is_none())`（语义等价，Err→false 一致）。GUI 遗留 `#[cfg(test)]` 的 `load_all_referenced_attachment_ids` 测试助手保留（测试 vault 批量 API，core 版路径不同）。fmt/clippy/core 145 测试全部通过，审查通过。

**P047 | watermark 注册闭包重复**：`register.rs:763-812` vs `:838-890` 除 `apply_to_image`/`apply_to_pdf` 一行外完全相同；抽 `register_watermark_fn(linker, name, apply)` 泛型注册；read_string 参数校验样板 20+ 次可抽帮助函数/宏。**注意：本项依附于 P012 的取舍结果，应先定 P012 方向再动。**

**决策记录（2026-08-01）**：P012 已定为方向 B（统一到 crate），且重复代码在两侧同构存在（本地 `register.rs` 与 crate `host.rs:937-1008/1012-1083`）。先在本地做会在方向 B 第④步删除本地 host 时被整体丢弃，故**本项并入 P012 第③步**，在统一后的 crate `host.rs` 上只做一次，顺带统一 `write_u32`/`write_handle` 命名。

**P048 | 手写 hover**：项目已有带 CSS hover 的 `ui/Button`，但大量页面仍内联 style + 双事件手写 hover，样式不一致且无法随主题统一调整。

**决策记录（2026-08-01，用户已确认：分批视觉等价重构）**

*规模核实（2026-08-01 实测）*：`onMouseEnter` 119 处/54 文件、`onMouseLeave` 128 处/57 文件；其中内联样式改写（`currentTarget.style`）471 处/49 文件。注意区分：`useLongPress.ts`、`ui/Card.tsx` 等处的鼠标事件是行为逻辑（长按/卡片交互），不在本项范围。

*为何不做全量 Button 替换*：`ui/Button` 仅 6 个 variant，而各页面手写 hover 的颜色/圆角/位移高度异构（如 `DataManagementPage.tsx` 34 处、`PageGuide.tsx` 26 处、`TrashDetailPanel.tsx` 36 处样式各不相同），直接全量替换必然产生视觉差异。

*执行策略（视觉等价重构）*：
1. **按目录分批**（每批 5-10 文件）：settings → components/guide → components/ocr → components/onboarding → components/template → components/recovery → components/layout → pages 其余。
2. 每处二选一：**样式匹配处迁移 `ui/Button`**（必要时补 variant）；**不匹配处将内联 hover 改为同文件 CSS module 的 `:hover` 类，颜色/圆角/位移值逐一保留**，保证视觉零差异。
3. 每批验证：`npx tsc --noEmit` + `npm run lint` + `npm run test` + 相关 Playwright e2e；一批一 commit。
4. 行为性鼠标事件（长按、tooltip 显隐等）不动，仅迁移纯样式 hover。

*第一批实施（2026-08-01，已提交）*：实际走**既有 `interactive-*` 工具类**路线（比原计划的 `ui/Button`/CSS module 更轻）——`src/styles/animations.css` 已有 `.interactive-accent/-light/-toolbar/-danger/-icon` 等共享类，本次新增 4 个变体（`.interactive-danger-soft` 红底、`.interactive-row` 行 hover、`.interactive-nav` 箭头、`.interactive-color-accent` 纯色）并给全部 11 个类的 `:hover` 加 `:not(:disabled)`（与 JS `if (loading) return` 守卫等价，disabled 时不再 hover）。迁移 3 文件 19 处：`DataManagementPage.tsx` 6 处（view-breakdown→accent，5 工具栏按钮→toolbar）、`SyncPage.tsx` 5 处（review→danger-soft，QR/scan/manual→toolbar+disabled 守卫，activity→color-accent）、`TrashDetailPanel.tsx` 8 处（back→icon，close→accent，tab1/tab2→toolbar/danger+selected-* 组合，行/子行→row，快照箭头→nav）。内联 `borderWidth/borderStyle` 保留（类仅提供 border-color）。**SyncPage sync 开关**（border/color 依赖 syncEnabled 动态状态）暂缓，留后续批次。tsc/lint/Vitest 415 全绿，两轮审查通过。

**P049 | 筛选 chip 块 ×5**：约 45 行「isActive 三态 style+hover 双事件+map」重复 5 处，抽 `FilterChipGroup({options,value,onChange})`。

**P050 | ObjectEditorPage 校验 switch 表驱动化**：:300-360 六个 case 同一「正则失败→写 errors」模式，改 `Record<PropertyType,{re,hintKey}>` 查表循环，顺带消除 5 层嵌套。

### P2 性能

**P051 | embedding 逐条事务**：`rag.rs:794-808,942` 重建时逐条 `save_guide_embedding`（每次抢锁+autocommit+fsync）。加批量接口：单事务+prepared statement 循环绑定。

**P052 | 回收站批量删除串行 IPC**：`trashStore.ts:137-140` 循环内顺序 await。后端加 `trash_permanent_delete_batch` 或前端 `Promise.all`。
`[x]` 已修复：按报告二选一中的最小改动方案，`permanentDelete` 改 `Promise.all(trashIds.map(...))` 并发化。并发安全已核验：后端 rusqlite 连接在 `Mutex<Option<Connection>>`（storage.rs:26）内串行化，并发 invoke 无数据竞争；后端无 batch 命令（已确认）。语义差异（串行首错中止后续不执行 vs Promise.all 已发起均执行）已注释说明并接受；任一失败整体 reject 与串行首错中止行为一致，本地列表保持至下次刷新。tsc/lint/Vitest 415 全部通过，审查通过。

**P053 | loadCustomPages N+1**：`settingsStore.ts:328-355` 每页单独 `object_get` 拉 description（已 Promise.all 并发，页面数少）。可让 `object_list` 附带 description 或加批量命令。
`[x]` 已修复：核验发现 solosoul-vault 的 `ObjectSummary`（lib.rs:263）已含 `properties: serde_json::Value` 字段，且 `list_objects` SQL 已 SELECT properties 列并 `serde_json::from_str(&decrypted_props)` **全量解密反序列化**（storage.rs:2356,2415，非截断）——`object_list` 本就返回每个页面的完整 properties，无需新增批量命令。改动：类型注解补 `properties?: Record<string, unknown>`，`objects.map` 同步化（去掉 Promise.all + 每页 `object_get` + logger.warn 捕获块），description 直接从 `o.properties?.description` 读取，并补 `!o.isDeleted` 守卫恢复原实现「deleted 页面 description 恒为 undefined」语义（审查发现的行为漂移已修正）。N+1 IPC 完全消除。tsc/lint/Vitest 415 全部通过，两轮审查通过。

**P054 | 附件批量下载串行**：`GlobalAttachmentManager.tsx:419-425`、`AttachmentViewer.tsx:343-361` 逐条 invoke+文件拷贝（同文件删除/恢复已有 batch 命令，下载没有）。加后端批量命令或并发化。

**P055 | 其余整店订阅**：`ObjectEditorPage.tsx:36`、`TrashPage.tsx:79`、`TemplateManagerPage.tsx:75`、`ObjectDetailModal.tsx:136` 多处 `useXxxStore()` 裸调用。已全部改为分字段 selector：ObjectEditorPage（objectStore 4 字段 + templateStore 2）、TrashPage（trashStore 16 字段 + getTemplate）、TemplateManagerPage（templateStore 8 字段 + settingsStore 2）、ObjectDetailModal（templateStore 2）。函数字段引用稳定不再触发重渲，值字段按引用比较（store 全部不可变更新），无无限重渲风险；变量名不变故 useEffect 依赖数组无需改。tsc/lint/Vitest 415 全绿，审查通过。

**P051 | embedding 重建逐条事务**：`rag.rs:794-808` 重建循环逐条 `save_guide_embedding`（每次抢锁 + autocommit + fsync）。已在 vault crate 新增批量方法 `save_guide_embeddings`（单次 conn.lock + 单事务 + prepared statement 循环绑定 + commit），rag.rs 重建改为先 collect 全部 chunk 再一次性批量写入；错误消息带 chunk 索引恢复逐条可调试性。单条 `save_guide_embedding` 保留（pub API，测试仍用）。cargo check/fmt/clippy 全绿，guide_embedding 4 测试通过，两轮审查通过。

**P049 | 筛选 chip 块 ×5**：`TrashPage`（×2）、`OperationLogPage`、`TemplateManagerPage`、`SampleTemplateGallery` 的「isActive 三态 style + hover 双事件 + map」重复块（约 45 行 ×5）收敛为共享组件 `src/components/ui/FilterChipGroup.tsx`（泛型，props = options/value/onChange/toggle/size/radius/gap/fontWeight/testId）。激活态 accent 边框+淡色底+阴影，非激活 hover accent 描边预览；`toggle` 模式支持 OperationLog 的「点击激活项取消」语义；`testId` 透传保留 SampleTemplateGallery 测试依赖（`page-filter-*`）；OperationLog 保留 radius 8/gap 4 原观感，SampleGallery 保留 caption 字号。字重统一 500（P049 收敛目标，OperationLog 原 500、其余原浏览器默认）。tsc/lint/Vitest 415 全绿，审查通过。

**P050 | ObjectEditorPage 校验 switch 表驱动化**：动态组子字段校验与普通字段校验各一份「switch 六 case」（email/url/phone/date/number）同一模式，且动态组版存在 5 层嵌套。已抽模块级常量 `FIELD_TYPE_VALIDATORS: Partial<Record<PropertyType, { hintKey, isValid }>>`（isValid 返回布尔），两处校验改查表（`validator && !validator.isValid(strVal)` 时写 error），URL 校验保留「无协议头补 https:// 再 new URL」原语义，unknown 类型短路跳过与原 switch 无 case 匹配一致，hintKey 路径 `editor:validation_*` 不变。tsc/lint/Vitest 415 全绿，审查通过。
`[x]` 已修复：两处批量下载按报告「并发化」选项改为 `Promise.allSettled` 并发（各附件独立 IPC + 独立目标文件，并发安全；allSettled 不因单项失败整体 reject，successCount 语义与串行一致）。① `useAttachmentManager.ts handleBatchDownload`：桌面端串行 for-await → filter（去无 path 项）+ map + allSettled；② `AttachmentViewer.tsx handleBatchDownload`：移动端 SAF（`attachment_export_tree_uri`）与桌面端（`attachment_download`）双串行循环合并为 downloadTasks 数组 + allSettled，平台检测 `isMobilePlatformSync()` 上提为 map 外单次求值（审查建议采纳）。tsc/lint/Vitest 415 全部通过，两轮审查通过。

**P055 | 其余整店订阅**：`ObjectEditorPage.tsx:36`、`TrashPage.tsx:79`、`TemplateManagerPage.tsx:75`、`ObjectDetailModal.tsx:136` 分字段 selector（负载小于 P010）。

### P2 架构 / 规范

**P056 | objectStore trash 死切片**：`objectStore.ts:184-220` 的 `trashObjects`/`loadTrashObjects`/`restoreObject`/`purgeObject` 无 UI 引用，且与 trashStore 调不同后端命令（`object_restore` vs `trash_restore`）操作同一后端数据。删除该切片及对应测试。
`[x]` 已修复：删除 objectStore.ts 的 `trashObjects`/`loadTrashObjects`/`restoreObject`/`purgeObject`（interface+实现+clearOnVaultLock 中的 `trashObjects:[]`），连带清理仅 restoreObject 使用的 i18next import；测试文件删除 `trash lifecycle` describe 块（3 个测试）、clearOnVaultLock 测试移除 trashObjects 引用、并清理失效的 i18next mock。全仓库零残留引用（AttachmentToolbar 的 `trashObjects:number` 为无关的 summary 计数字段）。tsc/lint/Vitest 415 全部通过，审查通过。

**P057 | updateObject 不同步列表**：`objectStore.ts:158-169` 只更新 `currentObjectCache`，`objects` 摘要保持旧值（现靠页面重挂载掩盖）。成功后同步更新 `objects` 对应项。
`[x]` 已修复：`updateObject` 成功后的 `set` 新增 `objects: s.objects.map(...)`，id 匹配项同步 name/sensitivityLevel/updatedAt/templateId/templateType/templateHash/contractTypeId/tags/propertyLabels（`...o` 展开保留 collectionType/createdAt/sectionType），非匹配项保持原引用；测试增强为 setState 两条对象后断言同步与隔离。tsc/lint/Vitest 415 全部通过，审查通过。

**P058 | profileStore 死代码+不同步**：`loadSection`/`updateField`（:75-98）无调用方且 `updateField` 写后端后不更新本地。删除或补同步并接入调用方。
`[x]` 已修复：全仓库确认 `loadSection`/`updateField` 零外部调用（仅 profileStore.ts 自引用，无测试文件），选择删除（零调用方下接入同步无意义）；接口+实现一并删除，`ProfileSectionData`/`RawProfileSection` 类型仍被 `loadProfile` 使用故保留。**顺带修复**：DatePicker 测试时间依赖 flaky（系统时间跨月到 8 月时 `getByText('Aug')`/`getByText('Feb')` 同时命中月份触发器与下拉选项，`Found multiple elements`），改用 `getByText(..., { selector: '[data-dd-value="..."]' })` 限定到下拉选项按钮（DropdownSelect 选项带 `data-dd-value`，触发器没有），消除月份/年份边界依赖。**观察记录（非 P058 范围）**：删除 store 函数后 Rust 命令 `profile_get_section`/`profile_update_field` 仍注册于 lib.rs:481-482，成为前端孤儿命令（后者另有 Rust 单测 `test_vault_profile_update_field_logic`），属 P020 风格后继清理候选。tsc/lint/Vitest 415 全部通过，审查通过。

**P059 | invoke 缺 catch ×3**：`HistoryPage.tsx:36-38`、`HistoryViewer.tsx:485-487`、`vaultStore.ts:40-43`（导航栏直接作 onClick）。补 `.catch` 错误提示；可抽公共 hook。
`[x]` 已修复：① HistoryPage `snapshot_list` 链补 `.catch`（`showToast` error + `t('common:history_load_failed', fallback)`）；② HistoryViewer `snapshot_list` 链补 `.catch`（新增 `useUiStore` showToast，同款提示）；③ 同文件 SnapshotCard 的 `snapshot_get_data` 补 `.catch`（`logger.warn` 记录，`snapData` 保持 null 优雅降级）；④ vaultStore.ts 项已随 **P015** 消除（文件删除、锁定收敛为 `authStore.lock`，自带 try/catch）。两处 useEffect 按项目既有惯例补 `// eslint-disable-next-line react-hooks/exhaustive-deps`（`showToast`/`t` 稳定引用，仅 objectId 变化重载）。tsc/lint（0 警告）/Vitest 415 全部通过，审查通过。

**P060 | PluginResultPanel 直接 FS+吞错**：:344,360 组件内 `copyFile` 且 `catch{}` 静默。下沉 Rust command 或 lib 封装，失败 toast 提示（与 P004 同文件，建议同轮处理）。
`[x]` 已修复：新增 Rust 命令 `plugin_copy_output_file(output_dir, path, dest_dir, file_name)`——复用 P004 的 canonical 包含校验（源必须位于插件 output_dir 内）+ `file_name` 严格净化（非空/非`.`/`..`、不含 `/` 与 `\`，防 Windows 路径遍历）+ `dest_dir` 必须存在目录；`std::fs::copy` 错误返回中文消息。**抽取共享助手 `resolve_output_file(output_dir, path) -> Result<PathBuf, String>`**，`plugin_open_output_file`（P004）与 `plugin_copy_output_file` 共用，消除 15 行重复（审查建议采纳）。前端 PluginResultPanel：移除 plugin-fs `copyFile` 与 `join` 导入，改 `dirname`/`basename`；`handleDownload`/`handleDownloadSelected` 改 `invoke('plugin_copy_output_file')`，成功/失败均 `showToast` 提示（新增 `useUiStore`）。已注册 lib.rs + ACL 白名单 default.toml（check_acl_consistency.py OK，197 命令）。cargo check/fmt/clippy、tsc/lint、Vitest 415 全部通过，两轮审查通过。

**P061 | GlobalAttachmentManager 重业务下沉**：12 处直接 invoke+多层聚合统计（:556-589）在组件内。数据编排下沉 store/lib hook（与 P024 同设计）。
`[x]` 已修复：**由 P024 连带完成**（核验确认，无需额外改动）。页面 `GlobalAttachmentManager.tsx` 已从报告时的 1202 行降至 **311 行**、**零直接 invoke**；数据加载（`loadData`/`attachment_list_all`）、附件树遍历（`displayPages`/`allVisibleKeys`）、10 处 invoke（open/rename/soft_delete/download/restore/delete/batch_*）、聚合统计（`summaryStats`）全部下沉至 `src/hooks/useAttachmentManager.ts`（552 行），页面降为纯编排层（子组件 + ConfirmDialog + 指南配置）。tsc/lint/Vitest 415 通过（无代码改动，仅核验）。

**P062 | 设置四副本**：zustand+localStorage+ui_preferences.json+vault 加密 preferences（settingsStore.ts:154-307），为「登录前主题正确」的有意设计但任一写入遗漏即产生主题跳变 bug。补「写入路径矩阵」注释；长期收敛单副本+登录前只读快照。
`[x]` 已修复：settingsStore.ts 顶部新增「四副本写入路径矩阵」注释块——① zustand store（loadUiPreferences/loadSettings/updateSetting/addCustomPage/removeCustomPage/clearOnVaultLock）、② localStorage（loadUiPreferences Step2 setItem ST_UI_PREFS、updateSetting(language) setItem i18nextLng）、③ ui_preferences.json 明文（loadSettings 同步块 + updateSetting(language) 走 ui_update_preference）、④ vault 加密 preferences（updateSetting → user_data_update_preference、loadCustomPages 迁移清理 customPages）；并标注读取优先级（登录前 ②③ → 解锁后 ④ 覆盖）与语言实际生效路径。审查逐项核对矩阵与实现一致。tsc/lint/Vitest 415 全部通过，审查通过。

**P063 | panic=abort 认知**：`tauri/Cargo.toml:73` release `panic="abort"`，任何遗漏 panic 都是无清理整进程终止。当前生产 unwrap 仅 2 处可证安全，风险低；作为评审认知保留，无需改动（可标记为「设计如此」）。
`[x]` 已处理（设计如此，无改动）：核验确认全 workspace 的 unwrap/expect 命中（local_embed.rs、profile.rs、log.rs、embed_model.rs、discovery.rs、window.rs、auth.rs、sync.rs、plugin/registry.rs）**全部位于各自 `#[cfg(test)] mod tests` 块内**（行号逐一对照），生产路径仅报告基线 settings.rs:73,80 两处可证明非空，无新增滥用。`panic="abort"` 作为既有发布配置保留，维持认知。

**复核注记（2026-08-01，判定通过）**：结论属实——生产路径唯一命中为 `settings.rs:74,81`（`remove_with_retry` 内，两次 unwrap 前 `last_err` 均已赋值，可证安全）；报告行号 73,80 为 off-by-one 小偏差，不影响结论。另：`remove_with_retry` 带 `#[cfg(any(target_os = "android", test))]`，Android 上属生产代码，「生产路径仅两处」的表述仍成立。

---

## 暂缓事项说明（依据流程约束：不自动执行破坏性操作）

- ~~P017 / P018（删除死文件）~~ **已解除暂缓**：2026-08-01 复核零引用后用户确认删除，待执行。
- 以下「疑似未接线而非真死代码」的函数已在复核轮次 1 判定完毕：`ensure_guide_embeddings_built`（已删，功能被 `llm_rebuild_guide_embeddings` 覆盖）、`clear_cache`（已接线到 lock/logout）、`object_backfill_*`（已删，迁移窗口已过）、`trigger_periodic`（已删，周期同步机制本身保留）。
- P012 方向 B 第④步删除本地插件模块（2894 行）属大规模代码删除，已随方向 B 决策一并确认。

## 剩余工作执行建议（2026-08-01 更新）

1. **安全残留（小改动，优先）**：~~P004（host 侧盖章真实 `output_dir`，连带 `plugin_copy_output_file`）~~ 已修复（2026-08-01）；~~P003（GUI 导入路径写回 `safe_name`）~~ 已修复（2026-08-01）。
2. **快速收敛（几行改动）**：~~P010（`useObjectWorkspaceData.ts:141` 分字段化）~~ 已修复（2026-08-01）；~~P015（LoginPage 两处 setState 收敛为 `authStore.completeUnlock(acc)`）~~ 已修复（2026-08-01）。
3. **死文件删除**：P017 + P018（已确认，独立 commit）。
4. **P041 继续拆分**：~~ObjectDetailModal、SyncPage 按标签页/阶段拆视图子组件至阈值以下~~ 已修复（2026-08-01，主组件 632/596 行）。
5. **P011 收尾**：~~GlobalAttachmentManager 附件树虚拟化或分页~~ 已修复（2026-08-01，顶层页面列表「加载更多」分页）。
6. **P012 方向 B（单独一轮，含 P047）**：按详情 6 步计划执行，每步独立 commit，第⑥步全量回归。
7. **P048 分批重构**：按详情策略逐批执行，可与上述小项穿插。

---

## 复核记录（复核轮次 1，2026-08-01）

针对开发者声称已完成的 58 项（`[x]`/`[~]`），按安全、Rust 性能、前端 store/性能、前端架构、死代码、结构重构六个维度做了独立代码核验（对照当前工作区代码与 git commit 逐条验证）。

### 复核结论

- **通过：52 项**——修复真实存在、解决原始问题、说明与代码一致。包括全部 P0/P1 项的主体：P001（mDNS TXT 已剔除 pin/nonce，QR/手动输入/LAN 三条带外链路完整可用）、P002、P005、P006–P009、P013（进程内单 ServiceDaemon，sync_enable 注入共享实例）、P014、P016、P019–P028、P029、P031–P040、P042–P046、P049–P063（除下方降级项）。
- **有出入（降级为 `[~]`）：5 项**
  - **P003**：GUI 导入路径 `import.rs:734` 元数据 `file_name` 未写回 `safe_name`（core 路径与 host 兜底已修，当前因纵深防御不可利用，但源头净化漏了一半）。
  - **P004（安全，建议优先跟进）**：`plugin_open_output_file`/`plugin_copy_output_file` 的包含校验基准 `outputDir` 取自插件可控 payload，恶意插件填 `outputDir:"/"` 即可绕过校验，经 `opener::open` 打开/执行任意本地文件——原始沙箱逃逸威胁对恶意插件依然成立。需 host 侧盖章真实 `output_dir`。
  - **P010**：`useObjectWorkspaceData.ts:141` 残留 `useTemplateStore()` 裸整店订阅。
  - **P015**：LoginPage PIN（`LoginPage.tsx:293`）与生物识别（`:347`）两条解锁路径仍直接 `useAuthStore.setState`，未收敛为 authStore action。
  - **P041**：ObjectDetailModal（870 行/主组件 754 行）与 SyncPage（790 行/主组件 749 行）抽取后仍为 750+ 行单组件，且超过报告基线行数。
- **未修复：0 项**（无虚假完成声明）。

### 说明与代码/流程不一致的注记（不改变判定，已记入各条目详情）

1. **P002**：commit/说明声称的「落点断言」（resolve_path+is_path_under_workspace）代码中不存在；白名单已闭环，属说明夸大。
2. **P005**：附属项「UI 明确风险提示」未实现。
3. **P006–P009**：正文缺修复说明段落；且四项改动混入 commit `61271ba7`（message 未提及），违反「一项一提交」追溯原则。
4. **P030**：`allow-all-custom-commands` 保留（commit `53523050` 已记录理由，实质为逐命令白名单，可接受）。
5. **P063**：生产 unwrap 行号实为 `settings.rs:74,81`（报告写 73,80，off-by-one）。

### 后续建议

- 优先跟进 **P004**（信任锚未闭环的安全问题）与 **P003** 的 GUI 路径补改，两者都是小改动。
- P010、P015 残留均为几行内的收敛改动。
- P041 两个组件需继续拆分方能达标。
- 待修复项不变：P012（插件双份实现）、P017/P018（暂缓删文件）、P047、P048。
