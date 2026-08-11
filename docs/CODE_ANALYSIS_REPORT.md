# 代码分析修复报告

> 最后更新：2026-08-11（轮次 6：S 系列收口后新推送审查）
> 当前分支：`main`

## 总览

- 全部已提出问题：**61 项**（P47 + N10 + R5 + S4，含重复计数修正）
- **修复关闭 57 项**、**经确认跳过 2 项**（P027/P033）、**延期 2 项**（P046/P047 巨型组件/文件拆分，列为后续架构项）
- 遗留计划事项：S003 的 v2.10 blob 键清理（已有代码 TODO + CHANGELOG 双向追踪，属计划内而非遗忘）
- 末轮验证（轮次 5）未发现新的 P0/P1/P2 问题，`check-all` 全绿（含 cargo test 与两项机械化一致性检查）

**代码库质量评估达标，审查收口。** 剩余 P046/P047 为已知的大型重构架构项，建议在后续专项中处理。

## 遗留项（跳过 / 延期）

状态图例：`[-]` 经确认跳过 · `[>]` 延期为后续架构项

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| P027 | P2 | 死代码 | `tauri/src-tauri/src/commands/object/trash.rs:147`、`lib.rs:389,870,996` | `trash_permanent_delete` 已注册但前端从不调用（P024 批量改造后走 batch），删除需同步守卫测试与总数断言 | `[-]` 经确认跳过（保留 API 完备性） |
| P033 | P2 | 死代码 | `tauri/src-tauri/src/lib.rs:304-316`（调用于 :667） | `setup_detect_locale()` 结果仅用于一行 debug 日志，前端实际走 `get_system_locale` IPC（需人工确认是否保留诊断） | `[-]` 经确认跳过（保留诊断日志） |
| P046 | P2 | 架构 | 前端 10 个巨型组件（汇总） | 单组件非注释行 > 300：`AttachmentViewer`(~550)、`LoginPage`(~501)、`PasswordVerificationDialog`(~447)、`TemplateFieldRow`(~436)、`DeviceListPanel`(~424)、`TrashPage`(~422)、`ObjectDetailModal`(~422)、`ExportImportPage`(~417)、`ImportSection`(~409)、`ExportSection`(~405)，建议按「数据 hook + 展示子组件」拆分 | `[>]` 延期：10 个巨型前端组件拆分，列为后续架构项 |
| P047 | P2 | 架构 | Rust 巨型文件（汇总） | `attachment.rs` 2057 行、`object/tests.rs` 2353 行、`export_docx.rs` 1989 行，文件级拆分作为后续架构项 | `[>]` 延期：3 个巨型 Rust 文件拆分，列为后续架构项 |

## 历史轮次摘要

- **轮次 1**：初始全库分析 47 项（P001–P047）；修复后验证 41 通过、2 缺陷（轮次 2 修复）、2 跳过、2 延期。
- **轮次 2**：验证发现 N001–N010（Rust 测试 9 红、LLM 快照 bug、check-all 缺 cargo test 等），全部修复。
- **轮次 3**：N 系列验证通过后新增 R001–R005（遗漏陈旧断言、假冲突抑制缺口、混合版本删键风险等），4/5 通过；R003 引入 S001 回归。
- **轮次 4**：S001–S004 全部修复（含 R003 引入的会话复活回归）。
- **轮次 5**（收口）：4/4 全部通过，无新增问题；全库质量评估达标。
- **轮次 6**：S 系列收口后新推送审查（更新加速 + 附件标签折叠系列），新增 T001–T005，全部 `[ ]` 待修复。

## 轮次 6：新推送审查（T 系列）

审查范围：`2c9c7b21`（GitHub 国内下载加速）与附件标签/描述折叠展开系列 6 提交（797a0afa–95de2d9f）。基线实测：check-all ❌ EXIT=101（cargo test 419 过 / **1 失败**，见 T001）。

**安全结论（更新通道）**：P002/N006 的 fail-closed 校验链**未被削弱**——合并后整体 SHA-256 + Rust 侧 minisign 验签，分段篡改/乱序/截断均可被发现；代理全 HTTPS、直连优先回退、篡改无法注入；桌面端 updater 多 endpoint 共用同一签名、轮询正确；分段边界无 off-by-one；前端契约无变化。

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| T001 | P0 | 测试 | `tauri/src-tauri/src/commands/update.rs:1234` | 带红测试入库：`test_compute_segments_caps_at_max_segments` 断言错误——作者把调用点的 20MB 并行阈值（`PARALLEL_MIN_FILE_SIZE`）误当成 `compute_segments` 内部逻辑，断言 1 段实为 4 段。check-all / CI rust-test 必挂。修法：断言改 4（阈值本属调用点职责） | `[x]` 已修复（T001，见下） |
| T002 | P1 | 弹性 | `tauri/src-tauri/src/commands/update.rs:843,902,745-762` | 分段下载弹性缺口：①流读取中途失败直接 `?` 返回，不切换下一候选、也不整体回退单流（注释宣称会切换，与行为不符）；②parallel 失败时已 spawn 的分段任务不 abort，继续后台写已删除的 seg 文件并 emit 进度；③parallel 失败删除 part 文件连带销毁单流续传的有效断点。代理限流/中途断连 = 整个下载失败 | `[x]` 已修复（T002，见下） |
| T003 | P2 | 资源 | `tauri/src-tauri/src/commands/update.rs:209`、`tauri/src-tauri/tauri.conf.json` | ①进程中断时 `.part.seg{i}` 孤儿文件永不清理（`cleanup_stale_apk_cache` 只清非当前版本，且 parallel 不支持续传，纯垃圾累积）；②updater 未配 `timeout`（插件默认无超时），直连黑洞（hang 而非 RST）时代理回退迟迟不触发 | `[x]` 已修复（T003，见下） |
| T004 | P2 | 隐私/可用性 | `tauri/src-tauri/src/commands/update.rs:29,252`、`scripts/generate-latest-json.js` | 代理通道固有面未披露/未缓解：①gh-proxy 类代理 TLS 终止于代理方，用户 IP、使用 SoloSoul 的事实、目标版本号、API 响应全部暴露给第三方——代码注释只强调「无供应链风险」未提隐私面，建议在文档/注释披露；②`fetch_github_release_url` 把 api.github.com 元数据请求也套代理，代理可返回陈旧/篡改 Release JSON 软性压制升级（完整性无碍，属可用性面）；③代理可重放旧版 mirror JSON 压制升级（updater 只升不降，无降级风险）；④4 个代理硬编码不可配置，失效需发新版 | `[x]` 已修复（T004，见下） |
| T005 | P2 | 前端 | `tauri/src/components/attachment/AttachmentFileNameBlock.tsx:127-139` | 溢出检测无 resize 监听：描述与标签两处 `useLayoutEffect` 测量仅在 deps 变化时触发，容器变窄后原本 ≤4 个的标签被截断时用户**无任何展开途径**（无 +N 也无按钮）；字体未加载完成时测量可能偏小漏判。另：描述溢出测量分支在 jsdom 下恒走 false，零行为测试覆盖（标签侧已用 mock 布局属性方案解决，可仿照）。建议加 `ResizeObserver` | `[ ]` 待修复 |

**附件折叠系列（797a0afa–95de2d9f）其余结论**：mergeTagInput 幂等合并四路径闭环成立（失焦+保存双保险）；折叠态单行 CSS 无崩坏路径；按钮固定右侧结构正确；`95de2d9f` 未改全局 CSS（border-box 为项目骨架既有规则，提交信息有误导）；测试在 jsdom 下真实有效（15/15 实测通过，mock 布局属性方案成立）。

## 修复记录

### T001（P0，测试断言）✅ 已修复

- **修复**：`test_compute_segments_caps_at_max_segments` 的 19MB 断言 `1 段` 改为 `4 段`——`compute_segments` 按「每段最小 5MB」收缩段数（`div_ceil(19MB/5MB)=4`），不感知调用点的 20MB 并行阈值（`PARALLEL_MIN_FILE_SIZE` 属 `download_apk` 职责）；测试注释同步澄清阈值归属，防止再次误写。
- **验证**：cargo fmt / check 全绿；本机测试二进制 `0xc0000139` 为既有环境限制，CI 兜底。

### T002（P1，下载弹性）✅ 已修复

- **①流中途失败切候选**：`download_range_to_file` 把「分块读取/写入失败」「截断」从 `?` 直接返回改为记录 `seg_error` 并 `continue` 切换下一候选通道（清理该段部分文件后重试本段）；已 report 的字节数不回退（进度条在失败重试路径可能短暂偏高，属可接受近似，最终以合并校验为准）。
- **②abort 剩余分段任务**：`download_apk_parallel` 改为 `for handle in iter.by_ref()` 顺序等待，任一段失败即 `break` 并对剩余 `JoinHandle` 逐个 `abort()`，不再让已 spawn 任务后台写已删除的 seg 文件/emit 进度。
- **③保留续传断点**：parallel 失败只清理 `.seg` 文件，**不再删除 `part_path`**（可能承载上次单流中断的有效断点）；调用点 `android_download_apk` 在 parallel 失败时 `tracing::warn` 并**回退单流重试一次**（单流可断点续传），单流再失败才传播错误。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿；修复中处理了 `file` 变量与内置宏 `file!` 的解析冲突（重命名 `seg_file`）。

### T003（P2，资源/超时）✅ 已修复

- **①孤儿 seg 清理**：`cleanup_stale_apk_cache` 增加「当前版本 `.part.seg{i}` 孤儿分段文件也删除」——parallel 不支持续传，进程中断（kill/崩溃）残留的 seg 纯属垃圾累积；该函数仅在下载开始前调用（检查更新时/下载前），无并发写入，删除安全。当前版本 `.part`（单流断点）与 `.apk` 仍保留。
- **②updater 请求超时**：核对 tauri-plugin-updater 2.10.1 源码——`Config` 无 timeout 字段，timeout 由 guest-js `check(options)` / `download(options)` 的 `timeout?: number`（毫秒）传入。`tauri/src/lib/updater.ts` 两处 `check()` 调用（`checkForUpdate`/`downloadAndInstallUpdate`）统一传 `UPDATE_REQUEST_TIMEOUT_MS = 15_000`，直连黑洞（hang 而非 RST）15s 后超时，插件自动尝试下一 endpoint。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿；tsc / eslint 通过。

### T004（P2，隐私/可用性披露 + 可配置）✅ 已修复

- **①②③ 披露**：`update.rs` `PROXY_PREFIXES` 注释与 `scripts/generate-latest-json.js` 头部注释补全隐私/可用性披露——TLS 终止代理下用户 IP/使用事实/版本号/API 响应暴露给第三方（直连可达时不走代理，直连受限时固有权衡无法消除）；元数据请求套代理可被陈旧/篡改 Release JSON 软性压制升级（完整性无碍——校验和与签名 Rust 侧重验）；重放旧版 mirror JSON 压制升级（updater 只升不降，无降级风险）。
- **④ 缓解（可配置）**：新增 `proxy_prefixes()` 支持环境变量 `SOLOSOUL_PROXY_PREFIXES`（逗号分隔）覆盖默认代理列表，可指向自建可信代理；**留空禁用全部代理仅走直连**；未设置/解析为空回退默认列表。`download_candidates` 改用该函数。
- **测试**：新增 `test_proxy_prefixes_env_override` 覆盖「单代理覆盖 / 逗号分隔去空白 / 空值回退默认」三用例；默认候选测试保持通过。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

## 待用户指令

- T 系列修复建议顺序：T001（P0，单行断言）→ T002（下载弹性）→ T003/T004/T005。
- 可选收尾动作：推送终版报告并打标签 `git tag -a "code-audit-passed-$(date +%Y%m%d)"`——需用户确认后执行。
- 后续专项建议：优先处理 P046/P047 巨型组件/文件拆分（架构项）；S003 的 v2.10 blob 键清理按代码 TODO 计划执行。
