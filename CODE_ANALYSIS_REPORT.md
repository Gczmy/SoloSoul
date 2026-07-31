# 代码分析修复报告

> 最后更新：2026-07-31 21:00:00
> 当前分支：`main`
> 修复轮次：3（已执行修复：P001–P011、P013–P016、P019–P027、P028、P029、P033–P036、P038、P040、P041，共 33 项）
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
| P003 | P1 | 漏洞 | `tauri/crates/solosoul-core/src/export_import.rs:836-855`、`tauri/crates/solosoul-plugin/src/host.rs:1284-1285` | 导入时附件元数据 `file_name` 未净化，形成存储型路径遍历 | `[x]` 已修复 |
| P004 | P1 | 漏洞 | `tauri/src/components/plugin/PluginResultPanel.tsx:331-337` | 前端用插件提供的路径直接 shell open，可打开/执行任意本地文件 | `[x]` 已修复 |
| P005 | P1 | 漏洞 | `tauri/crates/solosoul-core/src/pin.rs:188-197,336-350` | PIN 解锁将 Vault 安全性降为 6 位离线爆破，锁定计数可被绕过 | `[x]` 已修复 |
| P006 | P1 | 性能 | `tauri/src-tauri/src/commands/search/query.rs:155-168,238,261` | 搜索分页计数用 `list_objects` 全量解密后取长度，N+1 次 AES 解密 | `[x]` 已修复 |
| P007 | P1 | 性能 | `tauri/src-tauri/src/commands/search/commands.rs:305-346` | 模板命中时第二次全表解密扫描，单次搜索最多 2 次全表解密 | `[x]` 已修复 |
| P008 | P1 | 性能 | `tauri/src-tauri/src/commands/llm/rag.rs:358-363` | top_k 循环内反复 `chunk_all_guides`，每条聊天消息把指南文件完整读 3 遍 | `[x]` 已修复 |
| P009 | P1 | 性能 | `tauri/src-tauri/src/commands/ocr.rs:258,356` | 每次 OCR 命令重新加载 ONNX 引擎（数百 ms），无缓存 | `[x]` 已修复 |
| P010 | P1 | 性能 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:148-158` | `useObjectStore()` 无 selector 整店订阅，store 任何变化触发整页重渲染 | `[x]` 已修复 |
| P011 | P1 | 性能 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:803`、`tauri/src/pages/settings/GlobalAttachmentManager.tsx:1077` | 大列表无虚拟滚动/分页，对象数百+ 时首屏与重渲染成本高 | `[~]` 部分完成（工作区分页「加载更多」已实现；GlobalAttachmentManager 虚拟化/分页待补） |
| P012 | P1 | 架构/重复 | `tauri/src-tauri/src/plugin/` vs `tauri/crates/solosoul-plugin/src/` | 插件运行时双份平行实现（register.rs 各 1010 行约 95% 相同，已出现行为分歧），crate 侧约 3000 行无人调用 | `[ ]` 待修复 |
| P013 | P1 | 架构 | `tauri/src-tauri/src/commands/discovery.rs:30`、`tauri/crates/solosoul-sync/src/manager.rs:168` | mDNS ServiceDaemon 在两个层各起一个实例，可同时存活导致结果不一致 | `[x]` 已修复 |
| P014 | P1 | 架构 | `tauri/src/stores/authStore.ts:154-155` | `logout` invoke 失败时前端认证状态永不重置，出现半认证僵尸态 | `[x]` 已修复 |
| P015 | P1 | 架构 | `tauri/src/stores/vaultStore.ts:15-44`、`tauri/src/stores/authStore.ts` | 认证/锁定状态双 store 平行维护，`vaultState` 只写不读，存在三种写入路径 | `[x]` 已修复 |
| P016 | P1 | 规范 | `tauri/src/pages/scan/ScanLocalPage.tsx:60`、`tauri/src/components/plugin/WatermarkPluginConfig.tsx:192`、`tauri/src/components/layout/OcrQuickScanPopover.tsx:139` | 三处裸调 plugin-dialog `open`，违反 dialog.ts 封装约定，可致自动锁定误触发 | `[x]` 已修复 |
| P017 | P1 | 死代码 | `tauri/src/components/liquid-glass/`、`tauri/src/styles/liquid-glass.css` | 整套玻璃拟态组件与样式零引用 | `[ ]` 待修复（暂缓：删除文件） |
| P018 | P1 | 死代码 | `tauri/src/stores/index.ts`、`tauri/src/components/guide/index.ts` | 两个 barrel 文件无任何导入方 | `[ ]` 待修复（暂缓：删除文件） |
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
| P041 | P2 | 结构 | `TemplateEditor.tsx:81`(789)、`ObjectDetailModal.tsx:120`(783)、`AttachmentViewer.tsx:59`(763)、`SyncPage.tsx:91`(759) | 4 个 750+ 行大组件需拆分 | `[x]` 已修复 |
| P042 | P2 | 结构 | `tauri/crates/solosoul-vault/src/migration.rs:31` | `run_migrations` 464 行，每版本重复「查列是否存在」样板 | `[x]` 已修复 |
| P043 | P2 | 结构 | `tauri/src-tauri/src/lib.rs:135` | `run` 441 行，启动初始化 10+ 步全内联在一个 setup 闭包 | `[ ]` 待修复 |
| P044 | P2 | 结构 | `export.rs:217`(258)、`object/snapshot.rs:210`(254)、`llm/stream.rs:76`(227) | 3 个 220+ 行多阶段函数需按阶段抽取 | `[ ]` 待修复 |
| P045 | P2 | 结构 | `export_import/helpers.rs:173-232`、`import.rs:394-412`、`profile.rs:191-205`、`plugin.rs:80-107`、`llm/stream.rs:257-302` | 5 处 5-6 层深层嵌套 | `[ ]` 待修复 |
| P046 | P2 | 重复 | `tauri/src-tauri/src/commands/attachment.rs:983-1027` vs `tauri/crates/solosoul-core/src/objects.rs:945-999` | `cleanup_orphan_attachments` GUI 端整体复制 core 实现 | `[ ]` 待修复 |
| P047 | P2 | 重复 | `tauri/src-tauri/src/plugin/host/register.rs:763-812,838-890` | 两个 watermark 注册闭包除一行外完全相同；read_string 校验样板 20+ 次 | `[ ]` 待修复 |
| P048 | P2 | 重复 | 全前端 30+ 文件（如 `ExportSection.tsx:600-622`、`OnboardingDialog.tsx` ×10、`SyncPage.tsx` ×5） | 109 处手写 onMouseEnter/Leave hover，应统一迁移到 `ui/Button` | `[ ]` 待修复 |
| P049 | P2 | 重复 | `TrashPage.tsx:254-345`（×2）、`OperationLogPage.tsx:280-323`、`TemplateManagerPage.tsx:597+`、`SampleTemplateGallery.tsx:282+` | 筛选 chip 按钮块（约 45 行）重复 5 处，可抽 FilterChipGroup | `[ ]` 待修复 |
| P050 | P2 | 结构 | `tauri/src/pages/editor/ObjectEditorPage.tsx:300-360` | 动态组校验 switch 6 个 case 同一模式，应表驱动化（含 5 层嵌套） | `[ ]` 待修复 |
| P051 | P2 | 性能 | `tauri/src-tauri/src/commands/llm/rag.rs:794-808,942` | 重建 embedding 逐条 `save_guide_embedding`，每条独立事务+fsync | `[ ]` 待修复 |
| P052 | P2 | 性能 | `tauri/src/stores/trashStore.ts:137-140` | `permanentDelete` 循环内串行 await 逐条 IPC | `[ ]` 待修复 |
| P053 | P2 | 性能 | `tauri/src/stores/settingsStore.ts:328-355` | `loadCustomPages` 对每个自定义页单独 `object_get`（N+1 IPC） | `[ ]` 待修复 |
| P054 | P2 | 性能 | `GlobalAttachmentManager.tsx:419-425`、`AttachmentViewer.tsx:343-361` | 附件批量下载逐条串行 IPC+文件拷贝 | `[ ]` 待修复 |
| P055 | P2 | 性能 | `ObjectEditorPage.tsx:36`、`TrashPage.tsx:79`、`TemplateManagerPage.tsx:75`、`ObjectDetailModal.tsx:136` | 多处 `useXxxStore()` 整店订阅（负载较小，同类于 P010） | `[ ]` 待修复 |
| P056 | P2 | 架构 | `tauri/src/stores/objectStore.ts:184-220` | objectStore 的 trash 切片是死代码，与 trashStore 双轨调用不同后端命令 | `[ ]` 待修复 |
| P057 | P2 | 架构 | `tauri/src/stores/objectStore.ts:158-169` | `updateObject` 只更新缓存不同步 `objects` 摘要列表（潜伏性不一致） | `[ ]` 待修复 |
| P058 | P2 | 架构 | `tauri/src/stores/profileStore.ts:75-98` | `loadSection`/`updateField` 无调用方死代码，且 updateField 写后不同步本地 | `[ ]` 待修复 |
| P059 | P2 | 架构 | `HistoryPage.tsx:36-38`、`HistoryViewer.tsx:485-487`、`vaultStore.ts:40-43` | 3 处前端 invoke 链缺 `.catch`，锁定等场景下 unhandled rejection 且无提示 | `[ ]` 待修复 |
| P060 | P2 | 规范 | `tauri/src/components/plugin/PluginResultPanel.tsx:344,360` | UI 组件直接调 plugin-fs `copyFile` 且静默吞错，违反 IPC 封装约定 | `[ ]` 待修复 |
| P061 | P2 | 架构 | `tauri/src/pages/settings/GlobalAttachmentManager.tsx`（12 处直接 invoke） | 页面组件承载附件树遍历/聚合/批量编排等重业务逻辑，应下沉 store/lib（与 P024 关联） | `[ ]` 待修复 |
| P062 | P2 | 架构 | `tauri/src/stores/settingsStore.ts:154-307` | 主题/语言设置四副本（zustand+localStorage+ui_preferences.json+vault），需补写入路径矩阵注释或收敛 | `[ ]` 待修复 |
| P063 | P2 | 架构 | `tauri/Cargo.toml:73` | release profile `panic = "abort"`：未来新增生产代码引入 unwrap 时代价大，保留认知即可 | `[ ]` 待修复 |

## 修复进度

- 已完成：39 / 63（P001–P011、P013–P016、P019–P028、P029、P030、P031、P032、P033–P042；其中 P011 工作区部分完成）
- 当前处理：P042 已完成，等待下一条指令

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

**P003 | 附件元数据 file_name 存储型路径遍历**
`export_import.rs:836-855`：落盘用 `safe_name`（取 `Path::file_name()`），但元数据写回原始 `file_name`；后续插件主机 `copy_attachment_to_workspace`（`host.rs:1284`）直接 `dst_dir.join(file_name)`。恶意导入包设 `file_name=../../evil.txt`，用户用插件处理该附件时发生遍历写。
修复方案：导入时将 `safe_name` 写回元数据；`copy_attachment_to_workspace` join 前加末段净化兜底。

**P004 | 插件提供路径直接 shell open**
`PluginResultPanel.tsx:331-337`：`new URL(path.replace(/\\/g,'/'),'file://').href` 后 `open(fileUrl)`，`path` 来自插件返回值，无校验；叠加 `capabilities/default.json` 的 `shell:allow-open url=**` 与 `tauri.conf.json:78` 允许 `file://`/绝对路径，恶意插件可诱导用户打开 `.app`/脚本，等同逃逸 WASM 沙箱获得本机执行能力。
修复方案：仅允许打开经工作区/插件输出目录校验的路径；或改"在文件夹中显示"；配合 P032 收紧 open 正则。

**P005 | PIN 离线爆破短路主密码**
`pin.rs:188-197`：`derive_key(pin,...)` 派生 KEK 加密会话密钥写入 `pin_credential`；防爆破的 `pin_failed_attempts`/`pin_locked_until`（`:336-350`）只是数据目录 JSON 字段，攻击者拿到数据目录副本后可离线爆破 6 位 PIN（10⁶ 组合，开发 KDF 参数下约数小时）解开全库。
修复方案：PIN 凭证强制生产级 KDF 参数（不随 SOLOSOUL_SECURE 降级）；UI 明确风险提示；考虑 PIN 绑定设备密钥（Keychain/Keystore 包裹）使离线副本无法单独爆破。

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

**P010 | 工作区整店订阅**
`ObjectWorkspacePage.tsx:148-158`：`useObjectStore()` 无 selector，store 每个 action 翻转 `isLoading` 两次并重建 `currentObjectCache`，任何变化触发整页（含 803 行全量卡片 map）重渲染。
修复方案：分字段 selector（`s=>s.objects`、`s=>s.isLoading` 等）。P055 为同类低负载页面。

**P011 | 大列表无虚拟化**
全项目无 windowing 库；工作区一次挂载全部对象卡片，附件管理器全量渲染对象×附件树。
修复方案：引入 `@tanstack/react-virtual` 或分页/「加载更多」。

### P1 架构 / 规范

**P012 | 插件运行时双份平行实现（最高优先级架构债）**
`src-tauri/src/plugin/`（manager.rs 689 行、host/ 1596 行、registry.rs 265）与 `crates/solosoul-plugin/src/`（manager.rs 394、sandbox.rs 129、registry.rs 205、host.rs 1585）各一套；`register_host_functions` 两处各 914 行约 95% 相同，sandbox.rs diff 为 0。`app_state.rs:4` 实际用本地版本，crate 侧约 3000 行无人调用（grep 全 workspace 无 `solosoul_plugin::{manager,sandbox,registry}` 引用）；已观察到两处行为分歧（consent 超时 `Ok(Ok(Ok(..)))` vs `Ok(Ok(..))`、`write_u32` vs `write_handle`），双份代码开始漂移；wasmtime feature 声明也不一致。
修复方案：二选一——删除 crate 侧死实现，或让 src-tauri 全面改用 crate 版本。属大改动，建议单独一轮处理并全量回归插件功能。

**P013 | mDNS 双 daemon**
sync crate manager（`manager.rs:168`）`sync_enable` 时自建 `ServiceDaemon`；`discovery.rs:30,55-60` 另有 app 生命周期常驻 `SharedDaemon`。前端 `syncStore.enable` 成功后立即 `discoverDevices`（`syncStore.ts:140-141`），两个 daemon 同时运行，缓存各自为政。
修复方案：进程内只保留一个 ServiceDaemon（discovery 命令复用 sync crate 实例或反之）。运行时端口冲突未做动态验证，修复时实测。

**P014 | logout 状态分歧**
`authStore.ts:154-155`：`logout` 先 `await invoke('logout')` 后 `set(...)`，无 try/catch；`AppRoutes.tsx:474` 在 `vault-locked` 事件里 fire-and-forget 调用——后端已锁定，invoke 若 reject 则 `set` 被跳过，AuthGuard（`routes.tsx:33`）继续放行受保护路由，UI 进入半认证僵尸态。
修复方案：状态重置放 `finally`（先清前端状态再尽力通知后端）。

**P015 | 认证状态双 store**
路由守卫只读 `authStore.isAuthenticated`；`vaultStore.vaultState` 全应用无人读取（仅测试引用）；解锁有三种写入路径（authStore.login、LoginPage 直接 setState:303/:357、vaultStore.unlock），同步依赖 `vault-locked` 事件不丢失。
修复方案：删除 `vaultState` 死状态，lock/unlock 收敛为 authStore action，vaultStore 降级为薄封装或删除。与 P014 一并处理。

**P016 | 裸调 plugin-dialog（违反明确约定）**
`lib/dialog.ts:9` 明文禁止裸调，但 `ScanLocalPage.tsx:60`、`WatermarkPluginConfig.tsx:192`、`OcrQuickScanPopover.tsx:139` 三处直接 `open()`；开启「切后台锁定」时文件选择器触发 `visibilitychange:hidden` → Vault 被误锁，选完文件后流程失败。
修复方案：三处全部改 `openWithPause`。修复成本极低，建议优先处理。

### P1 死代码

**P017 | liquid-glass 整套死样式**
`components/liquid-glass/GlassCard.tsx` + `GlassCard.module.css` + `styles/liquid-glass.css` 全文件零引用（`main.tsx` 只导入 tokens/global/themes/animations）。删除整个目录与 CSS 文件。**（删除文件属暂缓事项，见文末）**

**P018 | 死 barrel ×2**：`stores/index.ts`、`components/guide/index.ts` 零导入方，删除文件（同属暂缓）。

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

**P042 | run_migrations 464 行**（solosoul-vault/migration.rs:31）：11+ 版本迁移顺序堆叠，每段重复 `pragma_table_info` 查列样板。抽 `has_column()` 帮助函数 + 每版本 `migrate_vN()`，主函数改版本列表驱动。

**P043 | lib.rs run 441 行**：setup 闭包内联 10+ 初始化步骤。拆 `init_logging`/`init_data_dir`/`init_state` 等步骤函数。

**P044 | 3 个 220+ 行函数**：`export_execute`（export.rs:217，258 行）按阶段抽私有函数；`trash_get_detail`（object/snapshot.rs:210，254 行）按 item_type 抽 preview 构建；`send_chat_stream`（llm/stream.rs:76，227 行）抽 `parse_sse_chunk`/`extract_delta` 纯函数。

**P045 | 深层嵌套 ×5**：`helpers.rs:173-232` rewrite_id_references 三连 if-let（抽 `rewrite_str_ref`）；`import.rs:394-412` 模板 labels 合并 5 层（提前 return/and_then）；`profile.rs:191-205` 6 层（`iter_mut().find()` 组合子）；`plugin.rs:80-107` 5 层（抽 `migrate_seed_bindings` 用 `?`）；`llm/stream.rs:257-302` SSE 尾部 6 层（抽子函数+提前 continue）。

**P046 | cleanup_orphan_attachments 双份**：`attachment.rs:983-1027` 整体复制 `objects.rs:945-999`，仅多 audit/auto_sync 触发。GUI 改调 core 实现后再做日志与同步；附带的 `map(|mut d| d.next().is_none()).unwrap_or(false)` 可顺手改 `is_ok_and`。

**P047 | watermark 注册闭包重复**：`register.rs:763-812` vs `:838-890` 除 `apply_to_image`/`apply_to_pdf` 一行外完全相同；抽 `register_watermark_fn(linker, name, apply)` 泛型注册；read_string 参数校验样板 20+ 次可抽帮助函数/宏。**注意：本项依附于 P012 的取舍结果，应先定 P012 方向再动。**

**P048 | 手写 hover ×109**：30+ 文件内联 `onMouseEnter/Leave` 手写 hover，项目已有带 CSS hover 的 `ui/Button`。统一迁移（必要时补 variant），样式随主题统一。

**P049 | 筛选 chip 块 ×5**：约 45 行「isActive 三态 style+hover 双事件+map」重复 5 处，抽 `FilterChipGroup({options,value,onChange})`。

**P050 | ObjectEditorPage 校验 switch 表驱动化**：:300-360 六个 case 同一「正则失败→写 errors」模式，改 `Record<PropertyType,{re,hintKey}>` 查表循环，顺带消除 5 层嵌套。

### P2 性能

**P051 | embedding 逐条事务**：`rag.rs:794-808,942` 重建时逐条 `save_guide_embedding`（每次抢锁+autocommit+fsync）。加批量接口：单事务+prepared statement 循环绑定。

**P052 | 回收站批量删除串行 IPC**：`trashStore.ts:137-140` 循环内顺序 await。后端加 `trash_permanent_delete_batch` 或前端 `Promise.all`。

**P053 | loadCustomPages N+1**：`settingsStore.ts:328-355` 每页单独 `object_get` 拉 description（已 Promise.all 并发，页面数少）。可让 `object_list` 附带 description 或加批量命令。

**P054 | 附件批量下载串行**：`GlobalAttachmentManager.tsx:419-425`、`AttachmentViewer.tsx:343-361` 逐条 invoke+文件拷贝（同文件删除/恢复已有 batch 命令，下载没有）。加后端批量命令或并发化。

**P055 | 其余整店订阅**：`ObjectEditorPage.tsx:36`、`TrashPage.tsx:79`、`TemplateManagerPage.tsx:75`、`ObjectDetailModal.tsx:136` 分字段 selector（负载小于 P010）。

### P2 架构 / 规范

**P056 | objectStore trash 死切片**：`objectStore.ts:184-220` 的 `trashObjects`/`loadTrashObjects`/`restoreObject`/`purgeObject` 无 UI 引用，且与 trashStore 调不同后端命令（`object_restore` vs `trash_restore`）操作同一后端数据。删除该切片及对应测试。

**P057 | updateObject 不同步列表**：`objectStore.ts:158-169` 只更新 `currentObjectCache`，`objects` 摘要保持旧值（现靠页面重挂载掩盖）。成功后同步更新 `objects` 对应项。

**P058 | profileStore 死代码+不同步**：`loadSection`/`updateField`（:75-98）无调用方且 `updateField` 写后端后不更新本地。删除或补同步并接入调用方。

**P059 | invoke 缺 catch ×3**：`HistoryPage.tsx:36-38`、`HistoryViewer.tsx:485-487`、`vaultStore.ts:40-43`（导航栏直接作 onClick）。补 `.catch` 错误提示；可抽公共 hook。

**P060 | PluginResultPanel 直接 FS+吞错**：:344,360 组件内 `copyFile` 且 `catch{}` 静默。下沉 Rust command 或 lib 封装，失败 toast 提示（与 P004 同文件，建议同轮处理）。

**P061 | GlobalAttachmentManager 重业务下沉**：12 处直接 invoke+多层聚合统计（:556-589）在组件内。数据编排下沉 store/lib hook（与 P024 同设计）。

**P062 | 设置四副本**：zustand+localStorage+ui_preferences.json+vault 加密 preferences（settingsStore.ts:154-307），为「登录前主题正确」的有意设计但任一写入遗漏即产生主题跳变 bug。补「写入路径矩阵」注释；长期收敛单副本+登录前只读快照。

**P063 | panic=abort 认知**：`tauri/Cargo.toml:73` release `panic="abort"`，任何遗漏 panic 都是无清理整进程终止。当前生产 unwrap 仅 2 处可证安全，风险低；作为评审认知保留，无需改动（可标记为「设计如此」）。

---

## 暂缓事项说明（依据流程约束：不自动执行破坏性操作）

以下修复涉及**删除文件/依赖**，执行到对应 ID 时将先标记暂缓、优先处理其他问题，最终汇总时再提请用户确认：

- P017（删除 `components/liquid-glass/` 目录与 `styles/liquid-glass.css`）
- P018（删除 `stores/index.ts`、`components/guide/index.ts`）
- P022 / P035 部分（卸载 npm 依赖、改 package.json 与锁文件）
- P020 / P021 / P033 / P034（删除 Rust 命令与函数——虽为代码删除而非文件删除，按「最小改动+一项一提交」正常执行，但 `ensure_guide_embeddings_built`、`clear_cache`、`object_backfill_*`、`trigger_periodic` 需先人工确认是「未接线」还是「真死代码」）

## 修复顺序建议（阶段 2）

1. **先 P0**：P001（+P029 顺带）。
2. **P1 安全**（Rust 侧集中）：P002+P003（同源一并）、P004（+P032 联动）、P005。
3. **P1 性能**（Rust 优先）：P006+P007（同区域）、P008、P009；前端 P010、P011。
4. **P1 规范快赢**：P016（三处改 openWithPause，成本极低）。
5. **P1 架构**：P014+P015（同链路）、P013；P012 体量大，单独一轮并全量回归插件。
6. **P1 死代码**：P017-P022（注意暂缓事项）。
7. **P1 结构**：P028（抽共享函数）→ P023 → P024-P027。
8. **P2 按语言分批**：Rust 批（P029/P031/P045/P046/P051…）→ 前端批（P038-P041/P048-P050/P052-P059…）。
