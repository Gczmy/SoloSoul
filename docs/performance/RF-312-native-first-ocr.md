# RF-312 Windows 首次 OCR 原生采样

本入口使用固定公开 `ocr_test.png`、真实应用 OCR 页面、原生文件选择器和现有 PP-OCRv6 small 模型，测量每个新进程的首次识别结果就绪。仅适用于 Windows 非默认 `native-perf` 构建；不修改默认登录、OCR、主题或 IPC 行为。本阶段的数据集为 100 和 5000 对象，各请求三个完整样本，RF-312 整体仍待后续归因与多端验收。

## 输入和就绪边界

公开图片为 500×200、6274 字节，SHA-256 `56a9d54d9b70a3ee1b22e0b08b755b6cfc15ee125df4b11d52f8523583dcacaa`。进入 owned 副本后以 create_new 写入专属 `profile/Documents/ocr_test.png`，前后比较字节与摘要。OCR 输入是这份公开明文 PNG；测试 Vault 中的附件密文另行核验，不混用两者摘要。

本口径不清空操作系统文件缓存，不能称为系统冷启动。每次新进程采用默认 small 档；专属根目录不存在 `ocr_preferences.json`，status 查询不初始化识别 session。正常应用流程为登录、工作区核验、返回首页、进入 OCR、选择公开图片、完成首次 `ocr_scan_image`。测试不会直接调用扫描 IPC 或伪造结果。

真实文件选择器必须是同一进程、由 main HWND 直接拥有、前台顶层为该对话框的 `#32770`。限定最多四个候选对话框和 128 个子控件，保留类名、ID、父链和状态，不读取其他窗口正文。只接受唯一 ComboBoxEx32/1148 下的非密码 Edit 和直接 Button/1；每次消息前重新检查目标及父链。使用 `WM_SETTEXT+readback+BM_CLICK` 选择固定公开路径，这段操作不是硬件键盘输入。应用内操作使用 SDK-CDP Input。

仅接受完整三行的规范化结果 `helloppocrv6solosoulocr1234567890`，可忽略大小写、标点与空格，不能只凭数字或任意一行通过。扫描调用必须恰好一次；结果探针要求样式/几何可见、结果中心在视口且 elementFromPoint 命中结果，连续两个 requestAnimationFrame 后仍满足。最多三次真实 SDK 滚动，使结果进入视口；沿用既有有界等待，不追加扫描或登录尝试。

该口径是应用 DOM 和视口命中的就绪观测，不是结果截图的逐像素验收，也不证明 DWM 合成或任意图片的准确率。

## 时间定义

所有原生边界使用 Instant。四项统计定义如下，单位 ms：

| 指标                       | 起止边界                                              |
| -------------------------- | ----------------------------------------------------- |
| `pickerOpenedObservedMs`   | 首个选择输入前 → 观察到 owned 原生选择器              |
| `pickerSelectionClosedMs`  | 观察到选择器 → 观察到选择器关闭，含固定路径写入及读回 |
| `firstOcrResultObservedMs` | 向选择器提交 Open → 完整结果的两帧/视口探针通过       |
| `totalOcrObservedMs`       | 首个选择输入前 → 完整结果探针通过                     |

时间包含应用排队、模型加载、推理、React 渲染、SDK 和观测开销；不以减法估算净引擎耗时。整组身份、公开输入、完整结果、源数据和清理都通过才输出中位/P95；任何失败保留原件，并将四项整组数值置空。n=3 的 P95 为这三次最大值，不能用来推断所有设备的尾部性能。

## 复跑

先按 [公开媒体合同](RF-312-media-fixture-contract.md) 准备两档源数据，在固定生产前端资源和模型上构建 `cargo build --locked --release -p solo_soul --features native-perf --bin solo_soul`，冻结源码、资源与 EXE 摘要。输出必须为新的绝对目录。

```powershell
cd C:\Users\40299571\SoloSoul\tauri
node scripts/native-perf-ocr.mjs --exe C:\public-perf\bin\solo_soul.exe --fixture C:\public-perf\vault100 --output C:\public-perf\new-first-ocr-samples --samples 3
```

测量结束后核验 fresh CIM 的 PID/birth/EXE，只清理本轮明确归属缓存与已核验链接。保留 Vault、公开输入、证明、日志副本和冻结构建。主密码和 OCR 返回的非公开数据不属于本入口采样范围。

## 已保留的首次失败

旧 EXE `68f15acc614f8e5e0b75622078d4841de30e8d844c031af6b1247ce378b2b204` 的 100 对象组请求三次，均在完整 OCR 结果探针超时；选择器均关闭、实际扫描各一次，四项整组统计为空。确定性 CTC 回归和真实图片测试复现了普通 OCR 被 MRZ 专用替换破坏，已由独立提交 `6baee574` 修复（RF-1094）。该修复的 446 项 core 测试和实际图片测试通过，不能把它替代新 GUI 构建回验。

[RF-1094 原始证据](../verification/rf1094-general-ocr-2026-10-05.json)保留失败完整证明、公开输入、选择器结构、源冻结及红/绿检查。旧失败组 fresh CIM 核验 27 个身份退出，缓存清理保留三份日志和十二份附件密文；没有终止额外进程。修复后结果和未通过范围如下。

## 修复后实际验收及失败保留

冻结 Release SHA-256 `dc7ccb7bbb6f40088bf2d76e44d001536211773fb1e90d6c0d62292ca3f1c239`，459 项输入、94 份资源、171 个公开源文件和九个旧构建保持。100 对象组三次完整原生行程通过，每次仅扫描一次，完整三行结果通过两帧及视口命中探针。5000 对象两轮各请求三次，均只有两次完整成功，四项整组统计全部为空；不跨组拼接成功样本。

设备：Intel Core i7-9700，8 逻辑核心、系统物理内存约 15.79GiB；Windows 11 Enterprise LTSC 10.0.26100 x64。rustc 1.96.0，驱动 Node v24.16.0。以下只报告 100 对象 n=3 的中位 / P95，单位 ms；P95 为这三次最大值，含模型加载、识别、界面与采样开销。

| 指标                       |          100 对象 | 5000 对象            |
| -------------------------- | ----------------: | -------------------- |
| `pickerOpenedObservedMs`   | 1328.81 / 1506.25 | 未验收，整组统计为空 |
| `pickerSelectionClosedMs`  | 1089.82 / 1119.88 | 未验收，整组统计为空 |
| `firstOcrResultObservedMs` | 1840.09 / 4073.93 | 未验收，整组统计为空 |
| `totalOcrObservedMs`       | 3157.94 / 5434.20 | 未验收，整组统计为空 |

第一轮 5000 的 sample-003、第二轮的 sample-001 均在前置进程快照超过既有 15000ms 预算后被驱动拒绝，未写入输入授权；原始原生证明只有 startup 阶段、扫描零次，最终原因分别为 input-authorization-timeout 和 document-invalidated。不能把退出后的原生最终原因直接认作独立产品根因。其他四个实际 OCR 样本的完整结果仍保留，但不用它们输出 5000 分布。停止继续无依据复跑，下一步归因 PowerShell 启动、查询、序列化和驱动等待阶段，预算未加长、身份门禁未弱化。

96 项原生回归、严格 Clippy 和格式检查通过；Node 204 passed / 0 failed / 1 既有 symlink 权限 skip。默认前端无改动，不将旧 F/WEB 结果冒充本次新运行；RF-1094 的 446 项 core 检查已有独立证据。

新构建三组 fresh CIM 核验 63 条 PID/birth 身份退出，另确认 15 个缺少存量 birth 的清理 PID 已不存在；后者不冒充完整身份。只清理 72 个精确缓存路径、解除 9 个核验链接；清理前后完整 Vault 清单、36 份附件密文、公开输入/源和旧构建保持，日志副本保留，没有终止额外进程。旧首次失败组另外保留三份日志、12 份附件密文及全部证明；83 份 RF-1094 原件仍逐字节一致。

[结构化验收](rf312-windows-first-ocr-2026-10-05.json)和[逐字节原件索引](rf312-windows-first-ocr-2026-10-05/index.json)保存全部失败/成功、源码、冻结、检查和清理。RF-312 保持待验证；下一步先定位进程采样超时，再补 5000 整组首次 OCR、原首页超时及阶段耗时归因，随后继续 Windows RF-121 完整窗口材质。RF-112/121 和 RF-122～127 前置不解除。独立本地提交，不推送。
