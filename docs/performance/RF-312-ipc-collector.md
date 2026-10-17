# RF-312 显式 Android IPC 元数据采集器构建

`rf312_ipc_collector.py` 在新的隔离目录编译 `rf312-ipc-collector/`，使用既有内部测试证书签名。该包是明确的非默认目标进程插桩工具，旧默认观察器读取器继续拒绝它。构建入口不会安装应用、启动插桩或接受原生性能样本。

## 执行

配置完整包含九项：`schemaVersion: 1`、`runtimeInputs`／`runtimeInputsSha256`、`diagnosticLibraryReceipt`／`diagnosticLibraryReceiptSha256`、`defaultPackageConfig`／`defaultPackageConfigSha256`、`nodeExecutable` 和 `toolSha256`。四个路径使用绝对路径；三个回执逐字节绑定，仍须通过原严格默认输入读取器和同源诊断库核验。`toolSha256` 只声明 `d8`、`zipalign`、`node` 三项 SHA-256，其余六项 SDK／JDK 身份取自绑定的默认打包配置。

```powershell
python -B -m unittest discover -s docs/performance -p test_rf312_ipc_collector.py -v
python -B docs/performance/rf312_ipc_collector.py --config collector-config.json --output ../collector-output
```

测试前将本进程的 `TEMP`／`TMP`／`TMPDIR` 指向可写工作区目录。构建的 Java 用户目录及全部临时目录位于新输出内；实际验证均位于 D 盘。SDK／JDK／Node、输入回执、源码和签名身份不能在构建中变化；输出不能位于不可变输入根内，也不能覆盖已有目录。

## 契约及边界

Manifest 只允许三个直接子节点，固定包名 `org.solosoul.rf312ipc`、入口 `org.solosoul.rf312ipc.Collector`，目标包及进程都为 `com.solosoul.app`；非 debug，无额外组件或权限。编译 SDK 36、min SDK 28、target SDK 35。签名后的 APK 只有清单、资源表、一个 DEX 和 `assets/snapshot.js` 四项内容，不含原生库。

采集器不获取 UiAutomation，也不发送界面输入；它仅在公开夹具的主进程、唯一主 WebView 中读取既有早期观察器的命令名、传输类型和时间元数据，不读取请求负载、不调用产品命令。请求顺序、run token、APK／PID／UID／进程出生时间、文档、累计事件前缀及读取期限均有门禁。Java 后续增加了缺省关闭的[启动堆栈取证](RF-312-startup-stack.md)，Manifest 和查询脚本保持；新包必须另建回执，不能重标旧原生证据。十二阶段完整接受仍需外层实际界面行程和恢复审计；仅构建成功不能关闭 RF-312。

## 初次公开构建的验证

- 17 项 Python 回归和 4 项 JavaScript 查询回归通过，零跳过；覆盖源码及编译清单身份、节点层级、布尔属性、版本／SDK、缺失／额外属性、严格配置、工具摘要及确定性 ZIP 元数据。
- 实际查询测试、Java 编译、D8、AAPT2、对齐、签名、证书／签名、签名后对齐、编译清单九步通过；81 份生产者／依赖源码和 HEAD 在构建期间保持。
- 独立审计确认三份运行源码及四项 APK 内容与此前采集器一致，实际 v3 签名和既有内部证书通过，原默认读取器精确拒绝目标插桩包。新包 16652 字节，DEX ZIP 时间固定为 1980-01-01；整个 APK 与旧包不同，旧 APK 绑定的原生证据不重标。本轮没有新增原生运行或性能样本。

后续使用新 APK 的精确哈希完成独立会话接入和真实 100／5000 十二阶段；实际 IPC、同源观察开销、应用本体时延、全部归属进程及全程／峰值内存、启动两帧和多端仍待。365/374 计数保持。

见[去标识验证摘要](rf312-ipc-collector-2026-10-17.json)。原始本机路径、签名材料、APK 和日志不入库。
