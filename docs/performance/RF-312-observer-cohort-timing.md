# RF-312 默认六对的原生控制耗时分布

本项复用已完成的[默认六对队列](RF-312-sampling-cohort.md)，重新按原状态、IME、UI 读取器核验九个完整模式，再用[原生控制时钟分析入口](RF-312-observer-timing.md)派生每轮区间。没有新增设备采集、SDK 或输入；三个原拒绝模式不补跑、不以成功前缀进入分布，亦不混入后续系统追踪队列。

## 已有数据的重复统计

四格各计划六轮，实际关闭每档 n=5、开启每档 n=4。表中“方法总和”是每轮 Observer 回包覆盖的原生方法经过时间之和，不包含其他控制通道未计时的部分；“通道跨度”从首请求开始到末请求完成。单位为秒，p95 使用最近秩，对这里的 n=4/5 等于最大值。

| 周期内存采样 | 对象数 | 完整模式样本 | 方法总和中位数 | 方法总和经验 p95 | 通道跨度中位数 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 关闭 | 100 | 5/6 | 33.554 | 35.200 | 123.129 |
| 关闭 | 5000 | 5/6 | 29.346 | 30.036 | 120.785 |
| 开启 | 100 | 4/6 | 36.236 | 39.620 | 135.411 |
| 开启 | 5000 | 4/6 | 35.675 | 38.414 | 137.063 |

九个完整模式共核验 7,154 个原始请求／回复、十八档完整行程、216 个原阶段观察窗口。逐 action 的请求数、区间总和／中位／p95、原 XML 相同的截图配对与负差值、每轮收据摘要均保留于[结构化结果](rf312-observer-cohort-timing-2026-10-18.json)。中位数以精确分子／分母保存，纳秒计算不先转浮点。

第 2 对开启模式在预检拒绝；第 4 对开／关均在原状态读取预算内拒绝。三个模式仍按原分类及案例前缀保留，耗时项为 null，不填零、不补齐完整 100 档、不替换失败。完整配对只有四对，其 on−off 差值逐项保留，只作描述；顺序按原六对平衡，不能证明宿主负载、就绪等待或采样成本的因果关系。

## 可用结论和限制

这些重复数据描述控制通道的耗时与波动，补齐了观察方法的成本证据。原生方法包含层级读取、输入派发、截图／编码和应用等待；通道跨度还包含宿主通信、校验与其他间隔。两者均不是应用独占耗时，不能从原行程总时间减去方法总和或配对差来制造应用延迟。

按轮统计与单轮请求分布分别保存。这里接受原生控制区间的描述分布，原应用性能分布、精确首次可交互、帧完整性和完整 RF-312 标志保持 false。原宿主行程上界、后置／轨迹内存、周期失败和错过时隙继续使用各自既有统计，不改变口径或重算为连续峰值。

当前证据显示观测流程本身含可观耗时，仍不足以把延迟归因于产品的路由加载、查询或分页；保持现有加载和首批 50 卡片策略。后续应以系统启动事件和原生内容就绪的真实设备时钟窗口补齐应用测量，再决定是否优化；避免继续用控制总窗口替代业务指标。

## 复跑

使用原六对的完整输出、对应 adapter、同源 206＋17 项回归证据和 Python 3.12。按原统计输入的模式状态选择九个 accepted 模式，每次给[已有 CLI](rf312-observer-timing/read-native-timing.py)全新输出；拒绝模式保持缺失。

```powershell
$cohort = Get-Content docs/performance/rf312-sampling-cohort-input-2026-10-18.json -Raw | ConvertFrom-Json
foreach ($pair in $cohort.pairs) {
  foreach ($mode in $pair.modes) {
    if ($mode.status -ne 'accepted') { continue }
    $name = 'pair-{0:d2}-{1}' -f [int]$pair.pair, $mode.mode
    $source = Join-Path $OriginalCohortRoot $name
    & $Python -B docs/performance/rf312-observer-timing/read-native-timing.py `
      --native (Join-Path $source 'native sample') `
      --adapter-inputs (Join-Path $source 'adapter-inputs.json') `
      --unit-evidence $OriginalUnitEvidence --output (Join-Path $NewOutput $name)
    if ($LASTEXITCODE -ne 0) { throw '保留原拒绝，停止本批重放' }
  }
}
```

每轮从输出的两份 `timing-*.json` 提取 `nativeOperationDurationSumNs`／`nativeChannelSpanNs`，按原 pair 1～6 放入向量，拒绝模式为 null；对非空值调用同一已验证 `native_timing.statistics()` 即可复算表格。禁止混合单个请求的分位数、失败前缀、不同来源或另一次追踪队列。统计入口与原读取器源码没有变化，不重复计数上一项 15 个唯一回归；本项完成的是九组实际独立重放与派生核验。

程序、原始回包、日志和本机身份留本地，公开记录只含通用计数、区间和摘要。RF-312、台账 368/377、九项未完成及 goal 保持，后续继续真实应用窗口及多端缺口。
