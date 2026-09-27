# SoloSoul 重构修复执行报告

> 最后更新：2026-09-27（继续逐项修复）
> 当前分支：`main`；调查基线：`f77c0e20`，执行时重新读取 HEAD。
> 修复轮次：第 1 轮，执行中。Cua 接入继续暂缓。

## 1. 文档用途与执行边界

本报告将 [全栈调查报告](ARCHITECTURE_REFACTOR_REVIEW_2026-09-25.md) 的 R01–R22 拆成可独立执行、验证和提交的任务，参考 [代码审查流程](review_code_process.md) 的“准备→排序→逐项修复→更新报告→独立提交→复审”步骤。原报告保留事实与背景；**本文件是这轮修复的执行台账**，无需从旧 `CODE_ANALYSIS_REPORT.md` 重新选择历史任务，也不覆盖旧终版报告。

用户已授权按照本报告开始逐项修复与独立提交；本次不自动推送或发布。任务涵盖缺陷修复、行为保持的重构、验证设施、文档和测量，不应将任务总数解读成缺陷总数。

### 1.1 对参考流程的具体化

- 保留“一项一修复一提交”。一个任务可包含同一行为修复所需的 Rust、TS、测试和规范更新；按语言分组只是同优先级、无依赖时的调度偏好，不能拆断跨端正确性。
- 参考流程要求先处理脏工作树；本轮已知的 Cargo 配置、NSIS 图片和搜索索引等改动不能不加区分地提交。后续先核对归属，保留无关改动，只暂存当前 ID 的路径或补丁；确实重叠时在实施前协调，或在合适的隔离分支执行。
- 仓库根目录含 `tauri/`；Rust workspace 清单位于 `tauri/Cargo.toml`，CLI 位于独立的 `solosoul_cli/`。不能照旧文档假设根目录有 Cargo.toml。
- 不照抄固定 `git push origin main` 或 `git push --tags`。后续推送沿用届时已授权的远端/分支；已有授权不重复询问，本报告也不额外授权发布或打“审计通过”标签。
- 长文件和重复选择器是排查线索，不能仅因行数超过阈值就重写。新架构先在现有 crates 和目录内落地，不引入 Cua 或全面换栈。
- 文档/测量任务通过产出和证据验收；可逆低影响样式变更优先复用现有 E2E，不编写仅复述实现的单元测试。
- 发现误报时记录当前代码、复现方法和证据，结论写“排除/现有实现满足”；不能为凑提交制造无行为价值的修改。此类任务提交的是有依据的报告更新。

### 1.2 状态与关闭条件

| 标记 | 含义 | 是否计入已关闭 |
| --- | --- | --- |
| `[ ] 待执行` | 未开始，或前置任务未完成 | 否 |
| `[~] 进行中` | 当前唯一的实施任务 | 否 |
| `[!] 待验证/阻塞` | 记录具体缺失环境、失败检查或外部条件 | 否 |
| `[x] 完成` | 行为验收、必需检查、规范更新与独立提交完成 | 是 |
| `[x] 排除` | 已核实误报/现有实现满足，并提交证据 | 是，但不计作修复 |

提交了代码但缺少必需的目标平台验证，仍为 `[!] 待验证`。跳过测试、没有设备、只做 cargo check、只通过浏览器 mock，均不能替代任务要求的原生运行证据。

**每项共同完成定义：**确认现状和影响范围；只实现任务列出的行为；新增有意义的回归或使用已有验证；运行该项验证配置；记录真实退出码、测试数量、跳过项与设备；更新受影响的 canonical 规范；检查 staged diff 只含本项；完成一条包含任务 ID 的提交。

## 2. 开始与续跑步骤

1. 读取本文件、当前 AGENTS.md、参考流程和相关领域规范；运行 `git status --short`、`git branch --show-current`、`git log -1 --oneline`。核对当前代码是否已被其他任务改变，不仅依据旧行号。
2. 记录分支、HEAD、工具版本和脏文件归属。不要自动 reset、清理用户数据或打包无关改动。新分支如有需要使用 `codex/` 前缀。
   若调查/执行报告尚未入库，进入实际修复阶段后先做单独的 docs 基线提交，不能将它与首个业务修复或未知本地改动打包。
3. 首次修复前运行适用的全量基线：`tauri/` 的 `npm run check-all`，以及 CLI 检查配置；原生相关任务核对 SDK/设备。基线失败先区分环境阻塞与产品缺陷；缺 SDK、工具下载失败等记录具体阻塞，确认独立代码缺陷后再登记 RF-900 起的新任务。先解决影响当前验收的失败，独立区域可以继续。
4. 按任务索引顺序选取第一个前置已关闭、执行环境可用的未完成任务。P1 优先；P1 必需的基础任务可以先执行。被设备阻塞的任务记录原因并保留未完成，不阻塞无依赖任务。
5. 更新“当前处理”为该 ID → 确认触发条件/建立回归 → 修复 → 定向验证 → 所属栈必需检查 → 回填记录 → 独立提交。
6. 提交前更新索引状态和日志；“提交”栏可写“本提交，使用 ID 检索”，避免试图把当前提交自身的 SHA 写进自身。下一任务或最终复审时补录 `git log --grep=<ID>` 得到的 SHA。
7. 不重复运行已经通过且未受后续改动影响的检查；每项仍需完成本次改动对应的必要检查。失败后只能把真实原因登记为失败/阻塞，不能通过减少测试、跳过错误或降低安全规则换通过。
8. 当前索引清空前不能宣称“全库所有问题修复”。最终复审按第 8 节进行；新发现继续追加 ID，不重新编号或删掉历史任务。

可直接用于后续执行的指令：

> 按 docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md 继续修复。先核对当前工作树和未完成任务，从依赖满足的最高优先级项开始；一项一修复一验证一提交，每项同步回填报告。保留无关改动，缺少平台验证时如实标记，不提前标完成。Cua 接入保持暂缓。

## 3. 验证配置与真实工作目录

每张任务卡列出配置代码及定向场景。**定向测试用于快速定位，不替代所属栈的公共检查。** 下列命令以正确工作目录执行，每条分别检查退出码；依赖未安装时先按 lockfile 安装，不为本次重构顺手升级依赖。

| 配置 | 工作目录 | 必需命令/证据 |
| --- | --- | --- |
| F | `tauri/` | `npx tsc --noEmit`；`npm run lint`；`npm run test`。对修改的 TS/TSX 文件单独运行 Prettier，不运行全仓库格式化。 |
| R | `tauri/` | `cargo fmt --all -- --check`；`cargo clippy -- -D warnings`；`cargo test --verbose`。修改前使用定向包/用例定位，完成时检查受影响 workspace。 |
| CORE | `tauri/` | 对实际受影响包执行 `cargo test -p solosoul-core`、`cargo test -p solosoul-vault` 等定向检查；仍须满足 R。GUI 包名是 `solo_soul`。 |
| CLI | `solosoul_cli/` | `cargo fmt --check`；`cargo clippy --all-targets -- -D warnings`；`cargo test --verbose --no-fail-fast`。共享 Rust 变化同时检查 R。 |
| CONTRACT | `tauri/` | `npm run check:acl`；`npm run check:pref-keys`；`node scripts/check-markdown-chunk-boundary.mjs`；RF-301 完成后再加其实际生成检查命令。 |
| WEB | `tauri/` | 卡片列出的 Playwright 用例，分别按适用的 `--project=chromium`/`--project=mobile` 运行；启动/路由/IPC/主题变化另跑 `npm run test:e2e:production`。 |
| COVERAGE | `tauri/` | `npm run test -- --coverage`，保存覆盖率报告和阈值结果；不得将未运行的阈值配置视为已达标。 |
| IOS | macOS 上的 `tauri/` | 安装需要的 Rust targets 后 `cargo check --target aarch64-apple-ios --target aarch64-apple-ios-sim`；原生桥接改动另执行 `npm run tauri:ios:build:sim` 和相关运行验收。 |
| ANDROID_BUILD | `tauri/` 与生成 Android 工程 | 下节 Android target check、Debug 构建及本项 Kotlin 单元检查；需要 SDK/NDK/JDK，不默认要求连接玻璃设备。 |
| ANDROID_NATIVE | 生成 Android 工程与设备 | 在已验证 Debug 构建上运行任务指定的 instrumented/设备场景；RF-310 必须执行现有玻璃原生测试。记录 ABI/API、测试计数与跳过项。 |
| NATIVE | 对应平台图形会话 | 原生应用或例程中的具体操作，记录 OS、设备、构建 ID、主题/辅助功能设置和结果；无统一跨平台替代命令。 |
| PERF | 对应平台 release 测试构建 | 固定合成数据集、冷/热启动口径、重复次数、设备/版本，记录时延分布、内存与 IPC 指标。单次主观观感不算证据。 |
| DOC | 仓库根目录 | 检查新改文档链接、路径、任务 ID、依赖与事实；`git diff --check`。纯文档任务不重新跑业务测试。 |

任务卡中的“新增测试”指建议落点，不声称当前文件已存在；筛选器必须确认确实运行了目标测试，0 tests 不算通过。优先复用现有测试模块，现有测试名在执行时通过 `--list`/源码核对，不臆造一个筛选名称然后接受零命中。

### 3.1 本机前端历史基线

2026-09-25 调查时 TypeScript、Lint、Rust fmt、ACL（219 命令）、偏好键（22 key）通过；默认 Vitest 进程池未正常结束，换成下列命令后 139 文件、1,142 测试通过：

```text
npm run test -- --pool=threads --maxWorkers=2
```

这只是调查基线，本轮编制报告未重跑。若后续再次出现同样环境现象，记录默认运行的失败/超时，再使用线程池核验；优先使用 CI 的 Node 22。RF-316 专门处理默认入口稳定性。不要取消检查或把替代命令的通过写成默认入口已经修好。

### 3.2 Android 命令与前置条件

**首次构建前先处理 JDK：**其他需要 Android 构建的任务先完成 RF-208；执行 RF-208 本身时，先移除项目中的机器专属 JDK 配置，再进行其构建验收。确认 `JAVA_HOME` 指向有效 JDK，运行生成 Android 工程的 `gradlew --version` 核对实际 JVM。仅设置 `JAVA_HOME` 不会覆盖仍存在的项目 `org.gradle.java.home`，不要等第一次 Tauri build 失败后才处理。

在 `tauri/`：

```text
cargo ndk -t aarch64-linux-android check
npx tauri android build --debug --target aarch64 --split-per-abi --apk --ci
```

需要有效 Android SDK、NDK、JDK 和 Rust Android target；当前 CI 配置 JDK 21、NDK 27.0.12077973。项目 `gen/android/gradle.properties` 固定了 macOS JBR 路径，因此按上面的 RF-208 前置先消除该依赖。Debug 验证不读取 Release 签名密钥。

在 `tauri/src-tauri/gen/android/`，Windows 的已存在文档路线如下；须先确认 `JAVA_HOME` 指向可用 JDK：

```powershell
.\gradlew.bat "-Dorg.gradle.java.home=$env:JAVA_HOME" :app:assembleArm64DebugAndroidTest -x :app:rustBuildArm64Debug
adb install -r app/build/outputs/apk/arm64/debug/app-arm64-debug.apk
adb install -r app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk
adb shell am instrument -w -e class com.solosoul.app.AndroidGlassInstrumentedTest com.solosoul.app.test/androidx.test.runner.AndroidJUnitRunner
```

只有**刚完成对应本次源码的 Rust/APK 构建**才可使用上述 `-x`；不能跳过待验证的 Rust 改动。macOS 用 `./gradlew`。可用连接设备任务应按 flavor 核对为 `:app:connectedArm64DebugAndroidTest`，先用 `tasks --all` 确认实际任务；本轮仅核对配置，未运行 Gradle。

上述路线需要 ARM64 设备/模拟器。现有 universal APK 仅包含 ARM ABI，不能拿普通 x86_64 模拟器安装失败来判断功能失败。玻璃测试要求 API ≥31 且系统允许窗口模糊；assumption 跳过只能登记覆盖不足。运行前使用专用测试账户/设备，安装覆盖不会用于用户生产数据。

### 3.3 macOS、Windows 与 iOS 原生证据

- macOS 窗口例程：在 `tauri/` 运行 `cargo run -p solo_soul --example macos_window_appearance -- --manual`；检查交通灯、圆角、背景恢复与主题切换。其他 OS 打印“不支持”且退出 0 不算通过。
- Windows：工作目录保持 `tauri/`，设置 `SOLOSOUL_DATA_DIR` 指向专用测试目录后运行 `npm run tauri -- dev`，验收 Mica、内容/AppBar、一致主题、缩放、最小化恢复、高对比。
- iOS 的 device/sim Rust check 已在 PR CI 存在，不能描述成零覆盖；它不证明 Xcode 链接、安装、Keychain/生物识别或 OCR 运行。
- 浏览器 mobile/native-theme/glass 用例保留，但只证明模拟下的布局与状态，不证明 DWM/NSGlass/Android 合成器。
- SDK 缺失、端口/子进程受限、设备未授权、工具下载失败单列为环境阻塞；编译错误、断言失败、错误 UI 行为列为产品失败。`npm run build` 会生成资源，执行后核对差异，禁止顺手提交无关生成产物。

## 4. 修复进度与执行索引

- 任务总数：**95**（P1：34；P2：60；P3：1）。
- 已关闭：**30 / 95**；实际修复（已关闭）：30；排除：0；待验证/阻塞：5。原计划 90 项，执行中新增 RF-900、RF-901、RF-902、RF-903、RF-905（RF-904 仅保留编号，改密目录疑点尚待真实复现，未登记为任务）。
- 当前处理：无；RF-008 已完成，下一可执行项为 RF-010（共享模板初始化），只读预研已保存。RF-905 的同步生命周期及 Host Vault 切换/附件 worker 接入被自动审批拒绝；46 份未集成源码已 SHA-256 校验归档至恢复目录 `rf905-proposal/`，44 份 tracked 源码恢复 HEAD、2 份本次新增草案移出工作树，未编译/测试，待具体授权。RF-021 Host 接入被自动审批拒绝，14份已批准草案已校验归档至恢复目录 `rf021-proposal/`，相关源码和规范已恢复 HEAD；未编译或运行草案测试，待明确授权。RF-016 Core 写入被自动审批拒绝，获批的11份源码/测试已校验归档至恢复目录 `rf016-proposal/` 并从当前源码恢复，未留下未集成 Rust 依赖；待明确授权后恢复该项。RF-014、RF-104 同样仍等待明确授权。RF-903 保持待执行，依赖 RF-905。
- 编号按领域分段，不代表优先级；下表已按依赖和风险排序。RF-101、RF-019 等基础项虽标 P2，可因 P1 依赖先执行。
- 默认一项完成后再进入下一项；同文件关联任务串行实施。下表是初始推荐顺序，续跑时跳过已关闭项，对环境阻塞项保留记录并选择无依赖任务。

| 顺序 | ID | 优先级 | 任务 | 前置任务 | 状态 |
| ---: | --- | --- | --- | --- | --- |
| 1 | [RF-100](#rf-100) | P1 | 自动上下文立即排除非公开字段 | 无 | [x] 完成 |
| 2 | [RF-001](#rf-001) | P1 | 建立最小会话捕获与提交校验机制 | 无 | [x] 完成 |
| 3 | [RF-002](#rf-002) | P1 | LLM 流式回复绑定请求开始时的会话 | [RF-001](#rf-001) | [x] 完成 |
| 4 | [RF-003](#rf-003) | P1 | 云同步一轮操作固定账户与会话 | [RF-001](#rf-001)、[RF-020](#rf-020) | [x] 完成 |
| 5 | [RF-101](#rf-101) | P2 | 本次用户消息只追加一次 | 无 | [x] 完成 |
| 6 | [RF-004](#rf-004) | P1 | Rust 生成普通聊天的受控自动上下文 | [RF-001](#rf-001)、[RF-100](#rf-100)、[RF-101](#rf-101) | [x] 完成 |
| 7 | [RF-005](#rf-005) | P1 | 普通聊天通过 provider ID 在 Rust 解析凭证 | [RF-001](#rf-001)、[RF-002](#rf-002)、[RF-004](#rf-004) | [x] 完成 |
| 8 | [RF-102](#rf-102) | P1 | 搜索查询与缓存写入绑定会话和请求代次 | 无 | [x] 完成 |
| 9 | [RF-103](#rf-103) | P1 | 聊天会话读取只接纳最新选择 | 无 | [x] 完成 |
| 10 | [RF-104](#rf-104) | P1 | 聊天流归属明确且最终回复只有一个持久化写入者 | [RF-101](#rf-101)、[RF-103](#rf-103)、[RF-002](#rf-002)、[RF-005](#rf-005) | [!] 阻塞：待明确授权 |
| 11 | [RF-105](#rf-105) | P1 | 历史未揭示值不再以原文加 blur 渲染 | 无 | [x] 完成 |
| 12 | [RF-106](#rf-106) | P1 | 对象详情采用共享字段展示策略 | [RF-100](#rf-100) | [x] 完成 |
| 13 | [RF-107](#rf-107) | P1 | 历史快照迁入共享字段展示策略 | [RF-105](#rf-105)、[RF-106](#rf-106) | [x] 完成 |
| 14 | [RF-108](#rf-108) | P1 | 搜索命中值采用共享保护与验证入口 | [RF-102](#rf-102)、[RF-106](#rf-106) | [x] 完成 |
| 15 | [RF-006](#rf-006) | P1 | GUI 回滚拒绝其他对象的快照 | 无 | [x] 完成 |
| 16 | [RF-007](#rf-007) | P1 | CLI 回滚恢复并保留字段标签 | 无 | [x] 完成 |
| 17 | [RF-009](#rf-009) | P1 | 修复 CLI 创建对象缺失模板元数据 | 无 | [x] 完成 |
| 18 | [RF-011](#rf-011) | P1 | CLI 恢复兼容 GUI Base64 Profile 备份 | 无 | [x] 完成 |
| 19 | [RF-012](#rf-012) | P1 | GUI 备份遇到 Profile 读取失败时中止 | 无 | [x] 完成 |
| 20 | [RF-014](#rf-014) | P1 | 全量云快照包含全部有效附件 | 无 | [!] 阻塞：待明确授权 |
| 21 | [RF-017](#rf-017) | P1 | 导出完成后才替换目标包 | 无 | [x] 完成 |
| 22 | [RF-018](#rf-018) | P1 | 导入模板保存失败必须传播 | 无 | [x] 完成 |
| 23 | [RF-019](#rf-019) | P2 | 数据库事务失败时自动回滚 | 无 | [x] 完成 |
| 24 | [RF-016](#rf-016) | P1 | 永久删除附件采用可恢复清理意图 | [RF-019](#rf-019) | [!] 阻塞：待明确授权 |
| 25 | [RF-020](#rf-020) | P1 | 导入失败返回真实部分提交状态 | [RF-018](#rf-018) | [x] 完成 |
| 26 | [RF-021](#rf-021) | P1 | 对象模板与历史按导入批次事务提交 | [RF-018](#rf-018)、[RF-019](#rf-019)、[RF-020](#rf-020) | [!] 阻塞：待明确授权 |
| 27 | [RF-022](#rf-022) | P1 | 附件导入可恢复且同一任务重试幂等 | [RF-020](#rf-020)、[RF-021](#rf-021) | [ ] 待执行 |
| 28 | [RF-208](#rf-208) | P2 | 移除 Android 构建的本机 JDK 路径依赖 | 无 | [ ] 待执行 |
| 29 | [RF-201](#rf-201) | P1 | 修正移动端跟随系统的主题来源 | [RF-208](#rf-208) | [ ] 待执行 |
| 30 | [RF-110](#rf-110) | P1 | 同次主题应用只解析一次系统模式 | 无 | [x] 完成 |
| 31 | [RF-111](#rf-111) | P1 | 设置保存失败返回明确结果并反馈用户 | 无 | [x] 完成 |
| 32 | [RF-112](#rf-112) | P1 | ThemeController 成为唯一主题应用协调器 | [RF-110](#rf-110)、[RF-111](#rf-111)、[RF-201](#rf-201) | [ ] 待执行 |
| 33 | [RF-202](#rf-202) | P1 | 将 APK 更新入口限定为 Android | 无 | [x] 完成 |
| 34 | [RF-203](#rf-203) | P1 | 明确 iOS OCR 不支持时的前后端行为 | [RF-208](#rf-208) | [ ] 待执行 |
| 35 | [RF-204](#rf-204) | P1 | 核实并修正 iOS Keychain 成功状态符号 | 无 | [ ] 待执行 |
| 36 | [RF-008](#rf-008) | P2 | GUI 与 CLI 迁移到同一回滚用例 | [RF-006](#rf-006)、[RF-007](#rf-007) | [x] 完成 |
| 37 | [RF-010](#rf-010) | P2 | 共享对象创建的模板初始化规则 | [RF-009](#rf-009) | [ ] 待执行 |
| 38 | [RF-013](#rf-013) | P2 | 共享 Profile 备份清单与兼容解码 | [RF-011](#rf-011)、[RF-012](#rf-012) | [ ] 待执行 |
| 39 | [RF-015](#rf-015) | P2 | 显式表示附件导出范围 | [RF-014](#rf-014) | [ ] 待执行 |
| 40 | [RF-023](#rf-023) | P2 | 加密包导出用例下沉 core | [RF-015](#rf-015)、[RF-017](#rf-017) | [ ] 待执行 |
| 41 | [RF-024](#rf-024) | P2 | 加密包导入用例下沉 core | [RF-018](#rf-018)、[RF-020](#rf-020)、[RF-021](#rf-021)、[RF-022](#rf-022) | [ ] 待执行 |
| 42 | [RF-025](#rf-025) | P2 | GUI 导出移出异步运行时工作线程 | 无 | [ ] 待执行 |
| 43 | [RF-026](#rf-026) | P2 | 解密导入预览移出异步运行时工作线程 | 无 | [ ] 待执行 |
| 44 | [RF-027](#rf-027) | P2 | 高级导入移出异步运行时工作线程 | 无 | [ ] 待执行 |
| 45 | [RF-028](#rf-028) | P2 | PDF OCR 临时页面由 RAII 清理 | 无 | [ ] 待执行 |
| 46 | [RF-029](#rf-029) | P2 | OCR 增加受控排队与分页取消 | [RF-001](#rf-001)、[RF-028](#rf-028) | [ ] 待执行 |
| 47 | [RF-109](#rf-109) | P2 | 回收站保护层复用共享字段策略 | [RF-106](#rf-106) | [ ] 待执行 |
| 48 | [RF-113](#rf-113) | P2 | 常驻壳配置注册和注销具有页面所有者 | 无 | [ ] 待执行 |
| 49 | [RF-114](#rf-114) | P2 | AppRoutes 生命周期编排按职责收敛 | [RF-112](#rf-112)、[RF-113](#rf-113) | [ ] 待执行 |
| 50 | [RF-115](#rf-115) | P2 | 普通操作按钮族迁入语义样式入口 | [RF-112](#rf-112) | [ ] 待执行 |
| 51 | [RF-116](#rf-116) | P2 | 图标按钮族统一结构和平台尺寸 | [RF-115](#rf-115) | [ ] 待执行 |
| 52 | [RF-117](#rf-117) | P2 | 互斥选项与下拉选择族统一状态语义 | [RF-115](#rf-115) | [ ] 待执行 |
| 53 | [RF-118](#rf-118) | P2 | 开关控件族统一尺寸与状态 token | [RF-115](#rf-115) | [ ] 待执行 |
| 54 | [RF-119](#rf-119) | P2 | Checkbox 控件族样式归属收敛 | [RF-115](#rf-115) | [ ] 待执行 |
| 55 | [RF-120](#rf-120) | P2 | 字段值与操作按钮采用统一行布局 | [RF-107](#rf-107)、[RF-109](#rf-109)、[RF-116](#rf-116) | [ ] 待执行 |
| 56 | [RF-121](#rf-121) | P2 | 普通卡片表面使用平台无关语义 | [RF-110](#rf-110) | [ ] 待执行 |
| 57 | [RF-122](#rf-122) | P2 | 模态对话框表面迁入统一语义 | [RF-121](#rf-121)、[RF-115](#rf-115)、[RF-116](#rf-116) | [ ] 待执行 |
| 58 | [RF-123](#rf-123) | P2 | 侧栏快捷浮层表面迁入统一语义 | [RF-121](#rf-121)、[RF-102](#rf-102)、[RF-104](#rf-104) | [ ] 待执行 |
| 59 | [RF-124](#rf-124) | P2 | 迁移菜单、日期选择和 Tooltip 表面 | [RF-117](#rf-117)、[RF-121](#rf-121) | [ ] 待执行 |
| 60 | [RF-125](#rf-125) | P2 | 迁移对象详情与附件预览浮层表面 | [RF-107](#rf-107)、[RF-109](#rf-109)、[RF-121](#rf-121)、[RF-122](#rf-122) | [ ] 待执行 |
| 61 | [RF-126](#rf-126) | P2 | 迁移独立业务对话框到中立表面标记 | [RF-122](#rf-122) | [ ] 待执行 |
| 62 | [RF-127](#rf-127) | P2 | 迁移通知表面并关闭旧材质兼容清单 | [RF-115](#rf-115)、[RF-116](#rf-116)、[RF-117](#rf-117)、[RF-118](#rf-118)、[RF-119](#rf-119)、[RF-120](#rf-120)、[RF-121](#rf-121)、[RF-122](#rf-122)、[RF-123](#rf-123)、[RF-124](#rf-124)、[RF-125](#rf-125)、[RF-126](#rf-126) | [ ] 待执行 |
| 63 | [RF-205](#rf-205) | P2 | 建立并接入平台能力契约 | [RF-202](#rf-202)、[RF-203](#rf-203)、[RF-204](#rf-204) | [ ] 待执行 |
| 64 | [RF-206](#rf-206) | P2 | 提取可恢复且按版本跳过的 Android 资源安装器 | [RF-208](#rf-208) | [ ] 待执行 |
| 65 | [RF-207](#rf-207) | P2 | 将 Android 资源准备移出主线程并接入就绪屏障 | [RF-206](#rf-206)、[RF-208](#rf-208) | [ ] 待执行 |
| 66 | [RF-211](#rf-211) | P2 | 为 CLI 建立任务事件与会话失效基础 | [RF-001](#rf-001) | [ ] 待执行 |
| 67 | [RF-212](#rf-212) | P2 | 将 CLI 模型下载迁移到任务事件 | [RF-211](#rf-211) | [ ] 待执行 |
| 68 | [RF-213](#rf-213) | P2 | 将 CLI 同步迁移到任务事件 | [RF-211](#rf-211) | [ ] 待执行 |
| 69 | [RF-214](#rf-214) | P2 | 将 CLI 插件安装迁移到任务事件 | [RF-211](#rf-211) | [ ] 待执行 |
| 70 | [RF-215](#rf-215) | P2 | 将 CLI OCR 迁移到可取消后台任务 | [RF-211](#rf-211)、[RF-029](#rf-029) | [ ] 待执行 |
| 71 | [RF-301](#rf-301) | P2 | 建立 Rust 到 TypeScript 的增量 IPC 契约生成 | 无 | [ ] 待执行 |
| 72 | [RF-302](#rf-302) | P2 | 迁移对象和回滚 IPC 契约 | [RF-301](#rf-301)、[RF-008](#rf-008)、[RF-010](#rf-010) | [ ] 待执行 |
| 73 | [RF-303](#rf-303) | P2 | 迁移 LLM 会话与流事件契约 | [RF-301](#rf-301)、[RF-002](#rf-002)、[RF-004](#rf-004)、[RF-005](#rf-005)、[RF-104](#rf-104) | [ ] 待执行 |
| 74 | [RF-304](#rf-304) | P2 | 迁移备份与导入导出 IPC 契约 | [RF-301](#rf-301)、[RF-013](#rf-013)、[RF-015](#rf-015)、[RF-024](#rf-024) | [ ] 待执行 |
| 75 | [RF-305](#rf-305) | P2 | 迁移同步 IPC 与事件契约 | [RF-301](#rf-301)、[RF-003](#rf-003) | [ ] 待执行 |
| 76 | [RF-306](#rf-306) | P2 | 迁移插件 IPC 与资源事件契约 | [RF-301](#rf-301) | [ ] 待执行 |
| 77 | [RF-307](#rf-307) | P2 | 建立结构化后端错误并迁移对象用例 | [RF-301](#rf-301)、[RF-302](#rf-302) | [ ] 待执行 |
| 78 | [RF-309](#rf-309) | P2 | 建立 Windows Rust 关键用例执行门禁 | 无 | [ ] 待执行 |
| 79 | [RF-310](#rf-310) | P2 | 把 Android 原生回归接入明确的设备任务 | [RF-201](#rf-201)、[RF-208](#rf-208) | [ ] 待执行 |
| 80 | [RF-313](#rf-313) | P2 | 修正 canonical 架构与安全事实文档 | 无 | [ ] 待执行 |
| 81 | [RF-314](#rf-314) | P2 | 建立平台能力与验收证据矩阵 | [RF-205](#rf-205) | [ ] 待执行 |
| 82 | [RF-315](#rf-315) | P2 | 对齐 LLM 数据流与隐私说明 | [RF-100](#rf-100)、[RF-004](#rf-004)、[RF-005](#rf-005) | [ ] 待执行 |
| 83 | [RF-316](#rf-316) | P2 | 诊断并稳定默认前端测试运行入口 | 无 | [x] 完成 |
| 84 | [RF-308](#rf-308) | P2 | 接入有实际执行证据的覆盖率门禁 | [RF-316](#rf-316) | [ ] 待执行 |
| 85 | [RF-311](#rf-311) | P2 | 收敛重复 CI 步骤且保持平台覆盖 | [RF-308](#rf-308)、[RF-309](#rf-309)、[RF-310](#rf-310) | [ ] 待执行 |
| 86 | [RF-317](#rf-317) | P2 | 迁移 LLM 结构化错误 | [RF-303](#rf-303)、[RF-307](#rf-307) | [ ] 待执行 |
| 87 | [RF-318](#rf-318) | P2 | 迁移备份与导入导出结构化错误 | [RF-304](#rf-304)、[RF-307](#rf-307) | [ ] 待执行 |
| 88 | [RF-319](#rf-319) | P2 | 迁移同步结构化错误 | [RF-305](#rf-305)、[RF-307](#rf-307) | [ ] 待执行 |
| 89 | [RF-320](#rf-320) | P2 | 迁移插件结构化错误 | [RF-306](#rf-306)、[RF-307](#rf-307) | [ ] 待执行 |
| 90 | [RF-312](#rf-312) | P3 | 建立可重跑的性能基线与下一步决策 | 无 | [ ] 待执行 |
| 91 | [RF-900](#rf-900) | P2 | CLI 中文断言测试显式隔离系统语言 | 无（Rust 任务验收前优先处理） | [x] 完成 |
| 92 | [RF-901](#rf-901) | P2 | Windows GUI Rust 测试嵌入 Common Controls 清单 | 无（R 配置恢复前优先处理） | [x] 完成 |
| 93 | [RF-902](#rf-902) | P2 | 生产启动冒烟使用当前桌面更新契约 | 无（生产包检查恢复前优先处理） | [x] 完成 |
| 94 | [RF-905](#rf-905) | P1 | GUI 与 CLI 遵守同一数据目录互斥与维护窗口 | 无（RF-903 必需前置） | [!] 阻塞：待明确授权 |
| 95 | [RF-903](#rf-903) | P1 | 孤儿附件清理保护账户归属与完整恢复引用 | [RF-905](#rf-905) | [ ] 待执行 |

## 5. 原报告到执行任务的映射

R 编号只用于追溯，不作为混合提交单位。每个 RF ID 才是本轮独立执行/提交单位；同一 R 拆分为多个 RF 时，所有相关任务完成或有明确排除证据，才能宣布该 R 的本轮目标收敛。

| 原建议 | 范围 | 执行任务 |
| --- | --- | --- |
| R01 | 后端账户会话绑定 | [RF-001](#rf-001)、[RF-002](#rf-002)、[RF-003](#rf-003) |
| R02 | 搜索和聊天请求生命周期 | [RF-102](#rf-102)、[RF-103](#rf-103)、[RF-104](#rf-104) |
| R03 | LLM 字段出站与凭证 | [RF-004](#rf-004)、[RF-005](#rf-005)、[RF-100](#rf-100)、[RF-315](#rf-315) |
| R04 | 聊天消息重复追加 | [RF-101](#rf-101) |
| R05 | 敏感字段显示策略 | [RF-105](#rf-105)、[RF-106](#rf-106)、[RF-107](#rf-107)、[RF-108](#rf-108)、[RF-109](#rf-109) |
| R06 | GUI/CLI 对象用例 | [RF-006](#rf-006)、[RF-007](#rf-007)、[RF-008](#rf-008)、[RF-009](#rf-009)、[RF-010](#rf-010) |
| R07 | 备份兼容与完整性 | [RF-011](#rf-011)、[RF-012](#rf-012)、[RF-013](#rf-013) |
| R08 | 云快照与附件导出范围 | [RF-014](#rf-014)、[RF-015](#rf-015)、[RF-023](#rf-023) |
| R09 | 附件删除提交过程 | [RF-016](#rf-016) |
| R10 | 导出目标文件完整性 | [RF-017](#rf-017)、[RF-023](#rf-023) |
| R11 | 导入提交与恢复契约 | [RF-018](#rf-018)、[RF-020](#rf-020)、[RF-021](#rf-021)、[RF-022](#rf-022)、[RF-024](#rf-024) |
| R12 | 主题解析、保存与协调 | [RF-110](#rf-110)、[RF-111](#rf-111)、[RF-112](#rf-112)、[RF-201](#rf-201) |
| R13 | 平台能力 | [RF-202](#rf-202)、[RF-203](#rf-203)、[RF-204](#rf-204)、[RF-205](#rf-205) |
| R14 | 数据库事务回滚 | [RF-019](#rf-019) |
| R15 | 长任务与资源生命周期 | [RF-025](#rf-025)、[RF-026](#rf-026)、[RF-027](#rf-027)、[RF-028](#rf-028)、[RF-029](#rf-029)、[RF-211](#rf-211)、[RF-212](#rf-212)、[RF-213](#rf-213)、[RF-214](#rf-214)、[RF-215](#rf-215) |
| R16 | 控件与材质设计系统 | [RF-115](#rf-115)、[RF-116](#rf-116)、[RF-117](#rf-117)、[RF-118](#rf-118)、[RF-119](#rf-119)、[RF-120](#rf-120)、[RF-121](#rf-121)、[RF-122](#rf-122)、[RF-123](#rf-123)、[RF-124](#rf-124)、[RF-125](#rf-125)、[RF-126](#rf-126)、[RF-127](#rf-127) |
| R17 | 常驻壳与应用生命周期 | [RF-113](#rf-113)、[RF-114](#rf-114) |
| R18 | IPC 类型与结构化错误 | [RF-205](#rf-205)、[RF-301](#rf-301)、[RF-302](#rf-302)、[RF-303](#rf-303)、[RF-304](#rf-304)、[RF-305](#rf-305)、[RF-306](#rf-306)、[RF-307](#rf-307)、[RF-317](#rf-317)、[RF-318](#rf-318)、[RF-319](#rf-319)、[RF-320](#rf-320) |
| R19 | Android 资源准备 | [RF-206](#rf-206)、[RF-207](#rf-207)、[RF-208](#rf-208) |
| R20 | 性能测量 | [RF-312](#rf-312) |
| R21 | 验证设施与 CI | [RF-208](#rf-208)、[RF-308](#rf-308)、[RF-309](#rf-309)、[RF-310](#rf-310)、[RF-311](#rf-311)、[RF-314](#rf-314)、[RF-316](#rf-316) |
| R22 | 架构与隐私文档 | [RF-313](#rf-313)、[RF-314](#rf-314)、[RF-315](#rf-315) |

R18 本轮明确迁移对象/快照、LLM、备份/导入导出、同步和插件领域。其余命令保留兼容入口，并在 RF-301 的登记清单中逐项标明“未迁移”；不能把试点或这些领域的完成宣称为全部 219 个命令已迁移。扩展到其余命令时按领域新增 RF-900 系列任务。

## 6. 可执行任务卡

“入口”是实施前应读的源码及建议新增位置，并不要求修改列出的每个文件。涉及安全/数据格式的行为修复需先建立失败路径回归；结构抽取保留旧入口适配，迁移完当前领域再移除死代码。任务中的公共名（如 SessionContext、ThemeController）表示责任边界，最终命名可以遵循项目现有风格。

## 6.1 Rust 核心与数据提交

### RF-001

**建立最小会话捕获与提交校验机制** · P1 · 来源：R01

- **前置：**无。
- **入口：**`tauri/crates/solosoul-core/src/vault_service/mod.rs`；`tauri/crates/solosoul-core/src/vault_service/session.rs`（本项新增）；`tauri/crates/solosoul-core/src/vault_service/unlock.rs`；`tauri/crates/solosoul-core/src/vault_service/account.rs`；`tauri/crates/solosoul-core/src/vault_service/tests.rs`。
- **执行：**增加包含账户、代次和原始 Vault 句柄的会话令牌，一次性捕获并拒绝账户不匹配。锁定、会话替换使旧令牌永久失效；提供短临界区内校验并提交的 API，明确锁顺序。仅服务已确认的 LLM/云同步竞态，不引入通用 JobRunner，不在网络、KDF、压缩或推理期间持有提交门闩。
- **验收：**同账户重新解锁也使旧令牌失效；失败解锁不复活旧代次；校验与写入之间插入锁定屏障时，提交只能在锁定前完整完成或明确拒绝；锁定不等待网络。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**新增 vault_service/tests.rs::rf001_*；在 tauri/ 运行 cargo test -p solosoul-core --lib rf001_ -- --test-threads=1。
- **建议提交：**`fix: resolve [RF-001] - add generation-bound vault session guards`。
- **2026-09-25 执行阻塞：**尚未修改本项代码。R 全量基线的 GUI 测试程序在执行用例前返回 `0xc0000139 / STATUS_ENTRYPOINT_NOT_FOUND`；需先定位入口点/运行依赖问题并恢复 R 验证能力，不能用 CLI 或 cargo check 通过替代。CLI 的语言和 sqlite3 前置已由 RF-900/环境配置解决。
- **阻塞解除：**RF-901 已修复 Windows MSVC 测试产物缺少 Common Controls v6 清单的问题，正式 R 全量 1,053 passed / 0 failed；本项恢复待执行，原阻塞记录保留作历史证据。

### RF-002

**LLM 流式回复绑定请求开始时的会话** · P1 · 来源：R01

- **前置：**[RF-001](#rf-001)。
- **入口：**`tauri/src-tauri/src/commands/llm/stream.rs`；`tauri/src-tauri/src/commands/llm/stats.rs`；`tauri/crates/solosoul-vault/src/storage/conversations.rs`；`tauri/crates/solosoul-core/src/llm/service.rs`（测试夹具账户一致性）。
- **执行：**llm_send_message_stream 开始时捕获会话；handle_sse_stream 事件、persist_conversation_reply 和 record_and_persist_usage 使用原会话，禁止完成后重取当前 Vault。提交使用会话门闩；普通会话写入口检查账户与 Vault 对应关系。保留 provider 登记及网络地址检查。 流事件携带 accountId、sessionGeneration、conversationId、requestId，RF-104 据此隔离前端；此项新增字段保留现有事件名称与旧调用兼容，后续再迁移消费者。
- **验收：**暂停 A 的流请求，锁定并登录 B 后放行：B 会话和统计不变，旧请求不再发有效业务事件；正常流只持久化一次；普通会话写入拒绝错误账户。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**在 stream.rs 和 storage/conversations.rs 新增 rf002_* 屏障测试；cargo test -p solo_soul --lib rf002_；cargo test -p solosoul-vault --lib rf002_。使用模拟 HTTP 流及临时 Vault。
- **建议提交：**`fix: resolve [RF-002] - bind LLM completion to its original session`。

### RF-003

**云同步一轮操作固定账户与会话** · P1 · 来源：R01

- **前置：**[RF-001](#rf-001)、[RF-020](#rf-020)。
- **入口：**`tauri/src-tauri/src/sync/cloud_auto_sync.rs`。
- **执行：**CloudPreContext 捕获会话；run_cloud_sync_round、export_full_snapshot、auto_import_one 的导出、导入、状态和应用水线均使用原会话。移除网络等待后重取当前 Vault 的写入；失效或部分失败不推进水线，不删除未完成的待导入包。
- **验收：**在下载结束、导入前、导入后水线提交前分别插入屏障，切换账户后新账户数据库与水线均不变；原账户重新进入后允许重试；完整成功才删除待导入源。
- **验证配置：**`R` + `F` + `CLI` + `CONTRACT`（共享附件提交、会话密钥入口及手动水线 IPC 改变）。**定向验证：**cloud_auto_sync/tests/rf003.rs 与 settings.rs 新增 rf003_*，模拟 connector 和阶段屏障；cargo test -p solo_soul --lib rf003_ -- --test-threads=1；复跑 RF-020 与现有导入导出回归；前端 useImportState.test.ts 覆盖迟到结果与跨账户事件，复跑恢复与 CloudSyncPage 冒烟。
- **建议提交：**`fix: resolve [RF-003] - bind cloud sync rounds to their source session`。
- **2026-09-25 依赖复核：**auto_import_one 只按 import_execute_internal 的 Ok/Err 判成功；当前 resolve_template_id 忽略保存错误，偏好和快照路径也存在 best-effort，无法证明“完整成功才推进水线”。该结果契约已归 RF-020（依赖 RF-018），先完成既有任务，避免在 RF-003 复制一套导入实现或提前宣称部分失败已被识别。导出/导入的固定句柄、事件、水线与文件删除仍全部由本项验收，未缩小范围。

### RF-004

**Rust 生成普通聊天的受控自动上下文** · P1 · 来源：R03

- **前置：**[RF-001](#rf-001)、[RF-100](#rf-100)、[RF-101](#rf-101)。
- **入口：**`tauri/crates/solosoul-core/src/llm/context.rs（拟新增）`；`tauri/crates/solosoul-core/src/llm/mod.rs`；`tauri/src-tauri/src/commands/llm/stream.rs`；`tauri/src-tauri/src/services/llm_context.rs`；`tauri/src/lib/llm/chatRequest.ts`；`tauri/src/lib/llm/systemPromptBuilder.ts`；`tauri/src/hooks/useLlmChatCore.ts`；`tauri/src/lib/llm/chatRequest.test.ts（拟新增）`。
- **执行：**实现 LlmContextProjection，统一解析标签副本、__fields、已删除模板和动态组，排除内部键，未知敏感度不自动附加。普通发送传上下文选择标识，由 Rust 在绑定会话中读取并追加受控上下文；区分用户输入、用户提示词与自动附加数据。RF-100 先提供前端止血，不等待本次迁移。
- **验收：**捕获模拟 provider 最终请求：public 对象内所有非 public/未知字段、内部元数据及嵌套敏感值均不外发；关闭自动上下文不读取附加对象；用户主动输入不被误当作自动字段处理。
- **验证配置：**`R` + `CORE` + `F` + `CONTRACT` + `CLI`。**定向验证：**新增 core/llm/context.rs::rf004_* 和 Host 出站请求测试；cargo test -p solosoul-core --lib rf004_；cargo test -p solo_soul --lib rf004_；定向 Vitest src/lib/llm/chatRequest.test.ts。
- **建议提交：**`fix: resolve [RF-004] - build permitted LLM context in Rust`。

### RF-005

**普通聊天通过 provider ID 在 Rust 解析凭证** · P1 · 来源：R03

- **前置：**[RF-001](#rf-001)、[RF-002](#rf-002)、[RF-004](#rf-004)。
- **入口：**`tauri/src-tauri/src/commands/llm/stream.rs`；`tauri/src-tauri/src/commands/llm/provider.rs`；`tauri/src-tauri/src/commands/llm/unified_chat.rs`；`tauri/crates/solosoul-core/src/llm/service.rs`；`tauri/src/hooks/useLlmChatCore.ts`；`tauri/src/hooks/useLlmChatCore.test.tsx（拟新增）`。
- **执行：**普通发送参数改为 provider ID，Rust 从绑定会话解析保存的地址、模型和凭证；移除普通发送中的 llm_get_api_key 往返。保留历史内置 provider ID 映射及网络校验；设置页明确使用的凭证查看/测试入口不在本项顺带删除。与 RF-004 的文件重叠按顺序处理。
- **验收：**普通发送 IPC 不携带 API key；未知、禁用或其他账户 provider 被拒绝；切换账户不能沿用旧凭证；历史内置 provider ID 可用；模拟传输收到正确测试认证头。
- **验证配置：**`R` + `CORE` + `F` + `CONTRACT` + `CLI`。**定向验证：**Host/provider 测试新增 rf005_*；cargo test -p solo_soul --lib rf005_；定向 Vitest src/hooks/useLlmChatCore.test.tsx，仅使用测试密钥和模拟传输。
- **建议提交：**`fix: resolve [RF-005] - resolve chat provider credentials in Rust`。

### RF-006

**GUI 回滚拒绝其他对象的快照** · P1 · 来源：R06

- **前置：**无。
- **入口：**`tauri/src-tauri/src/commands/object/snapshot.rs`；`tauri/src-tauri/src/commands/object/tests/snapshot.rs`；`tauri/crates/solosoul-vault/src/storage/snapshots.rs`。
- **执行：**snapshot_rollback 应用数据前校验快照归属；必要时增加按 object_id 与 snapshot_id 联合读取的方法。无归属、目标不存在和归属不符均在写入前退出；不迁移整个回滚用例。
- **验收：**A 的快照不能应用到 B；拒绝后对象、版本、历史与审计均不新增；正常同对象回滚保持行为。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**commands/object/tests/snapshot.rs 新增 rf006_*；cargo test -p solo_soul --lib rf006_；新增存储联合读取时运行 cargo test -p solosoul-vault --lib rf006_。
- **建议提交：**`fix: resolve [RF-006] - validate snapshot ownership before GUI rollback`。

### RF-007

**CLI 回滚恢复并保留字段标签** · P1 · 来源：R06

- **前置：**无。
- **入口：**`solosoul_cli/src/commands/history.rs`。
- **执行：**do_rollback 恢复 propertyLabels 并兼容 property_labels；生成的回滚快照也携带标签。明确字段缺失、显式 null 和历史格式处理，避免无意清空标签；保持现有归属校验及交互。
- **验收：**两种标签键名均能恢复；模板删除后字段语义副本保留；连续回滚不丢标签；跨对象拒绝测试继续通过。
- **验证配置：**`CLI`。**定向验证：**history.rs 现有测试模块新增 rf007_*；在 solosoul_cli/ 运行 cargo test --lib rf007_ 和 cargo test --lib rollback。
- **建议提交：**`fix: resolve [RF-007] - preserve field labels during CLI rollback`。

### RF-008

**GUI 与 CLI 迁移到同一回滚用例** · P2 · 来源：R06

- **前置：**[RF-006](#rf-006)、[RF-007](#rf-007)。
- **入口：**`tauri/crates/solosoul-core/src/objects.rs`；`tauri/src-tauri/src/commands/object/snapshot.rs`；`tauri/src-tauri/src/commands/object/tests/snapshot.rs`；`solosoul_cli/src/commands/history.rs`。
- **执行：**在 core 新增 rollback_object 用例，收敛归属校验、字段恢复、版本更新、快照构建及审计结果；GUI/CLI 只适配输入、通知和本地化。返回明确操作结果，保留既有审计失败可见行为，不顺带改对象 CRUD。
- **验收：**相同合成输入经两端得到等价对象及快照；RF-006/007 全部通过；错误阶段与已提交状态可识别；原有交互与同步通知不丢失。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**core/objects.rs 新增 rf008_*；cargo test -p solosoul-core --lib rf008_；Host rf006_；CLI rollback 测试。
- **建议提交：**`refactor: resolve [RF-008] - share the object rollback use case`。

### RF-009

**修复 CLI 创建对象缺失模板元数据** · P1 · 来源：R06

- **前置：**无。
- **入口：**`tauri/crates/solosoul-core/src/objects.rs`；`solosoul_cli/src/commands/vault_write.rs`。
- **执行：**在 core::objects::create_object 补齐模板字段定义、敏感度标签副本、契约 ID 与模板指纹，复用 template_fingerprint；以 GUI build_create_record/inherit_template_properties 为规则对照。保留 CLI 页面归属、交互及用户输入；本项不迁移 GUI 创建入口。
- **验收：**相同模板和输入下 GUI/CLI 的字段语义、标签及指纹一致；无模板路径兼容；删除模板后仍保留必要字段语义；不覆盖用户已填值。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**core/objects.rs 新增 rf009_*；cargo test -p solosoul-core --lib rf009_；CLI commands::vault_write 创建路径定向回归。
- **建议提交：**`fix: resolve [RF-009] - inherit template metadata in CLI object creation`。

### RF-010

**共享对象创建的模板初始化规则** · P2 · 来源：R06

- **前置：**[RF-009](#rf-009)。
- **入口：**`tauri/crates/solosoul-core/src/objects.rs`；`tauri/src-tauri/src/commands/object/mod.rs`；`tauri/src-tauri/src/commands/object/tests/crud.rs`；`tauri/src-tauri/src/commands/object/tests/template_sync.rs`。
- **执行：**将 build_create_record、inherit_template_properties、inherit_contract_type_id 的共同模板初始化与记录构造规则下沉 core，GUI 与 CLI 调用同一规则。保留 GUI 客户端指定 ID、CLI 默认页面等显式输入差异；不扩大调整父页面更新或通知。
- **验收：**RF-009 等价性 fixture 不变；GUI 乐观创建 ID、CLI 无模板创建、模板指纹及用户值保持兼容；被替代的平行初始化规则删除。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**core 新增 rf010_*；cargo test -p solosoul-core --lib rf010_；Host commands::object::tests；CLI 创建定向测试。
- **建议提交：**`refactor: resolve [RF-010] - share object template initialization`。

### RF-011

**CLI 恢复兼容 GUI Base64 Profile 备份** · P1 · 来源：R07

- **前置：**无。
- **入口：**`solosoul_cli/src/commands/backup.rs`。
- **执行：**RestoreProfileEntry/do_restore 兼容 data_b64 和旧 data 数组，明确双字段优先级及空数据处理。非法 Base64 报错，不退化成空 Profile；尽可能在写入前验证解码。本项不更改备份范围或格式版本。
- **验收：**含 Profile 的 GUI 2.0 fixture 可恢复；CLI 旧数组格式可读；非法编码不写入伪造空内容；空 profiles 清单仍合法。
- **验证配置：**`CLI`。**定向验证：**backup.rs 测试模块新增 rf011_*；在 solosoul_cli/ 运行 cargo test --lib rf011_ 和 cargo test --lib commands::backup。
- **建议提交：**`fix: resolve [RF-011] - restore GUI Base64 backups in CLI`。

### RF-012

**GUI 备份遇到 Profile 读取失败时中止** · P1 · 来源：R07

- **前置：**无。
- **入口：**`tauri/src-tauri/src/commands/backup.rs`。
- **执行：**backup_create 替换静默跳过读取失败的分支，区分读取错误与枚举后条目消失；清单数量来自完整收集结果；收集失败不发布成功备份或报告成功数量。本项不迁移共享 codec。
- **验收：**任意 Profile 读取失败使命令失败；已有有效备份不被不完整结果替代；成功清单数量与内容一致。
- **验证配置：**`R`。**定向验证：**backup.rs 新增 rf012_*；参考 CLI test_backup_create_aborts_on_unreadable_profile；cargo test -p solo_soul --lib rf012_。
- **建议提交：**`fix: resolve [RF-012] - fail GUI backup on unreadable profiles`。

### RF-013

**共享 Profile 备份清单与兼容解码** · P2 · 来源：R07

- **前置：**[RF-011](#rf-011)、[RF-012](#rf-012)。
- **入口：**`tauri/crates/solosoul-core/src/backup.rs（拟新增）`；`tauri/crates/solosoul-core/src/lib.rs`；`tauri/src-tauri/src/commands/backup.rs`；`solosoul_cli/src/commands/backup.rs`。
- **执行：**迁移共同 manifest/entry、版本检查、Base64/数组兼容和完整性验证到 core codec；两端调用同一解码规则。文件选择、确认与现有写出格式兼容策略留在适配器，不将 Profile 备份扩为完整 Vault 备份。
- **验收：**GUI→CLI、CLI→GUI、两种旧格式、损坏条目和非法版本共享 fixture 通过；相同输入两端解码结果一致；既有合法备份仍可读取。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**core/backup.rs 新增 rf013_*；cargo test -p solosoul-core --lib rf013_；GUI 与 CLI commands::backup 测试。
- **建议提交：**`refactor: resolve [RF-013] - share profile backup encoding contracts`。

### RF-014

**全量云快照包含全部有效附件** · P1 · 来源：R08

- **前置：**无。
- **入口：**`tauri/src-tauri/src/sync/cloud_auto_sync.rs`；`tauri/src-tauri/src/commands/recovery.rs`；`tauri/src-tauri/src/commands/export_import/export.rs`。
- **执行：**export_full_snapshot 显式提供全部未删除附件 ID；需要复用时只抽出与 recovery::collect_all_attachment_ids 等价的小助手。不把手动空选择集解释为全选，不等待 ExportPlan 重构；保留 RF-003 会话绑定。
- **验收：**云快照 ZIP 含附件字节，能在空测试 Vault 中恢复解密；已删除附件不重新带入；手动全不选仍为零附件。
- **验证配置：**`R` + `CORE`。**定向验证：**cloud_auto_sync/export tests 新增 rf014_*；cargo test -p solo_soul --lib rf014_。使用临时 Vault 与本地包往返，不接真实云端。
- **建议提交：**`fix: resolve [RF-014] - include attachments in full cloud snapshots`。

### RF-015

**显式表示附件导出范围** · P2 · 来源：R08

- **前置：**[RF-014](#rf-014)。
- **入口：**`tauri/src-tauri/src/commands/export_import/mod.rs`；`tauri/src-tauri/src/commands/export_import/export.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`；`tauri/crates/solosoul-core/src/export_import.rs`；`tauri/src-tauri/src/sync/cloud_auto_sync.rs`；`tauri/src-tauri/src/commands/recovery.rs`。
- **执行：**内部附件范围统一为 None/All/Selected(ids)；既有 IPC 布尔值和数组经适配器映射，手动空数组仍为零选中；云同步与恢复显式选择 All；收集逻辑只解释新类型。
- **验收：**三种范围参数化测试通过；手动全不选、部分选择、全量云同步与恢复均正确；存量 IPC 载荷继续可读。
- **验证配置：**`R` + `CORE` + `CONTRACT` + `CLI`。**定向验证：**Host/core 导出范围新增 rf015_*；cargo test -p solo_soul --lib rf015_；cargo test -p solosoul-core --lib rf015_；复跑 rf014_。
- **建议提交：**`refactor: resolve [RF-015] - model attachment export scope explicitly`。

### RF-016

**永久删除附件采用可恢复清理意图** · P1 · 来源：R09

- **前置：**[RF-019](#rf-019)。
- **入口：**`tauri/src-tauri/src/commands/attachment/crud.rs`；`tauri/src-tauri/src/commands/attachment/tests.rs`；`tauri/crates/solosoul-core/src/objects.rs`；`tauri/crates/solosoul-vault/src/storage.rs`；`tauri/crates/solosoul-vault/src/storage/attachment_cleanup.rs（拟新增）`。
- **执行：**在同一数据库事务提交附件元数据删除与清理意图，再删除实体文件；NotFound 可视为完成，其他错误保留重试记录。单删、批删和 core::objects::purge_attachment 复用执行器；在已解锁维护入口恢复未完成清理，保留路径边界校验。
- **验收：**数据库失败时实体文件仍存在；文件删除失败时意图可追踪可重试；事务提交前后中断均可恢复；GUI 单删/批删和 CLI 使用同一规则。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**新增 rf016_* 临时目录故障注入；cargo test -p solosoul-vault --lib rf016_；cargo test -p solosoul-core --lib rf016_；cargo test -p solo_soul --lib rf016_；CLI 附件删除回归。
- **建议提交：**`fix: resolve [RF-016] - make attachment deletion recoverable`。

### RF-017

**导出完成后才替换目标包** · P1 · 来源：R10

- **前置：**无。
- **入口：**`tauri/src-tauri/src/commands/export_import/export.rs`；`tauri/src-tauri/src/commands/export_import/helpers.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`。
- **执行：**execute_export_core 在最终目标同目录创建临时输出，ZIP 完成、刷新及必要校验后才平台适配替换；错误由 RAII 清理临时输出；复用流式加密，避免完整包内存副本。移动端本项覆盖本地 staging，不把后续 SAF 复制宣称为同卷原子替换。
- **验收：**超限、附件读取失败、写入失败及 ZIP 收尾失败均保持原目标字节不变；成功才替换；Windows 已有目标场景通过；临时输出不残留。
- **验证配置：**`R` + `NATIVE`。**定向验证：**新增 rf017_* 可失败 Writer/文件操作测试；cargo test -p solo_soul --lib rf017_；Windows 已有文件替换定向验证，不靠填满真实磁盘制造失败。
- **建议提交：**`fix: resolve [RF-017] - preserve existing exports on write failure`。

### RF-018

**导入模板保存失败必须传播** · P1 · 来源：R11

- **前置：**无。
- **入口：**`tauri/src-tauri/src/commands/export_import/import.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`；`tauri/src-tauri/src/commands/export_import/tests/rf018.rs`；`tauri/src-tauri/Cargo.toml`（测试依赖）。
- **执行：**resolve_template_id 的查询错误不再伪装不存在，保存错误不再返回成功模板 ID；rebuild_imported_templates 只登记已存在或成功保存的映射。本项不改整体导入事务。
- **验收：**原始 ID 和派生 ID 两个保存分支失败均报错；失败模板不进入映射，不产生引用该模板的新对象；按内容哈希复用保持行为。
- **验证配置：**`R`。**定向验证：**export_import/tests.rs 新增 rf018_*；cargo test -p solo_soul --lib rf018_。
- **建议提交：**`fix: resolve [RF-018] - propagate imported template storage errors`。

### RF-019

**数据库事务失败时自动回滚** · P2 · 来源：R14

- **前置：**无。
- **入口：**`tauri/crates/solosoul-vault/src/storage.rs`；`tauri/crates/solosoul-vault/src/storage/tests.rs`；`tauri/crates/solosoul-vault/src/storage/objects.rs`；`tauri/crates/solosoul-vault/src/storage/profile.rs`；`tauri/crates/solosoul-vault/src/storage/snapshots.rs`；`tauri/crates/solosoul-vault/src/storage/conversations.rs`；`tauri/crates/solosoul-vault/src/storage/metadata.rs`；`tauri/crates/solosoul-vault/src/storage/trash.rs`；`tauri/crates/solosoul-vault/src/storage/sync_apply.rs`；`tauri/crates/solosoul-vault/src/storage/sync_meta.rs`。
- **执行：**with_tx 核对当前 rusqlite API，将无需可变连接的事务 helper 收敛为 &Connection；使用 RAII Transaction，特殊路径才使用有 Drop 回滚的 guard；COMMIT 失败仍收尾；修正 prepare_cached 注释。只调整真实受影响的 helper，不全目录格式化。
- **验收：**回调 Err、保留活动事务的 COMMIT 失败和 unwind 后没有残留活动事务；下一次事务能执行；测试区分事务回滚与外围 Mutex 中毒，不把二者混为一项。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**storage/tests.rs 新增 rf019_*；cargo test -p solosoul-vault --lib rf019_；随后运行受影响对象、会话、同步存储测试。
- **建议提交：**`fix: resolve [RF-019] - restore automatic transaction rollback`。

### RF-020

**导入失败返回真实部分提交状态** · P1 · 来源：R11

- **前置：**[RF-018](#rf-018)。
- **入口：**`tauri/src-tauri/src/commands/export_import/mod.rs`；`tauri/src-tauri/src/commands/export_import/import.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`；`tauri/src-tauri/src/sync/cloud_auto_sync.rs`；`tauri/src-tauri/src/commands/recovery.rs`；`tauri/src/hooks/useImportState.ts`；`tauri/src/hooks/useImportState.test.ts（拟新增）`；`tauri/src/lib/ipc.ts`；`tauri/src/pages/settings/cloudSync/useCloudSyncPage.ts`。
- **执行：**ImportResult/import_execute_internal 累计真实已提交对象、附件及失败阶段，区分完成/部分完成/未提交；IPC 和界面准确展示；云同步部分完成不推进水线、不删除待导入源；恢复调用者不误报完成；错误明细不包含敏感值。
- **验收：**第 N 个对象或附件失败时，报告数量与数据库相符；未写入失败和部分写入失败可区分；成功路径兼容；云水线只在完整成功时推进。
- **验证配置：**`R` + `F` + `CONTRACT` + `CLI`（共享附件计数实现改变）。**定向验证：**Host 新增 rf020_*；cargo test -p solo_soul --lib rf020_；定向 Vitest src/hooks/useImportState.test.ts 与 src/hooks/useRecoveryReceive.test.tsx，覆盖普通导入、云页面和恢复结果适配。实际范围还含共享 core export_import.rs、结果类型 exportImport.ts / recoveryReceiveTypes.ts、importOutcome.ts 和双语提示；不要求修改无调用封装的 ipc.ts。
- **建议提交：**`fix: resolve [RF-020] - report partial import outcomes accurately`。

### RF-021

**对象模板与历史按导入批次事务提交** · P1 · 来源：R11

- **前置：**[RF-018](#rf-018)、[RF-019](#rf-019)、[RF-020](#rf-020)。
- **入口：**`tauri/src-tauri/src/commands/export_import/import.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`；`tauri/crates/solosoul-vault/src/storage/objects.rs`；`tauri/crates/solosoul-vault/src/storage/snapshots.rs`；`tauri/crates/solosoul-vault/src/storage/metadata.rs`；`tauri/crates/solosoul-vault/src/storage/tests.rs`。
- **执行：**在进入事务前解析验证模板映射、对象及历史操作；用一个存储事务提交对象、相关模板、历史替换与 HLC。事务内不做 KDF、ZIP 解密或附件 I/O；附件阶段仍经 RF-020 报告独立结果。
- **验收：**任一对象、模板或历史写入失败使该数据库批次全部回滚；Overwrite 不先丢旧历史；KeepBoth 引用重写不变；HLC 与对应记录共同提交。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**新增 rf021_* 第 N 条失败测试；cargo test -p solosoul-vault --lib rf021_；cargo test -p solo_soul --lib rf021_；覆盖三个导入策略。
- **建议提交：**`fix: resolve [RF-021] - commit imported records and history atomically`。

### RF-022

**附件导入可恢复且同一任务重试幂等** · P1 · 来源：R11

- **前置：**[RF-020](#rf-020)、[RF-021](#rf-021)。
- **入口：**`tauri/crates/solosoul-core/src/export_import.rs`；`tauri/src-tauri/src/commands/export_import/import.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`；`tauri/crates/solosoul-vault/src/storage.rs`；`tauri/crates/solosoul-vault/src/storage/import_operations.rs（拟新增）`。
- **执行：**为一次导入记录稳定操作 ID、源包标识及 KeepBoth ID 映射；附件先写加密 staging，再按可恢复阶段发布并提交元数据；同一任务重试复用记录，不持久化密码或会话密钥。独立新导入仍允许 KeepBoth 生成副本。
- **验收：**附件写入、发布及元数据提交间中断均能恢复；同一操作重试不重复创建对象/附件；失败不不可恢复地覆盖已有附件；重新解锁后需要时重新索取包密码。云端 skipExisting 重试已部分写入的对象时，必须继续补全缺失附件，不能因对象已存在而跳过后误报完整成功、推进水线或清理源包（RF-020 复核补充）。
- **验证配置：**`R` + `CORE` + `F` + `CONTRACT` + `CLI`。**定向验证：**core/Host 新增 rf022_* 稳定操作 ID 与阶段注入测试；cargo test -p solosoul-core --lib rf022_；cargo test -p solo_soul --lib rf022_；按实际重试交互补 useImportState 定向测试。
- **建议提交：**`fix: resolve [RF-022] - resume attachment imports without duplicate records`。

### RF-023

**加密包导出用例下沉 core** · P2 · 来源：R08、R10

- **前置：**[RF-015](#rf-015)、[RF-017](#rf-017)。
- **入口：**`tauri/src-tauri/src/commands/export_import/export.rs`；`tauri/src-tauri/src/commands/export_import/mod.rs`；`tauri/crates/solosoul-core/src/export_import.rs`；`tauri/src-tauri/src/sync/cloud_auto_sync.rs`；`tauri/src-tauri/src/commands/recovery.rs`；`solosoul_cli/src/commands/export_import.rs`。
- **执行：**抽出不依赖 Tauri/AppState/前端错误文案的导出计划与执行服务；保留 GUI/CLI scope 适配及包格式；GUI、云同步、恢复经薄入口转调，移除同步对 Commands 业务实现的反向依赖；CLI 旧公开接口保留兼容适配。只迁加密包导出，不迁文档导出。
- **验收：**相同 fixture 经兼容入口和 core 得到语义等价包；密码/KDF、模板、历史、偏好和附件范围保持契约；RF-014/017 继续通过；不引入 crate 循环。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**core 新增 rf023_*；cargo test -p solosoul-core --lib rf023_；Host commands::export_import::tests、恢复和云快照回归；CLI commands::export_import。
- **建议提交：**`refactor: resolve [RF-023] - move encrypted export execution into core`。

### RF-024

**加密包导入用例下沉 core** · P2 · 来源：R11

- **前置：**[RF-018](#rf-018)、[RF-020](#rf-020)、[RF-021](#rf-021)、[RF-022](#rf-022)。
- **入口：**`tauri/src-tauri/src/commands/export_import/import.rs`；`tauri/src-tauri/src/commands/export_import/mod.rs`；`tauri/crates/solosoul-core/src/export_import.rs`；`tauri/src-tauri/src/sync/cloud_auto_sync.rs`；`tauri/src-tauri/src/commands/recovery.rs`；`solosoul_cli/src/commands/export_import.rs`。
- **执行：**将验证、计划、提交、恢复和结果语义收敛为 core 服务；输入使用会话/存储上下文与普通 DTO，不要求宿主 RwLockReadGuard；Host 仅做路径授权、IPC和进度事件。保留 GUI 选择性导入、CLI 默认策略的适配参数，不以较窄 CLI 接口替换高级导入。
- **验收：**GUI、CLI、云同步和恢复使用同一提交规则；三策略、选择性附件、历史恢复、部分完成及重试 fixture 通过；既有包格式不变。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**core 新增 rf024_*；cargo test -p solosoul-core --lib rf024_；Host commands::export_import::tests；CLI commands::export_import；复跑 rf020_/rf021_/rf022_。
- **建议提交：**`refactor: resolve [RF-024] - share encrypted import execution`。

### RF-025

**GUI 导出移出异步运行时工作线程** · P2 · 来源：R15

- **前置：**无。
- **入口：**`tauri/src-tauri/src/commands/export_import/export.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`。
- **执行：**export_execute 用 spawn_blocking 执行同步导出，移动 owned 参数，必要锁在闭包内获取释放；保留路径授权与错误语义；显式处理 JoinError。无需等待 RF-023；若共享服务已落地则调度该服务。
- **验收：**屏障暂停模拟导出时，独立轻量异步任务仍推进；成功和失败结果不变；不持同步锁跨 await；任务 Join 失败不误报完成。
- **验证配置：**`R`。**定向验证：**新增 rf025_* 屏障测试；cargo test -p solo_soul --lib rf025_；RF-017 已完成时复跑，不用脆弱 sleep 阈值断言。
- **建议提交：**`fix: resolve [RF-025] - run GUI export on the blocking pool`。

### RF-026

**解密导入预览移出异步运行时工作线程** · P2 · 来源：R15

- **前置：**无。
- **入口：**`tauri/src-tauri/src/commands/export_import/import.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`。
- **执行：**import_decrypt_preview 在路径授权后将 KDF、解密、解析及同步预览读取放入阻塞闭包；密码沿 Zeroizing 路径转移；只跨 await 返回普通 DTO，不携带同步锁守卫。
- **验收：**慢解密期间轻量任务仍推进；错误密码、损坏包与正常冲突预览行为保持；Join 失败不返回空预览作为成功。
- **验证配置：**`R`。**定向验证：**新增 rf026_* 屏障及预览 fixture；cargo test -p solo_soul --lib rf026_；运行现有导入预览测试。
- **建议提交：**`fix: resolve [RF-026] - offload decrypted import preview`。

### RF-027

**高级导入移出异步运行时工作线程** · P2 · 来源：R15

- **前置：**无。
- **入口：**`tauri/src-tauri/src/commands/export_import/import.rs`；`tauri/src-tauri/src/commands/export_import/tests.rs`。
- **执行：**import_execute_advanced 以 owned 请求与状态句柄调度同步执行，闭包内取得上下文；外层按实际提交结果触发同步与通知。若 RF-020 已落地须保留部分完成契约，不能用调度抽取改变导入策略。
- **验收：**慢导入期间轻量异步任务推进；完整成功、未提交失败和部分完成的后续行为正确；JoinError 不误报完成；不持同步锁跨 await。
- **验证配置：**`R`。**定向验证：**新增 rf027_* 屏障测试；cargo test -p solo_soul --lib rf027_；RF-020/021 已完成时复跑对应回归。
- **建议提交：**`fix: resolve [RF-027] - offload advanced import execution`。

### RF-028

**PDF OCR 临时页面由 RAII 清理** · P2 · 来源：R15

- **前置：**无。
- **入口：**`tauri/crates/solosoul-core/src/ocr/engine.rs`；`tauri/crates/solosoul-core/src/ocr/pdf.rs`。
- **执行：**scan_pdf 用受控 TempDir 或等价所有权对象替代手工创建和成功尾部清理；渲染失败、某页推理失败和提前返回均由所有者清理；保留文本层优先路径。
- **验收：**渲染部分页面后失败、扫描中途失败及正常完成均不残留页面；文本层直接返回不创建多余目录；测试不要求真实模型。
- **验证配置：**`R` + `CORE` + `CLI`。**定向验证：**OCR 模块新增 rf028_*，用可注入渲染/识别实现；cargo test -p solosoul-core --lib rf028_；既有 OCR fixture 定向回归。
- **建议提交：**`fix: resolve [RF-028] - clean OCR page files on every exit path`。

### RF-029

**OCR 增加受控排队与分页取消** · P2 · 来源：R15

- **前置：**[RF-001](#rf-001)、[RF-028](#rf-028)。
- **入口：**`tauri/src-tauri/src/commands/ocr.rs`；`tauri/src-tauri/src/services/ocr_jobs.rs（拟新增）`；`tauri/src-tauri/src/services/mod.rs`；`tauri/crates/solosoul-core/src/ocr/engine.rs`；`tauri/src/pages/scan/OcrPage.tsx`；`tauri/src-tauri/src/lib.rs`。
- **执行：**仅为 OCR 建立有限队列、任务 ID、会话归属与取消令牌；获取推理资源前和 PDF 各页之间检查取消；事件包含任务身份，锁定后丢弃失效结果；UI 区分请求取消与实际结束。单次不可中断 ONNX 推理允许结束后停止，不承诺立即终止；同步登记必要 IPC/ACL。
- **验收：**取消排队任务不进入推理；分页取消后不启动下一页；当前推理结束后清理页面；A 锁定/切 B 后旧结果不进入新会话；取消与完成竞争只有一个终态。
- **验证配置：**`R` + `CORE` + `F` + `CONTRACT` + `CLI`。**定向验证：**Host/core 新增 rf029_* 引擎屏障测试；cargo test -p solo_soul --lib rf029_；cargo test -p solosoul-core --lib rf029_；OCR 实际调用入口补 Vitest，不使用真实敏感文件。
- **建议提交：**`feat: resolve [RF-029] - add session-bound OCR cancellation`。

## 6.2 前端行为与控件

### RF-100

**自动上下文立即排除非公开字段** · P1 · 来源：R03

- **前置：**无。
- **入口：**`tauri/src/lib/llm/systemPromptBuilder.ts`；`tauri/src/lib/llm/chatRequest.ts`；`tauri/src/lib/propertyFlatten.ts`；`tauri/src/components/trash/ProtectedTrashValue.tsx`；`tauri/src/lib/fieldSensitivity.ts（拟新增）`；`tauri/src/lib/llm/systemPromptBuilder.test.ts（拟新增）`。
- **执行：**从现有字段语义提取纯敏感度解析 helper，遵循 propertyLabels → __fields → 模板 → internal；显式非法标签不得回退成 public。buildSection3PublicObjectData 保留对象级 public 筛选，并逐属性仅输出明确允许的 public 值；排除 __* 内部键；动态组按既有定义递归解析，子项不得降低父级保护等级，不得用 String(value) 整组输出。先过滤再限数量和长度；不修改原对象/store，不增加外发设置。该前端止血先于后端 Rust 出站投影迁移，后续显示策略复用同一解析 helper，避免新增局部规则。
- **验收：**public 对象的四级字段中只出现明确允许的 public 值；缺失/非法标签、父级受保护的子项、内部元数据均不进入最终消息。模板删除、标签优先级冲突及动态组混合等级仍按同一规则；输入对象未被修改。
- **验证配置：**`F` + `DOC`。**定向验证：**新增 systemPromptBuilder.test.ts，用合成对象覆盖四等级、未知标签、__fields/模板回退、动态组、内部键；调用 buildChatRequestMessages 捕获最终请求消息，不发真实网络请求。
- **建议提交：**`fix: resolve [RF-100] - filter non-public fields from automatic LLM context`。

### RF-101

**本次用户消息只追加一次** · P2 · 来源：R04

- **前置：**无。
- **入口：**`tauri/src/hooks/useLlmChatCore.ts`；`tauri/src/lib/llm/chatRequest.ts`；`tauri/src/lib/llm/systemPromptBuilder.ts`；`tauri/src/lib/llm/chatRequest.test.ts（拟新增）`。
- **执行：**统一 builder 接收本次输入之前的历史。sendMessage 的 UI 仍使用已追加用户消息的 updatedMessages，传给 builder 的 history 使用追加前列表；修正接口注释和全部调用点，不夹带流状态或协议重构。
- **验收：**系统提示开启/关闭、空历史/多轮历史中，本次输入恰好一条，旧历史顺序及 system/guide 合并行为不变。
- **验证配置：**`F`。**定向验证：**新增 chatRequest.test.ts，断言最终消息序列；复核 useLlmChatCore 的真实传参，不只单测 builder 假定输入。
- **建议提交：**`fix: resolve [RF-101] - append the current chat message exactly once`。

### RF-102

**搜索查询与缓存写入绑定会话和请求代次** · P1 · 来源：R02

- **前置：**无。
- **入口：**`tauri/src/lib/searchShared.tsx`；`tauri/src/lib/searchShared.test.tsx`；`tauri/src/lib/searchCache.ts`；`tauri/src/pages/search/SearchPage.tsx`；`tauri/src/components/layout/SearchPopover.tsx`；`tauri/src/hooks/useUnifiedSearch.ts（拟新增）`；`tauri/src/hooks/useUnifiedSearch.test.tsx（拟新增）`。
- **执行：**新增 useUnifiedSearch 管理 debounce、查询代次、filter、清空和卸载，复用 sessionRequests。搜索执行函数返回结果而非直接接收页面 setter；仅当前会话/查询可写缓存、结果、错误和 loading。清空及 filter 变化立即失效旧查询并清理计时器；两个搜索入口共用控制器。
- **验收：**A/B 倒序响应只保留 B；清空后旧查询不能恢复结果或缓存；锁定/换账户后旧数据及错误不回填；旧请求 finally 不结束新请求 loading；卸载后无迟到写入。
- **验证配置：**`F` + `WEB`。**定向验证：**扩展 searchShared.test.tsx，新增 useUnifiedSearch.test.tsx，用 deferred Promise 和假计时器验证乱序、清空、切 filter、卸载及 A→锁定→B；复用 SearchPopover.test.tsx 和 sidebar-tools.spec.ts 搜索场景。
- **建议提交：**`fix: resolve [RF-102] - isolate search requests and cache writes by session`。

### RF-103

**聊天会话读取只接纳最新选择** · P1 · 来源：R02

- **前置：**无。
- **入口：**`tauri/src/hooks/useLlmChatCore.ts`；`tauri/src/pages/ai/LlmChatPage/useLlmChat.ts`；`tauri/src/components/layout/AiQuickChatPopover.tsx`；`tauri/src/hooks/useLlmChatCore.test.tsx（拟新增）`。
- **执行：**用 sessionRequests 隔离会话正文、列表和回收站读取；正文选择使用独立 latest-request key。新建、选择会话、关闭快捷聊天及账户变化使相关旧读取失效；替换仅保护部分读取的共享 AbortController 做法。
- **验收：**选择 A→B 后即使 A 最后返回仍显示 B；读取途中新建、关闭或锁定不会恢复旧内容；旧错误不提示到新会话；列表请求不意外取消正文请求。
- **验证配置：**`F`。**定向验证：**新增 useLlmChatCore.test.tsx，使用 deferred Promise 覆盖正文/列表乱序、回收站读取、新建会话、快捷入口卸载和会话切换。
- **建议提交：**`fix: resolve [RF-103] - reject stale conversation loads`。

### RF-104

**聊天流归属明确且最终回复只有一个持久化写入者** · P1 · 来源：R02

- **前置：**[RF-101](#rf-101)、[RF-103](#rf-103)、[RF-002](#rf-002)、[RF-005](#rf-005)。
- **入口：**`tauri/src/stores/llmStore.ts`；`tauri/src/stores/llmStore.test.ts`；`tauri/src/hooks/useLlmStreaming.ts`；`tauri/src/hooks/useLlmChatCore.ts`；`tauri/src/lib/llm/conversationPersistence.ts`；`tauri/src/hooks/useLlmStreaming.test.tsx（拟新增）`。
- **执行：**按 RF-002 后端事件契约将流与 accountId、conversationId、requestId 绑定；不再将全局 chunk 写入当前列表末尾的 assistant。聊天页与快捷聊天只读对应投影；后端负责最终回复保存，删除前端流结束时整段覆盖保存，保留创建、改名等明确操作及持久化失败提示。
- **验收：**A 生成时切 B、两个聊天入口同时打开、旧请求事件迟到及锁定换账户都不污染 B；只更新目标 assistant；最终回复只保存一次且目标明确；后端保存失败保留回复并给出一次提示。
- **验证配置：**`F`。**定向验证：**扩展 llmStore.test.ts，新增 useLlmStreaming.test.tsx；模拟带身份的流事件、两个订阅入口、会话切换、后端持久化成功/失败；统计最终保存调用。
- **建议提交：**`fix: resolve [RF-104] - bind chat streams to their originating conversation`。

### RF-105

**历史未揭示值不再以原文加 blur 渲染** · P1 · 来源：R05

- **前置：**无。
- **入口：**`tauri/src/components/object/HistoryViewer.tsx`；`tauri/src/components/object/HistoryViewer.test.tsx`。
- **执行：**renderValueSpan 对未揭示 sensitive/critical 值渲染占位内容，去掉原文 blur；使用可键盘操作的按钮触发揭示，保留 critical 验证与审计。本项暂不改变 internal 上下文规则，后续共享策略迁移单独提交。
- **验收：**验证前 DOM、title 和可访问名称均不含受保护原文；Enter/Space 可触发；验证取消不揭示；成功后显示，TTL 后恢复占位。
- **验证配置：**`F`。**定向验证：**修改 HistoryViewer.test.tsx 的旧 blur 断言；补 DOM 不含原值、键盘触发、验证取消及 TTL 用例。
- **建议提交：**`fix: resolve [RF-105] - remove concealed history values from rendered content`。

### RF-106

**对象详情采用共享字段展示策略** · P1 · 来源：R05

- **前置：**[RF-100](#rf-100)。
- **入口：**`tauri/src/components/object/ObjectDetailFieldsList.tsx`；`tauri/src/components/object/ObjectDetailFieldsList.test.tsx`；`tauri/src/components/object/useObjectDetailVerification.tsx`；`tauri/src/components/object/useObjectDetailVerification.test.tsx`；`tauri/src/hooks/useRevealState.ts`；`tauri/src/lib/masking.ts`；`tauri/src/lib/fieldPresentationPolicy.ts（拟新增）`；`tauri/src/lib/fieldPresentationPolicy.test.ts（拟新增）`；`tauri/src/components/ui/ProtectedFieldValue.tsx（拟新增）`。
- **执行：**新增 FieldPresentationPolicy 与共享保护值组件，首批接入对象详情。默认 public 明文、其他三级占位、critical 通过现有验证接口；等级解析复用 RF-100 helper。共享 UI 接收验证/复制 callback，不读取业务 store；字段 identity 绑定对象、字段和内容版本，旧验证不得揭示新内容；复制和揭示使用同一门控。
- **验收：**四等级和未知标签遵守同一规则；受保护原文不出现在未揭示 DOM；取消验证不揭示/复制；TTL 后重掩；动态组继承保护强度；对象、内容或账户变化后旧验证无效。
- **验证配置：**`F` + `DOC`。**定向验证：**扩展 ObjectDetailFieldsList.test.tsx、useObjectDetailVerification.test.tsx；新增 fieldPresentationPolicy.test.ts，覆盖等级解析、动态组、取消、复制、TTL、身份变化及锁定迟到验证。
- **建议提交：**`fix: resolve [RF-106] - apply the shared protected-field policy to object details`。

### RF-107

**历史快照迁入共享字段展示策略** · P1 · 来源：R05

- **前置：**[RF-105](#rf-105)、[RF-106](#rf-106)。
- **入口：**`tauri/src/components/object/HistoryViewer.tsx`；`tauri/src/components/object/HistoryViewer.test.tsx`。
- **执行：**使用共享保护值组件，internal 同样默认掩码；保留历史快照自身的名称、等级和顺序，不以当前模板覆盖历史语义。删除重复 TTL/揭示渲染，保留历史验证和审计适配。
- **验收：**四等级规则统一；删除/修改模板不改写历史展示语义；快照切换不继承旧揭示或迟到验证；动态组、TTL 和键盘揭示正确。
- **验证配置：**`F`。**定向验证：**更新 HistoryViewer.test.tsx 的 internal 明文测试，覆盖四等级、模板删除/改名、快照切换、动态组、迟到验证及 TTL。
- **建议提交：**`fix: resolve [RF-107] - unify protected-field behavior in history snapshots`。

### RF-108

**搜索命中值采用共享保护与验证入口** · P1 · 来源：R05

- **前置：**[RF-102](#rf-102)、[RF-106](#rf-106)。
- **入口：**`tauri/src/lib/searchShared.tsx`；`tauri/src/lib/searchShared.test.tsx`；`tauri/src/pages/search/SearchPage.tsx`；`tauri/src/components/layout/SearchPopover.tsx`；`tauri/src/components/layout/SearchPopover.test.tsx`。
- **执行：**替换 FieldValueHint 局部 span 揭示，复用共享保护和验证入口，critical 不得普通点击直接揭示。结果仅有聚合等级而无法确定命中字段等级时使用既有聚合最严格等级，不推测降级；保护值不进入未揭示 title/aria；显示操作阻止误触父结果导航。
- **验收：**四等级及混合等级按策略保护；验证取消不显示；键盘可操作；显示按钮不打开父结果详情；TTL 和新查询替换结果后旧揭示失效。
- **验证配置：**`F` + `WEB`。**定向验证：**扩展 searchShared.test.tsx、SearchPopover.test.tsx；用合成命中结果覆盖混合等级、验证取消、冒泡、键盘、TTL 和查询替换；复用 sidebar-tools.spec.ts 搜索交互。
- **建议提交：**`fix: resolve [RF-108] - enforce shared protection for search match values`。

### RF-109

**回收站保护层复用共享字段策略** · P2 · 来源：R05

- **前置：**[RF-106](#rf-106)。
- **入口：**`tauri/src/components/trash/ProtectedTrashValue.tsx`；`tauri/src/components/trash/ProtectedTrashValue.test.tsx`；`tauri/src/components/trash/TrashDetailSections.tsx`；`tauri/src/components/trash/TrashSnapshotView.tsx`；`tauri/src/components/trash/TrashDetailPanel.test.tsx`。
- **执行：**将 ProtectedTrashValue 收为共享保护值组件的薄适配；保留回收站解析、验证来源审计，以及账户/快照/内容变化使旧验证失效的保障。父子等级使用公共 helper；schemaOnly 模板定义与真实字段值保持区分。
- **验收：**现有保护行为不弱化；四等级、父子继承、旧验证失效和 schemaOnly 场景正确；操作按钮对齐、长值换行和触控面积保持。
- **验证配置：**`F` + `WEB`。**定向验证：**运行 ProtectedTrashValue.test.tsx、TrashDetailPanel.test.tsx；仅为新增行为边界补用例；复用 e2e/trash-field-layout.spec.ts。
- **建议提交：**`refactor: resolve [RF-109] - reuse protected-field presentation in trash views`。

### RF-110

**同次主题应用只解析一次系统模式** · P1 · 来源：R12

- **前置：**无。
- **入口：**`tauri/src/lib/theme.ts`；`tauri/src/lib/themeSchemes.ts`；`tauri/src/lib/theme.test.ts（拟新增）`；`tauri/e2e/native-theme.spec.ts`。
- **执行：**applyTheme 先得到本次统一 resolvedMode、scheme 和 accent，再使用同一结果更新 DOM、色板、标题栏及状态栏；内部函数不得再次独立解析 system，兼容已有 resolvedSystemTheme 调用者。不修改移动端系统来源，后者由 RF-201 负责。
- **验收：**IPC 与 matchMedia 返回相反值时，DOM 模式、色板、原生 RGB 和状态栏模式一致；system 一次解析，显式 light/dark 不查询系统；浏览器回退仍可用。
- **验证配置：**`F` + `WEB`。**定向验证：**新增 theme.test.ts，mock 相反主题来源并捕获原生调用参数；复用 native-theme.spec.ts。移动端实机最终验收由 RF-112 联同 RF-201 完成。
- **建议提交：**`fix: resolve [RF-110] - share one resolved theme across web and native surfaces`。

### RF-111

**设置保存失败返回明确结果并反馈用户** · P1 · 来源：R12

- **前置：**无。
- **入口：**`tauri/src/stores/settingsStore.ts`；`tauri/src/stores/settingsStore.test.ts`；`tauri/src/stores/sessionIsolation.test.ts`；`tauri/src/pages/settings/AppearanceSettingsPage.tsx`；`tauri/src/pages/settings/SecuritySettingsPage.tsx`；`tauri/src/pages/settings/BackupConfigPage.tsx`；`tauri/src/hooks/useSettingAction.ts（拟新增）`。
- **执行：**updateSetting 明确返回成功、失败或会话失效结果；失败回滚仅作用当前写入，不再只记日志后表现为成功。通过共享设置交互 helper 提示失败，调用者不得失败后写缓存或执行成功流程；保持每键代次，迟到失败不能撤销新值。
- **验收：**保存失败可见且提示一次，当前值回滚到正确基线；同键连续保存、切账户后失败不回滚新状态；失败不更新持久化缓存或触发成功逻辑。
- **验证配置：**`F`。**定向验证：**扩展 settingsStore.test.ts、sessionIsolation.test.ts，用 deferred Promise 覆盖失败、连续写和账户切换；外观与安全设置各补一个失败交互用例（对应测试文件不存在时实施阶段明确新增）。
- **建议提交：**`fix: resolve [RF-111] - surface setting write failures without stale rollback`。

### RF-112

**ThemeController 成为唯一主题应用协调器** · P1 · 来源：R12

- **前置：**[RF-110](#rf-110)、[RF-111](#rf-111)、[RF-201](#rf-201)。
- **入口：**`tauri/src/App/AppRoutes.tsx`；`tauri/src/hooks/useApplyThemeFromSettings.ts`；`tauri/src/hooks/useApplyThemeFromSettings.test.ts`；`tauri/src/pages/settings/AppearanceSettingsPage.tsx`；`tauri/src/pages/auth/useLoginPage.tsx`；`tauri/src/pages/auth/BootstrapPage.tsx`；`tauri/src/bootstrapApp.tsx`；`tauri/src/lib/theme.ts`；`tauri/src/lib/themeController.ts（拟新增）`；`tauri/src/lib/themeController.test.ts（拟新增）`。
- **执行：**应用级协调器订阅有效设置和系统主题并管理请求代次，慢 system 结果不得覆盖新 light/dark 或新账户。页面仅更新偏好；React 挂载前保留缓存首帧再交接协调器。移除 AppRoutes、登录/创建页及外观页面的重复应用入口，保留 nativeWindow 现有串行化，不改平台能力模型。
- **验收：**system→light 快速切换最终为 light；StrictMode 不重复监听；失败回滚、锁定解锁及账户切换后 DOM、色板和原生栏一致；旧任务不能应用过期外观；首帧缓存仍生效。
- **验证配置：**`F` + `WEB` + `NATIVE` + `DOC`。**定向验证：**新增 themeController.test.ts，调整 useApplyThemeFromSettings.test.ts；复用 appearance-layout.spec.ts、native-theme.spec.ts、login-method-layout.spec.ts；RF-201 完成后在 Android/iOS 系统切色和桌面锁定恢复场景实测。
- **建议提交：**`refactor: resolve [RF-112] - centralize theme application in a single controller`。

### RF-113

**常驻壳配置注册和注销具有页面所有者** · P2 · 来源：R17

- **前置：**无。
- **入口：**`tauri/src/components/layout/PageShell.tsx`；`tauri/src/components/layout/PageShell.test.tsx`；`tauri/src/components/layout/shellConfigStore.ts`；`tauri/src/components/layout/ShellLayout.tsx`；`tauri/src/components/layout/ShellLayout.test.tsx`。
- **执行：**配置增加 owner/route identity；PageShell 卸载仅注销仍归自己的配置，旧 cleanup 不能清掉新页面注册。会话变化清空 ReactNode 和闭包；无有效配置采用明确空态，页面渲染失败不得保留上一页操作。
- **验收：**A注册→B注册→A cleanup 后 B 配置仍在；页面抛错、动态参数切换、锁定换账户均无旧标题、按钮或 callback 残留；常驻壳不因正常导航卸载。
- **验证配置：**`F` + `WEB`。**定向验证：**扩展 PageShell.test.tsx、ShellLayout.test.tsx，包含故意抛错页面、交错注册/清理、账户变化；复用 desktop-shell.spec.ts 和 home-navigation.spec.ts。
- **建议提交：**`fix: resolve [RF-113] - scope shell actions to their owning page`。

### RF-114

**AppRoutes 生命周期编排按职责收敛** · P2 · 来源：R17

- **前置：**[RF-112](#rf-112)、[RF-113](#rf-113)。
- **入口：**`tauri/src/App/AppRoutes.tsx`；`tauri/src/App/index.tsx`；`tauri/src/lib/asyncListener.ts`；`tauri/src/lib/sessionRequests.ts`；`tauri/src/App/useSessionLifecycle.ts（拟新增）`；`tauri/src/App/useNativeAppEvents.ts（拟新增）`；`tauri/src/App/AppNotifications.tsx（拟新增）`；`tauri/src/App/appLifecycle.test.tsx（拟新增）`。
- **执行：**分别抽取会话启动/清理、原生及 SAF 事件监听、全局通知装配模块。清理保持单入口和幂等，复用已有 store 会话清理注册，避免重复全量名单。保留认证路由、常驻 Shell 和静态页面加载，不夹带 lazy 或数据模型修改。
- **验收：**StrictMode 无重复监听，卸载后 listener 被释放；单次 vault-locked 完成一次清理和导航；认证加载顺序、更新/OCR/SAF 横幅及常驻壳行为不变。
- **验证配置：**`F` + `WEB`。**定向验证：**新增 appLifecycle.test.tsx 验证真实事件与清理行为；复用 sessionIsolation.test.ts、startup.spec.ts、home-navigation.spec.ts、desktop-shell.spec.ts，不增加仅验证函数拆分的镜像测试。
- **建议提交：**`refactor: resolve [RF-114] - isolate application lifecycle orchestration`。

### RF-115

**普通操作按钮族迁入语义样式入口** · P2 · 来源：R16

- **前置：**[RF-112](#rf-112)。
- **入口：**`tauri/src/components/ui/Button.tsx`；`tauri/src/components/ui/Button.module.css`；`tauri/src/components/ui/Button.test.tsx`；`tauri/src/components/ui/DeleteButton.tsx`；`tauri/src/components/transfer/TransferButton.tsx`；`tauri/src/components/transfer/TransferButton.test.tsx`；`tauri/src/styles/desktop-controls.css`；`tauri/src/styles/android.css`。
- **执行：**保留 variant API，以 intent/size 语义映射公共 token；组件负责结构和状态，平台层提供颜色、圆角和目标尺寸。迁移 Button、DeleteButton、TransferButton 及其遗留 toolbar 适配，只删除这些已覆盖调用者对应规则；不同时改变 icon、choice、checkbox。
- **验收：**primary/secondary/danger/warning、disabled/loading、hover/focus 和长文案在三平台浅深主题保持正确；自定义强调色对比可读；旧 variant 调用兼容，无触控尺寸回退。
- **验证配置：**`F` + `WEB`。**定向验证：**保留 Button.test.tsx、TransferButton.test.tsx；运行 desktop-controls.spec.ts、update-button-style.spec.ts、notification-layout.spec.ts、android-touch-targets.spec.ts。低影响样式不新增 CSS 镜像单测。
- **建议提交：**`refactor: resolve [RF-115] - give action buttons a single semantic style contract`。

### RF-116

**图标按钮族统一结构和平台尺寸** · P2 · 来源：R16

- **前置：**[RF-115](#rf-115)。
- **入口：**`tauri/src/components/ui/BadgeIconButton.tsx`；`tauri/src/components/ui/BadgeIconButton.module.css`；`tauri/src/components/layout/ToolbarActions.tsx`；`tauri/src/components/guide/PageGuideButton.tsx`；`tauri/src/styles/desktop-controls.css`；`tauri/src/styles/android.css`。
- **执行：**统一 accessible label、图标尺寸、hit area、危险 intent、pressed/focus 状态，保留 BadgeIconButton 兼容 API。迁移这些入口的 icon legacy class，删除已无调用者的对应覆盖；不改变导航卡片点击模型。
- **验收：**移动端触控目标、桌面紧凑尺寸、禁用、键盘焦点及 tooltip 正确；图标视觉尺寸不随 hit area 放大失衡；操作只执行一次。
- **验证配置：**`F` + `WEB`。**定向验证：**运行 toolbar-actions.spec.ts、android-touch-targets.spec.ts、desktop-controls.spec.ts；仅在交互契约变化时补行为测试，不新增样式镜像单测。
- **建议提交：**`refactor: resolve [RF-116] - unify semantic icon-button sizing and states`。

### RF-117

**互斥选项与下拉选择族统一状态语义** · P2 · 来源：R16

- **前置：**[RF-115](#rf-115)。
- **入口：**`tauri/src/components/ui/FilterChipGroup.tsx`；`tauri/src/components/ui/FilterChipGroup.module.css`；`tauri/src/components/ui/DropdownSelect.tsx`；`tauri/src/components/ui/DropdownSelect.module.css`；`tauri/src/components/export/ExportDocumentSection.tsx`；`tauri/src/components/settings/ExportImportTabBar.tsx`；`tauri/src/components/settings/ExportImportTabBar.module.css`；`tauri/src/styles/android.css`；`tauri/src/styles/desktop-controls.css`。
- **执行：**选择组件自身的 selected/disabled/focus 语义驱动样式，不从背景色或历史 class 推断。保留 aria-pressed 及现有选择交互，平台 token 提供表面和尺寸；移除对应旧覆盖，不涉及 checkbox/switch。
- **验收：**深浅主题、自定义强调色下选中状态明确；长格式名不溢出；键盘选择、disabled 和 Portal 菜单行为正确；导出格式及现有选择结果不变。
- **验证配置：**`F` + `WEB`。**定向验证：**运行 export-choice-layout.spec.ts、appearance-layout.spec.ts、android-material.spec.ts，补到现有展示场景验证长文案与 Portal；不新增样式镜像单测。
- **建议提交：**`refactor: resolve [RF-117] - standardize choice-control presentation`。

### RF-118

**开关控件族统一尺寸与状态 token** · P2 · 来源：R16

- **前置：**[RF-115](#rf-115)。
- **入口：**`tauri/src/components/ui/ToggleSwitch.tsx`；`tauri/src/styles/android.css`；`tauri/src/pages/settings/SecuritySettingsPage.tsx`；`tauri/src/pages/settings/AppearanceSettingsPage.tsx`。
- **执行：**开关自身管理轨道、thumb、checked/disabled/focus 和触控目标，平台层只提供 token；保留 boolean callback，移除重复尺寸覆盖。不改变 RF-111 已定义的设置保存行为。
- **验收：**鼠标、Space、label 点击各切换一次；disabled 不变；Android 目标至少48px；深浅主题下状态可辨识，桌面布局无膨胀。
- **验证配置：**`F` + `WEB`。**定向验证：**复用 appearance-layout.spec.ts、android-touch-targets.spec.ts 的设置页面；增加开关键盘和 label 场景到现有 E2E，不写 CSS 镜像单测。
- **建议提交：**`refactor: resolve [RF-118] - centralize switch platform styling`。

### RF-119

**Checkbox 控件族样式归属收敛** · P2 · 来源：R16

- **前置：**[RF-115](#rf-115)。
- **入口：**`tauri/src/components/ui/SelectCheckbox.tsx`；`tauri/src/components/ui/SelectCheckbox.module.css`；`tauri/src/components/ui/SelectCheckbox.test.tsx`；`tauri/src/components/transfer/ObjectSelectionTree.tsx`；`tauri/src/components/trash/TrashItemCard.tsx`；`tauri/src/styles/android.css`；`tauri/src/styles/desktop-controls.css`。
- **执行：**保留既有三态、父行事件和触控尺寸契约；将尺寸/形状/边界 token 收入 checkbox 自身语义入口，平台仅提供变量；移除对应历史覆盖，不重写选择逻辑。
- **验收：**checked/mixed/disabled/focus 保持；整行选择和 label 不重复触发；三平台深浅主题对比和触控目标不回退；回收站选择框仍与图标对齐。
- **验证配置：**`F` + `WEB`。**定向验证：**运行 SelectCheckbox.test.tsx、checkbox-platform.spec.ts、trash-selection-layout.spec.ts；只有行为发生变化才补单测，不新增样式镜像测试。
- **建议提交：**`refactor: resolve [RF-119] - consolidate checkbox surface and target tokens`。

### RF-120

**字段值与操作按钮采用统一行布局** · P2 · 来源：R16

- **前置：**[RF-107](#rf-107)、[RF-109](#rf-109)、[RF-116](#rf-116)。
- **入口：**`tauri/src/components/ui/ValueContainer.tsx`；`tauri/src/components/object/ObjectDetailFieldsList.tsx`；`tauri/src/components/object/ObjectDetailModal.module.css`；`tauri/src/components/trash/ProtectedTrashValue.tsx`；`tauri/src/components/object/HistoryViewer.tsx`。
- **执行：**收敛标签、徽章、值、操作槽位与垂直对齐；保护组件负责行为，字段行仅负责布局。长文本换行重测后，值缩短或视口变宽能恢复适合的布局；不修改敏感度规则。
- **验收：**短值→长值→掩码、缩放、动态组、多语言及放大字体下不溢出；揭示/解锁按钮与值对齐；移动端目标尺寸保留；宽度恢复后不永久停留在扩展行布局。
- **验证配置：**`F` + `WEB`。**定向验证：**运行 trash-field-layout.spec.ts、object-ruler.spec.ts，复用现有详情/历史展示场景增加值变化、缩放与长文案；低影响布局不添加镜像单测。
- **建议提交：**`refactor: resolve [RF-120] - unify protected-field row layout`。

### RF-121

**普通卡片表面使用平台无关语义** · P2 · 来源：R16

- **前置：**[RF-110](#rf-110)。
- **入口：**`tauri/src/components/ui/Card.tsx`；`tauri/src/components/ui/Card.module.css`；`tauri/src/components/ui/CardGrid.tsx`；`tauri/src/styles/macos-glass.css`；`tauri/src/styles/windows-material.css`。
- **执行：**Card 输出平台无关 surface 标记，适配层映射现有 macOS 玻璃、Windows 内容表面和 Android Material token；Windows 不再依赖 data-macos-glass 解释普通卡片。保持导航区 Mica 与内容区分工，不调整侧栏结构。仅该 Card 调用族全部迁完后删除对应旧规则。
- **验收：**内嵌/浮动卡片在三平台浅深主题、减少透明度和高对比下表面正确；长内容不溢出；Windows 导航/内容分区及 macOS 玻璃恢复不回退。
- **验证配置：**`F` + `WEB` + `NATIVE`。**定向验证：**运行 macos-glass.spec.ts、desktop-shell.spec.ts、android-material.spec.ts、home-navigation.spec.ts；原生设备检查恢复和材质合成。浏览器 mock 不作为原生材质验证证据。
- **建议提交：**`refactor: resolve [RF-121] - express card surfaces without platform-specific markup`。

### RF-122

**模态对话框表面迁入统一语义** · P2 · 来源：R16

- **前置：**[RF-121](#rf-121)、[RF-115](#rf-115)、[RF-116](#rf-116)。
- **入口：**`tauri/src/components/ui/Dialog.tsx`；`tauri/src/components/ui/Dialog.module.css`；`tauri/src/components/ui/Dialog.test.tsx`；`tauri/src/components/ui/ConfirmDialog.tsx`；`tauri/src/components/ui/PromptDialog.tsx`；`tauri/src/components/forms/PasswordVerificationDialog.tsx`；`tauri/src/styles/windows-material.css`；`tauri/src/styles/macos-glass.css`。
- **执行：**统一 dialog/backdrop 表面和优先级标记，移除 Windows 对 data-macos-glass 的依赖；保留 Portal、关闭、表单和密码验证行为。不在样式迁移中重写验证流程或嵌套弹窗管理，旧规则仅在该模态族迁完后删除。
- **验收：**auth 优先级、Portal 继承、背景点击/Escape、窄屏和键盘遮挡、深浅主题与减少透明度行为保持；密码取消/提交流程不变。
- **验证配置：**`F` + `WEB` + `NATIVE`。**定向验证：**运行 Dialog.test.tsx 和相关验证组件测试；复用 notification-layout.spec.ts、login-method-layout.spec.ts、详情弹窗场景；原生检查材质表面，不写样式镜像单测。
- **建议提交：**`refactor: resolve [RF-122] - unify modal surface styling`。

### RF-123

**侧栏快捷浮层表面迁入统一语义** · P2 · 来源：R16

- **前置：**[RF-121](#rf-121)、[RF-102](#rf-102)、[RF-104](#rf-104)。
- **入口：**`tauri/src/components/layout/SearchPopover.tsx`；`tauri/src/components/layout/SearchPopover.module.css`；`tauri/src/components/layout/AiQuickChatPopover.tsx`；`tauri/src/components/layout/AiQuickChatPopover.module.css`；`tauri/src/components/layout/OcrQuickScanPopover.tsx`；`tauri/src/components/layout/OcrQuickScanPopover.module.css`；`tauri/src/components/layout/navButtonCards.tsx`；`tauri/src/styles/macos-glass.css`；`tauri/src/styles/windows-material.css`。
- **执行：**仅迁快捷浮层族的 surface/placement/density 标记和 token，替换平台命名标记；保留位置计算、外部点击、导航和既有玻璃容器层次。该族调用点迁完才删除旧选择器；其他菜单/预览浮层不作全局替换。
- **验收：**侧栏展开/折叠、左右侧、视口边缘、Portal、焦点、外部点击和长内容均正确；搜索/聊天功能不变；macOS/Windows 浮层材质恢复不回退。
- **验证配置：**`F` + `WEB` + `NATIVE`。**定向验证：**运行 desktop-card-position.spec.ts、sidebar-tools.spec.ts、plugin-quick-panel.spec.ts 中相关表面场景及 macos-glass.spec.ts；真实平台检查材质恢复，不写 CSS 镜像测试。
- **建议提交：**`refactor: resolve [RF-123] - standardize sidebar popover surfaces`。

### RF-124

**迁移菜单、日期选择和 Tooltip 表面** · P2 · 来源：R16

- **前置：**[RF-117](#rf-117)、[RF-121](#rf-121)。
- **入口：**`tauri/src/components/layout/SecondaryActionBar.tsx`；`tauri/src/components/forms/DatePickerCalendar.tsx`；`tauri/src/components/forms/PasswordInput.tsx`；`tauri/src/components/ui/DropdownSelect.tsx`；`tauri/src/components/layout/NavButton.tsx`；`tauri/src/components/export/AttachmentLimitsInfo.tsx`。
- **执行：**将该菜单/提示族 data-macos-glass 标记迁为中立 surface/role token，保留Portal定位、hover/焦点/键盘和外部关闭语义；删除本族已无引用的兼容选择器。
- **验收：**深浅主题、边缘定位、键盘导航与长tooltip可读；密码提示不新增明文；对应Windows表面不再借用macOS命名。
- **验证配置：**`F` + `WEB` + `NATIVE`。**定向验证：**现有desktop-controls/sidebar-tools/Android触控与日期选择场景，分别验证Portal和键盘。
- **建议提交：**`refactor(ui): migrate menu surfaces [RF-124]`。

### RF-125

**迁移对象详情与附件预览浮层表面** · P2 · 来源：R16

- **前置：**[RF-107](#rf-107)、[RF-109](#rf-109)、[RF-121](#rf-121)、[RF-122](#rf-122)。
- **入口：**`tauri/src/components/object/ObjectDetailModal.tsx`；`tauri/src/components/object/AttachmentViewer.tsx`；`tauri/src/components/object/HistoryViewer.tsx`；`tauri/src/components/trash/TrashDetailPanel.tsx`；`tauri/src/components/attachment/AttachmentPreviewOverlay.tsx`。
- **执行：**将详情/预览浮层接入中立surface与backdrop token；保留原生预览窗交通灯、独立窗口几何、关闭和附件打开逻辑；移除这些入口旧材质标记对应的覆盖。
- **验收：**对象→附件→返回层级与点击关闭一致；macOS圆角/交通灯、Windows内容表面和Android触控不回退；未揭示字段仍不出现在DOM。
- **验证配置：**`F` + `WEB` + `NATIVE`。**定向验证：**详情/附件现有组件测试、macos-preview-titlebar.spec.ts 与原生预览窗恢复验收。
- **建议提交：**`refactor(ui): migrate detail and preview surfaces [RF-125]`。

### RF-126

**迁移独立业务对话框到中立表面标记** · P2 · 来源：R16

- **前置：**[RF-122](#rf-122)。
- **入口：**`tauri/src/components/plugin`；`tauri/src/components/template`；`tauri/src/components/recovery`；`tauri/src/components/onboarding`；`tauri/src/components/guide`；`tauri/src/components/settings/PinSetupDialog.tsx`；`tauri/src/components/llm-config/RiskAcceptanceDialog.tsx`。
- **执行：**限定为自有Dialog包装未覆盖的业务弹层，逐一登记调用者，将材质/遮罩标记接入同一surface token；保持表单、PIN、授权与引导状态机，禁止借样式迁移重写验证流程。同一根因的标记迁移可一起提交，若发现行为缺陷另列ID。
- **验收：**登记的独立弹层无漏迁；Portal主题继承、认证弹层优先级、取消/确认与原行为一致；浅深色和减少透明度可读。
- **验证配置：**`F` + `WEB` + `NATIVE`。**定向验证：**既有Dialog/PIN/恢复/模板/插件授权测试及相关E2E，原生抽查认证和恢复弹层。
- **建议提交：**`refactor(ui): migrate standalone dialog surfaces [RF-126]`。

### RF-127

**迁移通知表面并关闭旧材质兼容清单** · P2 · 来源：R16

- **前置：**[RF-115](#rf-115)、[RF-116](#rf-116)、[RF-117](#rf-117)、[RF-118](#rf-118)、[RF-119](#rf-119)、[RF-120](#rf-120)、[RF-121](#rf-121)、[RF-122](#rf-122)、[RF-123](#rf-123)、[RF-124](#rf-124)、[RF-125](#rf-125)、[RF-126](#rf-126)。
- **入口：**`tauri/src/components/ui/ToastContainer.tsx`；`tauri/src/styles/windows-material.css`；`tauri/src/styles/macos-glass.css`；`tauri/src/styles/desktop-controls.css`；`tauri/src/styles/android.css`。
- **执行：**迁移Toast notification表面；逐条核对剩余data-macos-glass和控件legacy选择器的调用者，确认全部对应任务完成后才移除死兼容规则。确需保留的真实平台适配明确注释；发现未覆盖业务族先新增任务，不能全局替换后宣称完成。
- **验收：**通知焦点/关闭/层级不变；Windows组件语义不依赖macOS标记；旧控件覆盖清单可追溯且没有无主例外，既有平台回归矩阵保持。
- **验证配置：**`F` + `WEB` + `NATIVE`。**定向验证：**notification-layout、desktop-controls、android-material、macos-glass与生产启动E2E；检查所有剩余材质标记的归属。
- **建议提交：**`refactor(ui): retire legacy material selectors [RF-127]`。

## 6.3 平台适配与 CLI

### RF-201

**修正移动端跟随系统的主题来源** · P1 · 来源：R12

- **前置：**[RF-208](#rf-208)。
- **入口：**`tauri/src-tauri/src/commands/system.rs`；`tauri/src-tauri/src/setup/mod.rs`；`tauri/src/lib/theme.ts`。
- **执行：**移动端不再把固定 dark 当成成功检测结果；使用真实原生主题事件或明确让前端 matchMedia 接管，并为同一平台只保留一个系统主题事件源。保留桌面检测和登录前主题缓存。
- **验收：**Android/iOS 系统浅色→深色→浅色均可更新；跟随系统才响应事件，显式 light/dark 不被覆盖；IPC 不可用有回退，无每秒固定 dark 覆盖。
- **验证配置：**`F` + `R` + `IOS` + `NATIVE` + `ANDROID_BUILD`。**定向验证：**扩展 theme/native-theme 测试，模拟原生与 WebView 相反值；移动设备执行跟随系统切换。
- **建议提交：**`fix(theme): resolve mobile system appearance [RF-201]`。

### RF-202

**将 APK 更新入口限定为 Android** · P1 · 来源：R13

- **前置：**无。
- **入口：**`tauri/src/stores/updateStore.ts`；`tauri/src/stores/updateStore.test.ts`；`tauri/src/lib/platform.ts`；`tauri/src/lib/updater.ts`。
- **执行：**分别门控缓存检查、版本检查、下载和安装四阶段；只有 Android 进入 APK 路径，iOS 返回明确不支持该更新方式并提供适当状态；保留 Windows/macOS 更新逻辑，不等待能力框架。
- **验收：**iOS 检查/下载/安装均不调用 android_*；Android 全流程保持；桌面 updater 保持；不支持不显示为网络失败。
- **验证配置：**`F`。**定向验证：**updateStore.test.ts 增加 Android/iOS/macOS/Windows 调用断言及下载完成后安装状态用例。
- **建议提交：**`fix(updater): gate APK operations to Android [RF-202]`。

### RF-203

**明确 iOS OCR 不支持时的前后端行为** · P1 · 来源：R13

- **前置：**[RF-208](#rf-208)。
- **入口：**`tauri/src-tauri/src/commands/ocr.rs`；`tauri/src-tauri/src/mobile_ocr_plugin.rs`；`tauri/src/pages/scan/OcrPage.tsx`；`tauri/src/pages/settings/OcrSettingsPage.tsx`；`tauri/src/lib/ipc.ts`。
- **执行：**拆开 Android/iOS 编译分支；没有原生实现的 iOS 返回稳定 unsupported 错误，前端禁用相应扫描动作并说明原因。保留模型信息/设置中仍可用的操作，不在此任务新增 iOS OCR 引擎。
- **验收：**iOS 不调用 Android bridge、不进入无限 loading；Android 扫描与桌面本地推理不变；设备支持状态与页面一致。
- **验证配置：**`F` + `R` + `IOS` + `ANDROID_BUILD`。**定向验证：**OCR 命令分支/前端 OcrPage 测试；iOS 编译证明分支无 Android 实现引用。
- **建议提交：**`fix(ocr): handle unsupported iOS bridge explicitly [RF-203]`。

### RF-204

**核实并修正 iOS Keychain 成功状态符号** · P1 · 来源：R13

- **前置：**无。
- **入口：**`tauri/crates/solosoul-core/src/biometric/ios.rs`；`.github/workflows/pr_check.yml`。
- **执行：**先在 macOS 对 iOS 真机与模拟器目标编译，记录 errSecSuccess 是否缺失；若确认，使用当前依赖导出的正确常量并保持错误码映射。只修此符号/相关直接编译问题，不顺手改认证策略。若误报，给出目标编译和解析证据。
- **验收：**两个 iOS target 均能通过本项涉及模块的检查；不存在用裸 0 替代命名常量来规避问题；误报必须有证据后才排除。
- **验证配置：**`R` + `IOS` + `CLI`。**定向验证：**cargo check 的 iOS 双 target。仅修 import/命名常量且认证行为保持时，以双目标检查和相关回归关闭；若修改 Keychain 调用或认证行为，则真机/模拟器支持的成功、取消、失败运行验证成为必需，缺设备标待验证。
- **建议提交：**`fix(ios): resolve Keychain success status [RF-204]`。

### RF-205

**建立并接入平台能力契约** · P2 · 来源：R13、R18

- **前置：**[RF-202](#rf-202)、[RF-203](#rf-203)、[RF-204](#rf-204)。
- **入口：**`tauri/src-tauri/src/commands/system.rs`；`tauri/src-tauri/src/lib.rs`；`tauri/src/lib/platform.ts`；`tauri/src/lib/ipc.ts`；`tauri/src/stores/updateStore.ts`；`tauri/src/pages/scan/OcrPage.tsx`。
- **执行：**定义 PlatformCapabilities 的 OS、更新方式、OCR、原生材质、生物识别和文件打开能力，区分 supported/unsupported/unavailable 及原因；由后端编译目标和实际桥接探测提供，允许启动期读取。将 updater/OCR 的临时门控迁移到契约；同步 IPC/ACL，不扩大权限。
- **验收：**macOS/Windows/Android/iOS 有固定契约 fixture，Linux 桌面回退单列；未知能力安全禁用且有原因；UI 不再从 mobile 布尔值推导 APK/OCR 支持。
- **验证配置：**`F` + `R` + `CONTRACT` + `IOS` + `ANDROID_BUILD`。**定向验证：**新增能力契约与平台矩阵测试，复跑 RF-202/203 用例。
- **建议提交：**`refactor(platform): centralize capability contracts [RF-205]`。

### RF-206

**提取可恢复且按版本跳过的 Android 资源安装器** · P2 · 来源：R19

- **前置：**[RF-208](#rf-208)。
- **入口：**`tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/MainActivity.kt`；`tauri/scripts/stage-mobile-resources.cjs`。
- **执行：**提取 ResourceInstaller；随资源生成版本/内容 manifest，先写临时目录、校验后切换完成标记；相同版本跳过写入。此项先保持现有同步就绪语义，仅解决重复复制与中断恢复，Activity 仍等待安装完成。
- **验收：**首次安装可读、同版本重启零重写、升级替换、中断后重试可恢复；未知临时目录不会被 Rust 当成完成资源；保留用户数据目录中的非内置内容。
- **验证配置：**`ANDROID_BUILD`。**定向验证：**新安装器的版本、校验失败和中断恢复测试；记录重复启动写入次数，检查 docs/插件实际可读。
- **建议提交：**`refactor(android): version bundled resource installation [RF-206]`。

### RF-207

**将 Android 资源准备移出主线程并接入就绪屏障** · P2 · 来源：R19

- **前置：**[RF-206](#rf-206)、[RF-208](#rf-208)。
- **入口：**`tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/MainActivity.kt`；`tauri/src-tauri/src/setup/mod.rs`；`tauri/src-tauri/src/commands/llm`；`tauri/src-tauri/src/plugin`。
- **执行：**使用安装器的显式 ready/error 状态在后台准备资源；定位 docs、插件消费者，等待相应资源就绪再读取；Activity 重建复用同一安装操作。只阻塞依赖资源的能力，不阻塞整个窗口绘制。
- **验收：**低速复制期间主线程仍可绘制/响应；消费者不会读半成品；失败可重试且可见；Activity 重建不重复启动安装；首次/重复启动耗时有对照。
- **验证配置：**`R` + `ANDROID_BUILD` + `NATIVE` + `ANDROID_NATIVE`。**定向验证：**慢安装器与失败注入、Activity 重建 instrumentation；实际启动主线程 trace。
- **建议提交：**`perf(android): prepare resources off the main thread [RF-207]`。

### RF-208

**移除 Android 构建的本机 JDK 路径依赖** · P2 · 来源：R19、R21

- **前置：**无。
- **入口：**`tauri/src-tauri/gen/android/gradle.properties`；`.github/workflows/build-android.yml`；`docs/platform-mobile/android-glass-implementation.md`。
- **执行：**执行计划核验新增项：项目 Gradle 属性当前固定 macOS JBR 路径。移除机器专属路径，使用受支持的 JDK 环境/本地覆盖约定，CI 显式配置 JDK；文档区分项目配置与机器私有路径，不提交 Windows 绝对路径替代。
- **验收：**Windows/Linux/macOS 不依赖 /Applications/... 才能启动 Gradle；Gradle 实际 JVM 版本符合项目要求；Android 编译入口保持有效。
- **验证配置：**`ANDROID_BUILD` + `DOC`。**定向验证：**各环境 gradlew --version 与目标 Debug 构建；本机缺 SDK 记录阻塞，不伪造跨平台成功。
- **建议提交：**`build(android): remove machine-specific JDK configuration [RF-208]`。

### RF-211

**为 CLI 建立任务事件与会话失效基础** · P2 · 来源：R15

- **前置：**[RF-001](#rf-001)。
- **入口：**`solosoul_cli/src/app.rs`；`solosoul_cli/src/events.rs`；`solosoul_cli/src/tui.rs`；`solosoul_cli/src/commands`。
- **执行：**复用已有 runtime，建立 task ID、账户会话代次、进度/完成/失败/取消事件和退出清理；主循环只接收事件，不等待业务 Future。先通过一个测试任务接入，不迁移所有命令；旧事件到达不得恢复锁定状态。
- **验收：**慢测试任务期间按键、重绘、Tick 和自动锁定正常；锁定/换账户使旧任务结果失效；退出完成取消/回收，不另建每命令 runtime。
- **验证配置：**`CLI`。**定向验证：**CLI 事件循环屏障测试及假时钟自动锁定；任务事件是唯一应用状态写入入口。
- **建议提交：**`refactor(cli): add session-bound task events [RF-211]`。

### RF-212

**将 CLI 模型下载迁移到任务事件** · P2 · 来源：R15

- **前置：**[RF-211](#rf-211)。
- **入口：**`solosoul_cli/src/commands/embed_model.rs`；`solosoul_cli/src/app.rs`。
- **执行：**将模型下载的 block_on 从输入循环移到后台任务；显示进度并允许取消，完成前不登记半文件为可用模型；退出清理由任务所有者处理。
- **验收：**慢下载时能输入/重绘/锁定；取消不残留有效模型记录；失败可重试，旧任务不能覆盖新下载状态。
- **验证配置：**`CLI`。**定向验证：**本地假下载流与取消/网络错误测试，不下载真实大模型作为普通单测。
- **建议提交：**`refactor(cli): run model downloads as tasks [RF-212]`。

### RF-213

**将 CLI 同步迁移到任务事件** · P2 · 来源：R15

- **前置：**[RF-211](#rf-211)。
- **入口：**`solosoul_cli/src/commands/sync.rs`；`solosoul_cli/src/app.rs`。
- **执行：**将同步等待从输入循环迁出，进度/配对结果通过 task ID 回传；接入已有同步取消/停机机制，不改协议、SAS 或授权策略。
- **验收：**同步等待期间输入和 Tick 可处理；锁定/退出取消或停止提交，旧事件不切回已解密页面；配对取消不被当成功。
- **验证配置：**`CLI`。**定向验证：**假 peer/阻塞屏障覆盖取消、失败、锁定与迟到完成。
- **建议提交：**`refactor(cli): run synchronization as a task [RF-213]`。

### RF-214

**将 CLI 插件安装迁移到任务事件** · P2 · 来源：R15

- **前置：**[RF-211](#rf-211)。
- **入口：**`solosoul_cli/src/commands/plugin.rs`；`solosoul_cli/src/app.rs`。
- **执行：**将插件安装网络等待移入任务，复用既有签名/校验/取消能力；仅成功完成后更新已安装列表，保留授权与沙箱规则。
- **验收：**安装时 CLI 不冻结；取消和失败不留下成功状态；锁定期间无旧账户 UI 回填；不降低签名检查。
- **验证配置：**`CLI`。**定向验证：**假安装源/校验失败/取消/迟到事件测试。
- **建议提交：**`refactor(cli): run plugin installation as a task [RF-214]`。

### RF-215

**将 CLI OCR 迁移到可取消后台任务** · P2 · 来源：R15

- **前置：**[RF-211](#rf-211)、[RF-029](#rf-029)。
- **入口：**`solosoul_cli/src/commands/ocr.rs`；`solosoul_cli/src/app.rs`；`tauri/crates/solosoul-core/src/ocr/engine.rs`。
- **执行：**将 OCR 模型加载和推理放入受限阻塞任务；接入共享引擎分页取消点；状态修改经主循环事件，锁定后丢弃结果并清理临时页。分页取消能力任务完成后再接入，不用取消 Future 冒充已停止推理。
- **验收：**OCR 中能处理 Esc/Tick；锁定不回填识别原文；取消在约定页边界生效，临时文件被回收；连续发起不会无限并发。
- **验证配置：**`CLI` + `CORE` + `R`。**定向验证：**假 OCR 引擎/多页文档，测试队列取消、运行中取消、失败和锁定。
- **建议提交：**`refactor(cli): run OCR through cancellable tasks [RF-215]`。

## 6.4 契约、验证、性能与文档

### RF-301

**建立 Rust 到 TypeScript 的增量 IPC 契约生成** · P2 · 来源：R18

- **前置：**无。
- **入口：**`tauri/src-tauri/src/lib.rs`；`tauri/src/lib/ipcClient.ts`；`tauri/src/lib/ipc.ts`；`tauri/scripts/check_acl_consistency.py`；`tauri/package.json`。
- **执行：**以 Rust DTO/命令登记为来源建立参数、响应、事件类型生成和 check 模式；先用一个只读 system 命令贯通 typed invoke，保留未迁移命令兼容适配。记录生成工具与版本选择，不在此项迁移全部命令。
- **验收：**重复生成无 diff；改 Rust 试点参数可触发 TS 错误或生成检查失败；命令登记/ACL/生成命令集可对照；生成过程不读取密钥或运行业务命令。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**生成器 fixture、命令集合比较与编译型负例；生成检查加入 package script。
- **建议提交：**`refactor(ipc): generate incremental command contracts [RF-301]`。

### RF-302

**迁移对象和回滚 IPC 契约** · P2 · 来源：R18

- **前置：**[RF-301](#rf-301)、[RF-008](#rf-008)、[RF-010](#rf-010)。
- **入口：**`tauri/src-tauri/src/commands/object`；`tauri/src/lib/ipc.ts`；`tauri/src/stores/objectStore.ts`；`tauri/src/components/object`。
- **执行：**迁移 object_* 与 snapshot_* 参数/返回类型；由生成模型替代对应手写 wire DTO，前端衍生展示字段留在 ViewModel。删除前先确认该组旧声明无引用。
- **验收：**该组无任意字符串调用和自选返回泛型；序列化键、可空值、标签兼容不变；对象/回滚用例通过。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**对象与快照现有测试、生成无漂移检查。
- **建议提交：**`refactor(ipc): type object and snapshot commands [RF-302]`。

### RF-303

**迁移 LLM 会话与流事件契约** · P2 · 来源：R18

- **前置：**[RF-301](#rf-301)、[RF-002](#rf-002)、[RF-004](#rf-004)、[RF-005](#rf-005)、[RF-104](#rf-104)。
- **入口：**`tauri/src-tauri/src/commands/llm`；`tauri/src/hooks/useLlmChatCore.ts`；`tauri/src/hooks/useLlmStreaming.ts`；`tauri/src/stores/llmStore.ts`。
- **执行：**生成聊天命令与流事件类型，包含 account/session/request/conversation 标识以及完成/持久化失败区分；移除本组手写 wire 副本，保留展示模型。先完成会话修复和凭证迁移后冻结这一契约。
- **验收：**流事件缺标识能在编译/验证时暴露；普通发送不要求 API key；回放旧会话、临时聊天、保存失败提示保持。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**LLM 组序列化 fixture 与前端流隔离回归。
- **建议提交：**`refactor(ipc): type LLM commands and stream events [RF-303]`。

### RF-304

**迁移备份与导入导出 IPC 契约** · P2 · 来源：R18

- **前置：**[RF-301](#rf-301)、[RF-013](#rf-013)、[RF-015](#rf-015)、[RF-024](#rf-024)。
- **入口：**`tauri/src-tauri/src/commands/backup.rs`；`tauri/src-tauri/src/commands/export_import`；`tauri/src/lib/ipc.ts`；`tauri/src/pages`。
- **执行：**生成备份、导入预览/执行、导出请求与结果类型；表达附件范围、部分失败/恢复状态，不通过含糊布尔值丢失含义；不改已有包格式。
- **验收：**前后端的枚举/空值/失败结果一致，旧文件仍可读；前端不能将部分失败当全部成功；生成无 diff。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**备份/导入导出 fixture 和前端流程测试。
- **建议提交：**`refactor(ipc): type backup and transfer contracts [RF-304]`。

### RF-305

**迁移同步 IPC 与事件契约** · P2 · 来源：R18

- **前置：**[RF-301](#rf-301)、[RF-003](#rf-003)。
- **入口：**`tauri/src-tauri/src/sync`；`tauri/src/stores/syncStore.ts`；`tauri/src/lib/ipc.ts`。
- **执行：**分开后端 wire DTO 与 syncStore 中的 UI 派生字段；同步命令、配对/进度/完成事件使用生成类型；保留现有协议和旧数据兼容。
- **验收：**本组命令/事件无重复 wire 类型；乱序/失效事件回归通过；配对、冲突和停机状态不丢字段。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**syncStore 与后端同步测试；生成契约检查。
- **建议提交：**`refactor(ipc): type sync commands and events [RF-305]`。

### RF-306

**迁移插件 IPC 与资源事件契约** · P2 · 来源：R18

- **前置：**[RF-301](#rf-301)。
- **入口：**`tauri/src-tauri/src/commands/plugin.rs`；`tauri/src/lib/plugin.ts`；`tauri/src/stores/pluginStore.ts`；`tauri/src/lib/ipc.ts`。
- **执行：**生成插件查询、安装、运行与取消命令及事件类型，准确表达 Resource/Channel 生命周期；保留会话、授权、沙箱和安装取消能力。
- **验收：**不将 Resource 当普通 JSON DTO；旧插件 manifest/运行结果兼容；取消、会话过期和权限拒绝用例通过。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**plugin.install、pluginStore 及后端插件测试。
- **建议提交：**`refactor(ipc): type plugin commands and lifecycle events [RF-306]`。

### RF-307

**建立结构化后端错误并迁移对象用例** · P2 · 来源：R18

- **前置：**[RF-301](#rf-301)、[RF-302](#rf-302)。
- **入口：**`tauri/src-tauri/src/commands/object`；`tauri/src/lib/backendError.ts`；`tauri/src/lib/backendError.test.ts`；`tauri/src/lib/ipcClient.ts`。
- **执行：**定义 code/safeDetails/retryable 错误包及旧字符串兼容适配，先迁移对象/回滚错误；翻译只在展示层完成，原始 cause 留在脱敏日志。LLM、备份导入导出、同步、插件由 RF-317～RF-320 分别迁移；本项不一次改全部错误。
- **验收：**对象已知错误无需匹配英文句子；未知错误安全回退；日志与 UI 不含字段原文/密钥；旧字符串调用仍可识别。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**错误包序列化/本地化/脱敏 fixture 和对象失败用例。
- **建议提交：**`refactor(errors): add structured object errors [RF-307]`。

### RF-308

**接入有实际执行证据的覆盖率门禁** · P2 · 来源：R21

- **前置：**[RF-316](#rf-316)。
- **入口：**`tauri/vitest.config.ts`；`tauri/package.json`；`.github/workflows/pr_check.yml`；`.github/workflows/ci_cd.yml`；本轮复现涉及 `tauri/src/lib/updater.test.ts` 与 `tauri/src/components/recovery/LazyRecoveryReceiveDialog.test.tsx`。
- **执行：**实际运行 coverage 获取当前基线，新增显式 coverage script/job；沿现有门槛检查，不只配置阈值而不执行。若当前达不到门槛，登记具体缺口任务并保持本项未完成，禁止直接降低数值换通过。去除重复 coverage 执行。
- **验收：**CI 执行 coverage 并上传报告；故意降低已覆盖风险分支会触发门禁；阈值来源与排除范围有记录。
- **验证配置：**`F` + `COVERAGE`。**定向验证：**npm run test -- --coverage；核对 CI job 的实际日志，不以 YAML 出现 threshold 为通过。
- **建议提交：**`ci(test): enforce measured frontend coverage [RF-308]`。

### RF-309

**建立 Windows Rust 关键用例执行门禁** · P2 · 来源：R21

- **前置：**无。
- **入口：**`.github/workflows/pr_check.yml`；`.github/workflows/ci_cd.yml`；`tauri/src-tauri/build.rs`；`tauri/src-tauri/bundles`。
- **执行：**增加 Windows 关键 Rust 测试实际运行 job，先完成 Vault/core 与会话/导入导出 Host 用例；检查测试进程 manifest、运行时库等必要条件，采用正式配置。不得把 DLL 启动失败记为测试通过，也不提交本机二进制绕过。
- **验收：**windows-latest 实际执行测试并上传失败日志；不是仅 cargo check 或打包成功；平台配置不改变 release 安全属性。
- **验证配置：**`R` + `CORE`。**定向验证：**Windows cargo test -p solosoul-vault -p solosoul-core 与 cargo test -p solo_soul；失败按运行环境/断言失败分类。
- **建议提交：**`ci(windows): execute Rust regression tests [RF-309]`。

### RF-310

**把 Android 原生回归接入明确的设备任务** · P2 · 来源：R21

- **前置：**[RF-201](#rf-201)、[RF-208](#rf-208)。
- **入口：**`.github/workflows/build-android.yml`；`tauri/src-tauri/gen/android/app/src/androidTest/java/com/solosoul/app/AndroidGlassInstrumentedTest.kt`；`tauri/src-tauri/gen/android/app/build.gradle.kts`。
- **执行：**为现有 Android instrumentation 提供可重复的模拟器/设备 CI 入口、系统镜像条件和结果附件；在测试机支持的 API/ABI 上执行。浏览器 mobile 测试继续保留，不用它代替原生任务。
- **验收：**能看到实际设备 ID、API、测试计数和失败截图；跳过/无设备不能算通过；玻璃与系统主题验证区分支持和回退环境。
- **验证配置：**`ANDROID_BUILD` + `ANDROID_NATIVE`。**定向验证：**按验证矩阵的 :app:connectedArm64DebugAndroidTest，或 assembleArm64DebugAndroidTest + adb instrumentation 执行 AndroidGlassInstrumentedTest；运行前通过 Gradle tasks --all 核实任务可用。
- **建议提交：**`ci(android): run native glass regression tests [RF-310]`。

### RF-311

**收敛重复 CI 步骤且保持平台覆盖** · P2 · 来源：R21

- **前置：**[RF-308](#rf-308)、[RF-309](#rf-309)、[RF-310](#rf-310)。
- **入口：**`.github/workflows/pr_check.yml`；`.github/workflows/ci_cd.yml`；`.github/workflows/build-android.yml`。
- **执行：**将重复前端/CLI 检查抽成可复用 workflow 或共用脚本，保持 PR 与 main 触发意图、Linux/macOS/Windows差异、iOS编译、submodule 校验和失败附件。发布步骤不混入此次迁移。
- **验收：**列出重构前后 job 对照表；相同检查不无谓重复；原有受保护检查名/依赖明确兼容；每个平台仍有原来的覆盖与新门禁。
- **验证配置：**`F` + `R` + `CLI` + `CONTRACT`。**定向验证：**实际 PR workflow 运行记录；仅 YAML 静态检查不足以关闭。
- **建议提交：**`ci: share validation workflows without losing coverage [RF-311]`。

### RF-312

**建立可重跑的性能基线与下一步决策** · P3 · 来源：R20

- **前置：**无。
- **入口：**`tauri/src/bootstrapApp.tsx`；`tauri/src/App/routes.tsx`；`tauri/src/lib/logger.ts`；`tauri/e2e`。
- **执行：**用合成小/大 Vault 建立启动、解锁、搜索、首次 OCR/预览、锁定恢复、内存和 IPC 次数测量；记录设备/构建/样本量和采样口径，输出基线文档。测得瓶颈才新增具体优化 ID，当前任务不改全路由加载策略。
- **验收：**他人按记录能复跑并得到同口径数据；有重复样本与中位/尾部指标，失败样本不丢；明确是否值得懒加载/分页及其证据。若无瓶颈可完成测量项，不制造优化提交。
- **验证配置：**`PERF` + `NATIVE`。**定向验证：**启动/解锁/搜索测量脚本与多端实测；固定数据集种子、构建模式和可重复步骤。
- **建议提交：**`docs(perf): record reproducible application baselines [RF-312]`。

### RF-313

**修正 canonical 架构与安全事实文档** · P2 · 来源：R22

- **前置：**无。
- **入口：**`AGENTS.md`；`docs/attachment-storage-spec.md`；`docs/design_map/10_跨平台视觉规范与主题系统.md`；`docs/solosoul_cli/USER_GUIDE.md`。
- **执行：**修正 crates 路径、过时 Go API/会话描述、原生材质状态；明确新附件加密与旧明文兼容的区别，撤销尚不成立的 GUI/CLI 1:1 声明并链接对应待修任务。只描述当前证据，不把待实现能力写成已完成。
- **验收：**文档中的入口路径存在；安全描述可对应当前实现；旧数据是否迁移有清楚边界；历史报告保留日期不改为新事实。
- **验证配置：**`DOC`。**定向验证：**逐条核对代码/引用；文档链接与路径检查；无需业务测试。
- **建议提交：**`docs(architecture): align current implementation facts [RF-313]`。

### RF-314

**建立平台能力与验收证据矩阵** · P2 · 来源：R21、R22

- **前置：**[RF-205](#rf-205)。
- **入口：**`docs/design_map`；`docs/solosoul_cli/USER_GUIDE.md`；`.github/workflows`。
- **执行：**在现有架构文档内登记功能→入口→共享用例→支持平台→验证方式/日期；区分已实现、编译通过、浏览器模拟、设备验证和待实现。将测试/实现链接作为证据，不另建竞争的长期规范。
- **验收：**macOS/Windows/Android/iOS及Linux回退状态可查；每条“跨端一致/原生支持”均有证据或明确未验证；后续任务定义同步更新位置。
- **验证配置：**`DOC`。**定向验证：**抽查主题/OCR/更新/回滚/备份/玻璃六条能力链及引用。
- **建议提交：**`docs(platform): track capabilities and verification evidence [RF-314]`。

### RF-315

**对齐 LLM 数据流与隐私说明** · P2 · 来源：R03、R22

- **前置：**[RF-100](#rf-100)、[RF-004](#rf-004)、[RF-005](#rf-005)。
- **入口：**`docs/design_map/20_LLM配置与AI对话规范.md`；`docs/legal/隐私政策.md`；`docs/legal/服务条款.md`；`docs/legal/Privacy Policy.md`；`docs/legal/Terms of Service.md`；`tauri/src/locales/zh-CN/settings.json`；`tauri/src/locales/en-US/settings.json`；`tauri/src/components/llm-config/RiskAcceptanceDialog.tsx`；`tauri/src/pages/ai/LlmConfigPage.tsx`。
- **执行：**核对实际本地/远程 provider 模式、用户输入和自动附加字段的来源与去向；更新现有产品/隐私说明及风险确认的中英 UI 文案，使其与字段筛选和凭证边界一致。保留用户主动选择远程服务的流程，不声称所有启用模式都不外传任何内容；以当前 docs/legal 为准，不创建已过时文档中声称存在的 zh-CN/en-US 副本。
- **验收：**声明可逐项对应发送链路；不虚构第三方服务的数据保留政策；中英文含义一致；旧附件/备份等无关声明不混入。
- **验证配置：**`DOC`。**定向验证：**对照捕获的合成出站 fixture 审查文案；若修改 TSX 再执行 F。
- **建议提交：**`docs(privacy): describe actual LLM data flows [RF-315]`。

### RF-316

**诊断并稳定默认前端测试运行入口** · P2 · 来源：R21

- **前置：**无。
- **入口：**`tauri/vitest.config.ts`；`tauri/package.json`；`.github/workflows/pr_check.yml`；`.github/workflows/ci_cd.yml`。
- **执行：**在相同依赖和限定超时下复现默认 pool 未结束与 threads 成功的差异，区分发现范围、worker、未释放句柄和本机环境；修复确认的配置/测试生命周期原因，必要时把有证据的平台适配写入脚本。不能仅因一次挂起全局改 runner。
- **验收：**默认 npm run test 在干净环境可靠结束，测试文件/用例不因修复减少；不能复现时记录环境和有限复现证据，保留待复现状态。只有证据足以说明原问题已解决或仅为已排除的环境问题时关闭，不能以一次线程池成功认定默认入口已修复。
- **验证配置：**`F`。**定向验证：**对照 npm run test -- --maxWorkers=2 与 npm run test -- --pool=threads --maxWorkers=2 的退出码/用例集合；只结束本次启动的进程。
- **建议提交：**`test: stabilize the frontend test runner [RF-316]`。

### RF-317

**迁移 LLM 结构化错误** · P2 · 来源：R18

- **前置：**[RF-303](#rf-303)、[RF-307](#rf-307)。
- **入口：**`tauri/src-tauri/src/commands/llm`；`tauri/src/lib/backendError.ts`；`tauri/src/hooks/useLlmStreaming.ts`。
- **执行：**将 provider/发送/持久化失败错误迁到结构化错误包；区分流失败与回复已生成但保存失败，保留旧前缀兼容读取，敏感响应细节仅在脱敏日志中记录。
- **验收：**模拟网络、provider拒绝、会话失效和保存失败时 UI 行为与真实状态一致；不用匹配英文正文；旧错误 fixture 仍可解析。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**LLM 请求/流失败与 backendError 测试。
- **建议提交：**`refactor(errors): type LLM failures [RF-317]`。

### RF-318

**迁移备份与导入导出结构化错误** · P2 · 来源：R18

- **前置：**[RF-304](#rf-304)、[RF-307](#rf-307)。
- **入口：**`tauri/src-tauri/src/commands/backup.rs`；`tauri/src-tauri/src/commands/export_import`；`tauri/src/lib/backendError.ts`；`tauri/src/hooks/useImportState.ts`。
- **执行：**迁移格式、密码、范围、写入与恢复错误；错误码和 ImportOutcome 分开，不能把部分提交状态压成普通 error string；保留旧包格式与旧前缀适配。
- **验收：**用户能区分未提交失败/部分完成/可重试；文件路径和明文数据不进入不必要提示；旧兼容 fixture 不变。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**导入失败阶段 fixture、备份坏包与 UI结果测试。
- **建议提交：**`refactor(errors): type transfer failures [RF-318]`。

### RF-319

**迁移同步结构化错误** · P2 · 来源：R18

- **前置：**[RF-305](#rf-305)、[RF-307](#rf-307)。
- **入口：**`tauri/src-tauri/src/sync`；`tauri/src/stores/syncStore.ts`；`tauri/src/lib/backendError.ts`。
- **执行：**将同步连接、握手、配对、冲突与会话失效错误映射为稳定 code/safeDetails；底层 IO错误保持cause，UI仅使用可本地化字段；保留既有配对状态机。
- **验收：**超时/拒绝/等待配对/锁定被区分；SAS与节点标识处理不回退；旧事件/前缀在兼容期可读。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**syncStore配对/错误测试与后端握手失败fixture。
- **建议提交：**`refactor(errors): type synchronization failures [RF-319]`。

### RF-320

**迁移插件结构化错误** · P2 · 来源：R18

- **前置：**[RF-306](#rf-306)、[RF-307](#rf-307)。
- **入口：**`tauri/src-tauri/src/commands/plugin.rs`；`tauri/src/lib/plugin.ts`；`tauri/src/stores/pluginStore.ts`；`tauri/src/lib/backendError.ts`。
- **执行：**将权限拒绝、会话过期、校验失败、执行失败与取消转成稳定错误码；保留安装资源取消和WASM约束，不把用户取消提示成执行错误。
- **验收：**四类错误及取消可正确区分；不泄露字段/密钥；旧插件接口和结果schema保持兼容。
- **验证配置：**`F` + `R` + `CONTRACT`。**定向验证：**插件安装取消、会话隔离、授权拒绝与backendError测试。
- **建议提交：**`refactor(errors): type plugin failures [RF-320]`。

### RF-900

**CLI 中文断言测试显式隔离系统语言** · P2 · 来源：本轮执行基线新增（不属于原 R01–R22）

- **前置：**无；影响包含 CLI 配置的 Rust 任务验收，应优先修复。
- **入口：**`solosoul_cli/src/commands/ocr.rs`、`profile.rs`、`security.rs`、`template.rs`、`backup.rs` 中的测试 fixture；参考 `solosoul_cli/src/app.rs` 的 `App::new` / `detect_initial_locale`。
- **证据：**2026-09-25 英文 Windows 上 CLI lib 150 passed / 14 failed。其中 13 项断言中文字符串，但 fixture 调用 App::new 自动选择系统语言；OCR 失败输出明确为英文 `unknown flag` / `rejecting extra argument`。另 1 项缺少 sqlite3 CLI，作为环境前置单独记录，不与此修复混合。
- **执行：**让断言中文业务文案的测试 fixture 显式选择中文；保留产品自动检测系统语言的行为，避免修改进程全局语言或扩大串行锁。确认独立的语言检测/切换测试仍覆盖中文和英文。
- **验收：**上述 13 项在英文系统上通过；不修改生产语言策略，不把断言删除或仅改成 `is_some()`。
- **验证配置：**`CLI`。先运行 5 个 commands 模块的测试，随后完整 CLI fmt、Clippy 和 test；`sqlite3` 环境缺失未解决时仍记录全量检查受阻，不冒充全绿。
- **建议提交：**`test: resolve [RF-900] - isolate CLI command fixtures from system locale`。

### RF-901

**Windows GUI Rust 测试嵌入 Common Controls 清单** · P2 · 来源：本轮执行基线新增（不属于原 R01–R22）

- **前置：**无；RF-011 提交后优先恢复 R 验证能力，再返回 RF-001。
- **入口：**`tauri/src-tauri/build.rs`；Windows 构建资源；现有 tauri-build/tauri-winres 的资源链接输出。
- **证据：**原 GUI 库测试程序无资源区/嵌入清单，导入 `comctl32!TaskDialogIndirect`，启动返回 0xc0000139。现有资源编译只输出 `cargo:rustc-link-arg-bins`，未覆盖库测试。将原产物复制成单独诊断副本并用 mt 嵌入 Tauri 默认 Common Controls v6 清单后，`--list` exit 0，列出 485 tests；原产物未改。外部旁路清单未生效，故不能依赖本机临时文件解决。
- **执行：**让正常 Windows Cargo 构建的库测试产物携带所需 Common Controls v6 清单；保留应用的图标、版本资源与其他平台构建，不修改系统 DLL、注册表、测试二进制或依赖缓存作为产品修复。核对链接范围，避免重复嵌入资源。
- **方案探针：**独立零依赖微型工程验证：在原资源同时包含 manifest 时加 `/MANIFEST:EMBED` 会报 CVT1100 重复资源；保留图标/版本、去除原资源中的 manifest，再统一通过 MSVC `/MANIFEST:EMBED` + `/MANIFESTDEPENDENCY` 嵌入，则库测试 1/1、应用运行均 exit 0，ProductName/ProductVersion/FileDescription 保持。正式实现可在 Windows MSVC 目标使用 `WindowsAttributes::new_without_app_manifest`，其他目标仍保留 Tauri 原行为；此探针不替代正式 R 验收。
- **验收：**清理诊断产物后，正常 Cargo 生成的 GUI 测试程序可自行列出并运行全部测试；应用资源保持；R 全量检查取得真实结果，失败用例按实际原因处理，不裁剪测试。
- **验证配置：**`R` + `DOC`。定向先 `cargo test -p solo_soul --lib -- --list`，核验生成二进制的 manifest，再执行 `cargo test --verbose`；Windows 应用目标构建/资源读取检查，其他目标至少确认条件编译范围。
- **建议提交：**`build: resolve [RF-901] - embed Windows manifest in GUI Rust tests`。

### RF-902

**生产启动冒烟使用当前桌面更新契约** · P2 · 来源：执行中生产包验证新增

- **前置：**无；与 RF-003 云同步代码无关，独立提交。
- **入口：**`tauri/e2e/production-startup.spec.ts`；对照 `src/lib/updater.ts` 与 `@tauri-apps/plugin-updater` 的实际 Update 构造参数。
- **证据：**生产检查 13 passed / 1 failed；启动、路由与其他更新说明用例正常，旧夹具只模拟 desktop_check_update，实际调用 desktop_prepare_update 得到 undefined 后显示 Latest，预期 Markdown 未渲染。
- **执行：**改为模拟 desktop_prepare_update；仅 /about 返回带 rid/currentVersion/version/body/rawJson 的可用更新，其他路由返回 null。保留现有生产入口、启动、导航、Markdown 与 pageerror 断言，不改生产更新实现或超时。
- **验收：**真实 Vite 生产包中关于页展示 Release smoke 标题及粗体列表，全部 production 配置用例通过。
- **验证配置：**`WEB` + `DOC`；修改文件 Prettier check，`SOLOSOUL_E2E_CHANNEL=chrome npm run test:e2e:production`，记录本机 Chrome 与实际计数。未改前端生产源码，不重复已通过的 F 或启动另一套 Rust 构建。
- **建议提交：**`test: resolve [RF-902] - align production smoke with desktop update contract`。

### RF-903

**孤儿附件清理保护账户归属与完整恢复引用** · P1 · 来源：执行中真实回归

- **前置：**[RF-905](#rf-905)。RF-016 的已授权删除意图可复用执行设施，但不能替代孤儿候选的归属和引用证明。
- **现状：**Core cleanup 扫描所有账户共用的 `attachments/<object>/<attachment>`，仅排除当前账户活对象附件。真实同包导入 A/B 后对象 ID 相同、附件 UUID 不同；A 清理删除了 B 有效文件并返回 `Ok((2, 0))`。回收站/历史/冲突中的可恢复引用也未收全。
- **入口：**`tauri/crates/solosoul-core/src/objects.rs`；`tauri/crates/solosoul-vault/src/storage/`；`solosoul_cli/src/commands/attachment.rs`；Host 导入回归。
- **执行：**在可靠排他维护窗口中捕获原会话；无截断读取当前/软删对象、全部对象/页面回收站、全部历史及可重新应用的未决同步冲突。严格解密解析，坏数据不得降级为空；同时保护 ID 和路径别名。仅对可证明属于当前密钥域的候选执行清理；非空 SOLC v2 可复用完整 AEAD 认证并校验头、分块数和长度，冻结已校验头，限制读取内存。旧明文、零分块、其他密钥、损坏及归属不明文件保留并准确计数。逐文件/完整认证目录处理，拒绝 symlink/reparse；最终核对会话、数据库视图、文件身份及引用，失败保留可追踪状态，不虚报释放空间。
- **验收：**保留安全清理真实历史加密孤儿的能力，不能以关闭 cleanup 或只处理新意图代替。A/B 共用对象目录、回收站第501项、历史第51项、同步冲突、恢复换ID、零分块/伪头/截断/尾部追加、链接、文件失败、会话失效均覆盖；文件已发布但元数据尚未登记时，清理必须被可靠互斥阻止。外部任意位置的旧 metadata-only 导出包不可枚举，不能承诺其旧路径永久可用。
- **验证配置：**`R` + `CORE` + `CLI`，Windows 真实临时文件/占用/目录连接及跨进程互斥。已保存的 rf903 Host 回归不得删除断言以换绿。
- **建议提交：**`fix: resolve [RF-903] - preserve owned and recoverable attachments during cleanup`。

### RF-905

**GUI 与 CLI 遵守同一数据目录互斥与维护窗口** · P1 · 来源：RF-903 前置核查

- **前置：**无。此项独立完成后再实施 RF-903 的自动清理。
- **现状：**CLI App 获取 ProcessLock 失败只警告继续；GUI 未持有该锁。不能依靠现有锁排除“文件已发布、元数据待登记”的写入窗口。
- **入口：**Core `process_lock.rs` / `vault_service/`；GUI `state/saf_config.rs`、`state/app_state.rs`、`commands/vault_directory.rs` 及启动入口；CLI 构造、doctor、状态栏和后台任务收尾。
- **执行：**数据目录所有者持锁至该目录的所有任务和句柄释放，锁定真实 Vault 根而非 AppData 配置根；GUI/CLI 获取失败均明确停止该目录写入，不静默切换其他目录。收敛构造和热替换生命周期，避免 CLI 二次加锁自撞和 doctor 对本进程持锁误报。维护窗口须等待附件发布任务结束，不能将 abort 未 join 当作任务已经静止。移动端现有单实例/no-op 边界保留并说明，不宣称能锁远端 SAF 提供者。
- **验收：**独立子进程真实竞争同一目录时只一方可写；GUI与CLI两种启动顺序、失败不写、释放后恢复、目录切换失败保留原状态、同步/导入在途拒绝进入维护均覆盖。不得用同一线程第二次 acquire 代替跨进程证据。
- **验证配置：**`R` + `CORE` + `CLI`，本机 Windows 子进程与 GUI 状态入口；变更前端错误交互时另加 `F`/生产启动验收。
- **建议提交：**`fix: resolve [RF-905] - enforce shared vault directory ownership`。

## 7. 每项执行记录模板

选中任务时填写“当前处理”，完成后在本节按 ID 追加记录，并更新索引中的状态和统计。报告状态更新与本项代码/测试放入同一提交；不要以未运行的上轮测试作为本次验收证据。

### 2026-09-25 执行基线

- 环境：Windows x64，Node `v24.16.0`，Cargo `1.96.0`；分支 `main`，起点 `f77c0e20`。两份调查/执行报告已独立提交为 `e20afe85`。
- 保留原有未提交的 `tauri/src-tauri/Cargo.toml`、3 张 NSIS BMP、`resources/docs/guides/search-index.json`；不纳入修复提交。CLI 基线运行自动刷新了 5 个本地 crate 的 lockfile 版本，已仅撤销这部分工具副产物，未升级依赖。
- `tauri/npm run check-all` 与 CLI 检查已启动。CLI 首次 Clippy 因 Cargo 缓存目录写权限失败（exit 101），在获得执行权限后重试。Tauri TypeScript、Rust fmt/Clippy 和 CLI fmt/Clippy 已通过，Rust 测试结果另行回填。
- 合约检查 exit 0：ACL 219 个命令、偏好键 22 个、Markdown 分块边界 13 个依赖。
- 首次定向 Vitest 在线程工作进程启动阶段超时（exit 1，0 tests）；直接调用同一 Vitest 入口重试后 6/6 通过。保留此环境失败，不声称默认 runner 稳定性已修复（RF-316）。
- CLI Clippy 通过后执行 `cargo test --verbose --no-fail-fast`：lib 150 passed / 14 failed。13 项中文断言与系统英文语言不一致，登记 RF-900；`test_backup_create_aborts_on_unreadable_profile` 因缺少 `sqlite3` 命令失败，不认定为数据备份行为缺陷。
- CLI 全部测试已结束，exit 101：合计 152 passed / 14 failed / 1 ignored（lib 150/14，向导集成 2/0，文档示例 1 ignored）。没有将该全量检查登记为通过。
- 两条 Tauri 基线命令均已完成 Clippy；第二条仅等待第一条的 Cargo test 构建锁、没有编译子进程时已取消（exit -1），避免重复构建。第一条继续保留原始测试结果。
- 原始 Tauri 全量基线已结束：Rust 测试编译完成（30m27s），GUI 测试程序 `solo_soul-a1179f00737fbea1.exe` 在运行用例前报 `0xc0000139 / STATUS_ENTRYPOINT_NOT_FOUND`，shell 返回 `-1073741511`。没有获得 GUI Rust 用例通过证据；RF-001 所需 R 配置仍受阻。禁止用只做 cargo check 或修改测试二进制来声称通过。
- 阻塞解释：已确认失败发生在测试程序启动阶段，不是测试断言失败，也不涉及等待用户授权。具体缺失的运行库入口尚未定位，不能据此认定某个 DLL 有问题。下一步需检查该测试二进制实际加载的依赖及导出符号，修复后重跑 R；此前只暂缓需要该验收配置及相关依赖的任务，其他任务继续执行。
- RF-007 前复核：直接对原 GUI 测试二进制执行 --list，沙箱内进程存活但无输出，核实 PID/完整命令后仅结束该只读探测（未运行 Cargo）。随后在正常主机权限下设置父进程 SetErrorMode(0x8003) 避免加载错误弹窗，再执行同一 --list，真实退出码仍为 -1073741511 / 0xc0000139。因此 RF-006 的 R 验收同样暂缓。dumpbin 确认依赖 DirectML/VC++；系统 DirectML 导出包含所需 DMLCreateDevice1，不能把失败归因于缺少该导出。尚未定位具体入口，未改测试程序或系统运行库。

### RF-900 执行记录

- 修复前 HEAD：`3ae797ca`；2026-09-25，英文 Windows。
- 仅修改 backup / ocr / profile / security / template 的测试 fixture：在各自 `App::new` 后显式 `app.i18n.set_locale("zh-CN")`。生产 App 构造、系统语言探测、翻译及断言均未改动，不使用进程全局语言变量。
- CLI `cargo fmt --check` 和 `cargo clippy --all-targets -j 1 -- -D warnings` 已通过（exit 0）。完整 CLI 测试进行中，限制 `-j 1` 减少链接内存竞争。
- 补齐环境前置：从 [SQLite 官方下载页](https://www.sqlite.org/download.html) 获取 Windows x64 tools 3.53.4，SHA3-256 `88b4659fe747896b853af10157316b4ade143553efb89c1c8ca7423a278dcc8b` 与官网元数据一致后解压至 `solosoul_cli/target/rf-test-tools`，仅对测试进程 PATH 生效，没有系统安装。该目录是被忽略的测试依赖缓存，不入库；后续 CLI 验证须在 PATH 包含此目录或已有 sqlite3 CLI 的环境执行。
- 验证命令（`solosoul_cli/`）：`$env:PATH = "$PWD/target/rf-test-tools;" + $env:PATH`，随后 `cargo test -j 1 --verbose --no-fail-fast`。不跳过原先因缺 sqlite3 失败的损坏 profile 测试。
- 完整结果：exit 0，lib 164/164、向导集成 2/2，通过合计 166；1 个原有文档示例 ignored。中英文初始化及 `test_set_locale_switch` 均通过；13 个语言相关失败与损坏 profile 测试全部通过，未增加 skip 或放宽断言。仅恢复 Cargo 自动刷新的 5 个 workspace crate 版本，无 lockfile 变更纳入提交。
- 结论：**完成**；提交 `81acdc77`，未推送。

### RF-316 执行记录

- 修复前 HEAD：`81acdc77`。先处理 RF-100 全量验收中重复出现的 2 个失败文件；没有修改 Vitest pool、worker 配置、CI 或任何超时值。
- 确认原因：`ensureApkDownloaded` 在创建操作前动态导入真实 notification 模块，下载测试没有隔离系统权限及其 store 依赖，导致 waitFor 超时后旧异步操作进入后续测试。该模块现用异步权限成功 fixture 隔离，保留所有下载取消/释放/失败断言。
- 恢复弹窗测试原本既依赖冷模块加载在 1 秒内完成，又依赖摄像头能力探测尚未完成时的临时扫码 tab。现在先 render 触发懒加载，再通过 `act + vi.dynamicImportSettled` 等待真实模块与 React 更新；显式固定摄像头 supported，仅 mock 扫码视图，保留真实对话框、useRecoveryReceive 和命名导出映射断言。
- 定位过程中定向测试仍发现“加载已完成但已切换手动 tab”的失败，未删除扫码视图断言，改为明确其硬件 fixture。最终 `node node_modules/vitest/vitest.mjs run src/lib/updater.test.ts src/components/recovery/LazyRecoveryReceiveDialog.test.tsx --pool=threads --maxWorkers=2 --reporter=verbose` exit 0，2 文件/17 测试，无跳过，2.02s。
- 正在使用同一依赖比较默认 forks（`npm run test -- --maxWorkers=2 --reporter=dot`）与 threads，并保留原始默认入口的验收；未将单次定向通过当作 runner 已稳定。
- 默认 forks/2 的首次完整结果：140 文件启动，139 文件通过、1 文件失败，1,147 passed / 1 failed；剩余 PhotoAlbumOverlay 的冷查看器加载超过其原有 8 秒交互等待。该文件现预载真实 PhotoViewerOverlay 模块以隔离 Vite/framer-motion 冷编译耗时，lazy 导出和查看器交互保持真实，未增加 8 秒/12 秒超时值。该文件在 forks/2 定向重跑 12/12 通过（3.98s）。本项实际范围追加 `tauri/src/components/attachment/PhotoAlbumOverlay.test.tsx`。
- 最终代码的 `npm run test`（没有 pool、worker 或 reporter 参数）exit 0：140 文件、1,148 测试全部通过，无跳过/worker 错误，71.20s 正常退出。新修改文件定向 ESLint、完整 TypeScript 与 Prettier check 均 exit 0；先前完整 Lint 已通过且未修改其他前端文件。threads 全量对照仍在运行。
- 对照完成：`npm run test -- --pool=threads --maxWorkers=2 --reporter=dot` exit 0，140 文件/1,148 测试全部通过，无跳过/worker 错误，129.83s；用例数量与默认入口一致。原始内存压力下的 worker 启动超时仍保留在历史记录，未将所有环境问题归因于测试代码。结论：**完成**，已修复确认的测试依赖/生命周期问题并恢复默认完整入口；无证据需要切换全局 runner 或修改 CI。独立提交 `e6743095`，未推送。

### RF-100 执行记录

- 开始：2026-09-25；修复前 HEAD：`e20afe85`。
- 已确认：旧 `buildSection3PublicObjectData` 仅筛选对象级 public，随后截取前 8 个属性并直接 String 转换，没有字段级过滤。
- 修改：新增纯 `resolveFieldSensitivity` helper；按标签、对象字段定义、已加载模板的优先级解析，显式非法值与未知字段默认 internal。自动上下文只接受公开叶值，递归过滤动态组；排除内部键和无类型嵌套对象，父级保护不能被子项降低，深度上限 16。先过滤再截断，不修改 store 和原始对象。
- 回归：新增 `systemPromptBuilder.test.ts`，调用最终 `buildChatRequestMessages`，使用 mock 指南服务与合成数据，无外部 API 调用。6 项覆盖四等级/未知等级、优先级冲突、已删除模板、嵌套动态组、未识别结构、过滤后数量限制与对象不可变。
- 已通过：`tauri/npx tsc --noEmit`、`npm run lint`，exit 0；定向 `node node_modules/vitest/vitest.mjs run src/lib/llm/systemPromptBuilder.test.ts --pool=threads --maxWorkers=2 --reporter=verbose`，exit 0，1 文件/6 测试，无跳过；修改的 3 个 TS 文件已单独 Prettier 格式化。
- 规范：`docs/design_map/20_LLM配置与AI对话规范.md` §6.6。现有 `propertyFlatten` 与回收站显示行为未改动；本项不扩大出站授权、不变更手工消息，Rust 权限投影仍由 RF-004 承接。
- 初次提交结论为待验证；RF-316 修复测试基线后，默认与 threads 完整前端检查均已通过（140 文件/1,148 测试），其余 F/DOC 检查已通过。最终结论：**完成**。修复独立提交 `3ae797ca`，本轮不推送。
- 首次全量前端结果：`node node_modules/vitest/vitest.mjs run --pool=threads --maxWorkers=2 --reporter=dot`，exit 1，134 文件通过 / 2 文件失败，1,111 测试通过 / 3 失败，另 4 个工作进程启动超时。失败文件为 `updater.test.ts`（2 项）、`LazyRecoveryReceiveDialog.test.tsx`（1 项）；未启动文件为 Button、SensitivityBadge、exportFormat、syncPeer 的测试。正在串行复跑这 6 个文件以区分负载超时与实际缺陷，保留首跑失败记录。
- 串行复跑命令：`node node_modules/vitest/vitest.mjs run src/lib/updater.test.ts src/components/recovery/LazyRecoveryReceiveDialog.test.tsx src/components/ui/Button.test.tsx src/components/ui/SensitivityBadge.test.tsx src/lib/exportFormat.test.ts src/lib/syncPeer.test.ts --pool=threads --maxWorkers=1 --reporter=verbose`；exit 1，4 文件/34 测试通过，恢复弹窗 1 项仍在 1 秒断言窗口超时，updater 工作进程启动超时。4 个首跑未启动文件已得到通过证据；不把重跑描述为全绿，也不提高超时或删除断言。RF-316 优先排查 runner/资源争用及异步测试稳定性，再补 RF-100 全量验收。
- Tauri 编译结束后再次全量重跑（同 threads/2 命令）：exit 1，138 文件通过 / 2 失败，1,144 测试通过 / 4 失败；140 个文件均启动，没有 worker 启动错误。剩余 updater 3 项及 LazyRecoveryReceiveDialog 1 项，因此不能仅将所有失败归因于内存压力；需 RF-316 核对异步依赖和测试生命周期。RF-100 提交 SHA：`3ae797ca`。

```markdown
### RF-xxx 执行记录
- 开始/结束时间、分支与修复前 HEAD：
- 原问题是否仍存在；复现或静态证据：
- 实际修改范围与行为变化：
- 验证：工作目录、完整命令、退出码、用例数量、跳过项：
- 平台证据：OS/设备/API/构建、操作步骤、结果或阻塞原因：
- 兼容与恢复：旧格式/旧入口是否兼容；失败后数据状态：
- 规范更新位置：
- 剩余问题或新增任务：
- 结论：完成 / 排除 / 待验证 / 阻塞：
- 提交：本提交（标题含 RF-xxx，后续补 SHA）；推送结果（仅在已授权时）：
```

后续实施时的 Git 步骤（命令中的路径替换为该 ID 的实际文件，不能原样执行占位符）：

```text
git diff --check
git diff -- <当前任务路径>
git add -- <当前任务路径> docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md
git diff --cached --stat
git diff --cached
git commit -m "<任务卡的提交标题>"
```

同文件含其他未提交工作时使用补丁级暂存或隔离工作树，不把整个文件无差别加入。提交失败保持当前任务状态并解决实际原因；推送失败保留本地提交及错误，不 amend/force push 掩盖历史。只有届时明确授权推送时，才推送已核对的远端与分支。

## 8. 最终复审与新增任务

1. 核对每个完成 ID 都有独立提交、验证记录和实际关闭结论。未获设备验证的任务保留待验证；“已关闭”“实际修复”“排除”分别计数。
2. 对照第 5 节映射，确认 R01–R22 的每个子目标均有归宿。保留未迁移适配器时必须列出调用者和独立后续 ID，不能用“后续再做”关闭涵盖它的任务。
3. 重新检查账户切换、字段出站、GUI/CLI 互操作、文件/数据库提交失败、平台能力及原生主题；运行 F/R/CLI/CONTRACT、相关 WEB 与可用原生矩阵。扩大测试只针对最终集成和新增疑点。
4. 新问题使用 RF-900 起的新 ID，包含同样的入口、依赖、验收与提交要求；维护映射和计数，不复用旧 ID。严重程度高的新增问题优先进入队列。
5. 形成新的 `docs/REFACTOR_EXECUTION_REPORT_FINAL_YYYY-MM-DD.md`，记录本轮修复基线、最终 HEAD、已关闭/待验证/排除项、平台覆盖和残余限制；保留旧审查报告。
6. 尚有 P1、失败检查或必需原生验收缺口时，不使用“所有问题已修复/审计通过”等措辞，也不打通过标签。性能测量无瓶颈时可完成测量任务，不凭空追加优化实现。
7. 推送、标签和发布按届时用户授权执行；本执行台账不把“完成复审”自动等同于批准发布。

## 9. 本轮编制验证

本轮只编写执行报告，不运行修复，不执行业务测试，不提交/推送。交付前校验任务 ID 唯一、依赖存在且无环、全部 R01–R22 有映射、索引与任务卡计数一致、源文件路径可定位及新增文件有标注；保留调查基线而不冒充重新验证。

编制校验已通过：90 个唯一任务 ID、90 行执行索引、22 项来源映射；所有依赖均先于依赖者出现在推荐顺序中，无环或悬空引用；任务卡字段齐全，现有入口路径均可定位，计划新建文件明确标注。已复核 Android/iOS 命令与平台关闭条件，未把环境阻塞或跳过测试记作通过。

编制过程中识别出 Android Gradle 的机器专属 JDK 路径这一执行障碍，单列 RF-208；调查时默认 Vitest pool 未结束的问题单列 RF-316。两项均要求后续核实和验收，不在本轮修改。

工作树原有的 Cargo 配置、NSIS 图片和搜索索引改动继续保留；前一轮调查报告也保留。本轮专用任务合并临时文件在交付前删除。

### RF-101 执行记录

- 修复前 HEAD：`e6743095`。`useLlmChatCore.sendMessage` 原先将已追加输入的 `updatedMessages` 传给仍会追加输入的 builder，开启/关闭系统提示均重复发送。
- 修复：传入追加前的 `messages`；UI、首次持久化继续使用 `updatedMessages`，不改动流归属、保存流程或协议。明确两个 builder 的 history 契约，并更新 LLM 规范 §6.7。
- 回归：新增 `chatRequest.test.ts` 的 8 项测试，组合系统提示开启/关闭、空历史/多轮历史，既检查真实 builder 的完整序列、指南合并、历史不可变，也用真实 hook 与 builder 捕获最终 IPC 请求、UI 消息和首次保存。系统 IPC/指南/流监听采用合成 fixture，无外部发送。
- 修复前：测试 fixture 初次加载因 i18n mock 缺依赖失败（0 tests），补齐 store 隔离后 4 个 builder 测试通过、4 个真实 hook 测试稳定复现重复输入。修复后定向执行 chatRequest 与 systemPromptBuilder，2 文件/14 测试全部通过，exit 0。
- F 验证：TypeScript（本地 `node node_modules/typescript/bin/tsc --noEmit`）、`npm run lint` 均 exit 0；4 个修改的 TS 文件已单独 Prettier 格式化；默认 `npm run test` exit 0，141 文件/1,156 测试全部通过，无跳过或 worker 错误，75.69s。`git diff --check` 和报告 ID/索引一致性检查通过。
- 结论：**完成**；独立提交 `d8fe12b3`，未推送。原有 Cargo 配置、NSIS 图片与搜索索引改动未纳入。

### RF-102 执行记录

- 修复前 HEAD：`d8fe12b3`。确认 SearchPage/Popover 的异步 helper 直接写结果和缓存；旧请求 finally 能结束新查询 loading，页面清空遗漏 debounce，筛选不取消旧计时器。
- 修复：新增共享 useUnifiedSearch，输入/清空/筛选时同步取消计时器并失效旧查询；复用 createSessionRequests，通过 request.invoke 在 IPC 鉴权后再次校验。查询函数只返回结果，由 hook 验证当前请求后提交缓存、结果、错误和 loading。缓存自身订阅会话清理；sessionRequests 的订阅新增注销函数，组件卸载清理监听及请求，不泄漏订阅。
- 两个 UI 入口统一使用该 hook；保留系统页面名匹配、自定义页筛选、结果展示和点击行为。清空关键词但保留筛选时立即按现有筛选重查。前端架构规范 §5 已更新。
- 定向验证：共享 helper、SearchPopover 与 hook 首轮 3 文件/30 测试通过（exit 0）；覆盖 A/B 倒序、debounce 窗口旧错误、清空、筛选、锁定换账户、卸载、缓存与当前失败。复审后强化卸载断言并新增“同账户重新解锁、两个入口并存”，最终 hook 单独 9/9 通过（exit 0），未减少原场景。
- F：完整 TypeScript、完整 ESLint、默认 `npm run test` 均 exit 0；全量 142 文件/1,165 测试通过，无跳过、无 worker 错误，112.90s。此后仅补强测试，已定向重跑 9/9，并重做 TypeScript 和修改测试文件 ESLint；不把新用例伪计入先前全量数字。修改 TS/TSX/E2E 文件单独 Prettier 格式化，git diff --check 通过。
- WEB：本机 Chrome，`playwright test e2e/sidebar-tools.spec.ts --project=chromium --grep '侧栏打开搜索' --workers=1`，4/4 passed、exit 0；用例结束后本次 Vite 服务未自动退出，核对 PID/命令/父进程后用 .NET Process.Kill 清理该服务（Stop-Process 自身报内部错误），runner 正常输出最终摘要，未杀测试 worker。新增 `search-lifecycle.spec.ts` 在 chromium/mobile 各 1/1，通过真实页面操作与合成 IPC deferred 响应验证乱序及清空，exit 0（13.3s）。仅证明浏览器交互，不声称 Android 原生测试。未改启动/路由定义、IPC 命令契约或主题，因此不触发生产启动额外门禁。
- 结论：**完成**；独立提交 `0279100d`，未推送。Rust 基线阻塞保持原记录；无关原有改动保留。

### RF-103 执行记录

- 修复前 HEAD：`0279100d`。正文没有时序守卫；页面列表绕过 core 的部分取消；快捷入口 await 旧读取后仍把旧 ID 写入 localStorage，回收站读取同样没有会话保护。
- 修改：core 正文/list 使用独立 sessionRequests key；新建、发送时设置会话 ID 会失效旧正文；关闭/卸载使所有读取失效。账户会话变化同步清空本地正文、列表和输入。页面列表刷新复用 core.loadConversationList，回收站 list/body 分别保护，关闭预览、切回正常会话及新建会话都失效旧预览。
- 快捷入口只在 core 已接纳当前会话后记忆 ID；移除 await 后写旧 ID，恢复 effect 随账户变化重新读取，关闭动作在卸载前失效请求。保留在线状态 AbortController，不再与列表读取共用。LLM 规范 §6.7 已补读取边界，流与最终持久化未混入本项。
- 新增 useLlmChatCore.test.tsx，12 项使用 deferred IPC 验证正文 A/B 倒序、独立列表、最新列表、new/close/unmount、锁定重登、换账户、页面新建、回收站预览关闭与列表、真实快捷组件新建/关闭。与 RF-101 回归一起 20/20 通过（exit 0）。
- F：完整 TypeScript exit 0；完整 ESLint exit 0，最初有 2 条快捷入口 effect 依赖 warning，改为显式解构稳定函数后 4 个修改文件定向 ESLint exit 0、无 warning；最终真实 hook/快捷组件回归 12/12 通过。默认 npm run test exit 0，143 文件/1,178 测试通过，无跳过，94.72s；之后仅解构等价函数引用并重跑对应回归。修改 TS/TSX 单独 Prettier，git diff --check 通过。
- 结论：**完成**；独立提交 `b97778df`，未推送。无关原有文件保留，RF-104 后端依赖仍受 R 基线阻塞。

### RF-105 执行记录

- 修复前 HEAD：`b97778df`。HistoryViewer 原先将 sensitive/critical 原值放入 span，只用 CSS blur 遮蔽，鼠标点击揭示且没有原生键盘按钮语义。
- 修改：未揭示时只渲染共享 MASK_PLACEHOLDER 的原生 button，title/aria-label 只含揭示提示；已揭示和 public/internal 渲染明文 span。保留 critical 验证、真实 log_write 审计和 useRevealState 的 60 秒 TTL，不夹带 RF-107 的 internal/快照身份策略迁移。对象规范 §5.2 已补实际边界。
- 单元回归：修改旧 blur 断言为 DOM 原文缺失/原生按钮断言；新增 critical 取消/成功/审计/TTL。初次新增测试误传内部 SnapshotCard 的 onCriticalAccess 属性，TypeScript 和审计断言正确报错；已改为外部 HistoryViewer 的 objectName 并检查实际 log_write，最终 16/16 passed、exit 0（2.90s）。
- 浏览器：新增 history-keyboard.spec.ts 及仅供 Vite 测试挂载的 historyKeyboardHarness.tsx，真实 HistoryViewer 与合成 IPC；Chrome 验证两项隐藏值的 DOM 不含原文，实际按 Enter/Space 后分别揭示。1/1 passed，exit 0；本次 Vite 服务仍需核对 PID/命令后定向清理，未修改 runner 配置或中断测试用例。仅测试夹具动态挂载组件，不进入生产入口。
- F：完整 TypeScript、完整 ESLint 均 exit 0、无 warning；默认 npm run test exit 0，143 文件/1,180 测试全部通过，无跳过（86.90s）。修改 TS/TSX 文件单独 Prettier，git diff --check、91 项索引/任务卡一致性检查通过。结论：**完成**；独立提交 `97bb6dbb`，未推送。

### RF-106 执行记录

- 修复前 HEAD：`97bb6dbb`。详情等级解析遗漏对象 `__fields`，动态组未保留子项等级，旧揭示状态只按字段 ID 保存，内容或账户变化时待处理验证可能继续返回。
- 修改：新增 `fieldPresentationPolicy.ts` 与无业务 Store 依赖的 `ProtectedFieldValue`，复用 RF-100 的等级解析和 `useRevealState`。详情仅 public 默认明文，其他等级只渲染 8 圆点；复制与揭示共用授权和 60 秒有效期。账户/对象/字段/内容/等级构成内部实例身份，锁定或卸载同步失效请求。真实验证 hook 取消旧验证、清理揭示状态，审计写入也受会话请求保护。
- 动态组：仅详情展平显式保留子项等级，不改变尚未迁移的历史/工作区返回结构。子项继承父级最低保护，嵌套组按最高后代等级保护；整组揭示和组内/整组复制按最高等级验证，公开父级下显式公开的子项可直接显示。规范 `docs/design_map/08_对象与模板规范.md` §2.4、§4.5、§7 已更新，其他入口后续迁移的边界保留。
- 回归覆盖：取消/成功/TTL、复制与揭示共用授权、内容 A→B→A、账户/对象变化、锁定重登、卸载、迟到 PIN、混合敏感度动态组。新增策略 8 项、共享 UI 6 项、真实验证 hook 4 项、组级 1 项；模态框改用真实 hook 并验证原值不进入 DOM。最终相关 5 文件/40 测试通过，exit 0。
- F：完整 TypeScript exit 0；完整 ESLint exit 0，修复两条依赖/未用参数 warning 后受影响文件定向 ESLint exit 0、无 warning。第一次全量 1,194 passed/5 failed，原因是旧 Modal 测试 mock 缺少 revealRemainingMs，移除该 mock 后修复。第二次全量 1,195 passed/4 failed（引导页 3 项等待失败、编辑器日期 1 项断言失败）；这两个未修改文件定向复跑 21/21 passed、exit 0，波动根因尚未确定。最终保持默认 runner 与原配置执行 npm run test：145 文件/1,199 测试全部通过，exit 0，无跳过，67.15s。未修改无关业务代码或放宽测试标准。
- DOC：修改 TS/TSX 单独 Prettier；git diff --check 与 91 项任务索引/任务卡一致性检查通过。Rust 启动阻塞不属于本项 F+DOC 验收，仍单列保留。结论：**完成**；独立提交 `f534a8a7`，未推送；原有 Cargo/NSIS/搜索索引改动保留。

### RF-107 执行记录

- 修复前 HEAD：`f534a8a7`。历史 internal 默认明文；快照卡片切换复用字段揭示状态，读取响应无实例守卫；缺少快照元数据时当前模板名称/等级/废弃状态及排序影响历史展示。
- 修改：历史值迁入 ProtectedFieldValue，移除重复 TTL 与局部揭示状态；只有 public 默认明文。字段名称、等级、废弃状态与顺序仅依快照，缺失名称用字段键、缺失/未知等级按 internal。动态组保留 sensitivityLevel，继承父级最低保护，嵌套组按最高后代保护；子行独立验证和计时。保留旧调用方 props 兼容，展示不再调用这些现行模板回调。
- 生命周期：账户/会话/对象变化重建历史实例；翻页开始立即卸载旧卡片，抑制重复翻页；快照数据、列表读取与审计使用 sessionRequests，旧验证结果无法揭示新快照或追加过期审计。返回旧版本需重新揭示。对象规范 §4.5、§5.2 更新为实际规则。
- 定向：原 16 项历史用例通过，新增 13 项覆盖五种标签、模板改名/删除、历史顺序和缺元数据、动态组、版本/对象/会话/卸载期间迟到验证、返回旧版本及迟到数据；与共享组件 6 项合计 35/35 passed，exit 0。新测试首轮 5 项失败来自前一用例审计调用记录累积，加 beforeEach 清理 mock 调用记录后通过，没有放宽断言。
- F：完整 TypeScript、完整 ESLint、默认 npm run test 均 exit 0；145 文件/1,212 测试全通过，无跳过，88.34s。修改 TSX 单独 Prettier；git diff --check 与 91 项任务卡/索引核对通过。
- 键盘复测：本机 Chrome，playwright test e2e/history-keyboard.spec.ts --project=chromium --workers=1，1/1 passed、exit 0，真实 Enter/Space 与原文 DOM 缺失断言通过。用例结束后核对 PID 33576 的路径/命令，只清理本次 Vite 服务，runner 正常退出。此证据不替代原生平台验收，本项未改原生代码。
- 结论：**完成**；独立提交 `edda39d7`，未推送。原有无关修改保留，Rust 启动阻塞未解除；下一项 RF-108。

### RF-108 执行记录

- 修复前 HEAD：`edda39d7`。FieldValueHint 对 critical 直接点击揭示，空聚合标签默认明文；span 缺乏键盘语义，揭示点击冒泡到父结果。快捷弹层原生 button 包整行，嵌入揭示按钮会形成按钮嵌套。
- 修改：两入口共享 ProtectedFieldValue，按聚合最高等级保护，缺失/未知标签按 internal；key 绑定账户/对象/字段/查询/值/有效等级。critical 经统一 PasswordVerificationDialog 调用 verify_password，成功写 source=search 审计；验证取消、会话变化或卸载使旧请求失效。internal/sensitive 无密码揭示，共享 60 秒 TTL；标题/aria 不含原值。
- 交互：原生揭示按钮停止点击及 Enter/Space 冒泡；快捷结果容器保留键盘打开能力且不嵌套原生按钮。验证期间暂停弹层外部点击和 Escape 关闭，密码框操作与取消不触发详情。复用现有查询生命周期，输入或筛选立即移除旧结果并取消验证。对象规范与前端架构规范更新。
- 定向：新增共享搜索 13 项及真实弹层 1 项，与原 helper/Popover/useUnifiedSearch 合计 45/45 passed，exit 0。覆盖四等级及混合/未知/空等级、取消/错误密码/成功审计/TTL、查询/内容/账户/锁定/卸载迟到验证和事件冒泡。首轮新增成功断言因 Highlight 与字段名同一文本容器而定位不精确，改为检查容器完整 textContent；未改变保护实现或放宽安全断言。
- F：完整 TypeScript、完整 ESLint 均 exit 0；修改 TSX/TS/E2E 单独 Prettier。首轮默认全量 1,225 passed/1 failed，LazyPhotoViewerOverlay 冷加载超过测试默认 5 秒；保持测试和 runner 配置不变，浏览器检查结束后单独重跑 npm run test，145 文件/1,226 测试全部通过、exit 0，无跳过，60.46s。保留首轮超时事实，不宣称其长期波动已修复。
- WEB：本机 Chrome。首次运行 search-protection.spec.ts + sidebar-tools.spec.ts 的搜索筛选共 6 项，原侧栏 4/4 通过；新增 page 在登录前启动等待失败，popover 因用例错误定位 input[type=password] 超时（实际 SecurePasswordInput 为遮罩 text）。更正为可访问 textbox 定位，并给搜索 fixture 提供已安装 OCR 状态，单独复跑新增两项 2/2 passed、exit 0（40.8s），覆盖 Enter/Space、取消、实际密码框点击、验证成功、无详情误开及新查询重掩。两轮均在全部用例结束后核对并清理各自 Vite PID，未中断用例。
- DOC：git diff --check、修改文件格式检查及 91 项索引/任务卡一致性通过。结论：**完成**；独立提交 `6c5512f2`，未推送；无关原有文件保留。R 启动阻塞继续保留。

### RF-007 执行记录（2026-09-25）

- 修复前 HEAD：`6c5512f2`。CLI 回滚恢复属性但未恢复字段标签，新生成的回滚快照同样丢失标签，连续回滚不能保留历史敏感度。
- 修复：恢复 `propertyLabels`，兼容 `property_labels`，两者同时存在时 camelCase 优先；缺失保留当前标签，显式 null 清除，空对象恢复空标签表；非对象且非 null 的结构在任何持久化之前拒绝。新快照统一写出 `propertyLabels`，原有快照归属校验、交互和字段定义恢复保持。CLI 用户指南 §4.4 已明确兼容语义。
- 定向验证：新增 3 个真实临时 Vault 测试，覆盖双键名、连续回滚、无现存模板的字段定义与敏感度、缺失/null/空对象/键名优先级、非法结构时对象/版本/历史/审计均不变。`cargo test --lib rf007_` 3/3；`cargo test --lib rollback` 原有 3/3（含跨对象拒绝）。
- 环境：首次沙箱定向测试在创建临时账户的原子写入处因 Windows ACL 返回 Access denied，尚未进入回滚；正常主机权限下重新执行通过。测试仅使用临时目录，未访问真实保险库。
- CLI 全量：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --verbose --no-fail-fast` 均 exit 0；167 个库测试及 2 个集成测试通过，0 失败，1 个原有文档测试 ignored。sqlite3 仅通过当前测试进程 PATH 使用此前验证的缓存工具。Cargo 自动同步的 5 个本地依赖版本已在命令结束后还原，未混入本项。
- R 阻塞复核：正常主机下原 GUI 测试程序 `--list` 仍以 0xc0000139 退出。只读导入检查发现默认系统 comctl32 缺少 TaskDialogIndirect，mt 确认测试程序无资源区/嵌入清单；临时外部 Common Controls v6 清单未解决，已删除，故此线索尚不能认定为完整根因。未修改二进制或系统运行库。RF-009 要求 R+CORE+CLI，同样暂缓，优先继续 RF-011。
- DOC：`git diff --check`、任务索引/任务卡及状态计数核对。结论：**完成**；独立提交 `297721c5`，未推送；原有 Cargo/NSIS/搜索索引修改保留。

### RF-011 执行记录（2026-09-25）

- 修复前 HEAD：`297721c5`。CLI 恢复条目强制要求 `data` 数组，GUI 2.0 输出的 `data_b64` 无法读取；原路径逐条写入，未预先完成全部解码。
- 修复：读取 Base64 与旧字节数组；非空 Base64 优先，空 Base64 回退旧数组；显式空数据合法，缺失/null 的两种载荷均不可伪造为空 Profile，非法 Base64 不降级回退。先解码完整清单再开始写入，后续条目损坏不提前覆盖已有 Profile。原有确认、范围、版本和更新时间存储规则保持；用户指南 §4.6 已说明语义。
- 依赖：显式声明已存在于依赖树和 lockfile 的 base64 0.22，锁文件仅新增 CLI 直接依赖引用；还原 Cargo 自动同步的 5 个本地 crate 版本副产物，未升级库。
- 定向：新增 4 个真实临时 Vault 测试，覆盖 GUI/旧格式非 UTF-8 字节与元数据、确认前不写入、双字段优先级、显式空数据、空清单保留既有内容、非法后续条目不修改前面的 Profile、取消与错误反馈。首轮 2/4 失败源于测试误假设 updated_at 保留传入值；核对 storage/profile.rs 的统一更新时间规则后，改为恢复时间区间断言及与原持久化记录比较，保留完整不变性断言。最终 `cargo test --lib rf011_` 4/4、`cargo test --lib commands::backup` 10/10，exit 0。
- CLI 全量：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --verbose --no-fail-fast` 均 exit 0；171 库测试 + 2 集成测试通过，0 失败，1 个既有文档测试 ignored。正常主机权限运行临时 Vault 测试，sqlite3 仅加入该测试进程 PATH，未访问真实保险库。
- 阻塞调查：独立副本嵌入 Tauri 默认 manifest 后可列出 485 GUI 测试，随后微型链接探针验证避免双份 manifest 的构建方案，详见新增 RF-901。诊断副本/微型工程已删除，原 GUI 产物未改；尚未以正常项目构建验证，RF-001/006/009 保留阻塞。
- DOC：`git diff --check`、92 项索引/任务卡及状态计数核对通过。结论：**完成**；独立提交 `2d3c3b7a`，未推送；原有无关修改保留。
### RF-901 执行记录（2026-09-25）

- 修复前 HEAD：`2d3c3b7a`。仅 Windows MSVC 目标关闭 Tauri 资源中的重复 manifest，由 MSVC 链接器对库测试、应用和集成测试统一嵌入 Common Controls v6 依赖；图标/版本资源继续由 Tauri 生成，其他目标保持原路径。
- 已验证：`cargo fmt --all -- --check`、`cargo clippy -- -D warnings` 均 exit 0；正常 `cargo test -p solo_soul --lib -- --list` 完成编译（4m44s）并列出 485 tests、exit 0。mt 只读提取新测试产物的嵌入清单，确认 Common Controls v6 和 asInvoker；实际 rustc 命令包含新链接参数。原启动错误已消失，没有使用诊断副本替代正式构建。
- R 全量：正常主机 `cargo test --verbose` exit 0；编译 10m57s，全部 19 组测试结果合计 1,053 passed / 0 failed / 3 ignored，包含 GUI 库 485/485、core/vault/crypto/sync/plugin 及集成/文档测试。原有 ignored 项未改动或新增。构建期间多个原生目标同时链接、可用内存约 1 GB，等待全部完成；后续可通过当前进程的 CARGO_BUILD_JOBS=2 限制编译并发，测试范围不变。
- 应用验收：全量 Cargo 日志确认正常 src/main.rs 的 `--crate-type bin` 编译成功，产物时间晚于本次构建开始。mt 只读提取应用清单确认 Common Controls v6 / asInvoker；以数据资源方式加载并核对 group icon 32512、version 1、manifest 1 均存在，ProductName/FileDescription 为 SoloSoul，ProductVersion 为 2.13.2。未启动真实账户应用，未修改任何产物来取得验收结果。
- 平台范围：分支同时核对 CARGO_CFG_TARGET_OS=windows 与 CARGO_CFG_TARGET_ENV=msvc；其他目标继续传递原 Attributes。此轮未声称运行 macOS/Android/iOS 或 GNU 交叉构建。
- DOC：`git diff --check`、92 项索引/任务卡及状态计数核对。结论：**完成**；独立提交 `16516ea8`，未推送；RF-001/006/009 的环境阻塞已解除，回到 RF-001。原有 Cargo/NSIS/搜索索引修改保持。

### RF-001 执行记录（2026-09-25）

- 修复前 HEAD：`16516ea8`。新增 VaultSession（账户、代次、原 Vault Arc），capture_session 一次性捕获并校验预期账户；with_session 在同一短临界区验证代次/账户/Arc 身份并执行同步提交。令牌不复制密钥、不序列化进 IPC；回调错误原样传播，数据库多写原子性仍由业务事务负责。
- 生命周期：创建账户、密码解锁、会话密钥解锁及换密钥后重开统一发布会话。准备阶段仅捕获代次，KDF/打开数据库/读取偏好均在提交门闩外；发布前再次检查，防止锁定或新会话被迟到结果覆盖。替换时关闭旧 Vault 并擦除其密钥；锁定/重开使旧令牌失效，换密钥后重开失败保持锁定。锁顺序及禁止网络/压缩/推理/重入已记录于 session.rs。LLM/云同步调用方迁移留给 RF-002/003。
- 定向：8 个 rf001_ 测试通过（exit 0），包括真实临时 Vault、线程/通道屏障；覆盖账户/服务不匹配、同账户重新解锁、失败解锁、数据库打开失败不留下部分状态、旧句柄撤销、迟到发布拒绝、提交中途锁定、网络等待不阻塞锁定、回调错误及 panic 后显式锁定恢复。首轮仅出现一个新增无用导入警告，已清除；复核统一清理路径的状态锁顺序后，core 库全量 215/215 通过（exit 0），包含 8 个新增用例及原有改密/KDF/附件回归。
- 已通过：Tauri fmt、Tauri Clippy（exit 0，2m36s）、CLI fmt 与 CLI 全目标 Clippy。CLI 全量 `cargo test --verbose --no-fail-fast` exit 0：171 库测试 + 2 集成测试通过，0 失败，1 个既有文档测试 ignored；编译 22m23s。正常主机临时 Vault 测试，sqlite3 仅加入本次进程 PATH；Cargo 自动更新的 5 个本地 crate 版本已定向还原。
- R 全量：`cargo test --verbose` exit 0，19 组结果合计 **1,061 passed / 0 failed / 3 ignored**，包含 GUI 485/485、core 215/215、vault 183/183 及同步/密码学/插件/集成/文档测试。3 个 ignored 为既有的 2 个 legacy 插件字段用例与 1 个 P025 手动性能测量，未新增或修改跳过项。编译 37m20s；本轮 Tauri/CLI 编译并发分别限制为 2/1，两套原生链接同时运行耗时较长，后续共享核心验证改为顺序执行，避免内存竞争。
- 入口点弹窗复核：用户反馈 `solo_soul-a1179f00737fbea1.exe` 的 TaskDialogIndirect 错误。当前文件时间为 19:24:43；只读提取清单确认 Common Controls v6 / asInvoker，实际 rustc 链接参数包含 RF-901 的两项设置。直接运行当前文件 `--list` 正常列出 485 tests、exit 0；系统中错误窗口归属 csrss，无对应失败测试进程，本轮 Cargo 当时仍在编译、尚未启动 GUI 测试。证据支持此前加载失败遗留窗口，不能据此把当前全量判失败或通过，继续等待正式结果。
- 弹窗清理：按用户要求，通过 Computer Use 定位唯一错误窗口，定向发送 Return 后窗口列表确认消失；未结束 csrss、Cargo 或编译进程。
- 最终结论：当前正式 GUI 485 项及全部 workspace 测试成功运行，弹窗未在本轮复现。`git diff --check`、92 项索引/任务卡及状态计数核对通过。本项只改变共享 Rust 会话基础，不涉及前端或 LLM/云同步调用方迁移。**完成**；独立提交 `390a5f8e`，未推送；原有 Cargo/NSIS/搜索索引修改保留，下一项 RF-002。

### RF-002 执行记录（2026-09-25）

- 修复前 HEAD：`390a5f8e`。流事件只携带 conversationId；网络完成后的回复保存与统计持久化重新获取当前 Vault，账户参数与该 Vault 未绑定；统计内存更新发生在会话校验之前。
- 修改：首次异步等待前捕获 RF-001 令牌，StreamContext 将事件发布、回复提交与统计更新绑定原账户/代次/Vault。所有流事件统一携带 accountId/sessionGeneration/conversationId/requestId；新增可选 requestId 命令参数，旧调用方省略时生成 UUID，原事件名和内容字段保持。SSE、JSON 打字机、完成与持久化失败发布均经会话门闩校验。provider 登记及网络地址校验保留。
- 统计：先等待异步统计锁，再进入短会话临界区；原 Vault 持久化成功后才更新内存。缺内存缓存时从原 Vault 加载既有统计。普通会话保存/删除在生成 HLC、执行 SQL 之前拒绝账户错配。前端队列过滤与消除前后端重复写入继续由 RF-104 完成。
- 回归：新增真实本地 HTTP 流/临时 Vault 用例，覆盖完整后端成功路径、换账户/同账户重登/锁定后的迟到流、JSON 降级与完成后锁定、统计锁等待期间锁定；另有存储账户错配及数据/HLC/墓碑不变测试。未访问外部模型服务。
- 已通过：Tauri fmt；`cargo test -p solosoul-vault --lib rf002_` 1/1，exit 0，编译 1m24s；`cargo test -p solo_soul --lib rf002_` 4/4，exit 0，编译 9m40s、执行 1.26s。正常路径通过真实本地 HTTP 调用完整 run_chat_stream，验证仅追加一条回复与一次用量；其他用例验证会话失效后事件、持久化和内存统计均拒绝。首次 fmt 命令误重复工作目录前缀导致格式未执行，修正为 tauri/ 下相对路径后 fmt check exit 0。
- CONTRACT：`npm run check:acl`（219 命令）、`npm run check:pref-keys`（22 key）、`node scripts/check-markdown-chunk-boundary.mjs`（13 依赖同处 markdown-vendor）均通过，整组 exit 0。
- Tauri Clippy：`cargo clippy -- -D warnings` exit 0。进行中：workspace 全量 → CLI fmt/全目标 Clippy/全量测试，按顺序执行，CARGO_BUILD_JOBS=1。R/CORE/CLI 全量完成前不提交、不关闭；CLI 将使用既有 sqlite3 测试缓存，运行结束后定向还原 Cargo 自动更新的本地 crate 版本。
- 构建记录：workspace Cargo 提示同包 `solo_soul` 的 lib/bin 目标共用 `solo_soul.pdb` 文件名。两个目标名称均为本项修改前的配置；当前为非致命 Cargo 警告，保留实际记录，不在会话修复中改动产物命名。
- 首轮全量结果：编译 31m26s，GUI 489/489 通过；core 212 passed / 3 failed，整组 exit 101，后续 CLI 尚未启动。失败均来自 llm/service.rs 的共用测试夹具：VaultConfig 账户为 test，Profile 与会话账户为 test_account，被本项新增的存储校验拒绝。统一夹具使用同一个 account_id，不改生产校验或原有 CRUD/LWW/防复活断言；另核对同步与 vault 相关会话夹具，账户一致。
- 容错复核与修复：有效会话的统计读写失败记录错误、保持已完成回复成功且不污染缓存；事件交付失败记录日志后仍保存后台回复。两者均在会话门闩内处理，外层会话失效仍必须拒绝。新增真实 HTTP 完成路径回归覆盖统计读取/写入失败和事件交付失败。临时补丁已应用并删除，canonical 规范同步更新。
- 最终验证进行中：Tauri fmt/Clippy → core LLM 定向 → GUI rf002_（6 项）→ R 全量 → CLI fmt/全目标 Clippy/全量。此轮 Tauri 构建并发为 2，CLI 为 1，顺序执行，不同时链接两套工程。未完成的检查不能以首轮局部通过替代，本项继续保持进行中。
- 最终验证阶段结果：fmt 与 Clippy 已通过（exit 0；Clippy 2m07s）；core LLM 定向 5/5 passed、exit 0，包含首轮失败的 3 个用例。最终 GUI rf002_ 6/6 passed、exit 0（编译 6m44s，执行 1.40s），新增统计读/写失败和事件交付失败回归通过。当前进入最终 R 全量，CLI 随后顺序执行。92 个任务卡与索引逐一匹配，未提前增加关闭计数。
- 规范复核：清理当前“使用统计”章节残留的 30 秒 debounce/退出时保存描述，明确普通流完成后的立即持久化及独立步骤失败边界；历史实施记录注明由 RF-002 当前行为取代。
- 最终 R 全量：`cargo test --verbose` exit 0，编译 20m43s；19 组结果合计 **1,068 passed / 0 failed / 3 ignored**，包括 GUI 491/491、core 215/215、vault 184/184、同步/密码学/插件及集成/文档测试。3 个 ignored 仍为既有 legacy 字段 2 项及 P025 手动性能工具 1 项，未新增跳过。首轮账户夹具失败已在此轮验证消除。队列已进入 CLI 检查，结束前不关闭或提交本项。
- CLI 全量：fmt、全目标 Clippy、`cargo test --verbose --no-fail-fast` 均 exit 0；编译 7m17s，171 库测试 + 2 集成测试通过，0 失败，1 个既有 i18n 文档测试 ignored。正常主机临时 Vault 测试，sqlite3 仅通过本次进程 PATH 使用既有缓存。运行结束后定向还原 Cargo 自动更新的 5 个本地 crate 版本，CLI lockfile 无残余差异。
- 最终结论：定向、R/CORE/CLI/CONTRACT 全部满足；规范、92 项索引/任务卡与状态计数、`git diff --check` 和暂存范围核对通过。**完成**；独立提交 `d682485b`，未推送。提交只含本项 6 个文件，原有 Cargo/NSIS/搜索索引修改保留；下一项 RF-003。

### RF-018 执行记录（2026-09-25）

- 修复前 HEAD：`d682485b`。RF-003 排查发现完整导入判定依赖 RF-020，而模板保存失败被吞掉的问题已归 RF-018；依赖补入 RF-003 任务卡与索引，先完成 RF-018，再处理 RF-020，随后回到 RF-003。未开始或缩减 RF-003 的会话绑定实现。
- 修改：resolve_template_id 的两处按 ID 查询从 ok/flatten 改为传播错误，两处模板保存使用 `?`，仅在真实存在或保存成功后返回 ID。rebuild_imported_templates 原有 `?` 使模板阶段失败时对象阶段不执行；按内容哈希复用保持原行为，不顺带改整体导入事务。
- 回归：新增 rf018 测试模块，使用临时 SQLite 的拒绝写入触发器覆盖原始/派生 ID 两个分支，真实加密包走 import_execute_internal 并断言失败后无关联对象；损坏行验证两处读取错误均不被当成不存在；拒绝写入时按内容哈希复用仍成功。新增 rusqlite workspace 测试依赖，复用已有版本，无新增运行时依赖；原 Cargo.toml 的本地状态仅有换行差异，新增语义差异仅这一项 dev-dependency。
- 规范：导入导出设计新增模板失败边界，明确早先保存的模板可能保留，不误称整包回滚。验证队列已启动：fmt → Clippy → rf018_ 定向 → 原有 test_rebuild_templates_ → R 全量；完成前保持进行中，不提交。
- 已通过：fmt、Clippy 均 exit 0（Clippy 2m48s）。定向回归正在构建；92 项任务依赖引用与无环检查通过。Cargo.lock 只增加 GUI 包对现有 rusqlite 的依赖引用，未升级库。
- 定向结果：`cargo test -p solo_soul --lib rf018_` 3/3 passed、exit 0（编译 5m06s、执行 0.69s）。真实模板写入触发器和损坏行覆盖两处保存/两处查询，真实加密包导入在模板失败后未创建关联对象；哈希复用不触发保存。原有模板用例及 R 全量仍按队列继续，尚未关闭本项。
- 原有回归：`cargo test -p solo_soul --lib test_rebuild_templates_` 2/2 passed、exit 0，原始 ID 保留与冲突派生路径保持。当前已进入 `cargo test --verbose` 全量验证，结束前不关闭或提交。
- 最终 R 全量：`cargo test --verbose` exit 0，编译 13m14s；19 组结果合计 **1,071 passed / 0 failed / 3 ignored**，包括 GUI 494/494、core 215/215、vault 184/184 及同步/密码学/插件/集成/文档测试。3 个 ignored 仍为既有 legacy 字段 2 项及 P025 手动性能工具 1 项，未新增跳过。本项生产修改仅在 Host，未改变共享核心或 CLI 接口。
- 最终结论：定向与 R 检查满足，规范、92 项索引/任务卡与状态计数、暂存范围和 `git diff --check` 核对通过。**完成**；提交为本提交（按 RF-018 检索），未推送；只含本项 7 个文件，原有 NSIS 图片和搜索索引修改保留。下一项 RF-020。

### RF-020 执行记录（2026-09-25，完成）

- 修复前 HEAD：`38940f33`（RF-018）。Host 导入仅在最终成功返回两个计数，中途错误丢失已提交数量；历史及偏好保存错误可被吞掉。共享附件导入先写文件再逐对象关联元数据，后续失败无法报告先前已提交附件数。
- 正在实施：添加完整/部分/未提交状态、各阶段已提交数量与脱敏失败阶段；共享附件唯一实现提供进度累计，保留原 CLI API；普通导入、手动/自动云导入及账户恢复改为显式检查完整状态，保留未完成源包，拒绝误报成功或推进水线。云页面原有 null selections 与 Vec 契约不匹配，同时改为 Option 区分全量与空选择；Windows 路径分隔符纳入解析。
- 验证范围补充：本项改变共享 core 附件函数，因此除任务卡 R/F/CONTRACT 外，增加 CLI fmt/全目标 Clippy/全量测试，按顺序执行两套 Rust 构建。当前仅初步 `cargo check -p solo_soul --tests` 通过（exit 0，2m00s），不等同于回归或全量验收。
- 新增真实 SQLite 故障与加密包回归，覆盖对象、模板、历史、附件解密/元数据写回和偏好失败；自动云同步验证源包/水线仅在完整成功时变化。Rust 定向和前端类型/定向测试已启动，结果尚未全部返回；保持进行中，未提交。
- 首轮前端：TypeScript exit 0；定向 6 项中普通导入 3 项通过、云页面 3 项超时（exit 1）。测试全局 react-i18next mock 每次返回新的 t，导致依赖 t 的配置加载 effect 重复触发；定向 fixture 改为稳定的真实 i18next 绑定，不改生产逻辑或超时。增加恢复成功/部分/未提交 3 项回归，正在重跑。还原 JSON 格式化造成的无关缩进，双语文件仅新增本项 8 个键。
- 前端重跑：2 文件、9/9 passed、exit 0（22.88s）。最终 TypeScript、Lint、ACL 219 命令、偏好 22 key、Markdown 13 依赖检查全部 exit 0。完整默认 `npm run test` 已启动，定向通过不替代全量。
- 首轮 Rust 定向：编译 10m08s，6 passed / 1 failed，exit 101。新增模板夹具使用 snake_case 而 UserTemplate 契约为 camelCase，导入在写入前正确拒绝，因此与测试预期“先成功一个模板”不符；已修正夹具字段，未弱化验证。追加重复对象 ID 仅计一次的回归，并使 Windows 测试夹具先关闭数据库再删除临时目录。当前顺序运行 Clippy → rf020_ → 现有 export_import 回归，尚未取得最终 R/CLI 全量结果。
- F 全量：默认 `npm run test` exit 0，**147 文件、1,235 passed**，无失败/跳过，121.63s；未修改 pool/worker/超时。与已通过的 TypeScript、Lint 和 CONTRACT 共同满足前端检查。Rust Clippy exit 0，正在继续定向及现有回归，R/CLI 尚未完成。
- Rust 重跑：Clippy exit 0（3m52s）；`cargo test -p solo_soul --lib rf020_` **8/8 passed**、exit 0（编译 3m23s，执行 1.23s），模板部分写入、重复对象 ID 与真实云端水线/源包场景通过。现有 `commands::export_import::tests` **51/51 passed**、exit 0（1.12s），包含 RF-018 查询/保存错误与原有快照兼容行为。Windows 夹具顺序修正后本轮 11 个临时目录无残留；首轮 10 个已在核对日志归属和 Temp 路径边界后清理。
- 收尾检查：修改的 8 个 TS/TSX 文件 Prettier check exit 0；92 项台账核对为 16 已关闭、仅 RF-020 进行中；云同步 canonical 文档同步改为显式 complete 判定。已启动 workspace fmt/full → CLI fmt/Clippy/full 顺序队列，fmt exit 0，其余未结束前不提交或关闭本项。
- R 全量：`cargo test --verbose` exit 0，编译 19m49s；19 个 suite 合计 **1,079 passed / 0 failed / 3 ignored**，其中 GUI 502、core 215、vault 184。跳过项仍为既有两项 legacy field 测试与手动大数据基准，未新增跳过。CLI fmt exit 0、全目标 Clippy exit 0（1m22s）；CLI 完整测试仍在编译，保持进行中。
- CLI 全量：`cargo test --verbose --no-fail-fast` exit 0，编译 5m20s；**173 passed / 0 failed / 1 ignored**（171 单元 + 2 集成，既有 i18n 文档示例跳过）。顺序队列整体 exit 0。测试自动更新的 CLI lockfile 仅 5 个本地 crate 版本，已定向还原，无依赖升级。
- 最终结论：定向、R/F/CONTRACT/CLI 均满足；受影响规范同步更新，暂存范围仅本项 21 文件，92 项台账核对为 17 已关闭。**完成**；提交为本提交（按 RF-020 检索），未推送。附件持久重试与云端 skipExisting 补全仍由 RF-022 验收，本项不声称已实现事务或幂等；下一项 RF-003。

### RF-003 执行记录（2026-09-25—2026-09-26，完成）

- 修复前 HEAD：`d696efe0`（RF-020）。起始上下文只含 account_id，导出/导入在 spawn_blocking 内重新取当前 Vault；导入后水线与整轮 last_sync_at 也再次读取当前 Vault，允许账户切换后写入新库。待导入缓存目录未隔离账户。
- 实施中：CloudPreContext 持有 RF-001 会话令牌；导入/导出唯一算法读取原 Vault，入口绑定原附件密钥，短时数据库写入、附件文件发布、事件、水线和源包删除均校验原会话。HKDF/ZIP/加解密置于门闩外，CLI 继续复用同一附件算法。下载完整刷新后发布到按账户隔离的 incoming 目录，列表同步按当前账户读取，原无归属缓存保留且不自动认领。
- 新增真实加密包与内存 connector 场景：下载后、导入前、导入后水线前用通知屏障切换账户，检查新库无污染、原源包保留、重新进入原账户可重试；另覆盖导入中途切换、旧会话导出/导入和带身份事件。当前仅开始 cargo check --tests，尚未取得运行证据，不关闭或提交。
- 首轮 `cargo check -p solo_soul --tests` exit 0（2m50s）。复核后补充整轮完成保留当前配置、拒绝旧会话更新时间，以及 incoming 列表账户隔离回归；附件中途切换点放在 core 循环入口校验之后、发布文件之前。开始 Clippy → RF-003 定向顺序队列，尚未取得最终结果。共享 core 变化按任务卡追加 CLI，导入/云同步 canonical 文档同步说明边界；新增 dev-dependency async-trait 仅用于内存 connector，lockfile 无版本升级。
- 调用方复核待收尾：手动云页面在导入 IPC 返回后仍单独调用不含账户/代次的 `cloud_sync_mark_applied`；该入口直接使用当前 Vault，且 incoming 监听尚未过滤事件身份。后台路径的门闩不能替代这条手动路径的保障，RF-003 关闭前需把导入结果与水线提交绑定同一会话，并补迟到结果/事件测试（相应增加 F 验证），不能以本轮后台定向通过提前关闭。
- 首轮 Clippy exit 0（50.76s）；Rust 定向编译 6m27s，**4 passed / 3 failed**，exit 101。新内存 connector 的 HEAD 错误返回 NotFound，使下载路径提前退出，造成屏障未到达和对象不存在；另一个新列表断言比较 Windows 混合分隔符字符串而非同形路径。已正确模拟索引 HEAD，并用分段 join 构造预期路径，保留全部跨账户与数据断言，未放宽超时或删除场景。
- 手动路径实现：ImportResult 返回原 sessionGeneration；mark_applied 要求 accountId/sessionGeneration/sourcePath，拒绝旧会话与其他账户目录，自动/手动共用 finalize_cloud_import 的短时提交。前端采用 RF-101 请求守卫，退出会话清空表单与延迟计时器；事件不直接写入携带的旧文件列表，按当前账户重新读取。TypeScript exit 0；前端定向 **3 文件、12 passed**、exit 0（43.28s），含 RF-020 普通/云/恢复语义及新增迟到响应/事件回归。随后补齐清空其余表单开关，须完成最终 F。
- CONTRACT：ACL 219 命令、偏好 22 key、Markdown 13 依赖检查均 exit 0。本项保留原命令名，仅更新手动水线参数。当前重跑 Clippy → RF-003 → RF-020 → 全部导入导出定向队列；最终 R/F/CLI 尚未完成，未提交。
- 第二轮 Clippy exit 0（2m12s）；最终 TypeScript、Lint 均 exit 0。首次默认前端全量 exit 1：147 文件中 146 通过，**1,236 passed / 1 failed**，84.02s；唯一失败为未修改的 LazyPhotoViewerOverlay 冷动态导入测试触发全局 5s timeout（自身 waitFor 配置 8s），非云同步断言失败。该轮与 Rust 编译并行；保留失败，不上调超时或改 pool/worker，待原生编译结束后单独复跑失败用例及默认全量再判断。

- 2026-09-26 恢复记录：续跑时原 D 盘工作区 `.git/index` 缺失，NTFS 状态为 Full Repair Needed，系统存在坏块事件；1,509 个主仓库跟踪文件中 1,505 个可完整读取，2 份前端测试、RF-020 Rust 测试和 importOutcome.ts 共 4 文件损坏或缺失。先将全部本地引用导出 Git bundle、可读源码及原有无关改动复制到独立 C 盘并逐文件校验；恢复仓库的 git fsck 无错误，HEAD 仍为 d696efe0。4 文件从已提交版本恢复，其中两份前端测试的 RF-003 未提交改动按原验收场景重建；未修复/覆盖 D 盘现场，未推送。
- 原 RF-003 第二轮定向日志可读取：8 passed / 0 failed / 0 ignored（5.24s）；后续 RF-020 日志损坏、现有导入导出队列日志缺失，不能据此声称完成。C 盘重建前端回归已通过 2 文件、11 tests；追加已有 Rust 场景的附件密文真实解密、失效临时目录清理，以及 B 缓存目录真实存在时仍拒绝 A 源包断言，需在恢复副本重新执行原生验证。Rust fmt 与 CONTRACT 全部 exit 0；TypeScript、Lint、原 LazyPhotoViewerOverlay 失败用例单独运行均 exit 0，默认前端全量仍在运行。

- 恢复后的最终 F：TypeScript、Lint 均 exit 0；原失败 LazyPhotoViewerOverlay 单独运行 1/1 passed（3.92s）；默认 npm run test **147 文件、1,237 passed / 0 failed**、exit 0（98.89s），未改变超时、pool 或 worker。正在 C 盘从本机依赖缓存顺序执行 Clippy → RF-003 → RF-020 → 现有导入导出 → R 全量 → CLI 全量，尚未关闭本项。

- C 盘首次 Clippy exit 101：依赖检查推进到 GUI 入口后，tauri::generate_context! 因恢复副本没有 ../dist 产物而失败。已完整恢复 Windows PDF/OCR/插件运行资源（50文件哈希一致）、原插件子模块与本地 AGENTS.md；正通过 npx vite build 生成真实前端产物后续跑，未用占位 dist 或跳过入口替代验证。

- Vite 产物生成成功（exit 0，7.60s）；随后 Clippy exit 0（56.83s）。原生 RF-003 首次测试构建在链接阶段 exit 101，尚未运行测试：本次临时设置的 CARGO_NET_OFFLINE=true 使 ort-sys 进入 link_error 分支，缺失 OrtGetApiBase；这是验证环境配置错误。已核对所锁定 ort-sys 2.0.0-rc.12 的 build/vars.rs 与构建输出，移除离线参数，利用其内置运行库下载/哈希验证并自动重跑受影响构建脚本；未关闭 ONNX 功能或缩减测试范围。

- ONNX 环境修正后：构建输出已链接 C 盘缓存的 static onnxruntime；Clippy exit 0（1m02s）。RF-003 定向 **8/8 passed**、exit 0（重建1m13s、执行4.66s），包括此次补强的实际附件解密、临时清理与已有 B 目录隔离断言；RF-020 **8/8 passed**、exit 0（1.01s），原有导入导出 **51/51 passed**、exit 0（1.11s）。顺序队列已进入 R 全量，后续 CLI 尚待完成，RF-003 仍未关闭或提交。

- 恢复后 R 全量首轮 exit 101：GUI **510/510 passed**；core **214 passed / 1 failed**，唯一失败 test_apply_to_pdf_smoke 为 PDFium 动态库未找到（LoadLibraryError 126），后续 suite 与 CLI 未运行。已核对恢复资源中的 pdfium.dll 哈希；core 测试工作目录不在 GUI 资源目录，需显式设置现有 PDFIUM_LIBRARY_PATH 指向 C 盘已验证库后重跑，保留该失败记录，不更改或跳过测试。

- PDFium 路径修正后：仅对验证进程设置 PDFIUM_LIBRARY_PATH=C:\Users\40299571\SoloSoul\tauri\src-tauri\resources\pdfium\pdfium.dll；`cargo test -p solosoul-core --lib test_apply_to_pdf_smoke` **1/1 passed**、exit 0（该 feature 组合首次编译 14m02s，执行 0.12s）。DLL 加载、生成及重新读取加水印 PDF 均通过。后续 R 全量正运行，CLI 尚待完成；不因定向通过提前关闭。

- 恢复后的最终 R：`cargo test --verbose` exit 0，生产包冒烟更新 dist 后重编译 5m47s；19 组结果合计 **1,087 passed / 0 failed / 3 ignored**，其中 GUI 510、core 215、vault 184。3 个 ignored 为既有 legacy field 两项和 P025 手动性能工具，未新增跳过；本轮水印测试通过。CLI fmt exit 0，全目标 Clippy 与完整测试按队列继续，结束前不关闭或提交 RF-003。

- CLI 全目标 Clippy exit 0（恢复副本首次检查 12m01s），fmt 已通过。完整 `cargo test --verbose --no-fail-fast` 正在首次构建；13:04–13:08 出现一次编译子进程短暂停滞，随后自行恢复，未中断或重启构建。相关系统事件无明确崩溃/磁盘错误/拦截证据，不将审计事件误判为阻止。

- 最终 CLI：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --verbose --no-fail-fast` 均 exit 0。完整测试冷构建 35m31s；单元测试 **171 passed**（33.93s）、集成测试 **2 passed**（1.13s），合计 **173 passed / 0 failed / 1 ignored**；忽略项为既有 `i18n::t` 文档示例，未新增跳过。期间编译子进程和 rustdoc 曾短暂停顿并自行恢复，未终止或重新启动队列。
- 完成验收：F（147 文件、1,237 测试）、R（1,087 passed / 3 既有 ignored）、CLI、CONTRACT、RF-003 8 项、RF-020 8 项、导入导出 51 项均通过；生产包冒烟经独立 RF-902 修复后 **14/14 passed**。已同步导入导出、云同步 canonical 规范。验证日志保留于 `C:\Users\40299571\AppData\Local\Temp\SoloSoul-recovery-20260926-111523`；当前有效仓库为 `C:\Users\40299571\SoloSoul`。
- 提交边界：CLI 构建仅生成 5 个本地 crate 的版本号变化，核对后恢复，不混入本项；保留原有 3 张 NSIS 图片改动。仅提交 RF-003 的 17 个代码、测试和规范路径。提交：本提交（使用 `git log --grep=RF-003` 检索），未推送。

### RF-902 执行记录（2026-09-26，完成）

- 修复前 HEAD：`d696efe0`；RF-003 未提交代码与原有安装器图片保留。补充的生产包检查 exit 1，13 passed / 1 failed（49.8s），失败仅为旧启动夹具未覆盖 desktop_prepare_update；涉及夹具与生产适配器相对 HEAD 均无改动。
- 正在仅修正模拟命令与返回类型；待生产包全量通过后单独提交。RF-003 的完整进度一并保留在台账作为执行上下文，不将其业务代码混入本项。
- 最终验证：Prettier check exit 0；Windows x64 / Chrome 151.0.7922.108，`SOLOSOUL_E2E_CHANNEL=chrome npm run test:e2e:production` **14 passed / 0 failed / 0 skipped**、exit 0（23.4s），运行真实 Vite 生产包；原生产启动/Markdown 断言通过。93 项索引与任务卡一一对应，18 已关闭，只有 RF-003 进行中；`git diff --check` 通过。
- **完成**；本提交（按 RF-902 检索），未推送。暂存仅启动测试与执行台账 2 文件，RF-003 业务改动及其规范、原有 BMP 均保持未暂存；返回 RF-003 验证队列。
### RF-004 执行记录（2026-09-26，完成）

- 修复前 HEAD：`0665f333`（RF-003）；有效工作区 `C:\Users\40299571\SoloSoul`，保留原有 3 张 NSIS 图片。前置 RF-001/RF-100/RF-101 已完成。原普通聊天由前端 Store 读取对象字段并拼接 system；保存的 includeSystemPrompt 未传入聊天配置缓存，页面固定开启。
- 本项边界：前端只传 user/assistant 发送副本及上下文选择（none/publicProfile、候选对象 ID、界面语言、检索指南片段）；Host 绑定原会话、以保存开关为硬约束、从原 Vault 读取，Core 统一投影公开字段，Host 包装系统提示及指南为一条 system。保留持久化历史 role 字符串兼容；旧非 user/assistant 消息保留展示/存储但不作为新的出站角色。候选空列表不扩大为全库，关闭不读取附加对象/模板。指南仍由现有检索获取，回传文本不视为可信指令。provider 凭证迁移与流单写入者分别留 RF-005/RF-104。
- 已实施：Core 投影及 15 项定向回归；Host 实际发送路径及 13 项 TCP/契约/长度测试；前端 ID-only 选择、保存开关缓存传播及发送副本兼容。独立只读复核未发现新增 P1/P2 实现缺陷；补齐 camelCase 反序列化和 Unicode 1500/3000、指南数量/标题/正文边界。provider 校验仍会读取配置，所谓 none 零 Profile 读取仅针对自动上下文构造，不声称整个命令无 Profile IO。
- 前端定向 6 文件 **48/48 passed**（29.34s）；最终 F：TypeScript、Lint 均 exit 0，默认 `npm run test` **148 文件、1,249 passed / 0 failed**、exit 0（70.62s）。CONTRACT：ACL 219 命令、偏好 22 key、Markdown 13 依赖均 exit 0。生产包冒烟 **14/14 passed**、exit 0（20.1s），使用已安装 Chrome 151.0.7922.108 与真实 Vite production build，未跳过或调整测试配置。
- Rust 最终 fmt exit 0；已固定源码，在正确 PDFIUM_LIBRARY_PATH 与验证过的 SQLite 工具环境下顺序运行 Clippy → RF-004 Core → RF-004 Host → RF-002 → R 全量 → CLI fmt/Clippy/全量。原生检查尚未完成，本项仍进行中，未提交。

- 最终原生验证：fmt、Clippy 均 exit 0（Clippy 1m53s）；Core `rf004_` **15/15 passed**，Host `rf004_` **13/13 passed**，RF-002 **6/6 passed**，均 exit 0。R 全量 `cargo test --verbose` exit 0，编译 12m42s；19 组结果合计 **1,115 passed / 0 failed / 3 ignored**，含 GUI 523、core 230、vault 184。跳过项仍为两项 legacy field 和 P025 手动性能工具。
- CLI：fmt、全目标 Clippy、完整 `cargo test --verbose --no-fail-fast` 全部 exit 0；Clippy 54.41s，测试编译 2m32s，171 单元（34.38s）+ 2 集成（1.21s）通过，合计 **173 passed / 0 failed / 1 ignored**；既有 i18n 文档示例跳过。验证结束后核对并恢复 Cargo 自动刷新的 5 个本地 crate 版本，未升级依赖。
- 最终复审：纠正 canonical 规范残留的旧长度标准与空候选指南说明；93 项索引/任务卡、关闭计数与 `git diff --check` 核对通过。F/R/CORE/CLI/CONTRACT 与生产包冒烟全部满足；本项 **完成**，仅暂存本项 22 个路径，保留 3 张 NSIS 图片。提交：`c5b5dad3`，未推送。

### RF-005 执行记录（2026-09-26，完成）

- 修复前 HEAD：`c5b5dad3`（RF-004）；有效工作区仍为 C 盘恢复副本。原普通发送先由前端读取 API key，再传回地址、模型、协议与 key；保存配置只用于 URL 登记校验，不能保证按原账户启用的 provider 解析凭证。
- 实施边界：普通发送只传 provider ID；Core 从原会话 Vault 的单份 Profile 快照严格解析已保存且启用的 provider 和同份密钥，Host 保留 URL/主机解析/登记检查及发送前会话校验。设置页和在线状态检测的凭证入口维持现有行为；不声称整个渲染器不再接触凭证。RF-104 流归属与单一持久化写入者后续处理。

- 实现与复核：Core 新增严格 resolver 及 14 项真实 Vault 回归；Host providerId 网关新增 9 项测试、22 个场景，捕获真实 loopback HTTP 的 URL/模型/认证头，并在锁定/切换屏障后验证两个账户均无外连。旧 RF-002/RF-004 测试继续覆盖同一生产完成阶段，仅函数改名；新 RF-005 测试覆盖完整解析与出口链路。独立只读复核未发现新验收阻碍。所选 provider ID 重复时拒绝；错误固定脱敏文案，不透出配置值。
- 最终 F：三文件 Prettier、TypeScript、Lint 全部 exit 0；定向 2 文件 **30/30 passed**（27.67s），默认全量 **148 文件、1,251 passed / 0 failed**（79.64s）、exit 0。CONTRACT：ACL 219 命令、偏好 22 key、Markdown 13 依赖全部 exit 0。生产包冒烟 **14/14 passed**（27.5s）、exit 0，真实 Vite production build / Chrome 151.0.7922.108。未修改超时、pool、worker 或跳过规则。
- Rust fmt exit 0，代码已冻结；按既有已验证 PDFium/SQLite 环境顺序执行 Clippy、RF-005 Core/Host、RF-004/RF-002 回归、R 全量和 CLI fmt/Clippy/全量。此时尚未获得原生最终结果，本项保持进行中，未提交。

- 原生定向与回归：Clippy exit 0（1m18s）；Core RF-005 **14/14 passed**（编译1m25s、执行0.23s），Host RF-005 **9/9 passed**（编译3m57s、执行1.48s）；RF-004 **13/13 passed**（2.12s）、RF-002 **6/6 passed**（1.19s），全部 exit 0。真实认证头、保存配置覆盖、坏数据拒绝和解析后失效会话不建连均通过。
- R 全量：`cargo test --verbose` exit 0，编译10m27s；19 组结果合计 **1,138 passed / 0 failed / 3 ignored**，含 GUI 532、core 244、vault 184。跳过仍为两项 legacy field 和 P025 手动性能工具，未新增跳过。CLI fmt、全目标 Clippy（50.99s）、完整测试全部 exit 0；测试编译2m03s，171 单元（35.62s）+ 2 集成（1.12s）通过，合计 **173 passed / 0 failed / 1 ignored**，既有 i18n 文档示例跳过。
- 最终结论：F/R/CORE/CLI/CONTRACT、定向及生产包冒烟全部满足；规范、93 项索引/任务卡及 `git diff --check` 通过。已恢复 CLI 构建自动刷新的 5 个本地 crate 版本，保留原有 3 张 NSIS 图片。仅暂存本项 11 个路径；本项 **完成**，提交：`5ae18761`，未推送。下一项 RF-104 只读预研确认需每轮预存 user、在 invoke 完成前保留流归属与失败提示，尚未实施。

### RF-104 执行记录（2026-09-26，阻塞）

- 修复前 HEAD：`5ae18761`（RF-005），有效工作区 C 盘恢复副本。当前 Store 仅按 conversationId 收 chunk，两个入口都将全局 buffer 写到本地最后一条 assistant，再各自整段保存。Host 发 done 后才保存 assistant，失败会再发同身份的持久化失败事件；前端第一个 done 就退订/reset，可漏提示。done.chunk 还可能携带打字机降级的剩余正文。
- 范围与实施方案：保持 F-only；每轮先明确保存已有规范历史和新 user（不含本轮 assistant），失败中止，保留会话名称等元数据。Store 在首次等待前按 account/conversation 占位，固定 requestId/assistantMessageId，两个入口只读对应投影；实际 invoke 完成才释放 busy，最终 assistant 仅由现有 Host 保存。移除前端流结束/生成失败时的整段覆盖保存。普通发送仍使用 RF-004 上下文选择和 RF-005 providerId。
- 身份边界：前后端 generation 不是同一个计数，不直接比较。以本地会话 ticket、账户/会话/唯一 requestId 接纳已登记请求，再绑定首个匹配事件的后端 generation；后续缺字段/错身份/不同代次事件拒绝。锁定或换账户清理记录、监听注册等待及提示状态；独立通知监听改为消费已验收 Store 状态，权限等待后重新校验。成功权威重读后共享已确认正文，避免双入口确认先后造成回退；新请求/锁定再替换或清理，失败回复保留；同一请求提示只领取一次。
- 已写入：Store 及 23 项定向回归，通知消费者及 21 项单测、5 项真实 Store/AuthStore 集成测试，保存准备检查 helper 与 AppRoutes 清理接线。Store 23/23、通知 26/26 通过，定向 Prettier/ESLint 通过；未完成整体验证。
- 阻塞：自动审批两次拒绝 useLlmChatCore.ts/useLlmStreaming.ts 写入，认为涉及持久化和请求隔离且现有授权不足；补充原任务卡与历史授权后仍拒绝，已停止重试并请求用户明确确认。两个 Hook 及其测试尚未修改，旧接口与新 Store 尚未接通，当前未提交前端不能视为可构建完成品；不绕过审批或提交半成品。
- 按既有“阻塞时优先无依赖任务”目标保留这些改动，转入 RF-006；RF-104 不计入已关闭。

### RF-006 执行记录（2026-09-26，完成）

- 修复前 HEAD：`5ae18761`，有效工作区 `C:\Users\40299571\SoloSoul`。保留 RF-104 未提交代码与原有 3 张 NSIS 图片，本项只涉及 GUI 快照回滚及其验证和规范。
- 已确认：GUI 通过 snapshot_id 读取快照后直接作用于另一个 object_id；CLI 已检查 get_snapshot_owner。归属依据数据库 object_id 列，不信任快照 JSON。最小修复复用已有存储查询，在任何写入前拒绝缺失/空归属、错对象与缺失目标，不迁移 RF-008 共享用例。
- 验证计划：真实临时 Vault 覆盖跨对象（包括伪造 JSON 归属）、无快照、空归属、无目标及正常回滚；失败后对象、版本、快照与审计均不变。随后顺序完成 R/CORE/CLI，冻结源码期间不启动竞争 Cargo。

- 已实施：生产命令调用同模块 rollback_snapshot_in_vault，归属查询先于正文读取/解析，命令仅在 worker 成功后触发同步；其余恢复及 best-effort 历史/审计行为保留。原复制算法测试替换为 5 项生产 worker 回归、7 个场景，正向兼容两种标签键名，拒绝比较完整对象、历史正文/元数据/全量计数和既有审计。
- 独立只读复审未发现阻断性问题；新夹具账户与临时 Vault 一致，IPC §10 与对象规范 §5.2 已同步。定向 rustfmt 和全 workspace fmt check exit 0；源码冻结，正在按既有 PDFium/SQLite 环境顺序运行定向 → Clippy → R 全量 → CLI 检查，最终结果未出前不关闭/提交。
- 定向结果：`cargo test -p solo_soul --lib rf006_` **5 passed / 0 failed / 0 ignored**，exit 0，编译及执行 226.78s（用例执行 0.20s）；Tauri Clippy exit 0（38.45s）。队列继续 R/CLI 全量，尚未关闭。
- 最终 R：`cargo test --verbose` exit 0（队列耗时538.05s），19 组结果合计 **1,142 passed / 0 failed / 3 ignored**，含 GUI 536、core 244、vault 184。3 个跳过仍为两项 legacy field 和 P025 手动性能工具；保留既有 lib/bin PDB 输出重名警告，未更改构建配置。
- CLI：fmt、全目标 Clippy、`cargo test --verbose --no-fail-fast` 均 exit 0；Clippy 21.23s，测试队列86.15s（编译8.62s，171单元36.71s、2集成1.15s）。合计 **173 passed / 0 failed / 1 ignored**，既有 i18n 文档示例跳过。结束后只恢复 Cargo 自动刷新的 5 个本地 crate 版本，锁文件无差异。
- 最终结论：R/CORE/CLI、定向回归、两份 canonical 规范与台账检查满足；93 项索引/任务卡一致，22 已关闭、RF-104 仍阻塞。仅暂存 RF-006 的生产/测试、规范中的快照相关段落与执行台账；RF-104 代码/聊天规范和 3 张 NSIS 图片保持未暂存。本项 **完成**；提交：`0028acac`，未推送。
### RF-009 执行记录（2026-09-26，完成）

- 修复前 HEAD：`0028acac`（RF-006），有效工作区仍为 C 盘恢复副本；RF-104 和原有 NSIS 图片保持不动。CLI save_new_object 已传模板 ID 与用户值，但 core create_object 没有继承字段定义、敏感度标签、契约 ID、模板名称和指纹。
- 范围：仅在既有 core 创建入口补齐与 GUI 相同的元数据投影、复用 template_fingerprint；保留 CLI 对象 ID、类型、父页、图标、普通用户字段值与无模板行为。读取模板失败在写入前传播，缺失模板保持兼容；不迁移 GUI 或提前实施 RF-010。
- 验证计划：core 真实 Vault 定向回归、Host 同模板同输入直接对照两端创建结果、CLI 实际 save_new_object 路径及父页面/状态验证；实现冻结后完成 R/CORE/CLI。本项仍进行中，尚未验证或提交。
- 接入复核发现：CLI 详情和编辑器会把新保存的 `__fields`、`__templateName`、`__templateHash` 当作普通属性。为避免本项使新对象出现内部字段并允许误改，同项补充仅这三个保留键的展示/编辑过滤及真实 TUI/编辑回归；不扩大到所有下划线键。模板删除后的编辑器字段名/类型/敏感度回退仍有既有局限，本项只保证副本留存，不宣称已统一整个消费端。
- 范围核对：仅读取到真实模板时引入动态组校验；无/缺失模板保持旧输入行为。模板读取当前仅按 ID、所在 Vault 隔离，不在此项改变 GUI/CLI 的账户归属规则；该接口边界留 RF-010 共享创建收敛时评估。
- 实现完成并冻结：core 仅改 create_object 及本模块回归；Host 增加真实 build_create_record/Core 对照，CLI 增加真实创建→保存→详情→编辑回归，并精确过滤三个保留元数据键。共 13 个 rf009_ 测试（Core 6、Host 2、CLI 5），包含真实 TestBackend，尚未取得运行结果。
- 主 Agent 与独立只读复核未发现阻断问题；定向 rustfmt、diff 检查通过。Tauri workspace fmt exit 0、CLI fmt exit 0；正按既有 PDFium/SQLite 环境顺序运行定向与 R/CORE/CLI 队列。编译期间不修改源码或并行启动另一套 Cargo，本项仍进行中。
- Core 定向：`cargo test -p solosoul-core --lib rf009_` **6 passed / 0 failed / 0 ignored**，exit 0（队列115.50s，编译1m49s、执行0.13s）。其余 GUI/CLI 定向与完整检查尚未结束，不提前关闭。
- Host 定向 **2/2 passed**、exit 0（队列260.77s，编译4m12s、执行0.26s）；CLI 定向 **5/5 passed**、exit 0（队列115.08s，编译1m42s、执行0.73s）。合计 13 项定向全部通过，0失败、0跳过；真实 CLI 创建→编辑及 TestBackend 内部元数据过滤通过。Tauri Clippy exit 0（26.87s），现继续 R 全量与 CLI Clippy/全量。
- 最终 R：`cargo test --verbose` exit 0（队列510.29s），19 组结果合计 **1,150 passed / 0 failed / 3 ignored**，含 GUI 538、core 250、vault 184。既有两项 legacy field 与 P025 手动性能工具继续跳过，没有新增跳过。
- 最终 CLI：全目标 Clippy exit 0（48.77s）；`cargo test --verbose --no-fail-fast` exit 0（队列123.03s，编译1m13s），176 单元（35.13s）+ 2 集成（1.20s），合计 **178 passed / 0 failed / 1 ignored**；既有 i18n 文档示例跳过。结束后确认并恢复 Cargo 自动刷新的 5 个本地 crate 版本，未升级依赖。
- 最终结论：R/CORE/CLI、13项定向、主 Agent/独立复核、两份使用规范与执行台账检查满足；仅暂存本项 8 个文件。RF-104 未提交修改和 3 张 NSIS 图片继续保留。93项索引/任务卡一致，23已关闭，RF-104仍阻塞；本项 **完成**，提交：`f38506dd`，未推送。
### RF-012 执行记录（2026-09-26，完成）

- 修复前 HEAD：`f38506dd`（RF-009），有效工作区仍为 C 盘恢复副本。GUI backup_create 中 if let Ok(Some(profile)) 会静默跳过读取错误和枚举后消失条目，返回数量又与实际清单来源不同，可能覆盖已有文件并报告不完整备份成功。
- 范围：保留 Profile 备份 2.0/data_b64 格式与命令持有的 VaultService 读锁；所有枚举条目读取完整后才写备份文件，错误/消失分别失败，返回数量与清单数量来自同一完整集合，两个同步触发仅发生在成功后。不迁移 RF-013 codec，不将 Profile 备份扩成完整 Vault 备份。
- 验证计划：真实临时 Vault 和生产 worker 覆盖损坏密文读取失败、枚举后消失、同名同秒失败保留既有有效文件、多条成功内容/元数据/数量及空数据兼容；随后完成任务卡要求的 R。尚未验证或提交。
- 实施与独立复审：生产命令保留原服务读锁和 Vault，新增 worker 对读取错误、枚举后消失分别报错，收集/序列化完成后才建目录写文件。4 项定向回归覆盖 6 个场景，均使用真实临时 Vault；已有同名同秒文件及无关备份逐字节保持。已更新 IPC 备份契约，并纠正规范中将 Profile JSON/Base64 备份描述为加密全库备份的旧表述。
- 工作区 fmt exit 0；源码冻结，正运行定向 → Clippy → R 全量。此时尚未取得测试结果，不提前关闭或提交。
- 最终验证：workspace fmt exit 0（4.57s）；`cargo test -p solo_soul --lib rf012_` **4 passed / 0 failed / 0 ignored**、exit 0（队列244.76s，执行0.21s）；Clippy exit 0（36.80s）。R 全量 `cargo test --verbose` exit 0（555.72s），19 组结果合计 **1,154 passed / 0 failed / 3 ignored**，含 GUI 542、core 250、vault 184。3 个跳过仍为两项 legacy field 和 P025 手动性能工具；未新增跳过，保留既有 lib/bin PDB 输出重名警告。
- 最终结论：任务卡 R、定向回归、独立复审、两份 canonical 规范与台账检查满足；锁文件无差异。只暂存备份生产/测试、IPC 的 RF-012 段落、导出规范备份表及执行台账共 4 文件，保留 RF-104 与 3 张 NSIS 图片。93项索引/任务卡一致，24已关闭、1阻塞。本项 **完成**，提交：`b76ee6cb`，未推送。

### RF-014 执行记录（2026-09-26，阻塞）

- 修复前 HEAD：`b76ee6cb`（RF-012），有效工作区 C 盘恢复副本。云 export_full_snapshot 设置 include_attachments=true，但传入空 selected_attachment_ids；共用导出器只导出显式 ID，因此云快照缺少附件字节。
- 实施边界：复用恢复包的未删除附件枚举逻辑，将小助手参数固定为 VaultStore/account_id；云入口从捕获会话取得原 Vault/账户，短时校验后在门闩外枚举，再交给 execute_export_for_session。手动空 ID 仍为零附件，不改包格式、大小/路径校验或密码规则，不进入 RF-015 ExportPlan 重构。
- 验证计划：真实云导出入口与独立临时 Vault 导入往返，覆盖 SOLC 密文及旧明文附件字节、删除对象/附件排除、手动空选择、账户切换及同账户重新解锁、空库；之后完成 R/CORE 和 RF-003 会话回归。只使用本地测试包，不连接实际云端。
- 阻塞：自动审批在 CreateProcess 执行前拒绝生产写入，认为全量附件改变自动上传范围，而当前缺少对数据范围与已配置云端目的地的具体授权。生产/测试均未落盘、未运行 Cargo、未触发云同步；已停止重试并请求明确确认。规范草案补丁已保存在恢复日志目录 `rf014-documentation-proposal.patch`，两份 canonical 文档恢复到已提交内容，避免误标已实现。
- 本项不计入已关闭，继续无依赖的 RF-017。

### RF-017 执行记录（2026-09-26，完成）

- 修复前 HEAD：`b76ee6cb`（RF-012），有效工作区 C 盘恢复副本。Host 导出在附件校验、加密、ZIP 收尾前直接 File::create 最终路径，任一后续失败均可能截断原有效导出包。
- 边界：同目录随机临时输出，保持流式 ZIP/加密；finish、flush、sync_all 并关闭句柄后，在原会话短时校验内以平台支持的替换发布，错误由 RAII 清理临时文件。成功审计仍在发布之后。移动端只覆盖本地 staging，不宣称后续 SAF 复制原子性；CLI 独立导出迁移留 RF-023。
- 验证计划：真实临时 Vault 的导出成功可读/可解密；附件/总量超限、附件解密失败保留已有目标；可失败 Writer/文件操作覆盖写入、ZIP finish、flush/sync、发布失败；Windows 真实已有文件替换与禁止删除共享的占用失败；旧会话拒绝发布且不记成功审计，失败无临时残留。完成后运行 R 和本机 Windows 文件系统原生验证。
- 已实施并冻结：生产写入同目录随机临时文件；共用 finish helper 依次完成 ZIP、flush、sync_all 并关闭 writer，生产 finalize 在 with_session 内 persist 后记录 best-effort 审计。复用锁定的 tempfile 3.27.0，未新增依赖；Windows 实现为 MoveFileExW(REPLACE_EXISTING)，无删除目标 fallback。
- 新增 9 项 rf017_ 回归：5 项真实导出/导入/会话/Windows 流程，以及 4 项故障测试覆盖正文写入、ZIP finish 写入/seek、flush/sync 和实际 persist 失败。失败核对目标字节、输出目录和成功审计；只用合成临时数据。运行环境 Windows NT 10.0.26100 x64，测试产物位于 C 盘系统临时目录。
- workspace fmt exit 0（4.76s），正在顺序执行定向 → RF-003 会话回归 → Clippy → R 全量，源码冻结；尚未取得测试结果，不提前关闭。
- 首次定向编译 exit 101（104.54s，0 tests）：测试包装 FailingFile 未实现 zip 2.4.2 的 set_flush_on_finish_file 所需 Read trait。队列已停止，未执行后续检查。仅补透明 Read 转发，生产逻辑不变；保留首次失败日志，修正后重新冻结并重跑验证。
- 第二次定向 exit 101（189.36s，执行1.30s）：**8 passed / 1 failed / 0 ignored**。失败保留了有价值的真实依赖问题：zip 2.4.2 更新局部头时 write 返回错误，finish 按值退栈触发 Drop 重试，游标已回到头部，导致内部 debug_assert panic。成功往返、超限、SOLC、Windows占用/释放和会话失效场景已通过，但本项仍未关闭。
- 修复仅在 `zip.finish()` 外增加窄 catch_unwind，固定错误文案；writer 被消费并先关闭，TempPath 保留外层 RAII，不能发布。保留持续 Write/Seek 故障注入不降级，不修改 panic hook，也不将会话门闩包入 catch。项目 release 已为 panic=unwind；本项不声称能处理 abort/双 panic。重新冻结后完整重跑，保留两次失败日志。
- 扩展复审确认 start_file 收尾前一条目也存在同类依赖清理 panic；同项增加局部 write_export_output，在捕获范围内创建并拥有 writer，成功才转交 finish helper。附件收集与总量检查提前到临时输出创建之前，KDF 和会话发布保持范围之外。
- 新增第二条目开始时的持续 Write 故障回归，正文写入测试也改用真实写包 worker；原 finish 故障点未改变。现在共 **10 项 rf017_**，主 Agent 与独立复审通过。第三次 fmt exit 0（2.16s），源码冻结，正在重新运行全部定向、会话及 R 检查。
- 第三次定向：`cargo test -p solo_soul --lib rf017_` **10 passed / 0 failed / 0 ignored**、exit 0（队列371.46s，执行1.29s）。原持续故障与新增条目切换故障全部通过；Windows占用拒绝/释放后替换、真实独立 Vault 解密导入、会话失效拒绝、旧文件及目录清理证据齐备。故障测试有预期 ZipWriter drop failed stderr，属于合成 Write/Seek 故障，退出码为 0；未屏蔽错误输出。队列继续 RF-003/Clippy/R，尚未关闭。

- RF-003 会话回归 **8 passed / 0 failed / 0 ignored**、exit 0（队列7.30s，执行2.40s）；Clippy exit 0（315.24s）。最后 R 全量 `cargo test --verbose` exit 0（队列1,483.27s），19 组结果合计 **1,164 passed / 0 failed / 3 ignored**，含 GUI 552、core 250、vault 184。跳过项仍为既有两项 legacy field 和 P025 手动性能工具，未新增跳过；保留既有 PDB 输出重名警告。
- 构建等待期间只读核验确认编译子进程会短暂低活动后自行运行并写入 C 盘产物，没有中断、重复启动或修改系统设置；无法据此确定启动等待原因。文档测试最终正常完成。
- 最终结论：R、10项定向、RF-003 回归与本机 Windows 真实文件替换满足验收，canonical 规范、93项索引/任务卡及 staged diff 检查通过；锁文件无变化。仅暂存本项 6 个路径，保留 RF-104 未提交修改和原有 3 张 NSIS 图片。25已关闭、2阻塞；本项 **完成**，提交：`fe4e2c96`，未推送。

### RF-019 执行记录（2026-09-26—27，完成）

- 修复前 HEAD：`fe4e2c96`（RF-017），有效工作区仍为 C 盘恢复副本，RF-104 和原有 NSIS 图片保留。with_tx 手工 BEGIN/COMMIT/ROLLBACK，仅业务 Err 会回滚；COMMIT 失败或回调 unwind 可留下活动事务，且原 prepare_cached 需要可变连接的注释不符合所锁定 rusqlite API。
- 实施边界：使用 rusqlite 0.32.1 的 Deferred Transaction，回调及实际所需事务 helper 改收 &Connection；保留 begin/commit 脱敏上下文、原业务错误、既有锁和提交范围。仅由成功创建的事务负责回滚，不清理调用者已有事务，不自动清除 Mutex poison，也不改变同步逐条错误收集规则。
- 验证计划：6 项真实 SQLite 回归覆盖成功/prepare_cached、业务 Err、deferred FK 的真实 COMMIT 失败与裸 SQL 对照、直接 Connection 回调 panic、外层已有事务的 BEGIN 失败，以及持锁 panic 后保留 poison 但连接已回滚；之后完成 R/CORE/CLI。此时尚未验证或提交。

- 实施完成并冻结：8 个生产文件仅调整共用 with_tx、17 个 helper 连接参数及旧注释；测试文件新增 6 项真实 SQLite 回归。保留同步批量逐条错误收集规则、既有 SQL/HLC/持锁范围，未修改 snapshots 或独立事务。主 Agent 与独立只读复核均未发现必须修复的问题，已核对所锁定 rusqlite 的默认回滚及提交失败析构路径。
- 定向 rustfmt、git diff --check 通过；workspace fmt exit 0（1.98s）。正在顺序执行 rf019_ → Vault 全量 → Clippy → R 全量 → CLI fmt/Clippy/全量；未取得后续结果前保持进行中，不提前关闭或提交。

- 定向 `cargo test -p solosoul-vault --lib rf019_` **6 passed / 0 failed / 0 ignored**，exit 0（队列181.36s）。随后 `cargo test -p solosoul-vault` **190 passed / 0 failed / 1 ignored**、exit 0（252.63s）；跳过仅既有 P025 手动性能工具，文档测试正常结束。Clippy exit 0（174.46s），队列继续 R 全量和 CLI，本项未关闭。
- 编译等待期间只读环境核验未发现进程/用户/系统 PATH 或已知 Cargo 配置引用 D 盘；无证据据此调整环境。原队列保持不动。后续独立任务的只读预研保存在恢复日志目录 `readonly-preplans-20260927.md`，包含附件清理归属和实际附件布局与改密扫描不一致的待复现疑点，不混入 RF-019 修复。

- R 全量最终 `cargo test --verbose` exit 0（队列1,556.56s），19 组结果合计 **1,170 passed / 0 failed / 3 ignored**，含 GUI 552、core 250、vault 190；既有两项 legacy field 和 P025 手动性能工具继续跳过。编译/链接等待期间的只读采样确认链接器 CPU、IO 与工作集增长，没有中断或重启原队列。
- CLI fmt exit 0（1.58s），全目标 Clippy exit 0（171.31s），完整 `cargo test --verbose --no-fail-fast` 正在运行。验证完成前不恢复 Cargo 自动刷新的本地 crate 版本锁文件，不提前关闭本项。

- CLI 完整 `cargo test --verbose --no-fail-fast` exit 0（队列471.73s，编译6m27s），176 单元（36.64s）+ 2 集成（1.18s）全部通过，合计 **178 passed / 0 failed / 1 ignored**；跳过为既有 i18n 文档示例。所有 Cargo 已退出后，核对并恢复仅自动刷新的 5 个本地 crate 版本，锁文件无剩余差异。
- 最终结论：R/CORE/CLI、6项定向、主 Agent 与独立复核、两份 canonical 规范及台账检查满足；93项索引/任务卡一致，26已关闭、2阻塞。仅暂存本项 12 个文件，保留 RF-104 未提交修改和原有 3 张 NSIS 图片。本项 **完成**，提交：本提交（按 RF-019 检索），未推送。


### RF-903 / RF-905 核查记录（2026-09-27，待修复）

- 基线 HEAD `5a450b75`，只使用 C 盘恢复工作区和合成 TempDir。首次 rf903 红灯编译失败（TrashItemSummary 不含 data）不算复现；修正后真实 A/B 同包导入、切回 A 清理，在文件存在断言失败，旧函数返回 `Ok((2, 0))`。日志：恢复目录 `rf903-red-host-r2.log`。同轮回收站用例先遇到合法的恢复换 ID，已修正测试按 returned ID 检查，仍保留完整元数据、字节和解密断言。
- 三方源码核查确认：单靠当前账户对象目录、重复查引用、数据库指纹或短时 session gate 都挡不住附件发布至元数据登记的空窗；GUI 未参与进程锁且 CLI 失锁继续。登记 RF-905 独立前置，再实施 RF-903，不以禁用清理收尾。
- 完整预研保存于恢复目录 `readonly-preplans-20260927-rf903.md`。两项尚未修复，不计入已关闭；RF-904 改密附件目录疑点没有运行证据，暂未登记。

### RF-016 执行记录（2026-09-27，阻塞）

- 修复前 HEAD `5a450b75`。GUI 单删、批删与 Core purge 都先删除物理目录，再保存附件元数据；保存失败会留下指向已丢失文件的记录，文件删除失败又被忽略。
- 设计：schema 27 中记录本地附件清理意图，无对象级联、绝对路径、文件名或秘密，不导出/同步；单事务提交实际元数据移除、版本/HLC与意图。区分元数据 owner object ID 与恢复后保留的 storage object ID。执行器在原会话及数据库 Immediate 事务内复核当前/软删对象完整引用后执行路径校验和文件清理；NotFound 幂等，失败保留意图，成功才确认。明确的永久删除不承诺保留历史回链，RF-903 自动孤儿清理另有更广的恢复保护范围。
- 验证计划：真实 SQLite 失败保持元数据/版本/HLC/文件；批次第N条失败全部回滚；Windows文件占用、重开重试、删除后确认SQL失败、引用复生、恢复换ID、未知ID/坏元数据、路径/目录连接、旧会话及GUI/CLI实际入口。之后完成 R/CORE/CLI、规范和独立复审；当前未宣称验证通过。

- RF-903 最终红灯：`rf903-red-host-r3.log`，编译完成，**0 passed / 2 failed / 0 ignored**、exit 101，两项均在真实有效附件文件消失断言失败（A/B 清理返回2、回收站清理返回1，释放字节均错误显示0）。完整回归保存为恢复目录 `rf903-regression.rs`（SHA256 `9D7F4F651B295DC5D1B5D5AFC70AD8F92335CDDD67E2D2000806E61379A29C5A`），校验后移出当前注册，供独立任务验证；未忽略测试、未改清理生产实现，RF-903 保持待修复。
- RF-016 阻塞：自动审批拒绝 Core 生产写入，认为核心不可逆目录删除缺少具体授权，命令未执行；已停止该写入并向用户请求明确批准。此前获批的 vault/GUI/CLI 草案和10项Host/CLI测试（尚未编译）共11文件已完整保存在恢复目录 `rf016-proposal/`，附原路径、SHA256、`tracked.patch` 和说明。8份原有源码恢复 HEAD，3份本次新文件校验后归档移出；未触碰用户文件、真实附件及 RF-104 草案。本项不计入已关闭，原编译路径保持可继续独立 Rust 工作。

### RF-021 执行记录（2026-09-27，阻塞）

- 基线 HEAD `5a450b75`。RF-018/019/020 前置已完成，RF-016 授权阻塞与本任务无依赖。正在核对三个导入策略、模板 hash 映射、重复源 ID、KeepBoth 名称/引用和历史替换，随后在一个 VaultStore 事务中提交数据库批次，附件与偏好阶段保持独立 RF-020 结果。

- 实施补充：RF-021 保持 Host 既有三策略、模板与历史兼容规则；同源 ID 依次操作，由准备阶段内存视图模拟先前计划记录。提交前以原 Vault 修订标识复核准备期间写入；视图过期时本批零提交，不覆盖并发修改。数据库成功后才发布计数，附件/偏好部分结果不变。
- 后续验收边界：既有全局 KeepBoth 的 ID map 不包含在预建 per-object 映射中，RF-022 须覆盖稳定映射与附件关联；CLI 旧 `core::import_vault` 尚未使用 Host 的模板/对象/历史事务，RF-024 迁移须验证模板写入错误传播及批次原子性，不因本轮 CLI 依赖验证通过而提前关闭此差异。

### RF-104 草案保全（2026-09-27，仍待授权）

- 为使独立前端任务可继续验证，将此前本 Agent 获批但未集成的7份文档/源码与2份新测试完整归档至恢复目录 `rf104-proposal/`，附 `tracked.patch`、原路径和逐文件 SHA256。两个被拒的 `useLlmChatCore` / `useLlmStreaming` 始终未修改，本次没有重试其写入。
- 校验归档后，仅将这些自有草案恢复到当前已提交 HEAD；RF-101/102/103 等已提交修复保留，3张用户 NSIS 图片保留。前端不再因未集成 RF-104 store API 而处于不可构建草案状态；尚未重新运行 F，不据此宣称检查通过。RF-104 继续待明确授权，未计入已关闭。

- RF-021 阻塞与草案保全：自动审批拒绝生产导入入口替换，认为可能影响后续导入数据且具体授权不足；已停止重试并请求明确确认。14份已批准源码、测试及规范草案保存至恢复目录 `rf021-proposal/`，附 SHA256 清单和 tracked.patch；校验后仅恢复本项10份已跟踪文件并移出4份新文件。生产导入入口未改，测试未编译/运行；Host 草案另保留 ObjectSummary/ObjectRecord helper 待修正记录。

### RF-110 执行记录（2026-09-27，完成）

- 基线 HEAD `5a450b75`。确认 applyTheme 的 DOM 模式使用 IPC，但色板和标题栏分别重新以 matchMedia 解析；两个来源不同时会产生模式与背景不一致。先统一一次解析结果并覆盖相反来源、显式模式及浏览器回退，按任务卡完成 F+WEB。

- 实施：applyTheme 只解析一次 resolvedMode，resolveActiveScheme 显式接收该值；标题栏只接收已选 scheme ID，状态栏复用 mode。既有 resolvedSystemTheme 调用、显式模式、强调色与 Android Material 覆盖顺序保持；未改移动端系统主题来源或跨请求调度。
- 修复前：新增 theme.test.ts 12项真实实现测试，7 passed / 5 failed、exit 1；两项 IPC/MQ 相反时色板与原生 RGB 不匹配，拒绝/超时三项重复查询 MQ 3次。修复后 theme + themeSchemes **33/33 passed**（包含新增12项），exit 0。
- F：完整 TypeScript exit 0（16.66s）；ESLint exit 0（11.02s）；默认 npm run test **149文件 / 1,263 passed**、exit 0（82.06s）。外围 Python 日志打印因 Windows 默认 cp1252 无法编码勾号返回1，但已记录 npm 实际返回0，完整日志确认无失败；未用外围退出码冒充测试结果。修改 TS/TSX/E2E 文件 Prettier 通过。
- WEB：Chrome **153.0.8010.53**。开发服务器默认4 worker两轮均为6 passed / 4 failed，失败是首4页等待启动层移除超过5s。第二轮 trace 显示各页586–599个请求，约5.35s仍加载深层TSX/CSS；未执行到主题输出，无pageerror。相同模块首批等待首字节约350–423ms、后续页约0.6–48ms，支持冷开发模块并发加载压力，不能据此归因具体CPU/磁盘。保留原断言/5s时限，使用项目CI相同的 `--workers=1` 后 **10/10 passed**、exit 0（28.3s）。未修改全局并发配置，未将4 worker冷启动失败记为已修复。
- 生产 WEB：`npm run test:e2e:production` 使用原4 worker配置 **16/16 passed**、exit 0（17.7s），包括原有8项主题测试、新增2项相反来源测试、启动与Markdown回归。原生 RGB/状态栏为模拟 IPC 参数；RF-112/RF-201 的实机验收仍保留。
- 证据：恢复目录 rf110-theme-red.log、rf110-theme-green.log、rf110-tsc.log、rf110-lint.log、rf110-vitest-full.log、rf110-web-native-theme*.log、rf110-web-production.log；失败上下文与trace归档在 rf110-web-first-failure/、rf110-web-trace-r2/。
- 最终结论：F、定向回归、开发单worker及生产WEB、独立只读复审均满足本项契约。更新主题 canonical §4.3.5，并给旧材质章节添加历史范围及当前实施记录链接；95项索引/卡片一致，27已关闭、4待授权阻塞。仅提交本项5个文件，原有3张NSIS图片保持；本项 **完成**，提交：本提交（按 RF-110 检索），未推送。

### RF-111 执行记录（2026-09-27，完成）

- 基线 HEAD `d323f6a8`。已确认 updateSetting 吞掉保存失败，调用方继续应用主题/显示成功；连续乐观写的失败回滚基线可能是另一未保存值，缓存也会包含其他键未确认值。准备显式结果、同键有序写入与确认快照、共享失败反馈，以及调用方成功动作守卫；不改变后端偏好接口。


- 实施：updateSetting 显式返回 saved/failed/stale，并提供返回后可再次检查的当前请求守卫。同键 FIFO 保存、维护后端已确认基线，旧成功可推进确认基线但不能执行新界面成功动作；连续失败回到真实已保存值。缓存快照排除全部未确认乐观值，明文镜像与加载共用每键顺序队列；加载按编辑代次合并，切账户/锁定清空会话写入队列，旧任务不能清理新队列。
- 调用方：共享 useSettingAction 仅为当前失败提示一次；外观应用等待保存，异步解析系统主题后复查请求并读取确认快照，避免夹带另一尚在保存的强调色。Android/安全/备份设置接入同一反馈。生物识别保留实际凭证操作已完成的事实，偏好失败/失效不显示整体成功或关闭验证；页面元数据以 object_update 为唯一写入，成功后仅更新会话内列表投影；备份提醒不再提前改写回滚基线，已发送通知与提醒时间保存结果分开处理。
- 回归：最小红灯2项均失败（旧返回 undefined、连续失败错误回到10而非已保存5），exit 1。修复后 Store/会话三文件 **64/64 passed**，共享 hook/页面两文件 **15/15 passed**。新增附属调用测试首轮7/9通过，两个失败仅为全局 i18n mock 返回 key 与预期译文不符；改为检查实际 mock key 后生物识别 **5/5 passed**，未降低生产失败断言。完整 F 再次包含上述全部测试。
- F：TypeScript exit 0（15.23s），ESLint exit 0（10.16s），默认 npm run test **155文件 / 1,302 passed / 0 failed**、exit 0（77.98s）。生产 WEB 使用 Chrome 153.0.8010.53、原4 worker，**16/16 passed**、exit 0（27.1s测试，30.24s命令）。本项没有 Rust 改动；浏览器模拟 IPC 不充当 RF-112/201 的移动实机验证。
- 证据：恢复目录 rf111-settings-red-minimal.log、rf111-settings-targeted1.log、rf111-ancillary-tests.log、rf111-bio-targeted-r2.log、rf111-tsc-full.log、rf111-lint-full.log、rf111-vitest-full.log、rf111-web-production.log。独立只读复审未发现阻断问题，状态规范 §5.3 已同步确认基线、返回契约与部分成功边界；全局主题协调仍由 RF-112 处理。
- 最终结论：F、定向回归、生产 WEB 与独立复审满足本项；95项索引/任务卡一致，28已关闭、4待授权阻塞。仅提交本项19个文件，3张用户 NSIS 图片保留；本项 **完成**，提交：本提交（按 RF-111 检索），未推送。


### RF-202 执行记录（2026-09-27，完成）

- 基线 HEAD `3d877494`。四处 isMobilePlatformSync 将 iOS 错送 APK 缓存/检查/下载/安装；仅改 isAndroidSync 又会把 iOS 送入桌面管线。选用已存在的异步 getPlatform，四阶段在更新副作用前解析平台；iOS 明确 unsupported、横幅隐藏、关于页显示安装渠道说明。公开 APK helper 另设平台守卫，防止绕过 Store；保留下载取消和单任务语义，不改 Rust、分发源或签名。
- 验证计划：四平台 Store 矩阵、iOS 注入可下载/已下载状态、APK helper 直接调用、异步平台解析期间重复操作与取消；Android/桌面旧流程及关于页不误报最新版/网络失败；完成 F。按本项卡片不要求实机，不将模拟 IPC 测试写作 iOS 原生更新已支持。

- 实施：Store 在等待平台前登记唯一任务，解析后重查 token/controller/安装状态身份；iOS 用 unsupportedReason='ios' 区分能力不支持，清空旧错误及无效待安装引用，保持横幅隐藏和原成功检查时间。APK helper 使用固定 UnsupportedApkUpdateError/code，版本检查返回 unsupported；取消在平台解析前后均阻止更新副作用。关于页增加独立说明与双语文案，旧 available/error/up-to-date 分支分别保留。
- 修复前最小 iOS 检查回归 **1 failed / 2 skipped**、exit 1（1.66s）：确实调用 APK 缓存；skipped 为同文件仅筛选红灯时未执行的旧用例，未新增永久 skip。UI三文件先完成 **24/24 passed**、exit 0（3.82s）。Store/helper 首轮 **38 passed / 6 failed**、exit 1：一项暴露 unsupported 尚保留注入的待安装资源引用，已补清空；其余五项为新增测试漏导入及异步平台续点前读取旧计数，修正测试装配与等待实际 dispatch，保留行为断言和首轮日志。完整验证尚未完成。

- 修正后 Store/helper 两文件 **44/44 passed**、exit 0（21.69s）；三文件 UI 24项保持通过，独立只读复审无阻断问题。F：TypeScript exit 0（27.45s）、ESLint exit 0（9.05s）、默认 npm run test **155文件 / 1,330 passed / 0 failed**、exit 0（90.90s）。新增28项覆盖四平台、失效待安装状态、平台解析期间去重/取消和 iOS 界面结果。
- 生产 WEB：Chrome 153.0.8010.53、原4 worker，**16/16 passed**、exit 0（19.0s测试、22.01s命令）。保留构建既有 chunk 大小与静态/动态混用提示，未调整阈值。没有实际网络下载、APK安装或桌面重启；本项前端平台路由证据不替代 iOS 原生验证。
- 证据：恢复目录 rf202-ios-store-red.log、rf202-store-updater-targeted1.log、rf202-store-updater-targeted2.log、rf202-ui-targeted.log、rf202-tsc-full.log、rf202-lint-full.log、rf202-vitest-full.log、rf202-web-production.log。修改 TS/TSX 的 Prettier、双语 JSON、文档相对链接和 diff check 通过。
- 最终结论：F、68项相关定向回归、生产 WEB 与独立复审满足本项；canonical 自动更新 §5 与移动端入口文档同步。95项索引/任务卡一致，29已关闭、4待授权阻塞。仅提交本项14个文件，原有3张NSIS图片保留；本项 **完成**，提交：本提交（按 RF-202 检索），未推送。


### RF-905 执行记录（2026-09-27，进行中）

- 基线 HEAD `e627d160`。RF-202 已完成；RF-208/201/203 所需 Android 工具链和 RF-204 所需 macOS/iOS 编译环境仍不可用，RF-112 依赖 RF-201；继续无前置的 P1 目录互斥任务。有效工作区仅 C 盘恢复副本。
- 三方只读核查确认：CLI 构造失锁继续且构造前已写日志，GUI 不持同一根锁；Store/Session/FS 克隆和裸路径 blocking worker 可以活过 Service。同步 outbound 活动登记晚于派发、入站登记无共同准入门闩、mDNS 启动失败可能遗漏 accept worker，stop 的 abort 不证明退出。仅增加 Service 字段不能满足互斥生命周期。
- 实施边界：Core 严格 Result 构造、Arc 目录 owner；VaultStore 在打开/迁移前持 opaque lifetime pin，FS 克隆与真正 worker 也保活；活动登记和维护准入共用短门闩，待执行/在途发布未退出时明确拒绝维护。GUI 初始化/切换失败保留合法旧状态，CLI 移除二次锁并在成功拿锁后初始化日志；同步加入真实可等待收尾与启动失败回收。保留移动端 no-op，不宣称锁远端 SAF；不改 RF-903 的扫描/删除算法。
- 验证计划：Windows 真实独立子进程竞争、存活 DB/Session/FS/worker 持锁、失败零业务写入、不同根失败与同根显式复用、同步/导入发布屏障和维护拒绝；完成 R/CORE/CLI，必要 Host 启动验收。当前仅设计与预研，未宣称已修复或验证。


### RF-905 执行记录（2026-09-27，阻塞）

- 基线 HEAD `e627d160`。Core、GUI、CLI、Sync 的获批草案共46份源码，未格式化、未编译、未运行回归；Sync生产接入和Host Vault切换/迁移/附件worker写入在CreateProcess前被自动审批拒绝，理由是关键数据及生命周期风险、现有继续指令不足以涵盖具体修改。未重试或绕过被拒动作。
- 全部写入者冻结后，以SHA-256清单、原路径、tracked.patch及缺失API说明归档至恢复目录 `rf905-proposal/`；44份tracked源码恢复HEAD，2份自有新文件验证后移出。其余报告和3张用户NSIS图片保留，源码没有残留未集成依赖。已发出具体授权请求，本项保持阻塞，RF-903仍依赖本项；继续首个无阻塞任务RF-008。

### RF-008 执行记录（2026-09-27，完成）

- 基线 HEAD `e627d160`。GUI与CLI重复归属校验、字段恢复、版本/快照/审计；GUI对于标签null与非法值的处理仍不同于RF-007。拟共享Core回滚用例，统一字段语义并返回对象提交后快照/审计的分别结果；GUI保留可观测best-effort和同步触发，CLI部分成功明确已提交状态且不显示整体成功。仅使用合成TempDir，完成R/CORE/CLI再独立提交。

- 实施冻结：新增Core共享回滚及typed阶段/部分结果，GUI与CLI各为薄适配；GUI标签契约收敛至RF-007，版本溢出和序列化均在首写前退出。CLI后续历史失败仍尝试审计，并以双语提示已恢复的事实；历史列表将共享摘要 `diff_rollback` 映射为可读本地化文本。独立复审发现并修正内部摘要键直接展示问题，其余无阻断。
- 新增14项真实回归（Core8、GUI2、CLI命令3、CLI渲染1），覆盖对象/HLC事务失败零写、历史/审计独立失败、归属与标签、两端和Core完整数据等价、确认/取消与中英文通知。仅测试dev依赖新增已锁定rusqlite；源码冻结后启动顺序验证，workspace fmt exit0（2.66s）、CLI fmt exit0（0.56s）。此时其余检查尚未完成，不计入已关闭。

- 首轮Core定向编译通过，运行 **0 passed / 8 failed**、exit101（238.20s）；均在测试夹具初始化被SQLite template_type CHECK拒绝，尚未调用回滚。将夹具的非法custom改为合法user，保留所有断言和首轮日志 `rf008-core-targeted.log`，随后重新运行。

- 第二轮Core定向 **8 passed / 0 failed / 0 ignored**、exit0（46.04s，实际测试0.55s）；GUI完整快照模块 **11 passed / 0 failed / 0 ignored**、exit0（363.63s，测试0.89s），包含RF-006旧归属/数据保持回归以及新增GUI标签/真实写入失败警告用例。CLI及全量队列继续，尚未关闭。

- CLI回滚命令 **11 passed / 0 failed / 0 ignored**、exit0（187.83s，测试4.92s），含原RF-007与新增确认/取消、真实历史/审计失败及Core等价用例；双语历史渲染 **1 passed / 0 failed / 0 ignored**、exit0（3.10s，测试0.02s）。14项新增及原定向全部通过，继续Clippy与R/CLI完整检查。

- Workspace Clippy exit0（84.68s）。R完整 `cargo test --verbose` exit0（906.97s），19组结果合计 **1,180 passed / 0 failed / 3 ignored**，包含GUI554、Core258、Vault190；仅沿用原有两项legacy field及P025手动性能工具跳过。CLI全目标Clippy和完整测试继续；不提前处理运行中的Cargo锁文件。

- CLI全目标Clippy exit0（64.62s），完整 `cargo test --verbose --no-fail-fast` exit0（141.51s），**182 passed / 0 failed / 1 ignored**（180单元+2集成，既有i18n文档示例跳过）。全部Cargo退出后，仅恢复CLI锁文件自动刷新的5个本地crate版本，保留本项两个lockfile的测试依赖引用；无依赖包升级。
- 最终结论：R/CORE/CLI、14项新增回归、原RF-006/007、独立代码复审与三份canonical规范均满足。本项 **完成**，提交：本提交（按RF-008检索），未推送。95项索引/任务卡一致，30已关闭、5待具体授权阻塞；仅暂存本项18个文件，保留3张用户NSIS图片。验证日志为恢复目录 `rf008-r2-*.log`，首轮失败日志保留；下一RF-010只读方案在 `rf010-readonly-preplan.md`。
