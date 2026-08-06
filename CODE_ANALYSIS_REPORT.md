# SoloSoul 代码分析修复报告

> 最后更新：2026-08-05
> 当前分支：`main`
> 修复轮次：R1 已闭环；**R2（新一轮全库分析）进行中——HEAD = `c0478313`，仅出报告不修复**

---

## §R2 新一轮全库分析（2026-08-05，HEAD = `c0478313`）

> 触发：R1 全部闭环，按流程阶段 4 重新全库扫描。**本轮按用户指令只出报告，不开始修复，等待指令。**
> 分析方法：静态基线（check-all / CLI clippy+test / ACL）+ 三路并行启发式扫描（Rust 后端 / TS 前端 / CLI），全文 grep 交叉验证。

### R2-§1 分析基线

| 检查 | 结果 |
|------|------|
| `npm run check-all`（tsc + fmt + clippy + eslint + vitest + ACL） | ✅ 全绿；Vitest **59 文件 / 560 用例** |
| `check_acl_consistency.py` | ✅ OK: 188 个命令全部登记 |
| `cd solosoul_cli && cargo clippy -- -D warnings && cargo test` | ✅ 0 警告，测试全绿 |

**结论**：全部自动化检查通过，以下问题均来自启发式扫描（静态工具无法覆盖的语义层问题）。

### R2-§2 问题清单（按优先级 P0 > P1 > P2）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| R2-01 | P0 | 安全 | `tauri/src-tauri/src/commands/attachment.rs:785-811` | 路径校验回退分支用纯字符串前缀 `starts_with`，共享前缀的兄弟目录（如 `~/.solosoul_evil/`）可绕过 `in_vault` 检查；src 未拒绝 `..` 组件。该命令不要求解锁 Vault，是 webview 可达的读文件原语 | `[x]` 已修复 |
| R2-02 | P1 | 崩溃 | `solosoul_cli/src/app.rs:1635` | 裸输 `/plugin_run`（无参数）时 `&parts[2..]` 越界 panic，进程崩溃 | `[x]` 已修复 |
| R2-03 | P1 | 崩溃/UX | `solosoul_cli/src/commands/mod.rs:10-18`、`app.rs:1584`、`tui.rs:74` | 命令错误经 `?` 一路传播到 main：Locked 状态手输 `/list` 等命令直接退出 TUI 进程（`require_unlocked` 设置的 `error_message` overlay 设计意图被旁路） | `[x]` 已修复 |
| R2-04 | P1 | 安全 | `solosoul_cli/src/widgets/prompt.rs:43/56/174`、`commands/security.rs:52-110/225/251/300`、`export_import.rs:89/197`、`app.rs:594` | 改主密码/导出密码/删除账户等经 prompt 以纯 `String` 多副本流转且全程不清零，与 `PasswordInput` 的 `Zeroizing<String>` 约定矛盾 | `[x]` 已修复 |
| R2-05 | P1 | 隐患 | `tauri/crates/solosoul-core/src/watermark/mod.rs:606-618` | 代码与注释矛盾：注释明写临时 TTF 文件须存活到 PDF 保存完成，`let _ = temp;` 却立即 drop 删除。目前仅因 Pdfium 急切加载字体而「靠运气正确」 | `[x]` 已修复 |
| R2-06 | P1 | 错误吞没 | `tauri/crates/solosoul-core/src/export_import.rs:1060` | 导入偏好 `save_profile` 失败被 `let _ =` 吞掉，用户看到「导入成功」但 preferences 未落库 | `[x]` 已修复 |
| R2-07 | P1 | 错误吞没 | `tauri/crates/solosoul-core/src/objects.rs:667-670` | `purge_trash` 吞掉底层 `delete_object` 失败仍删 trash 记录 → 孤儿对象行永留数据库且无法再经回收站清理 | `[x]` 已修复 |
| R2-08 | P1 | 性能 | `tauri/crates/solosoul-sync/src/attachments.rs:88-120` | 每次同步会话全表 N+1：`list_object_metadata` 后对每个对象 `load_object`（全量解密）+ 对每个附件文件重新 `sha256_file`，同步延迟/IO 随附件体积线性增长 | `[x]` 已修复（N+1 部分） |
| R2-09 | P1 | UX/错误处理 | `tauri/src/stores/trashStore.ts:142`、`pages/settings/TrashPage.tsx:485`、`components/trash/TrashConfirmDialog.tsx:68` | 回收站永久删除失败时 unhandled rejection：对话框不关闭、无 toast、按钮可重复点击并发重复删除——破坏性操作关键路径 | `[x]` 已修复 |
| R2-10 | P1 | 性能 | `solosoul_cli/src/app.rs:806` 等 10+ 处 | 每次按键 `self.phase.clone()` 深拷贝整个 AppPhase（含大列表 items），大 Vault 下 TUI 掉帧根源 | `[x]` 已修复 |
| R2-11 | P2 | 死代码 | `tauri/src-tauri/src/services/llm_context.rs:32-417` | `build_context` 及整棵私有子树（约 330 行）仅被测试引用存活；活路径仅剩 `clear_cache`/`bump_public_data_version` | `[x]` 已修复 |
| R2-12 | P2 | 死代码 | `tauri/src-tauri/src/commands/llm/rag.rs:502`、`crates/solosoul-plugin/src/registry.rs:60` | `needs_rebuild`、`PluginRegistry::from_path` 全 workspace 零调用 | `[x]` 已修复 |
| R2-13 | P2 | 结构 | Rust 超长/深嵌套函数 | `app_state.rs:257`（189 行/深 10）、`search/query.rs:16`（120/深 10）、`import.rs:226`（213）、`storage.rs:649`（211）与 `:884 reencrypt_all`（207，6 个表处理块复制粘贴）、`plugin/manager.rs:170`（209）、`plugin/host.rs:437/893/662`、`sync/attachments.rs:387` 等 | `[ ]` 待修复 |
| R2-14 | P2 | 重复 | `tauri/src-tauri/src/commands/attachment.rs:394-418 vs 823-846` | `allowed_bases` 白名单构建块两处近乎逐字重复（~90%），且一处含移动端 temp_dir 分支一处不含——策略漂移风险 | `[x]` 已修复 |
| R2-15 | P2 | 性能/杂项 | Rust 轻项 | 主 `payload.enc` 导入未走流式（`import.rs:524`，≈2×payload 内存）；`watermark/mod.rs:88 load_font_bytes` 无缓存；`storage.rs:541` ALTER TABLE 吞掉所有错误；`examples/unlock_account.rs:8` 主密码放 argv | `[x]` 已修复 |
| R2-16 | P2 | 重构 | `tauri/src/`（>400 行文件 28→**40** 个） | P003 清单过期：前五仍准（LoginPage 793/ExportImportPage 754/AttachmentViewer 743/PageGuide 699/HistoryViewer 682），新进：AppRoutes 679、useObjectWorkspaceData 629、PasswordVerificationDialog 617、useAttachmentManager 585、TrashDetailSections 575、AddPageButton 555 | `[ ]` 待修复 |
| R2-17 | P2 | 重复/性能 | `tauri/src/hooks/useExportScope.ts:89-106/223-241/254-270` | 同一段 N+1 附件加载块逐字复制三遍（~55 行），每对象一次 IPC 且无并发上限 | `[x]` 已修复 |
| R2-18 | P2 | 规范 | 前端 100 处 | `t('key') \|\| '兜底文案'` 死兜底模式：i18next 缺 key 返回 key 本身（truthy），`\|\|` 右侧几乎永不执行；100 处硬编码与 i18n 集中管理约定相悖 | `[ ]` 待修复 |
| R2-19 | P2 | UX | `tauri/src/components/object/AttachmentViewer.tsx:169-177` | 重命名乐观更新失败后 `.catch` 仅 `logger.warn` 不回滚：前端显示新名、后端仍是旧名 | `[x]` 已修复 |
| R2-20 | P2 | 性能 | `tauri/src/stores/trashStore.ts:137-146` | `permanentDelete` `Promise.all` 无界并发逐条 IPC（P052 有意改并发但未设上限），清空数百条时瞬间数百 invoke | `[x]` 已修复 |
| R2-21 | P2 | 死代码 | 前端多余 export | `TrashSnapshotView.tsx:184 SnapshotDataView`、`conflictFieldMeta.ts:31/149/227`、`useOnboarding.ts:39 baseSteps`、`searchShared.tsx:136 resolveFieldLabel` 等仅本文件/测试引用却 export | `[x]` 已修复 |
| R2-22 | P2 | 性能 | `solosoul_cli/src/app.rs:2874/2886/2845`、`widgets/status_bar.rs:14`、`screens/object_list.rs:40-50` | 渲染路径每帧磁盘 IO（`load_ui_prefs` 读盘+JSON 解析）、`Theme::load()` 每帧探测环境变量（11 处）、`/list` 无 200 截断（与 /search 不对称） | `[x]` 已修复 |
| R2-23 | P2 | 性能 | `solosoul_cli/src/commands/plugin.rs:140/218/261/459`、`sync.rs:88`、`embed_model.rs:168` | 每次插件/同步/模型命令新建完整 tokio 多线程运行时，应 App 级共享 | `[x]` 已修复 |
| R2-24 | P2 | 安全 | `solosoul_cli/src/commands/log.rs:63` | `/export_log` 用 `fs::write` 默认权限（通常 0644），内容是解密后审计日志，不符合项目 0600 约定 | `[x]` 已修复 |
| R2-25 | P2 | 安全 | `solosoul_cli/src/main.rs:77-86` | `EnvFilter::from_env_lossy()` 无 crate 白名单（`RUST_LOG=debug` 会把依赖的 LLM 请求/vault 操作写进 cli.log）；`rolling::never` 日志无限增长 | `[x]` 已修复 |
| R2-26 | P2 | UX | `solosoul_cli` 多处 | 成功消息（导出成功/密码已修改等）复用红色「! 错误」overlay，已有 `success_message` toast 字段但仅 settings 使用 | `[x]` 已修复 |
| R2-27 | P2 | 死代码 | `solosoul_cli/src/widgets/account_list.rs:39`、`commands/mod.rs:95` | `render_empty` 无调用方；`CliError` type 定义后零使用 | `[x]` 已修复 |
| R2-28 | P2 | 结构 | `solosoul_cli/src/app.rs`（3650 行） | god-object：`render` 312 行、`handle_onboarding_key` 149 行等；解锁样板 `get_vault_store().ok_or_else(...)` 在 17 个文件出现 ~40 次，现成 `require_unlocked_with_vault` 仅 3 个模块使用 | `[x]` 样板收敛已完成；render 拆分列为长期候选 |

## R2 修复进度

- 已完成：11 / 28（第一批 6 项 + R2-08/11/12/14/15）
- 当前处理：第三批（CLI）：R2-04 → R2-10 → R2-22 → R2-23 → R2-24 → R2-25 → R2-26 → R2-27 → R2-28

### R2-§3 重点问题修复指引（P0/P1）

- **R2-01**：canonicalize 失败直接报错（或仅对 Android symlink 场景做组件级 `Path::starts_with` 比较）；src 与 dest 一样拒绝 `ParentDir` 组件。
- **R2-02**：`parts.get(2..).unwrap_or(&[])`（一行修复）。
- **R2-03**：`execute_command` 内 `if let Err(e) = ... { self.error_message = Some(...) }`，不再向上传播；与 overlay 设计意图对齐。
- **R2-04**：`PromptState.value` 与 `PromptResult::Text` 改用 `Zeroizing<String>`（至少 mask=true 时），回调消费后显式 zeroize。
- **R2-05**：`std::mem::forget(temp)`（可接受的小泄漏）或将 `NamedTempFile` 返还调用方持有到 PDF 保存完成，并让注释与实现一致。
- **R2-06/R2-07**：传播错误（`?`）或至少 `tracing::warn!` 并在结果中标注；R2-07 应中止 `delete_trash_item` 让用户重试。
- **R2-08**：SQL 侧过滤 `properties LIKE '%__attachments%'` 或批量接口；导入时算一次 sha256 存入 `AttachmentMeta`，同步读元数据。
- **R2-09**：`TrashConfirmDialog` 加 submitting 态 + try/catch（失败 toast + 保持对话框打开）；restore 路径一并覆盖。
- **R2-10**：`handle_key` 改 `match &self.phase` 只对需要的变体 `std::mem::take`/replace 所有权，避免克隆 items。

### R2-§4 核验为「干净」的维度（勿重复审查）

- **Rust**：unsafe 仍仅限平台 FFI（70+ 处，与 R1 一致）；命令注入零风险（4 处 `Command::new` 参数固定）；serde untagged 零使用；硬编码密钥仅在测试；敏感数据不入日志；命令路径 unwrap/expect 仅 3 处受控；crate 依赖单向无环；112 个注册 IPC 命令全部有前端引用。
- **前端**：XSS 零命中（`dangerouslySetInnerHTML`/`innerHTML`/`eval`）；敏感数据零持久化（OCR 结果/MRZ 不落盘）；IPC 61 文件统一走 `invokeCommand` 封装；Zustand 跨 store 仅单向 toast 通知；大列表均有分页增量挂载 + memo；effect 依赖禁用项均有注释；TODO/FIXME 零技术债标记；非测试 `as any` 零命中。
- **CLI**：Cargo.toml 依赖全部在用；敏感数据不入日志；命令路径 unwrap/expect 仅 2 处受控；薄封装架构边界清晰。

### R2-§5 建议修复顺序（待用户确认后执行）

1. **第一批（Rust/crates，崩溃与安全）**：R2-01 → R2-05 → R2-06 → R2-07 → R2-02 → R2-03（一行级/小范围修复先行）
2. **第二批（Rust 性能与死代码）**：R2-08 → R2-11 → R2-12 → R2-14 → R2-15
3. **第三批（CLI）**：R2-04（Zeroizing 整改）→ R2-10 → R2-22 → R2-23 → R2-24 → R2-25 → R2-26 → R2-27 → R2-28
4. **第四批（前端）**：R2-09 → R2-19 → R2-17 → R2-20 → R2-18 → R2-21 → R2-16

> R2-16 为长期重构候选（延续 P003 定位，随功能迭代顺带）；R2-18 为大范围机械替换，建议单独一批。遵循流程「Rust / TypeScript 分离、一项一提交」原则。

## R2 修复实施记录（逐项更新）

- **R2-01（P0 安全，2026-08-06）**：`attachment_download` 与 `attachment_open` 的路径校验移除字符串前缀回退分支（`src_path.starts_with(vault_base.to_string_lossy())`），改用组件级 `Path::starts_with`；src 与 dest 均显式拒绝 `ParentDir`（`..`）组件。共享前缀兄弟目录（`~/.solosoul_evil/`）不再能绕过 `in_vault`。Android symlink 场景保留 canonicalize 失败时的原始路径兜底，但经组件级比较校验。验证：`cargo test -p solo_soul attachment` 12 用例 + `cargo clippy -p solo_soul --all-targets` 零告警。
- **R2-05（P1 隐患，2026-08-06）**：`try_load_font` 的 TTC 分支改为直接经内存加载——pdfium-render `load_true_type_from_bytes` 内部经 `FPDFText_LoadFont` 复制字体数据到 PDFium 内存（已核读 crate 源码 0.9.2：`load_true_type_from_file` → `read_to_end` 急切读入 → `new_font_from_bytes` → `FPDFText_LoadFont`），彻底消除 `NamedTempFile` 生命周期隐患（原 `let _ = temp;` 立即 drop 删文件、仅靠急切加载才正确），并让注释与实现一致。验证：`cargo test -p solosoul-core` 163 用例全绿。
- **R2-06（P1 错误吞没，2026-08-06）**：`import_preferences` 的 `let _ = vault.save_profile(&profile);` 改为 `vault.save_profile(&profile)?;`——保存失败时整个导入报错返回，不再出现「导入成功」但 preferences 未落库的假象。验证：`cargo test -p solosoul-core` 163 用例全绿。
- **R2-07（P1 错误吞没，2026-08-06）**：`purge_trash` 的 `let _ = vault.delete_object(&trash.original_id, false);` 改为传播错误（`?`）——底层对象删除失败时中止并保留 trash 记录，避免「删了 trash 记录却留下无法再清理的孤儿对象行」。验证：`cargo test -p solosoul-core` 163 用例全绿。
- **R2-03（P1 崩溃/UX，2026-08-06）**：`execute_command` 拆分为 `execute_command`（错误捕获层）+ `dispatch_command`（命令分派，仍返回 `Result<bool>`）。所有命令错误在 `execute_command` 统一捕获并转为 `error_message` overlay，不再经 `?` 传播到 `tui.run()`/`main` 导致 TUI 进程退出；仅 `/exit` 返回 `Ok(true)`。Locked 状态手输 `/list` 等需解锁命令现在仅显示错误提示。顺带清理分派表 6 处 `&parts` 的 `needless_borrow`。新增防回归单测 `test_locked_command_error_shows_overlay_not_exit`。验证：`cargo test`（149+2+1）全绿、`cargo clippy -- -D warnings` 零告警。
- **R2-02（P1 崩溃，2026-08-06）**：`/plugin_run` 裸输（无参数）时 `&parts[2..]` 改 `parts.get(2..).unwrap_or(&[])`——一行修复消除越界 panic。新增防回归单测 `test_plugin_run_no_args_does_not_panic`（Locked 态裸输 `/plugin_run` 返回 Ok(false) 且提示用法，不崩溃不退 TUI）。验证：`cargo test`（150+2+1）全绿、`cargo clippy -- -D warnings` 零告警。
- **R2-08（P1 性能，2026-08-06）**：`collect_attachment_manifests` 的 N+1 已消除——原「`list_object_metadata`（无 properties）+ 逐对象 `load_object`（每对象一次 SQL + 一次全量解密）」改为单次 `list_objects`（一次查询、一次解码全部 properties 并直接解析 `summary.properties.__attachments`）。注：每次对附件文件 `sha256_file` 属于**正确性必需**（manifest 的 sha256 必须反映当前磁盘文件真实字节，接收端靠它校验），未做缓存；真正的查询/解密 N+1 开销已去除。验证：`cargo test -p solosoul-sync` 60 用例全绿。
- **R2-11（P2 死代码，2026-08-06）**：`llm_context.rs` 删除 `build_context` 及其整棵私有子树（7 个 section 构建器、`extract_properties`、`to_title_case`/`type_display_name`/`property_key_to_label`/`trim_to_limit`、`MAX_*` 常量，约 330 行）。因 `PROMPT_CACHE` 的唯一写入方即 `build_context`，缓存层随之成为「只清不写」死代码——`clear_cache` 及 `PROMPT_CACHE`/`CachedPrompt` 一并删除，并移除 vault.rs `lock` 与 auth.rs `logout` 中 2 处 `clear_cache()` 调用（原调用本就是空操作）。模块保留 `bump_public_data_version`/`load`/`save_public_data_version` 及其 3 个测试。验证：`cargo test -p solo_soul --lib llm` 36 用例 + `cargo clippy` 零告警。
- **R2-12（P2 死代码，2026-08-06）**：① `rag.rs` 删除 `needs_rebuild`（全 workspace 零调用；其姊妹 `mark_rebuilt` 仍被 `llm_rebuild_guide_embeddings` 使用，保留）；② `registry.rs` 删除 `PluginRegistry::from_path`——报告称零调用，经复核实际被 `src-tauri/tests/plugin_registry_update.rs` 3 处集成测试使用（生产零调用），属测试专属构造器，删除后 3 处测试改用 `new_with_dirs(dir, dir)`（`bundled_path`/`cache_path` 均为 `dir/registry.json`，与 `from_path` 语义逐位一致），并清理 `Path` 未用 import。验证：`cargo test --test plugin_registry_update` 3 用例 + `cargo test -p solosoul-plugin` 56 用例全绿、`cargo clippy` 零告警。
- **R2-14（P2 重复，2026-08-06）**：提取共享 `allowed_fs_bases()` helper——`attachment_copy_to_vault` 与 `attachment_download` 两处近乎逐字重复的 `allowed_bases` 内联块（各 ~25 行）收敛为单一实现，移动端 temp_dir 分支统一纳入（原仅 copy 侧有、download 侧无，策略漂移风险消除）。验证：`cargo test -p solo_soul attachment` 12 用例 + `cargo clippy` 零告警。
- **R2-15（P2 性能/杂项，2026-08-06）**：4 项全部处理——① **payload 流式导入**：`decrypt_package` 不再整体读入 `payload.enc`，新增 `decrypt_zip_entry_streaming`（复用 crypto 的 `decrypt_chunked_stream`，沿用 `MAX_ZIP_ENTRY_SIZE` 防 ZIP 炸弹上限）流式解密到 `NamedTempFile` 后经 `serde_json::from_reader` 解析；峰值内存由约 3×（密文+明文+JSON 树）降至约 1×。`tempfile` 从 dev-dependencies 提升至 dependencies。② **load_font_bytes 缓存**：`OnceLock` 缓存系统字体（进程内不变），消除每次图片水印重复读盘。③ **ALTER TABLE 错误吞没**：`init_schema` 的 2 处迁移改为先经 `column_exists`（PRAGMA table_info）探测、缺列才 ALTER 且错误传播。④ **示例主密码**：`examples/unlock_account.rs` 改从 `SOLOSOUL_TEST_PASSWORD` 环境变量读取（不再进 argv，防 `ps` 泄漏）。验证：export_import 36 / attachment 12 / vault 146 / watermark 5 用例全绿、example 编译通过、`cargo clippy` 零告警。

---

> 分析范围：`tauri/`（前端 + Rust workspace）、`solosoul_cli/`；忽略 `SoloSoul_plugin_market/`（独立仓库）
> **报告性质**：本报告为**全新一轮**代码审查。旧版 `CODE_ANALYSIS_REPORT.md`（2026-08-02~03，80 项闭环）已由用户决定删除且不恢复，本轮从 HEAD `22265c2d` 出发重新扫描并完成一轮迭代修复（P001/P002/P004 已闭环，P003 为长期重构候选，①② 已按该定位完成拆分）。

---

## §1 分析基线（2026-08-04 HEAD = `22265c2d`）

| 检查 | 命令 | 结果 |
|------|------|------|
| TypeScript | `npx tsc --noEmit` | ✅ 0 错误 |
| Rust 格式化 | `cargo fmt --check` | ✅ 通过 |
| Rust 静态分析 | `cargo clippy -- -D warnings`（workspace） | ✅ 0 警告 |
| ESLint | `npm run lint` | ✅ 0 错误 0 警告 |
| 前端测试 | `npm run test`（Vitest） | ✅ 55 文件 / 484 用例全绿 |
| Rust 测试 | `cargo test --workspace` | ✅ 全绿（vault 140 / solo_soul / core / crypto / plugin / sync） |
| CLI 静态分析 | `cd solosoul_cli && cargo clippy -- -D warnings` | ✅ 0 警告 |
| CLI 测试 | `cd solosoul_cli && cargo test` | ✅ 全绿 |
| ACL 一致性 | `python3 scripts/check_acl_consistency.py` | 修复前 ❌ 失败 → 修复后 ✅ **OK: 188 个命令全部登记** |

**结论**：修复前仅 ACL 一致性检查失败（P001 脚本缺陷 + P002 死命令）；修复后全部检查通过。代码库整体质量基线良好（此前的 P223-③ 分簇、P224 组件拆分等重构保持了零回归）。

---

## §2 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别       | 文件位置                                     | 描述                                             | 状态      |
|------|--------|------------|----------------------------------------------|--------------------------------------------------|-----------|
| P001 | P0     | 构建/CI    | `tauri/scripts/check_acl_consistency.py:27`  | ACL 脚本用 `re.search` 只解析**首个** `generate_handler!` 块——P223-③ 拆分为 5 簇后仅校验 core 簇，产生 68 条误报 WARN，同步/OCR/LLM/插件四簇失去校验 | `[x]` 已修复 |
| P002 | P1     | 死代码/安全 | `tauri/src-tauri/src/lib.rs`、`commands/vault.rs`、`commands/object/trash.rs` | 4 个死 IPC 命令（`object_restore`/`object_purge`/`get_state`/`delete_account`）注册于 handler 但生产前端**零调用**，且未登记 ACL 白名单 → 触发 P101 一致性检查失败 | `[x]` 已修复 |
| P003 | P2     | 重构       | `tauri/src/`（30 个文件 >400 行）             | 巨型组件长期重构候选（延续既有 P224 思路，随功能迭代顺带处理）——P003-①② ObjectDetailModal / SyncShowQrDialog 已拆分，剩余 28 个文件待后续迭代 | `[x]` P003-①② 已完成 |
| P004 | P2     | 文档同步   | `docs/design_map/08_IPC命令接口完整规范.md`、`docs/solosoul_cli/*` | 设计文档仍将 `get_state`/`delete_account`/`object_purge`/`object_restore` 列为活跃 IPC 命令（P002 删除后需同步） | `[x]` 已修复 |

## 修复进度

- 已完成：4 / 4（P003-①② ObjectDetailModal 与 SyncShowQrDialog 拆分完成，其余候选随功能迭代顺带处理）
- 当前处理：无

---

## §3 详细问题描述与修复指引

### P001（P0）ACL 一致性检查脚本失效——只解析首个 generate_handler! 块

**位置**：`tauri/scripts/check_acl_consistency.py:27`

```python
m = re.search(r"generate_handler!\s*\[((?:[^\[\]]|\[[^\[\]]*\])*)\]", text, re.S)
```

**影响**：`re.search` 仅返回第一个匹配。P223-③（commit `a7d5925d`）将原先单个 192 条命令的 `generate_handler![...]` 拆分为 `dispatch_ipc` 分发器 + `register_{core,sync,ocr,llm,plugin}_commands` 5 个独立 `generate_handler!` 块。当前脚本只解析 `lib.rs` 中**最先出现**的 `register_core_commands` 块：

1. **校验缺口**：sync / ocr / llm / plugin 四簇命令不再参与「handler ↔ 白名单」一致性校验，未来新增命令漏登记将无法被 CI 拦截；
2. **68 条误报 WARN**：白名单中 `guide_*`、`llm_*`、`ocr_*`、`plugin_*`、`sync_*` 等 68 条命令被误判为「白名单中存在但 handler 未注册」（实际均在对应簇内注册），噪声淹没有用告警。

**修复指引**：将 `re.search` 改为 `re.findall`，聚合所有 `generate_handler!` 块后再提取命令名；同时补一个回归断言（脚本自身单测或 CI 步骤确认 5 个簇全部覆盖）。

**✅ 修复说明**：`extract_handler_commands` 改用 `re.findall` 聚合全部 5 个 `generate_handler!` 块后并集提取；复跑脚本 68 条误报 WARN 全部消失，剩余缺失项收敛为 P002 的 4 个死命令。验证：`python3 scripts/check_acl_consistency.py` 仅报 P002 项。

### P002（P1）4 个死 IPC 命令未登记 ACL——应删除而非补登记

**✅ 修复说明**（详见 commit）：
1. `commands/vault.rs`：删除 `get_state`、`delete_account` 两个 `#[tauri::command]` 函数及其专用 import（`verify_password_core`/`AccountConfig`）；服务层 `get_vault_state()`/`delete_account()` 保留（CLI `/security delete-account` 与 recovery 流程依赖）；
2. `commands/object/trash.rs`：`object_restore` 移除 `#[tauri::command]` 降级为 `trash_restore` 的内部共享助手；`object_purge` 整体删除（语义已由 `trash_permanent_delete` 覆盖）；
3. `lib.rs`：从 `register_core_commands` 与 `test_dispatch_cluster_prefixes_consistent` 核心簇列表移除 4 条命令，总数 192 → 188；
4. `src/lib/ipc.test.ts`：删除 `get_state`/`delete_account` 两个 mock 测试块（12/14 用例保留）；
5. 保留：locale 键 `object_purge`/`object_restore`（历史操作日志渲染）与 `solosoul-core/src/objects.rs` 审计字符串（恢复流程共用）。

**验证**：`cargo fmt --check` ✅ / `cargo clippy -- -D warnings` ✅ 0 警告 / `npx tsc --noEmit` ✅ / `npm run lint` ✅ / `npx vitest run src/lib/ipc.test.ts` 12/12 ✅ / `cargo test -p solo_soul --lib test_dispatch_cluster_prefixes_consistent` ✅ / `check_acl_consistency.py` → **OK: 188 个命令均已登记到 ACL 白名单** ✅。

**位置**：
- `tauri/src-tauri/src/lib.rs:409/410/439/441`（handler 注册）
- `tauri/src-tauri/src/commands/vault.rs:38/61`（`get_state`/`delete_account` 定义）
- `tauri/src-tauri/src/commands/object/trash.rs:74/101`（`object_restore`/`object_purge` 定义）
- `tauri/src/lib/ipc.test.ts:76/96`（针对死命令的测试）

**证据**（全库引用核验）：

| 命令 | 生产前端调用 | 其余引用 |
|------|--------------|----------|
| `delete_account` | ❌ 无（仅 `ipc.test.ts`） | CLI 走 `svc.delete_account`（Rust 服务方法，非 IPC）；`recovery.rs` 同理 |
| `get_state` | ❌ 无（仅 `ipc.test.ts`） | `get_vault_state()` 服务方法仅被该命令与 vault 单测使用 |
| `object_purge` | ❌ 无 | 前端回收站走 `trash_permanent_delete`（`trashStore.ts`） |
| `object_restore` | ❌ 无 | 前端回收站走 `trash_restore`（`trashStore.ts`）；`trash_restore` 命令内部复用其函数体作为助手 |

**影响**：4 个命令在生产前端零调用，属死 IPC 面。由于它们不在 ACL 白名单，运行时被 Tauri 拦截（`Command not allowed by ACL`），且 `check_acl_consistency.py` 报错。**正确修复是删除死命令**（收缩攻击面，符合 P101 least-privilege 原则，与既往 P132「8 个死命令删除」先例一致），而非把它们加回白名单。

**修复指引**：
1. `vault.rs`：删除 `get_state`、`delete_account` 两个 `#[tauri::command]` 函数（保留服务层 `get_vault_state()`/`delete_account()`，CLI 与 recovery 依赖）；
2. `trash.rs`：`object_restore` 移除 `#[tauri::command]` 属性降级为内部助手（`trash_restore` 仍调用）；`object_purge` 整体删除（`trash_permanent_delete` 已覆盖其语义）；
3. `lib.rs`：从 `register_core_commands` 及 `test_dispatch_cluster_prefixes_consistent` 核心簇列表移除 4 条，总数 192 → 188；
4. `ipc.test.ts`：删除 `get_state`、`delete_account` 两个测试块；
5. 保留：`src/locales/*/settings.json` 中 `object_purge`/`object_restore` 键（历史操作日志展示仍需）；`solosoul-core/src/objects.rs` 中 `"object_restore"` 审计字符串（恢复流程共用）。

### P003（P2）前端巨型组件长期重构

**✅ P003-① 已完成（2026-08-04）：ObjectDetailModal.tsx 926 → 523 行**

按 P224 等价重构模式拆分为 4 个文件（commit 待写入）：

| 文件 | 行数 | 内容 |
|------|------|------|
| `ObjectDetailModal.tsx` | 523（原 926） | 编排层：保留全部状态/副作用/回调（fetch、生物识别、密码验证、审计日志、删除） |
| `ObjectDetailSections.tsx` | 262 | 纯展示：`ObjectDetailHeader` / `ObjectDetailTemplateSyncBanner` / `ObjectDetailDeprecatedEntry` / `ObjectDetailTags` / `ObjectDetailFooter` |
| `ObjectDetailDeleteDialog.tsx` | 73 | 删除确认对话框（原 P041 提取项，随迁） |
| `objectDetailUtils.ts` | 150 | 纯函数：`flattenProperties` / `buildDetailGuidePages` |

**验证**：tsc ✅ / eslint 0 警告 ✅ / prettier ✅ / 全量 Vitest **57 文件 / 493 用例全绿**（新增 `objectDetailUtils.test.ts` 8 用例 + `ObjectDetailModal.test.tsx` 3 渲染冒烟用例：头部/标签/底部操作栏/删除确认链路/关闭回调）/ code-reviewer-glm 确认等价（JSX 逐字、`detailTplMatch` `!!` 归一化、Footer 兜底分支、SyncBanner 可空 onDismiss）✅。

**新增 P2 去重项**（审查员建议，随本项记录）：`flattenProperties` 现存于 3 处（`objectDetailUtils.ts` / `HistoryViewer.tsx` / `WorkspaceObjectCard.tsx`），返回类型与 `__` 键语义不同（HistoryViewer 为树结构 `FlattenedField[]`，其余为平铺行），统一需改行为——列入后续 P225 式收敛候选，暂不实施。

**✅ P003-② 已完成（2026-08-04）：SyncShowQrDialog.tsx 878 → 270 行**

按 P224 等价重构模式拆分为 5 个文件：

| 文件 | 行数 | 内容 |
|------|------|------|
| `SyncShowQrDialog.tsx` | 270（原 878） | 编排层：保留全部状态/副作用/回调（`recoveryStartedRef` 生命周期、`[isOpen, t]` 加载 effect + 10s 超时保护、卸载兜底 cancel、`copyToClipboard` execCommand 回退） |
| `SyncQrTabSwitcher.tsx` | 80 | Tab 切换器 + `QrMode` 类型 |
| `SyncQrContent.tsx` | 177 | 同步二维码内容 + `SyncQrInfo` 类型 |
| `RecoveryQrContent.tsx` | 391 | 恢复二维码内容（含手动模式折叠面板）+ `RecoveryHostInfo` 类型 |
| `QrStatusBlock.tsx` | 52 | 加载/错误共享占位（原件两处 ~25 行占位**逐字**合并，消除 ~50 行重复） |

**验证**：tsc ✅ / eslint 0 警告 ✅ / prettier ✅ / 全量 Vitest **58 文件 / 498 用例全绿**（新增 `SyncShowQrDialog.test.tsx` 5 用例：关闭不渲染/同步加载链路/恢复会话启动 + PIN/加载失败错误占位/关闭回调）/ code-reviewer-glm ✅——`QrStatusBlock` 与原两处占位经 `git show HEAD` 逐字比对一致（`minHeight:360` + `t('common:loading')` / `#e74c3c` 错误样式），props 透传未改变回调语义（`switchMode`/`cancelRecoveryHost`/`handleClose` 收敛于编排层），审查建议的错误路径测试已补充闭环。

**剩余候选**：`tauri/src/` 中 28 个文件 >400 行，前五：

| 行数 | 文件 |
|------|------|
| 793 | `src/pages/auth/LoginPage.tsx` |
| 754 | `src/pages/settings/ExportImportPage.tsx` |
| 743 | `src/components/object/AttachmentViewer.tsx` |
| 699 | `src/components/guide/PageGuide.tsx` |
| 682 | `src/components/object/HistoryViewer.tsx` |

**定位**：延续既有 P224 思路（「结构性拆分建议随功能迭代顺带、不单独安排修复轮次」）。本轮完成 P003-①②，其余候选随功能迭代顺带处理。拆分时应保持「等价重构、零行为变更」，并复用已收敛的共享组件（`SensitiveValueWidget`、`useConfirm`、`useSyncPage` 等）。

### P004（P2）设计文档与 IPC 面同步

**位置**：
- `docs/design_map/08_IPC命令接口完整规范.md`（命令总览、Vault/Object 模块签名、安全约束表）
- `docs/solosoul_cli/solosoul_cli_research_report.md`（CLI↔IPC 映射表）
- `docs/design_map/09_对象规范.md` §4.6（回收站 invoke 示例）
- `tauri/docs/design_map/tauri_dev_plan.md`、`docs/design_map/12_状态管理_Zustand_Store设计.md`（历史注释标注）

**影响**：文档描述与代码面不一致，误导后续开发。随 P002 的删除一并更新（同一根因）。

**✅ 修复说明**：
1. `08`：命令总览 Vault 10→8、Object 8→6；Vault/Object 模块签名块移除 4 个命令；安全约束表移除 `delete_account` 密码接收项；顶部标注「权威来源为 ACL/handler」；
2. CLI 预研报告：映射表移除 `get_state`/`delete_account`/`object_restore`/`object_purge` 行并标注；
3. `09` §4.6：invoke 示例改为 `trash_restore`/`trash_permanent_delete`（代码审查员复核发现的主要缺口）；
4. `tauri_dev_plan`（历史审计文档）与 `12`（vaultStore 已合并入 authStore）加历史标注，不重写既有历史记录。

---

## §4 有意保留 / 误报说明

| 项 | 判定 | 说明 |
|----|------|------|
| `unsafe` 块 | ✅ 设计如此 | 仅存在于平台 FFI（`biometric/*`、`commands/window.rs`、`commands/system.rs`），为 macOS/Windows 系统 API 调用所必需，无裸指针越界风险 |
| 前端 `console.warn/error` | ✅ 设计如此 | 仅 `lib/logger.ts`（调试模式收敛）与 `lib/ipcClient.ts`（仅 dev 生效） |
| 非测试 `panic!`/`expect` | ✅ 无风险 | 均为启动期静态内容（`build.rs`、`i18n` 语言标识）或测试代码 |
| `get_vault_state()` 服务方法 | ✅ 保留 | 虽仅被 `get_state` 命令使用，但为 `VaultService` 公共 API（CLI/未来调用方），移除命令时保留 |
| 历史审计字符串 `object_purge`/`object_restore` | ✅ 保留 | `solosoul-core` 与 locale 键用于渲染历史操作日志，不可随命令删除 |
| 路径净化 | ✅ 已覆盖 | `sanitize_file_name`/`sanitize_import_file_name`/`sanitize_plugin_id`/`sanitize_backup_name` 均有穿越用例测试 |
| XSS | ✅ 干净 | 生产代码零 `dangerouslySetInnerHTML`/`innerHTML` |

---

## §5 最终复审结论（R1 收尾，2026-08-04）

### 5.1 提交记录

| commit | 内容 |
|--------|------|
| `480c2b1f` | P001：`check_acl_consistency.py` 聚合全部 5 簇 `generate_handler!` 块（`re.search`→`re.findall`），消除 68 条误报 WARN，恢复 sync/ocr/llm/plugin 四簇校验面 |
| `c1b30238` | P002：删除 4 个死 IPC 命令（`object_restore`/`object_purge`/`get_state`/`delete_account`），handler 面 192→188，ACL 一致性检查恢复通过 |
| `04c2e66e` | P004：08 IPC 规范 + CLI 预研报告同步命令面 |
| `d1d69f5a` | P004 补充：09 对象规范 invoke 示例 + tauri_dev_plan/12 store 设计历史标注（审查员复核缺口） |
| `5f2519ff` | P003-①：ObjectDetailModal 926→523 行等价拆分（Sections/DeleteDialog/Utils + 11 新测试） |
| `fed8d393` | P003-②：SyncShowQrDialog 878→270 行等价拆分（TabSwitcher/SyncQrContent/RecoveryQrContent/QrStatusBlock + 5 新测试） |

### 5.2 修复后全量验证

| 检查 | 结果 |
|------|------|
| `npm run check-all`（tsc + fmt + clippy + eslint + vitest + ACL） | ✅ 全绿；Vitest 58 文件 / 498 用例（482 + 16：P003-①② 新增测试）；ACL `OK: 188` |
| `cargo test --workspace` | ✅ 全绿（core 162 / crypto 34 / plugin 56+2ignored / sync 47 / vault 140 / solo_soul 357 等） |
| `cd solosoul_cli && cargo clippy -- -D warnings && cargo test` | ✅ 全绿 |
| 代码审查（code-reviewer-glm） | ✅ 正则与删除面无误；发现 P004 文档同步缺口（09/tauri_dev_plan/12）→ 已补齐 `d1d69f5a` |

### 5.3 结论

- **P0/P1 全部闭环**：ACL 一致性检查从 ❌ 恢复 ✅，188 个 IPC 命令 handler↔白名单双向一致；
- **P2**：P004 已闭环；P003-①② ObjectDetailModal（926→523）与 SyncShowQrDialog（878→270）拆分完成，剩余 28 个候选随功能迭代顺带处理；
- **遗留项**：`08_IPC命令接口完整规范.md` 中部分更早的命令名（如 `profile_save`/`search_advanced`/crypto 模块）为设计期陈旧描述，已由顶部「权威来源为 ACL/handler」声明覆盖，不属本轮范围。

✅ 本轮可识别问题已修复，代码库质量评估达标。
