# RF-312 Windows 原生 PDF 能力诊断

当前范围是固定公开 PDF 的真实 WebView2 能力诊断；PDF 性能指标保持为空，RF-312 总体验收未关闭。

## 入口与范围

以 `native-perf` 非默认 Windows feature 构建 Release。当前驱动要求新增的 `windows-native-sdk-pdf-target-diagnostic-requested` 二进制标记与内部 `pdfDiagnostic.schemaVersion=2`；旧 v1 证明继续支持离线校验，重跑旧诊断须使用 `1295be05` 的匹配驱动与源码。独立入口 `node scripts/native-perf-pdf-diagnostic.mjs --exe ABS_EXE --fixture ABS_MEDIA_SOURCE --output NEW_ABS_OUTPUT --samples 3` 创建并核验新 owned 副本；不复用已消费的证明。源数据只读，GUI、WebView2、插件、模型、日志和 Known Folder 保持已有隔离规则。默认 GUI、前端、旧 SDK 行程、启动模式和依赖不修改。

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
