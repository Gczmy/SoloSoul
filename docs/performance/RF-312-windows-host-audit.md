# RF-312：Windows 主机只读收尾核验

RF-1173 提供可复用的 [CLI](../../tauri/scripts/android-host-audit.py)，修复主机 JSON 依赖 Windows 默认编码的问题。它只查询 Windows CIM 和声明端口，不调用 ADB、改变窗口焦点、结束进程或修改系统设置。旧默认控制结果及原样源件包保持；这个入口没有接管完整 Android 性能行程。

## 输入与运行

需要 Python 3、PowerShell 7 和读取进程身份、TCP 监听状态的权限。在测试开始前保存 `expected.json`，测试后使用同一文件；不要根据收尾时的当前身份重新生成期望值。示例数值仅说明格式：

```json
{
  "schemaVersion": 1,
  "oldAdb": {
    "Name": "adb.exe",
    "ProcessId": 100,
    "ParentProcessId": 10,
    "CreationDate": "2026-10-16T10:00:00.123456+01:00",
    "ExecutablePath": "D:\\android-sdk\\platform-tools\\adb.exe",
    "CommandLine": "\"D:\\android-sdk\\platform-tools\\adb.exe\" -L tcp:localhost:5049 server nodaemon"
  },
  "ownedProcesses": [],
  "ownedPorts": [5057, 5578, 5579],
  "avd": "SoloSoul_public_test"
}
```

`oldAdb` 必须来自运行前的真实 `Win32_Process`，六个字段完整保存。出生时间必须带时区、最多六位小数；旧 ADB 必须是 loopback `nodaemon` 服务。`ownedProcesses` 的每项使用相同六字段结构，记录本次测试已启动的全部已知进程及辅助进程，退出前冻结清单。空数组只适合本次没有已记录进程的检查，不能代替运行时身份清单。`ownedPorts` 包含本次所有专用端口，不得包含旧 ADB 端口；`avd` 是本次专用 `SoloSoul_` 名称。

```powershell
python -X utf8 tauri/scripts/android-host-audit.py `
  --powershell 'C:\Program Files\PowerShell\7\pwsh.exe' `
  --expected 'D:\isolated-run\expected.json' `
  --output 'D:\isolated-run\host-audit'
```

输出目录必须不存在，避免覆盖前次结果。退出码 `0` 为通过，`1` 为核验或证据保存失败，`2` 为输入、平台或输出目录错误。失败后保留原目录；若需要后续只读复核，使用新目录，不重写首个失败。

## 检查与证据

- 旧 ADB 的名称、PID、父 PID、出生时刻、可执行路径和完整命令行必须一致。仅路径按 Windows 大小写规则比较，等价时区表示按同一时刻比较。
- 自有 PID、传递后代和命令行含精确专用 AVD 名称的 emulator/qemu 辅助进程必须缺席。PID 被复用也拒绝通过，不将新进程直接认定为本次所有，也不结束它。仅记录根进程无法保证识别所有已脱离父进程链的辅助进程，因此运行方仍须保存完整已知清单。
- 声明端口必须无 TCP 监听，包括由其他进程占用的情况。检查的是一次快照，不能证明持续空闲或替代设备内应用状态核验。
- PowerShell 在内部查询进程表，只序列化与旧 ADB、自有身份及专用 AVD 相关的行；stdout 仅输出计数、原因和哈希。完整身份保存在本地证据目录，不应将其直接作为公开摘要。
- 子进程按二进制采集，显式 UTF-8（可带 BOM）解码；错误 JSON、重复成员、不完整身份、超时和命令失败均拒绝。15 秒采集预算内只执行一次，无自动重试；超时/失败的原始 stdout、stderr 仍保存。

成功或核验失败时目录包含 `host-snapshot.json`、`host-snapshot.stderr` 和 `result.json`。结果绑定期望文件及快照 SHA-256；采集期间期望文件改变时拒绝。无法保存证据也返回失败，不能仅凭内存中判断报告通过。

运行编码、身份、端口及失败保留回归：

```powershell
python -X utf8 -m unittest discover -s tauri/scripts -p test_android_host_audit.py -v
```

[本项验证摘要](../verification/rf1173-windows-host-audit-2026-10-16.json)区分单元验证和真实主机对照；不声称已完成 RF-312 通用行程入口、默认完整业务行程或多端性能验收。
