# RF-312：启动标记重复队列的预声明

此协议在新增设备与原生采样开始前固定。此前 [100／5000 各一次的探索结果](RF-312-startup-milestones.md)单独保留，不进入本次重复统计。协议和统计器通过提交公开之后，才允许开始新队列；完整 RF-312、原生两帧、首次可交互与观察开销继续待验。

## 固定采样规则

见[协议 JSON](rf312-startup-milestone-cohort-protocol-2026-10-18.json)。采集器仍为同一默认目标 APK、同一诊断 APK 和同一 117 份采集源码；协议分别绑定这些 SHA-256 及统计器源码。源码字典按 `json.dumps(sources, sort_keys=True, separators=(',', ':'))` 的 UTF-8 字节计算摘要。

1. 预留五个唯一、未启动的专用 AVD，按组号 1→5 串行；每组使用同一[现有入口](RF-312-frame-runtime.md)运行 `--kind startup-milestones`，内部顺序仍为 100→5000。
2. 每组都实际检查本轮三片原图和完整窗口／lastanr／DropBox，原 600 秒审查预算保持。原 45 秒阶段／3 秒 JS／5 秒状态 SDK／240 秒 512 请求观察器／45 秒宿主退出预算不变；测量及退出期间不并发构建或重审。
3. 只有两档、原生身份／窗口焦点／UI 与命中证明、完整私有内容／APK／UID／设置恢复及严格宿主归零全部独立复核通过，才接纳整组。任何失败立即停止后续组；原拒绝、有效前缀和恢复记录保留，有效前缀不混入成功分布。
4. 失败组不补跑、不拿探索结果填缺口；未运行组保留为 `not-started`。变更源码、制品、时间口径或预算需要新的预声明，不能回写本组规则。
5. 指标仍为文档内相对毫秒：挂载调用、启动层交接、两标记间隔，以及可用的首次绘制／首次内容绘制条目。可选条目缺失为 null，不补零；导航事件原始零值不用于推断已完成或首次可交互。

每档计划 n=5；中位数和最近秩 p95 分别为排序后的第三、第五项。因此 n=5 时 p95 就是最大值，应连同样本量解读。缺失绘制条目会使该指标的有效 n 减少，逐指标报告数量和完整性。有效 n<2 时不生成中位数或 p95。统计队列完整只表示五组均被接纳，不能把缺失指标或平台／性能验收自动提升为完整。

## 汇总格式与复跑

[`rf312_milestone_statistics.py`](rf312_milestone_statistics.py)只计算已经独立复核的数据，不启动 SDK，也不替代原生、恢复或来源验证。输入精确字段为 `schemaVersion=1`、`scope=rf312-document-startup-milestone-observations`、实际协议字节的 `protocolSha256` 和五项 `queues`。

每组包含 `slot`、`status`（`accepted`／`refused`／`not-started`）、`reason`、`evidenceSha256`、`samplerSourcesSha256` 与 `samples`。接纳组必须有同源摘要和完整两档；每档包含 `objectCount`、原生元数据的 `metadataSha256`、五个固定指标的 `metrics`。拒绝组只保留固定原因码及证据摘要，统计样本为空；未启动组的原因／摘要为 null、样本为空。原始前缀与拒绝另存采集归档，不能因统计未接纳而删除。

```powershell
python -B docs/performance/rf312_milestone_statistics.py `
  --protocol docs/performance/rf312-startup-milestone-cohort-protocol-2026-10-18.json `
  --input $independentlyCheckedObservations --output $newSummary
```

输出保留五组状态、拒绝及未启动数量；每个指标分别给出有效 n、缺失 n、中位数、p95 和完整性。程序校验精确字段、顺序、同源摘要、固定协议、失败后的停止规则和同一文档标记间隔；不因统计计算完成而声称原生执行、默认性能、应用本体、首次可交互或物理呈现已通过。

15 项定向回归覆盖固定样本的中位／尾部值、单样本限制、缺失指标、失败保留、禁止补样／重排、来源漂移、跨时钟间隔、非法数值及大数中位数溢出。当前只完成协议与统计器；尚未取得本组重复原生样本，不改变探索记录、默认六对或七次原生帧拒绝。
