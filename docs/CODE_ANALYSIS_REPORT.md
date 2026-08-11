# 代码分析修复报告

> 最后更新：2026-08-11 22:20
> 当前分支：`main`
> 修复轮次：5（S 系列修复轮）

## 基线验证（本轮实测）

| 检查 | 结果 |
|------|------|
| 轮次 4 基线：`npm run check-all`（tsc → fmt → clippy → cargo test → lint → Vitest → ACL → pref-keys 同步检查） | ✅ 全部通过（EXIT=0） |
| 轮次 5 修复验证：S001–S004 相关 Rust 测试（solosoul-core llm::service 5、solosoul-sync delta 10、solosoul-vault conversations）+ 四 crate fmt/clippy + 偏好键机比脚本 | ✅ 全部通过 |

## 轮次 3 问题验证结果（R001–R005）

| ID | 优先级 | 状态 | 验证备注 |
|----|--------|------|----------|
| R001 | P0 | `[x]` | 断言改按整行 50 `=` 计数，与实现语义对齐；export_docx 28 测试实测全绿 |
| R002 | P2 | `[x]` | 远端信封解密为明文后比较，nonce 不可比问题已绕过；4 类失败全部 fail-safe 退化为记冲突（方向正确）；解密仅在冲突候选分支触发，性能可接受；新单测通过。**残留：2 条路径无会话级单测 → S002** |
| R003 | P2 | `[!]` | 核心改动（保留 blob 键 + CHANGELOG 警示 + 测试更新）与声明一致，**但引入确定性回归：永久删除的会话会被复活 → S001**；另有删键时机无追踪的技术债 → S003 |
| R004 | P2 | `[x]` | 拒绝文案已中文化 + 自救提示，无英文残留；UNC 纯字符串处理逻辑正确（盘符/UNC/Unix 三用例单测通过）。小瑕疵：dunce 行为注释论证不准确 → S004 |
| R005 | P2 | `[x]` | 机比脚本提取/接入/输出均正确（实测通过）；usage 助手抽取等价，4 个单测覆盖 N008 回归点。**残留：脚本正则不匹配可选属性 `key?:` → S004** |
| 收尾 | — | `[x]` | `\s+` 防格式化漂移正确；migrate 注释修正与「每次访问 LWW 复查」实际行为精确吻合（纯注释无行为变化） |

## 修复进度

- 轮次 1（P 系列）：41/47 通过，2 跳过，2 延期（P046/P047 巨型组件/文件拆分，仍为后续架构项）
- 轮次 2（N 系列）：10/10 通过（残留转 R 系列）
- 轮次 3（R 系列）：**4/5 通过**，R003 修复引入新回归 → S001
- 轮次 4 新问题（S 系列）：**4/4 全部完成**（S001–S004 已修复）
- 当前处理：全部完成，收口验证中

## 轮次 4 新问题清单（验证发现）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| S001 | P1 | 逻辑/数据 | `tauri/src-tauri/src/commands/llm/conversation.rs:180-187`、`crates/solosoul-core/src/llm/service.rs:163-178` | **R003 引入的回归**：`llm_permanent_delete` 只删行级记录不碰 blob，而 blob 键现在永久保留——此后任何 `load_conversations`/`get_conversation` 触发的懒迁移（条件「行级表无此 id 则写入」）会把已永久删除的会话从陈旧 blob 重新写回，**回收站 purge 的会话下次列表即复活**，且复活行作为新 delta 同步扩散到其他设备。修复：永久删除时同步从本设备 blob 值中移除该 id（改值不删键，旧设备不受影响），或迁移侧维护已物理删除 id 的墓碑集合；补「purge 后不复活」单测 | `[x]` 已修复（见下） |
| S002 | P2 | 测试 | `crates/solosoul-sync/src/delta.rs` | R002 残留测试缺口：新假冲突消解逻辑只有「内容收敛→不记冲突」单测；「会话内容不同仍记冲突」与「解密失败退化记冲突」两条路径无会话级单测（现有内容不同用例走 objects 表，覆盖不到新分支） | `[x]` 已修复（见下） |
| S003 | P2 | 技术债 | `crates/solosoul-core/src/llm/service.rs`、CHANGELOG | R003 残留：blob 键清理时机仅 CHANGELOG 一句「确认所有设备升级后清理」，代码中无 TODO/版本门槛/追踪 issue，容易永久遗忘。另注：混合版本期旧设备**删除**会话不传播到新设备行级表（迁移只增不删、无墓碑）——N005 迁移固有限制，建议与删键清理一并规划 | `[x]` 已修复（见下） |
| S004 | P2 | 规范 | `tauri/scripts/check_pref_keys_sync.py:55`、`tauri/src-tauri/src/commands/fs.rs:164-166` | 小项汇总：①机比脚本 TS 属性正则不匹配可选属性 `key?:` 与带引号键——未来 AppSettings 出现可选属性会静默漏检（当前 20 键均必选无实际影响，建议正则收紧为 `([A-Za-z][A-Za-z0-9]*)\?*:`）；②fs.rs 注释称「dunce 对非盘符 UNC 按设计原样返回」不准确（dunce 实际会转换安全 UNC），行为正确仅注释论证有偏差 | `[x]` 已修复（见下） |

## 轮次 4 修复记录

### S001（P1，数据复活回归）✅ 已修复

- **修复**：服务层新增 `LlmService::permanent_delete_conversation`（`crates/solosoul-core/src/llm/service.rs`）——先 `remove_legacy_conversation_from_blob`（从 profile `preferences.llmConversations` 值中移除该 id，键保留），再 `vault.delete_conversation`（行级删除 + 记墓碑）。GUI `llm_permanent_delete`（`conversation.rs`）改为委托该服务方法（与 migrate 委托模式一致），CLI 无删除路径不受影响。
- **顺序选择**：先改 blob 后删行级——blob 移除失败时删除整体中止（不留半删除状态）；行级删除失败时会话仍在行级表（用户可见可重试），且 blob 已无该 id、懒迁移不会复活。错误全传播（solosoul-core 无 log 依赖）。
- **语义**：改值不删键——键保留继续保护只读 blob 的旧版本设备（R003 不回归）；值移除保证本设备及经 profile delta 同步的其他新版本设备不再复活该会话。
- **单测**：`test_permanent_delete_no_resurrect`——迁移后永久删除 c1，验证行级已删、`load_conversations`/显式再迁移均不复活 c1、c2 不受影响、blob 键保留且 c1 已从值中移除。实测 5 测试全过（solosoul-core llm::service）。

### S002（P2，测试缺口）✅ 已修复

- **补两条会话级单测**（`crates/solosoul-sync/src/delta.rs`，均走 llm_conversations 表 + 本地赢 LWW 的完整生产路径）：
  - `test_conversation_conflict_recorded_when_content_differs`：本地内容与远端不同 → 真实差异，记录冲突（不自动消解），断言 `stats.conflicts.len() == 1` 且持久化冲突表非空。
  - `test_conversation_conflict_recorded_when_remote_decrypt_fails`：篡改远端信封 `data` 的一个合法 base64 字符（长度不变仍可解码、解码后字节损坏）→ 解密失败 → fail-safe 保守记录冲突。已验证 LWW 检查先于写库（`apply_sync_record_tx` 第 62-72 行），篡改数据不会落库。
- 实测 delta 模块 10/10 全绿（原 8 + 新 2），fmt/clippy 干净。

### S003（P2，技术债登记）✅ 已修复

- **代码内登记 TODO(S003) 版本门槛**（`crates/solosoul-core/src/llm/service.rs` 的 `migrate_legacy_conversations` 保留键注释处）：①删键时机——确认所有设备升级到 2.9.2+ 后，在下一个大版本（建议 v2.10，同步 bump versionCode）经 `save_profile_data` 移除键，让 profile delta 传播键删除；②混合版本期删除传播限制——旧版本设备（只读 blob）删除会话不传播到新版本设备行级表（迁移只增不删、无墓碑），新版本永久删除已由 S001 覆盖，彻底闭环旧端删除须与删键清理一并规划行级墓碑协议升级。
- **CHANGELOG 会话迁移警示同步补充**：清理门槛 + 混合版本期限制两条，与代码 TODO 互相引用，防止永久遗忘。
- fmt/check/clippy 全绿（纯注释/文档改动）。

### S004（P2，规范小项）✅ 已修复

- **①机比脚本正则收紧**（`tauri/scripts/check_pref_keys_sync.py`）：`^\s+([A-Za-z][A-Za-z0-9]*):` → `^\s+\"?([A-Za-z][A-Za-z0-9]*)\"?\??:`。调试中发现关键细节：TypeScript 带引号键 `"keyName":` 的引号**成对包裹键名**（闭引号在键名之后、冒号之前），只允许开引号会漏匹配——修正后四种形态（普通/可选 `?:`/带引号/带引号可选）均正确提取，注释行仍忽略。实测脚本 OK（20 键一致，EXIT=0）。
- **②fs.rs dunce 注释论证修正**（`src-tauri/src/commands/fs.rs`）：核对 dunce 1.0.5 源码——`simplified` 在 Windows 上仅把盘符 VerbatimDisk（`\\?\C:\...`）还原为常规路径，对网络 VerbatimUNC（`\\?\UNC\...`）按设计**刻意保留原样**，且非 Windows 为 no-op；修正注释为准确表述，并说明自实现两类前缀都处理且跨平台行为一致（便于 CI 单测）。
- fmt/check/clippy 全绿。

### 轮次 5 收口：审查与验证 ✅

- **代码审查通过**（无阻塞项）：S001 先 blob 后行级删除的顺序语义正确（部分失败自愈、无复活）；S002 解密失败测试的「LWW 检查先于写库、篡改字节不落库」推理与 `apply_sync_record_tx` 第 62-72 行一致；S004 正则成对引号修复与 dunce 行为核对均准确。审查建议 1 条已采纳：S001 文档补「极端部分失败时 profile delta 已传播、重试成功后墓碑传播自愈」说明。
- **验证全绿**：solosoul-core llm::service 5 测试（含 purge 不复活）、solosoul-sync 63 测试（含 delta 10）、solosoul-vault conversations 3 测试；四 crate fmt/clippy 全绿；偏好键机比脚本 20 键一致。

## 轮次 4 验证覆盖说明

- 验证方式：逐提交 `git show` 审读 + 当前代码语义比对 + 实跑相关测试（delta 8、export_docx 28、solosoul-core llm、fs、usage 4 个新单测，全绿）+ check-all 全量实测（EXIT=0）。
- R 系列修复质量：R001/R004/R005 与收尾完全正确；R002 方向正确（fail-safe 保守、性能有界）；唯一问题是 R003 修复时未考虑「保留 blob 键 × 永久删除」的交互，引入 S001 复活回归——**建议优先修 S001，它是数据语义级 bug（已删数据复活并扩散）**。

## 历史轮次摘要

- **轮次 1**：初始全库分析 47 项；修复后验证 41 通过、2 缺陷、2 跳过、2 延期。
- **轮次 2**：验证发现 N001–N010（Rust 测试 9 红、LLM 快照 bug、check-all 缺 cargo test 等），开发者全部修复。
- **轮次 3**：N 系列验证 9/10 通过，新增 R001–R005（遗漏 1 个陈旧断言、假冲突抑制缺口、混合版本删键风险等）。
- **轮次 4**（本轮）：R 系列验证 4/5 通过，check-all 首次全绿；R003 引入 S001 复活回归，另有 S002–S004 小项。

## 待用户指令

- 建议修复顺序：S001（P1，数据复活回归）→ S002（补两条路径单测）→ S003（技术债登记方式定夺）→ S004（小改）。
- 按流程一项一提交，每次提交前征得用户确认。
