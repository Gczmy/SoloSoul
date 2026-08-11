# 代码分析修复报告

> 最后更新：2026-08-11 19:15
> 当前分支：`main`
> 修复轮次：3（N 系列修复验证轮）

## 基线验证（本轮实测）

| 检查 | 结果 |
|------|------|
| `npm run check-all`（N007 后已含 cargo test） | ❌ **EXIT=101**：cargo test 411 通过 / **1 失败**（`test_text_markdown_multi_object_separator`，见 R001）；其余阶段（tsc/fmt/clippy/lint/Vitest/ACL）未执行到 |
| `cargo check --target aarch64-linux-android` | ✅ 通过（仅 5 个 warning）——N002 缺陷已消除 |
| `cd solosoul_cli && cargo test` | ✅ 通过——N003 缺陷已消除 |

## 轮次 2 问题验证结果（N001–N010）

状态图例：`[x]` 验证通过关闭 · `[!]` 未完全关闭 · 残留子项转 R 系列跟踪

| ID | 优先级 | 状态 | 验证备注 |
|----|--------|------|----------|
| N001 | P0 | `[!]` | 修复方式正确（纯测试侧对齐，未反向改实现迁就断言；escape_markdown 断言与实现逐条一致；fixture 补 createdAt/template_id 对症；fs 测试改 tokio::test 对症）。**但 9 个红测试只修了 8 个，遗漏 1 个 → 转 R001，check-all 仍红** |
| N002 | P1 | `[x]` | cfg 门控口径与调用矩阵逐平台吻合（Linux 上 copy_to_share_dir_async 被 share_linux 调用非 dead code）；share_linux 注释已修正；**Android 交叉编译实测通过** |
| N003 | P1 | `[x]` | 两行已删，CLI 全仓无残留；**CLI cargo test 实测通过** |
| N004 | P1 | `[x]` | 主 bug 已修（按主键 id 取数，id 为 TEXT PRIMARY KEY 全局唯一无跨账户风险，新单测通过）。**残留：假冲突抑制对 conversations 永不触发（快照 JSON 与线格式信封键形错配）→ 转 R002** |
| N005 | P2 | `[x]` | 懒迁移下沉单一实现、updated_at LWW（相等时现有行赢）、幂等、CLI 读取路径全覆盖均正确，LWW 单测通过。**残留：混合版本同步缓解（延迟删键/文档警示）被静默丢弃 → 转 R003** |
| N006 | P2 | `[x]` | find_apk_asset 收紧为纯 .apk 匹配，与发布脚本产出及现有 Release 资产命名一致；2 个谓词单测覆盖误命中场景；发布侧未动无兼容风险 |
| N007 | P2 | `[x]` | check-all 已插入 cargo test（位置合理），与 CI（本就含 cargo test）对齐 |
| N008 | P2 | `[x]` | 恢复逐字段更新后与 P034 修复前语义逐行等价，两处调用点均已修；无测试（逻辑简单低风险，已记入 R005 汇总） |
| N009 | P2 | `[x]` | 误挂属性已删；三组边界单测覆盖良好（±1 边界值）；当前白名单与前端 20 个 key 手工核对重合。**残留：白名单↔前端 key 同步仍无机比兜底 → 转 R005** |
| N010 | P2 | `[x]` | ②③④⑤⑥ 全部正确（FS_BASE 叠加语义统一、自救提示、\\?\ 前缀剥离在校验后不影响安全、log.rs 文档对齐、陈旧注释修正）。**残留：①漏改 attachment.rs:1046-1049 一处英文拒绝文案；④UNC 路径剥离边角 → 转 R004** |

## 修复进度

- 轮次 1（P 系列）：41/47 验证通过；2 缺陷已在轮次 2 修复并验证（P009→N002 ✅、P022→N003 ✅）；2 跳过；2 延期
- 轮次 2（N 系列）：**9/10 验证通过**（其中 5 项有残留子项转 R002–R005），N001 未完全关闭（→ R001）
- 轮次 3 新问题（R 系列）：**5/5 已修复**（R001–R005）
- 当前处理：审查收尾（R005 脚本 `^\s{2}` → `^\s+` 防缩进漂移；R003 文档注释改为「每次访问 LWW 复查」），准备推送

## 轮次 3 新问题清单（验证发现）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| R001 | P0 | 测试 | `tauri/src-tauri/src/commands/export_import/export_docx.rs:1906` | N001 遗漏：`test_text_markdown_multi_object_separator` 仍红，check-all EXIT=101。断言 `text.matches("=====").count() == 1` 陈旧——实现（:798）分隔线是 50 个 `=`，matches 按 5 字符计得 10。**实现行为正确**（两对象间一条分隔线），只需把断言改为对完整分隔行计数 | `[x]` 已修复 |
| R002 | P2 | 逻辑 | `tauri/crates/solosoul-sync/src/delta.rs:179`、`crates/solosoul-vault/src/storage/conversations.rs:200-216` | N004 残留：假冲突自动消解比较 `strip_bookkeeping(local) == strip_bookkeeping(remote)` 对 conversations 恒不成立——本地快照是解密会话 JSON（id/name/messages 键），远端是线格式信封 `{id, accountId, data: <base64>, updatedAt}`，键形错配。内容已收敛、本地赢 LWW 时仍必记假冲突（噪声问题非数据丢失）。**已修复**：新增 `conversation_remote_content` 解密远端信封为明文 JSON 再与本地快照比较（信封 data 为随机 nonce 加密 blob，base64 不可直接比对）；新增 `test_conversation_conflict_auto_resolved_when_content_converged` 单测验证本地赢 LWW 内容已收敛时不产生假冲突（delta 8 测试全绿） | `[x]` 已修复 |
| R003 | P2 | 架构 | `tauri/crates/solosoul-core/src/llm/service.rs` 懒迁移 | N005 残留：迁移仍立即删除 profile blob 的 `llmConversations` 键，键删除经 profile delta 同步到旧版本设备后，旧设备（不认识 llm_conversations 表）会话被抹且无法自行迁移。**已修复**：迁移后保留 blob 键（延迟删键，删除 `clear_legacy_conversations`），重复调用幂等（LWW 不覆盖较新行）；CHANGELOG 明确警示混合版本同步安全策略；LWW 测试更新断言键保留 + 重复迁移幂等（4 测试全绿） | `[x]` 已修复 |
| R004 | P2 | 规范 | `tauri/src-tauri/src/commands/attachment.rs:1046-1049`、`commands/fs.rs display_fs_path` | N010 残留：①`attachment_download` 目的地白名单拒绝文案仍为英文且无自救提示（与 :577 同类用户可见场景）；②Windows `\\?\UNC\...` 网络路径剥离不完美。**已修复**：①文案改中文 + `SOLOSOUL_FS_BASE` 自救提示（与复制路径一致）；②`display_fs_path` 纯字符串处理：`\\?\UNC\server\share` → `\\server\share`（dunce::simplified 对非盘符 UNC 按设计原样返回、非 Windows 为 no-op，不可用故不引入依赖），单测补 UNC 用例；fmt/check/clippy 全绿 | `[x]` 已修复 |
| R005 | P2 | 测试 | `tauri/src-tauri/src/commands/settings.rs`、`llm/stream.rs` | 测试兜底缺口：①P026 偏好 key 白名单与前端 settingsStore 20 个 key 的同步无机械化断言；②N008 的 `extract_openai_usage_from_chunk` 缺字段保留语义无单测。**已修复**：①新建 `scripts/check_pref_keys_sync.py`（解析 Rust ALLOWED_PREF_KEYS ↔ 前端 AppSettings 接口键，不一致 exit 1）接入 check-all + `check:pref-keys` 脚本，实测 20 键一致通过；②抽出 `apply_openai_usage_chunk` 助手（消除两处内联重复）并补 4 个单测：usage 缺失→None、双字段提取、缺字段→None 非 0、逐字段更新保留累积值；fmt/check/clippy 全绿 | `[x]` 已修复 |

## 轮次 3 验证覆盖说明

- 验证方式：逐提交 `git show` 审读 + 当前代码语义比对 + 历史版本对照（git show <sha>^）+ 实测（check-all、cargo test -p solo_soul --lib、Android 交叉 check、CLI cargo test）。
- 实测结论：Android 交叉编译与 CLI 测试已随 N002/N003 修复转绿；唯一红线是 R001 的陈旧断言。
- N 系列修复整体质量高：无 ❌ 级问题；N001/N002/N003/N006/N007/N008 完全正确；N004/N005/N009/N010 主体正确、残留子项均已登记。

## 历史轮次摘要

- **轮次 1**（2026-08-11 凌晨）：初始全库分析，47 项（P0×1 / P1×10 / P2×36）。开发者修复后验证：41 通过、2 缺陷（P009 cfg 门控、P022 CLI 测试）、2 跳过（P027/P033）、2 延期（P046/P047）。
- **轮次 2**（2026-08-11 傍晚）：验证发现 10 项新问题（N001–N010），含 Rust 测试 9 红、LLM 冲突快照恒 None 确证 bug、check-all 缺 cargo test 流程缺口。开发者已全部修复并提交（0f6d76f1–24d07e27）。
- **轮次 3**（本轮）：N 系列验证 9/10 通过，新增 R001–R005。

## 待用户指令

- 建议修复顺序：R001（P0，check-all 红，单行断言修改）→ R002（假冲突噪声）→ R003（发版前需定夺删键时机）→ R004/R005（小改）。
- 按流程一项一提交，每次提交前征得用户确认。
