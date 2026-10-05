# RF-312 Windows 原生 PDF 首屏文字可见采样

本入口测量固定公开 `text_only.pdf` 的首屏正文区域可见时间，使用真实 WebView2 SDK 输入与截图。原 PDF 能力、target/session 和结构诊断继续使用 `native-perf-pdf-diagnostic.mjs`；新入口为 `native-perf-pdf-preview.mjs`。仅 Windows 非默认 `native-perf` 构建支持，两种模式互斥。固定公开首屏正文区域在 Windows 两档各三次实际采样中通过验收；RF-312 整体仍未完成。

## 固定素材与就绪口径

- PDF 为 3035 字节，SHA-256 `ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe`。准备阶段已解密核验这一公开素材，再复制到全新的 owned Vault。磁盘附件为 3076 字节的 SOLC 密文，其摘要来自唯一的已核验文件证明；每次采样前后重新核对该密文摘要，不能用 PDF 明文摘要验证物理文件。GUI 通过正常附件协议解密并预览。
- 参考来自上一阶段十二份一致的真实公开截图，测试副本为 `tauri/src-tauri/src/native_perf/fixtures/pdf-first-page.png`，摘要 `5ffffdfc3c9f4a9db05a3170f5df71d07da13ce35a6528ffb011a435ff54b4c9`。PNG 为 1028×749、8-bit RGBA；正文区域固定为 x=180、y=225、width=430、height=36。
- 区域中 alpha≥240 且 RGB 各通道≤64 的像素记为 1，其余为 0。逐行二值掩码须同时满足 530 个深色像素和 SHA-256 `6f307e964c1dba0a23593241d6d193823cdf8feebb3946ca80b5af9a4a165044`。空白、缺字、错位及尺寸/格式变化均拒绝，不自动替换参考。
- 至少连续两次匹配；后一份 capture 开始与前一份身份核验结束实际相隔≥250ms。最多16次截图、首屏观察窗口25秒，沿用整个行程128协议调用和300秒上限。
- 主应用须为当前 owned main WebView、同一 browser PID、frame/loader 和 timeOrigin；截图 SDK 派发当下 Source 须为固定附件页。每次截图前后核验主树、同一公开 PDF embed、可见关闭入口与可信输入。主 frame/loader 变化立即拒绝；截图通过解码、主文档、DOM 和物理资源门禁后，若 PDF 子 frame 树变化，则标为 frameStable=false，保存原件并计入开销，不能用于首次匹配或连续就绪。在相同16次/25秒预算内继续捕获；仍要求连续两次前后树稳定且正文匹配，不为未知 target 附加 session。
- Rust 使用已有 image 0.25.10 依赖，在受限 PNG 长度、尺寸、格式和解码预算内提取掩码；Node 独立检查 CRC/格式/解压上限并交叉核对每份图像的五个像素字段。PNG 以 create_new 发布，驱动再核验路径、文件长度和摘要。

## 时间定义

`openingStartedAtMs` 是首个 SDK Preview pointer 输入之前的原生 Instant；各截图保存 capture 开始/完成、解码完成和前后身份核验完成时间。`firstMatchObservedMs` 是首次匹配且核验成功的观测时间差，`stableReadyObservedMs` 是连续两次匹配后的观测时间差，均为可见状态的观测上界，包含 SDK、解码与轮询开销。前序 PNG 写入也可能贡献耗时；最终 PNG 发布、Close 操作和 Node 事后复核不计入该就绪边界。

报告另列实际 capture 总开销、decode 总开销、poll 等待与截图次数，不用减法推算“净渲染时间”。只有要求的整组样本、源码数据集不变和进程清理均通过，才输出 median/P95；任一失败保留原件并抑制整组数值。三个样本只能描述该次固定素材观测，不外推全部设备性能。

验收范围是这个公开 PDF 的指定首屏正文区域，不能证明整页所有像素、其他页、任意 PDF 或 DWM 材质完成。OCR、系统睡眠及其他平台继续登记未测。

## 复跑

先按 [公开媒体合同](RF-312-media-fixture-contract.md) 准备媒体源。冻结源码、依赖、资源与 Release EXE 摘要后运行；输出必须是全新绝对路径。

```powershell
cd C:\Users\40299571\SoloSoul\tauri
node scripts/native-perf-pdf-preview.mjs --exe C:\public-perf\bin\solo_soul.exe --fixture C:\public-perf\vault100 --output C:\public-perf\new-pdf-samples --samples 3
```

旧 PDF 诊断结果不作为这项新就绪证明。收尾核对 fresh CIM 的 PID、birth、EXE 后，仅清理本轮明确归属的缓存与链接；保留公开源、Vault 密文、截图、证明及旧构建。RF-312 整体保持待验证，后续继续实际 OCR、性能归因及多端范围。

## 尚未确定原因的首页超时

第二轮100对象组的首个样本在登录请求后首页探针超时。窗口可见且有焦点，登录卡片仍在、提交按钮未禁用，observer仅证明一次login请求尝试。保留副本的passwordFailedAttempts为0、passwordLockedUntil为空；这不等于登录成功，也没有完成/错误分类或后端阶段耗时证据。后续独立归因应覆盖维护准入、同步停止、worker身份检查、密码派生/存储和前端完成，不能直接归为渲染慢、增加超时或补点击来宣称修复。

## 2026-10-05 固定公开 PDF 首屏文字可见验收

修复后的独立 Release SHA-256 `9d7fd32f8e603ee4d3fb0f444b2ec57178665d0032cf84752e2450abdb977058`，冻结 454 项输入和 94 份资源。90 项原生回归、严格 Clippy、格式与 diff 检查通过；Node 198 passed / 0 failed / 1 既有 Windows 文件 symlink 权限跳过。100/5000 对象各三次真实 owned 行程均通过，18 份实际 PNG 经 Node 独立解码复核，真实关闭及清理完成。

设备：Intel(R) Core(TM) i7-9700 CPU @ 3.00GHz，8逻辑核心；系统报告物理内存15.79GiB；Microsoft Windows 11 Enterprise LTSC 10.0.26100 64-bit。构建rustc 1.96.0 (ac68faa20 2026-05-25)，驱动Node v24.16.0。公开素材、固定视口与当前EXE限定了本次口径。

以下为每档 n=3 的观测中位 / P95，单位 ms；P95 为这三次的最大值，包含采样开销。不能据此声称产品瓶颈或任意 PDF 加载速度。

| 指标                    |          100 对象 |         5000 对象 |
| ----------------------- | ----------------: | ----------------: |
| `firstMatchObservedMs`  |   720.72 / 735.89 |   759.96 / 764.05 |
| `stableReadyObservedMs` | 1237.01 / 1293.28 | 1222.77 / 1241.83 |
| `captureOverheadMs`     |   441.76 / 501.34 |   452.29 / 455.11 |
| `decodeOverheadMs`      |     13.32 / 13.73 |     12.53 / 17.50 |
| `pollWaitMs`            |   523.38 / 527.21 |   520.48 / 526.25 |

第三轮三份原生首屏证明及九张截图均通过，但首个样本的事后进程快照超时，整组仍失败且五项统计均为空。没有选取成功子集；使用同一冻结EXE和原预算重新执行完整100对象组三次，再执行5000对象组三次。所有第三轮结果保留，采样超时仍未认定已修复。

第二轮三次分别在首页探针、驱动 PowerShell 进程采样和 PDF 子树变化门禁失败，整组指标为空，原件及第二版 EXE 保留。其中第三次调用过一次截图，但旧schema1在子树门禁失败后未发布PNG，原响应不可恢复；保留证明/调用列表/源码与EXE，不借用旧参考PNG补造原件。新 v2 合同保留已通过上述门禁的子树不稳定截图、记录 frameStable=false 并计入开销，只有两次后续树稳定且文字匹配才接受；主 frame/loader 门禁、16次/25秒及128协议调用上限保持。首页超时原因仍未确定，驱动采样超时仍判失败。

首轮100对象三次均因明文/密文资源边界错误拒绝，未输出统计数值；原组、EXE 和最初测试失败完整保留。修复仅在非默认采样器中绑定准备阶段已解密核验的公开素材及唯一物理密文摘要，未改变生产附件协议。

fresh CIM 核验最终两组 60 条进程身份已退出，仅删除 48 个精确 owned 缓存目录并解除 6 个私有链接，没有终止额外进程。24份最终副本附件密文、公开源171文件、旧构建及 Vault 保持；首轮失败清理另有独立回执。

[结构化验收](rf312-windows-pdf-first-page-2026-10-05.json)和[逐字节原件索引](rf312-windows-pdf-first-page-2026-10-05/index.json)保存完整失败与成功。只验收指定首屏正文区域；整页、其他页、任意 PDF、OCR、睡眠和多端不包含在该结果中。下一本地阶段为实际 OCR 和性能归因，原首页偶发超时继续待查。独立本地提交，不推送。
