# RF-312：系统追踪能力核查

在五组启动标记重复完成后，继续核对原 R20 要求的首次可交互和内存峰值。新的 API31 专用设备已完成八项只读能力查询及独立回放，确认系统侧追踪有可继续验证的入口。本轮没有启动 trace、安装产品或修改私有数据，不是新的应用性能样本。

## 实际发现

环境为 API31／x86_64、2 核／2048 MiB／SwiftShader；内核 `5.10.136-android12-9`，Perfetto 服务版本 15.0。使用普通 shell UID 2000，没有请求 root。完整服务状态人工检查列出 17 项数据源登记、16 个唯一名称，包括：

- `android.surfaceflinger.frametimeline`；
- `linux.ftrace`、`linux.process_stats`；
- `android.gpu.memory`、`android.packages_list`。

`/sys/kernel/tracing/available_events` 实际返回 1434 个唯一事件，含 `kmem:rss_stat`、进程 fork／exit 和 task newtask／rename。未列出 `mm_event:mm_event_record`；不能假定所有 Android 设备提供相同事件。旧 `/sys/kernel/debug/tracing/available_events` 路径不存在，原退出 1 与 stderr 保留。`perfetto --help` 同样退出 1，但实际返回了用法；六项查询退出 0、两项退出 1，不写成八项命令全部零退出。

本轮 `perfetto --query` 只供人工审查，原文明确提示不是稳定机器接口；当时服务内总会话数、其中处于 started 状态的会话数均为零。后者不是累计启动次数，两个零值也不证明从未发生过追踪。[v15 协议定义](https://raw.githubusercontent.com/google/perfetto/v15.0/protos/perfetto/common/tracing_service_state.proto)。本轮未启动追踪的结论另由完整实际命令核验支持。后续自动入口应使用版本绑定的 `--query-raw` 协议，不把本轮文本提取规则当作通用准入读取器。

## 对下一步的影响

[FrameTimeline](https://perfetto.dev/docs/data-sources/frametimeline)提供系统侧帧时间线，可进一步核对启动期间的实际帧；它与[旧视觉栅栏后两次提交探针](RF-312-startup-frame-diagnostics.md)口径不同，数据源登记不能证明已采到帧，也不改判原七次拒绝。

[RSS 事件](https://perfetto.dev/docs/data-sources/memory-counters)可补充定时轮询容易漏掉的短时变化。后续仍须验证事件是否实际可启用、trace 是否丢包、进程出生／退出及 renderer 归属覆盖和时钟口径；RSS 事件不是 PSS，也不能将不同进程的高水位直接相加为同时总峰值。

[Android 启动指标](https://developer.android.com/topic/performance/issues/launch-time)区分首帧 TTID 与由应用上报可用状态的 TTFD。当前自有 `MainActivity` 与前端启动交接没有 `reportFullyDrawn` 上报；父类行为和真实业务就绪条件仍需核对，不能从系统首帧、DOM 启动标记或两个 rAF 推断首次可交互已验收。

因此下一项可在本地推进：准备固定版本的 trace 分析工具和有限配置，先验证实际数据包、丢失统计及时间／进程身份，再接入小／大公开 Vault 的完整行程。TTFD 另需明确业务可用条件。继续保留原读取器、预算、恢复保护和失败记录；当前不新增加载／分页优化。

## 核验与复查

75 条实际 SDK 命令及八项查询的 stdout／stderr SHA 已逐条回放，命令限定为只读能力／身份／设置／包缺席检查和既有 SDK 启动退出流程，没有安装、输入、设置写入或追踪启动。五个产品／观察器包前后均缺席，九项设置前后相同。三片当轮原图与完整窗口／lastanr／38 条 DropBox 已实际检查，终态完整健康记录仍为 38 条且无当前启动 ANR。SDK 正常退出，原 45 秒宿主期限内严格归零；宿主原件独立重放通过。

独立核验首轮把事件组名限制为无连字符，遇到真实 `xhci-hcd` 名称拒绝。原件和失败脚本保留，修正本机包装器后重放通过，没有重跑原生查询或修改既有读取器。117 份采集来源和全部冻结输入保持。

在已经核验归属的专用设备上，可手动复查以下命令；它们仅列出能力，不是完整的设备准备／清理或性能采集入口：

```powershell
& $adb -H 127.0.0.1 -P 5057 -s $ownedSerial shell perfetto --query
& $adb -H 127.0.0.1 -P 5057 -s $ownedSerial shell atrace --list_categories
& $adb -H 127.0.0.1 -P 5057 -s $ownedSerial shell cat /sys/kernel/tracing/available_events
```

[去标识证据](rf312-system-trace-capability-2026-10-18.json)仅发布通用版本、计数及 SHA。原始命令、路径、运行身份和截图留本机；本轮不改变五组标记、默认六对或 IPC 统计。RF-312、368/377 和九项未完成保持，goal 仍有本地可执行项。
