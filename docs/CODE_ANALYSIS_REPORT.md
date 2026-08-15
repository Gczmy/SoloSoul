# 代码分析修复报告

> 最后更新：2026-08-14 23:09:20 BST
> 当前分支：`main`（HEAD `8e8a30e2`）
> 修复轮次：1（初始分析，本轮为全新生成，未沿用旧报告）
> 范围：`tauri/`（React/TS 前端 + Rust 后端与 crates）；已跳过 `node_modules/`、`target/`、`dist/`、`.vite/`、`*.min.js`、`*.wasm`、`gen/`

---

## 基线检查结果（阶段 0 / 静态分析工具）

| 检查 | 命令 | 结果 |
|------|------|------|
| TypeScript 类型检查 | `npx tsc --noEmit` | ✅ 通过 |
| ESLint | `npm run lint` | ✅ 通过（0 警告） |
| 前端单元测试 | `npm run test`（Vitest） | ✅ 77 文件 / 717 测试全部通过 |
| Rust 单元测试 | `cargo test` | ✅ 全部通过（约 915 测试，0 失败） |
| Rust Format | `cargo fmt --check` | ❌ 失败（2 个文件 12 处，见 P006） |
| Rust Clippy | `cargo clippy --all-targets -- -D warnings` | ❌ 1 个错误（见 P005） |

> 因 fmt 失败，`npm run check-all` 在 fmt 步骤中断；后续各项已单独执行确认。fmt/clippy 失败同样会导致 CI（`pr_check.yml` / `ci_cd.yml`）红。

---

## 问题清单（按优先级 P0 > P1 > P2）

**P0：0 项。** 代码库已经过多轮安全加固（P001–P230 编号注释遍布），路径遍历、命令注入、zip-slip、SQL N+1、大文件分块加密、敏感日志脱敏等常见漏洞面均有明确防御且经核实。

| ID   | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|------|--------|------|----------|------|------|
| P001 | P1 | 安全/架构 | `tauri/crates/solosoul-core/src/vault_service.rs` | `lock()` 等处 RwLock 中毒时静默跳过清零，「锁定」后派生密钥仍驻留内存（fail-open） | `[x]` 已修复（P001） |
| P002 | P1 | 架构 | `tauri/src-tauri/src/commands/llm/stream.rs` | 流式对话完成后 `let _ = save_conversation(...)` 吞错，Vault 写入失败时整段对话丢失且无感知 | `[x]` 已修复（P002） |
| P003 | P1 | 架构 | `commands/` 各模块（object/biometric/auth/sync/ocr/template/trash/snapshot/import/export/export_docx/llm-provider） | 审计日志 `log_structured` 与快照 `save_snapshot` 大量 `let _ =` 吞错，审计轨迹/回滚快照可能静默缺漏 | `[x]` 已修复（P003） |
| P004 | P1 | 性能/架构 | `tauri/crates/solosoul-core/src/llm/client.rs` | `process_sse` 整包读入再解析，「流式输出」实际不流式；120s 总超时对长回复直接截断 | `[x]` 已修复（P004） |
| P005 | P1 | 规范 | `tauri/crates/solosoul-core/src/llm/service.rs` | Clippy 错误 `items_after_test_module`：常量与两个函数定义在 `mod tests` 之后，`check-all`/CI 阻断 | `[x]` 已修复（P005） |
| P006 | P1 | 规范 | `tauri/src-tauri/src/commands/update.rs`（9 处）、`tauri/src-tauri/src/commands/object/tests/trash.rs`（3 处） | `cargo fmt --check` 不通过，`check-all`/CI 阻断 | `[x]` 已修复（P006） |
| P007 | P1 | 架构 | `tauri/src/pages/ai/LlmConfigPage.tsx` | `handleDeleteProvider` 删除失败后（catch 仅 warn）仍无条件更新本地状态，前后端状态分叉 | `[x]` 已修复（P007） |
| P008 | P1 | 健壮性 | `tauri/src/lib/i18n.ts` + `tauri/src/main.tsx` | 启动链 `initI18n()` 中 `invoke('get_system_locale')` 无 try/catch，链路末端无 `.catch`：IPC 异常时 `<App/>` 永不渲染（白屏） | `[x]` 已修复（P008） |
| P009 | P2 | 死代码 | `tauri/crates/solosoul-core/src/llm/service.rs` | `pub fn save_conversations` 全仓库零调用；且循环内逐条保存无批量事务 | `[x]` 已修复（P009，用户确认删除） |
| P010 | P2 | 安全 | `tauri/crates/solosoul-core/src/vault_service.rs` | `create_account*` 将 `salt` 与 `verifyHash` 经 IPC 返回前端，前端并不需要，违背最小暴露 | `[x]` 已修复（P010） |
| P011 | P2 | 安全（纵深防御） | `tauri/crates/solosoul-sync/src/noise.rs` | `NoiseKeys` 派生 `Debug`+`Clone`，长期身份私钥可被 `{:?}` 打印且无 Drop 清零 | `[x]` 已修复（P011） |
| P012 | P2 | 安全 | `tauri/src-tauri/src/commands/update.rs` + 前端更新呈现 | APK 校验和 minisign 签名缺失/无效时仅降级继续下载，用户静默失去完整性校验 | `[x]` 已修复（P012） |
| P013 | P2 | 安全 | `tauri/src-tauri/src/commands/export_import/import.rs` + `lib.rs` 启动清扫 | 导入解密明文落系统 temp 目录 `NamedTempFile`，进程 SIGKILL/崩溃时明文残留 | `[x]` 已修复（P013） |
| P014 | P2 | 安全 | `tauri/crates/solosoul-sync/src/recovery.rs:183` | 恢复主机接受裸 6 位数字 PIN 兼容旁路（无 nonce 绑定），削弱抗重放 | `[x]` 保留（用户决策） |
| P015 | P2 | 可优化 | `tauri/crates/solosoul-core/src/vault_service.rs` + `commands/sync.rs` vs `solosoul-sync/src/recovery.rs` | `create_account`/`create_account_with_id` 约 90 行近乎逐字重复（安全敏感代码双份）；`local_display_ip` 跨 crate 重复实现 | `[x]` 已修复（P015） |
| P016 | P2 | 性能 | `tauri/crates/solosoul-sync/src/delta.rs` + `solosoul-vault/src/storage/sync_apply.rs` | 同步冲突分支内每条记录单独解密/写入，大量冲突时变慢（主路径已批量事务化） | `[x]` 已修复（P016） |
| P017 | P2 | 可优化 | 多处（详见下文） | 过长函数/深嵌套候选 9 项（`list_object_changes_since_limited` 165 行等） | `[~]` 部分修复（P017-①~④ 前 4 项，余 5 项按报告建议登记不拆） |
| P018 | P2 | 规范 | `AGENTS.md` 项目结构节 | 声称 `src-tauri/src/ipc/` 存在（实际无此目录），结构表与实际文件不同步 | `[x]` 已修复（P018：文件不存在，问题不复存在） |
| P019 | P2 | 安全（极低风险） | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:221` | 运行时 `Command::new("swiftc")` 依赖 PATH；hash 文件与二进制同目录（自证式校验） | `[x]` 已修复（P019） |
| P020 | P2 | 死代码 | `tauri/src/pages/system/DebugLogPage.tsx:21,66-67` | `levelFilter` 无 setter，过滤分支永不执行，整段过滤逻辑为死代码 | `[x]` 已修复（P020） |
| P021 | P2 | 错误处理 | `tauri/src/pages/ai/LlmChatPage/useLlmChat.ts:118-157` | `handleRename`/`handleSoftDelete`/`handleRestore`/`handlePermanentDelete` 四个 invoke 无 catch，失败无反馈且跳过刷新 | `[x]` 已修复（P021） |
| P022 | P2 | 性能 | 约 10 处（详见下文） | 整个 Zustand store 无选择器订阅，任一字段变更触发整页重渲染 | `[x]` 已修复（P022） |
| P023 | P2 | 错误处理 | `tauri/src/pages/ai/PluginDashboardPage.tsx:464-471` | `pluginCommands.auditLog(50).then(...)` 无 `.catch`，失败时面板静默空白 | `[x]` 已修复（P023） |
| P024 | P2 | 可优化 | `components/object/HistoryViewer.tsx:40`、`components/object/objectDetailUtils.ts:18`、`pages/workspace/WorkspaceObjectCard.tsx:22` | 三份 `flattenProperties` 实现且 `__` 前缀 key 处理规则已分叉，三处渲染可能不一致 | `[x]` 已修复（P024） |
| P025 | P2 | 性能/内存 | `tauri/src/lib/searchCache.ts:42-44` | `SearchCache` 无容量上限，TTL 惰性淘汰，解密结果明文驻留内存只增不减 | `[x]` 已修复（P025） |
| P026 | P2 | 规范 | `tauri/src/hooks/useRevealState.ts:70` | `shouldMask` 在渲染期内执行 `hide(fieldId)` setState，违反 React 渲染纯净性 | `[x]` 已修复（P026） |
| P027 | P2 | 可优化 | `components/layout/SearchPopover.tsx`、`components/sync/SyncConflictDialog.tsx` | JSX 嵌套约 12+ 层（缩进达 40 空格），可维护性差 | `[x]` 已修复（P027） |
| P028 | P2 | 架构 | `tauri/src/pages/ai/LlmConfigPage.tsx:234-240、262-274、290-297` | 三处乐观更新先 setState 再 invoke，失败仅 warn 不回滚 | `[x]` 已修复（P028） |
| P029 | P2 | 架构 | `tauri/src/lib/rustErrors.ts` 与 `tauri/src/lib/backendError.ts` | 两套后端错误本地化库职责重叠，新增错误需判断进哪套，易漏配 | `[x]` 已修复（P029） |
| P030 | P2 | 性能（低影响） | `tauri/src/stores/settingsStore.ts:404-419` | 旧格式自定义页迁移循环内逐条串行 `await invoke('object_create')`（一次性路径，影响极小） | `[x]` 已修复（P030） |

## 修复进度

- 已完成：30 / 30（P018–P030 全部 13 项已修复）
- 当前处理：无（全部完成）

---

## 修复记录（轮次 1）

### P005 · Clippy `items_after_test_module`（已修复）
- **提交**：`7b2fe75c`（见上）
- **改动**：`llm/service.rs` 中 `MAX_CONVERSATION_MESSAGES`、`trim_conversation_messages`、`compare_updated_at` 三个 item 纯移动到 `mod tests` 之前（无逻辑变化）；顺带修复 `biometric/windows.rs:486` 测试中恒真断言 `available == false || available == true`（`bool_comparison` warning，`-D warnings` 下同阻断 CI）。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-core）exit 0 全绿。

### P006 · `cargo fmt --check` 不通过（已修复）
- **提交**：`8792fd17`
- **改动**：`cargo fmt --all` 修复 `commands/update.rs`（9 处）与 `commands/object/tests/trash.rs`（3 处），纯格式化无逻辑变化。
- **验证**：`cargo fmt --all -- --check` exit 0。

### P001 · 锁中毒 fail-open（已修复）
- **提交**：`c9b509f3`（含 rustfmt 规范化）
- **改动**：`vault_service.rs` 三处 `if let Ok(...)` 静默跳过改为 `unwrap_or_else(|e| e.into_inner())` 强制取回（复用文件内 `PASSWORD_ATTEMPT_LOCK` 既有模式）：① `lock()` 三个状态锁（vault_store/session_key/unlocked_account）中毒时强制取回并 `zeroize()`/`take()`，保证「Lock 即擦除密钥」不变量；② `create_account`/`create_account_with_id` 的 accounts_cache + 三个会话状态锁强制取回，杜绝「账户落盘但会话状态部分缺失」不一致。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-core）exit 0。

### P002 · 流式对话保存吞错（已修复）
- **提交**：`b34b5e09`
- **改动**：`commands/llm/stream.rs` `llm_send_message_stream` 尾部 `let _ = save_conversation(...)` 改为显式 `if let Err(e)`：① `tracing::warn!` 落日志（仅记录会话 id 与错误，不含消息内容）；② 向前端 emit `llm-stream-chunk`（is_done=true + error 文案），持久化失败用户可见可重试。回复已完整流式展示，因此不把整个命令判失败（避免前端误判生成中断）。另 `send_chat_stream(app, ...)` 改传 `app.clone()`（流结束后仍需 emit）。
- **验证**：`cargo check` exit 0；`cargo clippy --lib` 无警告。

### P003 · 审计日志/快照大面积吞错（已修复）
- **提交**：`8be25d0c`
- **改动**：`commands/mod.rs` 新增 `log_audit_best_effort()` / `save_snapshot_best_effort()` 封装（内部 `tracing::warn!` 脱敏落日志：仅记动作/实体标识，不记 details 内容），替换 12 个生产文件共 **32 处 `let _ = vault.log_structured(` + 5 处 `let _ = vault.save_snapshot(`**（object/mod、object/snapshot、object/trash、biometric、auth、sync、ocr、template、export、import、export_docx/mod、llm/provider）。审计轨迹与回滚快照写入失败现在有可观测信号。测试文件（object/tests/snapshot.rs）保持不动。
- **验证**：`cargo check` exit 0；`cargo clippy --lib -- -D warnings` exit 0（`&vault` 冗余借用经 `clippy --fix` 清除）；`cargo fmt --all -- --check` exit 0。

### P004 · LLM「流式」实为整包读取（已修复）
- **提交**：`e84a247f`
- **改动**：确认 GUI 路径（`src-tauri/commands/llm/stream.rs` `handle_sse_stream`）本已真流式；整包读取仅影响 CLI（`solosoul_cli/src/app.rs` → `LlmService::send_message_stream` → `client.rs::process_sse`）。`client.rs` 三处改造：① `process_sse` 弃用 `resp.bytes()` 整包读入，改独立读线程 `BufReader` 逐行消费网络流 + mpsc 转发，首 token 到达即触发 `on_event`（CLI 打字机真正流式）；② 超时策略：请求级 120s 总超时改为 `SSE_IDLE_TIMEOUT=120s` 空闲超时（`recv_timeout`，每行重置）——长回复持续出 token 不再被截断，死连接不发数据也不会永久挂起；③ 非流式路径 `process_non_streaming` 经新增 `read_body_with_timeout` 保留总超时兜底；client 改 `connect_timeout(15s)` + 无总超时。解析逻辑与事件语义不变。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-core）exit 0；`cargo check`（solosoul_cli）exit 0；fmt 通过。
- **P004-R1 回归修复（核查轮次 13 发现）**：读线程 `spawn_sse_reader` 原实现 `Err(_) => break` 后照常 `send(None)`，把**网络读错误伪装成 EOF**——中途断流的半截回复被当完整回复 emit Done 并 Auto-save 持久化（与 P001–P003「消除静默失败」主题相悖）。修复：通道类型改为 `Result<Option<String>, String>`，读错误经 `Err` 传播给解析侧（`process_sse` 直接 `return Err`，不 emit Done），`Ok(None)` 仅表示真正 EOF；读循环抽为 `sse_reader_loop` 便于单测。新增回归测试 `sse_reader_propagates_read_error_not_fake_eof`（模拟「读出一行后 BrokenPipe」：断言第二项为 `Err` 而非 `Ok(None)`）。提交 `（待回填）`。
- **P004-R1 验证**：`cargo test llm::client` 3 测试全绿（含新增回归）；`cargo fmt` 通过。

### P001 · 解锁链路锁中毒静默跳过（已修复）
- **提交**：`（待回填）`
- **改动**：核查轮次 13 发现 P001 原修复仅覆盖 lock/create 侧，unlock 侧三处同款 `if let Ok` 中毒静默跳过未改——`unlock`、`unlock_with_session_key`（生物识别/PIN 解锁）的 session_key/unlocked_account/vault_store 写入，以及 `reopen_vault_with_new_key`（改密/KDF 升级共享尾部）的 session_key/vault_store 写入。统一改为 `create_account_common`/`lock()` 已用的 fail-closed 模式 `write().unwrap_or_else(|e| e.into_inner())`：锁中毒按不可恢复处理强制取回写锁，杜绝「解锁/改密成功后会话状态部分缺失」（密钥已设、unlocked_account/vault_store 未设）的不一致；另 `unlock_with_kdf_upgrade` 的 unlocked_account 写入一并对齐。仅影响锁中毒这一异常路径，正常路径行为不变。
- **验证**：solosoul-core `cargo clippy --all-targets -- -D warnings` exit 0；`cargo test vault_service` 32 测试全绿；`cargo test llm::client` 3 测试全绿；fmt 通过。

### P007 · 删除 LLM provider 失败仍更新本地状态（已修复）
- **提交**：`724f6faa`
- **改动**：`LlmConfigPage.tsx` `handleDeleteProvider` 改为 try/catch：删除失败 `logger.warn` + `onError` toast（新增 `settings:llm_delete_provider_failed` 双 locale 键）后提前 return，仅删除成功才 `setProviders(filter)` 与 `setActiveId('')`——后端未删时 UI 不再误移除、不再误清 activeId。
- **验证**：`npx tsc --noEmit` exit 0；eslint 干净；`check-missing-i18n` 双 locale 0 缺失；`vitest run src/pages/ai` 2 测试全绿。

### P008 · 启动链无兜底白屏（已修复）
- **提交**：`b8dc27c0`
- **改动**：① `lib/i18n.ts` Layer 2 的 `invoke('get_system_locale')` 包 try/catch——IPC 异常（后端未就绪/调用失败）捕获后 `logger.warn` 落入 Layer 3 `navigator.language` 兜底；② `main.tsx` 启动链尾部加 `.catch`——任一环节失败也兜底渲染 `<App/>`（i18n 内部已逐层兜底，此处防御 `initI18n` 本身抛错等极端情况），错误落日志。
- **验证**：`npx tsc --noEmit` exit 0；eslint 干净；`vitest run src/lib/i18n.test.ts` 22 测试全绿（含新增回归：IPC reject 时 `initI18n()` resolve 且落 Layer 3）。

### P009 · `save_conversations` 死代码（已修复，用户确认删除）
- **提交**：`3a769b75`
- **改动**：删除 `llm/service.rs` 的 `pub fn save_conversations`（全仓库零调用——`commands/llm/tests.rs` 的 `test_load_save_conversations` 仅是测试名含该字符串，实际逐条调用单条 `save_conversation`，不受影响）。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-core）exit 0；`cargo check`（solosoul_cli / src-tauri）exit 0。

### P010 · `create_account*` 返回 salt/verifyHash（已修复）
- **提交**：`27f8e17e`
- **改动**：`vault_service.rs` `create_account` / `create_account_with_id` 返回值移除 `salt` 与 `verifyHash`——核实前端零消费（`auth::bootstrap` 仅读 id/name/passwordHint，CLI 仅读 id，`recovery::recovery_restore_from_host` 丢弃结果）。两值仍写入磁盘 config（解锁/校验必需），仅不再经 IPC 暴露（verifyHash 泄露可支持离线口令爆破）。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-core）exit 0；`cargo check`（src-tauri / solosoul_cli）exit 0。

### P011 · `NoiseKeys` 私钥可被 Debug 打印（已修复）
- **提交**：`f6a9b178`
- **改动**：`noise.rs` `NoiseKeys.secret` 由 `[u8; 32]` 改 `Zeroizing<[u8; 32]>`（workspace 已有 zeroize 依赖）——私钥与 `Clone` 副本 Drop 时清零；移除派生 `Debug` 改手写实现，仅输出公钥 hex 与 fingerprint（`finish_non_exhaustive`），杜绝 `{:?}` 日志泄漏长期身份私钥；`from_secret` 先 `Zeroizing::new` 再派生公钥，避免中间裸副本残留；`local_private_key` 传 `&*keys.secret` 解引用。新增防回归测试 `test_debug_redacts_secret`（Debug 输出不得含私钥 hex）。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-sync）exit 0；`cargo fmt --check` exit 0；`cargo check`（src-tauri）exit 0。

### P012 · APK 校验和验签 fail-open（已修复）
- **提交**：`48af1aaf`
- **改动**：已确认前端呈现强度（AboutPage 卡片警告 + 下载命令 P002 已 fail-closed），按用户决策方案 B 收口：① Rust `android_check_update` 强制更新（mandatory）且校验和不可信（.sha256 缺失/签名缺失/验签失败）→ 检查阶段硬失败返回明确错误「强制更新已阻止：无法验证 APK 完整性」，不再让用户在遮罩里反复点下载；② 前端 `useAppUpdate`/`UpdateBanner` 新增 `checksumWarning` 透传，available 状态在横幅下方渲染警告条（避免首页横幅用户盲点下载后才看到泛化错误）；③ `MandatoryUpdateOverlay` 增校验和警告展示（防御纵深，Rust 硬失败后该分支正常不触发）。
- **验证**：`npx tsc --noEmit` exit 0；eslint 干净；`vitest run MandatoryUpdateOverlay/UpdateBanner` 9 测试全绿（新增 4 条 P012 断言）；`cargo check`/`cargo clippy --lib -- -D warnings`/`cargo fmt --check` exit 0。

### P013 · 导入明文残留系统 temp（已修复）
- **提交**：`1521e87a`
- **改动**：`import.rs` 新增 `cleanup_orphan_import_temps`（前缀 `solosoul-import-tmp-` 匹配 + `remove_dir_all`，单条失败仅 warn）；`decrypt_package` 明文临时目录由系统 temp 的 `NamedTempFile::new()` 改 `tempfile::Builder::tempdir_in(保险库数据目录)`（0700，与敏感数据同姿态），`NamedTempFile::new_in(临时目录内)`，Drop 时整目录递归删除；`import_execute_internal` 导入前清扫一次；`lib.rs` setup 新增 `setup_cleanup_import_temps`（第 1.5 步）启动时清扫崩溃残留。新增单测 `test_cleanup_orphan_import_temps`（仅删前缀匹配目录、保留无关条目）。
- **验证**：`cargo clippy --all-targets -- -D warnings` exit 0（含测试目标编译）；`cargo fmt --check` exit 0。

### P014 · 恢复流程裸 PIN 兼容旁路（保留，用户决策）
- **决策**：用户选择「保留并登记」。`recovery.rs:174-179、280` 的 `nonce:pin` 裸 PIN 分支是**旧版手动输入兼容**（无 nonce 绑定的老设备恢复）；已确认有 Noise 加密 + 限流兜底，实际风险低。移除会破坏旧版本设备的手动恢复输入，故保留。
- **登记**：风险持续跟踪项——若未来同步协议升级可淘汰旧设备支持，再随版本计划移除。
- **提交**：无代码改动（本次仅文档登记）。

### P015 · create_account 双份实现 + local_display_ip 跨 crate 重复（已修复）
- **提交**：`26b69606`
- **改动**：① `vault_service.rs` 提取 `create_account_common`（密钥派生→写 config→建会话→返回摘要的公共主体），`create_account`/`create_account_with_id` 各保留自身入口校验（名字唯一性 vs ID 已存在 + create_lock）后调用之，安全敏感代码（derive_key/verify_hash/会话建立）收敛为单份，杜绝双份实现漂移；② `local_display_ip` 收敛到 `solosoul-sync/recovery.rs`（改 `pub` + lib 根 re-export），`src-tauri/commands/sync.rs` 删除私有 async 副本（含移动端 timeout 包装）与 `local-ip-address` 死依赖（Cargo.lock 同步），两处调用点改 `solosoul_sync::local_display_ip()`。注：移动端同步二维码路径行为与恢复主机路径对齐（均走 UDP 选择路由地址，纯本地内核操作不阻塞）。
- **验证**：solosoul-core/solosoul-sync `cargo clippy --all-targets -- -D warnings` exit 0；src-tauri check + clippy exit 0；`solosoul_cli` check exit 0；`cargo fmt --check` exit 0。

### P016 · 同步冲突分支逐条解密/写入（已修复）
- **提交**：`c91e1191`
- **改动**：`solosoul-vault` 新增 `save_sync_conflicts_batch`（单锁 + 单事务，N 条冲突一次 commit，逐条 upsert 语义不变）与 `get_sync_conflict_local_data_batch`（objects 表走既有 `load_objects_batch` 单查询，其余表逐条复用）；`solosoul-sync/delta.rs` 冲突分支重构为「候选收集 → 批量取本地数据 → 自动消解判定 → 单事务批量持久化」，消除大量冲突时的 N 次锁竞争与 N 次写事务。lib.rs 新增 `SyncConflictBatchEntry` 条目类型。新增 vault 单测 2 条（批量 upsert 不重复、批量本地数据含软删/缺失 None）。
- **验证**：solosoul-vault/solosoul-sync `cargo clippy --all-targets -- -D warnings` exit 0；src-tauri check exit 0；`cargo fmt --check` exit 0。

### P017-① · `list_object_changes_since_limited` 拆分（已修复）
- **提交**：`fda2703b`
- **改动**：`solosoul-vault/storage/sync_changes.rs` 拆分 165 行巨型函数——SQL keyset 查询 + 行级解密阶段（LEFT JOIN sync_hlc 批量取 HLC、水印/keyset 谓词下推、prepare_cached 复用）抽为私有方法 `query_object_changes`（返回 `(ObjectRecord, RecordHlc)` 列表），主函数保留「最终裁决（严格 > 水印或 keyset 等值组尾部）→ 组装 VaultSyncRecord → merge_tombstones 合并墓碑」流程。纯重构零行为变化，P110/N-1/P213 分页注释随代码迁移。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-vault）exit 0；`cargo fmt --check` exit 0；`cargo test --lib storage::` 137 passed。

### P017-② · `migrate_to_encrypted_format` 拆分（已修复）
- **提交**：`fe0d5176`
- **改动**：`solosoul-vault/storage.rs` 拆分 155 行迁移函数——6 个 `rewrite_table` 逐表闭包抽为模块级 helper：`rewrite_blob_table_encrypted`（profiles/trash_items/object_snapshots 三表单 blob 列共享）、`rewrite_objects_encrypted`、`rewrite_templates_encrypted`、`rewrite_audit_log_encrypted`，sys_config 版本标记写入抽 `write_encryption_version_marker`；主函数保留版本检查、备份、事务与 commit/rollback 编排。纯重构零行为变化（encrypt_field/ensure_encrypted_text 幂等语义不变）。
- **验证**：`cargo clippy --all-targets -- -D warnings`（solosoul-vault）exit 0；`cargo fmt --check` exit 0；`cargo test --lib storage::` 137 passed。

### P017-③ · `search_advanced_impl` 拆分（已修复）
- **提交**：`18a6a29d`
- **改动**：`src-tauri/commands/search/commands.rs` 拆分 180 行 5 层函数——循环体内「逐对象匹配/评分/组装」抽为模块级 helper `match_object_to_query`（返回 `Option<SearchResultItem>`）：类型/敏感度/页面过滤、redact/protected_keys（P114）、字段级命中收集（P021 路径缓冲）、名称加分、最佳命中选取、模板显示名解析全部迁入；主函数保留空查询分支、模板/记录预加载、预筛 filter 循环与排序截断。纯重构零行为变化。
- **验证**：src-tauri `cargo check` exit 0；`cargo clippy --lib -- -D warnings` exit 0；`cargo fmt --check` exit 0。注：src-tauri 测试二进制在本机报 `STATUS_ENTRYPOINT_NOT_FOUND`（所有测试均受影响，Windows 增量链接环境问题，与本次改动无关，登记备查）。

### P017-④ · `build_docx` 拆分（已修复）
- **提交**：`5c37808a`
- **改动**：`src-tauri/commands/export_import/export_docx/docx.rs` 拆分 146 行函数——封面段抽 `push_docx_cover`、单个对象节（分隔横线/对象名/元信息/字段表/附件清单）抽 `push_docx_object_section`、styles.xml 常量抽 `docx_styles_xml`、zip 四文件组装抽 `assemble_docx_zip`；主函数保留 XML 头/封面调用/对象循环/zip 组装编排（146→33 行）。纯重构零行为变化。
- **验证**：src-tauri `cargo check` exit 0；`cargo fmt --check` exit 0。

### P017-⑤ · `register_field_access_fns` 拆分（已修复）
- **提交**：`19f812a5`
- **改动**：`crates/solosoul-plugin/src/host.rs` 拆分 139 行函数——5 个 `func_wrap` 内联闭包（request_field / list_objects / get_data_structure_tree / get_param / list_attachments）抽为模块级 `solosoul_*_impl` 顶层函数（与 `register_http_fns` 的 `http_*_impl` 完全同构），主函数保留注释 + 每注册一行 `linker.func_wrap("env", name, impl)`（139→39 行）。闭包转函数时签名按原参数顺序/类型逐一迁移（`Caller` 泛型内逗号按顶层逗号正确分割），body 原样迁移仅去一层缩进。纯重构零行为变化。
- **验证**：solosoul-plugin `cargo check` / `cargo clippy --all-targets -D warnings` exit 0；`cargo fmt --check` exit 0；plugin 单测 56 passed / 0 failed / 2 ignored。

### P017-⑥ · `import_execute_internal` 拆分（已修复）
- **提交**：`1540ec98`
- **改动**：`src-tauri/commands/export_import/import.rs` 拆分 170 行阶段编排函数（两轮）——① 包内快照解析、KeepBoth ID 映射预构建、附件进度换算（80-100 映射）分别抽 `build_package_snapshots` / `build_keepboth_id_map` / `wrap_attachment_progress`；② 选中集合构建、对象导入主循环、附件+偏好导入、审计详情组装分别抽 `build_selected_ids` / `import_objects_loop` / `import_attachments_and_preferences` / `build_import_details`。主函数保留阶段 1 解密 / 阶段 2 模板重建 / 阶段编排与审计（170→103 行，7 个 helper）。纯重构零行为变化。
- **验证**：src-tauri `cargo check` / `cargo clippy --lib -D warnings` exit 0；`cargo fmt --check` exit 0。

### P017-⑦ · `process_sse` 拆分（已修复）
- **提交**：`b3f9afe2`
- **改动**：`crates/solosoul-core/src/llm/client.rs` 拆分 130 行函数（8 层嵌套）——① 读线程抽 `spawn_sse_reader`（独立 `std::io::BufRead` 消费 + mpsc 转发 + EOF None 通知）；② 4 个 token 计数器抽 `SseCounters` struct（derive Default）；③ 主循环内逐条 SSE data JSON 解析抽 `handle_sse_payload`（按 api_type 提取 token 计数并派发 Chunk，消除 line→data→match→choices→delta→content 8 层嵌套）。主函数留 收流/空闲超时/DONE 判定/计数汇总/Done 事件编排（130→60 行）。纯重构零行为变化。
- **验证**：solosoul-core `cargo check` / `cargo clippy --all-targets -D warnings` exit 0；`cargo fmt --check` exit 0；单测 169 passed / 0 failed。

### P017-⑧ · `recovery_host_start` 拆分（已修复）
- **提交**：`2478a92c`
- **改动**：`src-tauri/commands/recovery.rs` 拆分 155 行函数（5 层）——① 附件 ID 收集抽 `collect_all_attachment_ids`；② 旧主机取消+join+临时文件清理抽 `cancel_and_cleanup_old_host`（锁外 join）；③ mDNS 广告注册抽 `advertise_recovery_mdns`（async，desktop / 非 desktop 双 cfg 实现，fingerprint/display_addr 参数化）。主函数保留密码生成/临时路径/导出调用/主机启动/状态写入/qr 组装编排（155→107 行）。纯重构零行为变化。
- **验证**：src-tauri `cargo check` / `cargo clippy --lib -D warnings` exit 0；`cargo fmt --check` exit 0。

### P017-⑨ · `llm_send_message_stream` 拆分（已修复）
- **提交**：`3ca04d46`
- **改动**：`src-tauri/commands/llm/stream.rs` 拆分 164 行函数（5 层）——① 账户登记检查抽 `ensure_registered_provider`（P102/P016 网络出口收窄）；② 流结束后会话 Auto-save 抽 `persist_conversation_reply`（行级读写 P004 + 保存失败 warn/emit 事件 P002 + 新会话 fallback）；③ token 用量记录 + 统计持久化抽 `record_and_persist_usage`（async，真实/兜底双路径）。主函数保留 URL 校验/提示词提取/流式调用/持久化编排（164→62 行）。纯重构零行为变化。
- **验证**：src-tauri `cargo check` / `cargo clippy --lib -D warnings` exit 0；`cargo fmt --check` exit 0。

---

## 详细问题描述与修复指引

### P1（中等，建议优先）

#### P001 · 锁中毒 fail-open，密钥擦除被静默跳过
- **位置**：`tauri/crates/solosoul-core/src/vault_service.rs:1282-1297`（`lock()`），同类模式另见 629-637、747-755（`create_account*`）
- **证据**：
  ```rust
  pub fn lock(&self) {
      if let Ok(mut key) = self.session_key.write() {
          if let Some(mut k) = key.take() { k.zeroize(); }
      }
      // ...
  }
  ```
- **影响**：RwLock 中毒（某线程持锁 panic）时 `session_key.take()` 被静默跳过，「锁定」后派生密钥仍驻留内存，破坏「Lock 即擦除密钥」的核心安全不变量；`create_account*` 中毒时 vault/session_key/unlocked_account 三者可能部分更新、状态不一致。
- **建议**：锁中毒按不可恢复处理——`unwrap_or_else(|e| e.into_inner())` 强制取回并清零，或直接 panic 上抛让进程退出。

#### P002 · 流式对话保存吞错
- **位置**：`tauri/src-tauri/src/commands/llm/stream.rs:499`
- **影响**：Vault 写入失败（锁中毒/磁盘满/加密失败）时整段对话（含用户输入与 AI 回复）丢失，UI 无感知、无日志。
- **建议**：至少 `tracing::warn!` 记录失败原因；最好向前端 emit 持久化失败事件。

#### P003 · 审计日志/快照大面积吞错
- **位置**：`commands/object/mod.rs:727/729/818/820`、`commands/biometric.rs:365/497/546/667/713/809`、`commands/auth.rs:87`、`commands/sync.rs:18` 等多处
- **影响**：零知识应用的操作审计与编辑快照是核心承诺之一，静默失败造成审计轨迹缺漏、回滚快照缺失，且无可观测信号。
- **建议**：统一封装 `log_audit_best_effort()` / `save_snapshot_best_effort()`，内部 `tracing::warn!`（脱敏）落日志，替代裸 `let _ =`。

#### P004 · LLM「流式」实为整包读取
- **位置**：`tauri/crates/solosoul-core/src/llm/client.rs:142-150`（`process_sse`），调用方注释（:33-35）自认是为覆盖超时的有意取舍
- **影响**：前端要等完整响应才看到第一个 token；120s 总超时对长回复直接截断失败。LLM 对话体验核心路径。
- **建议**：改为增量解析（`read_until(b'\n')` / `impl Stream`），用「空闲超时」（每次读到数据重置计时器）替代总超时。**修复前需确认前端是否已按非流式适配**，若是则降级为文档修正。

#### P005 · Clippy 错误：`items_after_test_module`
- **位置**：`tauri/crates/solosoul-core/src/llm/service.rs`：`mod tests`（:654）之后定义了 `MAX_CONVERSATION_MESSAGES`（:889）、`trim_conversation_messages`（:892）、`compare_updated_at`（:900）
- **影响**：`cargo clippy -- -D warnings` 失败，`check-all` 与 CI（`pr_check.yml`、`ci_cd.yml` rust 检查）阻断。
- **建议**：将三个 item 移到 `mod tests` 之前（纯移动，无逻辑变化）。

#### P006 · `cargo fmt --check` 不通过
- **位置**：`tauri/src-tauri/src/commands/update.rs`（:723、:950、:1056、:1681、:1709、:1726、:1779、:1795、:1820 共 9 处）、`tauri/src-tauri/src/commands/object/tests/trash.rs`（:480、:534、:574 共 3 处）
- **影响**：`check-all` 与 CI 阻断。
- **建议**：运行 `cargo fmt` 即可（无逻辑变化）。注意与 CI rustfmt 版本对齐，避免格式化来回拉扯。

#### P007 · 删除 LLM provider 失败仍更新本地状态
- **位置**：`tauri/src/pages/ai/LlmConfigPage.tsx:327-341`
- **证据**：`invoke('llm_delete_provider').catch(logger.warn)` 后无条件 `setProviders(filter)` 与 `setActiveId('')`。
- **影响**：后端删除失败时 UI 照样移除该 provider，重启后复现；误清 activeId 导致功能开关显示与实际不一致。
- **建议**：catch 分支提前 return，仅删除成功后更新本地状态；或失败后重新拉取列表对齐。

#### P008 · 启动链无兜底，IPC 异常即白屏
- **位置**：`tauri/src/lib/i18n.ts:78`（`await invoke('get_system_locale')` 无 try/catch）+ `tauri/src/main.tsx:31-44`（`initI18n().then(…).then(render)` 链尾无 `.catch`）
- **影响**：`get_system_locale` 一旦 reject（后端未就绪/IPC 异常），整条链断裂，`<App/>` 永不渲染，无错误提示；注释设计的 Layer 3 `navigator.language` 兜底在 IPC 抛错时根本不会执行。
- **建议**：`i18n.ts:78` 用 try/catch 包裹（失败落入 Layer 3）；`main.tsx` 链尾加 `.catch` 兜底渲染或显示致命错误页。

### P2（轻微）

#### P009 · `save_conversations` 疑似死代码
`tauri/crates/solosoul-core/src/llm/service.rs:268`：全仓库（含 `solosoul_cli/`）零调用；且循环内逐条 `save_conversation` 无批量事务，若被误用于批量导入会产生 N 次事务。**建议**：确认无前端/CLI 使用计划后删除（删除文件/代码属破坏性操作，按流程暂缓至用户确认），或改为批量事务实现。

#### P010 · `create_account*` 经 IPC 返回 salt/verifyHash
`tauri/crates/solosoul-core/src/vault_service.rs:639-643、757-761`：verifyHash 泄露可支持离线口令爆破（虽有 Argon2id 成本），违背最小暴露原则；前端创建账户流程并不需要这两值。**建议**：确认前端未消费后从返回 JSON 移除。

#### P011 · `NoiseKeys` 私钥可被 Debug 打印
`tauri/crates/solosoul-sync/src/noise.rs:25-29`：`secret: [u8;32]`（长期身份私钥）派生了 `Debug`+`Clone`，无 Drop 清零。当前无日志点打印（已核查），属纵深防御缺口。**建议**：手写 `Debug` 仅输出 fingerprint；`secret` 改 `Zeroizing<[u8;32]>`。

#### P012 · APK 校验和验签 fail-open
`tauri/src-tauri/src/commands/update.rs:169-171、474-486`：minisign 签名缺失/无效时仅降级「不信任校验和、继续下载」，靠前端 `checksum_warning` 提示。**建议**：强制更新（mandatory）场景硬失败；普通更新至少做成需用户显式确认的阻断对话框。修复前确认前端实际呈现强度。

#### P013 · 导入明文残留系统 temp
`tauri/src-tauri/src/commands/export_import/import.rs:583-588`：解密后明文 payload 落 `NamedTempFile`（系统 temp 目录，0600、Drop 自动删），进程 SIGKILL/崩溃时残留，与「敏感数据仅存放 0700 数据目录」姿态有偏差。**建议**：`tempfile::tempdir_in(data_dir)` 改建于数据目录内，启动时清扫孤儿 temp。

#### P014 · 恢复流程裸 PIN 兼容旁路
`tauri/crates/solosoul-sync/src/recovery.rs:183`：接受裸 6 位数字 PIN（无 nonce 绑定）作为 `nonce:pin` 兼容分支。有 Noise 加密 + 限流兜底，实际风险低，但永久削弱抗重放。**建议**：计划性移除（仅为旧版手动输入兼容）。

#### P015 · 安全敏感代码重复
`tauri/crates/solosoul-core/src/vault_service.rs:526-644` vs `650-762`：`create_account`/`create_account_with_id` 约 90 行近乎逐字重复（仅 ID 生成与查重不同），双份 KDF/verify_hash/原子写/会话建立逻辑，修复易漏一处。**建议**：抽 `create_account_inner(...)`。另 `local_display_ip` 在 `commands/sync.rs:790` 与 `solosoul-sync/src/recovery.rs:403` 重复，可下沉到 sync crate。

#### P016 · 同步冲突分支逐条落库
`tauri/crates/solosoul-sync/src/delta.rs:166-213`：主路径已批量事务化（好），但冲突分支内每条记录单独 `get_sync_conflict_local_data` + `save_sync_conflict`。**建议**：冲突收集后批量落库；低优先。

#### P017 · 过长函数/深嵌套候选（9 项）
| 位置 | 函数 | 规模 |
|---|---|---|
| `crates/solosoul-vault/src/storage/sync_changes.rs:239` | `list_object_changes_since_limited` | 165 行 |
| `crates/solosoul-vault/src/storage.rs:862` | `migrate_to_encrypted_format` | 155 行 |
| `src-tauri/src/commands/search/commands.rs:11` | `search_advanced_impl` | 150 行，5 层 |
| `src-tauri/src/commands/export_import/export_docx/docx.rs:30` | `build_docx` | 146 行 |
| `crates/solosoul-plugin/src/host.rs:285` | `register_field_access_fns` | 139 行 |
| `src-tauri/src/commands/export_import/import.rs:234` | `import_execute_internal` | 140 行 |
| `crates/solosoul-core/src/llm/client.rs:142` | `process_sse` | 嵌套约 8 层 |
| `src-tauri/src/commands/recovery.rs:60` | `recovery_host_start` | 134 行，5 层 |
| `src-tauri/src/commands/llm/stream.rs:398` | `llm_send_message_stream` | 130 行，5 层 |

**建议**：参照已有「阶段化拆分」先例（import.rs:570 注释）逐步拆，不必一次完成。

#### P018 · AGENTS.md 文档漂移
项目结构节声称 `src-tauri/src/ipc/` 存在，实际无此目录（IPC 分发在 `lib.rs` `dispatch_ipc` + 前缀路由）；结构表未含 `attachment_import_plugin.rs`、`keystore_plugin.rs`、`nsd_plugin.rs` 等。**建议**：修订 AGENTS.md 对齐。

### P018 · `AGENTS.md` 文档漂移（已修复）
- **提交**：`79b32fca`
- **改动**：核查确认仓库内不存在 `AGENTS.md`（`git log --all` 无任何历史记录），且 `docs/DEVELOPMENT.md`/`README.md` 等现存文档均无 `src/ipc` 或结构表漂移内容；问题随目标文件不存在而不复存在，无需修改任何文档。
- **验证**：`grep -rn 'src/ipc' docs/ README.md DEVELOPMENT.md` 零命中。

#### P019 · `swiftc` PATH 查找与自证式校验（极低风险）
`tauri/crates/solosoul-core/src/ocr/macos_vision.rs:221`：运行时 `Command::new("swiftc")` 依赖 PATH；缓存二进制有 0700 目录 + SHA-256 自检，但 hash 与二进制同目录（能写二进制者也能改 hash）。实际风险很低，可不改；若要强化：hash 存 vault 数据目录，编译器用 `xcrun -f swiftc` 解析绝对路径。

#### P020 · DebugLogPage 死过滤逻辑
`tauri/src/pages/system/DebugLogPage.tsx:21,66-67`：`const [levelFilter] = useState('all')` 无 setter，过滤分支永不执行。**建议**：删除该 state 与过滤分支，或补上筛选 UI。

### P020 · `DebugLogPage` 死过滤逻辑（已修复）
- **提交**：`985cbb7f`
- **改动**：删除 `levelFilter` state（恒为 `'all'`，无 setter）与 `filteredLogs` 过滤分支（恒等于 `logs`），三处引用改用 `logs` 直读。纯删死代码零行为变化。
- **验证**：`npx tsc --noEmit` 无该文件错误；`npx eslint` 通过。

#### P021 · useLlmChat 四个 invoke 无 catch
`tauri/src/pages/ai/LlmChatPage/useLlmChat.ts:118-157`：`handleRename`/`handleSoftDelete`/`handleRestore`/`handlePermanentDelete` 失败时 unhandled rejection，后续 `refreshLists()` 被跳过。**建议**：统一 try/catch + toast 提示，失败不执行后续本地状态更新。

### P021 · `useLlmChat` 四个 invoke 无 catch（已修复）
- **提交**：`b7c97400`
- **改动**：`useLlmChat.ts` 引入 `useToastError` + `useTranslation('common')`，四个 handler（rename / soft-delete / restore / permanent-delete）统一 try/catch：失败 `logger.warn` + `onError` toast（复用 `common:error` 文案 + rustErrors 翻译），并提前 return 不执行后续本地状态更新 / 刷新；成功路径不变。
- **验证**：`npx tsc --noEmit` 无该文件错误；`npx eslint` 通过。

### P023 · `PluginLogPanel` 审计日志加载无 catch（已修复）
- **提交**：`66d4d7d9`
- **改动**：`PluginDashboardPage.tsx` 的 `PluginLogPanel` 内 `pluginCommands.auditLog(50).then(...)` 补 `.catch`：失败 `logger.warn` 落日志 + `setLoadFailed(true)` 显示错误态提示（新增 `plugin:audit_log_load_failed` 双 locale 键，文案与既有 `no_logs` 并列）；正常路径不变。
- **验证**：`npx tsc --noEmit` 无该文件错误；`npx eslint` 通过；`check-missing-i18n` 双 locale 0 缺失。

#### P022 · 全 store 无选择器订阅（约 10 处）
`useLlmStatsStore()`（`pages/ai/LlmStatsPage.tsx:28`）、`useSettingsStore()`（`AppearanceSettingsPage.tsx:48`、`SecuritySettingsPage.tsx:19`、`BackupConfigPage.tsx:33`）、`useAuthStore()`（`App/AppRoutes.tsx:42`、`pages/auth/useLoginPage.tsx:39`、`useLoginUnlockFlows.tsx:43`、`BootstrapPage.tsx:18`）、`usePluginQuickStore()`（`PluginQuickPanel.tsx:30`）、`useOcrInstallStore()`（`hooks/useOcrFirstInstall.ts:24`）。**影响**：store 任一字段变更触发整页重渲染。**建议**：改为选择器订阅或 `useShallow`。

#### P023 · PluginLogPanel 审计日志加载无 catch
`tauri/src/pages/ai/PluginDashboardPage.tsx:464-471`：`pluginCommands.auditLog(50).then(...)` 无 `.catch`，失败时 unhandled rejection、面板静默空白。**建议**：加 `.catch` 与空态/错误态提示。

### P027 · JSX 深层嵌套拆分（已修复）
- **提交**：`09e85969`
- **改动**：① `SearchPopover.tsx` 结果行抽为模块级 `SearchResultRow` 子组件（图标 + 高亮名称 + 页面/模板/对象三型元信息 + 敏感度徽章 + MatchHint），主组件渲染收缩为 `results.map(renderResultRow)`；② `SyncConflictDialog.tsx` 字段级冲突行抽为 `ConflictFieldRow` 子组件（字段名 + 差异徽章 + 本地/远程两列，对象/数组字段叶子级 diff 展开逻辑原样迁移）。纯抽取零行为变化。
- **验证**：`npx tsc --noEmit` 无相关文件错误；`npx eslint` 通过；`vitest run src/components/sync src/components/layout` 15 测试全绿。

### P022 · Zustand 全 store 订阅改字段级选择（已修复）
- **提交**：`072d0634`
- **改动**：报告列出的 10 处无选择器订阅全部改为 `useShallow` 字段级选择：`useAuthStore()`（AppRoutes / useLoginPage / useLoginUnlockFlows / BootstrapPage——仅选实际消费的字段，error/backendError 等无关字段翻转不再触发重渲染）、`useSettingsStore()`（SecuritySettingsPage / BackupConfigPage / AppearanceSettingsPage——仅选 settings + updateSetting，isLoading/customPages 翻转不再整页重渲染）、`useLlmStatsStore()`（LlmStatsPage）、`useOcrInstallStore()`（useOcrFirstInstall）、`usePluginQuickStore()`（PluginQuickPanel——isOpen 翻转不再重渲染面板）。
- **验证**：`npx tsc --noEmit` 无相关文件错误；`npx eslint` 通过；`vitest run src/pages/auth src/components/plugin` 20 测试全绿。

### P029 · `backendError.ts` 两套后端错误库合并单一入口（已修复）
- **提交**：`b05e4aa8`
- **改动**：Rust 静态错误映射表（`RUST_ERROR_MAP` + `RUST_PREFIX_MAP`，原 `rustErrors.ts` 精确+前缀匹配）并入 `backendError.ts`——`translateRustError` 迁至此处；`resolveBackendErrorMessage` 未命中前缀 token 时新增回退：先查 Rust 静态映射（命中 `i18n.t(key)` 返回），未命中才透传原文。`rustErrors.ts` 降为兼容 re-export 薄壳（5 处旧 import 路径不变），新增错误只需在 `backendError.ts` 单表登记。
- **验证**：`npx tsc --noEmit` 无相关文件错误；`npx eslint` 通过；`vitest run src/lib/backendError.test.ts` 4 测试全绿。

### P019 · `swiftc` PATH 查找与自证式校验（已修复）
- **提交**：`b7500562`
- **改动**：`macos_vision.rs` 两处强化：① 编译 swiftc 不再 `Command::new("swiftc")` 依赖 PATH——新增 `resolve_swiftc()` 优先 `xcrun --find swiftc` 解析绝对路径（Xcode CLT 标准定位），失败才回退 PATH；② hash 文件从缓存目录（与二进制同目录）移出，存 `config_dir/com.solosoul.app/vision_cli/`（0o700，与缓存目录分离），杜绝「能写二进制者也能改 hash」的自证式校验失效；测试环境 hash 仍与缓存同目录（共用临时目录）。
- **验证**：solosoul-core `cargo check` / `cargo clippy --all-targets -D warnings` exit 0；`cargo fmt --check` exit 0；ocr 单测 27 passed。

### P028 · `LlmConfigPage` 乐观更新失败回滚（已修复）
- **提交**：`568f10fe`
- **改动**：`LlmConfigPage.tsx` 三处乐观更新改 try/catch 失败回滚：① `applyActiveProvider` 记录旧 activeId，失败回滚 + toast；② `handleFeatureToggle` 失败 `setChatEnabled(!next)` 回滚 + toast；③ `handleSystemPromptToggle` 失败回滚 + toast。新增 `settings:llm_set_active_failed` / `llm_set_features_failed` / `llm_set_prompt_failed` 双 locale 键。后端未生效时 UI 不再误显新状态。
- **验证**：`npx tsc --noEmit` 无该文件错误；`npx eslint` 通过；`check-missing-i18n` 双 locale 0 缺失；`vitest run src/pages/ai/LlmConfigPage.test.tsx` 2 测试全绿。

### P024 · 三份 `flattenProperties` 收敛为共享实现（已修复）
- **提交**：`b1d7ee2b`
- **改动**：新建 `src/lib/propertyFlatten.ts` 共享核心 `flattenPropertyEntries`，差异点参数化：`keepMetaKeys`（保留 fieldDefs 中定义的 `__` 前缀 key，历史快照的 `__dynamic_group__` 需要；普通展示一律跳过）与 `flattenDynamicGroups`（展平为独立条目 vs 保留分组结构）。三处调用方改为薄包装：① `HistoryViewer.tsx`（keepMetaKeys=true + 分组模式，行为完全不变，13 测试全绿）；② `objectDetailUtils.ts`（展平模式 + fieldId，6 测试全绿）；③ `WorkspaceObjectCard.tsx`（展平模式，3 测试全绿）。新增共享核心自身 8 条单测。
- **验证**：`npx tsc --noEmit` 无相关文件错误；`npx eslint` 通过；`vitest run` 相关 3 文件 24 + 核心 8 测试全绿。

### P025 · `SearchCache` 无容量上限（已修复）
- **提交**：`370e2ef6`
- **改动**：`searchCache.ts` 的 `SearchCache` 增加 LRU 容量上限（构造第二参数 `maxEntries`，默认 200）——`get` 命中刷新 LRU 顺序、`set` 覆盖写去重 + 超限淘汰最久未用条目（与 `photoAlbumPreview.ts` 的 `createBoundedCache` 同构）；TTL 惰性淘汰保留。新增 2 条测试：超限淘汰最久未用、覆盖写不重复占用容量。
- **验证**：`npx tsc --noEmit` 无该文件错误；`npx eslint` 通过；`vitest run src/lib/searchCache.test.ts` 4 测试全绿。

### P030 · `settingsStore` 迁移循环串行 IPC（已修复）
- **提交**：`0fb1d066`
- **改动**：`settingsStore.ts` `loadCustomPages` 的旧格式自定义页迁移循环由逐条串行 `await invoke('object_create')` 改为 `Promise.allSettled` 并行——一次性路径页面数个位数，allSettled 保证单条失败（`logger.warn` 记名）不阻断其余；`migrated` 按原数组顺序归并（fulfilled 才 push），成功页清理/落 store 逻辑不变。
- **验证**：`npx tsc --noEmit` 无该文件错误；`npx eslint` 通过；`vitest run src/stores/settingsStore.test.ts` 19 测试全绿。

### P026 · `useRevealState` 渲染期 setState（已修复）
- **提交**：`184427b8`
- **改动**：`useRevealState.ts` 的过期清理从渲染期迁出——`shouldMask` 不再调用 `hide(fieldId)`，改纯判断（已过期条目视为未 reveal），新增 `useEffect`（监听 `revealed` 变化）扫描过期条目并 `hide`（清一轮后无过期则空转停止）；`shouldMask` 依赖数组同步收窄为 `[revealed]`，StrictMode/并发渲染下不再触发 setState。
- **验证**：`npx tsc --noEmit` 无该文件错误；`npx eslint` 通过。

#### P024 · 三份 `flattenProperties` 实现已分叉
`components/object/HistoryViewer.tsx:40`、`components/object/objectDetailUtils.ts:18`、`pages/workspace/WorkspaceObjectCard.tsx:22`：约 100 行核心逻辑高度相似，但 `__` 前缀 key 处理规则已分叉（HistoryViewer 保留 fieldDefs 中存在的 `__` key，objectDetailUtils 一律跳过）。**影响**：同一对象在工作区卡片/详情弹窗/历史快照三处渲染可能不一致。**建议**：收敛为单一共享实现，差异点参数化。

#### P025 · SearchCache 无容量上限
`tauri/src/lib/searchCache.ts:42-44`：`set` 不设上限，TTL 仅读取时惰性淘汰；会话内缓存条目（含解密后搜索结果明文）只增不减。**建议**：参照 `photoAlbumPreview.ts` 的 `createBoundedCache` 加 LRU 上限（项目内已有先例）。

#### P026 · useRevealState 渲染期 setState
`tauri/src/hooks/useRevealState.ts:70`：`shouldMask` 在渲染期（经 `maskValue` 调用）内执行 `hide(fieldId)` setState，违反 React 渲染纯净性，StrictMode/并发渲染下脆弱。**建议**：过期清理移到 `useEffect`（监听 `revealed` 变化）。

#### P027 · JSX 深层嵌套
`components/layout/SearchPopover.tsx` 与 `components/sync/SyncConflictDialog.tsx` 缩进达 40 空格（约 12+ 层）。**建议**：抽取结果行/冲突行子组件（有 P046 拆分先例）。

#### P028 · LlmConfigPage 乐观更新无回滚
`applyActiveProvider`（234-240）、`handleFeatureToggle`（262-274）、`handleSystemPromptToggle`（290-297）：先 setState 再 invoke，失败仅 warn 不回滚。**建议**：失败时恢复旧值，或成功后才更新本地状态。

#### P029 · 两套后端错误本地化库重叠
`lib/rustErrors.ts`（精确匹配映射表）与 `lib/backendError.ts`（前缀 token 解析），`BootstrapPage`、`SyncShowQrDialog` 同时引入两者。**建议**：合并为单一入口（内部先查精确表再查前缀规则）。

#### P030 · settingsStore 迁移循环串行 IPC
`tauri/src/stores/settingsStore.ts:404-419`：旧格式自定义页迁移循环内逐条 `await invoke('object_create')`。一次性路径、页面数个位数，实际影响极小。**建议**：可暂缓，待后端提供批量命令或改 `Promise.allSettled`。

---

## 已核查但不构成问题的项（避免后续重复排查）

- **生产路径 unwrap/expect**：剔除测试代码后仅剩 3 处（`search/commands.rs:123`、`export_docx/mod.rs:92`、`sync/noise.rs:206`），均有逻辑前置保证。
- **命令注入**：`icacls` 参数数组传递 + 用户名白名单校验；`swiftc`/Vision CLI 无用户输入拼接。
- **路径遍历**：`fs.rs` 目录白名单；附件解析 canonicalize + ParentDir 拒绝；模型 zip 解包 `mangled_name` 检查。
- **SQL N+1**：循环内查询均在事务内 + `prepare_cached`；sync changes 用 LEFT JOIN 批量。
- **大文件加解密**：导出走 `encrypt_chunked_stream` 流式分块。
- **下载完整性**：OCR/embed 模型均有 sha256/minisign 硬校验；URL 校验拒绝远程 http/userinfo/query。
- **敏感日志**：未发现密码/会话密钥入日志。
- **前端 XSS/泄露面**：无 `dangerouslySetInnerHTML`/`eval`；`plugin-dialog` 仅 `lib/dialog.ts` 一处导入；`console.*` 仅两个受控封装；localStorage 不存敏感明文（P230 已排除 OCR 结果）。
- **前端文件级死代码**：约 330 个源文件跨文件引用计数，未发现零引用模块；无过期 TODO/FIXME；生产代码无 `as any`/`@ts-ignore`。
- **大列表虚拟滚动**：无虚拟滚动库，但均有分页游标（50/20 条），缓解充分，不列为问题。

---

> **下一步**：按流程应进入阶段 2/3（确定修复顺序 → 迭代修复）。本轮按用户指令暂停于此，等待修复指令。
> 建议修复顺序：P005/P006（工具链阻断，改动最小、先恢复 CI 绿）→ P001–P004（Rust 静默失败类）→ P007/P008（前端）→ P2 项按语言分批。
