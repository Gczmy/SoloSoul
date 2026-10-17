# RF-312 可配置的非默认诊断 APK 生成

新增 `rf312_ipc_package.py`，将已通过的同源诊断库与已验收默认 APK 绑定。仅替换 `lib/x86_64/libsolo_soul.so`，保留其余全部条目内容，以原内部测试证书重新签名；不安装应用，也不接受性能或完整 IPC 样本。

## 执行

配置必须完整包含 `schemaVersion: 1`、`runtimeInputs`／`runtimeInputsSha256`、`diagnosticLibraryReceipt`／`diagnosticLibraryReceiptSha256`、`defaultPackageConfig`／`defaultPackageConfigSha256`，以及只声明 `zipalign` SHA-256 的 `toolSha256`。三个引用文件使用绝对路径，仍须满足原严格读取器与同源库生产者回执。输出必须是不可变来源目录之外的全新目录。

```powershell
python -B -m unittest discover -s docs/performance -p test_rf312_ipc_package.py -v
python -B docs/performance/rf312_ipc_package.py --config ipc-package-config.json --output ../ipc-package-output
```

签名工具及对齐工具的哈希必须匹配。签名前核验原 APK 的证书，签名后核验产物证书和 16 KiB 原生库对齐；签名／对齐不得改变额外条目。工具与 Java 临时目录仅在本进程设置，实际执行均位于 D 盘工作区。

## 本轮证据

- 13 项回归通过：独立核心替换、非核心变化拒绝、路径／重复条目／原始 ZIP 名字规范、旧 v1 签名、不可覆盖输出、严格配置、工具摘要及单一证书；大小写不同的合法资源分别保留。
- 完整实际 CLI 生成通过，六条签名／对齐命令均成功。独立审计逐条比较 ZIP 内容，确认只有核心库改变，其余 740 项内容一致，核心与已验收诊断库逐字节相同。
- 新 APK 为 78630040 字节，逐字节等于此前完成启动功能探测的 APK。因文件相同复用其启动元数据／恢复证据，不新增原生运行或样本；29 次传输尝试仍只属于空账户启动阶段。
- 首次测试因沙箱临时目录权限在 setUp 阶段失败，不计通过；改为显式 D 盘工作区临时目录后通过，拒绝记录保留。宿主侧原专用沙箱路径已不存在，未额外删除其他目录。

旧默认 APK／库及读取器不变。该 APK 明确为非默认诊断，完整十二阶段 100／5000 行程、全部 IPC、同源观察开销、应用本体时延、全部归属进程及全程／峰值内存、启动两帧与多端仍待；RF-312 与 365/374 计数不关闭。

证据：[去标识摘要](rf312-ipc-package-2026-10-17.json)。原始输入、签名材料、APK 和日志只留本机。
