# RF-312 显式 Android IPC 完整运行入口

本目录把已经实测的诊断行程整理为可从仓库直接执行的入口。它使用同源的非默认 Release APK、独立 UI 观察器和目标进程元数据采集器，记录十二阶段实际传输发送尝试；不等同于后端成功次数，也不接受默认产品性能分布。

原默认输入、APK、签名、标记及来源读取器保持。元数据只包含命令名、传输类型和时间，不包含请求负载。目标插桩和只读 JavaScript 元数据求值在结果中明确标记。

## 准备

1. 按[默认运行输入](../RF-312-runtime-inputs.md)准备完整来源、公开固定 Vault、原恢复包、UI 观察器和工具描述。
2. 按[诊断库](../RF-312-ipc-diagnostic-library.md)、[诊断 APK](../RF-312-ipc-package.md)和[采集器](../RF-312-ipc-collector.md)生成通过的回执。采集器回执必须绑定同一份默认输入，诊断 APK 的其他负载必须与默认 APK 逐字节一致；只允许既有内部测试证书。
3. 按[专用 AVD](../RF-312-portable-avd.md)准备从未启动的新 API31/x86_64 AVD。保留生成目录中的 `config.json` 和 `avd-preparation.json`。默认构建回执及原运行输入不因每轮 AVD 改名而重写；本目录的配置显式声明本轮 AVD 及同一构建版本的 SDK 客户端。

在仓库根目录生成本机配置；路径全部使用真实绝对路径，摘要来自独立核对的文件。配置只接受以下十一项，不接受省略或额外字段：

```python
import hashlib, json, sys
from pathlib import Path
sys.path.insert(0, str(Path("docs/performance/rf312-ipc-runtime").resolve()))
import diagnostic_binding

config = {
    "schemaVersion": 1,
    "runtimeInputs": runtime_inputs,
    "runtimeInputsSha256": runtime_inputs_sha256,
    "diagnosticPackageReceipt": diagnostic_package_receipt,
    "diagnosticPackageReceiptSha256": diagnostic_package_receipt_sha256,
    "collectorPackageReceipt": collector_package_receipt,
    "collectorPackageReceiptSha256": collector_package_receipt_sha256,
    "avd": new_avd_name,
    "adbExecutable": adb_executable,
    "adbSha256": adb_sha256,
    "sourceSha256": diagnostic_binding.sources(),
}
Path(new_config_path).write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
```

完整来源绑定包含原采集器生产链、状态／启动身份读取器和本目录的运行源码／测试。配置读取前后检查字节，准备和恢复时重新核对来源；不存在本机目录或工具回退。构建、缓存、临时文件及原始证据均应放在用户选定的非系统盘目录。

## 检查与运行

运行前将当前进程的 `TEMP`、`TMP`、`TMPDIR` 指向可写的验证目录，避免测试临时文件落回系统盘。定向检查包括本目录全部测试，以及共享状态、状态门禁和启动身份测试：

```powershell
python -B -m unittest discover -s docs/performance/rf312-ipc-runtime -p 'test_*.py' -v
python -B -m unittest discover -s docs/performance -p test_rf312_instrumentation_state.py -v
python -B -m unittest discover -s docs/performance -p test_rf312_state_gate.py -v
python -B -m unittest discover -s docs/performance -p test_rf312_launch_identity.py -v

python -B docs/performance/rf312-ipc-runtime/diagnostic_owner.py `
  --kind diagnostic-full --preparation $avdPreparation `
  --diagnostic-inputs $diagnosticInputs --powershell $powershell `
  --powershell-sha256 $powershellSha256 --output $newEvidenceDirectory
```

PowerShell 7 必须是显式的 `pwsh.exe`，程序摘要须匹配；不从 PATH 猜测。宿主必须无既有 SDK 进程或相关监听。外层固定启动自己的回环 ADB 服务及本轮 AVD，执行原 SDK 身份、静默、设备配置和包缺席检查，然后保存 Launcher PNG、窗口、完整 DropBox 与 lastanr，等待独立检查。

在原十分钟检查窗口内实际查看 `preflight.png` 和本轮健康日志。通过后，在同一输出目录新建 `continue-native.json`，包含该 PNG 的 `pngSha256`、`screenInspected: true`、`healthReviewed: true`。拒绝则写 `reject-native.json`；没有真实查看不能生成通过标记。

外层安装并预热精确的空账户 Debug 包，完整备份后按 100→5000 固定顺序调用 `diagnostic_driver.py`。子入口也可在外层已核验、已安装原包的环境中显式运行；不应绕过前述准备和后验：

```powershell
python -B docs/performance/rf312-ipc-runtime/diagnostic_driver.py `
  --adb $adb --serial $serial --avd $avd --apk $diagnosticApk `
  --restore-apk $originalDebug --test-apk $runtimeTest --fixture $fixedFixture `
  --object-count 100 --mode supported --inputs $runtimeInputs `
  --diagnostic-inputs $diagnosticInputs --output $newChildEvidenceDirectory
```

原 45 秒阶段、240 秒观察器、512 请求、3 秒元数据读取及恢复／退出预算保持。只允许实际目标插桩和自己的 UI 观察器，包、PID、UID、进程出生、完整累计事件前缀及物理 UI 内容全部核验。新状态读取会占用原阶段预算，不能从完成时间上界中相减推导应用本体开销。

## 证据与恢复

每档保留完整十二阶段、原生截图、PDF 物理内容、元数据原件／摘要、传输命令及终态；精确核对 50 份自有元数据后才移除设备端目录。私有内容、链接、权限、原 APK／UID 和系统设置均完整恢复。正常停止 SDK 后，在原四十五秒内执行严格只读宿主归零核验。子步骤失败仍保留失败，外层恢复不能将其改为成功。

只有父 `result.json` 及两档独立审计均通过，才能接受该次完整 UI／IPC 运行。离线合成回执、单元测试或历史运行不能替代新入口的原生验收。本目录不接受默认性能、应用本体时延、全部进程／全程／峰值内存、启动两帧或多端完成，RF-312 仍按原任务范围验收。

## 本次验证与未完成范围（2026-10-17）

- 72 项不同测试通过，无重复和跳过；包含九处主要运行入口的全局绑定，以及两个 CLI 从含空格的新目录直接运行。配置完整绑定 101 份来源，读取前后校验配置和来源；错误摘要、缺失／额外字段、原默认拒绝和验证中变化仍拒绝。默认 71 份工具和产品包未改。
- 子入口主函数与此前实测版 AST 一致，八个共享函数与 canonical 源码一致，元数据读取器及其原测试逐字节保持。新公开读取器独立回放历史两档 24 阶段、343 次状态、42 份不同 PNG 和 12 份物理文字证明，发送尝试仍为 235／240。这是历史原件回放，不能记为新公开入口运行成功。
- 第一台全新 AVD 在原 240 秒启动预算内未就绪，应用安装和测量阶段均为零。原失败保持；自有 SDK 正常停止，原期限内第九次严格后验归零，离线重放通过。
- 第二台全新 AVD 通过实际 Launcher／完整 38 项健康记录预检，100 对象档完成十阶段、十份元数据及六份物理内容证明，锁定阶段验证时观察器明确报 `Full journey observer bound exhausted`。请求数 330，小于原上限 512；实际耗尽的是原总时间界限。整档仍失败，5000 档没有启动。三个内部清理结果仍为拒绝，原错误、控制临时数据和终态保留，不能用外层恢复重标为通过。
- 第二次失败的独立核验重读了 18 份不同 PNG、六份内容证明、155 份已接受状态及一份被原精确期望拒绝的状态；不完整元数据仍被十二阶段读取器拒绝。外层及子步骤各完整恢复 134 项私有内容／链接／权限、原 APK／UID 和设置；SDK 正常关闭，原四十五秒内第八次严格后验归零及离线重放通过。末尾系统记录无当前启动 ANR，此前 ANR 根因仍未证明。

十个完成阶段的宿主观察上界合计 174.04 秒，对应追加元数据读取上界合计 4.39 秒；两条插桩同时活动期间的状态 SDK 调用累计墙钟约 12.85 秒。范围不同且有重叠，不能相加或相减推导应用耗时，也不代表有开／关配对的因果观察开销。

因此本轮只接受公开 CLI、显式配置与历史读取器回放，**新完整原生队列尚未通过**。下一步先按既有记录核对总预算中准备、界面及观察工作各自的占用，再决定是否调整取证实现；不延长现有期限、不减少验收阶段、不重试替换失败。见[去标识摘要](../rf312-public-ipc-runtime-2026-10-17.json)。原始截图、程序、日志、私有归档及本机路径仅留本地，RF-312 和 365/374 保持。
