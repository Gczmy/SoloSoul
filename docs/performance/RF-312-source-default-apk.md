# RF-312：当前源码绑定的默认 Android APK 生成

## 当前结果

新增显式配置的 APK 生成组件，消费通过正式验收的当前默认库回执，在新的私有目录执行真实 Android Release 打包并验证 APK。完整 355 项唯一回归通过，新增 46 项，零跳过。此处只证明生成与内容绑定，不证明原生行程或性能。

`package_payload.py`复核默认库、原始 Cargo 产物、当前前端、官方桥接及生成元数据，从锁定的 crate 归档提取六个 Android 模块，再应用两份产品补丁；不修改共享 Cargo 源码。`package_environment.py`固定工具、Gradle 版本与完整运行时、JDK、Gradle 配置及实际项目图，隔离构建输出、资源和签名环境。`package_build.py`执行真实 Release Gradle 任务，复核版本、APK 内容、内部测试签名及全部输入。

## 实际核验

- 当前 1,801 份产品源码、108 份插件及 46 份移动资源进入新的私有树；使用正式库生成的十份桥接及元数据。
- 六个模块共 79 份文件与锁定归档／两份产品补丁匹配；真实 Gradle 图为八个私有项目，实际 JDK 与声明一致，全部外部解析依赖记录并复核。
- APK 完整资源为当前源码 98 项、原依赖 25 项和当前 Gradle 任务实际生成的两份 profile，共 125 项。先核对97份已跟踪Android主资产，再用当前90份生成资源更新相应内容；生成清单外的8份已跟踪文档仍来自当前源码。不会复制旧包的应用资源或将旧资源重解释为依赖。
- 原生库集合恰为当前默认核心库和原 OCR 依赖。核心逐字节等于通过验收的真实 Cargo 产物，全部 38 项当前前端资源键存在，可选观察器标记不存在。
- 实际包为 x86_64 Release，minSdk 28／targetSdk 36，非 debug；版本来自当前 Tauri 配置及锁定 CLI 规则。未签名校验按预期拒绝，内部测试签名验证通过，签名前后所有 ZIP 文件内容一致。

打包前已有真实 Gradle 隔离钩子测试及归档模块提取测试。首次沙盒内钩子测试因不能连接本地 Gradle daemon 失败，原件保留；不能据此判断 JVM 崩溃。实际组件采用正常本地构建执行环境，无网络下载。计数、产物摘要及原件哈希见[去标识摘要](rf312-source-default-apk-2026-10-17.json)。程序、密钥、原始日志与本机路径留本机。

首次345项完整回归使用了受限沙盒的默认临时目录，因文件权限失败；另建D盘临时目录后同源345项用例完整通过，不改断言。随后真实打包准备发现产品已跟踪`tauri.properties`，原要求文件不存在而拒绝，Gradle尚未启动。修正为绑定、解析并逐字节保留当前配置匹配的源码版本文件；补充3项回归后348项完整通过。版本不符、未知／重复配置或未绑定既有文件仍拒绝，不覆盖源码。首轮私有恢复资源诊断误抄哈希多一个字符而拒绝，改为读取原已验输入后确认恢复包清单及89项当前资源一致。所有初始拒绝保留，不重标为首次成功。

第二轮真实Gradle因依赖收集使用旧接口而触发Android项目多产物歧义；同一真实Gradle夹具复现旧失败，使用[ArtifactView组件过滤](https://docs.gradle.org/current/userguide/artifact_views.html)后准确保留外部模块及项目传递依赖。第三轮完成266项任务至封装，因`afterEvaluate`签名调整晚于变体捕获而失败；改为Android的[finalizeDsl阶段](https://developer.android.com/reference/tools/gradle-api/7.1/com/android/build/api/extension/AndroidComponentsExtension)，真实Gradle生命周期夹具复现晚改失败并验证新实现通过。第三轮另被完整资产范围审计拒绝；遗漏8份已跟踪文档的4项回归中原实现3项失败，候选修正4项通过，正式接入后352项完整通过。全部失败保留，第三轮未用于原生验收。

第四轮真实Release构建、未签名检查和内部签名均成功，但结束后依赖复核被Windows旧路径长度限制拒绝，整体回执仍失败。7份缓存依赖的路径长262～283字符，普通访问报告不存在，扩展本地路径能完整读取。修正仅在读取长路径时使用扩展本地盘符语法，保留普通文件／reparse／身份／完整大小和哈希保护；拒绝外来设备别名、UNC、相对路径及遍历，不改全局路径策略。增加3项回归，355项完整通过；247份真实依赖逐项复核通过，原Gradle初始化脚本的源码和输出保持。第五轮另建目录重新执行完整生成，没有把第四轮重标通过。

## 复跑入口

从仓库根目录执行`python -B docs/performance/rf312-default-observer/package_build.py --config <配置文件> --output <新目录>`。输出父目录必须存在，目标必须不存在且不在冻结的前端／库输入树内。Windows本地Gradle需要允许其客户端与本地daemon通信；构建保持离线。

配置必须完整包含`package_build.FIELDS`中的字段：

| 字段 | 输入要求 |
| --- | --- |
| `schemaVersion` | 整数1 |
| `libraryResult`／`libraryResultSha256` | 正式通过的当前默认库回执绝对路径及SHA-256；失败回执、来源不匹配或可选观察器库拒绝 |
| `sdkRoot`／`jdkRoot`／`gradleRoot`／`gradleHome` | 显式本地根目录；Gradle实际版本须与已跟踪wrapper一致 |
| `toolSha256` | `java`、`javac`、`jvm`、`aapt2`、`apksigner`、`androidJar`六项完整哈希；具体相对路径见`package_environment.tool_inputs` |
| `gradleFiles` | 完整Gradle分发文件的相对路径到`{bytes,sha256}`映射；可用`frontend_build.files`生成 |
| `gradleHomeConfiguration` | Gradle用户根的`gradle.properties`、`init.gradle`、`init.gradle.kts`及`init.d`全部实际文件映射；不存在的项不列入，无配置时为空对象 |
| `dependencyReference` | `apk:{path,bytes,sha256}`、`appAssets`和`certificateSha256`；`appAssets`是前一已验证包全部应用资产的名称到SHA-256映射，不包括依赖或profile |
| `testSigner` | `keyFile`、`sha256`、`certificateSha256`和`purpose:'internal-validation-only'`；使用内部Android测试密钥，证书必须与参考一致 |

文件哈希为小写64位十六进制，可用`frontend_build.entry`计算；输入文件和目录必须是普通路径。全套离线回归入口为`runtime_units.py --output <另一新目录>`，先显式把本进程`TEMP`／`TMP`设到D盘已存在目录。组件本身固定构建临时位置、移除继承的生产签名与运行覆盖变量、保留原版本文件；签名只在独立APK生成后使用内部测试密钥。

## 范围与后续

没有安装新包或发送设备输入。原默认包、读取器、运行时保护和历史拒绝记录保持，当前新包需要单独冻结同源验收输入后执行原生行程，不能直接套用旧包资源清单或改写旧失败。尚需重复性能分布、全程内存、同源观察器开销、实际 IPC、启动两帧和多端测量。构建耗时不作应用性能，RF-312 未完成；365/374 计数及三个父任务、六项依赖保持。
