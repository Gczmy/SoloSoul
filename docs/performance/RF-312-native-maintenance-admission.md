# RF-312 Windows 维护准入归因

本阶段补充现有认证诊断：登录在 `maintenance-admission` 结束为 `interrupted` 时，记录原返回错误的固定类别，并辨认部分仍存活的真实目录访问许可。本阶段已从C盘候选迁入当前主分支基线 `895c86e7`，在D盘完成集成检查和两档原生复跑；源码及实际结果见[结构化证据](rf312-windows-maintenance-admission-2026-10-09.json)。不据部分许可见证宣布首页超时根因。RF-312 保持待验证，RF-121 和后续表面迁移的前置条件不解除。

## 观测边界

仅 Windows 非默认 `native-perf`、显式隔离的合成 Vault 生效。默认构建、Core 准入规则、原错误传播、worker 持有许可直至真实退出的规则及依赖保持。更新源偏好登记现有 `Arc<RootActivityGuard>`；Host `spawn_owned_blocking` 在该测试构建内以 Arc 将同一 guard 交给实际 worker。记录对象仅存 Weak，不 upgrade 或保留新的强引用。这一使用边界与[Rust Weak 官方说明](https://doc.rust-lang.org/std/sync/struct.Weak.html)一致；实际许可及owner释放由本阶段真实guard测试验证，不将单次计数当作全量活动快照。

只覆盖两个固定类别 `update-source-preferences` 与 `owned-blocking-worker`。许可在准入开始前已登记，原错误返回后仍有强引用，才作为该失败的见证。Strong count 降至零不能复活同一 Arc；见证因此覆盖准入区间。不存在见证时，原因仍未知；不得推断所有任务已经空闲。`directory-unavailable`、`directory-busy` 和未知错误不接受活动见证，只有 `operations-active` 绑定已覆盖入口。未知错误只记 `unclassified`。

认证、权限命令和维护诊断在同一把记录锁、同一 process-monotonic 时刻捕获。独立文件 `native-perf-maintenance-admission.json` 使用固定schema，包含公共runId/PID、固定枚举、数值和数组，不含路径、账户、密码、错误原文、弱引用地址或 IPC 正文。最多256次许可登记、64次失败、每次最多256个见证；溢出和重复使证据失效。新文件独立4 MiB读取上限，已有认证文件256 KiB限制保持。

## 校验与复跑

Rust 验证器将每次失败绑定到同次认证的已核验 login、唯一 `maintenance-admission` token 和其时间区间；拒绝已完成阶段、遗漏真实中断、错误时钟、隐私字段、晚登记见证、重复 ID、改变历史见证类别/时刻及越界记录。Node 验证器先执行原认证文档完整契约，再独立复核这些条件；普通认证行程的原四条路由白名单保持，不能用于 PDF 专用结束路径。

在 `tauri/` 执行：

```text
cargo test --locked -p solo_soul --features native-perf native_perf:: -- --nocapture
cargo test --locked -p solo_soul --features native-perf rf905 -- --nocapture
node --test scripts/native-perf-maintenance.test.mjs scripts/native-perf-auth-attribution.test.mjs
node scripts/run-node-tests.mjs
npm run tauri -- build --config src-tauri/tauri.native-perf.conf.json --features native-perf --no-bundle --ci
node scripts/native-perf-auth-attribution.mjs --exe ABS --fixture ABS --output NEW_ABS --samples 3
```

测试子进程可使用现有 PDFium 的绝对路径；原生行程禁止继承该覆盖。驱动预检同时要求认证及维护观测的固定能力标记，旧认证构建在启动前拒绝。EXE、资源与冻结源码需逐字核验；两档合成数据分别至少三次完整行程，原 SDK 128调用及50秒阶段、300秒总预算保持，不选成功子集替代失败整组。所有新增诊断为 `diagnosticOnly`、`performanceMetrics=null`。

## 当前检查状态

2026-10-07旧工作分支的冻结源码检查已完成：Node完整入口222项，221通过、0失败、1既有符号链接权限跳过；Rust格式化、默认Clippy及workspace测试通过，默认Rust共1861通过、0失败、3既有跳过；非默认Windows观测定向113项和RF-905许可/取消等待26项通过，非默认release测试构建成功。旧检查的13个源码文件SHA与迁入前C盘工作文件一致。

2026-10-09 当前基线的D盘集成检查实际完成：TypeScript、ESLint、Prettier通过；完整默认前端260文件/2346 Vitest通过，Node221通过/0失败/1既有符号链接权限跳过；ACL、偏好键、Markdown边界、IPC合同及12项Python检查通过。默认Rust 1864 passed / 0 failed / 3既有ignored；非默认观测 113 passed；RF-905 29 passed。默认与非默认严格Clippy、Rust格式和非默认Windows GUI Release通过。

首轮Tauri公共特性配置不匹配、前端dist缺失和10项D盘测试白名单失败均完整保留。RF-1102只对齐既有公共特性；RF-1103只修测试根，两项独立提交。生产路径与准入规则保持，累计284/293关闭、9项未完成。

两档普通v1夹具由本轮Release生成器在D盘重新生成并实际验证，使用生产KDF64MiB/3/4；未复用媒体夹具或虚称缺失的旧普通数据目录。EXE SHA256 `801281f2b4cd748c0269b1ac3a9cfa205bb2e3d6f82ed060006db7bdb13a170c`，Windows GUI子系统2，94份运行资源逐字核验；原生进程不继承PDFium路径覆盖。冻结1827项构建输入、13项迁移代码及C盘原始备份的一致性记录在证据中。

原生结果：100对象 2/3完整行程通过，退出码1，3/3维护文档经当前合同核验；5000对象 2/3完整行程通过，退出码1，3/3维护文档经当前合同核验。所有失败及完整样本均保留，原128调用/50秒阶段/300秒总预算保持。本阶段仍有整组失败，诊断设施通过源码与回归检查，原生成功验收不提前标通过。RF-312继续待验证；睡眠、同profile热启动、峰值内存及多端同口径验收保持未完成。

本轮经过合同验证的错误类别：operations-active；已覆盖入口的许可见证类别：update-source-preferences。无见证时原因未知，不能推断所有后台任务空闲。

100对象第3次、5000对象第2次均在主密码重解锁的 `maintenance-admission` 被拒绝，两次都记录到覆盖准入区间的更新源偏好许可。下一步审查 `SourcePreferences` 的异步使用路径和许可持有范围，保留真实写入时的准入与worker生命周期约束；本轮不修改Core互斥，也不将这两次重解锁拒绝直接归为旧首页超时根因。

Release编译退出0；Tauri CLI将Cargo.toml从CRLF改写为LF，原始receipt保留sourceUnchanged=false，收尾校验因此停止。完整检查确认其余1826项输入未改、该清单的正文及TOML语义一致；恢复1827项输入原字节后，只继续尚未执行的夹具命令。原始成功日志、停止记录、CLI清单及换行前后SHA全部归档，没有修改依赖或覆盖首次记录。证据目录的Git属性保留原始字节，避免不同系统签出时转换换行。

首次沙箱内两档各在第1个样本的夹具准备退出1；Known Folder被重定向到沙箱目录，且无法核验所属进程，驱动按原清理门禁停止后续样本。因此首次两组各仅尝试1/3，未伪造三次行程或诊断记录。确认无遗留测试进程后，在解除沙箱限制的新auth100-unrestricted/auth5000-unrestricted目录，以相同EXE、资源、夹具、三样本和预算复跑；上述原生结果来自这两组。四组原始结果均归档，生产门禁和清理规则保持。

旧分支文档中的PDFium路径预检失败、f64类型推断编译失败及PowerShell查询超时属于早期历史记录。本轮D盘归档范围与文件哈希以结构化证据为准，成功结果逐项绑定本轮冻结源码。没有为通过检查调整互斥、安全门禁或原生行程预算。
