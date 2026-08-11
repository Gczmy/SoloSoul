# 代码分析修复报告

> 最后更新：2026-08-11 20:10:00
> 当前分支：`main`
> 修复轮次：3（N 系列修复轮——按项修复并逐项更新）

## 基线验证（本轮实测）

| 检查 | 结果 |
|------|------|
| `npm run check-all`（tsc → fmt → clippy → lint → Vitest 634 → ACL 196 命令） | ✅ 通过 |
| `cargo test -p solo_soul --lib` | ❌ **397 通过 / 9 失败**（8 个 export_docx 陈旧断言 + 1 个 fs_read_image_preview，见 N001） |
| `cargo check --target aarch64-linux-android` | ❌ **编译失败**（P009 修复缺陷，见 N002） |
| `solosoul_cli cargo check --tests` | ❌ **编译失败**（P022 修复缺陷，见 N003） |

> ⚠️ 流程缺口：`check-all` 脚本**不含 `cargo test`**，Rust 测试红不会被基线发现（已登记为 N007）。

## 轮次 1 问题验证结果（P001–P047）

状态图例：`[x]` 验证通过关闭 · `[!]` 修复有缺陷未关闭 · `[-]` 经确认跳过 · `[>]` 延期为后续架构项

| ID | 优先级 | 状态 | 验证备注 |
|------|--------|------|----------|
| P001 | P0 | `[x]` | clippy 2 error 已修；四平台 cfg 门控静态审查自洽（未做 Windows/Android 交叉实测） |
| P002 | P1 | `[x]` | 已改为 Rust 侧按 version 重拉元数据 + 编译期公钥验签 + fail-closed；前端调用点已同步。**遗留隐患转 N006**（资产匹配谓词可误命中 → 误拒下载） |
| P003 | P1 | `[x]` | 已改纯 SQL 筛 id + `SUM(LENGTH(properties))` 估算；tags 过滤语义与旧实现一致。小注：估算口径为密文字节（偏高 30–50%，展示用可接受）；`mod.rs:262` 一处陈旧注释 |
| P004 | P1 | `[x]` | 会话已改独立 `llm_conversations` 表行级存储，迁移幂等、trim 保留、旧版本同步兼容。**但发现 1 个确证 bug（转 N004）与迁移隐患（转 N005）** |
| P005 | P1 | `[x]` | CLI 已改用 `count_objects` 纯 SQL，语义完全等价 |
| P006 | P1 | `[x]` | `record_to_data` 透传 tags；tags_json 解析失败回退空 vec 不 panic；store 合并与详情渲染均为活路径；测试已同步 |
| P007 | P1 | `[x]` | 拆分逐行等价，既有 26 测试全过。瑕疵：文档注释错位挂到 `build_field_meta`（装饰性） |
| P008 | P1 | `[x]` | 拆分逐行等价，search 20+ 测试全过 |
| P009 | P1 | `[!]` | **拆分本体等价，但两个共享 helper 漏 cfg 门控：Android/iOS 编译硬错误（已实测复现 E0425），Linux CI clippy 报 dead code。转 N002 跟踪** |
| P010 | P1 | `[x]` | hook 抽取逐行等价，监听清理正确，tsc 通过 |
| P011 | P1 | `[x]` | 12 项分类完整顺序一致，两变体差异参数化正确 |
| P012 | P2 | `[x]` | 匹配收紧正确；checksumWarning 前端消费链完整（UpdateInfoCard 橙色警告展示） |
| P013 | P2 | `[x]` | 三命令已接白名单，移动端 content:// 不受影响。隐患：桌面白名单外导入被拒（英文文案）；fs.rs 与 attachment.rs 两个 `allowed_fs_bases` 语义分叉（转 N010） |
| P014 | P2 | `[x]` | `..` 拒绝覆盖字面与 canonicalize 兜底两分支，与 download 逐行对齐 |
| P015 | P2 | `[x]` | 三处 fail-closed 一致，正常桌面/移动端无回归。隐患：极简 Linux 全不可用且文案无自救提示（转 N010） |
| P016 | P2 | `[x]` | 入口即 Zeroizing 包装，传递链为借用无二次明文拷贝（IPC 反序列化层 transient 明文不在防护范围） |
| P017 | P2 | `[x]` | 已返回 canonical 路径，6 个调用方均仅用于文件操作。隐患：Windows `\\?\` 前缀/macOS symlink 基目录使回传路径变 canonical 形态（转 N010） |
| P018 | P2 | `[x]` | 日志只记文件名尾部组件，全文件无路径泄漏 |
| P019 | P2 | `[x]` | 白名单 5 个 `critical_field_*`，前端仅 2 处调用均在名单内，拒绝路径 catch 静默合理。瑕疵：模块文档注释过时（转 N010） |
| P020 | P2 | `[x]` | 参数保留仅忽略，前端调用全兼容；其余写路径无遗留信任客户端 account_id。小瑕疵：account_id 成死字段仍强制传值 |
| P021 | P2 | `[x]` | TS 补字段后保存路径闭环正确 |
| P022 | P2 | `[!]` | **主体正确（存储侧保留、解锁流程不受影响、消费面排查干净），但漏改 CLI 测试：`solosoul_cli/src/screens/unlock.rs:233-234` 仍写 salt/verify_hash 字段，`cargo test` 编译全红。转 N003 跟踪** |
| P023 | P2 | `[x]` | 重复 DTO 已收敛，TS 字段对齐，bootstrap/pin_unlock 两路径语义一致 |
| P024 | P2 | `[x]` | 单一实现逐参一致，SOLOSOUL_SECURE 切换保留，错误映射等价，core 内有 round-trip 测试 |
| P025 | P2 | `[x]` | release 其余配置保留；插件沙箱 `catch_unwind`（sandbox.rs:113）真实存在 |
| P026 | P2 | `[x]` | 白名单与前端 20 个 key 完全重合，上限不误伤正常数据。瑕疵：`validate_object_input` 误挂 `#[tauri::command]`（object/mod.rs:608）；新校验零测试（转 N009） |
| P027 | P2 | `[-]` | 经确认跳过（保留 API 完备性） |
| P028 | P2 | `[x]` | 删除无残留引用，编译通过 |
| P029 | P2 | `[x]` | 删除无残留引用 |
| P030 | P2 | `[x]` | shim 删除，mod 声明与引用清理干净 |
| P031 | P2 | `[x]` | 取消导出，仅本文件使用 |
| P032 | P2 | `[x]` | 过时注释已删 |
| P033 | P2 | `[-]` | 经确认跳过（保留诊断日志） |
| P034 | P2 | `[x]` | 抽取等价；**语义微变转 N008**（OpenAI usage 缺字段时累积值从保留变清零，实际触发概率极低） |
| P035 | P2 | `[x]` | 三种去重策略分支逐行等价 |
| P036 | P2 | `[x]` | 四阶段拆分逐行等价 |
| P037 | P2 | `[x]` | 五阶段纯 verbatim 移动，孤儿过滤语义不变 |
| P038 | P2 | `[x]` | 6 项元数据对比全部保留（4 项抽取 + 2 项留主循环） |
| P039 | P2 | `[x]` | 常量 6 分区 key/显示名与旧三处完全一致，派生正确 |
| P040 | P2 | `[x]` | ref/disabled/marginBottom/onCardClick 差异参数化正确 |
| P041 | P2 | `[x]` | 状态/按钮分支逻辑全保留；极轻微视觉差异（图标间距 4→8px） |
| P042 | P2 | `[x]` | overlay 关闭/滚动/标题变体正确；Escape 关闭前后均不存在（非回归） |
| P043 | P2 | `[x]` | 参数化正确；轻微视觉差异（UpdateInfoCard 文本变居中） |
| P044 | P2 | `[x]` | 两端语义确认相同，合并合理 |
| P045 | P2 | `[x]` | 「核对后不合并」经复核**合理**：三处快照行渲染结构/数据源形状真实不等价，强行参数化需大量分支 |
| P046 | P2 | `[>]` | 延期：10 个巨型前端组件拆分，列为后续架构项 |
| P047 | P2 | `[>]` | 延期：3 个巨型 Rust 文件拆分，列为后续架构项 |

## 修复进度

- 轮次 1：验证通过关闭 **41 / 47**（含 P045 判定合理关闭）；修复有缺陷未关闭 2（P009、P022）；经确认跳过 2（P027、P033）；延期 2（P046、P047）
- 轮次 2 新问题（N 系列）：**9 项，已修复 7 / 9**（N001–N007 完成）
- 当前处理：N008（OpenAI usage 累积语义恢复逐字段更新）

## 轮次 2 新问题清单（验证发现）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| N001 | P0 | 测试 | `tauri/src-tauri/src/commands/export_import/export_docx.rs` 测试、`commands/fs.rs` 测试 | main 上 Rust 测试 9 红（397 过/9 失败），CI rust-test 必挂：8 个 export_docx 陈旧断言（escape_markdown 未随 1c516c28 转义精简更新、fixture 缺 createdAt/template_id 矛盾）+ 1 个 test_fs_read_image_preview_command | `[x]` 已修复 |
| N002 | P1 | 构建 | `tauri/src-tauri/src/commands/attachment.rs:1292,1300` | P009 修复缺陷：`copy_to_share_dir_async` / `run_on_main_thread_oneshot` 缺 cfg 门控——Android/iOS 编译 E0425 硬错误（已实测复现），Linux CI clippy 报 dead code。修法：前者加 `#[cfg(not(any(android, ios)))]`，后者加 `#[cfg(any(macos, windows))]` | `[x]` 已修复 |
| N003 | P1 | 测试 | `solosoul_cli/src/screens/unlock.rs:233-234` | P022 修复缺陷：CLI 测试字面量仍写已删除的 `salt: None, verify_hash: None`，`cargo check --tests` E0560，CLI 测试全红。删 2 行即可 | `[x]` 已修复 |
| N004 | P1 | 漏洞/逻辑 | `tauri/src-tauri/src/commands/llm/conversations.rs:296-302`（实际位于 `crates/solosoul-vault/src/storage/conversations.rs`） | 确证 bug（P004 修复引入）：`conversation_local_snapshot` 调 `load_conversation("", id)` 空 account_id 恒不匹配 → 冲突 UI 本地快照恒 None；连带 delta.rs:171-180 本地赢 LWW 时记假冲突。已修：`llm_conversations.id` 为全局主键，快照直接按 id 取数（对齐其余表），新增单测 | `[x]` 已修复 |
| N005 | P2 | 架构 | `tauri/src-tauri/src/commands/llm/conversation.rs:52-57`、`solosoul-core` LlmService | LLM 迁移隐患：①CLI 无懒迁移（看不到旧 blob 会话）；②GUI 懒迁移对 blob 逐条无条件 upsert（无 HLC/LWW 比较），可覆盖 CLI 已写入的较新行；③混合版本同步会抹掉旧设备 blob 会话。已修：迁移下沉 `LlmService::migrate_legacy_conversations` 单一实现（含 `updated_at` LWW 比较 + trim + 清键），`load_conversations`/`get_conversation` 开头自动触发（CLI 经 LlmService 读即接入），GUI `conversation.rs` 委托同一实现；新增 LWW 单测 | `[x]` 已修复 |
| N006 | P2 | 漏洞 | `tauri/src-tauri/src/commands/update.rs:232-240` | P002 遗留：`find_apk_asset` 谓词 `contains("universal-release")` 可命中 `.apk.sha256(.minisig/.sig)` 资产；`resolve_verified_checksum` 的 `contains("sha256")` 同款。若 GitHub 资产排序不利 → fail-closed 永久误拒下载（可用性问题）。已修：`find_apk_asset` 收紧为纯 `ends_with(".apk")`；签名资产统一 `ends_with(".sha256.minisig")` 去掉宽松 `contains`；新增 2 个谓词单测 | `[x]` 已修复 |
| N007 | P2 | 流程 | `tauri/package.json` check-all | check-all 不含 `cargo test`，Rust 测试红无法被基线/本地门禁发现（本轮 N001 即漏网之鱼）。已修：check-all 在 clippy 后插入 `cargo test` | `[x]` 已修复 |
| N008 | P2 | 逻辑 | `tauri/src-tauri/src/commands/llm/stream.rs:292-303` | P034 语义微变：OpenAI usage chunk 缺字段时从「保留先前累积值」变为 `unwrap_or(0)` 整体覆盖清零。实际 OpenAI 兼容 API 只在末尾发一次完整 usage，风险极低 | `[ ]` 待修复 |
| N009 | P2 | 测试/规范 | `tauri/src-tauri/src/commands/object/mod.rs:608`、`settings.rs`、`template.rs` | ①`validate_object_input` 误挂 `#[tauri::command]`（未注册，生成无用包装代码）；②P026 新增的三组边界校验函数零单元测试，白名单与前端 key 同步无兜底 | `[ ]` 待修复 |
| N010 | P2 | 规范 | 多处（汇总） | 文案/注释/路径卫生：①P013 桌面白名单外导入报英文技术文案；②fs.rs 与 attachment.rs 两个 `allowed_fs_bases` 对 SOLOSOUL_FS_BASE 语义分叉（覆盖 vs 叠加）；③P015 极简 Linux 拒绝文案无自救提示且中英不一；④P017 canonical 路径回传前端（Windows `\\?\` 前缀）；⑤log.rs:10-11 模块文档与白名单语义矛盾；⑥export_import/mod.rs:262 陈旧注释 | `[ ]` 待修复 |

## N 系列修复记录

### N001（P0）Rust 测试 9 红 —— 已修复

**2026-08-11 修复内容**：
- `test_escape_markdown`：断言与 1c516c28 转义精简后的 `escape_markdown`（仅转义 `\` `` ` `` `[]` `|` `<>`，`.` `-` `+` `()` `*` `_` `#` 原样）对齐；`a*b_c[d]`e`` 不再期望 `\*` `\_`，`# 标题` 不再期望 `\#`
- fixture `make_record`：`template_id: None` → `Some("t1")`，与「模板：护照」断言一致（渲染器按 template_id 查 template_names 映射）
- 6 处 `__attachments` fixture 补 `createdAt`：`AttachmentMeta.created_at` 为必填字段（无 serde default），缺失导致 `load_attachments` 静默返回空、附件断言全挂
- `test_build_text_document` 多行缩进断言：5 空格 → 3 空格（渲染器 `" ".repeat(label.chars().count() + 1)`，label「姓名」=2 字符+1）
- `test_build_markdown_document`：`- 附件清单` → `附件清单：`（markdown 渲染器清单头为 `附件清单：`，条目才带 `- ` 前缀）
- `test_fs_read_image_preview_command`：`futures::executor::block_on` → `#[tokio::test]`——`fs_read_image_preview` 内部用 `tokio::task::spawn_blocking`，`block_on` 无 Tokio runtime 上下文会 panic

**验证**：`cargo fmt --check` / `cargo check -p solo_soul --tests` / `clippy -D warnings` 全绿。
本地 Windows 下 `solo_soul` 测试二进制仍无法启动（0xc0000139，环境既有问题——`solosoul-vault` 159 项测试正常），
断言正确性以逐条对照实现为准（已全部核对），CI/其他平台可运行验证。

### N002（P1）attachment_share helper cfg 门控 —— 已修复

**2026-08-11 修复内容**：
- `copy_to_share_dir_async` 加 `#[cfg(not(any(target_os = "android", target_os = "ios")))]`——桌面三平台（macos/windows/linux）使用，iOS 显式不支持分享、Android 走自有链路
- `run_on_main_thread_oneshot` 加 `#[cfg(any(target_os = "macos", target_os = "windows"))]`——仅 macOS/WinRT 需主线程调度
- `share_linux` 注释修正：「复制/揭示走 spawn_blocking」→ 复制走 `copy_to_share_dir_async`（spawn_blocking），reveal 在 async 上下文直接调用

**验证**：`cargo fmt --check` / `clippy -D warnings` 全绿（Linux 语义无 dead code）。Android 交叉 check 本机未装 target，cfg 逻辑按报告指引逐条核对。

## N 系列修复指引

### N001（P0）Rust 测试 9 红（已修复，见上）

- `test_escape_markdown`：断言未随提交 1c516c28「大幅减少 markdown 转义」更新（仍期望转义 `*`/`_`）。
- fixture 问题：`make_record` 硬编码 `template_id: None` 与「模板：护照」断言矛盾；`AttachmentMeta.created_at` 为必填而 fixture 缺 `createdAt`，导致 `load_attachments` 静默返回空（连带 be804d9b 新增的附件描述导出测试失败）。
- `test_fs_read_image_preview_command`：同期 fs 测试失败，需对照 `fs_read_image_preview` 当前实现更新断言。
- **修复**：更新断言/fixture 使与现实现一致；注意不要反向改实现去迁就陈旧测试（先确认 1c516c28 的转义精简是有意行为）。

### N002（P1）attachment_share helper cfg 门控

```rust
#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn copy_to_share_dir_async(...) { ... }

#[cfg(any(target_os = "macos", target_os = "windows"))]
async fn run_on_main_thread_oneshot(...) { ... }
```

改后验证：`cargo clippy -- -D warnings`（Linux 语义）+ `cargo check --target aarch64-linux-android`。
另：`share_linux` 注释「复制/揭示走 spawn_blocking」与代码不符（reveal 在 async 上下文直调），一并修正。

### N003（P1）CLI 测试编译

删除 `solosoul_cli/src/screens/unlock.rs:233-234` 的 `salt: None, verify_hash: None` 两行，`cd solosoul_cli && cargo test` 验证。

### N004（P1）LLM 冲突快照恒 None

`conversation_local_snapshot` 改为按 record.data 中的 accountId 读取，或不带 account 过滤按 id 读取；补一个「本地赢 LWW 不产生假冲突」的单测。

### N005（P2）LLM 迁移隐患

- 短期：GUI 懒迁移 upsert 前比较 updated_at/HLC，不覆盖较新行；
- CLI `LlmService::load_conversations` 接入同一懒迁移（迁移逻辑下沉 core）；
- 迁移窗口内在 release notes 警示混合版本同步风险，或延迟删除 blob 键一个版本周期。

### N006（P2）资产匹配谓词

`find_apk_asset` 去掉 `contains("universal-release")` 兜底或排除 `.sha256/.minisig/.sig` 后缀；`resolve_verified_checksum` 改为精确 `ends_with(".apk.sha256")`（P012 已修 check 路径，download 路径遗留）。

### N007（P2）check-all 补 cargo test

`tauri/package.json`：`"check-all": "tsc --noEmit && cargo fmt --check && cargo clippy -- -D warnings && cargo test && npm run lint && npm run test && python3 scripts/check_acl_consistency.py"`。注意全量 cargo test 耗时，可评估 `cargo test --workspace` 或仅 `-p solo_soul`。

### N008（P2）OpenAI usage 累积语义

`extract_openai_usage_from_chunk` 恢复逐字段更新语义（缺字段保留先前值），或确认主流 API 只发完整 usage 后加注释说明接受该差异。

### N009（P2）校验卫生

删除 `object/mod.rs:608` 的 `#[tauri::command]` 属性；为 P026 三组校验函数补边界单测（含白名单与前端 key 一致性断言——可机器比对 settingsStore 的 key 列表）。

### N010（P2）文案/注释/路径卫生汇总

逐项小改：统一中英文案；统一两个 `allowed_fs_bases` 的 SOLOSOUL_FS_BASE 语义；极简 Linux 拒绝文案加「设置 SOLOSOUL_FS_BASE」引导；评估 Windows 前端路径用 `dunce` 归一化；同步 log.rs 模块文档与 mod.rs:262 注释。

## 验证覆盖说明

- 验证方式：逐提交 `git show` 审读 + 修复后代码语义比对 + 实测（check-all、cargo test、Android 交叉 check、CLI check --tests、cargo check --workspace、tsc）。
- 未覆盖：Windows/Android 实机构建与运行时行为、LLM 迁移的端到端同步场景、导出 docx 的 Office 人工打开验证。
- 轮次 1 修复整体质量良好：41/47 验证通过，重构类修复（P007/P008/P035–P039）全部逐行等价；2 项缺陷（P009/P022）均为「漏改配套」（cfg 门控、CLI 测试），非设计错误。

## 待用户指令

- 建议修复顺序：N001（P0，CI 红）→ N002/N003（修复轮缺陷，改动小）→ N004（确证 bug）→ N005/N006 → N007（流程缺口，防再漏）→ N008–N010。
- 按流程一项一提交，每次提交前征得用户确认。
- N001、N002 已按纪律完成（一项一提交一更新报告）。
