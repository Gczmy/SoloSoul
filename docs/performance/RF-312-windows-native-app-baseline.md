# RF-312 Windows 原生应用采样（2026-09-30）

本记录定义并执行真实 Tauri Release 与 WebView2 界面采样，只使用 [RF-312 合成 Vault](RF-312-native-vault-baseline.md)。`native-perf` 是非默认 Windows feature；默认构建的入口、数据目录和路由加载保持原行为。当前尚无成功GUI样本，CDP连接仍被拒绝；Windows阶段不足以关闭 RF-312：OCR、附件预览、系统睡眠恢复及 macOS/Android 尚未取得同口径数据。

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
