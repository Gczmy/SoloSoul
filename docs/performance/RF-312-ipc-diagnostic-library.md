# RF-312 同源非默认 Android IPC 库编译组件

当前默认包不包含早期 IPC 观察器。新增 `rf312_ipc_library.py` 从已通过的默认输入另建诊断库，只添加已有的 `android-native-perf` 功能。原默认 APK、默认库及严格读取器保持。

## 可执行入口

配置包含 `schemaVersion: 1`、`runtimeInputs`、`defaultBuildConfig`、`defaultLibraryReceipt`，后三项各附 `Sha256` 字段，必须指向已验收默认生成链。输出目录必须全新且位于全部不可变输入目录之外。

```powershell
python -B docs/performance/rf312_ipc_library.py --config ipc-library-config.json --output ../ipc-library-output
```

该命令复制完整同源来源，使用原工具链、配置及标准库创建全新目标目录，不复用目标缓存。仅在实际 Cargo 成功、精确功能集合／Release profile、ELF、前端和全部来源复核后写入通过回执。编译失败及校验失败保留原输出，不能重标为通过。

## 已验证范围

- 复制 1955 份工作区来源及 38 项前端，13 份官方生成来源重生成并逐项匹配，完整绑定仍为 2006 项。
- 实际 `default`、`custom-protocol`、`android-native-perf` 优化 Release 库构建通过；profile 与默认库一致，38 个前端键保留。库为 59459944 字节，具体摘要见下方记录。
- 两个诊断标记存在，原默认库读取器仍以原原因拒绝该产物；默认 APK／库和 73 份生产者及共享代码保持。
- 新增 12 项生产者测试、现有 12 项 JavaScript 观察器测试通过；复用源码未变的 355 项既有运行回归证据。独立复核实际 Cargo 输出、二进制及完整复制来源通过。

## 未验收范围

本组件仅验收库生成，不将诊断包或真实 IPC 计为通过。独立默认包功能预检曾在 240 秒冷启动门槛失败，尚未进入应用测试；失败记录保留，随后正常关机及宿主无残留复核通过。需要以全新 AVD 重试，不用该轮替代原生通过。

后续仍需诊断 APK、真实十二阶段 100／5000 对象行程、进程／包身份、界面物理内容及恢复审计；还需同源观察开销、应用本体时延、全程／峰值及全部归属进程内存、启动两帧和多端。RF-312、365/374 台账计数及其余未完成项保持。

证据：[去标识摘要](rf312-ipc-diagnostic-library-2026-10-17.json)。原始产物、日志和失败材料仅留本机。
