# RF-312 首批主窗口帧与启动计时边界

已有两份完整行程 trace 中，各自最早观察到的 20 个 MainActivity buffer 帧，都唯一关联期望 token 与 SurfaceFlinger 显示帧。PID／UID／进程出生与原十三次身份检查一致，设备 BOOTTIME 偏移为零；没有新增 SDK、输入或原生样本。

## 结果

| 对象数 | 进程出生→第一个已观察显示帧结束 | 进程出生→第二个已观察显示帧结束 |
| --- | ---: | ---: |
| 100 | 461.348 ms | 3884.909 ms |
| 5000 | 460.604 ms | 1062.109 ms |

这些是每档一次队列内的观察值，起点是进程出生，不能作为从启动请求开始的 TTID。两帧均早于原 startup 状态截图；截图证明稍后的登录内容，不能追溯证明前两帧已经显示该内容或可以交互。没有据此比较两档速度或公布启动性能分布。

[Perfetto 对 FrameTimeline 的说明](https://perfetto.dev/docs/data-sources/frametimeline)区分应用提交与 SurfaceFlinger 显示：应用 slice 的结束包含 GPU／提交工作，关联显示帧的结束才描述显示链路完成。因此本项分别保存 surface 结束与显示结束，拒绝无链接、多链接、未结束、非 buffer 或身份不符的候选。原解析器以 UPID＋surface token 关联期望帧；关联显示帧时使用显示 token 和实际 SurfaceFlinger 进程。

早期离线查询错误要求显示帧 `surface_frame_token=0`，两组各误报 20 个未匹配。固定 v58.2 [解析源码](https://raw.githubusercontent.com/google/perfetto/v58.2/src/trace_processor/importers/proto/frame_timeline_event_parser.cc)及实际结果证明该列为 NULL；修正查询后全部唯一匹配。两个查询及输出均保留，原 trace、采集器和读取器不改。公开查询另要求目标主进程唯一，其输出与已核验查询逐字相同。

## 明确缺失的输入

固定 v58.2 的[原 Android startup 模块](https://raw.githubusercontent.com/google/perfetto/v58.2/src/trace_processor/perfetto_sql/stdlib/android/startup/startups.sql)在两份原 trace 中均得到：总 activity startup 0、目标 startup 0、对应 activity 指示 slice 0。现有配置只包含进程／RSS 与 FrameTimeline，未声明 ActivityManager atrace。进程出生、全局 trace 起点和宿主请求时间均不能替代缺失的启动请求时钟；也不能跨时钟直接相减。

[Android 的启动定义](https://developer.android.com/topic/performance/issues/launch-time)区分首次显示与完整可交互。本项只验收已观察窗口帧的身份和显示链接，TTID、业务首帧、精确首次可交互及完整帧覆盖仍不提升。此前两组各两项 system_server 帧解析提示、RSS 提示、大规模一项服务丢弃及小规模一行未结束应用帧全部保留；最早“观察到”不等于证明绝对最早。

下一步先验证能否在独立采集配置中增加系统启动事件，并保持旧配置、旧数据和所有预算不变；在获得同一设备时钟下的真实启动请求之前，不用本项数字填补启动性能缺口。没有证据支持更改产品懒加载或分页。

## 复跑

先完成[原完整行程](RF-312-system-trace-journey.md)的会话、进程身份、恢复和原四组独立重放，再完成[原生控制时钟核验](RF-312-observer-timing.md)。使用[固定 v58.2 分析器](RF-312-system-trace-transport.md)、原输入摘要与 45 秒查询预算，分别读取同一份未经修改的 trace：

```powershell
& $TraceProcessor query --full-sort -f docs/performance/rf312-first-main-frames.sql $OriginalTrace
& $TraceProcessor query --full-sort -f docs/performance/rf312-startup-indicators.sql $OriginalTrace
```

查询输出仅是待审计数据：必须回验唯一目标身份、原 CLK_TCK 出生区间、BOOTTIME、全部链接和对应原生截图观察时钟，不能仅因 CLI 退出零就接受语义结论。出现不同结构或缺失项时保留原输出并拒绝；不补零或替换失败。

[去标识结果](rf312-first-main-frames-2026-10-18.json)只含相对区间、计数、方法和摘要。原路径、身份、CSV、日志与 trace 留在本地。RF-312 及台账 368/377、九项未完成保持。
