# RF-312 Windows 原生 PDF 能力诊断

当前范围是固定公开 PDF 的真实 WebView2 能力诊断；PDF 性能指标保持为空，RF-312 总体验收未关闭。

## 入口与范围

以 `native-perf` 非默认 Windows feature 构建 Release。当前驱动要求 `windows-native-sdk-pdf-structure-diagnostic-requested` 二进制标记与内部 `pdfDiagnostic.schemaVersion=4`；旧 v1/v2/v3 证明继续支持离线校验，重跑旧诊断分别须使用 `1295be05` / `a093e732` / `749f1b25` 的匹配驱动与源码。独立入口 `node scripts/native-perf-pdf-diagnostic.mjs --exe ABS_EXE --fixture ABS_MEDIA_SOURCE --output NEW_ABS_OUTPUT --samples 3` 创建并核验新 owned 副本；不复用已消费的证明。源数据只读，GUI、WebView2、插件、模型、日志和 Known Folder 保持已有隔离规则。默认 GUI、前端、旧 SDK 行程、启动模式和依赖不修改。

使用真实 SDK 输入完成主密码解锁、对象工作区和附件列表，然后点击固定 `text_only.pdf`。主文档、进程、frame、loader、时钟和输入可信性继续核验。只有 PDF 操作期间允许有界子 frame 创建，原行程仍拒绝子 frame。保存四次 frame 快照、实际上下文候选和 viewport PNG，再真实关闭预览并复核原主文档，移除事件订阅。

候选状态按快照绑定同一实际上下文；后续快照重新检查，可观察到较晚加载完成。每快照最多四个候选、总计最多十六个，禁止同快照重复上下文。API 或 embed 存在、主文档两帧和截图本身均不自动提升为 PDF 性能验收。报告 `diagnosticOnly=true`、`renderVerified=false`、`performanceMetrics=null`，汇总为空；四个前缀阶段不伪装成完整行程指标。

## 首次实测与修正

首个 Release 的三次实际诊断均完成四个前缀阶段后因 `pdf-media-source-invalid` 失败；源数据及附件密文保持不变。独立文件摘要复核确认，副本中变化的是 GUI 正常更新的 `accounts.json` 与 `ui_preferences.json`。在 GUI 写入后再次校验启动前的封闭摘要造成误拒绝。

修正是在 consume 阶段通过完整 owned manifest/ready/文件摘要核验后绑定固定 PDF 路径，后续使用这份已核验绑定，保持启动前完整文件门禁。新增正常 GUI 写入后绑定可用及路径/素材身份越界拒绝回归。首次失败、旧构建及一次 Clippy 失败均保留，未覆盖为成功。

## 定向检查

最终 70 项原生回归通过；严格 Clippy、Rust/JS 格式与 diff 检查通过。固定 Node 入口 181 通过、0 失败、1 项既有 Windows 文件 symlink 权限跳过。测试覆盖固定模式互斥、旧模式门禁、实际绑定与候选状态、快照上下文一致性和只读 viewer 探针；不代表真实 PDF 渲染验收。

## 实际新诊断

Windows 11 Enterprise LTSC 26100、i7-9700（8 核）、约 16 GiB RAM、WebView2 154.0.4258.53。新 Release EXE SHA `0385cc3bc82db0264cba7f77bdf92c15f988241228f4b56ea68027e7c075180a`，构建 exit0，444 项输入和 94 份资源保持。

100 对象公开媒体源的三次诊断均通过；每次 59 次 SDK 调用、四个前缀阶段、四次主文档快照，真实打开及关闭 PDF，主文档身份和事件订阅清理通过。三份 PNG 各 34,024 字节，SHA 均为 `5ffffdfc3c9f4a9db05a3170f5df71d07da13ce35a6528ffb011a435ff54b4c9`。人工查看第一份，可见固定公开 PDF 第一页正文与“1 of 2”指示；另外两份字节相同。这确认本组截图时内容可见，不是加载就绪时间的自动验收。

三次实际 `Page.getFrameTree` 均只有应用主文档，四份快照的子 frame 列表为空；`Runtime.executionContextCreated` 各只有一个 application 上下文，没有可绑定的 PDF 候选。不能据此判断 PDF 不支持渲染，也不能声称已验证 viewer 内部 API。下一步在同一 owned WebView 中调查有界 related target/session，再定义可靠的 PDF 就绪门禁。官方 [ICoreWebView2_11](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_11?view=webview2-1.0.4022.49) 提供特定 session 协议调用，锁定的 `webview2-com-sys 0.38.2` 已含绑定；这只证明接口条件，不代替实际子 session 验收。

原 runner 六份样本均清理完整。另行 fresh CIM 核验首次组 26 个进程身份，新组 27 个身份及三个额外清理 PID 均已退出；仅清理 48 个精确 owned 私有缓存目录、解除六个私有缓存链接且不遍历目标。Vault、markers/proofs、截图、全部 EXE、资源和公开源保留，没有结束额外进程。

[结构化证据](rf312-windows-pdf-diagnostic-2026-10-05.json)和[逐字节原件索引](rf312-windows-pdf-diagnostic-2026-10-05/index.json)保留两份构建、首次三份失败、最终三份诊断、全部检查及清理。诊断成功不关闭 RF-312；本轮不报告 PDF 中位数、尾部延迟或完整 IPC/内存指标。

## 后续

先按实际 frame、候选状态及截图确认可用的渲染观察机制，再开展 PDF 重复性能测量。OCR 真实文件选择与首次推理、KDF/存储/React/SDK 归因、原偶发首页超时、睡眠及多端场景继续由 RF-312 承接。RF-112/121 与 RF-122～127 的前置保持。

## 2026-10-05 · 相关目标与 session 实测

新增 v2 诊断通过同一 owned controller 的 `Target.getTargetInfo`、`Target.getTargets` 和 `Target.setAutoAttach` 记录有界相关目标，使用 `ICoreWebView2_11::CallDevToolsProtocolMethodForSession` 读取固定 viewer 状态。只接受主 session 发出的直接相关附加事件；最多四个 session、每快照最多八个目标和两个候选、总调用预算仍为 128。陌生目标只保存分类，URL、标题和正文不写入证明。原启动、主文档、frame/loader、时钟、输入可信性和默认行程门禁保留；检查前后目标及文档一致，并清理两族订阅和自动附加。

75 项原生回归、严格 Clippy、Rust/JS 格式和 diff 检查通过。Node 185 项中 184 通过、0 失败、1 项既有 Windows symlink 权限跳过；当前合同另实际接受旧三份 v1 原生证明。新 Release 构建 exit0，用时 1,157.34 秒；EXE SHA `f9b6b14a0e838a8ff15a1f5cc6e68b4b1243ee7f49bbec9122c46bbdd38f8ac0`，445 项输入与 94 份资源冻结并保持。

100 对象三个新 owned 副本的实际诊断均通过，组壁钟 150.04 秒，每次 87 次 SDK 调用。其中一个直接相关 PDF iframe 的 session 调用 17 次，四份候选均确认文档一致，但 `viewerPresent=false`、`loadSucceededMethodPresent=false`、加载状态与页数为空、绘制帧为零。主 frame 树仍无子 frame，主上下文一份；不能将 iframe 出现或协议调用成功当作 PDF 加载完成。

每次目标数量为 3/4/4/4。目标列表实际还出现一个 `component-extension` 的 `webview`：`parentId` 指向应用主目标、`browserContextId` 与主目标一致，但 `attached=false`；其下另有 owned PDF iframe。自动附加未把该组件 webview 纳入可读 session，这是下一步调查位置；本轮没有读到它的 viewer 状态，也未证明其内部 API 可用。

三份截图各 34,024 字节，与旧 v1 的 SHA `5ffffdfc3c9f4a9db05a3170f5df71d07da13ce35a6528ffb011a435ff54b4c9` 相同。实际查看第一份，可见公开 PDF 第一页正文、1 of 2 指示及右下角备份提醒，另两份字节相同。截图仍不证明自动就绪或延迟。

fresh CIM 确认 30 个记录身份已退出；本轮没有额外未知创建时间的清理 PID。核对 absolute owned 路径及 reparse 边界后，清理 24 个缓存目录、解除三个链接；全部 Vault、证明、截图、EXE、94 份资源、171 份公开媒体源文件和 12 份副本附件密文保持。原 stash、子模块与旧 PDF 构建保持，不推送。

[结构化验收](rf312-windows-pdf-targets-2026-10-05.json)和[逐字节原始索引](rf312-windows-pdf-targets-2026-10-05/index.json)记录完整检查、冻结、构建、三份原生证明、截图与清理。`diagnosticOnly=true`、`renderVerified=false`、`performanceMetrics=null` 和空汇总保持；本阶段仅验收相关目标/session 诊断，RF-312 继续未完成。下一步验证上述主目标关联及 browser context 后受控附加组件 webview，读取固定 viewer 状态，再定义 PDF 就绪门禁；OCR、性能归因、原偶发首页超时、睡眠及多端缺口保留。

## 2026-10-05 · 组件 webview 受控附加实测

v3 只对实际观察到的唯一 `component-extension` webview 附加：主目标必须为当前应用 page，组件 `parentId` 和非空 `browserContextId` 必须分别与主目标一致，且已有活跃的 directly-related owned PDF iframe。附加前重新读取同一目标，绑定实际 SDK 返回的 sessionId 与主 session 的附加事件；固定 viewer 探针前后复核目标、frame 和 loader。真实关闭预览前显式解除组件 session，再取消自动附加和两族订阅。未创建或关闭协议目标，没有子 session 输入；默认前端与依赖不修改。

80 项原生回归、严格 Clippy、Rust/JS 格式和 diff 检查通过。Node 188 项中 187 通过、0 失败、1 项既有 Windows symlink 权限跳过；另以当前 v3 合同实际接受旧 v1/v2 的六份原生证明。Release exit0，994.79 秒，EXE SHA `61333412d672dc61dcb2ea98c0cb492c66fab67b3d3f135092a7c2337f13b6b7`；446 项源码输入、94 份资源冻结并保持。

100 对象源的三个 fresh owned 副本诊断均通过，组壁钟 194.30 秒。各 99/107/107 次 SDK 调用、26/34/34 次 session 调用，组件候选为 3/4/4 份，共 11 份。首样本第零次目标快照只有主目标，后续才出现组件；另两份首快照即已观察到，未丢弃这一差异。三次均用 SDK 成功回复显式解除组件，清理状态明确为 `explicit-reply`，不将它记作收到解除事件。

11 份组件候选 `documentMatches=true`，但 `viewerPresent=false`、`loadSucceededMethodPresent=false`、页数与加载状态为空、绘制帧为零；同组 owned PDF iframe 也未提供该 viewer API。此结果确认目标关联、协议可达与固定探针的观察，不提供自动就绪或性能验收。微软的 [WebView2 PDF 阅读器策略文档](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-webview-policies#newpdfreaderwebview2list)描述可配置的 PDF 阅读器版本，但本轮未核验本机策略或引擎，不能从 API 缺失推断具体版本。下一步调查当前组件的有界 DOM 结构、同一 PDF 子 frame 和实际可用的就绪信号。

Native 单调时钟在真实打开输入前保存起点，每份候选保存观察时间，所有观察均位于本次打开到诊断结束之间。记录保留实际时刻；探针未观察到加载成功，因此不计算 PDF 延迟、中位数或尾部指标。`diagnosticOnly=true`、`renderVerified=false`、`performanceMetrics=null` 和空汇总保持。

三份 PNG 各 34,024 字节，SHA 均为 `5ffffdfc3c9f4a9db05a3170f5df71d07da13ce35a6528ffb011a435ff54b4c9`，与 v1/v2 实测相同。实际查看第一份，可见公开 PDF 第一页正文、1 of 2 指示与备份提醒，另两份字节相同；这是截图时可见性的人工证据。fresh CIM 核验 29 个记录进程身份退出，仅清理 24 个精确 owned 缓存目录和三个缓存链接，不结束额外进程。全部 Vault、证明、EXE、资源、171 份公开源文件、12 份副本附件密文及旧 PDF 构建保持。

[结构化验收](rf312-windows-pdf-component-2026-10-05.json)和[逐字节原件索引](rf312-windows-pdf-component-2026-10-05/index.json)保存检查、冻结、Release、三份原生样本、时钟与清理。本阶段仅验收组件 session 诊断；RF-312 继续未完成。OCR 真实文件选择与首次推理、性能归因、偶发首页超时、系统睡眠及多端缺口保留。

## 2026-10-05 · 实际结构与子 frame 实测

v4 沿用组件附加和所有 owner/PID/source/主文档/frame/loader/输入门禁，只扩展固定只读探针与严格结构合同。每候选最多 512 元素、8 个开放 shadow root、32 个自定义标签及各 8 份 embed/canvas 几何；预算外明确 truncated。只保存数量、标签名、来源分类和几何，不返回正文、id/class 属性、HTML 或原始 URL。两份 session frame 树各最多 8 节点、深度 3，记录前后 root/child 身份；未知子来源只分类。closed shadow root 和原生 plugin 内部状态不在该遍历范围。

首次 Rust 格式检查发现新增字段名用了单引号，已修正；复核同步恢复被误调整的原 DOM v1 合同，将 v2 仅用于候选状态。原 DOM 校验函数与 HEAD 字节相等。UTF-8 源码读回错误、首次格式失败与最终检查均保留。最终 84 项原生回归、严格 Clippy、格式和 diff 通过；固定 Node 192 项中 191 通过、0 失败、1 项既有权限跳过。九份旧 v1/v2/v3 原生证明由当前 v4 合同实际接受。源码输入 448 项、资源 94 份冻结，Release exit0、1,036.66 秒；EXE SHA `1035877c42f6f65437159519de5d360255dbe942b79094532ee2462a22f73d8f`。

三个 fresh owned 100 对象样本诊断通过，组壁钟 154.24 秒，各 103/99/107 次 SDK 调用、30/26/34 次 session 调用。组件候选 3/3/4 份，共 10 份；第三样本 snapshot0 仍为 loading、15 元素、embed0，后续 complete。最初复核均一化假设的断言失败，实际变化保留，未修改校验器。其余 9 份 document complete 且 embed 可见，均 980×604、来源分类 component-extension；没有 pdf-viewer、canvas、页数或加载成功 API。后续 159 元素和唯一开放 shadow root 对应 `edge-toast-notification`，不能将该通知结构当作 PDF 渲染状态。

10 份候选的 frame 树在各自探针前后相同；第二样本首次组件观察含一个分类 other 的子 frame，随后不再出现，其余只有 session 根。没有对该未知来源做进一步脚本读取。本组结构完整、DOM complete 或 embed 可见均不能证明 PDF 解析和正文绘制完成。实际关闭、显式解除、主文档复核及自动附加/两族订阅清理通过。

三份截图各 34,024 字节，与前九份 v1/v2/v3 截图相同，SHA `5ffffdfc3c9f4a9db05a3170f5df71d07da13ce35a6528ffb011a435ff54b4c9`。实际查看第一份，可见公开第一页正文、1 of 2 与备份提醒。离线分析固定 1028×749 参考的正文区域 x180/y225/w430/h36：530 个深色像素，行序二值掩码 SHA `6f307e964c1dba0a23593241d6d193823cdf8feebb3946ca80b5af9a4a165044`；空白、错位一像素与部分正文均不匹配。此结果仅支持候选信号可行性，尚未实际部署为就绪门禁或测量延迟。

下一阶段用现有 image 依赖验证有界的固定公开首屏像素观察，保持源数据、窗口/进程/文档/loader/视口尺寸一致，记录截图调用开销并重复原生采样。接受范围只覆盖此公开素材的首屏可见性，不能扩展为任意 PDF 全页加载完成。若视口或字体不同须如实失败，不自动用运行结果替换参考。

fresh CIM 核验 30 个记录身份退出，仅清理 24 个精确 owned 缓存及三个链接；12 份附件密文、全部 Vault/证明/截图/旧 EXE、公开源与冻结输入保持。[结构化结果](rf312-windows-pdf-structure-2026-10-05.json)和[逐字节原件索引](rf312-windows-pdf-structure-2026-10-05/index.json)保存全过程。`diagnosticOnly=true`、`renderVerified=false`、`performanceMetrics=null` 和空汇总保持；RF-312 未关闭，OCR、归因、首页偶发超时、睡眠和多端缺口保留。
