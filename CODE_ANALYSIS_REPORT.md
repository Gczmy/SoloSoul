# 代码分析修复报告

> 最后更新：2026-08-02 01:35:28
> 当前分支：`main`
> 修复轮次：1（初始分析，全新报告）
> 分析范围：`tauri/src/`、`tauri/src-tauri/src/`、`tauri/crates/`（约 6.4 万行 Rust + 323 个前端文件）；按流程忽略 `node_modules/`、`.git/`、`target/`、`dist/`、`.vite/`。

## 基线检查结果（阶段 0）

| 检查 | 命令 | 结果 |
|------|------|------|
| TypeScript | `npx tsc --noEmit` | ✅ 0 错误 |
| ESLint | `npm run lint` | ⚠️ 0 错误，1 条 warning（`useOnboarding.ts:131` 失效的 eslint-disable 指令） |
| Rust Format | `cargo fmt --check` | ✅ 通过 |
| Rust Clippy | `cargo clippy --workspace --all-targets` | ✅ 零警告 |
| 前端测试 | `npm run test` | ✅ 46 文件 / 430 用例全部通过 |
| Rust 测试 | `cargo test --workspace` | ✅ 全部通过（151+23+56+34+103 等） |

## 问题清单（按优先级 P0 > P1 > P2）

### P0（严重：安全高危 / 数据丢失风险）

| ID | 类别 | 文件位置 | 描述 | 状态 |
|----|------|----------|------|------|
| P001 | 安全 | `crates/solosoul-sync/src/session.rs:83-132,204-298` | 同步握手未校验对端静态公钥，`record_peer` 存的指纹来自加密通道内自报消息且每次会话被覆盖；mDNS 明文广播 node_id；信任检查前 HelloAck 已回 account_id——LAN 攻击者可冒充已信任 peer 拉取全量同步数据（含明文附件） | `[ ]` |
| P002 | 安全 | `crates/solosoul-core/src/biometric/legacy.rs:88-103`、`windows.rs:16-52`、`mod.rs:299` | Windows 生产路径生物识别凭证文件仅用 `SHA256(account_id)` 派生密钥保护，任何用户态进程可重算密钥还原主密钥，完全绕过 Windows Hello，打破零知识模型 | `[ ]` |
| P003 | 安全 | `crates/solosoul-crypto/src/kdf.rs:44-50`、`crates/solosoul-core/src/vault_service.rs:423,540` | `SOLOSOUL_SECURE` 全代码库无任何地方设置，所有真实用户的 Vault 主密钥实际用开发档 Argon2id（8MiB/2iter）派生，低于 OWASP 最低建议 | `[x]` 已修复（2026-08-02：`from_env()` release 构建默认 production，仅 debug 用开发档；`unlock` 解锁成功后对低于生产档的旧账户透明升级 KDF 参数并重加密整个 Vault（新 salt/verify hash/参数落盘、生物识别凭证同步、PIN 凭证清除、SAF 远端同步）；pin.rs 旧凭证回退改用显式 development() 避免 release 下无法解锁） |
| P004 | 安全 | `src/stores/trashStore.ts:165`、`src/App/AppRoutes.tsx:470-474` | `trashStore.clearOnVaultLock` 已定义但 vault-locked 清理链从未调用，锁定后回收站解密摘要残留内存 | `[x]` 已修复（2026-08-02：vault-locked 清理链补调 `useTrashStore.getState().clearOnVaultLock()`，与 P005 同 commit） |
| P005 | 安全 | `src/lib/searchCache.ts:12-55`、`src/App/AppRoutes.tsx:470` | 模块级 searchCache 缓存解密后搜索结果明文，全项目无一处调用 `clear()`，锁定后仅靠 30s TTL 自然过期 | `[x]` 已修复（2026-08-02：vault-locked 清理链补调 `searchCache.clear()`，与 P004 同 commit） |
| P006 | 错误处理 | `src/components/object/ObjectDetailModal.tsx:463` | 删除对象失败被静默（`catch { /* ignore */ }`），用户看到弹窗关闭以为删除成功 | `[x]` 已修复（2026-08-02：catch 改为错误 toast + `logger.warn`，确认弹窗仅在成功后关闭，失败时保持打开可重试） |
| P007 | 错误处理 | `src/hooks/useLlmChatCore.ts:314` | `llm_save_conversation` 失败静默（`catch { /* continue */ }`），整段聊天记录可丢失且无提示 | `[x]` 已修复（2026-08-02：三处保存失败路径均补 `showToast(error)` + `logger.warn`，新增 `settings:ai_save_conversation_failed` 双语 key；finalize effect 补 `t` 依赖） |

### P1（中等：安全中危 / 性能高 / 错误吞没 / 大面积死代码与重复）

| ID | 类别 | 文件位置 | 描述 | 状态 |
|----|------|----------|------|------|
| P101 | 安全 | `src-tauri/capabilities/default.json:59`、`src-tauri/src/commands/crypto.rs:5-102` | `allow-all-custom-commands` 使前端可调用全部 200+ command，含 session key 加解密 oracle 与任意密码 Argon2 派生 oracle（入参/回传均不 zeroize）；一旦 XSS 等同 Vault 失陷 | `[ ]` |
| P102 | 安全 | `src-tauri/src/commands/llm/chat_http.rs:5-50` | LLM command 接收任意 `base_url`+`api_key` 由后端发 POST；CSP 禁止 webview 外连使其成为唯一网络出口，XSS 可借此外传数据 | `[ ]` |
| P103 | 安全 | `crates/solosoul-sync/src/session.rs:252,277-287` | 入站连接认证前即 `record_peer` 落库（任意 LAN 主机可刷 peer 表）且 HelloAck 明文回传 account_id 与指纹 | `[ ]` |
| P104 | 安全 | `src-tauri/src/commands/ocr.rs:694-882` | OCR 模型下载 `base_url` 完全由前端传入，无哈希/签名校验、无大小上限，下载后被原生 `ort` 加载执行 | `[ ]` |
| P105 | 安全 | `crates/solosoul-crypto/src/cipher.rs:120-128,210-258` | 自有分块格式的 `chunk_count` 头部不参与 GCM 认证，篡改头部可让导入包/附件静默截断解密而不报错 | `[ ]` |
| P106 | 安全 | `crates/solosoul-crypto/src/aes.rs:119-130` | `decrypt_chunked_blob` 用攻击者可控的 `original_size` 做 `Vec::with_capacity`，篡改头部可致巨额分配 DoS | `[ ]` |
| P107 | 安全 | `src-tauri/src/commands/fs.rs:31-38,180-215` | 桌面端 `allowed_fs_base` 默认为整个 `$HOME`，`fs_read_file_as_text/data_url` 可读 home 下任意文件（含 `~/.solosoul/**`）回传前端 | `[ ]` |
| P108 | 安全 | `src-tauri/capabilities/default.json:23-55` | fs capabilities 允许在 `$DESKTOP/$DOCUMENT/$DOWNLOAD/$TEMP/$APPCACHE/**` 间任意 copy/stat | `[ ]` |
| P109 | 性能 | `crates/solosoul-vault/src/storage.rs:1371-1475` | `list_object_changes_since` 无水印过滤全表解密 + 逐对象一次 HLC SELECT，每轮同步 O(N) 解密 + O(N) 查询 | `[ ]` |
| P110 | 性能 | `crates/solosoul-vault/src/storage.rs:1289-1299` | `list_sync_changes_since_paginated` 名为分页实为全量解密后 `skip/take`，大库同步分页无效 | `[ ]` |
| P111 | 性能 | `crates/solosoul-vault/src/storage.rs:2390-2477` | `list_objects` 对结果集每行解密 properties+labels 并完整 JSON 解析，即使调用方只要元数据（主列表/page_delete/attachment_list_all/llm_context 公共路径） | `[ ]` |
| P112 | 性能 | `src-tauri/src/commands/attachment.rs:505-526,548,593-596` | `attachment_list_all` 同一数据约 4 轮全量解密（list_objects + load_objects_batch 重复 + build 两次 + 每页面再查） | `[ ]` |
| P113 | 性能 | `src-tauri/src/commands/ocr.rs:301-317` | `ocr_scan_image` 在 tokio worker 上同步执行秒级 ONNX 推理，无 `spawn_blocking` | `[ ]` |
| P114 | 性能 | `src-tauri/src/commands/object/mod.rs:459-484` 等 | 所有 vault async command（list/update/search 等）直接在 runtime 上同步做 rusqlite+AES-GCM 解密，重路径未 `spawn_blocking` | `[ ]` |
| P115 | 性能 | `crates/solosoul-sync/src/delta.rs:117-179`、`storage.rs:1628-1653` | `apply_sync_records` 每条记录约 4 条 auto-commit SQL（HLC 重复查询 ×2）且整批无事务、逐条克隆 JSON | `[ ]` |
| P116 | 前端性能 | `src/components/llm/ChatMessageList.tsx:74-124`、`src/hooks/useLlmChatCore.ts:220-230` | 流式期间每个 token 对整个会话所有消息重新 Markdown 解析+语法高亮（消息项未 memo），CPU 随消息数线性放大 | `[ ]` |
| P117 | 前端性能 | `src/hooks/useLlmChatCore.ts:66` | `useLlmStore()` 整店订阅 + effect deps 含整个 store，每个 token 整页（含会话列表）重渲染、effect 重跑 | `[ ]` |
| P118 | 前端性能 | `src/pages/workspace/ObjectWorkspacePage.tsx:123-155` | `WorkspaceObjectCard` 的 memo 被父组件每次新建的内联回调击穿，搜索框每次击键触发最多 50 张卡片全量重渲染 | `[ ]` |
| P119 | 前端性能 | `src/pages/settings/TrashPage.tsx:138-140,369-518` | 回收站可达数百条但无分页、`filtered` 未 useMemo、条目为非 memo 内联 JSX，任何状态变化重建全部卡片 | `[ ]` |
| P120 | 错误处理 | `src/pages/settings/ExportImportPage.tsx:151` | `export_get_scope_tree` 失败被吞，用户看到"空导出范围"误以为数据丢失 | `[ ]` |
| P121 | 错误处理 | `src/hooks/useAttachmentManager.ts:367,398,430` | 批量软删/永久删/恢复 best-effort 吞错，失败对象真实错误丢失只报成功计数 | `[ ]` |
| P122 | 错误处理 | `src/pages/settings/TrashPage.tsx:223` | `trash_get_detail` 失败静默 `setDetailItem(null)`，无法区分"无数据"与"加载失败" | `[ ]` |
| P123 | 错误处理 | `src/components/settings/PinSection.tsx:90` | 验证当前密码的任何异常（含后端崩溃/锁定）统一报"当前密码不正确"，误导用户 | `[ ]` |
| P124 | 错误处理 | `src/hooks/useObjectWorkspaceData.ts:229,248`、`ObjectDetailModal.tsx:313,380` | unlock 失败统一 return false，密码错误与后端异常不可区分，错误细节全丢 | `[ ]` |
| P125 | 错误处理 | `src/pages/system/DebugLogPage.tsx:65` | `log_export` 失败完全静默，点击"导出诊断包"无任何反馈 | `[ ]` |
| P126 | 错误处理 | `src/stores/templateStore.ts:83` | `template_get` 失败返回 null，与"模板不存在"语义混淆 | `[ ]` |
| P127 | 架构 | `src/App/AppRoutes.tsx:396` | `loadCustomPages(account.id)` 未 await 也无 catch，失败产生 unhandled rejection 且自定义页面静默缺失 | `[ ]` |
| P128 | 架构 | `src/pages/settings/AppearanceSettingsPage.tsx:72-83` | 直接 `localStorage.setItem` 写主题缓存绕过 settingsStore，与"四副本写入矩阵"产生第 5 个写入点易漂移 | `[ ]` |
| P129 | 架构 | `src/stores/settingsStore.ts:152-165` | 设置存在 4 份副本（zustand/localStorage/ui_preferences.json/vault 加密 preferences）靠注释矩阵人工维持一致 | `[ ]` |
| P130 | 架构 | `src/App/AppRoutes.tsx:15,345` | 裸调 plugin-dialog `confirm`（SAF 目录失效警告），原生对话框触发 visibilitychange 可致误锁定；封装仅覆盖 open/save | `[ ]` |
| P131 | 架构 | `src/lib/ipc.ts`（全局） | AGENTS.md 约定的"IPC 封装"实为纯类型定义文件，73 个文件全部裸调 `invoke`，无统一错误规范化/未解锁守卫层 | `[ ]` |
| P132 | 死代码 | `src-tauri/src/commands/crypto.rs:40,56,106,119`、`profile.rs:77,96`、`sync.rs:609,615` | 8 个 Tauri 命令前端从未调用（4 个甚至未注册进 generate_handler），含 2 份 cfg 重复的 `sync_listen_port` | `[ ]` |
| P133 | 死代码 | `crates/solosoul-core/src/ocr/macos_vision.rs`（389 行） | 整个 macOS Vision OCR 桥接模块零引用（OCR 走 PP-OCRv6） | `[ ]` |
| P134 | 死代码 | `crates/solosoul-core/src/biometric/macos_keychain.rs`（439 行） | Keychain 方案整模块 `#[allow(dead_code)]`，注释称保留待 Apple Developer Program——建议移出主分支 | `[ ]` |
| P135 | 死代码 | `crates/solosoul-vault/src/safe_storage.rs`（98 行） | 整模块仅自身测试调用，生产零引用 | `[ ]` |
| P136 | 死代码 | `crates/solosoul-core/src/llm/service.rs:114-448` | `LlmService` 两个死方法簇（provider 管理 8 方法 + 会话管理 5+ 方法）生产零调用（GUI 走 commands/llm 自建实现） | `[ ]` |
| P137 | 重复代码 | `crates/solosoul-core/src/llm/config.rs:20` ↔ `src-tauri/src/commands/llm/mod.rs:6` | 8 个 LLM 数据结构在两个 crate 各定义一份（44+29 行），易漂移 | `[ ]` |
| P138 | 重复代码 | `src-tauri/src/commands/sync.rs` | 12 对 `#[cfg(desktop)]`/`#[cfg(mobile)]` 命令函数体逐字节相同（约 133 行），仅 sync_enable/sync_with_device 真有平台差异 | `[ ]` |
| P139 | 重复代码 | `crates/solosoul-sync/src/manager.rs` ↔ `mobile.rs` ↔ `service.rs` | MobileSyncManager 与 SyncService/SyncManager 约 120 行重复（audit_log/trust_peer/forget_peer/connect 流程） | `[ ]` |
| P140 | 重复代码 | `src/pages/settings/OcrSettingsPage.tsx` ↔ `src/pages/scan/OcrPage.tsx` | OCR 模型安装/下载/tier 切换逻辑约 70 行逐字重复 | `[ ]` |
| P141 | 重复代码 | `src/pages/search/SearchPage.tsx` ↔ `src/components/layout/SearchPopover.tsx` | 搜索 state 四件套 + 缓存 + 过滤排序 + 结果行渲染约 80 行重复 | `[ ]` |
| P142 | 重复代码 | `src/components/layout/TopFunctionBar.tsx:181` ↔ `SecondaryActionBar.tsx:158` | hover 展开/收起 + `renderButtonWithCard` 约 58 行几乎相同 | `[ ]` |

### P2（轻微：低危安全 / 中低性能 / 小型死代码 / 结构优化）

| ID | 类别 | 文件位置 | 描述 | 状态 |
|----|------|----------|------|------|
| P201 | 安全 | `src-tauri/src/commands/export_import/import.rs:20-23` | zip 内 `manifest.json` 读取无大小上限，构造的 zip 炸弹可耗尽内存 | `[ ]` |
| P202 | 安全 | `src-tauri/src/commands/export_import/mod.rs:229-235` | 导出包密钥固定 balanced 档（16MiB/3iter）低于 OWASP 推荐；导出包是最可能的离线攻击目标 | `[ ]` |
| P203 | 安全 | `src-tauri/src/commands/attachment.rs:892-899,921-925,934-938` | `attachment_open` 每次以 `error!` 记录完整 vault 路径/object_id/mime，属残留调试日志 | `[ ]` |
| P204 | 安全 | `src-tauri/src/commands/biometric.rs:318-321` | session key hex 放进普通 String 长期残留堆内存 | `[ ]` |
| P205 | 安全 | `src-tauri/src/commands/crypto.rs:77-102` | `derive_key` command 密码入参与返回密钥均不 zeroize，密钥明文经 IPC 进前端 JS 堆 | `[ ]` |
| P206 | 安全 | `src-tauri/tauri.conf.json:30` | CSP `frame-src data:` 无明确必要；`style-src 'unsafe-inline'` 留 CSS 注入口 | `[ ]` |
| P207 | 安全 | `src-tauri/src/commands/embed_model.rs:11,195-233` | Embedding 模型 registry 与 sha256 同通道下发无独立签名（对比插件注册表有 minisign） | `[ ]` |
| P208 | 安全 | `crates/solosoul-plugin/src/sandbox.rs:71` | 插件 WASI `inherit_stdio()` 可向宿主日志注入伪造内容 | `[ ]` |
| P209 | 安全 | `crates/solosoul-core/src/biometric/legacy.rs:32` | `LEGACY_XOR_KEY` 硬编码 XOR 密钥（仅旧凭证迁移用，迁移窗口关闭后应删除整个模块） | `[ ]` |
| P210 | 性能 | `crates/solosoul-vault/src/storage.rs:2480-2487,2647-2653` | 关键词过滤每次把整个 JSON Value 重新 `to_string().to_lowercase()`，Value→String 往返浪费 | `[ ]` |
| P211 | 性能 | `src-tauri/src/commands/object/trash.rs:295-340` | `page_delete` 全量解密筛选 + 逐对象二次解密 + 逐条 auto-commit 写入 | `[ ]` |
| P212 | 性能 | `crates/solosoul-core/src/export_import.rs:365,369-448` | `import_vault` 整体克隆对象数组、循环内逐对象解密判存在 + auto-commit 写入，无事务 | `[ ]` |
| P213 | 性能 | `crates/solosoul-vault/src/storage.rs`（全库） | 无一处 `prepare_cached`；`load_object` 每次 `format!` 分配 SQL 字符串再 prepare | `[ ]` |
| P214 | 性能 | `src-tauri/src/services/llm_context.rs:155-169` | `build_section3` 全量解密所有对象只为筛 public 级（sensitivity_level 是明文列可先 SQL 筛） | `[ ]` |
| P215 | 前端性能 | `src/pages/sync/useSyncPage.ts:13`、`src/components/llm/`、`src/stores/pluginStore.ts:226,236` 等 | 多处整店订阅（sync/plugin/ocr/ui store）致过度重渲染；pluginStore 日志不可变累积 O(n²) | `[ ]` |
| P216 | 前端性能 | `src/components/layout/SecondaryActionBar.tsx:80-84` | onScroll 每帧写 Zustand store 并触发自身 layout effect 重渲染 | `[ ]` |
| P217 | 前端性能 | `src/components/attachment/AttachmentRow.tsx:39`、`AttachmentObjectGroup.tsx:42`、`AttachmentPageCard.tsx:29` | 附件三级列表组件未 memo，rename 状态置顶致每击键整树重渲染 | `[ ]` |
| P218 | 前端性能 | `src/pages/settings/OperationLogPage.tsx:188-200,299` | 200 条审计日志 `filteredLogs` 未 memo 且全量渲染，搜索每击键重建全部卡片 | `[ ]` |
| P219 | 死代码 | `src/lib/sampleTemplates.ts:456`、`useNavigationItems.ts:98,105`、`lib/ipc.ts:86`、`attachmentManagerTypes.ts:36`、`LlmChatPage/index.tsx:17-18` | 前端死导出：`SAMPLE_TEMPLATES`、`LOCK_ITEM`/`SETTINGS_ITEM`、`VaultStateStr`、`AttachmentCompositeKey`、LlmChatPage 冗余再导出 | `[ ]` |
| P220 | 死代码 | `src/components/layout/OcrQuickScanPopover.tsx:1`、`IconPicker.tsx:1`、`hooks/useOnboarding.ts:131` | 2 处未使用 `import React` + 1 处失效 eslint-disable 指令（即基线 lint warning） | `[ ]` |
| P221 | 死代码 | `crates/solosoul-vault/src/storage.rs`（877,3320,3601,3701,1054,3912）、`cipher.rs:121`、`delta.rs:51,183`、`transport.rs:111`、`noise.rs:50`、`pdfium.rs:106`、`template_service.rs:128,325`、`vault_file_system.rs:86`、`profile.rs:55`、`registry.rs from_path` | 各 crate 约 20 个死函数/死类型（多数仅测试调用或零调用） | `[ ]` |
| P222 | 规范 | `crates/solosoul-core/src/ocr/postprocess.rs:252-317`、`mrz.rs:14-372`、`engine.rs:55-136` 等 | 12+ 处仅模块内使用的 `pub` 项可见性过度，应降私有/pub(crate) | `[ ]` |
| P223 | 结构 | `crates/solosoul-plugin/src/host.rs:264`（825 行）、`src-tauri/src/lib.rs:338`（257 行）、`storage.rs:360`（211 行）等 | Rust 过长函数 Top15（>140 行，含嵌套深度 6-9）；`process_sse` 深度 9、`AppState::new` 深度 8 | `[ ]` |
| P224 | 结构 | `src/components/trash/TrashDetailPanel.tsx`（1282 行）、`OcrPage.tsx`（714 行函数）、`TemplateManagerPage.tsx`（688）、`AboutPage.tsx`（682）、`SyncPage.tsx`（650）等 | 前端巨型组件：15 个 >300 行函数、5 个 >800 行文件，JSX 嵌套最深 11（PageGuide/SearchPopover） | `[ ]` |
| P225 | 重复代码 | `crates/solosoul-vault/src/storage.rs:2122,2220,2597`、`vault_service.rs:634,759`、`export_import` 附件收集双份、`pin.rs:202,379` 等 | Rust 中小重复块 10 处（22-60 行）：行解密闭包三份、unlock/verify_password 45 行、附件收集 core/GUI 双份等 | `[ ]` |
| P226 | 重复代码 | `TemplateDetailModal ↔ SampleTemplateDetail`、`AttachmentRow ↔ AttachmentListItem`、`RecoveryScanView ↔ SyncScanQrDialog` | 前端组件重复 3 对（模态外壳/附件行/QR 扫描视图） | `[ ]` |
| P227 | 错误处理 | `authStore.ts:93`、`useUpdateChecker.ts:115`、`LlmConfigPage.tsx:125`、`pluginStore.ts:164,179`、`AttachmentViewer.tsx:268-380`、`useLlmChatCore.ts:142-391` | 低危错误吞没 10 处（静默降级可接受但应补 logger.warn / 错误占位） | `[ ]` |
| P228 | 架构 | `src/stores/authStore.ts:151 ↔ lib/notification.ts:9`、`objectStore.ts:3 ↔ lib/templateSync.ts:56` | 2 处循环依赖（靠动态 import / import type 勉强化解，脆弱） | `[ ]` |
| P229 | 安全 | `src/components/guide/GuideRenderer.tsx:96-105` | 自定义 a 组件直渲 href，当前依赖 react-markdown 默认 urlTransform 拦截 `javascript:`，属隐式依赖应显式白名单 | `[ ]` |
| P230 | 安全 | `src/stores/ocrScanStore.ts:44-117` | OCR 结果（含 MRZ 证件号）常驻 Zustand 内存，无锁定/退出清理路径（persist 已正确排除） | `[ ]` |
| P231 | 杂项 | `src/pages/system/AboutPage.tsx:481-483` | `window.open` 兜底在 Tauri webview 中无效，应删除或改 toast | `[ ]` |

## 修复进度

- 已完成：5 / 69（P003-P007）
- 当前处理：P001（同步握手身份绑定，最高优先安全项）

## 审查通过项（已排查，无需修改）

- **XSS 主体**：0 处 `dangerouslySetInnerHTML`/`innerHTML`；markdown 统一走 SafeMarkdown（react-markdown v10，无 rehype-raw）。
- **命令注入**：`Command` 仅 3 处，均参数数组传递无 shell 拼接；`opener::open` 有 canonicalize+白名单。
- **硬编码密钥**：仅测试 fixture 与已记录的 legacy XOR key；日志无密码/session key 输出。
- **加密原语**：AES-256-GCM 全程 OsRng nonce；verify token 常数时间比较；auth 边界 `Zeroizing`。
- **SQL**：全部参数化查询，sync apply 表名白名单，无 format! 拼接。
- **unwrap/expect**：生产代码仅 2 处且有前置检查，无用户输入可触发的 panic 路径。
- **前端孤儿模块/TODO/注释代码块/不可达分支**：全部为 0。
- **对话框 open/save**：全部走 `lib/dialog.ts` 封装（唯一例外 confirm 见 P130）。
- **监听器/定时器清理**：无泄漏；无过短轮询。
- **ONNX session 缓存**、附件/导出加密流式分块、迁移批量写入事务：均已正确实现。

## 详细问题描述与修复指引

### 一、P0 安全问题

**P001 同步握手身份未绑定（最高优先）**
Noise XX 握手后从未校验对端静态公钥：`record_peer`（session.rs:486-507）存储的 `public_key_fingerprint` 来自加密通道内的 Hello/HelloAck 自报消息（攻击者可伪造），且每次会话被对端上报值覆盖；`noise.rs:175` 的 `remote_fingerprint()`（真实握手密钥）仅在 recovery.rs:265 使用。mDNS 明文广播 node_id，`handle_inbound` 在信任检查前就回复含 account_id 的 HelloAck。
修复：配对时将 `session.remote_fingerprint()` 与 node_id 绑定落库，后续握手强制比对；UI 展示指纹改用握手密钥；信任检查前不要回复 HelloAck（配合 P103）。

**P002 Windows 生物识别凭证保护薄弱**
Windows 生产路径用 `FileBiometricStorage` 存主密钥（`derive_master_key` 产物），文件加密密钥仅由 `SHA256(account_id)`（公开值）经 HKDF 派生——任何用户态进程可读文件重算密钥还原主密钥，Windows Hello 只是应用层弹窗不参与密钥保护。
修复：Windows 改用 DPAPI（CryptProtectData）或 Windows Credential Manager 保护该文件。

**P003 KDF 生产参数从未启用**
建户/解锁参数取 `KdfConfig::from_env()`，终端设备上 `SOLOSOUL_SECURE` 不会被设置，即所有真实用户都用开发档 8MiB/2iter。
修复：release 构建默认 `production()`（64MiB/3iter），仅 `debug_assertions` 下用开发档；已有账户解锁成功后透明升级参数并重加密 verify token。

### 二、P0 敏感数据残留与错误吞没

**P004/P005 锁定后明文残留**：`vault-locked` 事件清理链（AppRoutes.tsx:470-474）只调 objectStore/settingsStore/profileStore，需补 `useTrashStore.getState().clearOnVaultLock()` 与 `searchCache.clear()`；P230（ocrScanStore）同理。

**P006/P007 数据丢失级静默失败**：catch 中至少 toast 报错并保持当前 UI 状态（弹窗不关闭/会话可重试）。同类中危项见 P120-P126、低危见 P227。

### 三、P1 安全中危

- **P101**：用显式 command allowlist 替代 `allow-all-custom-commands`；评估 `encrypt_bytes`/`decrypt_bytes`/`derive_key` 是否仍需暴露，密码参数改 Zeroizing 包装。
- **P102**：对 base_url 做 scheme/host 校验，仅允许用户在设置中登记过的 provider URL。
- **P103**：信任检查前只回最小错误帧；peer 落库延迟到配对确认后。
- **P104**：固定官方模型源或校验内置 sha256 清单；流式下载并限制单文件大小。
- **P105**：把分块头部作为 AAD 纳入每个 chunk 的 GCM，或末尾加整体摘要块；解密后校验总字节数。
- **P106**：`with_capacity` 前对 `original_size` 设上限，或改增量扩展。
- **P107**：`allowed_fs_base` 默认收窄到 Desktop/Documents/Downloads（与 ocr.rs:216-238 的 `is_path_in_allowed_dir` 一致）。
- **P108**：fs capabilities 收窄到 `$APPCACHE`+`$TEMP`，其余经 Rust command 中转校验。

### 四、P1 性能高（Rust）

修复顺序建议：同步链路（P109/P110/P115）→ attachment_list_all（P112）→ spawn_blocking 化（P113/P114）→ list_objects metadata-only（P111）→ prepare_cached（P213）。

- **P109**：watermark 下推到 SQL（updated_at/HLC 表 JOIN），HLC 一次批量查出。
- **P110**：真正的 SQL LIMIT/OFFSET + 水印下推。
- **P111**：拆出 metadata-only 查询（不 SELECT properties 列），或按 keyword 是否为空延迟解密。
- **P112**：复用已解密的 `summary.properties` 做 `load_attachments`，删掉批量重载；页面 children 一次查询按 parent_id 分组。
- **P113/P114**：重 CPU/IO 路径（OCR 推理、全表解密、search/sync）统一 `tokio::task::spawn_blocking`。
- **P115**：整批包一个事务；`apply_sync_record` 接收已查出的 HLC 避免重复查询；`data` 传引用。

### 五、P1 前端性能高

- **P116**：抽 `ChatMessageItem` 用 `memo` 包裹，仅最后一条 assistant 消息在流式期间重渲染。
- **P117**：改字段级选择器 `useLlmStore((s) => s.streamBuffer)`；effect deps 去掉整个 store 对象。同类中低危整店订阅见 P215。
- **P118**：卡片内部接收 `obj` 后自行分发，或父级 `useCallback` + 传 id 而非闭包。
- **P119**：加"加载更多"分页（参照 OBJECT_PAGE_SIZE 模式）；`filtered` 用 useMemo；抽 memo 的 `TrashItemCard`。

### 六、P1 死代码与重复（大面积）

- **P132**：删除 8 个死命令及 lib.rs 注册项（`encrypt_with_key`/`decrypt_with_key`/`generate_salt`/`constant_time_compare`/`profile_get_section`/`profile_update_field`/`sync_listen_port`×2）；`mobile_ocr_plugin.rs:155` 去掉误导性 `#[tauri::command]` 属性。
- **P133-P135**：三个死模块共约 926 行。macos_keychain 属"有意保留"，建议移到分支/文档；删除属破坏性操作，按流程暂缓待用户确认。
- **P136-P139**：LlmService 死方法簇（确认 CLI 路线图后删除）；LLM 8 个结构体统一用 `solosoul_core::llm::config`；sync.rs 12 对无差异 cfg 命令去重；sync 三处管理器抽共享 helper。
- **P140-P142**：抽 `useOcrModelManager` hook、抽 `useObjectSearch` + 共享结果行组件、抽 `renderNavButtonWithCard` 共用函数。

### 七、P2 指引（择要）

- **性能**：P210 解密后缓存原始字符串做匹配；P211 用 metadata-only 列表 + 批量加载 + 事务；P212 存在性判断改轻量预查 + 导入循环包事务；P213 热点语句 `prepare_cached` + SQL 常量化；P214 先按明文列 SQL 筛 public 再解密。
- **死代码/规范**：P219-P221 直接删除（P220 同时修复基线 lint warning）；P222 降可见性。
- **结构**：P223/P224 为长期重构项，建议随功能迭代顺带拆分，不单独安排修复轮次。
- **循环依赖（P228）**：notification 依赖注入；共享类型抽到 `types/`。

## 备注与局限

- 函数行数/嵌套深度由脚本统计（±2 行误差），排序与定性不受影响。
- "死代码"判定均经全 workspace（含 `solosoul_cli/`）词边界 grep + 人工核对；`solosoul-plugin` 的 host fn 供 WASM 插件动态调用未计入。
- 删除文件类修复（P133-P135、P221 部分）按流程属破坏性操作，执行前需用户确认。
