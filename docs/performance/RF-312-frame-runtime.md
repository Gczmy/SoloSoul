# RF-312：显式 Android 启动帧诊断入口

此入口用于复跑启动帧诊断及严格拒绝，当前两档原生验收仍未通过。它向默认 APK 的目标进程注入采集器并执行只读 JavaScript，不能作为默认性能样本、应用本体耗时或物理显示呈现证明。没有登录、输入密码、主动请求焦点、强制重绘或业务 IPC。

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

构建执行十二个真实工具步骤，包括编译、11 项纯 Java 顺序回归、9 项从实际 Java 字面量提取的就绪检查、DEX／资源打包、内部签名、签名／对齐及编译清单校验。四个业务类进入 APK，测试类不打包。纯 Java 回归模拟已核对的参考框架事件顺序，本身不是原生渲染证据。

运行配置字段精确为 `schemaVersion`（整数 1）、`scope`（`explicit-default-apk-native-frame-commits`）、`sharedPublic`（本仓库 `docs/performance` 的绝对路径）、`runtimeInputs`、`runtimeInputsSha256`、`collectorPackageReceipt`（新采集器 `package.json`）、`collectorPackageReceiptSha256`、`avd`（本次专用设备名）和 `sourceSha256`。最后一项由 `frame_binding.sources()` 生成；各回执及输入必须实际存在且内容匹配。源码目录保持仓库布局时自动定位共享目录；移动整个诊断目录时，另设 `RF312_FRAME_SHARED_PUBLIC` 为明确的共享目录，且须与配置一致。

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

## 读取结果与当前边界

查看父级 `result.json`、子级 `report.json`、`target-instrumentation/terminal-failure.json` 及其 SHA 标记、各清理回执和严格宿主后验。失败终态必须在实际采集器结束后收回，再由对应来源版本的 `refusal_contract` 校验；保存原始失败和不完整尾部，不混入成功样本。

当前六轮均未收到两次原生提交；第六轮采用本目录相同源码及同一字节的 APK，仍在 100 档拒绝，5000 档未启动。监听已安装，但未观察到后续有效预绘制／绘制或提交记录。注册顺序缺陷与缺少后续绘制不能混为同一根因。正向原生两帧、完整两档入口与性能样本均未验收；API、编译、CLI、回归测试或恢复通过不能替代这些结果。

原始 APK／构建产物、截图、系统回复、备份和本机标识只留本地。公开[六轮去标识证据](rf312-startup-frame-diagnostics-2026-10-18.json)及[诊断记录](RF-312-startup-frame-diagnostics.md)保留每一轮拒绝、首轮缺口和原统计。RF-312 仍未完成。
