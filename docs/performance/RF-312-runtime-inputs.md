# RF-312：完整界面子行程的显式运行输入

本入口将已验证默认构建的工作区、SDK、产物和固定Vault路径显式重绑定，运行原十二步子行程。源件位于[观察器目录](rf312-default-observer/README.md)：[准备器](rf312-default-observer/runtime_inputs.py)、[驱动](rf312-default-observer/default_driver.py)、[会话](rf312-default-observer/observer_session.py)和[动作调用方](rf312-default-observer/default_journey.py)。

## 准备配置

先准备并独立核验默认Release、恢复用Debug／测试APK、运行期恢复测试APK、默认原生库、公开合成Vault和独立观察器，保留原构建与签名证明及输入快照。准备器继承这些快照的校验值，复验整包、默认构建标记、APK内原生库、全部固定Vault文件和源件；角色路径由配置提供。

保存本机`config.json`，替换所有占位符：

```json
{
  "schemaVersion": 1,
  "referenceInputs": "<已独立验证的原输入清单>",
  "referenceInputsSha256": "<原清单SHA-256>",
  "referenceWorkspace": "<原清单记录的工作区根>",
  "workspaceRoot": "<当前SoloSoul工作区>",
  "sdkRoot": "<当前Android SDK>",
  "adbSha256": "<已核验adb.exe的SHA-256>",
  "serial": "emulator-5578",
  "avd": "SoloSoul_public_test",
  "observerPackage": "<独立观察器package.json>",
  "observerPackageSha256": "<该package.json的SHA-256>",
  "artifactPaths": {
    "release": "<默认Release APK>",
    "originalDebug": "<恢复用Debug主APK>",
    "originalTest": "<原测试APK>",
    "runtimeTest": "<运行期恢复测试APK>",
    "nativeLibrary": "<已验证的默认libsolo_soul.so>",
    "observer": "<独立观察器APK>"
  },
  "fixtureRoots": {
    "100": "<原100对象固定Vault>",
    "5000": "<原5000对象固定Vault>"
  },
  "externalSourceRoots": {
    "frontend": {
      "referenceRoot": "<原清单中独立前端产物的根>",
      "root": "<当前同字节前端产物的根>"
    }
  }
}
```

`referenceWorkspace`和外部`referenceRoot`用于解析原快照的逻辑路径；实际读取只使用当前声明的根。每个工作区条目转换为相对路径，独立构建资源进入命名根；未知外部路径、多个根同时匹配、映射碰撞或条目丢失均拒绝。角色的SHA不随路径重绑定而改变。当前参考共有1,881项：工作区内1,843项，独立前端38项。

```powershell
python -B docs/performance/rf312-default-observer/runtime_inputs.py `
  --config '<config.json>' --output '<全新的输入输出目录>'
```

成功输出`inputs.json`、原配置副本和`binding-result.json`。准备阶段不运行SDK命令或派发输入；验证失败保留非通过结果，不生成可用清单。重复输出目录拒绝。配置、原参考和观察器描述文件在准备期间必须保持字节不变。

## 运行子行程

外层须先准备专用、健康的AVD，确认没有个人账户、目标及观察器插桩未运行，并按既有流程完成原始APK／资源初始化、完整ANR检查和截图审阅。当前复验环境为API31 x86_64、软件GPU、2CPU／2048MiB；ADB服务固定loopback5057。此入口保持这些既有环境要求，外层自动化仍待参数化。

下面从准备好的清单取值；输出目录须不存在，父目录须存在：

```powershell
$inputFile = '<准备好的inputs.json>'
$runtime = Get-Content -LiteralPath $inputFile -Raw | ConvertFrom-Json
python -B docs/performance/rf312-default-observer/default_driver.py `
  --inputs $inputFile --adb $runtime.host.adbExecutable `
  --serial $runtime.host.serial --avd $runtime.host.avd `
  --apk $runtime.release.apk.path --restore-apk $runtime.originalDebug.path `
  --test-apk $runtime.runtimeTest.path --fixture $runtime.fixedFixtures.'100'.root `
  --object-count 100 --mode supported --output '<新的100档结果目录>'
```

5000档将两个100选择器同时改为5000。SDK、serial、AVD、APK及固定Vault参数必须与准备清单精确一致；没有临时工作区或默认SDK回退。

驱动在执行前、恢复后检查完整输入及源件，报告绑定输入文件和20份运行源件哈希。观察器仍只插桩独立包；通用Client输入检查、稳定就绪、公开密码及文件选择器、三页PDF实际文字／页码、12步顺序、45／240秒与512请求限制保持。完整私有目录、原APK／UID／debug、系统设置和公开临时文件按原实现恢复。读取器正常退出后才以原SDK执行严格内存检查点。

## 检查范围

本项原206门禁与同库61项唯一回归分别运行，计数不相加冒充独立总数。同库回归覆盖显式根、完整源件重绑定、外部资源根、路径／哈希拒绝、原协议、只读原生目标探测及物理证据根。

完整界面子行程、原始控制帧、后置内存及状态恢复按实际队列结果另行记录。完整机器准备／编排与终态审计、重复分布、IPC、启动两帧和多端仍有后续工作；本入口不关闭RF-312。


## 本轮实际结果（2026-10-17）

同一默认Release的新独立队列已通过100／5000对象各十二步、各三页PDF和各22条严格后置内存命令。原严格审计规则未改，5,874控制帧、695请求／返回及43张截图逐字回放通过；驱动报告、实际命令行、SDK／观察器入口与20份源件、同一输入清单全部绑定。1,881项原哈希及条目一份不丢，重绑定结果由原参考重新计算一致。

两档各190项完整私有记录、原APK／UID／debug及九项设置恢复，观察器／控制正常关闭、临时目录移除，完整DropBox107项无ANR，宿主零进程／监听。PSS为231.68／281.43MiB，检查点2889.80／1349.01ms；仅为两个行程后串行点，不推断改善、峰值或分布。

首次离线准备因未声明独立前端根而拒绝，未运行SDK／派发输入；保留拒绝后显式绑定原38项前端文件，原校验值不变。新入口使用同库宿主身份模块；历史配方保持。见[去标识验证摘要](rf312-runtime-inputs-2026-10-17.json)。
