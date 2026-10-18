# RF-312：Android 文档内启动标记

本模式只读取默认 APK 已有的启动标记和浏览器时间条目，补充启动测量；不替代[仍未通过的原生两帧诊断](RF-312-startup-frame-diagnostics.md)。100／5000 对象各一次真实采集、恢复和独立重放通过，尚未形成重复分布或性能优化结论。

目标 APK 的产品源码仍为 `305b86020a33b5368299b07594b94542f0869e09`，默认功能、非 debuggable、内部测试签名；没有更改产品标记或业务代码。目标进程注入了非默认采集器并执行只读 JavaScript，观察开销没有扣除。这不是默认无插桩性能、完整进程启动、首次可交互、物理呈现或跨平台验收。

## 时钟与结束条件

- `solosoul:react-mount`：调用 React 挂载之前，已有初始化阶段之后的标记，见 [`bootstrapApp.tsx`](../../tauri/src/bootstrapApp.tsx)。
- `solosoul:startup-dismissed`：应用既有双 rAF 交接中，启动覆盖层设置 ready／aria-hidden 的时刻；覆盖层节点稍后移除，见 [`startupScreen.ts`](../../tauri/src/lib/startupScreen.ts)。两个 rAF 不证明两次原生提交，也不证明屏幕已呈现。
- `first-paint`／`first-contentful-paint`：浏览器缓冲区的原始时间条目，可能对应品牌启动层，不能称为登录卡片首屏。
- 导航条目：只记录五个数值字段，不读取 URL、条目 detail、资源名称或账户内容。`domInteractive` 不等于 React 页面可交互；导航事件的原始零值不改写成已完成事件。

标记的 `startTime` 与 `performance.now()` 使用同一文档时间原点。文档身份和时间原点须在前后两次可见、可命中的登录状态之间一致；原生 `System.nanoTime` 单独验证原 45 秒窗口，两个时钟不换算、不相减。[User Timing](https://www.w3.org/TR/user-timing/)、[Paint Timing](https://www.w3.org/TR/paint-timing/)。

采集不创建／清除标记，不调用定时绘制、强制重绘、焦点或业务 IPC。只接受固定名称的唯一条目；重复条目拒绝，不选择较有利的一项。缺少可选绘制／导航条目时保留 null；必需启动标记缺失时保留原始可用性结果，并拒绝将该档列为标记成功。

## 本次观察

下列值为文档内毫秒值，显示到一位小数；原数值保存在结构化证据中。每档 n=1，不生成中位数、p95、两档性能比较或瓶颈结论。全新模拟器中按 100→5000 顺序启动独立应用进程，未清空 OS／存储缓存。

| 对象数 | React 挂载调用标记 | 启动层交接标记 | 两标记间隔 | 首次内容绘制条目 |
| --- | ---: | ---: | ---: | ---: |
| 100 | 1182.0 | 1968.2 | 786.2 | 381.1 |
| 5000 | 892.8 | 1642.0 | 749.2 | 742.6 |

两档 API 可用性实际读取，必需标记均唯一，首绘／导航条目均存在。独立复核 34 份原生状态、94 份完整控制回复、40 份只读 UI 回复及四张 PNG；没有控制拒绝尾部。四张实际登录截图已查看：100 档两张字节相同，5000 档两张存在像素差异，未将其记作相同或连续稳定帧。

每档及外层均核验恢复 99 项私有内容／链接／权限，原 APK／UID／系统设置和公开夹具恢复；SDK 正常退出，原 45 秒宿主期限内严格进程／端口归零，回执独立重放通过。预检逐片查看原图并读取完整窗口／lastanr／38 条 DropBox；终态完整 DropBox 为 59 条，lastanr 未见 ANR，原警告保留。首个独立复核脚本汇总变量重名失败保留，修正包装器后重放通过，没有重跑应用。

## 复跑

使用[同一显式生产者与准备流程](RF-312-frame-runtime.md)。当前目录共 22 份源码，运行／构建分别绑定 117／126 份来源。公开目录实际十三步构建的 APK 与本次原生测试包逐字一致；68 项 Python、11 项纯 Java、9 项原就绪和 10 项标记脚本检查通过，零跳过。

构建配置仍使用 `build_frame_collector.source_hashes()`，运行配置字段仍按 `frame_binding.FIELDS`，但 `scope` 必须明确为 `explicit-default-apk-startup-milestones`。使用本次完整源码字节、同源默认 APK、采集器回执、实际工具和未启动的全新专用 AVD 生成配置，不能复用旧的来源字典或已消费设备。

```powershell
python -B "$frameSource/build_frame_collector.py" --config $builderConfig --output $newCollectorOutput
python -B "$frameSource/frame_owner.py" --kind startup-milestones `
  --preparation $freshAvdPreparation --output $newNativeOutput `
  --diagnostic-inputs $milestoneConfig --powershell $actualPowerShell `
  --powershell-sha256 $actualPowerShellSha256
```

本模式不能同时启用 `--window-frame-trace`。仍须在原 600 秒内实际检查本轮三片原图和完整健康输出后放行；原身份、原生窗口焦点／硬件绘制／几何、独立 UI／命中、备份恢复、SDK 关闭及原 45 秒阶段／3 秒 JS／5 秒状态 SDK／240 秒 512 请求观察器／45 秒宿主期限不变。

读取 `target-instrumentation/ready.json`、`startup-milestones.json`、原 SHA 标记、`metadata-contract.json`、独立 UI 证明及各恢复／宿主回执。ready、结果、终态和拒绝采用独立标记命名空间；旧两帧读取器拒绝这些记录。采集完成可以为 true，但 `nativeFramesAccepted`、`firstInteractiveTimeProven`、`applicationOnlyLatencyAccepted`、`defaultPerformanceSampleAccepted` 和 `fullRF312Accepted` 均为 false。

## 继续范围

保留七次原生帧拒绝、默认六对样本和当前 IPC 口径，原读取器及回调顺序不变。本模式并未验证新包的原生两帧分支。后续补重复样本与同源观察开销对照，继续首次可交互、应用本体、全部归属进程内存及物理 ARM／多端缺口；不凭这两次采集新增优化 ID。368/377、九项未完成及 goal 保持。

见[去标识证据](rf312-startup-milestones-2026-10-18.json)。原始时间原点、运行标识、路径、截图、APK、系统回复和私有备份只留本地。


重复采样先遵循[五组预声明及失败保留规则](RF-312-milestone-cohort.md)。本页两次探索观察单独保留，不填入该队列。
