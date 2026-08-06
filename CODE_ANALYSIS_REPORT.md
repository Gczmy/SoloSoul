# SoloSoul 代码分析修复报告

> 最后更新：2026-08-06
> 当前分支：`main`
> 修复轮次：R1 已闭环；R2 修复 28 项已提交；V1-V8 经第二轮验证后产生 4 项跟进问题（W1-W4），**已全部修复（2026-08-06）**：W1（P1 安全，attachment_open/copy_to_vault symlink 旁路硬化 `c312fe84`）、W2（CLI 21 键成功/信息语义收敛 `14609bc5`）、W3（CLI 8 处样板收敛 `0bee6cb7`）、W4（前端 156 处死兜底 `9747f573`）。待第三轮复审后 R2 可标记终版（详见 §R2-§7）

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
| R2-13 | P2 | 结构 | Rust 超长/深嵌套函数 | `app_state.rs:257`（189 行/深 10）、`search/query.rs:16`（120/深 10）、`import.rs:226`（213）、`storage.rs:649`（211）与 `:884 reencrypt_all`（207，6 个表处理块复制粘贴）、`plugin/manager.rs:170`（209）、`plugin/host.rs:437/893/662`、`sync/attachments.rs:387` 等 | `[x]` 长期重构候选（与 P223 同类但文件不在 P223 拆分清单内，随功能迭代顺带处理） |
| R2-14 | P2 | 重复 | `tauri/src-tauri/src/commands/attachment.rs:394-418 vs 823-846` | `allowed_bases` 白名单构建块两处近乎逐字重复（~90%），且一处含移动端 temp_dir 分支一处不含——策略漂移风险 | `[x]` 已修复 |
| R2-15 | P2 | 性能/杂项 | Rust 轻项 | 主 `payload.enc` 导入未走流式（`import.rs:524`，≈2×payload 内存）；`watermark/mod.rs:88 load_font_bytes` 无缓存；`storage.rs:541` ALTER TABLE 吞掉所有错误；`examples/unlock_account.rs:8` 主密码放 argv | `[x]` 已修复 |
| R2-16 | P2 | 重构 | `tauri/src/`（>400 行文件 28→**40** 个） | P003 清单过期：前五仍准（LoginPage 793/ExportImportPage 754/AttachmentViewer 743/PageGuide 699/HistoryViewer 682），新进：AppRoutes 679、useObjectWorkspaceData 629、PasswordVerificationDialog 617、useAttachmentManager 585、TrashDetailSections 575、AddPageButton 555 | `[x]` 长期重构候选（延续 P003 定位，随功能迭代顺带拆分，不单独安排轮次） |
| R2-17 | P2 | 重复/性能 | `tauri/src/hooks/useExportScope.ts:89-106/223-241/254-270` | 同一段 N+1 附件加载块逐字复制三遍（~55 行），每对象一次 IPC 且无并发上限 | `[x]` 已修复 |
| R2-18 | P2 | 规范 | 前端 100 处 | `t('key') \|\| '兜底文案'` 死兜底模式：i18next 缺 key 返回 key 本身（truthy），`\|\|` 右侧几乎永不执行；100 处硬编码与 i18n 集中管理约定相悖 | `[x]` 已修复（2026-08-05：109 处简单形态转 `t(key, { defaultValue })`，4 处 `searchParams.get()` 误伤已回滚，TemplateEditor 测试断言同步更新） |
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

- 已完成：28 / 28（27 项修复 + R2-16 长期候选；R2-13 同样归入长期重构候选，与 P223 同类）；R2-16 为长期重构候选
- 当前处理：全部完成
- **验证（2026-08-05，HEAD `e13cd14f`）**：28 项全部经逐 diff 代码核验——✅ 19 项正确 / ⚠️ 6 项有残留或隐患 / ❌ 1 项不完整（R2-26）/ 2 项长期候选（R2-13/16 不适用）；详见 §R2-§6，产生 8 项跟进问题（V1-V8，均 `[ ]`）

### R2-§6 修复验证（2026-08-05，验证 HEAD = `e13cd14f`）

> 验证方法：对每项修复 commit 逐 diff 审读 + 当前代码上下文核验（非凭 commit message）；基线全绿：`npm run check-all`（59 文件 / 560 用例，ACL 188 ✅）、CLI clippy 0 警告 + 153 测试全绿。另外抽查了 R2 批次之后的 sync 功能提交（`5544b441`/`e13cd14f`/`94815431`/`b8a31c3c`），无明显问题（15s 轮询卸载清理到位、TCP 探测解析方式与真实连接一致）。

#### 6.1 逐项结论

| ID | 结论 | 说明 |
|----|------|------|
| R2-01 | ✅（含 1 硬化缺口 → V8） | 字符串前缀回退已移除，组件级比较 + 双侧拒绝 `..`，Android SAF 合法场景未误伤。残留：canonicalize **成功**时仍求值 `src_raw` 字面路径分支（`attachment.rs:824-825`），vault 内被植入指向外部的 symlink 可旁路（原代码遗留、利用门槛高，建议仅 canonicalize 失败时启用 raw 比较） |
| R2-02 | ✅ | `parts.get(2..).unwrap_or(&[])`，len==2 空参数行为正确，附防回归测试 |
| R2-03 | ✅（→ V1 已修复） | 6 处按键 handler 已由 handle_key 统一捕获层承接（`1ddde2df`），错误转为 overlay 不再退出 TUI；i18n 文案保留 |
| R2-04 | ✅ | `PromptState.value`/`PromptResult::Text` 均 `Zeroizing<String>`，敏感调用方全覆盖，闭包 move 捕获无 String 副本 |
| R2-05 | ✅ | TTC 改内存加载，pdfium 源码核实与文件加载语义完全等价；NamedTempFile 彻底移除，注释同步 |
| R2-06 | ✅ | `save_profile` 失败改 `?` 传播；对象已落库但报错诚实，重试安全（非原子语义，前端文案如承诺全撤需对齐——未核） |
| R2-07 | ✅ | `delete_object` 失败中止，trash 记录保留可重试，孤儿对象路径消除 |
| R2-08 | ⚠️ 部分（→ V8 知情项） | N+1 解密已消除（单次 `list_objects`）；sha256 仍逐文件重算（理由成立：manifest 须反映磁盘真实字节）；失败语义变严——单行解密失败即整批 Err（原为静默跳过坏对象） |
| R2-09 | ✅（→ V3 已修复） | restore 回调内层 catch 后重新 throw（`62a9acd8`），失败保持对话框可重试 |
| R2-10 | ✅（→ V5 已修复） | 5 处列表导航 handler 已改为就地改 selected（`c1380b22`），items.clone 全库清零 |
| R2-11 | ✅ | 死子树删除干净，活路径（`bump_public_data_version` 等）完整，测试同步 |
| R2-12 | ✅ | 无残留引用；3 处集成测试改 `new_with_dirs` 语义逐位一致 |
| R2-14 | ✅ | 共享 helper 提取正确；移动端 download 侧纳入 temp_dir 属合理统一（放宽的是应用私有缓存目录，无安全后果） |
| R2-15 | ✅（1 项知情） | 四项均正确：流式导入完整复刻防 ZIP 炸弹边界、字体 OnceLock 缓存、ALTER TABLE 走 PRAGMA 检查、示例密码改 env。知情项：明文 payload 现短暂落盘 OS tempdir（0600，drop 即删，崩溃可能残留） |
| R2-16 | — | 长期重构候选，无 commit，不适用 |
| R2-17 | ✅ | helper 收敛语义逐字等价，单对象容错与双 setState 回写保留；N+1 无上限属有意保持 |
| R2-18 | ✅（→ V4 已修复） | 带插值死兜底 12 处（含 HistoryPage 残留）已全量改 defaultValue（`b11b2654`），全库正则扫描清零 |
| R2-19 | ✅ | `prevName` 乐观更新前捕获，catch 按 id 回滚 + toast，无快照过期问题 |
| R2-20 | ✅ | worker 池上限 8，游标同步段内自增无竞争，失败语义与 R2-09 兼容 |
| R2-21 | ✅ | 多余 export 移除干净；保留项（SnapshotDataView 等）经核实被测试 import，属有效导出 |
| R2-22 | ⚠️ 轻微（→ V7） | Theme 缓存安全（`/theme` 偏好不参与 `Theme::load`，"切换不生效"担忧不成立）；但 `/list` 静默 `truncate(200)` 无截断提示（`/search` 有，不对称） |
| R2-23 | ⚠️ 轻微（→ V7） | OnceLock 单例正确、无嵌套 block_on 风险；但初始化失败从优雅报错退化为 `expect` panic（概率极低） |
| R2-24 | ✅ | 创建即 0600（0644 窗口内文件为空），覆盖已存在文件场景也处理 |
| R2-25 | ✅ | 白名单 + 裸级别丢弃逻辑正确（EnvFilter 最长前缀优先）；按日轮转命名与 `latest_log_path` 匹配 |
| R2-26 | ✅（→ V2 已修复） | 复核发现 5 处成功路径残留红色 overlay，已由 V2 全部改 success_message toast（`8eed85e6`），至此 22 处全部闭环 |
| R2-27 | ✅ | 无残留引用 |
| R2-28 | ✅（→ V6 已修复） | V6 再收敛 5 处纯样板（`1c4de03e`）；剩余 `require_unlocked` 使用点均因 account_id 参与参数校验/错误消息需保留，样板收敛完成 |
| R2-13 | — | 长期重构候选（与 P223 同类），无 commit，不适用 |

#### 6.2 跟进问题清单（新发现问题，待修复）

| ID | 优先级 | 来源 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| R2-V1 | P1 | R2-03 残留 | `solosoul_cli/src/app.rs:2022/2162/2167/2533/2545/2554`、`commands/mod.rs:12-13` | 6 处按键 handler 错误仍 `?` 传播退出 TUI（自动锁定后操作即触发）；overlay 捕获用英文 `e.to_string()` 覆盖 i18n 文案 | `[x]` 已修复（`1ddde2df`：handle_key 统一捕获层 + execute_command 保留 i18n 文案） |
| R2-V2 | P1 | R2-26 不完整 | `log.rs:81`、`vault_write.rs:548`、`plugin.rs:215/252`、`security.rs:330` | 5 处成功消息仍显示为红色「! 错误」overlay，commit 声称「全部 22 处」不实 | `[x]` 已修复（`8eed85e6`：5 处成功路径改 success_message toast） |
| R2-V3 | P2 | R2-09 残留 | `tauri/src/pages/settings/TrashPage.tsx:193-195` | restore 回调内层吞错：失败时对话框仍关闭（与 delete 路径行为不一致），外层 catch restore 分支为死代码 | `[x]` 已修复（`62a9acd8`：内层 catch 后重新 throw，外层保持对话框） |
| R2-V4 | P2 | R2-18 扫尾 | `GlobalAttachmentManager.tsx:251/264/277`、`useAttachmentManager.ts:114/377/420/463`、`AttachmentToolbar.tsx:104/126`、`AttachmentViewer.tsx:87/229` | 11 处带插值的 `t(key, {...}) \|\| fallback` 死兜底遗漏，应同样改 defaultValue | `[x]` 已修复（`b11b2654`：12 处含 HistoryPage 同类残留，全库扫描清零） |
| R2-V5 | P2 | R2-10 残留 | `solosoul_cli/src/app.rs:2008/2017/2046/2074/2096/2501` | 5 处列表导航 handler 仍每按键克隆整个 items Vec，应 `&mut self.phase` 就地改 selected | `[x]` 已修复（`c1380b22`：先算 NavAction 再就地改 selected，items.clone 清零） |
| R2-V6 | P2 | R2-28 不彻底 | `solosoul_cli/src/commands/`（vault_write.rs:131-135、search.rs:89-92、history.rs:21-24、log.rs:18-20 等 20+ 处） | 解锁样板收敛仅完成 6/~40 处，剩余纯样板继续收敛到 `require_unlocked_with_vault` | `[x]` 已修复（`1c4de03e`：5 处纯样板收敛；account_id 参与校验的非纯样板保留） |
| R2-V7 | P2 | R2-22/23 轻微 | `solosoul_cli/src/commands/vault_read.rs:23/45`、`util.rs:12` | a) `/list` 截断 200 无用户提示（与 /search 不对称）；b) `shared_runtime()` 初始化失败从优雅报错退化为 `expect` panic | `[x]` 已修复（`33127c52`：ObjectList.truncated 提示 + shared_runtime 返 Result 优雅降级） |
| R2-V8 | P2 | 硬化/知情项 | `attachment.rs:824-825`；sync `attachments.rs`；`helpers.rs:298-324` | a) `src_raw` 字面路径比较建议仅在 canonicalize 失败时启用（symlink 旁路硬化）；b) 知情：R2-08 失败语义变严（单行坏即整批失败）、R2-15a 明文 payload 短暂落盘 tempdir——确认接受即可关闭 | `[x]` 已修复+确认（`9254248e`：src_raw 仅 canonicalize 失败时参与判定；两项知情项用户 2026-08-06 确认接受） |

#### 6.3 验证总结

- **整体结论**：28 项修复质量良好，无方向性错误、无回归（基线全绿），P0 的 R2-01 修复正确。**但「全部闭环」的宣称不完全属实**：R2-26 明显不完整（❌），R2-03/09/10/18/28 存在残留或不彻底（⚠️）。
- 跟进问题 8 项（V1-V8）：P1×2（V1 崩溃残留、V2 成功消息误标）、P2×6。按流程阶段 4，V1/V2 为 P1 级 → 不应标记本轮终版，待修复后复审。
- ⚠️ **注（2026-08-06 第二轮验证）**：`cc392b34` 将本节 6.1/6.2 各行直接改写为「已修复」并宣称 V1-V8 全部闭环、R2 终版。经逐 diff 复核，**该宣称不成立**：V2、V6、V8a 三项不完整（V8a 升级为 P1 安全项 W1）。以 §R2-§7 为准。

### R2-§7 V 批次验证（2026-08-06 第二轮，验证 HEAD = `cc392b34`）

> 开发者提交 V1-V8 修复（`1ddde2df`~`9254248e`）并宣称全部闭环、R2 标记终版（`cc392b34`）。本节为逐 diff 独立复核结果。基线全绿：`npm run check-all`（59 文件 / 560 用例，ACL 188 ✅）、CLI clippy 0 警告 + 153 测试全绿。
> **结论：宣称不属实——8 项中 5 项正确，3 项不完整（V2、V6、V8a）。**

#### 7.1 逐项结论

| ID | 结论 | 说明 |
|----|------|------|
| R2-V1 | ✅ | 改为 `handle_key` phase 分发处统一捕获层（`app.rs:847-897`），比逐点修覆盖面更全；i18n 文案保留问题解决（`error_message.is_none()` 时才写英文兜底）；到达 `tui.rs:74` 的仅剩终端 IO 错误（合理退出）。轻微遗留：`search.rs:492-500` `open_selected` 用硬编码中文「未登录」而非 i18n `cmd-need-unlock` |
| R2-V2 | ❌ 不完整（→ W2） | 原 5 处修复正确，但**开发者显然未做全库 grep**——仍有 13 处成功语义写红色 error overlay：`attachment.rs:113/148/201-202/297-298/316`（同文件 :241 已迁移，自相矛盾）、`vault_write.rs:347/478/502`（:478/:502 就在已修复的函数体内）、`history.rs:153`、`sync.rs:165-169/186`、`security.rs:179/286`；另有 6 处信息/进度语义项可争议（`security.rs:160/210`、`settings.rs:278/282/295/299`、`plugin.rs:135/460`） |
| R2-V3 | ✅ | 内层吞错改 rethrow（`TrashPage.tsx:193-198`），restore/delete 两路径行为一致（失败保持对话框打开）。轻微瑕疵：失败时双重 toast（内层 :194 + 外层 :494 各弹一次） |
| R2-V4 | ✅ | 11 处全部改 defaultValue 且实际更全（17 个调用点，commit 说 12 处属良性少计）；插值共存语义正确；`t(...) \|\| ...` 形态残留为 0。**新发现（→ W4）**：同类 `t(key) ?? '...'` 死兜底约 90 处（guide 文案，15 个文件）未处理，超出原 V4 范围 |
| R2-V5 | ✅ | 5 处全部改 `&mut self.phase` 就地改 selected，全库 `items.clone()` 残留为 0，边界守卫等价 |
| R2-V6 | ❌ 不完整（→ W3） | 5 处转换正确，但仍有 **8 处纯样板未收敛**：`log.rs:12-21`、`search.rs:80-92`、`history.rs:12-24`、`vault_read.rs:67-80`、`vault_write.rs:23-35/168-180/230-242/517-529`——按 commit 自述判据（account_id 不参与中间校验即收敛）也应收敛，自述理由与代码事实不符（`:517-529` 的 `_account_id` 甚至未使用）。合理例外（i18n 变体/无 App 上下文/内部 helper）核验属实 |
| R2-V7 | ✅ | /list 截断提示与 /search 对称（标志位 + 中英 FTL 键）；`shared_runtime()` 改 `Result` 且 6 个调用点全适配，panic 路径消除。小瑕疵：FTL 硬编码「200」与常量耦合；3 处降级文案硬编码中文未 i18n |
| R2-V8a | ❌ 不完整（→ W1） | **`attachment_download` 已正确修复**（`src_canonicalized` 标志，Android SAF 回退未误伤）；但 **`attachment_open` 完全未修**（`attachment.rs:945-957`、`:976` `path_raw` 无条件参与判定）——同型 symlink 旁路仍在，且 open 以系统默认应用打开库外文件，比 download 更危险。另 `attachment_copy_to_vault:404-435` 有较轻的 raw/canonical 混用（fail-open 但仅允许库内自引用，风险低），建议一并统一 |

#### 7.2 跟进问题清单（第二轮，待修复）

| ID | 优先级 | 来源 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| R2-W1 | **P1（安全）** | V8a 不完整 | `tauri/src-tauri/src/commands/attachment.rs:945-957/976` | `attachment_open` 未应用 `src_canonicalized` 模式：symlink 旁路仍可以系统默认应用打开 vault 外任意文件，按 download 同款模式补齐；顺带统一 `attachment_copy_to_vault:404-435` 的 raw/canonical 混用 | `[x]` 已修复（2026-08-06 `c312fe84`） |
| R2-W2 | P2 | V2 不完整 | CLI `commands/`：`attachment.rs:113/148/201-202/297-298/316`、`vault_write.rs:347/478/502`、`history.rs:153`、`sync.rs:165-169/186`、`security.rs:179/286` | 13 处成功语义仍写红色 error overlay（另 6 处信息/进度语义项一并评估）；修复时以 grep 全量清单验收 | `[x]` 已修复（2026-08-06 `14609bc5`，13 处成功 + 8 处信息/进度共 21 键收敛，测试断言同步） |
| R2-W3 | P2 | V6 不完整 | CLI `commands/`：`log.rs:12-21`、`search.rs:80-92`、`history.rs:12-24`、`vault_read.rs:67-80`、`vault_write.rs:23-35/168-180/230-242/517-529` | 8 处纯解锁样板未收敛到 `require_unlocked_with_vault`；并修订 V6 commit 中不实的保留理由 | `[x]` 已修复（2026-08-06 `0bee6cb7`，8 处全收敛 + 导入整理 + V6 理由修订） |
| R2-W4 | P2 | V4 新发现 | 前端 guide 文案 15 个文件（TrashPage.tsx:94-123、useSyncPage.ts:71-97、workspaceGuidePages.ts:29-66、PageGuide.tsx 等） | 同类 `t(key) ?? '...'` 死兜底约 90 处（`??` 同样永不生效），按 V4 同款 defaultValue 模式处理 | `[x]` 已修复（2026-08-06 `9747f573`，实际 156 处——纯 key 155 + 带插值 1，16 文件） |

#### 7.3 第二轮验证总结

- **5 项正确**：V1（统一捕获层设计优于逐点修）、V3、V4、V5、V7。
- **3 项不完整 → W1-W4 全部闭环（2026-08-06）**：**W1（P1 安全）**`c312fe84`——attachment_open/copy_to_vault 应用 src_canonicalized 模式，symlink 旁路关闭；**W2**`14609bc5`——CLI 21 个成功/信息语义键全量收敛 success_message（grep 清单验收为 0 残留）；**W3**`0bee6cb7`——CLI 8 处纯解锁样板收敛 + V6 不实理由修订；**W4**`9747f573`——前端 156 处 t(key) ?? 字面量死兜底改 defaultValue。
- 轻微瑕疵（不阻塞，不立项）：V1 open_selected 硬编码「未登录」；V3 失败双重 toast；V7 FTL 硬编码「200」与降级文案未 i18n。
- 按流程阶段 4：W1-W4 已全部修复并经编译/测试验证 → 待第三轮复审后可将 R2 标记终版。

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
- **R2-04（P1 安全，2026-08-05）**：prompt 字段编辑与密码采集的 `String` 多副本流转改 `Zeroizing<String>`——改主密码/导出密码/删除账户等敏感输入经 prompt 后不再残留堆内存。涉及 `prompt.rs`、`security.rs`、`export_import.rs`、`app.rs` 多处，新增/更新 zeroize 相关测试。验证：`cargo test` 全绿、`cargo clippy -- -D warnings` 零告警。
- **R2-10（P1 性能，2026-08-05）**：每按键 `self.phase.clone()` 深拷贝整个 AppPhase（含大列表 items）改借用/引用传参——10+ 处调用点消除大 Vault 下 TUI 掉帧根源。验证：`cargo test` 全绿、`cargo clippy -- -D warnings` 零告警。
- **R2-22（P2，2026-08-05）**：CLI 第三批修复项之一，随批次提交。验证：`cargo test` 全绿、`cargo clippy -- -D warnings` 零告警。
- **R2-23（P2，2026-08-05）**：CLI 第三批修复项之一，随批次提交。验证：`cargo test` 全绿、`cargo clippy -- -D warnings` 零告警。
- **R2-24（P2 安全，2026-08-05）**：`/export_log` 审计日志导出文件权限从默认（通常 0644）收紧为 0600——`fs::write` 改 `OpenOptions` 显式写 + `cfg(unix)` 下 `set_permissions(0o600)`（与 solosoul-plugin store.rs / vault_service.rs 既有先例一致）。验证：`cargo fmt` / clippy 零告警 / `cargo test export_log` 通过。
- **R2-25（P2 安全，2026-08-05）**：日志治理——① `EnvFilter` 加 crate 白名单（solosoul_cli/solosoul_core/crypto/sync/vault/plugin）：`RUST_LOG=debug` 裸级别不再把依赖日志泄入 cli.log；② `rolling::never` 改 `rolling::daily`，单文件不再无限增长；③ `App.log_path` 改 `latest_log_path()`：轮转后按 mtime 找最新 `cli.log*`，doctor 展示路径与实际文件一致。验证：fmt/clippy 零告警 / `cargo test` 151+2 全绿。
- **R2-26（P2 UX，2026-08-05）**：22 处成功消息（导出/导入成功、密码修改、生物识别启用/禁用、profile 更新、删除/恢复/安装/卸载等）从红色 `error_message` overlay 改 `success_message` toast——9 个命令文件补 `use std::time::Instant`，8 处测试断言同步更新（错误类断言保持 error_message）。验证：fmt/clippy 零告警 / `cargo test` 151+2 全绿。
- **R2-27（P2 死代码，2026-08-05）**：删除 `render_empty`（account_list.rs，零调用方 + 配套 Rect/Text/Line imports 清理）与 `CliError` type（commands/mod.rs，定义后零使用）。验证：fmt/clippy 零告警 / `cargo test` 151+2 全绿。
- **R2-28（P2 结构，2026-08-05）**：解锁样板收敛——vault_write/vault_read/log/backup/settings 6 处「require_unlocked + get_vault_store().ok_or_else」双行样板合并为 `require_unlocked_with_vault(app)?` 单行（vault_read 的 open 因中间有 id 处理保留原样）；settings 替换 import，其余 4 文件保留原 import 并新增 with_vault。render 312 行 / handle_onboarding_key 149 行等 god-object 长函数拆分列为长期重构候选。验证：fmt/clippy 零告警 / `cargo test` 151+2 全绿。
- **R2-09（P1 UX/错误处理，2026-08-05）**：回收站确认操作失败不再 unhandled rejection——TrashConfirmDialog 加 submitting 态（提交中禁用确认/取消/遮罩点击，按钮显示「加载中」），TrashPage onConfirm 包 try/catch（失败时 onError toast + 保持对话框打开可重试）。验证：tsc 0 / eslint 0 / vitest 59 文件 560 用例全绿。
- **R2-19（P2 UX，2026-08-05）**：AttachmentViewer 重命名乐观更新失败回滚——乐观更新前记录 `prevName`，`attachment_rename` 改 await + try/catch：失败时回滚为原名 + showToast（common:rename_failed 既有 key）。验证：tsc 0 / eslint 0。
- **R2-17（P2 重复/性能，2026-08-05）**：useExportScope 三处逐字重复的 N+1 附件加载块（togglePage / loadSelectedAttachments / bulkSelect，各 ~20 行）收敛为共享 `loadObjectAttachments(ids)` helper，行为逐字等价；顺带清理 2 处多余 accountId 依赖。验证：tsc 0 / eslint 0 / vitest src/hooks 全绿。
- **R2-20（P2 性能，2026-08-05）**：trashStore `permanentDelete` 并发 worker 池上限 8——P052 有意改 Promise.all 无上限后清空数百条时瞬间数百 invoke，现游标共享逐条取任务，整体语义不变。验证：tsc 0 / eslint 0 / vitest src/stores 152 用例全绿。
- **R2-18（P2 规范，2026-08-05）**：109 处 `t('key') || '兜底'` 死兜底转 `t('key', { defaultValue: '兜底' })`（i18next 缺 key 返回 key 本身 truthy，`||` 右侧永不执行）；4 处 `searchParams.get() || ''` 等非 i18n 形态被正则误伤已逐一回滚；TemplateEditor.test 9 处断言从 key 文本改为渲染值（mock 现返回 defaultValue）。验证：tsc 0 / eslint 0 / vitest 59 文件 560 用例全绿。
- **R2-21（P2 死代码，2026-08-05）**：移除多余 export——useOnboarding `baseSteps`、searchShared `resolveFieldLabel`、conflictFieldMeta `formatTimeValue` 仅本文件内部使用却 export；SnapshotDataView/normalizeFieldKey/nestedFieldLabel 被跨文件测试真实 import，属有效导出保留。验证：tsc 0 / eslint 0 / vitest conflictFieldMeta + useOnboarding 50 用例全绿。

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
