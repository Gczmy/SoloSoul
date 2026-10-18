# RF-312：显式 Android 启动帧诊断入口

此入口提供原生帧诊断及独立的[文档启动标记模式](RF-312-startup-milestones.md)。原生两帧尚未通过；标记模式已完成两档采集及恢复，验收口径分别记录。它向默认 APK 的目标进程注入采集器并执行只读 JavaScript，不能作为默认性能样本、应用本体耗时或物理显示呈现证明。没有登录、输入密码、主动请求焦点、强制重绘或业务 IPC。

## 注册顺序与计数

Android 12 的参考实现先派发预绘制，再提取本帧提交回调，随后派发 `OnDraw`；在 `OnDraw` 内才注册会错过当前帧的提取。当前在 `OnPreDraw` 注册，仍由实际 `OnDraw` 写入绘制时间，并由实际提交回调完成同一预约。预绘制始终返回 true，不取消绘制；仅收到预绘制或 Java 定时回调不能计数。[AOSP ViewRootImpl](https://android.googlesource.com/platform/frameworks/base/+/android12-release/core/java/android/view/ViewRootImpl.java)、[AOSP ViewTreeObserver](https://android.googlesource.com/platform/frameworks/base/+/android12-release/core/java/android/view/ViewTreeObserver.java)。参考源码不等同于已验证模拟器的框架二进制来源。

`FrameCommitSequence.java` 只校验回调顺序，不安排绘制。一次仅有一个预约；第一帧实际提交后才预约第二帧，第二次绘制必须晚于第一次提交；重复、外来预约、未绘制的提交及到期后的回调拒绝。原读取器仍要求视觉栅栏之后的两次真实绘制与原生提交、同一文档／进程／视图、有效几何、硬件渲染和原生窗口焦点。

## 准备输入

源码在 [`rf312-frame-runtime/`](rf312-frame-runtime/frame_owner.py)。来源保持已验证的原始字节；`frame_manifest.py` 的历史尾空行保留，Git 仅对此文件例外处理 EOF 空行样式，其他空白检查保持。使用正常 Python（不可 `-O`／`-OO`）、Pillow、显式 Windows Node，以及与默认打包配置一致的 JDK 和 Android SDK。先按[默认 APK 生成](RF-312-source-default-apk.md)和[独立环境与运行输入](RF-312-portable-owner.md)生成完整默认 `runtimeInputs`、默认打包配置及未启动的专用 API31／x86_64 AVD。不能拿旧失败回执替代新输入，也不能复制旧 AVD 用户数据。

所有配置均为 UTF-8 JSON，路径必须是实际绝对路径；输入、工具及源码 SHA-256 按实际字节计算。输出目录必须不存在。SDK 端口固定为 5057，设备固定为 `emulator-5578`，三个端口 5057／5578／5579 必须空闲。原有 5037／5049 服务不属于此入口。

构建配置字段精确为：

| 字段 | 内容 |
| --- | --- |
| `schemaVersion` | 整数 1 |
| `runtimeInputs`、`runtimeInputsSha256` | 完整默认运行输入及其 SHA-256 |
| `defaultPackageConfig`、`defaultPackageConfigSha256` | 同源默认打包配置及其 SHA-256 |
| `nodeExecutable` | 实际 Windows `node.exe` |
| `toolSha256` | `java`、`javac`、`jvm`、`androidJar`、`aapt2`、`apksigner`、`d8`、`zipalign`、`node` 九项实际 SHA-256 |
| `sourceSha256` | `build_frame_collector.source_hashes()` 的完整字典，包含共享来源及本目录来源 |

内部测试签名和 SDK／JDK 根目录来自已绑定的默认打包配置，不读取生产签名。构建只生成和检查采集器，不启动 ADB、模拟器或产品。

```powershell
# 在仓库根目录执行；以下路径变量均须先赋实际绝对路径。
$frameSource = (Resolve-Path 'docs/performance/rf312-frame-runtime').Path
$env:PYTHONPATH = $frameSource
python -B -m unittest discover -s $frameSource -p 'test_*.py' -v
python -B "$frameSource/build_frame_collector.py" --config $builderConfig --output $newCollectorOutput
```

构建执行十三个真实工具步骤，包括编译、11 项纯 Java 顺序回归、9 项就绪与 10 项启动标记检查（均从实际 Java 字面量提取）、DEX／资源打包、内部签名、签名／对齐及编译清单校验。六个诊断类进入 APK，测试类不打包。纯 Java 回归模拟已核对的参考框架事件顺序，本身不是原生渲染证据。

运行配置字段精确为 `schemaVersion`（整数 1）、`scope`（两帧模式为 `explicit-default-apk-native-frame-commits`，标记模式为 `explicit-default-apk-startup-milestones`）、`sharedPublic`（本仓库 `docs/performance` 的绝对路径）、`runtimeInputs`、`runtimeInputsSha256`、`collectorPackageReceipt`（新采集器 `package.json`）、`collectorPackageReceiptSha256`、`avd`（本次专用设备名）和 `sourceSha256`。最后一项由 `frame_binding.sources()` 生成；各回执及输入必须实际存在且内容匹配。源码目录保持仓库布局时自动定位共享目录；移动整个诊断目录时，另设 `RF312_FRAME_SHARED_PUBLIC` 为明确的共享目录，且须与配置一致。

## 运行与人工预检

```powershell
python -B "$frameSource/frame_owner.py" --kind native-frames `
  --preparation $freshAvdPreparation --output $newNativeOutput `
  --diagnostic-inputs $frameConfig --powershell $actualPowerShell `
  --powershell-sha256 $actualPowerShellSha256
```

入口先验证真实 SDK 进程身份、空白专用设备和原 240 秒启动预算。在出现 `preflight-ready.json` 后，查看本轮完整 `preflight.png`；1080×1920 原图应按三个连续的 1080×640 区域逐片实际检查，不能缩放后漏看，也不能把图像哈希校验当成已查看。还须完整检查对应 `preflight-window.stdout`、`preflight-anr.stdout` 和 `preflight-dropbox.stdout`，确认没有阻塞界面或异常。

原 600 秒审查预算内，在该输出目录新建 `continue-native.json`：

```json
{"pngSha256":"本轮原图实际 SHA-256","screenInspected":true,"healthReviewed":true}
```

仅在上述检查实际完成后填 true。发现异常则写 `reject-native.json`，让当前进程按原流程拒绝并清理。不要因观察超时、输出暂未出现或请求未返回而重启；先查询当前进程或工具句柄。

放行后只使用固定公开夹具，按 100→5000 运行；第一档拒绝时不会启动第二档。全过程保留原私有数据／APK／UID／设置备份和恢复、原生插桩身份、物理 UI／命中检查及正常 SDK 关闭／严格宿主后验。原 45 秒阶段、3 秒 JavaScript、5 秒状态 SDK、240 秒／512 请求观察器及 45 秒宿主退出期限不变。

## 可选被动窗口追踪

仅在上述运行命令末尾显式添加 `--window-frame-trace` 才启用；默认关闭，不启动追踪线程、不挂接这些附加监听器、不生成追踪文件。启用后在 Activity 创建完成时观察已有 Window／Decor，不主动创建 Decor、安排绘制、请求焦点或输入。`OnPreDraw` 始终返回 true。

它分别记录早期预绘制／绘制计数，以及最多 64 条连续的窗口指标前缀；总回调、原生报告丢失、截断和到期后回调另记。指标对象在回调内复制，保留不可用值 `-1`；投递时间与绘制时间不同，原始 VSYNC 时钟不转换、不与视觉栅栏比较。窗口指标不能计为两次原生提交，也不声称覆盖了所有帧。[窗口指标回调](https://developer.android.com/reference/android/view/Window.OnFrameMetricsAvailableListener)、[FrameMetrics](https://developer.android.com/reference/android/view/FrameMetrics)。

新增 `target-instrumentation/window-trace.json`、原 SHA 标记及读取结果 `window-trace-contract.json`。追踪是独立的 52 字段 schema 1，绑定原进程／APK／就绪回执和原 45 秒期限；同一把锁划分栅栏与实际停止采集时刻，原三秒拆除预算从拆除尝试开始计算。附加监听器和 worker 均须清理。失败时只在采集器真正结束后收回完整文件，旧失败原件与原两帧读取器保持。

## 读取结果与当前边界

查看父级 `result.json`、子级 `report.json`、`target-instrumentation/terminal-failure.json` 及其 SHA 标记、各清理回执和严格宿主后验。失败终态必须在实际采集器结束后收回，再由对应来源版本的 `refusal_contract` 校验；保存原始失败和不完整尾部，不混入成功样本。

七轮帧候选及最近一轮 19 份来源冻结在 `92d13ad4f`，原始证据和旧 APK 保留。最后一轮记录到 32 次预绘制、32 次绘制和 31 次窗口指标回调，但栅栏后没有新的 Java 绘制；一次指标投递不证明新帧。两帧阶段仍拒绝，5000 档未启动。

当前目录为 22 份来源，新增独立 `--kind startup-milestones`，不能与 `--window-frame-trace` 同用。新模式不经过两帧成功分支，ready／结果／终态／拒绝使用独立命名空间，旧帧读取器拒绝这些记录；原正向读取器、回调顺序和恢复块原字节保持。新包没有重新验收原生两帧，不能继承旧包的原生证据。

当前 68 项 Python、11 项纯 Java、9 项就绪与 10 项标记回归通过；公开十三步构建的 APK 与标记模式实测包逐字一致，绑定 117 份运行、126 份构建来源。三个正常 CLI、十二项优化模式拒绝及三个异地路径帮助通过。原生两帧、首次可交互、应用本体、重复分布与默认无插桩性能继续待验，详见[标记模式及复跑](RF-312-startup-milestones.md)。

原始 APK／构建产物、截图、系统回复、备份和本机标识只留本地。公开[七轮去标识证据](rf312-startup-frame-diagnostics-2026-10-18.json)及[诊断记录](RF-312-startup-frame-diagnostics.md)保留每一轮拒绝、首轮缺口和原统计。RF-312 仍未完成。
