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
