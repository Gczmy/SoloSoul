# RF-312 当前 Windows 同 profile 启动复核

2026-10-12，Windows 11 Enterprise LTSC 26100、console 会话 1，Node 24.16.0。
使用已冻结的非默认 Release 程序 `6f9e1c33a449eafd3843549b56db8e5c3ecee5f24157416ed655ee4a66a0c682`，
公开 100／5000 对象夹具各运行五对，共 20 次原生启动；两个驱动均 exit 0，全部十对接受。
本轮未重建程序。1839 个当前验证输入与原构建清单分别保存；四处差异只涉及 Android Card 测试、
其 README 和外部 Python 验证，Windows 生产源码、启动引擎、前端及 94 项资源未变。

| 对象数 | 样本量 | 新 profile 中位／p95（ms） | 同 profile 重启中位／p95（ms） |
| --- | --- | --- | --- |
| 100 | 5 对 | 3255.583／4068.808 | 999.580／1049.335 |
| 5000 | 5 对 | 2790.783／3546.896 | 1004.859／1119.384 |

计时从原生 runtime 配置开始，到主密码表单可用、两次绘制和真实 WebView2 SDK 绑定。
不含 OS spawn、准备和重启预检，包含 SDK 开销。每对首次使用新私有 profile；自有进程退出后，
第二次保持同一 Vault／profile／WebView 目录及其出生时间，使用不同 runId、PID、端口、
frame、loader 和 timeOrigin。没有清空 OS 缓存，不称为 OS 冷启动。n=5 的 nearest-rank p95 是最大值；
本轮不与旧构建、不同会话的样本拼接，也不据这组数据提出产品优化。

入口只读，20 次证明的 SDK 方法均无 Input 调用；未执行解锁、搜索、OCR、预览、锁定恢复、
系统睡眠或其他平台测量。40 份登录就绪前后工作集快照中 39 份有效；100 对象首对首次启动的
afterSnapshot 有一项已核验进程内存读数不可用，原聚合值 `null` 和原因保留。
它不是完整内存基线，也不是启动峰值；未丢弃该启动对或将缺失值补为零。

独立读取全部原件，重验原启动合同、身份／票据 SHA、同目录复用、公开夹具内容、IPC 和统计。
首次读取误用了字段名，第二次正确触发历史票据的实时五分钟过期拒绝，第三次额外要求所有内存
读数可用而拒绝；三个原件保留。最终归档核验在记录的 warm.beforeSnapshot.observedAt 时刻
调用原五分钟票据合同，保持原预算和字段条件；该核验不授权今天重新消费票据。
内存缺失单独报告，原启动合同未修改；没有重跑原生样本。

fresh CIM 确认 180 个记录身份不再存活后，缓存工具只清理 80 个精确测试路径、解除 10 个自有联接。
260 份 Vault／根级／两代启动证明文件及应用日志副本在清理前后保持字节一致；缓存工具没有终止
其他进程。主要 profile、WebView 和临时目录位于 D 盘；系统创建的十个唯一测试 identifier 的
Local／Roaming 目录也按 owned 标记逐项核对后清理，正式客户端数据不在清理范围内。

复跑使用新输出目录，在 `tauri/` 执行：

```powershell
node scripts/native-perf-startup.mjs --exe D:\SoloSoul\build\rf121-theme-binding-20261010\ss-rf1129-diagnostic-runtime-20261011\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault100 --output D:\SoloSoul\build\ss-rf312-new-startup-100 --samples 5
node scripts/native-perf-startup.mjs --exe D:\SoloSoul\build\rf121-theme-binding-20261010\ss-rf1129-diagnostic-runtime-20261011\bin\solo_soul.exe --fixture D:\SoloSoul\build\rf312-maintenance-20261009\fixtures\vault5000 --output D:\SoloSoul\build\ss-rf312-new-startup-5000 --samples 5
```

执行前沿用本轮冻结输入和无 PDFIUM 覆盖的环境；结束时重新核验自有进程、保全证明并清理精确缓存。
[结构化结果](rf312-current-windows-paired-startup-2026-10-12.json)及
[238 份原件索引](rf312-current-windows-paired-startup-2026-10-12/index.json)
包含所有样本、原拒绝、来源／构建、独立复算、退出与缓存清理。压缩包逐成员回读长度和 SHA；
不发布 EXE、资源实体、Vault 或缓存树。RF-312 和 RF-121 的完整范围继续待验，本项不增加关闭数量。
