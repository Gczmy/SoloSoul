# RF-312 Windows 原生 PDF 能力诊断

当前范围是固定公开 PDF 的真实 WebView2 能力诊断；PDF 性能指标保持为空，RF-312 总体验收未关闭。

## 入口与范围

以 `native-perf` 非默认 Windows feature 构建 Release。独立入口 `node scripts/native-perf-pdf-diagnostic.mjs --exe ABS_EXE --fixture ABS_MEDIA_SOURCE --output NEW_ABS_OUTPUT --samples 3` 创建并核验新 owned 副本；不复用已消费的证明。源数据只读，GUI、WebView2、插件、模型、日志和 Known Folder 保持已有隔离规则。默认 GUI、前端、旧 SDK 行程、启动模式和依赖不修改。

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
