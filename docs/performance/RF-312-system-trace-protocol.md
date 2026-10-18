# RF-312：追踪服务协议读取器

本项修复系统追踪准备阶段的真实编码误拒绝。新的只读离线入口能解析两次实际 API31 `--query-raw` 回复；两次原生采集尝试仍为失败，尚无完整 trace 或新增应用性能样本。

## 根因与修复

首轮本机原型要求所有 varint 都使用最短编码，遇到真实回复的嵌套消息长度后拒绝。[ProtoZero 设计](https://perfetto.dev/docs/design-docs/protozero)明确使用预留的四字节长度再回填；这种冗余长度不是截断或损坏。

[读取器](rf312-system-trace/service_state.py)仅为长度字段增加四字节冗余编码支持。标签和标量仍须最短编码，原有 64 位溢出、截断、字段类型、256 KiB／4096 字段上限、重复字段、producer 身份和数据源归属检查保留。不新增依赖，不调用 SDK，不改变原帧／性能读取器。

读取器限定于 [v15 服务状态协议](https://raw.githubusercontent.com/google/perfetto/v15.0/protos/perfetto/common/tracing_service_state.proto)及当前验证的字段布局，不是通用 protobuf 解析器，也不自动准入设备。两个会话计数必须实际存在且唯一；总会话数和当前 started 数均为零只说明查询时状态，不能证明历史上没有追踪。启动采集前仍需调用方检查零占用、设备与进程归属及原有准备／清理条件。

## 验证与复查

[28 项回归](rf312-system-trace/test_service_state.py)覆盖四字节长度、冗余标签／标量拒绝、溢出／截断、重复或缺失计数、缺失数据源、无归属数据源、输入上限及优化模式 CLI 拒绝。相同 28 项在带空格的异地副本再次通过；它们不是 56 个独立案例。公开 CLI 对两台实际设备保留的两份 825 字节回复重放，与本机审计结果一致：4 个 producer、17 行／16 个唯一数据源、查询时总会话和当前 started 均为零。两份原始回复字节相同，不据此形成统计分布。

从仓库根目录运行以下命令；`query.raw` 应是此前在已核验设备上保留的二进制回复，不使用人类可读 `--query` 文本：

```powershell
python -B docs/performance/rf312-system-trace/test_service_state.py -v
python -B docs/performance/rf312-system-trace/service_state.py "D:\Checks\query.raw"
```

CLI 仅输出解码结果，缺失、含糊或越界输入非零退出；`python -O` 也保留校验。路径和原始回复无需进入仓库。[去标识证据](rf312-system-trace-protocol-2026-10-18.json)记录来源、计数和摘要。

## 两次原生尝试的原始结果

首轮在录制命令之前因长度误拒绝结束；64 条完成命令的输出、原失败及冻结输入独立回放。修正版在新的隔离设备通过实际服务回复解码，随后五秒追踪命令触及原 45 秒 SDK 期限。60 条已完成命令的输出与摘要通过回放，超时命令的 traceback 保留；该命令没有保存部分 stdout／stderr，这个证据缺口如实保留，不补成空输出，也不声称录制是否实际开始。

已核对 [ADB 客户端源码](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/commandline.cpp)：`exec-out` 分支只读取远端输出，并不转发本地 stdin。本机原型使用该通道向 `perfetto -c -` 送配置，因此需要另行修复传输通道和超时输出保存。本次只发布服务状态读取器，不发布这个未通过的录制入口，不延长期限或盲目重试。

两轮均实际检查当轮三片原图、完整窗口／lastanr 和 38 条 DropBox，五个产品／观察器包初始缺席，没有安装或访问私有数据。两轮九项设置前后相同，SDK 正常退出、原 45 秒内严格宿主归零独立回放通过，全部冻结输入保持。由于提前失败，没有采到预定终态健康记录，不能以预检替代终态验收。

## 后续与验收边界

下一项先改用能传入配置并保留原始二进制输出的通道，完善超时原件保存，再核验实际数据包、丢失统计和时间／进程身份。通过后仍需接入当前默认构建、小／大公开 Vault 的完整启动、解锁、首页／搜索、首次 OCR／预览和锁定恢复行程，补齐全进程峰值与首次可交互条件；RSS 不是 PSS，系统首帧不是业务可用。

本项未运行真实 trace 的主机分析，不接受原生帧、首次可交互、应用本体延迟、默认性能或完整 RF-312。旧七次帧拒绝、五组文档标记、默认六对和 IPC 统计不变；368/377、九项未完成和父 goal 保持。
