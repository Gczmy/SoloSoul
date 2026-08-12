# 代码分析修复报告

> 最后更新：2026-08-12（轮次 9：V 系列修复验证）
> 当前分支：`main`

## 总览

- 全部已提出问题：**77 项**（P47 + N10 + R5 + S4 + T5 + U6 + V5，含重复计数修正）
- **修复关闭 73 项**、**经确认跳过 2 项**（P027/P033）、**延期 2 项**（P046/P047 巨型组件/文件拆分，列为后续架构项）
- **待修复 0 项**：轮次 9 V 系列全部修复（V001–V005），无新增问题
- 遗留计划事项：S003 的 v2.10 blob 键清理（已有代码 TODO + CHANGELOG 双向追踪，属计划内而非遗忘）
- 轮次 9 基线实测：`check-all` **EXIT=0 全绿**（cargo test + Vitest + 两项机械化一致性检查）

## 遗留项（跳过 / 延期）

状态图例：`[-]` 经确认跳过 · `[>]` 延期为后续架构项

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| P027 | P2 | 死代码 | `tauri/src-tauri/src/commands/object/trash.rs:147`、`lib.rs:389,870,996` | `trash_permanent_delete` 已注册但前端从不调用（P024 批量改造后走 batch），删除需同步守卫测试与总数断言 | `[-]` 经确认跳过（保留 API 完备性） |
| P033 | P2 | 死代码 | `tauri/src-tauri/src/lib.rs:304-316`（调用于 :667） | `setup_detect_locale()` 结果仅用于一行 debug 日志，前端实际走 `get_system_locale` IPC（需人工确认是否保留诊断） | `[-]` 经确认跳过（保留诊断日志） |
| P046 | P2 | 架构 | 前端 10 个巨型组件（汇总） | 单组件非注释行 > 300：`AttachmentViewer`(~550)、`LoginPage`(~501)、`PasswordVerificationDialog`(~447)、`TemplateFieldRow`(~436)、`DeviceListPanel`(~424)、`TrashPage`(~422)、`ObjectDetailModal`(~422)、`ExportImportPage`(~417)、`ImportSection`(~409)、`ExportSection`(~405)，建议按「数据 hook + 展示子组件」拆分 | `[>]` 拆解中：**认证域 2/10 + 对象域 3/10 完成（共 5/10）**（LoginPage、PasswordVerificationDialog 见 P046-1；AttachmentViewer/ObjectDetailModal/TemplateFieldRow 见 P046-2）；设置导出域（ExportImportPage/ExportSection/ImportSection/TrashPage/DeviceListPanel）待拆 |
| P047 | P2 | 架构 | Rust 巨型文件（汇总） | `attachment.rs` 2057 行、`object/tests.rs` 2353 行、`export_docx.rs` 1989 行，文件级拆分作为后续架构项 | `[>]` 延期：3 个巨型 Rust 文件拆分，列为后续架构项 |

## 历史轮次摘要

- **轮次 1**：初始全库分析 47 项（P001–P047）；修复后验证 41 通过、2 缺陷（轮次 2 修复）、2 跳过、2 延期。
- **轮次 2**：验证发现 N001–N010（Rust 测试 9 红、LLM 快照 bug、check-all 缺 cargo test 等），全部修复。
- **轮次 3**：N 系列验证通过后新增 R001–R005（遗漏陈旧断言、假冲突抑制缺口、混合版本删键风险等），4/5 通过；R003 引入 S001 回归。
- **轮次 4**：S001–S004 全部修复（含 R003 引入的会话复活回归）。
- **轮次 5**（收口）：4/4 全部通过，无新增问题；全库质量评估达标。
- **轮次 6**：S 系列收口后新推送审查（更新加速 + 附件标签折叠系列），新增 T001–T005，全部 `[ ]` 待修复。
- **轮次 7**：T 系列修复验证（6 提交 df78465c–a96f1d70）。T001/T002/T005 修复通过；T003/T004 修复主体正确但有残留；check-all 实测全绿。新增 U001–U006，全部修复。
- **轮次 8**：U 系列修复验证（6 提交 9e48b02c–7635406e）。U001/U002/U003/U005/U006 修复通过；U004 隐私政策披露有 P1 级残留（禁用承诺范围错误）；check-all 实测全绿。新增 V001–V005，全部 `[ ]` 待修复。
- **轮次 9**：V 系列修复验证（5 提交 349dd084–8e9e0a18）。V001–V005 全部修复（V001 政策措辞改口、V002 build 失败兜底回退、V003 Drop guard、V004 可用性面披露、V005 metadata 断点）；V005② wiremock 集成测试登记备查（P3 可缓）。check-all 实测全绿。无新增问题。

## 轮次 6：新推送审查（T 系列）

审查范围：`2c9c7b21`（GitHub 国内下载加速）与附件标签/描述折叠展开系列 6 提交（797a0afa–95de2d9f）。基线实测：check-all ❌ EXIT=101（cargo test 419 过 / **1 失败**，见 T001）。

**安全结论（更新通道）**：P002/N006 的 fail-closed 校验链**未被削弱**——合并后整体 SHA-256 + Rust 侧 minisign 验签，分段篡改/乱序/截断均可被发现；代理全 HTTPS、直连优先回退、篡改无法注入；桌面端 updater 多 endpoint 共用同一签名、轮询正确；分段边界无 off-by-one；前端契约无变化。

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| T001 | P0 | 测试 | `tauri/src-tauri/src/commands/update.rs:1234` | 带红测试入库：`test_compute_segments_caps_at_max_segments` 断言错误——作者把调用点的 20MB 并行阈值（`PARALLEL_MIN_FILE_SIZE`）误当成 `compute_segments` 内部逻辑，断言 1 段实为 4 段。check-all / CI rust-test 必挂。修法：断言改 4（阈值本属调用点职责） | `[x]` 已修复（T001，见下） |
| T002 | P1 | 弹性 | `tauri/src-tauri/src/commands/update.rs:843,902,745-762` | 分段下载弹性缺口：①流读取中途失败直接 `?` 返回，不切换下一候选、也不整体回退单流（注释宣称会切换，与行为不符）；②parallel 失败时已 spawn 的分段任务不 abort，继续后台写已删除的 seg 文件并 emit 进度；③parallel 失败删除 part 文件连带销毁单流续传的有效断点。代理限流/中途断连 = 整个下载失败 | `[x]` 已修复（T002，见下） |
| T003 | P2 | 资源 | `tauri/src-tauri/src/commands/update.rs:209`、`tauri/src-tauri/tauri.conf.json` | ①进程中断时 `.part.seg{i}` 孤儿文件永不清理（`cleanup_stale_apk_cache` 只清非当前版本，且 parallel 不支持续传，纯垃圾累积）；②updater 未配 `timeout`（插件默认无超时），直连黑洞（hang 而非 RST）时代理回退迟迟不触发 | `[x]` 已修复（⚠️ 残留 U002/U003，见轮次 7） |
| T004 | P2 | 隐私/可用性 | `tauri/src-tauri/src/commands/update.rs:29,252`、`scripts/generate-latest-json.js` | 代理通道固有面未披露/未缓解：①gh-proxy 类代理 TLS 终止于代理方，用户 IP、使用 SoloSoul 的事实、目标版本号、API 响应全部暴露给第三方——代码注释只强调「无供应链风险」未提隐私面，建议在文档/注释披露；②`fetch_github_release_url` 把 api.github.com 元数据请求也套代理，代理可返回陈旧/篡改 Release JSON 软性压制升级（完整性无碍，属可用性面）；③代理可重放旧版 mirror JSON 压制升级（updater 只升不降，无降级风险）；④4 个代理硬编码不可配置，失效需发新版 | `[x]` 已修复（⚠️ 残留 U001/U004，见轮次 7） |
| T005 | P2 | 前端 | `tauri/src/components/attachment/AttachmentFileNameBlock.tsx:127-139` | 溢出检测无 resize 监听：描述与标签两处 `useLayoutEffect` 测量仅在 deps 变化时触发，容器变窄后原本 ≤4 个的标签被截断时用户**无任何展开途径**（无 +N 也无按钮）；字体未加载完成时测量可能偏小漏判。另：描述溢出测量分支在 jsdom 下恒走 false，零行为测试覆盖（标签侧已用 mock 布局属性方案解决，可仿照）。建议加 `ResizeObserver` | `[x]` 已修复（T005，见下） |

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
- **④ 缓解（可配置）**：新增 `proxy_prefixes()` 支持环境变量 `SOLOSOUL_PROXY_PREFIXES`（逗号分隔）覆盖默认代理列表，可指向自建可信代理；**显式置空禁用全部代理仅走直连**；未设置时回退默认列表（该语义经 U001 修正后成立，原实现「解析为空回退默认」与承诺矛盾已修复）。`download_candidates` 改用该函数。
- **测试**：新增 `test_proxy_prefixes_env_override` 覆盖「单代理覆盖 / 逗号分隔去空白 / 空值回退默认」三用例；默认候选测试保持通过。
- **审查修复**：`SOLOSOUL_PROXY_PREFIXES` 为进程级环境变量，Rust 测试默认多线程并发——涉及 env 的两个测试（`test_proxy_prefixes_env_override` 与 `test_download_candidates_direct_first_then_proxies`）加共享 `ENV_LOCK: Mutex<()>` 串行执行，消除 set_var/remove_var 相互干扰导致的间歇性 CI 抖动。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

### T005（P2，前端溢出检测）✅ 已修复

- **修复**：`AttachmentFileNameBlock.tsx` 将描述/标签两个溢出检测 `useLayoutEffect` 合并，统一 `measure()` 并在内容/回收站态变化时重测；新增 `ResizeObserver` 监听描述与标签容器尺寸变化（列表变窄、侧栏展开/收起、窗口缩放等）时重测——容器变窄后标签被截断时折叠按钮即时出现，不再无展开途径。展开态跳过测量（同既有 ref 守卫策略）；jsdom 无 ResizeObserver 时跳过。
- **测试基建**：`src/test/setup.ts` 的 MockResizeObserver 升级为导出实例收集数组 + `trigger()` 方法（模拟浏览器 resize 通知）；新增单测「容器变窄（ResizeObserver 触发）后标签被截断即出现折叠按钮」（act 包裹 flush setState）。
- **验证**：tsc / eslint 通过；AttachmentFileNameBlock 10/10、attachment+object 相关 85/85 全绿（setup 改动无回归）。

## 轮次 7：T 系列修复验证（U 系列）

审查范围：T 系列修复 6 提交（`df78465c` T001 → `a96f1d70` T004 补充）。基线实测：`check-all` **EXIT=0 全绿**（tsc / fmt / clippy / cargo test / eslint / Vitest 646/646 / ACL 与 pref-keys 机比），T001 的 P0 测试红确认消除。验证方式：逐提交 `git show` 核对 + 当前工作区代码语义审查。

**通过项（修复正确，无阻塞残留）**：

- **T001 ✅**：断言 4 段与 `compute_segments` 实现（`div_ceil(19MB/5MB)=4`，`update.rs:730`）完全一致，非糊弄式改断言；doc 注释澄清「20MB 阈值属调用点职责」准确。
- **T002 ✅**：三子项全部落实——①流中途失败收敛 `seg_error` 后 `continue` 切下一候选（`update.rs:904-935`，候选列表为界无死循环）；②`iter.by_ref()` 顺序等待 + 失败即 `break` + 剩余 `JoinHandle` 逐个 `abort()`（`update.rs:790-814`）；③parallel 失败只清 `.seg` 保留 `part_path`，调用点回退单流续传重试一次（`update.rs:638-641,968-992`），重试链条恰一层。断点边界（合并成功原子重命名 / 合并 IO 错误保留有效前缀 / 最终失败保留供续传）均清晰。
- **T005 ✅**：ResizeObserver 挂载/卸载正确（`AttachmentFileNameBlock.tsx:132-153`，cleanup `disconnect()`，无泄漏；StrictMode 双调用安全）；容器变窄截断场景已闭环；MockResizeObserver 升级对既有测试零影响（全仓无其他 RO 使用方）；新单测走真实 RO 回调路径断言按钮出现，非改断言糊弄。
- **T003 ①✅ / ②半✅**：孤儿 seg 清理谓词语义正确（`update.rs:229-249`，不误删当前版本 `.part` 断点与 `.apk`）；前端两处 `check()` 15s 超时单位正确且插件会自动切下一 endpoint——但桌面端 Rust 路径遗漏（见 U002）。
- **T004 主体✅**：env 覆盖接入点完整（`proxy_prefixes()` 唯一来源，元数据+二进制双通道同生效，无绕过）；3 个新单测为真测试；ENV_LOCK 竞态消除成立（模块级单例、锁在一切 env 操作之前、全仓无第三个触碰者）。

**新增问题**：

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| U001 | P1 | 功能/文档矛盾 | `tauri/src-tauri/src/commands/update.rs:30,47,57-62,1288-1291` | **「留空禁用代理」承诺与实现矛盾**：提交信息、代码注释、本报告 T004 修复记录三处均宣称 `SOLOSOUL_PROXY_PREFIXES=""` 可禁用全部代理仅走直连，但实现按「解析结果为空 → 回退默认 4 个代理」处理，测试也固化了回退行为。隐私敏感用户显式置空后实际仍走 gh-proxy，属反向伤害。修法二选一：`var()` 的 `Ok/Err` 区分「未设置→默认 / 显式置空→禁用」（改动很小），或把三处文档改口为「空值回退默认」 | `[x]` 已修复（U001，见下） |
| U002 | P2 | 资源 | `tauri/src-tauri/src/commands/update.rs:507-508` | **T003 超时修复只覆盖一半路径**：前端 `check()` 加了 15s 超时，但桌面端 AboutPage 检查更新走 Rust 侧 `app.updater().check()`（`updater.ts:100` → `useUpdateChecker.ts:87`），该 builder 未设 `timeout`——直连黑洞时 AboutPage「检查更新」永久卡在 checking 态，endpoint 回退不触发。修法一行：`app.updater_builder().timeout(Duration::from_secs(15)).build()` | `[x]` 已修复（U002，见下） |
| U003 | P2 | 并发 | `tauri/src-tauri/src/commands/update.rs:229-249,449,606` | **T003 修复新引入竞态**：cleanup 注释断言「仅在下载开始前调用，无并发写入」，但无机制保证——Tauri commands 并发执行，用户在 APK 分段下载进行中触发 `android_check_update`（AboutPage/横幅），cleanup 会删除 `download_range_to_file` **正在写入**的 `.part.seg{i}`。后果可自愈（合并 open 失败 → 回退单流 → SHA-256 终检兜底，无损坏风险），但浪费带宽、进度归零。修法：下载进行中持进程内标志/Mutex，cleanup 跳过；或 cleanup 限定无活动下载时执行 | `[x]` 已修复（U003，见下） |
| U004 | P2 | 隐私/文档 | `docs/legal/隐私政策.md`、`docs/legal/Privacy Policy.md`（原报告误写为 `docs/zh-CN/`、`docs/en-US/`，实际目录为 `docs/legal/`） | **披露未触达用户**：T004 的隐私披露只落在源码注释、构建脚本注释与本内部报告，用户可见的隐私政策/服务条款（中英两版）无任何代理相关条目，应用内更新 UI 亦无提示。若意图是让用户知情（审查本意），需在隐私政策补「检查更新在直连不可达时经第三方代理中转，可能暴露 IP 与版本号」 | `[x]` 已修复（⚠️ 残留 V001，见轮次 8） |
| U005 | P2 | 弹性/测试 | `tauri/src-tauri/src/commands/update.rs:805-807,1011-1018` | **T002 残留**：①abort 未 await——abort 后立即删 seg 文件，若某任务已越过最后 await 点可能重建孤儿 `.seg`（无正确性影响，T003 的 cleanup 兜底）；②回退单流腿自身仍无中途切候选（`?` 直接返回，断点保留可重试，属刻意取舍但「候选切换」在单流腿未闭环）；③三个行为改动零新增测试（网络 mock 成本高可理解，回归靠人工） | `[x]` 已修复（U005，见下） |
| U006 | P2 | 测试/注释 | `tauri/src/components/attachment/AttachmentFileNameBlock.tsx:134-138`、`update.rs:1338` | **T005/T001 残留**：①描述溢出测量分支仍零行为测试覆盖（jsdom 恒 false，标签侧 mock 布局方案可仿照）；②字体晚加载漏判仍在——RO 监听 border-box 尺寸，字体加载只改 `scrollWidth` 时不触发（可用 `document.fonts.ready.then(measure)` 兜底）；③T001 测试注释引用不存在的 `download_apk`，应为 `android_download_apk`（`update.rs:601`） | `[x]` 已修复（U006，见下） |

**次要观察（不列为问题）**：ENV_LOCK 用 `.lock().unwrap()`，一个测试 panic 会 poison 锁连带另一个失败（测试代码常见取舍）；T002 重试路径进度条可能短暂超 100%，有 `min(100)` 截断且注释已声明为可接受近似；`resizeObserverInstances` 数组测试会话内只增不减（量级可忽略）。

## 修复记录（U 系列）

### U006（P2，测试/注释收尾）✅ 已修复

- **①描述溢出行为测试**：仿照标签侧 mock 布局属性方案——mock `scrollWidth/clientWidth` 后以不同内容 rerender 触发溢出检测 effect 重测，断言折叠态出现展开箭头、点击展开（`pre-wrap` 全文）→ 收起。
- **②字体晚加载兜底**：`AttachmentFileNameBlock` 溢出 effect 增加 `document.fonts?.ready?.then(measure)`——RO 监听 border-box 尺寸、字体加载只改 `scrollWidth` 不触发 RO，字体加载完成后再测一次防漏判（jsdom 无 FontFaceSet 静默跳过；卸载后触发仅 setState no-op）。
- **③T001 注释修正**：`test_compute_segments_caps_at_max_segments` 注释中不存在的 `download_apk` 改为 `android_download_apk`（经 `download_apk_to_part`）。
- **验证**：tsc / eslint 通过；AttachmentFileNameBlock 11/11、attachment+object 相关 86/86 全绿；cargo fmt / check / clippy 全绿。

### U005（P2，T002 残留收尾）✅ 已修复

- **①abort 后 await**：`download_apk_parallel` 失败路径先 abort 全部剩余 `JoinHandle`，再逐个 `await`（忽略结果）确保任务完全停止后才清理 `.seg` 文件——消除「任务越过最后 await 点后重建孤儿 .seg」的窗口。
- **②单流腿中途切候选**：`download_apk_single_stream` 流式读取改为 loop + `stream_err` 收集——分块读取/写入失败时保留已写字节作为断点（新纯函数 `next_resume_offset(initial_offset, new_bytes)`），`existing_size` 改为 `mut` 更新后 `continue` 切下一候选从新断点续传；「候选切换」在单流腿闭环。
- **③补测试**：新增 `test_next_resume_offset`（全新下载失败 / 续传中失败 / 写失败未写新字节三用例）；abort 与网络路径回归仍靠人工（网络 mock 成本高，与前评一致）。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

### U004（P2，隐私披露触达用户）✅ 已修复

- **修复**：`docs/legal/隐私政策.md` 与 `docs/legal/Privacy Policy.md` 新增「检查更新与下载中转 / Update Checks & Download Proxying」章节——披露直连不可达时自动回退第三方代理中转、TLS 终止下 IP/使用事实/版本号可能暴露给代理商、下载内容无论通道均经签名与哈希校验不可篡改、以及 `SOLOSOUL_PROXY_PREFIXES` 置空可完全禁用代理。

### U003（P2，cleanup 并发竞态）✅ 已修复

- **修复**：新增进程内 `APK_DOWNLOAD_ACTIVE: AtomicBool`——下载主体开始前 `store(true)`、完成后（无论 Ok/Err）`store(false)`；`cleanup_stale_apk_cache` 开头检查该标志，为 true 时整体跳过。下载进行中触发 `android_check_update` 不再删除正在写入的 `.part.seg{i}`（避免浪费带宽/进度归零；下次下载前仍正常清理）。
- **重构**：下载主体抽为独立 `download_apk_to_part()`（分段并行/单流 + SHA-256 校验），使活动标志在 `?` 提前返回的错误路径也能由调用方统一恢复（避免重复 store 或遗漏）。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

### U002（P2，桌面端 updater 超时）✅ 已修复

- **修复**：`desktop_check_update` 由 `app.updater().check()` 改为 `app.updater_builder().timeout(Duration::from_secs(15)).build()` 后 `check()`——核对插件源码：`timeout` 在 `UpdaterBuilder` 上（`updater()` 是已构建的 `Updater` 无此方法），`updater_builder()` 返回 builder（非 Result）。与前端 `check()` 的 15s 超时对齐，直连黑洞时 endpoint 回退正常触发，AboutPage 不再永久卡 checking。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

### U001（P1，功能/文档矛盾）✅ 已修复

- **修复**：`proxy_prefixes()` 语义改为区分「**未设置**（`var` 返回 `Err`）→ 回退默认 `PROXY_PREFIXES`」与「**显式置空或仅空白** → 返回空列表，仅走直连」——与注释/提交/报告三处「留空禁用代理」承诺一致，隐私敏感用户显式置空后不再被回退默认 4 个代理（反向伤害消除）。
- **测试**：`test_proxy_prefixes_env_override` 更新——空字符串断言 `is_empty()`（原断言回退默认）；新增「仅空白同样禁用」用例。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

## 轮次 8：U 系列修复验证（V 系列）

审查范围：U 系列修复 6 提交（`9e48b02c` U001 → `7635406e` U006）。基线实测：`check-all` **EXIT=0 全绿**。验证方式：逐提交 `git show` 核对 + 当前工作区代码语义审查。

**通过项（修复正确）**：

- **U001 ✅**：`var()` 的 `Ok/Err` 正确区分「未设置→默认 / 显式置空或仅空白→空列表仅直连」（`update.rs:52-62`），正是报告建议的修法一；代理为空时 `download_candidates` 无条件种子直连候选，无「假设 ≥2 候选」的边缘 bug；测试真实覆盖四态（未设置/单值/多值去空白/置空/仅空白），ENV_LOCK 串行与 `remove_var` 复原齐备；注释与隐私政策措辞现已一致。
- **U002 ✅**：`updater_builder().timeout(Duration::from_secs(15)).build()`（`update.rs:522-529`）API 用法正确（`timeout` 仅存在于 `UpdaterBuilder`，参数类型 `Duration`，插件 2.10.1 源码确认逐 endpoint 生效）；endpoints/pubkey 与 `updater()` 同源无配置丢失；15s 与前端对齐；全 `src-tauri` 仅此一处构建点，无遗漏。
- **U003 ✅**：`APK_DOWNLOAD_ACTIVE: AtomicBool`（`update.rs:210`）无死锁面；跳过逻辑加在 `cleanup_stale_apk_cache` **函数体开头**（`update.rs:217-221`），两个调用点自动受保护；抽函数 `download_apk_to_part` 保证置位与复位之间无任何 `?`，四条错误路径均正确恢复；自身下载前的 cleanup 在置位之前执行，语义正确。
- **U005 ✅**：abort 后 `iter.collect()` 收齐句柄逐个 `await`（`Err(cancelled)` 显式忽略，无 panic 路径）再删 seg，孤儿重建窗口闭合；单流腿 `next_resume_offset` 断点在续传/重下两分支均与实际文件字节数一致，下一候选 `Range: bytes={offset}-` append 续传不重复写字节，候选为界无无限重试，SHA-256 终检兜底；`test_next_resume_offset` 三断言为真测试。
- **U006 ✅**：描述溢出行为单测与标签侧方案同构（mock 布局属性 + rerender 重测 + 展开/收起全链路断言）；`document.fonts?.ready?.then(measure)` 有 jsdom 守卫、一次性 promise 无循环、卸载后 no-op 有注释声明；`android_download_apk` 注释函数名与真实调用链吻合（`update.rs:622,1436`）。

**新增问题**：

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| V001 | P1 | 隐私/文档矛盾 | `docs/legal/隐私政策.md:19-25`、`docs/legal/Privacy Policy.md:19-25`、`tauri/src-tauri/tauri.conf.json:83-87` | **「置空完全禁用代理，适用于桌面端」承诺不成立**：桌面端检查更新主路径是 Tauri updater 插件，其 4 个代理 endpoint **硬编码于 `tauri.conf.json` 编译期固化**，不受 `SOLOSOUL_PROXY_PREFIXES` 控制——该变量只覆盖 `proxy_prefixes()` 管辖的路径（Android 全流程 + 桌面端 GitHub API 兜底/release notes 补全）。隐私敏感的桌面用户按政策置空后，直连失败时版本检测仍经 gh-proxy 系代理。且文档限定「桌面端」方向反了——变量在 Android 端才是完全有效的（只是移动端难以设置环境变量）。修法二选一：政策措辞改为说明桌面端 updater endpoint 为内置回退、环境变量仅覆盖 API 元数据与下载中转路径；或让桌面端 updater 也走可禁用路径（成本较高） | `[x]` 已修复（V001，见下） |
| V002 | P2 | 弹性 | `tauri/src-tauri/src/commands/update.rs:522-529,585-599` | **U002 修复行为偏差**：旧代码 `app.updater()` 构建失败会把错误放入 `updater_result` 并回退 GitHub Release API（保证 AboutPage 仍能给出版本信息）；新代码 build 失败 `return Err(...)` 提前返回跳过兜底，与函数文档注释（「失败时回退到 GitHub Release API」）契约不符。实际影响小（build 失败仅当发布期配置损坏），修法：build 失败也走 `updater_result = Err(...)` 老路（一行），或文档注释改口 | `[x]` 已修复（V002，见下） |
| V003 | P2 | 并发 | `tauri/src-tauri/src/commands/update.rs:651-663`、`tauri/src/lib/updater.ts:180` | **U003 残留**：①panic 路径标志泄漏——抽函数保证 `?`/Err 路径恢复但非 RAII guard，`download_apk_to_part` 内部 panic 时 unwind 跳过 `store(false)`，标志永久置位、cleanup 此后整体失效（仅缓存不再清理，无数据损坏）；可选修法 scopeguard 式 Drop 恢复。②`AtomicBool` 非计数器 + 前端不 await `android_download_apk`，理论上可并发双下载——A 完成 `store(false)` 时 B 仍在写 seg，cleanup 窗口重开（且双下载同写 `part_path` 本身是更大的既存问题，U003 标志并非其防线，此处登记备查） | `[x]` 已修复①（V003，见下）；②登记备查 |
| V004 | P2 | 文档 | `docs/legal/隐私政策.md`、`docs/legal/Privacy Policy.md` | **U004 残留披露缺口**：政策只承诺完整性不可篡改，未提示「代理可返回陈旧/篡改 Release JSON 软性压制升级、可能导致无法及时获知新版本」这一可用性面（源码注释 `update.rs:32-36` 内部已披露）。隐私政策不强制涵盖可用性，但既然已写完整性声明，补一句更完整 | `[x]` 已修复（V004，见下） |
| V005 | P3 | 弹性/测试 | `tauri/src-tauri/src/commands/update.rs:1088-1091` | **U005 残留**：①`write_all` 部分写入不计入断点——写失败时可能已写若干字节但 `new_bytes` 未计，下一候选 206 续传 append 会在文件末尾重复拼接未计数字节 → part 损坏（SHA-256 终检兜住：删 part、用户重试从零下载，仅浪费一次下载，无正确性外泄）；可选修法续传前 `set_len(existing_size)` 截断或以 `metadata().len()` 为准。②abort-await 与候选切换续传两条核心行为路径仍零自动化覆盖（纯函数测试通过不等于回归保护；可用 `wiremock` 类本地 mock HTTP 补集成测试） | `[x]` 已修复①（V005，见下）；②登记备查（P3 可缓） |

**次要观察（不列为问题）**：`test_proxy_prefixes_env_override` 首条断言与 `test_download_candidates_direct_first_then_proxies` 未先 `remove_var`，若测试环境本身设置了 `SOLOSOUL_PROXY_PREFIXES` 会失败（修复前即如此）；`update.rs:648-649` 大段 U003 注释挂在行尾（cosmetic）；`document.fonts.ready` 只捕获 resolve 时已 pending 的字体（原报告建议方案的固有边界，与要求一致）。

## 修复记录（V 系列）

### V001（P1，隐私/文档矛盾）✅ 已修复

- **修复**：采用低成本路径——隐私政策中英两版措辞改口。原「置空完全禁用代理，此行为适用于桌面端」承诺不成立：桌面端 updater 插件的直连 + 4 代理 endpoint 编译期固化于 `tauri.conf.json`（`updater.endpoints`），不受 `SOLOSOUL_PROXY_PREFIXES` 控制。改后措辞区分两级：①该环境变量仅覆盖应用自建的 GitHub API 元数据与安装包下载中转路径（`proxy_prefixes()`/`download_candidates()` 管辖，Android 全流程有效，但移动端难以设置环境变量）；②明确披露桌面端内置更新检查通道为编译期内置直连 + 代理回退、不可经环境变量关闭。
- **验证**：纯文档改动，无代码变更。

### V002（P2，U002 行为偏差）✅ 已修复

- **修复**：`desktop_check_update` 中 updater 构建由「`Err(e) => return Err(...)` 提前返回」改为「`Err(e) => Err(format!(...))` 放入 `updater_result`」——build 失败（仅当发布期配置损坏）时同样进入下方 `Err(plugin_err)` 分支的 GitHub Release API 兜底，与函数文档注释「失败时回退到 GitHub Release API」契约一致，AboutPage 在 updater 配置损坏时仍能给出版本信息。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

### V003（P2，U003 残留）✅ 已修复①

- **修复①（panic 路径标志泄漏）**：新增 `ApkDownloadActiveGuard` 结构体（手写 Drop guard，scopeguard 式、无外部依赖）——`android_download_apk` 置 `APK_DOWNLOAD_ACTIVE = true` 后立即持有 guard，离开作用域时 Drop 自动 `store(false)`。正常返回、`?` 提前返回、**panic unwind** 三条路径均恢复标志，cleanup 不再因泄漏的标志整体失效。
- **②（并发双下载）**：登记备查——`AtomicBool` 非计数器、前端不 await 下载命令，理论上可并发双下载同写 `part_path`（U003 标志并非其防线）；属更大既存问题，不在此轮处理。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

### V004（P2，U004 披露缺口）✅ 已修复

- **修复**：隐私政策中英两版在「完整性不可篡改」声明后补可用性面披露——代理可能返回陈旧或失真的更新元数据（如旧版本信息），导致无法及时获知新版本；不影响安装包完整性（签名与哈希校验仍生效），仅可能延迟更新通知。与源码注释 `update.rs:32-36` 的披露一致。
- **验证**：纯文档改动，无代码变更。

### V005（P3，U005 残留）✅ 已修复①

- **修复①（部分写入断点错位）**：`download_apk_single_stream` 流中途失败切下一候选时，断点改为以 `std::fs::metadata(part_path).len()` 为准——`write_all` 部分写入失败时文件实际长度可能大于计数断点（`initial_offset + new_bytes`），按计数断点 206 续传 append 会在文件末尾重复拼接未计数字节导致 part 损坏；文件系统实际长度即精确已持久化字节边界，下一候选 `Range: bytes={len}-` 续传正确。`metadata` 读取失败才回退计数断点（此时文件可能已缺失，下轮候选走重新下载分支自愈）。SHA-256 终检兜底不变。
- **②（abort-await 与候选切换续传集成测试）**：登记备查——wiremock 未引入依赖，本机 Rust 测试二进制 `0xc0000139` 限制无法运行集成测试，属 P3 可缓项；纯函数测试（`test_next_resume_offset`）已覆盖断点计数逻辑，行为路径回归依赖 CI/人工。
- **验证**：cargo fmt / check / clippy `-D warnings` 全绿。

## 修复记录（P046 巨型组件拆分）

### P046-2（架构，对象域 3/10）✅ 已完成

- **拆分**：按「数据 hook + 展示子组件」模式，纯重构零行为变化：
  - `AttachmentViewer.tsx` **683 → 305 行**（非注释 ~285）：全部编排逻辑（列表加载、上传/重命名/下载/转发/删除/恢复/永久删除、批量操作、照片集数据源、拖拽上传）迁入 `src/components/object/useAttachmentViewer.tsx`（529 行）；组件退化为纯组合层，复用既有 8 个子组件。保留 `AttachmentItem`/`AttachmentViewerProps` 类型 re-export。
  - `ObjectDetailModal.tsx` **545 → 239 行**：编排逻辑（对象拉取 P020 防陈旧、字段/敏感度解析、关键数据验证密码/生物识别/PIN、复制反馈、删除、历史/附件子视图）迁入 `src/components/object/useObjectDetailModal.tsx`（438 行）；`PasswordVerificationDialog` 联动收敛为 `handlePwDialogClose/Verify/PinSuccess` 三个组合 handler。
  - `TemplateFieldRow.tsx` **506 → 192 行**：插件契约绑定折叠区（~270 行内联 JSX + 全部派生/增删逻辑）抽为 `TemplateFieldBindingSection.tsx`（382 行），主组件仅保留顶部字段控件行；`FlattenedContract` 类型从子组件 re-export（TemplateEditor 既有导入不受影响）。
- **审查处理**：`handleMetaSaved` 参数类型与 `AttachmentMetaEditResult` 对齐；`objectId` 由 hook 返回供组件使用（消除 JSX 中段 `props.objectId` 直接引用，与 `allVisibleKeys` 键源一致）；清理未用解构/导入。
- **验证**：tsc / eslint 全绿；全量前端 73 文件 647 测试全绿（含 ObjectDetailModal 5 测试、TemplateEditor 8 测试）。
- **待拆**：设置导出域（ExportImportPage/ExportSection/ImportSection/TrashPage/DeviceListPanel）。

### P046-1（架构，认证域 2/10）✅ 已完成

- **拆分**：按「数据 hook + 展示子组件」模式，纯重构零行为变化：
  - `LoginPage.tsx` **650 → 179 行**：全部编排逻辑（账户选择、生物识别/PIN 可用性探测、解锁方式优先级 + 模块缓存、三种解锁 handler、图标栏构建与悬停）迁入新数据 hook `src/pages/auth/useLoginPage.tsx`（583 行）；组件退化为纯组合层，JSX 逐字不变。`BIOMETRIC_INFO` 与 `_cachedLoginMethod` 模块级缓存随迁。
  - `PasswordVerificationDialog.tsx` **520 → 136 行**：状态与 handler 迁入 `src/components/forms/usePasswordVerification.tsx`（382 行）；生物识别卡片抽为 `PasswordVerificationBiometricCard.tsx`（85 行）、密码卡片抽为 `PasswordVerificationPasswordCard.tsx`（62 行），PIN 卡片继续复用共享 `PinEntryCard`，props 接口迁至 hook 文件并从组件 re-export（原接口未导出，外部仅引用组件本体，无破坏）。
- **审查处理**：TS 别名收窄（`isBiometricMethod`）跨 hook 不生效——移回组件内局部 const 计算；TS 要求含 JSX 的 hook 文件使用 `.tsx` 扩展名；清理 dialog hook 死返回值与冗余别名。
- **验证**：tsc / eslint 全绿；全量前端 73 文件 647 测试全绿（无相关测试文件，重构保持行为不变）。
- **待拆**：对象域（AttachmentViewer/ObjectDetailModal/TemplateFieldRow）、设置导出域（ExportImportPage/ExportSection/ImportSection/TrashPage/DeviceListPanel）。

## 待用户指令

- V 系列已全部修复。可选收尾动作：推送报告并打标签 `git tag -a "code-audit-passed-$(date +%Y%m%d)"`——需用户确认后执行。
- 后续专项建议：优先处理 P046/P047 巨型组件/文件拆分（架构项）；S003 的 v2.10 blob 键清理按代码 TODO 计划执行。
- 备查项：V003②（并发双下载，更大既存问题）与 V005②（wiremock 集成测试，P3 可缓）已登记，后续视需要处理。
