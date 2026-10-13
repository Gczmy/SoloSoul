# RF-312 当前 Windows 首页行程与内存复核

2026-10-10，RF-1104 后的新冻结构建完成公开 100/5000 对象各五次完整行程。十次均通过，97/97 个内存点有效，二十条认证流程结束，维护准入失败为零；本组没有复现旧首页超时。**RF-312 整项仍待验证，旧超时根因仍未知。**

冻结基线为 `9687d53a0dfa91487e58631e70d1013d2a8f7980`；GUI EXE SHA256 为 `62c67208b195b6f738f6dce120914fa9f27e9cb0bc2024c086b61fedc016bd42`。复用 RF-1104 已验收构建，94 个资源与 1827 项源码输入前后逐字节核验；本阶段不改生产代码、依赖、默认路径或采样预算。

设备为 Intel Core i7-9700、8 核/8 线程、约 16 GiB RAM，Windows 11 Enterprise LTSC 26100，Node 24.16.0。两档均使用生产 Argon2id 64 MiB/3 次/并行 4 的公开合成 Vault；各样本新建私有 root/profile，不访问正式账户。TEMP/TMP 使用 D 盘，GUI 不继承 PDFium 路径覆盖。

## 当前观测

下面每格为中位数 / nearest-rank P95，单位 ms，每档 n=5，P95 是本组最大值。包含 SDK、认证诊断和查询开销，不能当作未插桩性能基线或与旧构建的受控优化对照。

| 阶段 | 100 对象 | 5000 对象 |
|---|---:|---:|
| 登录表单启动 | 2731.40 / 3723.52 | 3369.32 / 6921.49 |
| 主密码解锁至首页 | 572.66 / 655.27 | 638.91 / 1160.13 |
| 工作区首批卡片 | 480.62 / 572.62 | 1045.67 / 1342.18 |
| 搜索 needle | 403.33 / 416.01 | 464.38 / 492.36 |
| 应用锁定 | 85.81 / 100.00 | 101.66 / 120.32 |
| 主密码重解锁至首页 | 542.92 / 774.31 | 633.60 / 664.32 |

每两秒目标间隔进行串行 owned 进程树查询，UI 前后另有固定检查点，实际间隔保存在原件中。查询不重叠，每点重新验证 PID/birth、唯一 WebView browser、精确 UDF 与 working set 总和；确认退出的短命后代单列，不伪造零读数。

| 内存与查询观测 | 100 对象 | 5000 对象 |
|---|---:|---:|
| 有效点数 | 44 | 53 |
| 采样最大工作集中位 / P95（MiB） | 588.62 / 601.72 | 698.53 / 758.31 |
| 单次查询中位 / 最大（ms） | 1039.04 / 1217.08 | 1028.63 / 1345.25 |

工作集相加可能重复计算共享页，查询窗口不是原子快照，采样最大值不能代表连续峰值。旧首页超时记录、旧失败组与旧构建全部保留；本组成功不证明 RF-1104 修复了旧首页超时，不把成功子集拼成整组。

## 复跑与证据

在已按公开夹具合同准备并冻结的 D 盘测试包上，从 `tauri/` 执行，两个输出目录必须不存在：

```powershell
node scripts/native-perf-sdk-journey.mjs --exe D:\SoloSoul\build\rf1104-update-preferences-20261010\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault100 --output D:\SoloSoul\build\new-home-memory-100 --samples 5 --memory-interval-ms 2000
node scripts/native-perf-sdk-journey.mjs --exe D:\SoloSoul\build\rf1104-update-preferences-20261010\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault5000 --output D:\SoloSoul\build\new-home-memory-5000 --samples 5 --memory-interval-ms 2000
```

沿用原 SDK 128 调用、50 秒阶段和 300 秒完整行程预算。两个驱动 exit 0；另一次独立复核重读十份原生证明，执行现有行程/认证/维护合同，并重新计算所有内存点的身份、读数和分布，exit 0。源码和工具没有变化，不重复未受影响的 Rust/前端全套检查，不将旧检查当作本阶段新运行。

fresh CIM 核对 90 个记录身份退出后，缓存工具仅删除本轮 80 个精确缓存目录、解除 10 个联接，没有终止额外进程。240 个 Vault/根级证明文件及十份应用日志在清理前后逐字节校验；原始公开夹具、构建和资源保持。

完整[结构化验收](rf312-windows-current-home-memory-2026-10-10.json)与[原始字节索引](rf312-windows-current-home-memory-2026-10-10/index.json)归档全部十次样本、驱动输出/退出码、冻结输入、设备、复核和清理脚本。压缩包逐成员回读长度和 SHA256，不包含 EXE、DLL、模型、Vault 实体或缓存树。

下一步补 RF-121 Windows 实际主题/色板/前台状态绑定的窗口矩阵；首页问题出现时继续有界失败归因，并可补当前构建的同 profile 重启复核。系统睡眠、多端与 RF-112/121 原生缺口仍未完成，RF-122～127 前置不解除。按 RF-312 本阶段独立本地提交，不推送。
