# RF-312 Windows 一体顶栏构建的配对启动基线

2026-10-14，新一体顶栏前端所用的非默认 Windows Release 完成 100／5000 对象各五对启动，共二十次原生启动，全部通过。EXE SHA 为 `446e6221baa45285607fa594ba4dd7ea151edf8de83ce281c6fcd80aad3734a1`，构建源提交为 `a7fbda801`；验证时 HEAD 为仅补验证文档的 `217ef3028`，1983 份运行源与构建清单逐字一致。复用新程序及已核验的 94 项资源，未重复编译或修改产品。

设备为 Windows 11 Enterprise LTSC 26100、i7-9700（8 核／8 线程）、约 15.8 GiB 可见内存、Node 24.16.0。两个专用公开合成 Vault 使用生产 KDF。每对首次使用新隔离 profile，原有进程退出后保持同一 Vault、profile 和 WebView 目录再启动；两次 PID、runId、frame、loader 和 timeOrigin 独立。

| 对象数 | 样本量 | 新 profile 中位／p95（ms） | 同 profile 重启中位／p95（ms） |
|---|---:|---:|---:|
| 100 | 5 对 | 2654.12／3159.82 | 1260.05／1345.77 |
| 5000 | 5 对 | 2485.95／2805.79 | 1280.73／1293.00 |

计时从原生 runtime 配置开始，到主密码表单可用、两次绘制及 WebView2 SDK 绑定，排除 OS spawn、准备和重启预检，包含 SDK 开销。没有清除 OS 缓存，不称为系统或存储冷启动。每档 n=5 的 nearest-rank p95 是最大值；不与旧构建拼接，也不据未经受控的前后差值宣称性能改进。

二十次均只读，无 `Input.*` 方法、密码输入、像素采集或系统设置改变。40 份登录就绪前后工作集快照身份和读数有效；合计可能重复计算共享页，不是连续峰值或完整行程内存。未执行解锁、搜索、OCR、预览、锁定恢复、系统睡眠或其他平台测量；RF-121 材质矩阵和跨进程持久化窗口状态恢复仍未验收。

独立复核重读全部证明，调用原启动、配对、偏好及票据合同，重算统计并核验夹具和资源。归档票据在记录的 `warm.beforeSnapshot.observedAt` 时刻重放原五分钟门禁，不获得当前消费权限；本轮没有新增过期票据现场测试。两个驱动与独立复核均 exit 0，原预算未改变。

fresh CIM 确认 180 个记录身份退出后，清理 80 个精确测试缓存目录、解除 10 个自有联接；260 份公开 Vault／根级／两代启动证明及日志清理前后 SHA 相等。清理工具没有终止其他进程，正式客户端目录不在清理范围内。

复跑时先按本轮输入清单准备同源非默认 Release 和两档公开夹具，再使用不存在的输出目录，在 `tauri/` 执行：

```powershell
node scripts/native-perf-startup.mjs --exe D:\SoloSoul\build\rf121-theme-binding-20261010\current-windows-native-retry1-20261014\runtime\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault100 --output D:\SoloSoul\build\ss-rf312-new-integrated-startup-100 --samples 5
node scripts/native-perf-startup.mjs --exe D:\SoloSoul\build\rf121-theme-binding-20261010\current-windows-native-retry1-20261014\runtime\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault5000 --output D:\SoloSoul\build\ss-rf312-new-integrated-startup-5000 --samples 5
```

沿用 D 盘 TEMP/TMP、清除 PDFium/WebView 测试覆盖环境，并在每档开始前核验当前交互会话和输入桌面。结束后用记录的实际进程身份和 owned 标记核验退出、保全证据并清理精确缓存。

[结构化数据](rf312-integrated-windows-paired-startup-2026-10-14.json)和[246 份原件索引](rf312-integrated-windows-paired-startup-2026-10-14/index.json)保留所有样本、构建／来源、设备、命令、独立复核和清理证据。归档逐成员核对长度与 SHA，不包含 EXE、资源实体、Vault 或 profile。RF-312 整项保持待验证，goal active。
