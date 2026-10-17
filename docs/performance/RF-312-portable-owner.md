# RF-312：显式输入的专用运行环境与两档队列

本入口接收已验证的默认APK、原始恢复包、公开固定Vault和SDK，执行原生启动、独立预检、两档十二步、完整恢复及离线审计。源码位于[组件目录](rf312-default-observer/README.md)。它不生产默认APK或固定Vault，也不代替原构建／签名来源证明。

## 输入与准备

先依照[运行输入](RF-312-runtime-inputs.md)准备`inputs.json`，依照[新AVD入口](RF-312-portable-avd.md)创建未启动的专用AVD。两份描述中的SDK和AVD名必须一致。当前协议固定API31 x86_64、原端口5057／5578／5579，原始保护端口5037／5049保留；设备内实际核验2核、约2048MiB及原WebView版本。

SDK还须包含原`platform-tools`和`build-tools/35.0.0`，JDK由AVD准备记录声明。路径须满足原SDK／JDK代码页限制，输入及输出均须为普通路径。没有固定SDK、工作区或临时构建目录回退；系统路径保持原读取器的判定方式。

运行同库完整离线回归，结果绑定全部运行源件与测试ID；重复测试ID、跳过或源件变化均不能生成通过的证据：

```powershell
python -B docs/performance/rf312-default-observer/runtime_units.py --output '<新的回归结果目录>'
```

保存本机`owner-config.json`。所有SHA须来自独立核验的对应输入；资产映射使用既有已验证清单的完整值，不能用示例条目替代：

```json
{
  "schemaVersion": 1,
  "runtimeInputs": "<准备好的inputs.json>",
  "runtimeInputsSha256": "<该清单SHA-256>",
  "avdPreparation": "<新AVD的avd-preparation.json>",
  "avdPreparationSha256": "<该记录SHA-256>",
  "powershell": "<PowerShell 7的pwsh.exe>",
  "powershellSha256": "<该程序SHA-256>",
  "hostSourceSha256": {
    "android-host-audit.py": "<同库原宿主身份读取器SHA-256>",
    "android-fresh-host-audit.py": "<同库空宿主读取器SHA-256>",
    "android-home-native-regression.py": "<同库完整恢复读取器SHA-256>",
    "android-process-memory.py": "<同库严格内存读取器SHA-256>"
  },
  "unitEvidence": "<本轮回归result.json>",
  "unitEvidenceSha256": "<该结果SHA-256>",
  "packageReference": {
    "previousRelease": {
      "path": "<原已验证默认Release APK>",
      "sha256": "<原APK的SHA-256>",
      "bytes": 0
    },
    "appAssets": {},
    "generatedProfiles": {}
  }
}
```

`bytes`、`appAssets`和`generatedProfiles`须替换为真实完整清单；上述0及空对象会被拒绝。当前基线分别有98项应用资产、25项依赖资产、2项生成配置，共125项；原默认APK用于复核依赖和全部原生库逐字一致。五个实际APK签名、默认Release非debug属性、APK内原生库、所有资产及恢复Debug包的89项资源均重新核验。资源初始化清单直接来自APK，原始字节与恢复包一致，不再读取某个旧构建目录。

```powershell
python -B docs/performance/rf312-default-observer/owner_inputs.py `
  --config '<owner-config.json>' --output '<新的环境计划目录>'
```

通过后生成`owner-plan.json`及完整包核验记录。计划冻结原输入、源码、宿主读取器、工具及完整新AVD清单。启动前再次逐项检查AVD文件／目录及注册文件；被启动过、增加lock或改变字节的AVD拒绝使用。运行代码或测试改变后须重新生成回归证据和计划。

## 启动和独立预检

```powershell
python -B docs/performance/rf312-default-observer/runtime_owner.py `
  --plan '<owner-plan.json>' --output '<新的本轮证据目录>'
```

宿主必须没有ADB／模拟器和相关监听。原只读宿主CLI先核验空基线，源件与原始输出保留。程序只启动本次回环ADB和新AVD，不重置或结束其他服务器。原240秒启动及5秒采样静默窗口保持；未识别SDK客户端不会被赋予所有权。

输出`native-default-full-journey-r1/environment-preflight.json`后，程序在原600秒窗口等待独立审阅，此时尚未安装应用：

```powershell
python -B docs/performance/rf312-default-observer/runtime_preflight.py collect `
  --plan '<owner-plan.json>' --output '<本轮证据目录>'
```

收集器核验实际CIM归属、VM出生时间、API／ABI／WebView、四个包缺席、完整且未达上限的DropBox、所有年代的ANR及Launcher焦点。没有按时间筛除错误，也没有截取最近若干行。额外保存第二张Launcher图。

必须实际检查以下两张图片，确认Launcher正常且无系统错误弹窗：

- `native-default-full-journey-r1/preflight.png`
- `native-default-full-journey-r1/preflight-fresh-independent/preflight.png`

检查后才记录对应原图和独立结果的SHA，继续原队列：

```powershell
python -B docs/performance/rf312-default-observer/runtime_preflight.py review `
  --plan '<owner-plan.json>' --output '<本轮证据目录>' `
  --original-png-sha256 '<第一张图SHA-256>' `
  --latest-png-sha256 '<第二张图SHA-256>' `
  --independent-result-sha256 '<独立result.json的SHA-256>' --screens-inspected
```

`--screens-inspected`是操作者对实际看图的记录，不是自动图像验收。已通过／拒绝的标记不覆盖，图片或独立结果不匹配不能放行。

## 行程、恢复与审计

APK内资源由实际MainActivity初始化后检查标记及全部哈希，再执行100／5000各十二步。原45／240秒、512请求限制、只读就绪判定、精确触摸、物理文字／计数和三页PDF检查保持。观察器只插桩自身包；原SDK回调恢复后，才运行严格的22条后置内存命令。

原私有目录TAR、模式、链接、APK／UID／debug及九项系统设置完整恢复。不能证明恢复时保留包和证据，不用清理掩盖失败。模拟器与自有ADB按原归属流程退出。SDK的关闭助手可能继续执行20秒延迟；只在原模拟器45秒退出预算的剩余时间内，读取原CIM查询并等待严格匹配的关闭后代自然退出。目标PID、SDK路径、命令、祖先、出生时间或受保护监听不符立即拒绝，不结束进程或重试输入。原宿主CLI再绑定本轮before及所有已验证PID／出生时间，独立复核零残留。

成功退出还要求原终态断言、全部原始控制帧／请求／回复／PNG、完整内存读取重放及Tools就绪轨迹通过。实际子进程参数、运行输入SHA和20份原行程源件逐项绑定。`owner-result.json`的`passed`表示本次环境与队列通过，`fullDefaultJourneyAccepted`和`performanceDistributionAccepted`仍为false。

所有失败输出保留，不重启服务、重放未知输入、扩大原预算或覆写已有证据目录。一次后置PSS或完整队列不等于重复分布、全行程峰值、观察器完整开销、实际Tauri IPC、两帧提交或多端验收；这些仍属于RF-312的后续工作。


## 本轮实际结果（2026-10-17）

244项唯一同库回归、零跳过；当前默认Release在新SDK／JDK／输出空格路径下，通过全新100／5000各十二步、各三页PDF及各22条严格后置内存命令。125项资产、五个包签名及89项实际初始化资源均复验；原1,881项输入全部保留，二十份原运行源件逐字未改。完整原终态断言、5,825个控制帧、689份请求／回复、42张PNG及整个内存检查点回放通过。

两档各148项原始私有记录、APK／UID／debug及九项设置完整恢复。SDK的20秒关闭助手经过九次原只读宿主查询自然退出，跟踪六份已验证身份，原45秒预算还剩20.09秒；原宿主CLI随后复核零进程／零监听。完整终态DropBox共106项，无任何ANR。此处PSS为239.22／268.69MiB，只是两个行程后串行点，不构成分布、峰值或改善结论。

首轮两档界面和内存均通过，但即时复核仍有三个SDK关闭后代，因此整轮失败保持；后续自然退出及原读取器零残留复核、原始帧／内存重放另存，没有改写首个失败或混合两轮作性能分布。原SDK识别器、九项恢复、PDF／帧／内存断言保持；只在原退出预算余量内加入严格归属的只读等待，不结束未知进程。新增回归的最初夹具排列错误及其失败日志也保留。

本入口接收已验证产物，不代表全部产物生成器已联通；重复分布、全行程峰值与完整观察器开销、实际Tauri IPC、两帧提交及多端仍未完成。见[去标识摘要](rf312-portable-owner-2026-10-17.json)。
