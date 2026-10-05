# RF-312 Windows 原生附件预览测量

本入口只用于显式启用 `native-perf` 的 Windows 测试构建和已验收的公开媒体数据集。默认客户端、旧 v1 输入行程、同 profile 启动行程及诊断入口继续使用原合同。

## 固定行程与边界

SDK 发送真实指针及文本输入，依次完成启动、主密码解锁、对象工作区、附件列表、PNG 预览、文本预览、搜索、锁定及再次解锁。图片和文本预览之间通过真实关闭按钮返回附件列表。当固定目标在主内容滚动容器的可见区域外时，最多三次发送经过主文档与命中检查的真实 SDK 滚轮输入，然后重新核对坐标；不直接写 scrollTop，不扩大窗口或绕过可点击检查。业务读取仍走真实 Tauri IPC 和加密附件存储，不向探针开放任意密码、表达式、文件或业务命令。

图片结束条件为公开 PNG 的 `complete`、500×200 自然尺寸、`decode()` 成功、可见预览和两帧绘制。文本结束条件为 71 字节固定公开内容完整匹配、可见预览和两帧绘制。证明只记录类型、布尔状态、尺寸和帧数，不保存解密文本、图片 URL 或 IPC payload；对应阶段必须观察到真实 `fs_read_file_as_data_url`／`fs_read_file_as_text` 调用。

准备模式复用[媒体数据合同](RF-312-media-fixture-contract.md)的校验和复制代码，只操作新 owned 副本并重定位附件路径；源数据前后保存全部文件摘要。独立核对 owned 标记与九份文件证明后才能启动 GUI；运行前继续检查目录所有权、一次消费和原始字节。原有进程、主文档、frame、loader、时钟和输入可信性门禁保持。

九阶段中任一样本失败、清理不完整、源数据变化或采样中断，整组预览统计均为空，不从成功子集汇报中位数或 p95。失败样本及原始日志保留。计时包含 SDK 操作和文档核验开销；内存是有实际查询窗口的工作集采样最大值，不等于连续峰值。

PDF 预览、OCR、系统睡眠及其他平台均未由本行程测量。已有同 profile 启动测量属于另一入口，不能混作同一组结果。开发与单元验证完成也不等于真实 GUI 采样完成。

## 复跑命令

在 `tauri/` 构建专用 Release 客户端。测试 EXE 需要与当前构建一致的打包资源，冻结其 SHA、源代码、前端产物和资源清单；不要使用旧 EXE 或正式账户。

```powershell
cargo build --locked --release -p solo_soul --features native-perf

node scripts/native-perf-media.mjs --exe 'C:\TEMP\rf312-preview\bin\solo_soul.exe' --fixture 'C:\TEMP\rf312-media\vault100' --output 'C:\TEMP\rf312-preview\samples100' --samples 5 --memory-interval-ms 2000
node scripts/native-perf-media.mjs --exe 'C:\TEMP\rf312-preview\bin\solo_soul.exe' --fixture 'C:\TEMP\rf312-media\vault5000' --output 'C:\TEMP\rf312-preview\samples5000' --samples 5 --memory-interval-ms 2000
```

两个源目录应由媒体生成器生成，`--output` 的父目录必须存在且输出目录尚不存在。输出保存 `native-perf-sdk-journey-results.json`、逐样本报告和私有 Vault；每条命令等待真实退出并保存 stdout、stderr、退出码。只有整组成功才能接受其性能分布。后续检查日志、sourceUnchanged、same-document、各阶段媒体结束条件、进程身份和清理回执，不能仅看工具退出码。

## 本地检查

```powershell
cargo +1.99.0 fmt --all -- --check
cargo test --locked -p solo_soul --features native-perf --lib native_perf -- --test-threads=1
cargo test --locked -p solosoul-core --example perf_baseline -- --test-threads=1
cargo clippy --locked -p solo_soul --features native-perf --all-targets -- -D warnings
cargo clippy --locked -p solosoul-core --example perf_baseline -- -D warnings
node scripts/run-node-tests.mjs
```

格式化工具链版本是本轮环境选择，业务编译使用当前 workspace 的锁文件与工具链。所有检查需记录实际结果和既有跳过，不能将命令清单视为验收证据。

## 2026-10-05 实测结果

Windows 11 Enterprise LTSC 26100、i7-9700（8 核）、约 16 GiB RAM、WebView2 154.0.4258.53；专用 Release EXE SHA `11f374b8d936f43a9c96b6116243c317c0b24b0c8e109d1b0945b8550c5c00f5`。100/5000 对象各 5/5 完整通过，共 90 阶段，使用同一构建与固定公开素材。

| 阶段               | 100 对象中位数 / p95（ms） | 5000 对象中位数 / p95（ms） |
| ------------------ | -------------------------: | --------------------------: |
| 启动               |            2188.0 / 3620.0 |             1759.4 / 3341.6 |
| 主密码解锁         |            2360.4 / 2543.5 |             2330.6 / 3284.0 |
| 对象工作区         |              387.5 / 541.0 |               871.4 / 959.4 |
| 附件列表（含滚动） |              290.0 / 326.6 |               384.1 / 426.5 |
| PNG 预览           |              167.8 / 180.8 |               167.0 / 183.4 |
| 文本预览           |              187.1 / 189.7 |               172.2 / 183.5 |
| 搜索               |              462.0 / 472.3 |               512.1 / 523.2 |
| 锁定               |              151.3 / 153.6 |               150.9 / 154.1 |
| 再次解锁           |            2366.7 / 4030.0 |             2197.4 / 2405.4 |

两组分别有 59/56 个有效内存采样点，观察工作集最大值的中位数为 667.7/760.6 MiB。目标间隔 2000 ms 不保证实际分辨率；最长查询窗口及实际起点间隔随原件保存。计时包含 SDK 开销，n=5 时 p95 即最大值，不据此归因产品瓶颈。

首版五次都在屏幕外的附件入口拒绝，整组指标为空。补充有界真实滚轮输入后，两组每次均使用一次滚轮、16 次可信指针输入和 3 次文本输入，98 次 SDK 调用；没有更改窗口大小、点击门禁或源数据。旧 EXE、五份失败和所有原始阶段保留。

62 项原生回归、7 项共享示例回归、严格 Clippy、格式与 Release 构建通过；固定 Node 入口 175 通过、0 失败、1 项既有 Windows symlink 权限跳过。两组原件、源摘要、冻结输入、失败检查与清理记录见[结构化证据](rf312-windows-media-preview-2026-10-05.json)和[逐字节原件索引](rf312-windows-media-preview-2026-10-05/index.json)。

RF-312 仍未完成。下一本地项是实际 PDF 渲染与 OCR 测量，再进行 KDF/存储/React/SDK 归因；原偶发首页超时、多端和睡眠缺口保持，RF-112/121 及 RF-122～127 的前置不解除。


## PDF 后续能力诊断（2026-10-05）

100 对象三次独立 PDF 能力诊断已完成，可见正文的三份截图相同，但主 session 未暴露 PDF 子 frame/context。详见[实际诊断与证据](RF-312-native-pdf-diagnostic.md)。该范围保持空性能指标，不并入本页图片/文本分布；下一步先建立可靠 PDF 就绪门禁，再补 PDF/OCR 测量与归因。
