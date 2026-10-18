# RF-312 已记录行程的进程与帧归属核验

本项只分析[完整行程接入](RF-312-system-trace-journey.md)已经保留的两份原始 trace，没有启动 SDK、重新操作 UI 或增加原生样本。固定 v58.2、完整排序、45 秒单次离线期限及原始文件前后摘要检查保持。它补齐已观察进程／帧的对应关系，尚不接受完整性能结果。

## 采样计数解释修正

原读取器的 `refusedPoints` 包括 `missed-periodic`。此前说明分别列出“拒绝点”和“错过时隙”，容易被理解为可相加；现在明确为下表的互斥类别。原数值和读取器均不改。

| 公开对象数 | 有效数值内存点 | 有效应用启动前未运行证明 | 定时采集失败 | 错过时隙 | 总记录 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 | 84 | 1 | 1 | 1 | 87 |
| 5,000 | 85 | 1 | 0 | 2 | 88 |

两组 `refusedPoints=2` 分别是 `1+1` 和 `0+2`，与错过时隙不能再相加。应用启动前 `not-running` 是通过原生 UID／进程检查的有效缺席证据，不能计为数值 PSS 点或采集失败。24 个阶段边界均在有效数值点中；缺口没有补值，最大串行采样 PSS 和保持原结果，也没有转为连续峰值。

## 已观察到的归属

从原独立读取器已经重放的 84／85 个完整数值快照提取实际身份，每组均只有一个应用主进程和一个经 ActivityManager／服务归属绑定的隔离 WebView renderer。两组 renderer 使用不同隔离 UID；本次使用 PID、UID、完整进程名和出生 tick 联合核对，未按包名或一个固定 UID 扩大范围。

两组的两个进程都在 trace 中有唯一对应，出生时间落入实际 `CLK_TCK=100` 的同一 BOOTTIME 量化区间，名称与命令行一致。每组关联的 150 条线程记录均属于这两个 trace UPID。该结果只证明这些已记录身份之间的对应；采样间隙内是否存在其他短命进程尚未被证明。

| 公开对象数 | 实际应用帧行 | 唯一显示帧关联 | 唯一期望 token 关联 | 未结束应用帧 |
| --- | ---: | ---: | ---: | ---: |
| 100 | 569 | 569 | 569 | 1 |
| 5,000 | 526 | 526 | 526 | 0 |

显示帧按 `display_frame_token` 关联到 SurfaceFlinger；关联均无缺失或歧义。期望帧关联遵循[固定解析器](https://raw.githubusercontent.com/google/perfetto/v58.2/src/trace_processor/importers/proto/frame_timeline_event_parser.cc)的去重方式：同一 UPID 的 surface token 共用一条期望记录，跨层可复用该 token。初次按层名称额外限制的查询误报 7／10 行无期望对应；纠正关联键后，全部实际帧均有唯一对应。两种查询和原始输出都保留，没有删除实际帧、修改 parser 或归零错误统计。

WebView renderer 没有独立的应用表面帧行；已记录表面归在主进程的 MainActivity／PdfPreviewActivity。本次未把 system_server 的过渡动画、系统栏、输入法或文件选择器帧计入应用帧。应用帧与业务阶段的精确时间窗口、未结束帧的完整终点和全量帧覆盖仍待验证，因此没有公布卡顿率，也没有改判原七次 Java 帧拒绝。

## RSS 与服务丢弃限制

逐行读取 210,950／210,105 条已规范化 RSS 事件并核对原始负值、`curr`、`mm_id`、线程和进程字段。按[固定 RSS 读取器](https://raw.githubusercontent.com/google/perfetto/v58.2/src/trace_processor/importers/ftrace/rss_stat_tracker.cc)构造的缺失 mm 映射模型，重现了 512／1,824 的全局未知所有者计数；加入已结束线程检查后结果一致。负值仍为 29／3。模型未完整重放[进程生命周期／替换逻辑](https://raw.githubusercontent.com/google/perfetto/v58.2/src/trace_processor/importers/common/process_tracker.cc)，这些计数没有被移除或改判无关。

大规模有 10 条未知所有者事件的 mm_id 后来被观察为应用 mm；它们全部早于应用真实出生区间。后来相同的 mm_id 不能证明过去属于该应用，也不能据此忽略其他未知事件。这次复核没有证明全量 RSS 或 PSS 完整。

大规模服务级 1 项 `traced_chunks_discarded` 的具体 producer／时间／业务影响范围仍无足够证据。已有零 `error`／`data_loss` 基础门槛通过，不会覆盖该限制。RSS、串行 PSS、单进程 watermark 和全家族同一时刻峰值继续分开解释。

见[去标识核验摘要](rf312-trace-attribution-2026-10-18.json)。下一步继续检查服务丢弃范围、未结束帧、业务时间窗口和完整内存／观察开销／重复样本；368/377、九项未完成及 goal 保持，完整 RF-312 未通过。
