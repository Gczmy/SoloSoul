# RF-312 默认完整行程的系统追踪接入

本项把有界 Perfetto 会话接到现有 100／5,000 对象公开 Vault 的十二阶段行程。启动、主密码解锁、首次 OCR、首页、搜索、附件、图片／文本／两种 PDF、锁定和重新解锁仍由原有原生 UI、物理内容和状态读取器验收。产品源码及默认 APK 保持 `305b86020a33b5368299b07594b94542f0869e09`；没有向测量中的应用注入 JavaScript、instrumentation 或业务 IPC。

## 接入和保护

新增 `rf312-system-trace/trace_native_owner.py`、`trace_driver.py`、`trace_journey.py` 和 `trace_session.py`。原 SDK 所有者负责启动、健康检查、双规模顺序执行、原 APK／UID／私有数据／设置恢复及正常退出；会话库只负责当前 nonce 的采集。新增入口派生自原队列，原读取器没有修改。驱动异常／恢复尾部及原生所有者的正常关机代码块保持逐字一致，并由回归验证。

会话合同固定为 240 秒 TTL、8 MiB 环形缓冲、500 ms 进程采样／flush／文件写入、32 MiB 文件上限。先确认零会话，独占保留两个输入文件，逐字读回，再证明 nonce detach key 和一个活动会话。结束时只 attach／stop 本轮 key 一次，确认服务归零，核验远端／本地摘要和 packet envelope，最后仅删除三个已证明归属的文件。未知所有权、错误、超时、TTL 到期、异常日志或不完整清理均拒绝；不重试停止、不终止其他会话、不递归清理。SDK 小命令 5 秒，stop／pull 45 秒。

原有开机 240 秒、完整预检 600 秒、每阶段 45 秒、Observer 240 秒／512 请求、JavaScript 3 秒及正常宿主退出 45 秒预算保持。启动后的一个检查点及十二阶段结束各读取原始 stat／status／cmdline／stat，要求 PID、UID、出生 tick 一致。`CLK_TCK` 来自实际 `getconf`，不假设固定时钟频率。主进程名之后只允许有界 NUL 填充，拒绝额外参数和其他名称；[Android 12 的 `setArgv0`](https://android.googlesource.com/platform/frameworks/base/+/refs/tags/android-12.0.0_r1/core/jni/AndroidRuntime.cpp)说明该参数块会先清零。这是源码解释，不能替代实际进程身份检查。

`prepare_journey.py` 接受现有已声明 runtime、owned UI adapter 和 PowerShell，创建两组互不复用的 nonce 输入及冻结计划。冻结计划必须包含实际执行位置的模块和三个原入口；同名文件放到别处不能代替它们。输出目录必须全新，输入、SDK、AVD、源文件和 hash 继续由原合同验证。所有入口从仓库位置解析依赖，不含本机绝对路径。

## 使用

先按[原队列步骤](RF-312-public-native-queue.md)准备全新专用 AVD、同一默认 APK、公开夹具、runtime／owned UI adapter 及原 206＋17 项测试证据。以下变量均由调用方指向其已准备的声明；不要复用已经启动的 AVD：

```powershell
& $Python -B docs/performance/rf312-system-trace/prepare_journey.py `
  --runtime-inputs $RuntimeInputs --adapter-inputs $AdapterInputs `
  --powershell $PowerShell --output $NewTraceInputDirectory

& $Python -B docs/performance/rf312-system-trace/trace_native_owner.py `
  --kind default-state --preparation $AvdPreparation --output $NewNativeOutput `
  --adapter-inputs $AdapterInputs --sampling on --unit-evidence $UnitEvidence `
  --trace-plan $TracePlan --validate-only
```

`--validate-only` 只验证真实完整参数和冻结输入，不启动模拟器或 ADB；通过不等于设备可交互。正式执行移除该参数后，仍须按原队列对本轮三片实际预检图、完整 Window／Last ANR／DropBox 执行实际查看／读取并在 600 秒内准入。不能预填准入或使用上一设备的截图。原四个独立重放读取器继续用于输出核验；新增会话和时间／进程归属记录还须单独核验。

## 本轮证据与边界

路径尚在本机原型中的一次实际队列已完成两组各十二阶段：130 条外层原始 SDK 记录、两组各 69 条新增采集记录、78 项冻结输入、97 项原 source 绑定和 12 项原型源码均独立核验。两组会话分别在真实主进程出生前开始、十二阶段结束后停止，13 次身份检查一致；实际 `CLK_TCK=100`，trace 的 BOOTTIME 偏移为零，出生时间落入原 `/proc` tick 的精确量化区间。原 UI、六次键入、物理内容、内存和恢复读取器全部重放通过；没有补发输入或重写原数值。

预检 38 条、终态 107 条 DropBox 均保留，终态完整文本已读，没有 ANR／crash 标签；这不表示没有 warning。所有原始设置一致，原 APK／UID／私有 TAR、公开图片及探针恢复，正常关机后原 45 秒宿主归零检查通过。

公开代码与该原型的会话协议、行程和恢复实现，仅有声明的仓库路径／说明文字差异；冻结校验另加强为实际执行路径。116 项唯一测试和带空格异地副本的相同 116 项通过。公开会话代码还对两组原始 16 条会话命令及 52 条身份命令逐字离线重放，通过原读回／hash／协议／清理检查。公开完整入口在新的未启动 AVD 声明上通过 85 项冻结输入的真实参数准入；已启动的旧 AVD 被原保护拒绝。新 AVD 仅运行一次 avdmanager 准备，没有开机或发送 ADB 命令。**这些验证不另算一次公开入口的原生重跑。**

两份未经修改的 trace 分别为 11,817,142／11,969,562 字节。按[固定完整排序方法](RF-312-system-trace-transport.md)和原零 `error`／`data_loss` 门槛分析通过，但还存在语义限制：

- 两组各有 2 项帧解析提示；原始 packet 与[固定 v58.2 解析器](https://raw.githubusercontent.com/google/perfetto/v58.2/src/trace_processor/importers/proto/frame_timeline_event_parser.cc)逐项核对，均为 `system_server` 表面帧的 present enum，未归到应用主进程。应用主进程实际帧行分别为 569／526，其中小规模有一行未结束；显示帧 token 链接和完整业务窗口尚未验收，不据此公布卡顿率。
- RSS 负值计数分别为 29／3，缺失 mm 所有者计数为 512／1,824；[固定 RSS 解析器](https://raw.githubusercontent.com/google/perfetto/v58.2/src/trace_processor/importers/ftrace/rss_stat_tracker.cc)会丢弃这些不可归属或负值事件。大规模另有 1 项服务级 `traced_chunks_discarded`；服务统计未提供足够的本轮应用归属证据，不能把它忽略为无关事件。基础零严重度检查不能证明 RSS、家族或帧数据完整。
- 原独立内存读取器重放的最大**串行采样 PSS 和**为 316,201,984／306,184,192 字节。两组通用拒绝计数均为 2：小规模是 1 次定时失败＋1 个错过时隙，大规模是 2 个错过时隙；错过时隙包含在拒绝计数中，不能再相加。所有计划时隙与 24 个阶段边界均已核验，不填补失败或缺口。这不是连续峰值；RSS、PSS、各进程 watermark 不能互换或简单相加为同时峰值。互斥类别及后续归属核验见[计数与归属说明](RF-312-trace-attribution.md)。
- 首次密码框与 Unlock 的原生命中／稳定状态检查只给出包含观察成本的功能就绪上界，不能充当精确 TTI、TTFD 或应用独占耗时。全行程包括 Observer、周期内存采样及核验开销；单次双规模队列没有给出性能分布或因果比较。

前一轮在首次身份读取处因“单个 NUL”假设拒绝的 99 字节原件保持拒绝，两个启动 SDK 之前的入口失败及缺少终态完整 DropBox 的证据缺口也保留；修正为“精确名称＋有界 NUL”后才使用全新设备执行本轮。原七次 Java 帧拒绝、五组文档标记、默认六对和 IPC 方法均不改判、不混合统计。

本项不接受帧完整性、全进程连续峰值、首次业务可交互或完整 RF-312。下一步先核对孤立 renderer 家族、显示帧链接和上述缺口的影响范围，再推进原完整性能目标及观察开销／重复样本；368/377、九项未完成和 goal 保持。去标识计数和摘要见[公开证据](rf312-system-trace-journey-2026-10-18.json)，程序、原始日志、截图、trace 和本机路径不进入公开记录。


## 公开入口实际原生验证补充（2026-10-18）

在公开源码 `92fa0faa2` 上，使用已经通过准入、此前从未启动的专用 AVD，实际执行仓库内入口一次完整 100／5,000 顺序队列。两组各十二阶段、十三次 PID／UID／出生时间、两份 trace 传输及 APK／UID／私有数据／设置恢复均通过；正常退出后，原 45 秒宿主检查确认本轮 SDK 进程和监听归零。此前导出阶段的离线／准入证据仍按其原范围保留，本补充才是公开入口的第一次实际原生执行。

130 条外层原始 SDK 记录、85 项冻结输入、19 个公开模块、97 项原 reader 绑定及两组各 69 条新增采集记录独立核验通过。原四组独立重放通过 24 个界面阶段、357 次状态检查、六次键入、12 份物理内容证明及完整恢复；没有补发输入或改变原数值。三片本轮预检实际截图及完整健康记录已查看／读取，DropBox 38→93 条保留，无 ANR／crash 标签。此前 116 项唯一回归所对应的 Python 源码没有变化，本次不把重放计为重新运行这些测试。

本轮两份 trace 为 11,683,070／11,694,331 字节，基础零严重度分析通过；各 2 项帧解析提示仍落在 system_server，RSS 负值为 24／0、未知 mm 为 731／101，大规模仍有 1 项服务级丢弃，全部保留。有效数值内存点 83／82、有效启动前缺席证明各 1；小规模 1 次定时失败＋1 个错过时隙，大规模 1 个错过时隙，互斥总记录为 86／84。最大串行采样 PSS 和为 283,899,904／297,139,200 字节；不与前一队列合并为分布，不转为连续峰值。

本补充接受公开入口的完整功能执行与恢复，尚不接受服务统计的完整时间覆盖、帧完整性、精确业务可交互、应用独占耗时、性能分布或完整 RF-312。继续核查原周期 TraceStats 的覆盖范围及业务窗口，保留所有旧失败和门槛。见[本次去标识摘要](rf312-public-trace-entry-2026-10-18.json)。
