# RF-312：系统追踪二进制传输与离线分析

本项修复 Windows ADB 向 API31 Perfetto 传入配置和取回二进制文件的通道。五秒平台追踪组件已完成一次无错误／丢失标志的采集与分析；没有安装产品，也没有新增应用性能、原生应用帧或首次可交互样本。此前[协议修复](RF-312-system-trace-protocol.md)及各次拒绝记录保留。

## 通道根因与修复

`exec-out` 不转发 stdin；改用 Windows `shell -T` 输送全部字节又在实际往返中把 280 字节截成 49 字节，出现 `0x1a` 截断及换行变化。向 `/data/local/tmp` 同步配置虽逐字正确，Perfetto 服务仍无法读取；系统配置目录允许 shell 创建文件，却不允许创建子目录。不能通过规范化二进制、修改权限或扩大期限来改判这些失败。

[采集库](rf312-system-trace/transport.py)沿用已经实际验证的传输路径，并增加下述 CLI 完整结束校验：对固定目录下的随机平面文件先用远端 `set -C` 独占保留，成功后才取得所有权，再通过 ADB file sync 推送。280 字节全字节夹具及完整配置都经 `exec-out cat` 逐字读回；不经 Windows 文本 stdin。服务按配置的 `output_path` 独占创建 trace，命令省略 `-o`，避免客户端截断已有文件。随后比较远端 SHA-256 与本地 pull 文件，检查 1～32 MiB 上限、非空 TracePacket 外层和追踪前后零会话；回收范围仅为本轮证明拥有的文件。

源码依据为固定 [v15 配置协议](https://raw.githubusercontent.com/google/perfetto/v15.0/protos/perfetto/config/trace_config.proto)、[CLI](https://raw.githubusercontent.com/google/perfetto/v15.0/src/perfetto_cmd/perfetto_cmd.cc)和[服务实现](https://raw.githubusercontent.com/google/perfetto/v15.0/src/tracing/core/tracing_service_impl.cc)：服务的 `output_path` 使用 `O_EXCL`，不能同时传 CLI `--out`。Android 策略参考和实际目录标签／读写结果分别留证，不以参考源码代替设备验证。

[基础配置](rf312-system-trace/base-config.pbtxt)仍为 5,000 ms、8,192 KiB、原 RSS／进程／FrameTimeline 数据源及 500 ms 进程轮询；改用 `RING_BUFFER`，并指定 500 ms 写文件和 flush 周期，消除先前 DISCARD 与无周期 flush 的配置风险。不能把这些配置告警等同于已经观测到实际丢包。

## 准备与接入合同

从仓库根目录运行，目标父目录应已存在，目标目录必须全新；示例路径由执行者选择：

```powershell
python -B docs/performance/rf312-system-trace/prepare_inputs.py "D:\Checks\new trace inputs"
python -B -m unittest discover -s docs/performance/rf312-system-trace -p "test_*.py" -v
```

[准备入口](rf312-system-trace/prepare_inputs.py)验证三个固定源文件的 SHA-256，再生成独立目录、随机文件身份、配置、280 字节夹具及全部输入摘要。不会运行 SDK、准入设备、创建系统目录或安装程序。已有目标、源码篡改及缺失父目录均拒绝，失败目录不自动复用。把准备目录中的 `transport.py` 作为独立模块加载后调用 `run(call, out)`；`out` 必须是调用方独占的新输出目录，不能并发复用模块或目标目录。

`call(label, argv, timeout=...)` 必须由既有原生 owner 提供：

- 每次绑定已核验的 ADB 可执行文件、独立 server、设备身份及本轮冻结输入，不能自行选择默认设备。采集前须完成原有三片截图、完整健康记录及 600 秒准入；本库不替代这些检查，也不是独立原生启动器。
- `argv` 是追加在固定设备前缀后的参数列表；原样执行，不额外拼接本机 shell。`stdout`／`stderr` 返回原始 bytes；非零退出、超时或身份改变必须抛异常。不能将失败包装成成功返回值。
- query、保留／推送／读回、hash 和删除各保留原 5 秒期限，实际 trace 与 pull 各 45 秒，不自动重试。每条命令保留参数、期限、返回码及原始输出和摘要；stdin 必须为空。
- 超时调用 [preserve_timeout](rf312-system-trace/timeout_streams.py)保存**实际可取得**的部分 bytes；不可取得为 `null`，与已观测空输出区分。保存原异常并继续抛出。录制结果不明时不擅自删除未证明拥有的 trace，清理记录必须明确可能残留。
- owner 仍负责终态完整健康记录、九项设置前后一致、产品包状态、正常 SDK 退出和原 45 秒严格宿主归零。仅有 `trace-transport.json` 不构成完整原生或应用验收。

### CLI 零退出服务错误的清理保护

进一步审查发现，[v15 CLI 的 `OnTracingDisabled(error)`](https://raw.githubusercontent.com/google/perfetto/v15.0/src/perfetto_cmd/perfetto_cmd.cc)会在服务错误分支设置 `update_guardrail_state_` 后退出事件循环；[RateLimiter](https://raw.githubusercontent.com/google/perfetto/v15.0/src/perfetto_cmd/rate_limiter.cc)在该标志为真且不上传时返回成功。因此，返回 0 不能单独证明服务完成录制或输出文件归本轮所有。初版库在此处过早认领了文件。

当前库在认领 trace 之前要求原始 bytes、整数零返回码、空 stdout，以及有界、完整且恰好包含“连接五秒会话→写出结束”的 v15 两行日志。缺失、截断、重复、服务错误、未知额外日志或非原始输出均拒绝；不从混合错误里提取一条成功消息。失败时仅清理已独占保留的夹具／配置，不查询、拉取或删除未经证明的 trace。新增 `cliCompletionProtocolVerified` 只表示这个 CLI 协议检查，不提升 trace 内容或应用性能验收。其他 CLI 版本或额外正常日志也会保守拒绝，需单独核验后更新合同。

离线服务错误／文件碰撞模型对原提交重放，旧库错误接受并删除了模拟的非本轮文件；新库保留该文件、拒绝认领并明确记录可能残留。这是源码和离线故障回归，**不是新增实际设备错误样本**。74 项唯一测试及带空格副本相同 74 项通过，其中 10 项新增覆盖结束协议。原实际 13 条传输命令及成功原始日志仍被新库接受，原件和历史结果不改写，也不再计为一次原生采集。见[本次修复证据](rf312-system-trace-completion-2026-10-18.json)。

## 离线分析方式修订

第一次可取回的完整 trace 仍被零错误／丢失门槛拒绝：DISCARD 及缺少周期 flush 两项配置风险。修正配置后，默认窗口排序又报告 8 项 `sorter_push_event_out_of_order`，该原结果同样保留拒绝。

在不重录设备、不改变原始 trace／SQL／工具／门槛的条件下，增加显式 `--full-sort` 比较。[Perfetto 数据流说明](https://perfetto.dev/docs/concepts/buffers)解释了晚到事件超过排序窗口会被丢弃，以及完整排序会增加主机内存占用；[CLI](https://perfetto.dev/docs/reference/trace-processor-cli)和实际固定二进制帮助均确认支持该参数。本次分析方式据此独立标为 `offline-full-sort-r2`，不能追溯改判原默认排序成功，也不证明旧配置合理。

对**未经拼接、注入或时间戳改写的同一份原生文件**执行：

```powershell
& $Processor query --full-sort -f docs/performance/rf312-system-trace/analyze.sql $OriginalTrace
```

`$Processor` 由调用方提供，须先核验[固定方法合同](rf312-system-trace/analysis-method.json)中的官方 Windows amd64 v58.2 二进制 SHA-256、SQL 摘要、输入大小及 45 秒期限；不运行自动下载包装器、不使用默认 C 盘缓存。保存完整 stdout／stderr 和 trace 前后摘要。六个 SQL 结果集必须齐全，ftrace、RSS、进程内存计数和时钟记录各至少一条，**所有** `error`／`data_loss` 统计必须为零。不能过滤错误行或仅凭进程退出码接受。全量排序会缓冲整个文件，32 MiB 输入限制不等于主机内存峰值限制。

同一旧配置使用完整排序仍有两项配置风险并被拒绝，证明本次没有忽略质量错误。当前配置在完整排序下有 4 个 RSS 事件、8,804 个进程内存计数样本、78 条时钟记录，错误／丢失标志为零；FrameTimeline 从默认排序的 2 行恢复为 4 行。它们属于未装产品的系统／Launcher，不能接受为应用帧、旧 Java 两帧门槛或首个业务可用时间。

## 验证与后续

首版 64 项定向回归通过：原状态读取器 28 项、传输 21 项、超时保存 7 项、输入准备 8 项；后续 CLI 保护增加 10 项，当前合计 74 项。异地带空格副本执行同组测试，不重复计数。覆盖文件碰撞时保留旧数据、逐字读回／hash／外层拒绝、大小及原期限、配置篡改、未知所有权、正常和失败清理、真实主机子进程超时、不可用输出与空输出的区别。

实际本轮 80 条 SDK 命令、117 项 source 绑定、47 项冻结输入、远端所有权／本地原件、前后设置、38 条预检及 38 条终态 DropBox、完整健康记录及正常关机后宿主归零已独立回放。没有新增当前 ANR。完整排序的正例和旧配置反例逐字核验，原默认排序及所有前序失败不覆盖。程序、trace、截图和原始记录仅留本机；[公开证据](rf312-system-trace-transport-2026-10-18.json)只含通用版本、计数和摘要。

下一项将采集能力接入当前默认构建的 100／5,000 对象公开 Vault 完整行程，核验进程 PID／UID／出生时间、家族、时钟、丢失和事件归属，覆盖启动、解锁、首页／搜索、首次 OCR／预览及锁定恢复，补齐首次业务可交互和全进程内存峰值。RSS 不是 PSS，分进程 watermark 不能相加当成同一时刻峰值。原七次帧拒绝、五组文档标记、默认六对及 IPC 统计不变；368/377、九项未完成和 goal 保持。
