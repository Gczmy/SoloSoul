# RF-312：默认包的采集队列与独立回放

本目录公开四格预跑实际使用的外层队列、启动预检和独立回放。工作区从脚本位置定位，SDK、JDK、制品、夹具、AVD 和输出取自显式输入；不包含本机路径或程序。这里的 12 份 Python 来源独立绑定，测量运行分组仍为 97 份；另冻结两份启动身份源件，以及原输入中的产品来源、宿主读取器和制品。原状态／内存读取器、输入命中／就绪、原生截图／正文、恢复及 240／45 秒等原预算保持。

仅用于 Windows 上的专用 API31／x86_64 AVD、2 CPU／2048 MiB／SwiftShader 和公开合成夹具。该入口不会下载工具或替代产品制品生成链；完整 RF-312 验收仍包括重复、启动两帧、应用本体、实际 IPC、观察成本和多端。

## 1. 准备显式输入

先按照[默认 APK 生成链](../RF-312-source-default-apk.md)、[库](../RF-312-portable-library.md)、[前端](../RF-312-portable-frontend.md)、[AVD](../RF-312-portable-avd.md)和[运行输入](../RF-312-runtime-inputs.md)准备并校验以下内容：

宿主还需要 Python、Pillow、PATH 中可找到的 `pwsh.exe`，以及 Windows OCR 的 `en-GB` 英语识别资源。本轮使用 CPython 3.12.14／Pillow 12.3.0；原 OCR 脚本严格指定 `en-GB`，资源缺失时会拒绝，不能用空识别结果代替物理正文证明。

- 当前源码、默认非 Debuggable Release APK／库及生成回执；原空账户 Debug APK、原测试 APK和夹具准备测试 APK。
- 固定 100／5000 对象公开 Vault 和公开 OCR 图片；独立 Observer 的构建回执与 APK。
- SDK／JDK 工具的实际路径和 SHA，以及全新、从未启动过的 API31 AVD 准备回执。固定序列号为 `emulator-5578`，专用端口为 5057／5578／5579；5037／5049 受保护。
- `runtime_inputs.py` 为该 AVD 生成的完整严格输入；[Java UI 传输](../RF-312-owned-ui-transport.md)的构建回执与 JAR。

每个模式使用另一个全新 AVD。适配配置按[默认状态入口](../RF-312-default-supported-state.md)生成完整 `sourceSha256`，使用 `schemaVersion: 2`，再加入 `ownedUiPackage` 的绝对路径和 `ownedUiPackageSha256`。两种模式绑定同一 Java 包；原运行输入只允许专用 AVD 名称不同。不得手工裁剪来源、数值或恢复清单。

所有构建、缓存、临时目录和证据优先使用 D 盘；输出的父目录须已存在，各次输出目录必须全新。以下命令在仓库根目录执行，变量均替换为本次确实准备的完整路径。

## 2. 运行离线回归

```powershell
python -B docs/performance/rf312-native-queue/related_units.py --output $newUnitOutput
```

它执行原 206 项相关回归和队列回归，保存完整用例 ID、原始日志及 SHA、97 份运行来源、外部严格读取器和本目录来源。`result.json` 必须通过且零跳过。测试临时目录位于本次输出内，结束后清理；不启动 SDK。

原生入口在任何 SDK 启动之前验证这份实际回归回执和原日志。源码、测试用例或日志改变后必须重新运行。所有入口拒绝 `python -O`，不能关闭原断言门禁。

## 3. 启动一种模式的完整队列

```powershell
python -B docs/performance/rf312-native-queue/owner.py `
  --kind default-state --preparation $newAvdPreparation `
  --adapter-inputs $adapterInputs --unit-evidence $unitResult `
  --sampling off --output $newNativeOutput
```

该命令保持运行，先捕获真实启动 PNG、窗口和完整健康记录，等待最多原 600 秒的人工预检。此时应用尚未安装。另一终端先执行只读 SDK 序列化探针：

```powershell
python -B docs/performance/rf312-native-queue/probe-native-pairs-r1.py `
  --native $newNativeOutput --inputs $runtimeInputs
```

实际打开 `$newNativeOutput/preflight.png`，查看完整画面，确认正常 Launcher、没有错误对话框；再确认健康记录没有 ANR 或崩溃标签。确实查看后，用原 PNG 的实际 SHA 发布准入：

```powershell
$viewedPngSha = (Get-FileHash -LiteralPath "$newNativeOutput/preflight.png" -Algorithm SHA256).Hash.ToLowerInvariant()
python -B docs/performance/rf312-native-queue/review-native-preflight.py `
  --native $newNativeOutput --viewed-png-sha256 $viewedPngSha
```

SHA 参数不能替代实际看图。异常时不要发布准入；可在该次输出内创建 `reject-native.json` 拒绝本次启动，保留原件。

准入后，外层核对包缺席和系统状态，准备原空账户 Debug 数据并完整备份，再顺序执行 100→5000 两档。一次失败就结束该模式队列；不能续接后档或重发失败输入。每档恢复原 APK／UID／私有内容／权限／设置，父级再次核验恢复并正常退出 SDK，原 45 秒内严格后验归零。仅父级 `result.json` 的 `passed: true` 表示整组通过；仍不表示 RF-312 完成。

另一种模式使用全新 AVD 和新输出，改为 `--sampling on`。两种模式都使用同一个 composite UI 通道；关闭组不创建行程采样目录。开启组保留每 2 秒计划槽位、12 个阶段边界、失败及漏采，不把周期拒绝转换为有效数值。

## 4. 独立回放

父级退出之后执行，原源码、制品、输入、备份和日志仍须保留：

```powershell
python -B docs/performance/rf312-native-queue/replay.py `
  --native $nativeOutput --adapter-inputs $adapterInputs `
  --unit-evidence $unitResult --output $newReplayOutput
```

回放输出与原生输出分开，不执行 SDK 或派发 UI 输入。它依次核验状态／严格内存与恢复、输入法几何就绪、双通道原始帧／原客户端响应／PNG和正文；模式由原生结果的布尔值决定，不能手工跳过开启组的完整内存轨迹、UID 清单、原物理 SDK 输出、阶段确认和周期缺口核验。原时延数值不被回放时长替代。

`context.json` 绑定输入、原生结果和当前来源。各独立结果、完整 stdout／stderr 和失败回溯保留；`result.json` 汇总是否全部通过。失败队列不能冒充完整成功队列，仍须按失败发生位置单独检查已有前缀与恢复证据。

## 结果使用

串行 PSS 不是原子总量，已采样最大值不能证明连续峰值。周期缺口始终单列。完成时间包含 UI 观察、核验及模式初始化／收尾，行程后共同内存检查点另计；不能当作应用本体时延。

比较时保留同源、同制品、同输入规则、实际顺序、样本数和全部失败；先按预定顺序交替开／关，再报告重复分布。旧外层的[四格预跑](../RF-312-same-source-pilot.md)与本目录的新原生验收分开记录，不因为入口公开就追认完整 RF-312。
