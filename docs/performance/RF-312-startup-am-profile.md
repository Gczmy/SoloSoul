# RF-312：显式系统启动追踪配置

本项为完整公开 Vault 行程增加 `startup-am` 配置，采集真实的系统启动请求，并修复生产者延迟就绪和 Android 12 空应用列表的兼容问题。默认配置仍使用 schema 1，既有配置字节和 16 条会话命令保持；新配置显式使用 schema 2，不能仅通过改声明或刷新摘要互换两种配置。

## 就绪与应用过滤检查

[v15 CLI](https://raw.githubusercontent.com/google/perfetto/v15.0/src/perfetto_cmd/perfetto_cmd.cc)在请求启用后即可 detach，返回成功不足以证明生产者已经就绪。实际新设备读到两次 0 后才出现 AM 标志 64。入口在同一 5 秒总期限内进行最多 51 次只读查询，每次使用剩余期限，保留尚未就绪及读取中变化的回复；仍须真正读到稳定的 64，不延长原 UI 或采集预算。

[Android 12 atrace](https://android.googlesource.com/platform/frameworks/native/+/refs/tags/android-12.0.0_r1/cmds/atrace/atrace.cpp)会把空应用列表写成一个空条目；[同版本 libcutils](https://android.googlesource.com/platform/system/core/+/refs/tags/android-12.0.0_r1/libcutils/trace-dev.inc)按条目匹配进程命令名。因此不能只凭计数 1 判断启用了某个应用。入口在一条有界命令中读取标志、计数、首条目、计数、标志。活动态只接受稳定的 64、计数 0 或 1 且条目实际为空；命名应用、通配符、额外条目、未知标志及不完整回复仍拒绝。前后计数及标志必须归零。回复保留真实计数及摘要，不把 1 改写成 0，不发送 `setprop` 或应用通配符配置。

## 复跑

按[完整行程说明](RF-312-system-trace-journey.md)准备声明、默认 APK、公开夹具和全新专用 AVD；在准备命令中显式增加配置：

```powershell
python -B docs/performance/rf312-system-trace/prepare_journey.py `
  --runtime-inputs $RuntimeInputs --adapter-inputs $AdapterInputs `
  --powershell $PowerShell --output $NewTraceInputs --profile startup-am
```

随后按原入口执行真实 `--validate-only`，再正式运行；本轮三片实际预检图、完整健康日志、原配对探针和原四组重放仍是必要步骤。240 秒启动／Observer、600 秒实际预检、45 秒阶段／正常宿主退出、512 请求、3 秒 JavaScript、小 SDK 5 秒、stop／pull 45 秒及原会话归属／恢复保护保持。TTL 240 秒、8 MiB 缓冲、32 MiB 文件上限及 500 ms 周期保持。

SDK 正常退出后，可用固定 v58.2 处理器对未修改的 trace 执行 `query --full-sort -f`，分别运行[原质量 SQL](rf312-system-trace/analyze.sql)、[启动事件 SQL](rf312-startup-events.sql)和[主窗口帧 SQL](rf312-first-main-frames.sql)。查询须绑定原输入摘要、45 秒期限、实际 PID／UID／出生时间及 BOOTTIME 时钟；通用查询输出不能单独代替这些归属检查。

## 实测结果与边界

140 项唯一回归及异地副本的相同 140 项通过，包含默认配置金样、schema 混用／摘要重写拒绝、延迟和超时、空条目、命名／通配符拒绝、读取中变化及原清理门槛。正式运行一台全新 API31 x86_64 隔离设备，两档各十二阶段、十三次身份、原四组独立重放、原 APK／数据／设置恢复及正常退出后的严格进程／监听归零均通过。129 条外层记录、85 项冻结输入、19 个新增绑定源码及 97 项原读取器绑定独立核验；两档新增记录为 74／72 条，保留 2／0 次未就绪回复。

两份 trace 为 11,604,870／11,431,833 字节。固定质量检查的非零 `error`／`data_loss` 均为零；各 2 项帧解析提示、RSS 未知 mm 170／1,437、负值 0／25 和服务级丢弃 0／1 保留。本轮帧提示未逐包归属，不套用旧队列的归因，也不接受全帧或全内存完整性。实际预检和终态完整 DropBox 38→107 条已读，没有 ANR／crash 标签，warning 保留。

固定启动模块为 SoloSoul 各解析出一个 `cold` 窗口，系统时长分别为 414,148,520／549,114,365 ns。请求、成功通知、模块进程映射、十三次实际身份及出生量化区间匹配；两组最早各二十个主窗口 buffer 帧均有唯一 expected／显示链接，BOOTTIME 偏移为零。前两个已观察显示帧结束距请求分别为 432,203,817／3,723,483,887 ns 和 574,399,530／1,097,436,390 ns。这是各 n=1 的系统窗口和已观察帧，帧内容、登录卡片可交互时刻、应用独占耗时及性能分布仍未证明；每组另外两条未归属 Intent 失败通知也保留。

内存互斥记录为：100 对象 84 个数值点＋1 次有效启动前缺席证明＋1 个错过时隙；5,000 对象为 82＋1＋1，均无失败定时采集。错过时隙已经包含在各自拒绝计数 1 中，不重复相加。最大串行采样 PSS 和为 269,769,728／305,081,344 字节，不是连续峰值。

此前立即读取到 0 的失败、随后读到 64／计数 1 但未证明条目内容的失败，以及 SDK 启动前的受限 CIM 拒绝均保留；后续成功不追认旧失败。新配置与默认六对、旧五组文档标记、七次帧拒绝及既有 IPC 方法分别统计。台账保持 369/378、九项未完成和原目标；下一步继续绑定业务就绪窗口、观察成本与多端条件。见[去标识证据](rf312-startup-am-profile-2026-10-18.json)，原始日志、路径、运行标识、截图、程序和 trace 留在本地。
