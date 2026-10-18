# RF-312：Android 主进程与 renderer 内存采集入口

2026-10-14，新增只读采集器，补齐旧 Android 诊断只读取主进程 PSS 的范围缺口。相同冻结 APK 的 100/5000 对象完整 12 阶段行程各一次通过，主进程与 renderer 的绑定、UID、PID 和出生标识均实际核对。**只接受采集器可用性；RF-312 未关闭，这份旧 APK 也不是当前 main 的性能基线。**

主 APK SHA-256 为 `098c26120fc4ab6954efb10116d62c294f1f4afa5478a4a5cff35adc24ba7f6f`，独立测试 APK 为 `ea896d90db399a8cab6160835edbaf45ed2a6b544ec53c0f0e46bbd8f6b114d8`。沿用此前重复行程的 API31 x86_64 / WebView 91.0.4472.114 / SwiftShader / 2048 MiB / 2 核和私有 ADB5049，产品源码、依赖及两 APK 未修改。旧队列 guard 与当前工具来源的 76 项差异单独保留，不能把旧主包的来源声明成当前源码。

## 采集与拒绝条件

`tauri/scripts/android-process-memory.py` 从实际 ActivityManager 进程及服务表核对应用 UID、renderer 的隔离身份、唯一客户端绑定和 WebView provider。逐 PID 读取 `/proc/.../stat` 的出生标识及真实 UID；内存查询前后和整个组结束再次核对身份及服务集合。缺失、替换、未知绑定、命令失败和缺少数值都拒绝本快照；未运行和 renderer 尚未出现分别记录，不返回零内存。

只执行 `dumpsys meminfo --local -s <PID>`。该选项在系统端生成表格，避免向目标进程调用 `dumpMemInfo`，见[实际 Android 框架实现](https://android.googlesource.com/platform/frameworks/base/+/d42d2f86e906c68aa5f3bb7091c1875867e2b1e3/services/core/java/com/android/server/am/ActivityManagerService.java)。本机帮助原文也已归档。PSS 按共享页面的份额计数，见[Android 工具说明](https://developer.android.com/tools/dumpsys)；串行求和仍不是原子快照。

最初探索轮的完整行程失败：一次未加 `--local` 的单独 renderer 查询只有标题，随后 renderer Binder 线程空指针、主进程因 renderer 退出而终止。时间、栈和失败记录保留；没有据此认定产品 OCR 的唯一故障原因。改用 `--local` 后两档完整行程均通过。压缩 crashpad 数据不入库，只归档当前失败的诊断文本。

## 本轮结果

| 对象数 | 原生完整行程 | 快照尝试 | 行程内有效快照 | 单轮最大采样合计 PSS | 单组查询最长耗时 |
|---|---|---:|---:|---:|---:|
| 100 | 12 阶段通过、恢复通过 | 27 | 14 | 280.93 MiB | 2022.64 ms |
| 5000 | 12 阶段通过、恢复通过 | 28 | 13 | 298.68 MiB | 2845.52 ms |

55 次快照尝试全部保留：27 份有效，26 份应用未运行；5000 组另有一份启动绑定未稳定、一份退出时 `/proc/PID/status` 已消失，均拒绝。有效原件通过同一采集器回放，数值及身份逐项相等，并与真实 native PID、UID 和原生行程时钟对齐。采样组之间等待 5 秒，外层最长耗时分别 2493.51/3034.55 ms，包含 AVD 核验和记录开销。

主进程 PSS 包含测试 runner；共享 WebView 服务、zygote 和系统进程不计入这份应用与直接绑定 renderer 的合计。每档仅一次，表中的最大采样值不是连续峰值，不能形成中位数/尾延迟结论，也不证明未插桩性能或实体设备表现。

12 项 Python 回归通过，覆盖真实标题无数值的输出、归属不明确、PID 复用、UID 变化、读取期间集合变化和零值误报等。三次诊断（含失败）各 216 条私有目录内容/链接/权限独立恢复核验一致，原安装包/UID/系统设置和公开输入恢复；六份私有 TAR 已精确删除。

RF-1137 已在后续冻结 APK 的真实两档行程中把 listen/unlisten 从 435/425 降至 35/25，见[既有验收](../verification/rf1137-native-acceptance-2026-10-13/result.json)。本次复用较早 APK 验采集能力，不能以旧发送量再报告已修复的订阅缺陷。后续需要当前源码 APK 的重复全进程内存、仪器开销与进程冷启动，再补物理硬件/多端及 IME 退场后的稳定截图。

## 复跑与原件

在 `tauri/scripts/` 执行 `python -m unittest test_android_process_memory -v`。单次严格快照的输出目录必须不存在，AVD/UID/provider 必须从专用隔离环境核对，正式账户不用于本入口：

```powershell
python android-process-memory.py --adb D:\SoloSoul\build\android-native-feasibility-20261010\sdk\platform-tools\adb.exe --serial emulator-5586 --avd SoloSoul_RF121_API31_20261010 --package com.solosoul.app --uid 10105 --provider com.android.webview --output D:\SoloSoul\build\new-owned-memory-snapshot
```

[结构化结果](rf312-android-process-memory-2026-10-14.json)、[1566 成员索引](rf312-android-process-memory-2026-10-14/index.json)和[原件包](rf312-android-process-memory-2026-10-14/raw.zip)包括工具来源、原失败、两档所有查询、两次完整 native 结果及三次恢复核验。全部成员长度/SHA-256 与 ZIP CRC 回读通过；不含 APK、库、签名密钥、实体 Vault、夹具 TAR 或私有备份。


## 2026-10-16：当前摘要格式兼容（RF-1170）

当前默认 Release API31 控制的第二轮在完整 renderer 摘要上被误拒绝：框架输出`TOTAL SWAP PSS`，旧解析器只识别`TOTAL SWAP (KB)`。框架根据`hasSwappedOutPss`选择这两种行，见[实际格式分支](https://android.googlesource.com/platform/frameworks/base/+/2567308affb2a60e7a78f296ebe6ab243fc17cdd/core/java/android/app/ActivityThread.java)；比例swap与普通swap的定义分别见[Debug.MemoryInfo](https://android.googlesource.com/platform/frameworks/base/+/179923a/core/java/android/os/Debug.java)。不能把比例数值直接记成普通swap。

- `TOTAL SWAP (KB)`保持既有`swapBytes`字节值，旧结果结构不变。
- `TOTAL SWAP PSS`输出`swapPssBytes`，同时`swapBytes=null`表示普通swap不可知；0只用于系统实际报告0，不用0补缺。PSS/RSS使用原始报告的数值，不将swap再次加入合计。
- 只接受完整且唯一的单行总计；错误PID/名称、缺字段、未知swap标签、同一行重复两类swap、重复总计，或不满足PSS大于0／RSS不小于PSS的条件时仍拒绝。归属、进程出生/UID、前后集合、`--local`、超时和失败记录保护保持。

七项新增回归与原读取器/Home回归共73项通过，四份本次真实完整摘要回放通过；原始比例swap误拒绝得到复现，16份变造的截断/重复/错误身份/缺失数值输出仍拒绝。[去标识验证摘要](../verification/rf1170-native-swap-summary-2026-10-16.json)绑定原始失败及恢复证据。失败控制队列只尝试2/10，首轮通过、次轮失败后停止；两轮均恢复142项内容/链接/权限及根权限，自有包/进程和系统设置收尾通过。**旧队列保持失败，不形成分布；本项是解析器修复及回放，不是新完整原生控制验收，RF-312继续待验证。**
