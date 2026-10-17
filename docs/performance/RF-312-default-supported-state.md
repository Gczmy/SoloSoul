# RF-312 默认包的有效状态与只读内存入口

`rf312_default_state_driver.py` 使用原默认 Release APK、原 UI 行程原语、恢复流程和严格内存解析器，补齐有效系统状态检查。测量过程中不安装目标采集器、不向应用执行 JavaScript。首版绑定 83 份源码；新增[可选行程内存模式](RF-312-journey-memory.md)及合并 UI 交换后为 88 份，原 83 份来源及旧输入完整保全且仍能由旧版本读取。原 71 份默认工具及已冻结输入保持。

## 输入与执行

先通过原 `runtime_inputs.py` 为新的专用 AVD 生成默认输入；原严格读取器继续校验全部产品／外部来源、APK、默认库、签名、夹具及观察器。额外的适配配置只包含四项：`schemaVersion: 1`、绝对路径 `runtimeInputs`、其 `runtimeInputsSha256`，以及 `rf312_default_binding.sources()` 返回的完整 `sourceSha256`。

```python
from pathlib import Path
import hashlib, json
from rf312_default_binding import sources

runtime = Path(runtime_inputs_path).resolve(strict=True)
config = {
    "schemaVersion": 1,
    "runtimeInputs": str(runtime),
    "runtimeInputsSha256": hashlib.sha256(runtime.read_bytes()).hexdigest(),
    "sourceSha256": sources(),
}
Path(adapter_inputs_path).write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
```

原生外层仍需负责新建隔离模拟器、真实 SDK 身份／静默／桌面／健康预检、原空账户 Debug APK 及测试包准备、退出和全局归零。以下入口用于这个已经校验的环境，参数取自同一份默认输入；不能用非默认诊断包或已有正式 Vault 替代。

```powershell
python -B docs/performance/rf312_default_state_driver.py `
  --adb $adb --serial $serial --avd $avd --apk $defaultApk `
  --restore-apk $originalDebug --test-apk $runtimeTest `
  --fixture $fixedFixture --object-count 100 --mode supported `
  --inputs $runtimeInputs --state-inputs $adapterInputs --output $newOutput
```

随后对 5000 对象使用另一新输出，保持固定顺序。入口要求完整私有目录保全和原恢复门禁；源码及输入在准备和恢复阶段再次校验。所有构建、缓存、临时目录及原始证据使用 D 盘，进程环境不写入全局设置。

## 检查与范围

- `rf312_state_gate.py` 在解析前保存原始 SDK 输出，只接受[有效状态读取器](RF-312-instrumentation-state.md)验证的完整区段。默认行程开始／结束要求无活动插桩，实际操作的检查点只允许自身 UI 观察器；原目标 PID 和窗口归属检查保留。
- `rf312_default_memory.py` 复用原严格进程／内存解析器，在原二十秒采样期限内，采样前后要求有效系统状态为空、原主 PID、观察器包缺席及应用聚焦。SDK 非零退出、stderr、超时、未知状态、进程变化及不完整渲染器范围均拒绝，不补零、不重发输入。
- 原四十五秒界面阶段、观察器与外层退出时限不变。新状态读取的开销包含在相应完成时间中；不能与旧上界耗时相减后当作应用本体开销。

首版 45 项回归、零跳过；入口、行程、内存和状态模块等七处运行入口的全局名称均检查。初版两轮在测量前暴露标准库导入遗漏，实测阶段数均为零；已保留两份 83 文件源码快照、失败回执和负向回归。外层分别完整恢复 71／124 项原私有条目及 APK／UID／设置并归零，失败不改为成功。

## 原生结果

第三台全新 API31／x86_64 模拟器，配置 2 CPU／2048 MiB／SwiftShader。同一默认 APK、2006 项来源及本轮 83 份适配来源保持。

| 固定对象数 | 完成 UI 阶段 | 行程状态核验 | 内存期完整进程转储 | 原始内存 SDK 命令 | 行程后串行 PSS |
| --- | --- | --- | --- | --- | --- |
| 100 | 12 | 169 | 4 | 22 | 231.64 MiB |
| 5000 | 12 | 171 | 4 | 22 | 281.57 MiB |

两档均核验空状态→自身观察器→空状态、观察器 PID 稳定、内存采样的两个必需缺席检查及严格解析器内部两个进程转储。44 条原始内存命令、8 份完整进程状态均独立回放，原严格解析器复算的身份、归属及全部数值一致；离线回放产生的采集时长不用于比较。范围为主进程及绑定的隔离渲染器，串行 PSS 不是原子总量、全部归属进程、全程或峰值内存。共享解析器的通用说明保留，插桩是否存在由本轮独立系统状态证据判定。

24 个实际 UI 阶段含两档各三页 PDF 的原物理内容核验。两档各 84 项私有内容／链接／权限及原 APK／UID／设置恢复，原会话清理通过；正常关闭 SDK 后，原四十五秒内第 8 次只读严格后验归零，后验重放通过。本轮没有 ANR，此前 FocusEvent 超时的根因仍未证明。

## 旧五轮证据的补充

旧专用状态子命令的 20 次不支持回复仍无效。进一步检查发现，原严格内存解析器在每个检查点内部还保存了前后两份有效、完整的系统进程转储。10 个旧内存点的这 20 份实际回复均通过当前读取器，活动插桩为零，且与原 SDK 记录的摘要和当时独立回放副本逐字节一致。

因此补齐的是旧 PSS 采样边界的直接状态证据，未重写原失败／回执或数值，不把本轮两个新点并入旧五轮分布，也不将这些内存边界扩大解释为旧完整 UI 行程每个检查点的直接证明。

见[去标识摘要](rf312-default-supported-state-2026-10-17.json)。本项只验收默认入口、具体界面行程与只读内存点，RF-312、365/374 和其他前置不变；同源观察开销、应用本体时延、其余内存口径、启动两帧及多端仍待。原始源码快照、日志、系统回复、截图、程序和私有目录保全仅留本地。
