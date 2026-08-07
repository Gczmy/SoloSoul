# 代码分析修复报告

> 最后更新：2026-08-07（P000-P001/P004-P005/P007/P011/P015 已修复）
> 当前分支：`main`
> 修复轮次：1（初始分析）
> 基线版本：v2.8.5（HEAD `cdc6afb6`）
> 说明：本报告为全新生成，未沿用任何历史报告内容。

---

## 阶段 0 / 静态分析基线结果

| 检查项 | 命令 | 结果 |
|--------|------|------|
| TypeScript 类型检查 | `npx tsc --noEmit` | ✅ 通过 |
| Rust 格式化 | `cargo fmt --check` | ✅ 通过 |
| Rust Clippy | `cargo clippy -- -D warnings` | ✅ 通过（零警告） |
| ESLint | `npm run lint` | ✅ 通过 |
| 前端单元测试 | `npm run test`（Vitest） | ✅ 61 文件 / 573 用例全部通过 |
| ACL 白名单一致性 | `python3 scripts/check_acl_consistency.py` | ✅ 190 个命令均已登记 |
| Rust 单元测试 | `cargo test` | ❌ **349 通过 / 1 失败**（`tests::test_dispatch_cluster_prefixes_consistent`，见 P000） |

工具链静态检查全部干净，但 Rust 单元测试存在 1 个失败用例（P000），按流程应优先修复。其余问题均为启发式扫描（人工 + 脚本辅助）发现，按四个维度执行：A. Rust 死代码/质量、B. Rust 性能、C. 安全漏洞、D. 前端死代码/性能/架构一致性。已按流程忽略 `node_modules/`、`target/`、`dist/`、`.vite/`、`*.wasm` 等生成目录。

## 问题清单（按优先级 P0 > P1 > P2）

本轮共 46 项：P0 × 1、P1 × 15、P2 × 30。

| ID   | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|------|--------|------|----------|------|------|
| P000 | P0 | 测试 | `tauri/src-tauri/src/lib.rs:1010` | `test_dispatch_cluster_prefixes_consistent` 断言硬编码 188，簇列表实际已有 190 条命令，`cargo test` 红、CI 必挂 | `[x]` 已修复（断言 188→190，注释同步） |
| P001 | P1 | 安全 | `tauri/src-tauri/src/commands/export_import/export.rs:253-272` | 导出 `save_path` 无落盘基目录限制（附件下载有，导出没有，校验不一致） | `[x]` 已修复（桌面端复用 allowed_fs_bases 白名单 + `..` 拒绝） |
| P002 | P1 | 安全 | `tauri/crates/solosoul-core/src/biometric/legacy.rs:88-103` | 遗留生物识别文件加密密钥派生自公开 account_id，可还原主密钥（存疑：仅限未迁移老安装的迁移窗口） | `[ ]` 待修复 |
| P003 | P1 | 安全 | `tauri/src-tauri/src/commands/update.rs:202-245` | Android APK 校验和与安装包同通道下发，无独立签名验证 | `[ ]` 待修复 |
| P004 | P1 | 性能 | `tauri/crates/solosoul-plugin/src/field.rs:219-240` | 插件每解析一个字段就全表解密一次（模板+对象），无缓存，K 字段 × N 对象放大 | `[x]` 已修复（FieldCache 惰性缓存：templates/all_objects/by_type 三类） |
| P005 | P1 | 性能 | `tauri/src/hooks/useExportScope.ts:52-76` + `export.rs:644-667` | 导出附件勾选 N+1：前端逐对象 IPC、后端逐对象整解密 | `[x]` 已修复（export_get_attachments_batch 批量命令 + load_objects_batch） |
| P006 | P1 | 性能 | `tauri/src-tauri/src/commands/llm/conversation.rs:13-57` | LLM 会话存为单个加密 Profile blob，每条消息全量解密+全量重加密 | `[ ]` 待修复 |
| P007 | P1 | 性能 | `tauri/src-tauri/src/commands/attachment.rs:479,930` | `attachment_copy_to_vault`/`attachment_download` 在 tokio worker 上同步复制大文件，未走 `spawn_blocking` | `[x]` 已修复（两处 fs::copy 移入 spawn_blocking，guard 块作用域释放） |
| P008 | P1 | 性能 | `tauri/src-tauri/src/commands/search/commands.rs:43` | 搜索每次缓存未命中即全表解密，无索引（存疑：取决于目标 Vault 规模） | `[ ]` 待修复 |
| P009 | P1 | 死代码 | `tauri/src-tauri/src/commands/export_import/import.rs:177-198` | `import_execute` 为 `#[tauri::command]` 但未在 `lib.rs` 注册、前端无调用，已被 `import_execute_advanced` 取代 | `[ ]` 待修复 |
| P010 | P1 | 死代码 | `tauri/crates/solosoul-crypto/src/cipher.rs:141`、`aes.rs:96,135` | `encrypt_chunked_to_bytes`/`encrypt_chunked_blob`/`decrypt_chunked_blob` 仅测试引用（存疑：可能有意保留对称 API） | `[ ]` 待修复 |
| P011 | P1 | 架构 | `tauri/src/stores/syncStore.ts:466-571` | 入站同步完成后不刷新 `objectStore`，工作区显示过期数据直至重新导航 | `[x]` 已修复（applied>0 时 loadObjects + 清详情缓存，首事件与合并分支） |
| P012 | P1 | 规范 | `tauri/src/components/settings/BiometricSection.tsx:330-417`、`PinSection.tsx:278-327` | 两处自行实现主密码验证对话框，未用共享 `PasswordVerificationDialog`（违反硬约定） | `[ ]` 待修复 |
| P013 | P1 | 死代码 | `uiStore.ts:24-90`、`llmStore.ts:22,86`、`templateStore.ts:26,94`、`ocrScanStore.ts:38-40,148-150` | 多个 store 状态/action 仅被测试引用，生产零引用 | `[ ]` 待修复 |
| P014 | P1 | 规范 | `AGENTS.md`（敏感数据分级节约定） | AGENTS.md 强制要求的 `SensitiveValueWidget`/`SensitivityBlurredWidget`/`SensitivityTag` 组件不存在，实际掩码机制是 `useRevealState`+`SensitivityBadge`；文档称 6 级敏感度，实现为 4 级 | `[ ]` 待修复 |
| P015 | P1 | 安全 | `tauri/src-tauri/src/commands/export_import/import.rs:230`、`export_import/mod.rs:112`、`solosoul-sync/src/recovery.rs:46-49` | 导入/导出/恢复密码未走 `Zeroizing` 模式（auth 已全部 P031 化，这三处遗漏） | `[x]` 已修复（导入/恢复 IPC 边界 Zeroizing；RecoveryHost.recovery_password Zeroizing） |
| P016 | P2 | 重复代码 | `tauri/crates/solosoul-vault/src/storage.rs:707,942` | `migrate_to_encrypted_format` 与 `reencrypt_all` 各 211 行互为镜像，按表复制样板 5-6 次；且均整表 collect 进内存 | `[ ]` 待修复 |
| P017 | P2 | 重复代码 | `src-tauri/src/sync/auto_sync.rs:134`、`sync/device_auto_sync.rs:148` | 两个自动同步状态机约 90 行近乎逐行重复 | `[ ]` 待修复 |
| P018 | P2 | 可维护性 | 7 处超长函数（详见下文） | `import_execute_internal`(224行) 等 7 个 >150 行函数，嵌套最深 d11 | `[ ]` 待修复 |
| P019 | P2 | 可维护性 | `crates/solosoul-vault/src/storage/sync_meta.rs:499` | `cleanup_expired_tombstones` 嵌套 d13，手写 HLC 三元组 min 比较 | `[ ]` 待修复 |
| P020 | P2 | 规范 | `src-tauri/src/commands/llm/rag.rs`（8 处）、`llm/guide.rs:515,617` | `eprintln!` 绕过 tracing 日志体系，release GUI 中不可见 | `[ ]` 待修复 |
| P021 | P2 | 可维护性 | `crates/solosoul-core/src/watermark/mod.rs:345` | `WatermarkPosition::Tile => unreachable!()` 依赖上方守卫，守卫改动即生产 panic | `[ ]` 待修复 |
| P022 | P2 | 性能 | `crates/solosoul-plugin/src/manager.rs:568-572`、`sandbox.rs:33-46` | 插件 WASM 每次运行重新编译 + 每次从磁盘全量读入 | `[ ]` 待修复 |
| P023 | P2 | 性能 | `src-tauri/src/commands/log.rs:105` | `log_export` 在 async 命令内同步解密万行审计日志 + JSON 序列化 + 写盘 | `[ ]` 待修复 |
| P024 | P2 | 性能 | `src-tauri/src/commands/fs.rs:183-231` | `fs_scan_directory` 在 async 命令内同步递归遍历目录 | `[ ]` 待修复 |
| P025 | P2 | 性能 | `src-tauri/src/commands/llm/rag.rs:144-147,230-232`、`solosoul-plugin/src/manager.rs:441,474` | RAG embedding 每次调用新建 `reqwest::Client`，重建 TLS 连接 | `[ ]` 待修复 |
| P026 | P2 | 性能 | `crates/solosoul-core/src/ocr/model.rs:194` | OCR 每次 `scan_rgb` 重读并重解析 det 后处理配置 | `[ ]` 待修复 |
| P027 | P2 | 性能 | `crates/solosoul-core/src/export_import.rs:324-328` | 导入包 payload 一次性全量入内存（密文+明文峰值 ~200MB，有 100MB 上限兜底） | `[ ]` 待修复 |
| P028 | P2 | 性能 | `crates/solosoul-core/src/watermark/mod.rs:612` | 水印每次调用读入整个 TTC 字体（CJK 常 10-50MB） | `[ ]` 待修复 |
| P029 | P2 | 性能 | `crates/solosoul-vault/src/storage.rs:272-310` | `probe_data_key` 每次探测新建 SQLite 连接（仅解锁恢复路径，存疑可不修） | `[ ]` 待修复 |
| P030 | P2 | 安全 | `crates/solosoul-core/src/llm/client.rs:33-36` | LLM 阻塞客户端 `.timeout(None)`，慢速滴流可永久挂起线程 | `[ ]` 待修复 |
| P031 | P2 | 安全 | `src-tauri/src/commands/llm/chat_http.rs:5-29,34-55` | `llm_check_connection`/`llm_test_provider` 接受任意 URL+api_key，构成受限带凭证转发原语（无内网段防护） | `[ ]` 待修复 |
| P032 | P2 | 安全 | `crates/solosoul-core/src/vault_service.rs`（`unlock_secure` 路径） | 主密码解锁无失败限流（PIN 有，主密码没有），dev KDF 参数下字典攻击可行 | `[ ]` 待修复 |
| P033 | P2 | 安全 | `src-tauri/tauri.conf.json:30` | CSP `object-src data:` 过宽、`style-src 'unsafe-inline'` | `[ ]` 待修复 |
| P034 | P2 | 安全 | `tauri/src/pages/auth/LoginPage.tsx:53`、`BootstrapPage.tsx:20-22`、`ExportImportPage.tsx:105-106` | 密码驻留 React state（JS 堆不可清零，Web 栈固有限制，可提交后立即置空缓解） | `[ ]` 待修复 |
| P035 | P2 | 安全 | `tauri/src-tauri/src/commands/llm/`（聊天路径） | AI 对话将解密内容发往第三方云端 LLM，需确认 UI 有明确隐私提示（存疑：产品决策） | `[ ]` 待修复 |
| P036 | P2 | 规范 | `pages/workspace/WorkspaceObjectCard.tsx:320-377`、`hooks/useRevealState.ts:62-65`、`components/object/HistoryViewer.tsx:232` | 掩码逻辑分散三处且规则不一致（internal 掩码与否、占位符 4/8 圆点不一致） | `[ ]` 待修复 |
| P037 | P2 | 架构 | 多处（`SnapshotEntry`×3、`ConversationSummary`×3、`ObjectSummary`×3 等） | 前后端镜像类型在前端重复定义 10+ 组，无防漂移机制 | `[ ]` 待修复 |
| P038 | P2 | 架构 | `tauri/src/lib/searchCache.ts` | 搜索缓存 30s TTL 无写失效，新建/编辑对象 30 秒内搜不到 | `[ ]` 待修复 |
| P039 | P2 | 性能 | `components/llm/ChatMessageList.tsx:178`、`ConversationHistory.tsx:46`、`pages/editor/HistoryPage.tsx:96` | 大列表无分页/虚拟滚动（HistoryPage 快照随编辑次数无限增长） | `[ ]` 待修复 |
| P040 | P2 | 重复代码 | `attachment/ConfirmDialog.tsx`、`workspace/ConfirmDeleteDialog.tsx`、`template/DeleteConfirmDialog.tsx` 等 5 处 | 5+ 个手写确认对话框重复 `ui/ConfirmDialog` 骨架 | `[ ]` 待修复 |
| P041 | P2 | 可维护性 | `pages/auth/LoginPage.tsx`(约750行)、`App/AppRoutes.tsx`(约630行) 等 | 超长组件/hook 5 处，职责混杂 | `[ ]` 待修复 |
| P042 | P2 | 错误处理 | `TemplateManagerPage.tsx:77`、`SampleTemplateDetail.tsx:33`、`TemplateEditor.tsx:121`、`TemplateDetailModal.tsx:61` | 4 处 `loadInstalled().catch(() => {})` 静默吞错，加载失败表现为「无插件」假象 | `[ ]` 待修复 |
| P043 | P2 | 架构 | `tauri/src/stores/objectStore.ts:185-196` | `deleteObject` 不清 `currentObjectCache` 详情缓存，残留旧数据 | `[ ]` 待修复 |
| P044 | P2 | 可维护性 | `hooks/useRevealState.ts:90-93`、`pages/auth/LoginPage.tsx:32` | 遗留注释：字段类型感知部分掩码规格未实现；`__DEBUG_SHOW_ALL` 调试开关遗留 | `[ ]` 待修复 |
| P045 | P2 | 重复代码 | `tauri/src/hooks/useExportScope.ts`（同 P005 根因） | 占位（合并至 P005 处理，不单独计数） | `[x]` 误报/合并 |

## 修复进度

- 已完成：8 / 46（P045 为合并占位，实际待修复 45 项）
- 当前处理：P000-P005/P007/P011/P015（已修复）→ P014

---

## 详细问题描述与修复指引

### P000（P0 · 测试）调度簇一致性测试断言过期

- **位置**：`tauri/src-tauri/src/lib.rs:1010`
- **问题**：`test_dispatch_cluster_prefixes_consistent` 末尾 `assert_eq!(total, 188)` 为硬编码字面量；上方注释「P002：删除 4 个死命令后共 188 条」已过期。近期新增 2 个命令后簇列表实际 190 条，断言 `left: 190, right: 188` 失败。
- **影响**：`cargo test` 红，CI `rust-test` 必挂，阻塞所有 PR 合并。
- **建议**：将断言更新为 190 并同步注释；更稳妥的做法是去掉硬编码数字，改为断言簇列表与 `invoke_handler` 注册集合一致（防止再次漂移）。修复后需 `cargo test -p solo_soul` 验证。

### P001（P1 · 安全）导出 save_path 无落盘基目录限制

- **位置**：`tauri/src-tauri/src/commands/export_import/export.rs:253-272`
- **问题**：`export_execute` 的 `save_path` 由前端 IPC 传入，仅做 `~/` 展开与 `.solosoul` 后缀补全；对比 `attachment.rs:83-124` 的 `path_within_base`/`allowed_fs_bases`，导出路径不校验目标必须位于允许的基目录（Desktop/Documents/Downloads）。
- **影响**：若 webview 被 XSS 攻破，可以应用权限向任意路径写入攻击者可控内容的 zip（配置/启动文件覆写、DoS）。
- **建议**：复用 `attachment.rs` 的 `allowed_fs_bases` + canonicalize 前缀判定，将导出落盘限制在用户下载类目录。模式现成，修复成本低。

### P002（P1 · 安全，存疑）遗留生物识别文件密钥派生自公开 account_id

- **位置**：`tauri/crates/solosoul-core/src/biometric/legacy.rs:88-103,127-165,32`
- **问题**：`biometric_key` 文件（内容为主密钥 hex）的 AES 密钥 = HKDF(SHA-256(account_id))，account_id 即公开目录名，非秘密；更旧的 <2.0 格式仅用硬编码 XOR key（`LEGACY_XOR_KEY`）混淆。
- **威胁场景**：同 UID 恶意进程读 `~/.solosoul/{acc}/biometric_key` → 重算密钥 → 解密主密钥 → 无需主密码/生物识别解密整个 Vault。代码注释自知「主要安全防护：OS 文件权限 0o600」。
- **缓解**：macOS/Windows/iOS 主存储走 Keychain/DPAPI，此文件仅为遗留迁移路径，读取后即迁移并删除。风险窗口 = 未迁移老安装 + 删除前。
- **建议**：设定迁移窗口截止版本，到期强制迁移并删除遗留文件；参考 P209 诊断计数 `count_legacy_key_files` 归零后移除整个 legacy 模块。

### P003（P1 · 安全）Android APK 校验和同通道下发

- **位置**：`tauri/src-tauri/src/commands/update.rs:202-245`
- **问题**：Android 更新的 SHA-256 校验和与 APK 来自同一个 GitHub Release 资产列表，无独立签名验证。代码库在 `embed_model.rs:46-47` 已认识到同通道问题并对 embedding 注册表用 minisign 修复，Android APK 通道没有。
- **缓解**：Android 包管理器强制升级包签名一致，实际可利用性低；但对首次安装/旁加载无保护。
- **建议**：对 `.sha256` 或 `latest.json` 用 updater 同款 minisign 密钥签名，客户端硬失败校验。

### P004（P1 · 性能）插件字段解析每字段全表解密

- **位置**：`tauri/crates/solosoul-plugin/src/field.rs:219-240,299-301,366-367`
- **问题**：插件每次 `solo_get_field` host 调用都 `list_user_templates()`（模板全表解密）+ `list_objects(...)`（对象全表 AES 解密），`FieldResolver` 无任何缓存。
- **影响**：K 个字段 × 1000 对象 = K×1000 次 AES-GCM 解密/次插件运行，随 Vault 规模线性恶化，秒级延迟。
- **建议**：在 `FieldResolver`（生命周期=单次插件运行）内加惰性缓存，或在 `run()` 入口预取一次注入。

### P005（P1 · 性能）导出附件勾选 N+1 IPC

- **位置**：`tauri/src/hooks/useExportScope.ts:52-76` → `tauri/src-tauri/src/commands/export_import/export.rs:644-667`
- **问题**：`loadObjectAttachments` 对全部选中对象无并发上限地逐个调 `export_get_attachments`；后端每调用 `load_object()` 整解密只为取 `__attachments`。前端注释自认 N+1。
- **影响**：全选 500 对象 = 500 次 IPC + 500 次解密瞬时并发，导出对话框明显卡顿。
- **建议**：仿照已有 `attachment_count_batch` 增加 `export_get_attachments_batch(object_ids)`，后端走已存在的 `load_objects_batch`。

### P006（P1 · 性能）LLM 会话单体加密 blob

- **位置**：`tauri/src-tauri/src/commands/llm/conversation.rs:13-57`、`llm/stream.rs:433-467`、`services/profile_prefs.rs:15-44`
- **问题**：所有会话存于 `profile.preferences.llmConversations`。每次发消息/重命名/删除都整 Profile 解密 → 改一项 → 整体重加密写回。
- **建议**：会话拆表存储（每会话一行独立加密），消息追加只重写单行。属结构性改动，可排队靠后。

### P007（P1 · 性能）附件复制/下载阻塞 tokio worker

- **位置**：`tauri/src-tauri/src/commands/attachment.rs:479,930`
- **问题**：两个 async 命令直接在 tokio worker 线程上 `std::fs::copy` 大文件，未走 `spawn_blocking`。同文件其他命令已 P114 化，这两个是漏网。
- **建议**：把路径校验后的 `fs::copy` 段移入 `tokio::task::spawn_blocking`。修复成本最低，与既有模式完全一致。

### P008（P1 · 性能，存疑）搜索全表解密无索引

- **位置**：`tauri/src-tauri/src/commands/search/commands.rs:43`
- **问题**：每个新关键词触发全部对象 AES 解密 + 递归 JSON 匹配（在 spawn_blocking 中不卡 UI，但结果慢）。
- **建议**：中期可考虑明文索引表或增量复用上次解密结果。是否修复取决于目标 Vault 规模。

### P009（P1 · 死代码）`import_execute` 未注册

- **位置**：`tauri/src-tauri/src/commands/export_import/import.rs:177-198`
- **证据**：`lib.rs:436` 只注册 `import_execute_advanced`；前端仅 `invoke('import_execute_advanced')`；全库 grep 仅命中定义处与第 497 行审计日志字符串。
- **建议**：删除该函数。**注意：属删除代码，按流程约束先标记暂缓，最后由用户确认。**

### P010（P1 · 死代码，存疑）crypto crate 仅测试引用的 pub API

- **位置**：`tauri/crates/solosoul-crypto/src/cipher.rs:141`、`aes.rs:96,135`
- **问题**：`encrypt_chunked_to_bytes`/`encrypt_chunked_blob`/`decrypt_chunked_blob` 生产路径无人调用，仅自身测试引用；但解密对偶 `decrypt_chunked_from_bytes` 有真实生产调用，API 不对称。
- **建议**：若非有意保留对称 API，删除或改 `#[cfg(test)]`。**删除需用户确认。**

### P011（P1 · 架构）入站同步后前端缓存不刷新

- **位置**：`tauri/src/stores/syncStore.ts:466-571`
- **问题**：`sync-completed` 处理器只刷新 sync 状态与冲突列表，全库无任何地方在同步完成后调用 `objectStore.loadObjects` 或清除 `currentObjectCache`；`objectStore.ts:97` 的 `loadObjects` 是全量覆盖式缓存，仅由 `useObjectWorkspaceData.ts:321-326` 在 accountId/pageId 变化时触发。
- **影响**：用户停留在工作区时，对端同步进来的新增/修改对象不可见，直到切换页面。
- **建议**：`sync-completed` 处理器中（applied > 0 时）触发当前页面对象列表重载，或让工作区页面订阅 syncStore 的 lastInboundResult。

### P012（P1 · 规范）两处自行实现密码验证对话框

- **位置**：`tauri/src/components/settings/BiometricSection.tsx:330-417`（约 90 行手写浮层）、`PinSection.tsx:278-327` 及 447 附近
- **对比**：共享组件 `PasswordVerificationDialog.tsx`（618 行，含 PIN 回退、自动锁定暂停、密码提示）仅在 `ObjectWorkspacePage.tsx:359`、`ObjectDetailModal.tsx:498` 被使用。
- **影响**：三份密码验证 UI 并存，行为不一致风险（两处手写版各自 import `useAutoLockPauseStore` 正是复制成本的体现）。
- **建议**：迁移到 `PasswordVerificationDialog`（自定义 title/onVerified 回调即可覆盖）。

### P013（P1 · 死代码）store 仅测试引用的状态/action

- **位置**：`uiStore.ts:24,26,60,62,70,90`（`sidebarCollapsed`/`toggleSidebar`/`globalLoading`/`setGlobalLoading`）；`llmStore.ts:22,86`（`stopStream`）；`templateStore.ts:26,94`（`saveFromObject`）；`ocrScanStore.ts:38-40,148-150`（三个 selector）
- **建议**：删除（同步清理测试）或改用——ocrScanStore 的 getter 可替换生产代码中的内联 filter。**删除需用户确认。**

### P014（P1 · 规范）AGENTS.md 掩码组件约定漂移

- **证据**：`SensitiveValueWidget|SensitivityBlurredWidget|SensitivityTag` 在 `tauri/src/` 零命中——三个「必须使用的共享组件」不存在。实际掩码机制是 `useRevealState.ts` + `SensitivityBadge.tsx`。AGENTS.md 称敏感度 6 级，前后端实际均 4 级（`types/template.ts:21`、`template_service.rs:24`）。
- **建议**：更新 AGENTS.md 指向真实组件与 4 级模型；配合 P036 统一掩码规则。

### P015（P1 · 安全）导入/导出/恢复密码未 Zeroizing

- **位置**：`tauri/src-tauri/src/commands/export_import/import.rs:230`、`export_import/mod.rs:112`、`tauri/crates/solosoul-sync/src/recovery.rs:46-49`
- **问题**：`auth.rs` 所有密码入口均 `Zeroizing::new(password)`（P031 模式），但导出/导入/恢复通道密码全程普通 `String`；恢复密码在 `RecoveryHost` 中驻留整个会话期（最长 5 分钟）。
- **威胁场景**：内存转储/交换分区恢复出导出/恢复密码 → 解密已导出的 `.solosoul` 备份包（全部用户数据）。
- **建议**：IPC 边界统一 `Zeroizing<String>` 包装；`RecoveryHost` 字段改 `Zeroizing<String>`。

### P016（P2）`storage.rs` 迁移/重加密镜像重复

- **位置**：`tauri/crates/solosoul-vault/src/storage.rs:707,942`
- **问题**：`migrate_to_encrypted_format` 与 `reencrypt_all` 各 211 行，「SELECT 整表 → 逐行重加密 → UPDATE」样板按表重复 5-6 次，两函数互为镜像；新增加密表需同时改两处。附带：均整表 collect 进内存，大 vault 换密钥内存峰值可观。
- **建议**：抽表驱动公共 helper（表名+列清单+行转换闭包），考虑分批处理。

### P017（P2）自动同步双状态机重复

- **位置**：`src-tauri/src/sync/auto_sync.rs:134`、`sync/device_auto_sync.rs:148`
- **问题**：Idle/Scheduled/Running 三态 select 循环、重试退避、test spawn 分支约 90 行近乎逐行重复，仅事件类型与 enabled 开关不同。
- **建议**：泛型化或抽共享函数；若不抽象，至少注释互相指向。

### P018（P2）超长函数 7 处

| 函数 | 位置 | 行数 | 嵌套 |
|---|---|---|---|
| `import_execute_internal` | `commands/export_import/import.rs:226` | 224 | d6 |
| `install_from_registry` | `crates/solosoul-plugin/src/manager.rs:170` | 213 | d8 |
| `AppState::new` | `src-tauri/src/state/app_state.rs:257` | 203 | d11 |
| `compute_sync_changes` | `src-tauri/src/commands/object/mod.rs:880` | 185 | d6 |
| `import_vault` | `crates/solosoul-core/src/export_import.rs:307` | 175 | d7 |
| `handle_inbound` | `crates/solosoul-sync/src/session.rs:313` | 173 | d7 |
| `search_unified` | `src-tauri/src/commands/search/commands.rs:185` | 156 | d9 |

`AppState::new` 尤甚：移动端 SAF 校验、降级、缓存迁移、本地初始化多分支揉在一个构造函数。建议按职责拆私有函数。

### P019（P2）`cleanup_expired_tombstones` 嵌套 d13

- **位置**：`crates/solosoul-vault/src/storage/sync_meta.rs:499`
- **建议**：`RecordHlc`/`SyncWatermark` 实现 `Ord` 后用 `min_by` 收敛 :573-593 的手写三元组比较（约 20 行）。

### P020（P2）`eprintln!` 绕过 tracing

- **位置**：`src-tauri/src/commands/llm/rag.rs` 8 处（:334/345/354/365/469/920/944/950）、`llm/guide.rs:515,617`
- **建议**：改 `tracing::warn!/info!`。

### P021（P2）脆弱的 `unreachable!()`

- **位置**：`crates/solosoul-core/src/watermark/mod.rs:345`
- **建议**：改为返回 `cfg.position` 单点坐标或合并分支，消除「守卫改动即生产 panic」隐患。

### P022（P2）插件 WASM 每次运行重新编译

- **位置**：`crates/solosoul-plugin/src/manager.rs:568-572`、`sandbox.rs:33-46`
- **建议**：启用 wasmtime 模块缓存（`Config::cache_config_load_default`）或以 plugin_id 为键缓存 `Module`。

### P023（P2）`log_export` 阻塞 worker

- **位置**：`src-tauri/src/commands/log.rs:105` — `list_audit_log(10000)` 逐行解密 + `to_string_pretty` + `fs::write` 全程同步。
- **建议**：主体移入 `spawn_blocking`。

### P024（P2）`fs_scan_directory` 阻塞 worker

- **位置**：`src-tauri/src/commands/fs.rs:183-231` — 同步递归遍历 + 逐文件 `metadata()`。
- **建议**：移入 `spawn_blocking`。

### P025（P2）RAG 每次新建 `reqwest::Client`

- **位置**：`src-tauri/src/commands/llm/rag.rs:144-147,230-232`、`solosoul-plugin/src/manager.rs:441,474`
- **建议**：`OnceLock` 共享一个带超时的 Client。

### P026（P2）OCR 每次重读 det 后处理配置

- **位置**：`crates/solosoul-core/src/ocr/model.rs:194`（引擎本体已缓存于 `commands/ocr.rs:36`，配置却每次 `read_to_string` + JSON parse）。
- **建议**：解析结果随 `OcrModelBundle` 缓存进 `OcrEngine`。

### P027（P2）导入 payload 全量入内存

- **位置**：`crates/solosoul-core/src/export_import.rs:324-328` — 密文+明文同时驻留，峰值 ~200MB（100MB 上限兜底）。附件本体已流式，仅 payload 未流式。
- **建议**：可对 payload 用 chunked decrypt 写临时文件再流式解析。优先级低。

### P028（P2）水印每次读整个 TTC 字体

- **位置**：`crates/solosoul-core/src/watermark/mod.rs:612` — CJK TTC 常 10-50MB，每次水印导出重复读盘+解析。
- **建议**：进程级缓存已提取的首字体字节。

### P029（P2，存疑）`probe_data_key` 新建连接

- **位置**：`crates/solosoul-vault/src/storage.rs:272-310` — 仅解锁恢复路径调用，低频。列出仅为完整性，可不修。

### P030（P2 · 安全）LLM 阻塞客户端无超时

- **位置**：`crates/solosoul-core/src/llm/client.rs:33-36` — `.timeout(None)`，慢速滴流可永久挂起后端线程。
- **建议**：读超时 60s + 总时长上限，SSE 用逐 chunk 空闲超时。

### P031（P2 · 安全）连接测试命令构成受限转发原语

- **位置**：`src-tauri/src/commands/llm/chat_http.rs:5-29,34-55` — 仅校验 scheme，不要求 URL 属于已登记 provider（与 N-4 embedding 门禁不同），api_key 任意外发。
- **建议**：限制仅 HTTPS + 拒绝内网 IP 段（防 SSRF 探测），或对齐 N-4 登记校验。

### P032（P2 · 安全）主密码解锁无限流

- **位置**：`crates/solosoul-core/src/vault_service.rs`（PIN 在 :136-143 有 `pin_failed_attempts`/`pin_locked_until`，主密码路径无对应机制）。dev KDF 参数（8MiB/2iter）下每次尝试仅几十毫秒，自动化字典攻击可行。
- **建议**：确认 release 强制生产级 KDF 参数；主密码连续失败加指数退避，与 PIN 锁对齐。

### P033（P2 · 安全）CSP 偏宽

- **位置**：`src-tauri/tauri.conf.json:30` — `object-src data:`、`style-src 'unsafe-inline'`。
- **建议**：`object-src` 改 `'none'`；`unsafe-inline` 若 CSS Modules 必需可保留。

### P034（P2 · 安全）前端密码驻留 React state

- **位置**：`LoginPage.tsx:53`、`BootstrapPage.tsx:20-22`、`ExportImportPage.tsx:105-106`
- **说明**：JS 堆不可清零，Web 栈固有限制。建议提交后立即 `setPassword('')` 覆盖引用（核查各页面是否已做）。

### P035（P2 · 安全，存疑/产品决策）云端 LLM 隐私提示

- **说明**：聊天上下文（可含对象内容）经后端转发到用户配置的 provider，N-4 门禁已收窄出口，RAG 不 embedding Vault 对象（`rag.rs:845` 仅内置指南）。但与「敏感数据绝不上传云端」宣传存在张力。
- **建议**：确认 UI 在启用云端 LLM 前有明确隐私提示；provider 设置页标注「对话内容将发送至该第三方服务」。

### P036（P2）掩码规则分散不一致

- **位置**：`WorkspaceObjectCard.tsx:320-377`（internal 也掩码、占位 4 圆点）、`useRevealState.ts:62-65`（internal 不掩码、占位 8 圆点）、`HistoryViewer.tsx:232`（又一处自行 blur）。
- **建议**：提取共享 `shouldMask`/`maskValue` 规则到一处（顺带落地 P014 的约定）。

### P037（P2）前端镜像类型重复定义

- **明细**：`SnapshotEntry`×3（`trash/types.ts:45`、`HistoryViewer.tsx:20`、`HistoryPage.tsx:16`）、`ConversationSummary`×3、`ObjectSummary`×3（两处语义不同却同名）、`AttachmentInfo`/`AuditLogEntry`/`BackupInfo`/`SyncProgressPayload`/`LlmStreamPayload`/`ListTemplate`/`ProviderConfig` 各×2；`useRevealState.ts:3` 本地复制 `SensitivityLevel` union。
- **建议**：收敛到 `types/` 单一来源。

### P038（P2）搜索缓存无写失效

- **位置**：`lib/searchCache.ts`（30s TTL，仅锁定/登出时 clear）；对象 CRUD 不失效缓存。
- **建议**：对象写操作后按 accountId 前缀清除相关缓存键。

### P039（P2）大列表无分页/虚拟滚动

- **位置**：`ChatMessageList.tsx:178`、`ConversationHistory.tsx:46`、`HistoryPage.tsx:96`（快照随编辑次数无限增长）。
- **说明**：OperationLogPage/TrashPage/ObjectWorkspacePage 已有手动分页模式可复用。
- **建议**：至少给 HistoryPage 加 `visibleLimit` 分页。

### P040（P2）手写确认对话框重复

- **位置**：`attachment/ConfirmDialog.tsx`、`workspace/ConfirmDeleteDialog.tsx`、`template/DeleteConfirmDialog.tsx`、`trash/TrashConfirmDialog.tsx`、`object/ObjectDetailDeleteDialog.tsx`，均重复 `ui/ConfirmDialog.tsx` 骨架。
- **建议**：逐步迁移到 `ui/ConfirmDialog`（children 插槽已支持自定义内容）。

### P041（P2）超长组件 5 处

- **明细**：`LoginPage.tsx`（约 750 行）、`AppRoutes.tsx`（约 630 行，混合导航/主题/自动锁定/OCR 首装/双平台更新检查）、`useObjectWorkspaceData.ts`（629 行）、`ExportImportPage.tsx`（750 行）、`AttachmentViewer.tsx`（754 行）。
- **建议**：按职责拆 hook/子组件（如 `useAndroidUpdate`、`useOcrFirstInstall`）。

### P042（P2）静默吞错

- **位置**：`TemplateManagerPage.tsx:77`、`SampleTemplateDetail.tsx:33`、`TemplateEditor.tsx:121`、`TemplateDetailModal.tsx:61` — `loadInstalled().catch(() => {})`，失败表现为「无插件」假象。
- **建议**：至少 `logger.warn` + toast（项目已有 `useToastError` 惯例）。

### P043（P2）`deleteObject` 不清详情缓存

- **位置**：`stores/objectStore.ts:185-196` — 删除只过滤 `objects` 列表，`currentObjectCache[objectId]` 残留。
- **建议**：delete 时同步 evict 缓存槽。

### P044（P2）遗留注释与调试开关

- **位置**：`useRevealState.ts:90-93`（`NOTE: Product spec needed — implement field-type-aware partial masking`，已知未实现规格）；`LoginPage.tsx:32`（`__DEBUG_SHOW_ALL` 调试开关遗留）。
- **建议**：规格缺口记入产品 backlog；调试开关确认无使用后移除。

---

## 已核实无问题的重点项（排除误报）

- **编译器级死代码**：`cargo check`（含 `--all-targets`）零警告；180 个 `#[tauri::command]` 除 P009 外全部注册且被调用；TODO/FIXME/XXX/HACK 全库零命中。
- **生产 panic 面**：生产代码无 `unwrap/expect/panic!/todo!`（唯一例外 `solosoul-sync/src/noise.rs:206` 定长转换，逻辑不可触发）。
- **前端模块级死代码**：全部 314 个非测试 TS/TSX 文件 import 反查（含 lazy/dynamic import），无未引用模块。
- **硬约定遵守**：`plugin-dialog` 全部走 `lib/dialog.ts` 封装（20+ 处无一裸调）；附件批量操作已批命令化；列表项 memo 普遍应用。
- **安全已加固项**：无硬编码密钥（生产）、无日志泄露敏感数据、无命令注入、无 XSS sink（无 `dangerouslySetInnerHTML`）、zip-slip 已防御、同步引擎 Noise+指纹绑定+信任模型完整、恢复通道 PIN 限流、KDF 返回 `Zeroizing`、lock() 显式擦除、capabilities 收紧、OCR/embedding 模型下载 sha256+minisign 签名校验。
- **性能已优化项**（代码内 P007/P109-P115/P202/P210-P212 注释为证）：Embedding/OCR 模型全局缓存、同步附件 32KB 分块、同步/导入单事务批量写、快照列表只读元数据、导出附件流式加密、Argon2 KDF 无热路径重复派生。

## 备注

- **P000 优先**：测试断言过期导致 CI 红，按阶段 0 约束应最先修复。
- 按流程「Rust/TypeScript 分离原则」，建议修复顺序：先集中处理 Rust 侧（P001-P010、P015-P035 中 Rust 项），再处理前端侧（P011-P014、P036-P044）。
- 涉及删除代码的项（P009、P010、P013）按流程约束标记为**暂缓**，待用户确认后执行。
- 标记「存疑」的项（P002、P008、P010、P029、P035）建议修复前先与用户确认处置方式。
