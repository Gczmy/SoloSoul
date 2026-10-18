# RF-312：既有原生转储中的进程 RSS 高水位

2026-10-17，在已完成的默认包原生记录中补读内核 `VmHWM`，不新增设备命令或性能样本。原五轮各 100／5000 对象及后续有效状态入口的一轮各一档，合计十二个检查点；原来的 PSS 数值、SDK 记录、失败、APK 和来源保持。

## 定义与边界

[Linux 内核文档](https://www.kernel.org/doc/html/v6.12/filesystems/proc.html)将 `VmHWM` 定义为进程 RSS 高水位，也说明 RSS 记账异步、数值精度有限。本项读取截至行程后检查点仍存活进程的内核记账值；具体峰值发生时间未知，不能定位到 OCR、PDF 或其他阶段。

主进程与绑定的隔离 renderer 分开呈现。它们的高水位发生时间可能不同，RSS 还包含共享页面，因此不相加生成同时总峰值；没有得到 PSS 峰值或退出进程的历史峰值。共享 WebView 服务、zygote 和系统进程仍在原口径之外。

## 读取与复跑

[离线入口](rf312_kernel_highwater.py)需要显式的新输入 JSON：

```json
{
  "schemaVersion": 1,
  "checkpoint": "D:/validation/prior-run/post-journey-memory",
  "evidenceSha256": {
    "checkpoint": "<该目录 result.json 的 SHA-256>",
    "commands": "<该目录 commands.json 的 SHA-256>",
    "sdkCommands": "<上级原生行程 commands.json 的 SHA-256>",
    "nativeReport": "<上级原生行程 report.json 的 SHA-256>"
  },
  "readerSha256": {
    "memory": "<tauri/scripts/android-process-memory.py 的 SHA-256>",
    "state": "<docs/performance/rf312_instrumentation_state.py 的 SHA-256>"
  }
}
```

在仓库根目录执行，结果路径必须不存在：

```text
python -B docs/performance/rf312_kernel_highwater.py --input D:/validation/highwater-input.json --output D:/validation/new-highwater-result.json
python -B -m unittest discover -s docs/performance -p test_rf312_kernel_highwater.py -v
```

入口验证原默认行程、完整恢复和检查点通过，再将全部 22 条记录的命令、退出码及 stdout／stderr SHA 与原 SDK 台账中的唯一连续序列匹配。复用未修改的原 PSS／进程归属解析器，精确重放 UID、PID、出生时间、renderer 服务绑定、前后集合和数值；仅忽略离线重放重新产生的采集耗时。两个完整有效进程转储必须实际无活动插桩，旧专用命令的 Unknown command 回复不作缺席证据。

每个进程两份 status 必须绑定原 kernel identity，`VmHWM`／`VmRSS` 字段唯一、单位为 kB、数值有效，高水位不倒退。缺失、歧义、错误、stderr、变更或原 SDK 摘要不符均拒绝，不修补或替换失败原件。结果保留原始字节摘要；它是历史原件的新指标回放，不是新原生运行。

## 五轮结果

默认包保持 API31/x86_64、2 CPU／2048 MiB、SwiftShader 和同一 APK。每格 n=5；按原轮次保存所有值。单位 MiB，经验 p95 为这五次中的最大值，不推断总体尾部。

| 对象数 | 进程 | 五次内核 RSS 高水位 | median | 经验 p95 / max |
| --- | --- | --- | ---: | ---: |
| 100 | 主进程 | 380.05 / 379.55 / 374.91 / 377.50 / 376.00 | 377.50 | 380.05 |
| 100 | renderer | 222.34 / 217.72 / 223.73 / 221.55 / 220.78 | 221.55 | 223.73 |
| 5000 | 主进程 | 365.66 / 368.44 / 360.24 / 364.29 / 365.93 | 365.66 | 368.44 |
| 5000 | renderer | 221.90 / 224.64 / 228.17 / 225.34 / 230.24 | 225.34 | 230.24 |

后续有效状态入口是另一系列，每档仅一轮：100 对象主进程／renderer 为 358.11／221.23 MiB，5000 对象为 371.68／227.40 MiB；不并入五轮或给出其尾部分布。

## 验证与后续

56 项相关测试通过，其中新增 15 项，原内存和有效状态测试均保持，无重复或跳过。首轮 1 项 CRLF 解析错误及对应六份源码已保全；修正后整组通过。十二份 CLI 结果、264 条原命令、24 份有效状态转储和 48 份 status 原件独立复核一致，得到 24 条逐进程高水位记录。没有设备输入、重启应用、重置高水位或改动默认／诊断包。

[去标识数值与证据哈希](rf312-kernel-highwater-2026-10-17.json)包含完整原轮次数据和两系列边界；路径、实际进程身份和原始日志只留本机。本组没有证明全程同时总内存峰值、因果观察开销、应用本体时延或启动提交帧，也不足以决定全路由懒加载或分页。RF-312 与 365/374 保持未完成，下一步仍需补充完整行程内存覆盖和其余原验收范围。
