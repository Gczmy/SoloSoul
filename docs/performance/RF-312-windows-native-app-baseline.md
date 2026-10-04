# RF-312 Windows 原生应用采样（2026-09-30）

本记录定义并执行真实 Tauri Release 与 WebView2 界面采样，只使用 [RF-312 合成 Vault](RF-312-native-vault-baseline.md)。`native-perf` 是非默认 Windows feature；默认构建的入口、数据目录和路由加载保持原行为。当前尚无成功GUI性能样本，TCP CDP连接仍被拒绝；2026-10-04已取得一次SDK交接后文档绑定诊断；Windows阶段不足以关闭 RF-312：OCR、附件预览、系统睡眠恢复及 macOS/Android 尚未取得同口径数据。

## 隔离入口

测试程序在创建 Tauri Builder、插件或线程前检查参数。准备模式只复制已验证的 100/5,000 对象合成数据，以真实 VaultService 解锁副本并验证对象、搜索、Profile 和偏好；源数据只读。每次只能创建绝对新目录，最后发布 ready 标记，运行时核验内容 SHA 并原子消费一次。已有目录、链接、源目录内输出、错误 KDF、篡改副本及占用端口均拒绝，不回退默认路径。

每轮具有新 identifier，并原子声明实际 Windows Roaming/Local Known Folder 下的同名新目录。Vault、日志、导入暂存、插件、模型与 OCR 偏好使用显式隔离路径；插件初始化失败直接终止，不走生产/共享临时目录兜底。WebView2 使用新数据目录和仅 loopback 的 CDP 端口，runner 在输入公开合成密码前检查实际 browser 进程的 user-data-dir、原生 PID 和 runId。API user-data-folder 为本轮 webview 根目录，WebView2 Runtime 在其下自动追加 EBWebView；browser 命令行必须精确指向该子目录，根目录直用、其他子目录或重定向均拒绝，见 [Microsoft 的 WebView2 DevTools 说明](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/devtools-mcp-server#step-2-find-the-webview2-user-data-directory)。Windows Known Folder 路径不能仅靠 APPDATA 变量替换；WebView2 也可能被环境/Registry 配置覆盖，见 [Microsoft API 说明](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl?view=webview2-1.0.4022.49)。测试进程拒绝继承 WEBVIEW2\_\* 或 PDFIUM_LIBRARY_PATH，再设置本轮 owned 值；不修改系统环境或 Registry。

runner 在运行任何 EXE 前流式检查四个 feature 专属常量，缺一即拒绝。这只防止误传默认构建，不是二进制签名或来源认证。需要使用自己从本提交构建的测试 EXE。进程清理必须重新核验 PID、创建时间和可执行路径，只结束本轮自有进程；不能核验或清理未完成时保留失败记录并停止后续样本。

## 复跑步骤

先按后端基线文档用 Release 生成并在第二进程验证 `vault100` 和 `vault5000`，不要传真实 Vault。需要当前仓库已有 Node 依赖、Rust/MSVC 与 Windows WebView2 Runtime；无需 Windows Sandbox 或额外账户。以下命令在 `tauri/` 运行：

```powershell
npx tauri build --config src-tauri/tauri.native-perf.conf.json --features native-perf --no-bundle --ci
node --test src-tauri/src/native_perf/observer.node.test.mjs scripts/native-perf-run.test.mjs
```

独立 config 不生成安装器，beforeBuild 只暂存桌面公开资源、检查 TypeScript 并构建 Vite，避免重写用户 NSIS 图片。测试feature额外启用 tauri/devtools，并只在隔离窗口允许DevTools以供CDP连接，不自动打开调试面板；它是带观察的Release基线。Release 优化沿用项目配置（opt-level=3、thin LTO、codegen-units=1），不是 Debug 或禁用媒体依赖的简化版本。

在新的专用 TEMP 父目录建立 `bin`，将 `target/release/solo_soul.exe` 与同级运行库 DLL 复制到 bin；资源必须与 EXE 相邻并保持下列 Windows 打包布局。复制前检查源、父路径和目标没有链接/reparse，末级目标必须新建；保留资源 SHA 清单。

| 仓库来源（相对 tauri/）                            | bin 内目标             |
| -------------------------------------------------- | ---------------------- |
| src-tauri/resources/docs                           | docs                   |
| src-tauri/resources/pdfium/pdfium.dll              | pdfium/pdfium.dll      |
| src-tauri/resources/models/pp-ocr-v6-small         | models/pp-ocr-v6-small |
| src-tauri/resources-desktop/SoloSoul_plugin_market | SoloSoul_plugin_market |

即使当前阶段不执行 OCR，也要保持模型资源一致：首装会复制随包模型，缺资源会改变启动 I/O。插件市场没有 Release 源码目录兜底，缺少市场资源会使隔离初始化失败。不要把测试 EXE 直接放到正在使用的安装目录。

```powershell
# 三个变量均为明确的绝对路径；sampleParent 是刚创建的独立父目录。
node scripts/native-perf-run.mjs --exe (Join-Path $sampleParent 'bin/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'windows100') --samples 5
node scripts/native-perf-run.mjs --exe (Join-Path $sampleParent 'bin/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault5000') --output (Join-Path $sampleParent 'windows5000') --samples 5
```

output 末级目录必须不存在，最少三次，建议五次。每次有独立副本和 WebView2 profile。脚本保留 sample JSON、失败原因与原始结果，不重用 consumed 目录；退出码 0 要求所有请求样本的六项 UI 行程及进程清理成功；IPC/内存仍可能因证据无效而返回带原因的 null，须分别核对有效样本数。原始合成源应以全文件 SHA 前后对照确认未变。

## 采样口径

- 启动：spawn 到合成账户密码表单与可用提交按钮可见，再经过两帧；不包含独立准备进程；包含运行入口预检、新 WebView2 profile、正常产品初始化、CDP 连接及末次 observer 检查，时钟在随后 browser 目录/CIM 核验前停止。另记录 CDP attach 与前端 performance marks。每次新进程/新 profile，但未清系统文件缓存，不称操作系统冷启动，也没有同 profile 的热启动对照。
- 主密码解锁/再次解锁：真实输入与提交，到认证侧栏可见、首页路由成立、两帧；不声称首页所有异步卡片均加载完毕。KDF 为 Argon2id 64 MiB/3次/并行4。
- 工作区：真实点击 Identity 后 Clear，等 `/workspace` 无过滤并渲染首批 50 张对象卡片，再经过两帧。固定对象总量 100/5,000；没有扫描滚动全列表。
- 搜索：先从首页真实 Search 卡片导航至搜索页并等待输入框，再开始计时；计时覆盖填入 needle、等 5/50 张结果卡片和两帧；包含产品 300ms debounce。GUI limit=50，5,000 对象的完整后端命中是250，但本口径不把 GUI total/hasMore 当完整总数。
- 锁定：真实 Lock Vault 按钮到密码表单可见、侧栏卸载和两帧。它是应用锁定，不代替 Windows 睡眠恢复。
- 内存：各阶段之外取核验过的原生根进程与后代进程 working set 快照，合计可能重复计入共享页。缺任何必须进程的身份/内存证据时合计为 null 并记录原因；不称峰值、独占物理内存或 JS heap。
- IPC：初始化脚本在 Tauri core/plugin 脚本之前观察 Windows 默认 http invoke transport 的 JavaScript 调用尝试，包括 app/plugin/channel 命令，只存命令名与时刻，不读参数、响应或密码。fallback、替换 observer、溢出、额外 document/page/frame、身份/timeOrigin 变化等使整个样本的次数为 null。不是操作系统全部 IPC 或后端内部消息数。
- 正常原生更新联网保留；不拦截 reqwest 或伪造后端返回。每轮没有真实账户、云配置、设备配对或真实附件。Windows 主题是固定英文、浅色/ocean，不能据此验收其他主题或平台材质。

中位数为排序后中间值，偶数取中间两项均值；P95 使用 nearest-rank。统计仅使用该阶段成功的有限时延，同时记录成功数/请求数，失败样本仍完整保留。阶段之间的 IPC 纳入全程次数，不能简单把阶段次数相加当总量。采样 API/CDP 也有开销，数值不是未经观察的用户体验下界。

## 失败样本与回归

首次100对象档请求5次，只执行第1次后停止：准备exit0，GUI在CDP连接前exit1，六阶段成功数均为0，中位数/P95为null。[原始失败JSON](rf312-windows-native-preflight-failure-100.json)与[owned清单](rf312-windows-native-preflight-failure-owned.json)完整保留。该次不是成功的启动性能样本，也没有进入真实账户界面；未核验到可清理的存活PID，因此没有结束任何不明进程。

清单的roaming/local字段是创建后的canonical路径，位于本机Package LocalCache下，ownedPaths仍保留逻辑AppData路径，两组不一致导致运行入口拒绝。修正为创建后立即一致记录canonical字段及ownedPaths，消费时只解析当前Known Folder加固定identifier的精确候选目录；不接受任意manifest路径或目录前缀。回归先真实红测1 failed，再6 passed；其中新增用例覆盖普通与extended path表示差异，不宣称完整模拟MSIX重定向。真实修正后采样结果另行记录。

其余五个UI阶段的前后observer核验及全部内存采集均在阶段时钟之外。不同阶段之间的检查会影响操作节奏，不能把各阶段时延相加当完整操作链的耗时。
第二轮使用修正canonical路径后的Release，准备exit0且consumed成功，实际WebView2的EBWebView目录及文件系统核验正确。browser参数包含正确的loopback地址和50376端口，但60秒内CDP连接持续ECONNREFUSED，六阶段仍无成功样本，不能将62秒失败耗时当正常启动值。[第二轮原始失败](rf312-windows-native-cdp-failure-100.json)保留超时和清理失败；当时的8进程working set快照573,050,880 bytes只描述失败过程，不是稳态或峰值。

清理的PowerShell 5.1管道把JSON数组包成外层数组，逐PID转换在Kill之前失败；之后按记录中的精确PID、创建时间、父PID与EXE路径重新核验，仅结束本轮根进程及browser，其余后代随之退出，见[独立清理记录](rf312-windows-native-cdp-failure-cleanup.json)。采样脚本另修正数组展开、错误证据及失败退出句柄。Release DevTools开关作为下一轮排除实验，不把它预先认定为端口缺失根因，也不修改Registry或系统网络策略。

## 本轮结果与未完成范围

启用DevTools后的第三轮Release构建exit0；同配置6项Rust隔离入口测试通过，runner24项与observer9项Node测试通过。100对象实际请求并执行5次，每次准备exit0、消费标记成功、正确EBWebView目录、8个可核验进程，但均在60秒CDP等待中返回ECONNREFUSED；六阶段成功数均0/5，中位数/P95均null，[完整第三轮结果](rf312-windows-native-devtools-failure-100.json)保留全部失败。启用DevTools未解决连接问题，不能将其称为已确认根因修复。

五轮清理均complete，40个已记录PID在收尾只读核验中均已不存在；前两轮失败目录和程序仍保留以供排查。原始100/5000两档的14个文件SHA全部保持，三个既有NSIS图片SHA也保持。5000对象GUI测量因共用CDP前置失败尚未执行，不能写成该档性能结果。源码、三次Release程序SHA、资源清单、测试计数与失败记录汇总于[本轮结构化证据](rf312-windows-native-infrastructure-2026-09-30.json)。

默认完整Rust检查曾因18项PDFium加载测试未找到DLL而失败；只在测试子进程显式提供仓库PDFIUM_LIBRARY_PATH后，同样完整范围25组、1,363 passed、0 failed、3项既有ignored，exit0；运行后撤去该变量，再运行native检查。默认与native的Clippy、fmt以及实际Release前的TypeScript/Vite均通过；未修改阈值、跳过失败或重跑未受影响的前端Vitest。

本机为Dell OptiPlex7070、i7-9700（8核/8线程）、约16GiB内存、Windows11 Enterprise LTSC 26100、Balanced电源方案、Node24.16.0/Rust1.96.0。第二轮浏览器进程实际为WebView2 154.0.4258.37，已观察到正确remote-debugging-port/address参数而无监听。微软上游[WebView2Feedback #5718](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5718)报告153运行时有相近连接拒绝症状；它与本机版本和宿主不同，只作候选线索，不能据此确认154回归。需要先在可连通的受控Runtime/宿主环境证明CDP前置，再运行两档真实UI流程；不修改本机Registry或默认Runtime来凑数。

本轮完成的是隔离入口、采样脚本、失败回归与收尾验证。登录、工作区、搜索等实际DOM选择器尚未在连通CDP后走通；不能称该脚本已完成六阶段端到端验收。OCR图片/扫描流程、附件预览、系统睡眠恢复、同profile热启动、macOS/Android同口径数据及内存峰值尚缺，RF-312继续保持[!]。

## CDP 单轮现场诊断与 TEMP 修正（2026-09-30）

`native-perf-diagnose.mjs` 复用上述隔离入口和严格进程清理，只启动一次公开合成 GUI，在启动后计划 5/15/30 秒作三次现场查询；实际查询耗时和时刻另存 JSON。它不输入密码或操作 UI，不产出性能指标。输出目录必须新建，使用同一已构建的 native-perf EXE 与 fixture：

```powershell
node scripts/native-perf-diagnose.mjs --exe (Join-Path $sampleParent 'bin/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'cdp-diagnostics')
node --test scripts/native-perf-diagnose.test.mjs
```

每次先核验 consumed 的 runId/PID/端口、活着的原根进程、精确 browser 目录及 PID/创建时间/父 PID/EXE。PowerShell helper 再通过 fresh CIM、StartTime 和有限查询句柄复核同一身份，之后只读取这两个进程的 Token 权限、AppContainer、包身份、文件版本与三个限定 browser 参数；不保存完整命令行。监听查询只针对已核验 root/browser；查询失败单列 unknown。只有自有 browser 在预期端口监听、参数仍匹配且原身份再次通过，才允许请求 loopback `/json/version`，并禁止 HTTP 重定向。

首轮三次 helper 均在 C# 编译阶段失败，runner exit1；隔离 GUI 的准备与清理完成，公开源 Vault 未变。离线对照使用同一 helper、合成输入及同一 temp 目录，仅改变 TEMP/TMP 表示：普通路径编译 exit0，Rust canonical 的 `\\?\` 扩展路径编译 exit1；最小 Add-Type 同样抛出 `System.NotSupportedException`、HRESULT `0x80131515`。两组都没有实际 PID 查询。wrapper 修正为先确认该 owned temp 是正常目录且 realpath 等价，再仅为 helper 子进程使用普通路径；不改变全局 TEMP、原生隔离 manifest 或 helper 的身份规则。这个修正解释诊断编译失败，不解释此前 CDP 无监听。

第二轮 runner exit0、三次现场记录完整：宿主 2.13.2 与实际 WebView2 154.0.4258.37 均为 Medium（RID8192）、`isAppContainer=false`、unpackaged；browser 参数为端口62537、地址127.0.0.1及本轮精确 EBWebView。三次监听查询均成功但返回空，故没有发送 HTTP/CDP 请求；没有密码输入或 GUI 行程，performanceMetrics 为 null。该轮排除了提升权限、AppContainer 与包身份这三种本轮解释，不能据此确认 Runtime 回归或认定已有性能基线。

两次隔离运行均清理9条已记录进程身份，收尾18条 PID/创建时间核验均已不存在；每次100对象源的7个文件 SHA 前后相同，3张既有 NSIS 图片 SHA 保持。[完整现场与离线因果证据](rf312-windows-cdp-diagnostics-2026-09-30.json)保留首次失败、第二次成功诊断及脚本 SHA。新增11项 Node 边界测试与既有runner24项、observer9项通过；PS5.1 helper 编译及16项合成断言通过，空身份拒绝 exit1且未进入 live 查询。

诊断 exit0 只表示记录完整、源未变和清理完成。仍需受控 Runtime 对照或原生 Chromium 日志定位无监听，再取得成功 CDP/真实 UI 行程；不修改本机默认 Runtime 或 Registry。RF-312保持待验证，其他缺失范围沿用上节。

## 显式 Chromium 日志实验（2026-09-30）

诊断脚本新增可选 `--chromium-log`，只向原生 run 入口传递严格的 `--native-perf-diagnostics chromium-log`。普通诊断和 benchmark 的原有 browser 参数保持，prepare 不接受该选项；日志仅用新建 owned temp 下固定文件名，保留原生 TEMP/TMP 的 canonical 表示。启用后发布绑定 runId/PID/端口的独立标记，benchmark 遇到标记即拒绝接纳，避免日志开销混入性能样本。

```powershell
node scripts/native-perf-diagnose.mjs --exe (Join-Path $sampleParent 'bin/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'cdp-chromium-log') --chromium-log
```

使用 [Microsoft WebView2 flags](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags) 和 [Chromium 日志说明](https://www.chromium.org/for-testers/enable-logging/) 所述的 `--enable-logging --v=1 --log-file="<owned absolute path>"`；Release 的空值 enable-logging 选择文件目标，见 [Chromium 实现](https://chromium.googlesource.com/chromium/src/+/lkgr/chrome/common/logging_chrome.cc)。helper 子进程环境先移除继承的诊断变量，未请求日志时完全省略 LOG_FILE，再使用已验证的普通 TEMP 表示，保留 PS5.1/.NET 的无日志兼容路径。

实际 Release 构建通过（Cargo 12m43s，beforeBuild TypeScript/Vite 通过，Vite 6.84s）；新增入口检查后 Rust 9 passed，Node observer/runner/diagnose 合计48 passed，PS5.1 离线41条断言及默认无日志环境互操作通过；fmt 与 native all-target Clippy -D warnings 通过。Release EXE SHA 为 `292B1FF4D932CC0108B849763F605BE911E219DC277BC2B690BE12AEE4AFA2F1`，沿用95项同 SHA 公开资源，不重写用户 NSIS 图片。

100对象单轮实测中3次现场查询均完整，实际 browser 三个日志参数精确匹配；生成2,229字节 owned 普通文件，SHA 为 `6162085e244b8181007b48925da8dd1d6ea9a37709197f3869b580a990ad6a6c`。3次 TCP 查询仍无 owned 监听，没有发送 HTTP/CDP 或密码/UI操作，performanceMetrics 为 null。10个清理记录 PID 收尾只读 CIM 查询均已不存在，公开源7文件和3张用户 NSIS 图片 SHA 保持。[完整日志、原始现场记录与验证证据](rf312-windows-chromium-log-2026-09-30.json)保留失败前置与本轮结果。

日志确认 `connect-src 'self'` 拦截 `http://ipc.localhost/set_titlebar_color`，随后 Tauri 切换 postMessage；这是需要另项修复的生产 CSP 配置问题，也使现有 IPC observer 无法接纳完整次数。它不证明 CDP 无监听的原因。日志末尾 Network/GPU 退出发生在末次观察之后的主动 owned 清理中，不能据此认定产品启动崩溃。RF-312仍缺成功 CDP/六阶段 UI、5000对象 GUI、OCR/预览、系统睡眠/同 profile 热启动、多端与内存峰值证据，保持待验证。

## RF-1059 本地 IPC CSP 修复后复验（2026-09-30）

RF-1059 将生产 `connect-src` 补为 `'self' ipc: http://ipc.localhost`，其余指令与默认 nonce/hash 注入保持。真实浏览器先用旧配置复现本地 fetch 失败，修复后的4项CSP正反例与原16项生产测试全部通过；负例直接断言 connect-src 违规事件与目标来源。浏览器网络响应为合成 fixture，不证明 Rust 命令执行。

相同公开100对象、原生隔离入口/脚本和95项资源，使用新Release（EXE SHA `0708B2C24B6B36C076E4B83393742635CD5D396B35ED341379D1B868D661FBAC`）与新profile复验：3次查询完整、1,370字节有效日志中先前三条CSP拒绝/fetch失败/postMessage回退均消失，10个记录PID收尾已不存在，源7文件SHA保持。[完整前后对照](../verification/rf1059-windows-local-ipc-csp-2026-09-30.json)保留原始记录和限制。

CDP仍无owned监听，performanceMetrics为null；不能恢复未经observer接纳的完整IPC次数或关闭RF-312。后续只读核对发现，[#5718作者](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5718#issuecomment-5715071958)后来无法在最小项目复现，并[确认应用服务器先就绪时153调试成功](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5718#issuecomment-5801655559)。该条候选不能当作153/154普遍Runtime回归或本机根因证据。下一步可评估[官方Fixed Version Runtime隔离对照](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution#the-fixed-version-runtime-distribution-mode)，但当前工具拒绝继承WEBVIEW2覆盖，尚未实现受控选择入口，也未下载或更换默认Runtime。

## 原生 TMP 单变量诊断（2026-09-30）

新增显式 `--ordinary-native-tmp`，必须同时指定 `--chromium-log`；对应严格原生 run mode `chromium-log-ordinary-tmp`，prepare 不接受。默认入口、原日志模式及性能 runner 的调用保持。只将新测试主进程的 `TMP` 从 `\\?\` canonical 表示改为同一 owned temp 的普通本地绝对路径，`TEMP`、`USERPROFILE`、WebView 数据目录与 browser flags 的生成规则保持。标记 `native-perf-ordinary-tmp.json` 在设置后读取这四项实际环境值，绑定 root/runId/PID/port；Node 每次观察前精确核验，不能仅凭路径归一化接受另一个表示或目录。既有 Chromium 日志标记继续表示 TEMP 保持，并使 benchmark 拒绝该诊断；本模式额外改变 TMP，不将日志标记的 `nativeTempUnchanged` 扩张解释为全部环境均未变。

```powershell
node scripts/native-perf-diagnose.mjs --exe (Join-Path $sampleParent 'bin/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'cdp-tmp-baseline') --chromium-log
node scripts/native-perf-diagnose.mjs --exe (Join-Path $sampleParent 'bin/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'cdp-tmp-ordinary') --chromium-log --ordinary-native-tmp
```

选取 TMP 是因为 [Windows GetTempPathW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-gettemppathw) 优先读取 TMP，其后才是 TEMP；[Chromium 公开 GetTempDir 实现](https://github.com/chromium/chromium/blob/main/base/files/file_util_win.cc#L702) 在这个入口没有去掉扩展前缀。这支持对照候选，不能证明 Microsoft WebView2 当前 Runtime 内部实现或无监听根因。当前短根目录没有超过长度限制的证据；也不能由此排除所有嵌套路径问题。[公开 DevTools handler](https://github.com/chromium/chromium/blob/main/content/browser/devtools/devtools_http_handler.cc#L256) 先建立监听再写 DevToolsActivePort，所以不能仅凭该文件缺失归因于路径。

两轮按顺序使用同一新构建 EXE、相同95项资源和100对象公开源；每次使用新 owned root/profile、端口与运行ID，不重用 consumed 目录。两次输出目录名等长，只改变受控参数 TMP 的表示。环境标记证明新主进程设置后的值，不能单独证明 browser 子进程继承或使用了 TMP；实际 browser 身份、flags、监听仍分别查询。两轮均为故障诊断，不输入密码、不执行UI、不产生性能指标。

实际验证：新模式新增3项Rust用例后，首次12项运行为5 passed/7 failed；根失败是既有“准备失败保留owned路径”用例得到2条而非3条路径，随后6项因锁poison连带失败。原断言没有返回实际prepare错误，早退原因未确认；未更改代码、断言或范围，精确重跑该项1 passed，再完整12 passed/0 failed（6.77s）。首次失败原文仍保存，不能将复测写成修复了该偶发现象。Node为56 passed/0 failed/0 skipped（diagnose23、runner24、observer9）；格式、语法、定向Prettier和native all-target Clippy均通过。专用Release构建exit0，Rust20m24s，beforeBuild TypeScript/Vite通过；EXE SHA `6E402CD773E4C0845E60DE9D1CAEE373D4309BAA43D84A77D557B9C034035E7F`，95项资源与旧清单SHA一致。

canonical对照与ordinary TMP实验均exit0、三次现场记录完整；两轮实际Runtime均为154.0.4258.37，browser目录和日志flags匹配，三次TCP查询均无owned监听，未发HTTP/CDP、未执行密码/UI流程，performanceMetrics仍为null。ordinary标记的TEMP/profile/UDF保持canonical表示，TMP为同目录普通表示；两轮普通temp根均96字符。两份日志各1,617字节，SHA分别为 `31668E7927C198332D7BAE71E85A7962748626B343660EFF53D14A04E28E7F4D` 与 `4ADAA79E23B9B4843F5A07DBC3C0AFFC3E5ACC6055224314668F4BDE4B243FEC`。20条记录PID收尾fresh CIM核查全部不存在，两档源14文件与用户3张NSIS图片SHA保持。

[完整对照及检查证据](rf312-windows-ordinary-tmp-2026-09-30.json)保存两轮原始记录、实际主进程环境标记、日志、首轮测试失败、复测、程序/资源/源码SHA与清理。结论仅为本轮主进程TMP表示变化未恢复CDP；不排除直接读取TEMP的组件、UDF等其他路径入口、Runtime行为或所有嵌套路径限制，也不证明browser继承/使用TMP。后续受控Runtime选择仍需独立实施、核验实际browser路径/版本/身份；本轮没有下载/复制Runtime、修改Registry或默认Runtime。RF-312保留[!]，208/249不变。


## 复制已安装 Evergreen 的受控 Runtime 对照（2026-09-30）

诊断入口新增成对参数 `--runtime-source ABS --runtime-version VERSION`，必须同时指定 `--chromium-log`，与 `--ordinary-native-tmp` 互斥；prepare 和 benchmark 的原有参数保持。源目录限定本机 `ProgramFiles(x86)/Microsoft/EdgeWebView/Application/<version>`。本次认证的是已安装 Evergreen 的本地副本，`sourceKind` 为 `copied-local-evergreen`；[官方 Fixed Version 分发](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution#the-fixed-version-runtime-distribution-mode)要求单独的版本包，本轮未下载该包。

Node 先检查新版 EXE 特征，再认证三个核心文件的真实 Microsoft Authenticode、精确 FileVersion/ProductVersion 和 AMD64 PE，记录全部文件 SHA/字节及完整目录集合（包括空目录）。只向新的 owned `root/runtime` 进行排他复制；前后源/副本全树必须一致，完成清单排他发布，失败保留阶段与部分副本。上限为 3000 文件、3000 目录、1 GiB、4 MiB 清单，各相对路径最多64段。版本为四段 canonical u16，目录与文件不得有大小写冲突、链接或额外/缺失项。

Rust 独立验证 owned 身份、清单、全树与核心 PE，然后仅为该新进程设置 `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`。显式目录 [SDK 版本查询](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl?view=webview2-1.0.4022.49#GetAvailableCoreWebView2BrowserVersionString)必须匹配，输出由 `CoTaskMemPWSTR` RAII 释放；默认入口不设置该变量或调用此查询。TEMP/TMP/profile/UDF 和 browser flags 生成规则保持。Rust 的签名/文件版本字段属于 Node 来源声明，不能称作 Rust Authenticode 重验。

主进程环境与 SDK 标记尚不足以接纳观察。每次现场诊断还核对实际 browser 的 owned 身份、物理 `root/runtime/msedgewebview2.exe`、文件版本及 SHA；probe 前重查相同 root/browser 身份、UDF 和 EXE SHA。第二次沿用本次 helper 的版本字段，没有再次读取版本/签名，也不证明全部 DLL 已加载。任何默认安装路径回退均拒绝，诊断始终 `performanceMetrics=null`。

```powershell
$runtimeVersion = '153.0.4234.48'
$runtimeSource = Join-Path ${env:ProgramFiles(x86)} "Microsoft/EdgeWebView/Application/$runtimeVersion"
node scripts/native-perf-diagnose.mjs --exe (Join-Path $sampleParent 'bin-runtime/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'cdp-runtime-default1') --chromium-log
node scripts/native-perf-diagnose.mjs --exe (Join-Path $sampleParent 'bin-runtime/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'cdp-runtime-copied01') --chromium-log --runtime-source $runtimeSource --runtime-version $runtimeVersion
```

先顺序执行默认 Runtime 对照，再执行复制153实验；同一新 EXE、95项同 SHA 公开资源及100对象源，各自 fresh root/profile/runId/端口、等长输出标签。每个输出必须不存在，不能重用 consumed 目录。系统默认 Runtime 与 Registry 保持。

本轮实际结果与收尾：

- 源版本153.0.4234.48：917文件、45目录、902,624,319字节，三个核心文件均Microsoft Authenticode Valid、AMD64、FileVersion/ProductVersion精确匹配。第一次Node→PowerShell5.1源检查在复制/GUI前因隐式Security模块加载失败；改为显式PSHOME系统模块后真实认证通过，没有绕过签名。
- 默认实际154.0.4258.37为exit0、3/3完整观察；副本153为exit1、2/3。计划5秒时SDK/Runtime标记不存在，ENOENT保留；15/30秒实际browser路径、版本、EXE SHA及probe前身份复核匹配副本，但均无owned监听。默认三次也无监听；五次完整观察均未发送HTTP/CDP、输入密码或执行UI，performanceMetrics=null。没有重试或修改采样时刻消除这次部分失败。
- Node四文件79项、Rust定向19项全通过，fmt/native all-target Clippy/语法/Prettier通过；Release exit0、Rust23m58s，beforeBuild TypeScript/Vite通过。EXE SHA `646313CDFDB11FFDCAF76EF8A0E049EF7161E794097F26089351A77251C0C6D3`；95项资源及21项冻结源码/配置保持。初次51项Node运行误传缺失observer文件，不计该观察器验收；最终存在性检查后运行四文件79项。
- 20条进程记录fresh CIM均不存在（19个不同数值，PID跨轮复用一次）。源fixture14文件、系统安装Runtime及用户3张NSIS图片SHA保持。首次临时清理在删除前被reparse保护中止；两条junction确认只指向各自owned profile内缓存后，非递归解除且验证目标仍存在，再删除六个owned临时目录及约861MiB副本。公开fixture、安装源、staged EXE/资源与输出父目录中的原始报告保留。

[完整结构化证据](rf312-windows-copied-runtime-2026-09-30.json)包含原始两轮报告/日志、源清单、五秒失败、各次检查输出、程序/资源/源码SHA、保全与清理。结论仅为本轮153副本的两次完整晚期观察未恢复TCP监听，不确认普遍Runtime回归，也不替代RF-312性能验收。日志末尾Network/GPU退出发生于末次观察后的主动owned清理，不能称启动崩溃。

只读限定策略查询：HKLM/HKCU及32/64位视图的Edge `DeveloperToolsAvailability/RemoteDebuggingAllowed` 没有发现值；WebView2 `AdditionalBrowserArguments` 四个指定键均不存在。未读取无关值或更改Registry。[微软企业策略说明](https://learn.microsoft.com/en-us/deployedge/webview2-enterprise#browser-policies-vs-webview2-policies)说明Edge浏览器策略不应用于WebView2；[WebView2策略列表](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-webview-policies)未列出上述两个浏览器策略。这只是限定候选排查，不能证明不存在其他策略或定位根因。

下一步可独立实施非默认Windows原生SDK诊断：在Tauri `with_webview` 的UI线程取得当前controller/WebView，用异步 [CallDevToolsProtocolMethod](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2?view=webview2-1.0.4022.49#calldevtoolsprotocolmethod)先只读Page.getFrameTree与Runtime.evaluate，绑定actual browser PID、唯一主frame、实际origin/timeOrigin和既有observer runId。导航、文档/frame变化、回调错误或超时即拒绝；COM遵循[UI线程约束](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/threading-model)。当前仅完成锁定源码/文档可行性核对，没有协议调用；完整UI采样还需后续状态机、真实浏览器输入和既有IPC计数验收。RF-312保持[!]，208/249不变。

## 原生SDK最小只读CDP诊断（2026-09-30）

该独立入口只用于非默认Windows native-perf构建，内部传入 `--native-perf-diagnostics sdk-cdp`，与日志、ordinary TMP、Runtime选择互斥，prepare拒绝该模式。新EXE的SDK特征检查在输出目录/准备/GUI之前执行；系统默认Runtime与既有TEMP/TMP/profile/UDF/browser flags规则保持。GUI前独占发布requested marker，最终proof单独发布；benchmark遇到任一marker即拒绝，不依赖其JSON是否有效。

```powershell
node scripts/native-perf-sdk-cdp.mjs --exe (Join-Path $sampleParent 'bin-sdk/solo_soul.exe') --fixture (Join-Path $fixtureParent 'vault100') --output (Join-Path $sampleParent 'sdk-cdp-one01')
```

只在setup成功及真实main文档Finished两个门闩都满足后，从该窗口的UI线程 `with_webview` 取得真实controller/CoreWebView2。先注册导航、Source变化、进程失败、FrameCreated守卫，读取SDK browser PID/Source；异步顺序调用一次Page.getFrameTree与一次Runtime.evaluate。表达式只读origin/href/documentURL、readyState、主frame与frame数、React容器有无内容、timeOrigin和现有observer，不读取账户文本、密码、表单值或IPC body。不通过业务IPC解锁或操作DOM。

成功proof只证明两个回调捕获区间：唯一主frame、预期 `http://tauri.localhost` origin、同一Source/browser PID、守卫无变化、observer runId/valid/timeOrigin一致。`#root`有内容不代表登录或所有前端已就绪。未知Source、JS异常原文和原始CDP响应不写入失败proof。回调有界复制PCWSTR借用数据，Source返回值用RAII释放；COM请求和回调保持UI线程，不同步等待。

原生回调按20秒上限检查迟到结果，Node从GUI spawn起只等45秒；没有回调或加载门闩不满足时由Node判为超时，不能声称原生必定自行发布timeout proof。Node读取proof前后分别复核原owned root/browser的EXE、creationMs、parentPid和UDF，SDK browser PID必须与唯一核验browser相同，不能依据proof里的任意PID查询其他进程。CDP调用不依赖TCP监听，脚本不通过TCP/HTTP连接CDP端点。端口占用预检仍建立loopback socket，正常产品初始化的联网行为保持。

SDK协议请求单列 `sdkProtocolCalls`，不计入Tauri invoke。单次observer snapshot仅描述当时状态，不能称全程IPC总量；elapsed仅诊断边界计时，performanceMetrics保持null。本节描述实施与验收契约；真实回调、fixture保全、进程清理和检查结果将在实际运行后追加。

截至本轮收尾，SDK草稿已实现并通过97项Node回归及Rust源码格式检查；9项新增Rust回归尚未执行。独立预审修正COM getter重入后守卫复查、async lstat完成后的45秒预算重查，并将网络说明限定为CDP端点。自定义有界completion的宏需要直接 `windows-core` crate路径，锁文件已有0.61.2；其Windows optional/native-perf-only声明被自动审批拒绝，Cargo两文件尚未修改，精确补丁已交人类待授权。[本阶段完整预检证据与未应用提案](rf312-windows-sdk-cdp-preflight-2026-09-30.json)保留源码SHA、原始检查输出和边界。原生编译、Rust测试/Clippy、Release及GUI均未执行；没有真实SDK调用，不产出性能数据。

## 授权后原生SDK实测结果（2026-09-30）

用户明确授权最小补丁后，Windows target新增精确版本optional `windows-core =0.61.2`，仅native-perf启用；Cargo.lock只关联已存在的包，无升级。上节及[授权前预检](rf312-windows-sdk-cdp-preflight-2026-09-30.json)保留当时状态。本轮首次E0603改为webview2_com公开导出，首轮Clippy长度比较改为等价的newline预留条件；两次失败保留。最终Rust定向lib28项（全部9新增）通过，Clippy all-target/fmt通过；同SHA Node五文件97项通过，非全库测试。非默认Release锁文件构建exit0，Rust23m12s、TypeScript/Vite通过、Vite7.16s，未打包。新EXE SHA `201BE7FDBF2730EE6BC22D1EBD6D04BA865B31B3C76DE1471508B1603696B097`，28文件freeze与95资源SHA保持。freeze中的测试仍运行字段为填录错误；原记录与时间证据保留，最终测试/fmt实际先于freeze结束。

一次新隔离100对象实测使用默认154.0.4258.37，exit1；两个SDK方法均取得成功HRESULT且可解析的有界JSON回调，Page.getFrameTree校验主frame、loader与Source。Runtime.evaluate在聚合document条件被拒，`stage=evaluation/reason=document-mismatch`、native elapsed8ms，proof在spawn后1704ms被观察；这些时间是诊断边界，不能当启动/就绪/性能时延。失败proof的document/observer/timeOrigin为空，原始被拒值未保存，所以具体不符项和根因未知；Finished门闩不保证异步初始化/React已渲染只是源码事实，不能认定这就是本次原因。observer/timeOrigin和读取后身份校验未完成，runner `sdkProtocolCalls=null`、`performanceMetrics=null`；未重跑或放宽要求。

原runner `cleanupIntegrity=false/unverifiedDescendants=true` 保留。额外fresh CIM确认9个记录PID和本轮owned命令行都不存在，随后仅清理三个精确owned目录；原始四个native标记/proof已存档。首次删除前普通/扩展路径比较误拒保留，确认同一本地绝对路径身份后非递归解除目录内缓存junction，再清理临时目录。源14文件与用户3张NSIS图片SHA保持，EXE/95资源与原始输出/日志保留。[完整本轮证据](rf312-windows-sdk-cdp-2026-09-30.json)包含授权、所有失败/最终检查、source/resource/EXE SHA、原始proof与独立收尾。下一步先为文档聚合拒绝增加固定枚举子原因，不保存拒绝值，再验证新构建；本次失败不可覆盖或改成成功。RF-312仍[!]、208/249不变，尚缺成功文档绑定、完整UI行程及多端性能验收。

## 固定文档拒绝码诊断（2026-10-03）

上一轮 `document-mismatch` 未指出哪个条件不符。本轮仅在非默认 native-perf 的 Runtime.evaluate 校验中拆分固定拒绝码；原有成功条件、检查顺序、load/setup 门闩、一次协议调用限制和采样时机不变。不保存被拒绝的 URL、DOM、时间或 observer 原值，也不把失败转换成成功。

| 固定 reason | 首个不符条件 |
| --- | --- |
| document-result-type | SDK 返回值类型不是 object |
| document-payload-type | 值不是 JSON object |
| document-origin-mismatch | origin 与预期应用 origin 不同 |
| document-href-mismatch | href 与已绑定 Source 不同 |
| document-url-mismatch | document URL 与已绑定 Source 不同 |
| document-ready-state | 文档状态不是 complete |
| document-not-main-frame | 不是主窗口文档 |
| document-child-frames | 存在子 frame 或 frame 数缺失/类型不符 |
| document-ui-root-absent | 原有 uiRootPresent 条件未满足；不能单凭此码区分容器不存在和容器无子节点 |
| document-clock-invalid | 采样时钟缺失或不是有限非负数 |

成功 proof 的字段和 observer/timeOrigin 要求保持。Node runner 仍拒绝任何 native success=false 的 proof；SDK诊断继续 performanceMetrics=null。新增回归验证每种实际拒绝、多个不符项的固定优先级以及私有哨兵值不出现在 reason；原有回归不删改。实际结果如下，不从固定码推断应用根因。


同一源冻结的非默认 Release 构建 exit0（约19m46s，Rust18m57s，TypeScript/Vite通过），新 EXE SHA `8a2a6c42850c07afa64317e4a8ca7db9e6663bb405f8d179d0225fbb2d2ac2af`；94份公开资源和相邻运行库逐份复制/复核，不新增依赖。原生29项、Node五文件97项、严格native all-target Clippy和fmt均通过；新增1项覆盖10种拒绝条件、固定优先级和私有哨兵值不外泄。默认完整R沿用本轮RF-1071的1858通过/0失败/3既有ignored，后续改动仅在非默认原生诊断内，不重复称新运行。

一次 fresh 隔离100对象 SDK诊断 exit1，实际 WebView2 **154.0.4258.48**；两个只读协议方法回调完成，首个拒绝为 `stage=evaluation/reason=document-ui-root-absent`。此前的返回类型、origin/href/document URL、complete、主文档和无子frame条件均通过。原生边界elapsed6ms、spawn后2122ms观察到proof，均不能当作启动或就绪性能。失败proof仍不保存拒绝值，document/observer/timeOrigin为空，Node未接纳完整protocol次数和读取后成功身份验证，performanceMetrics=null。

本次已将聚合原因缩小到原有 `Boolean(document.getElementById('root')?.hasChildNodes())` 条件。尚未区分根容器缺失/无子节点，也未证明初始化失败或异步React挂载时序；不得以此认定产品空白根因，更不能放宽规则、延迟重试后冒充成功样本。下一步单独增加有界只读根容器/挂载状态诊断，再决定如何建立真实UI就绪边界。

原runner清理integrity完整、unverifiedDescendants=false；收尾fresh CIM确认9个记录PID与owned应用/WebView进程均不存在。首轮过宽命令行扫描匹配到两个执行检查的PowerShell宿主，已保留并收窄核验范围，没有结束任何额外进程。公开100对象源7文件、12份源码/依赖冻结、EXE及94资源SHA保持。原始失败proof、现场身份、全部检查日志与清理范围见[本轮结构化证据](rf312-windows-document-reasons-2026-10-03.json)。RF-312保持[!]，仍缺成功文档绑定、真实UI行程及多端性能验收；本轮诊断改进独立本地提交，不推送。


## 根容器与 React 挂载调用点诊断（2026-10-03）

本轮仍在同一个有界、一次性的 Runtime.evaluate 中仅读取结构状态，根条件不满足后增加 `uiRootDiagnostic`。诊断只保存根容器存在/有子节点、`solosoul:react-mount` 标记是否出现、启动层存在，以及白名单 `loading/error/ready/unavailable`；不读取 DOM 文本、input.value、localStorage、账户数据或原始错误详情。仅在已有文档身份条件通过、原条件返回 `document-ui-root-absent` 后由 Rust 严格校验类型、键数和逻辑一致性；不符时不保存这份辅助诊断，原失败仍保留。成功 proof 和其他拒绝的字段形状保持。

| classification | 本次可观察的结构状态 |
| --- | --- |
| root-container-absent | 根容器不存在 |
| root-empty-before-react-mount | 根容器存在但无子节点，挂载调用点标记尚未出现；不能区分模块加载、等待初始化或尚未被启动层记录的失败 |
| react-mount-marked-root-empty | 已出现挂载调用点标记但根仍无子节点；不证明 createRoot/render 调用成功或 React commit 已完成 |
| startup-error-before-react-mount | 挂载调用点标记未出现，启动层已记录 error；不区分 timeout/initialization-failed/backend-unavailable |

`performance.mark('solosoul:react-mount')` 位于生产 createRoot/render 调用之前；此观测不是 UI 就绪或性能指标。启动层 phase/reason 位于闭包，仅渲染到诊断文本，故本轮不读取它们。若后续需要更细原因，必须另设固定的只读契约，不抓取文本来反推敏感信息。Node runner 继续拒绝失败 proof，不放宽校验、增加任意延迟或重试；本轮实际结果如下。


最终原生31通过/0失败/0ignored、六文件Node101通过/0失败/0skip；将4项新Node回归加入固定runner后，完整runner124项中123通过、0失败、1既有符号链接权限skip。首次runner格式检查exit1（CRLF），定向Prettier后exit0，实际diff只新增一条测试路径；原始记录保留。严格native all-target Clippy与fmt通过。实际非默认Release exit0，Rust14m38s、总约15m27s，TypeScript/Vite通过；EXE SHA `84b6e069b3e2402bda32072def4a439af957dc57ab174ee1d68b62dd4c906340`，15份源码/依赖冻结与94公开资源SHA保持。runner只改测试文件清单，发生于构建后，另存SHA；它不是Release输入，不声称其参加了该构建。默认R沿用未受非默认诊断变化影响的本轮RF-1071结果1858通过/0失败/3原有ignored。

一次 fresh 100对象诊断exit1，WebView2 154.0.4258.48；SDK回调取得，仍是 `document-ui-root-absent`，新增分类 **root-empty-before-react-mount**：rootExists=true/rootHasChildren=false/reactMountMarked=false/startupScreenPresent=true/startupState=loading。采样时根容器存在，挂载调用点标记未出现，启动层未记录错误；不能扩张为以后正常挂载或“产品初始化失败”的结论。原生elapsed7ms/spawn后2329ms观察proof只作边界计时，document/observer/timeOrigin为空，performanceMetrics=null，完整SDK次数和成功读取后身份验证仍未接纳。

下一步需要明确在真实React内容提交后产生的有界UI就绪诊断边界，保留本次早期观测，不以任意等待/重试冒充性能样本。未改变生产启动代码或全路由加载策略。原runner清理完整，fresh CIM确认9个记录PID和owned应用/WebView进程不存在；原始proof存档后，核对manifest、绝对父路径与reparse边界，仅清理3个本轮owned目录。公开源7文件、旧失败、依赖与stash保持。[本轮完整证据](rf312-windows-ui-root-2026-10-03.json)保存全部检查、格式首败、原生失败和清理。RF-312仍[!]，本项独立提交后转RF-121 Windows辅助功能补证。


## 真实 UI 交接后的文档绑定（2026-10-04）

React 的挂载调用点不能代替提交后的内容。复用 `useSessionLifecycle` 已有的账户状态确认与双 `requestAnimationFrame` 交接：`solosoul:startup-dismissed` 后发出固定 `solosoul:startup-handoff` 事件，不改变原交接时长。启动层增加只读 `diagnostic()`，仅返回 schemaVersion 与白名单 phase/state/reason；新增 i18n/platform/capabilities 阶段，不读取 DOM 文本、input、偏好、账户或原始错误。

首版在旧文档绑定后，通过一次 `Runtime.evaluate` 的 Promise 等待真实交接（[`awaitPromise`](https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-evaluate)）；原有身份守卫严格拒绝了启动期间的 `source-changed`，exit1。失败结果不含迟到的 UI 状态。结合 BrowserRouter 认证重定向代码与修正轮最终 `/login` 绑定，这支持初始化导航与采样时机竞争的判断；首轮没有保存被拒绝的目的 URL，不能单凭它确定导航目的。

修正后仅在显式 SDK 诊断模式安装一次固定控制脚本：注册交接/错误监听后立即检查，防止交接早于安装；8秒渲染器定时器只报告超时，不宣告成功。固定内部事件携带4个受限字段，不读业务 IPC body；Native 的一次性监听、原20秒总预算和原子门闩避免迟到通知/超时并发开启重复采样。真实交接后才获取主 WebView 并安装原身份守卫，仍恰好调用 `Page.getFrameTree`、`Runtime.evaluate` 各一次。控制通知只决定开始时机；成功仍要求原 source/frame/loader/browser/timeOrigin/observer 条件，以及新 UI 交接条件全部成立。等待开始后再次导航仍拒绝。此控制流不进入默认构建或普通 benchmark；SDK requested/proof 标记仍令 benchmark 拒绝该轮。

`uiReadyDiagnostic` 严格限定9个键，嵌套阶段各3键；Rust 与 Node 均拒绝额外键、未知私有值、缺失标记和提前就绪。一次 evaluate 保留其初始/最终固定结构。最终采样开始于交接之后，所以两端观察均已就绪；先前空根与本轮 source-changed 原始失败单独保留，不能把这些观察拼成一次性能样本。

```powershell
node scripts/native-perf-sdk-cdp.mjs --exe <new-owned-bin>/solo_soul.exe --fixture <public-fixture>/vault100 --output <new-owned-output>
node scripts/run-node-tests.mjs
cargo test --locked -p solo_soul --features native-perf --lib native_perf:: -- --test-threads=1
```

最终 Node **134 passed / 0 failed / 1既有符号链接权限skip**；原生 **36 passed / 0 failed / 0 ignored**，严格 all-target Clippy、1.99 fmt及定向格式通过。本轮前端 **256文件/2241 Vitest**、TypeScript/ESLint通过；门闩续改未修改这些前端源码。完整默认R **25组/1858 passed/0 failed/3原有ignored**、严格workspace/all-target Clippy通过，PDFium路径仅提供给测试子进程；续改仅在非默认模块内，默认R沿用同轮结果。生产WEB首轮缺Playwright headless shell导致1 passed/25启动失败；按已有配置复用Chrome后26 passed，包括生产交接只发一次的断言。Node24、Rust1.96测试/Clippy/Release与1.99格式版本分别记录，不称未执行的远端CI。

两版非默认Release均exit0；首版总23m38s，修正版总15m11s、Rust14m18s。最终EXE SHA `5d79a67c8a4f502faa9fcfc4ab23d1a60036936beeafdd6a9d27080beb836e5c`；22份关键源码/配置/依赖冻结、94公开资源和运行库逐份核验。每版只进行一次新的合成100对象诊断：首版source-changed保留；修正版runner **exit0**，WebView2 **154.0.4258.53**，当前`/login`主文档、零子frame与原frame/loader/browser/timeOrigin身份检查全部接纳，outcome=handoff、根内容和交接标记均为true，启动阶段accounts/ready/none。

原生elapsed344ms包含交接门闩；spawn后2971ms观察到proof与runner总35.74s均不能当作启动性能。`performanceMetrics=null`，没有GUI性能样本、中位数或尾部指标。两轮公开源均保持、runner清理完整，独立fresh CIM各确认9个记录PID及owned应用/WebView均不存在。RF-312仍[!]：Windows真实UI行程、重复100/5000对象采样、OCR/预览/锁定睡眠恢复及多平台数据待完成；RF-121至127的材质前置不解除。源码、实际失败、全部检查和收尾记录见[本轮结构化证据](rf312-windows-ui-handoff-2026-10-04.json)。

归档目录以独立 `.gitattributes` 保留字节与SHA；CRLF原件另存 `.json.gz`，配套LF JSON视图。首次暂存检查对原CRLF报尾随空白的失败与修正均保留，严格检查规则保持。清理首轮在INetCache junction处停止、未删除数据；确认两个链接仅指向各自owned根内的IE缓存后单独解除链接，重新检查无reparse并清理6个本轮数据/cache根和被替代候选。最新候选保留供后续诊断，公开源、旧轮证据及stash保持。清理首败与修正过程均归档。
