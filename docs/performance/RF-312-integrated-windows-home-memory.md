# RF-312 Windows 一体顶栏构建的首页行程与内存基线

2026-10-14，同一新 Windows 非默认 Release（SHA `446e6221baa45285607fa594ba4dd7ea151edf8de83ce281c6fcd80aad3734a1`，运行源 `a7fbda801`）完成 100／5000 对象各五次完整首页业务行程，十次通过。1983 份运行源、94 项资源和公开夹具前后逐字一致，未重新编译。设备与[同构建配对启动](RF-312-integrated-windows-paired-startup.md)相同；每次使用新私有 Vault／profile，目标内存采样间隔为 2000 ms。

每格为中位数／nearest-rank p95，单位 ms。每档 n=5，p95 是最大值；包括原 SDK、认证／维护诊断、输入／绘制和采集开销，不代表默认未插桩性能。

| 阶段 | 100 对象 | 5000 对象 |
|---|---:|---:|
| 登录表单启动 | 2990.19／3515.78 | 2750.16／3256.65 |
| 主密码解锁至首页 | 624.12／646.94 | 637.43／682.24 |
| 工作区首批卡片 | 357.65／508.52 | 1004.12／1107.07 |
| 搜索 needle | 468.92／469.33 | 500.56／531.93 |
| 应用锁定 | 112.15／120.19 | 158.29／185.63 |
| 再次解锁 | 566.62／626.67 | 662.30／681.23 |

原 SDK 128 调用、50 秒阶段和 300 秒行程预算保持。两档 driver 与独立复核 exit 0；原生行程、认证与维护归属、进程身份、数值和统计均重验，本轮没有复现旧首页超时，不宣称旧根因已修复。工作区等待首批 50 张卡片；搜索实际显示 5／50 条，5000 档的完整匹配是 250，原显示上限为 50。

两档各 44／44 个内存点有效。每轮最大采样工作集合计的中位／p95 为 645.47／692.06 和 769.40／778.95 MiB；包括主进程和经身份、目录及出生时间核验的 WebView 树。合计可能重复计算共享页，串行查询不是原子快照，最大采样不是连续峰值或独占内存。

实际观察器事件逐项重数后，完整行程的 JavaScript HTTP invoke 尝试中位数为 151／148、最大为 153／156；十次事件 listen／unlisten 均为 28／17。口径排除参数、响应和最后观察点后的卸载，不是后端或 OS 全部 IPC，差额不证明监听泄漏。总对象数放大五十倍时调用量未成倍增长；保留现有共享订阅和首批卡片呈现，不据这组插桩样本启动全路由懒加载或全面分页迁移。工作区大数据档仍可进一步归因，但本轮不建立未经证实的优化 ID。

fresh CIM 确认 90 个实际记录身份退出后，清理 80 个精确缓存路径；本轮无联接需要解除，240 份公开 Vault／根级证明和日志清理前后字节一致。清理工具未终止其他进程。OCR、媒体预览、系统睡眠、跨进程持久化窗口状态、完整材质／辅助功能和其他平台不在本组验收范围。

复跑时冻结同源非默认 EXE、94 项资源及两档公开夹具，使用 D 盘 TEMP/TMP、无 PDFium/WebView 覆盖环境和新输出目录，在 `tauri/` 执行：

```powershell
node scripts/native-perf-sdk-journey.mjs --exe D:\SoloSoul\build\rf121-theme-binding-20261010\current-windows-native-retry1-20261014\runtime\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault100 --output D:\SoloSoul\build\ss-rf312-new-integrated-home-100 --samples 5 --memory-interval-ms 2000
node scripts/native-perf-sdk-journey.mjs --exe D:\SoloSoul\build\rf121-theme-binding-20261010\current-windows-native-retry1-20261014\runtime\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault5000 --output D:\SoloSoul\build\ss-rf312-new-integrated-home-5000 --samples 5 --memory-interval-ms 2000
```

每档开始前核对交互会话／桌面；结束后保持原合同，重读身份与内存点，保全证明并按 owned 标记清理。所有十次样本、来源、命令、独立复核及清理保存在[结构化数据](rf312-integrated-windows-home-memory-2026-10-14.json)和[197 份原件索引](rf312-integrated-windows-home-memory-2026-10-14/index.json)，逐成员长度／SHA 回读通过。后补的 [IPC 重数原件](rf312-integrated-windows-home-memory-2026-10-14/ipc-supplement.json)独立核验，保留初次离线读取字段误用的源码，原主归档字节不变；没有重跑原生样本。归档排除程序、模型、实体 Vault 和缓存，RF-312 整项保持待验证。
