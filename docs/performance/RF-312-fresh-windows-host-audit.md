# RF-312：空宿主的参数化前后核验

2026-10-17，[入口](../../tauri/scripts/android-fresh-host-audit.py)将原本位于本机配方中的空宿主检查公开为Windows CLI。它从同目录读取RF-1173身份模块，不依赖其他工作区或固定SDK位置。原[受保护旧服务入口](RF-312-windows-host-audit.md)字节保持。

## 配置与运行

需要Python 3、PowerShell 7及只读CIM／TCP权限。运行前确认没有ADB、emulator或qemu，再创建`before.json`：

```json
{
  "schemaVersion": 1,
  "baseline": "no-existing-adb-or-emulator",
  "ownedProcesses": [],
  "protectedPorts": [5037, 5049],
  "ownedPorts": [5057, 5578, 5579],
  "avd": "SoloSoul_public_test"
}
```

`protectedPorts`固定保留5037／5049；`ownedPorts`显式列出本次全部端口，不能重复或与受保护端口重叠。AVD名称须以`SoloSoul_`开头。CLI不启动或连接ADB，不创建模拟器，不结束进程或改变窗口。

```powershell
python -B tauri/scripts/android-fresh-host-audit.py before `
  --powershell '<PowerShell 7的pwsh.exe>' `
  --expected '<before.json>' --output '<全新的before输出目录>'
```

只有实际快照中相关进程和全部声明监听均为空才通过；不能将已有服务接纳为空基线。保留输出中的`expected.json`、`host-snapshot.json`和`result.json`，不要按收尾时的现场重建运行前证据。

测试运行方在自有进程仍存活时冻结完整身份清单，形成`after.json`：其余字段与`before.json`一致，`ownedProcesses`必须非空，每项包含真实CIM的`Name`、`ProcessId`、`ParentProcessId`、带时区微秒`CreationDate`、绝对`ExecutablePath`和完整`CommandLine`。应保存全部已知进程，包括控制台及其他辅助进程；仅存根进程不能识别所有后来脱离父链的助手。

```powershell
python -B tauri/scripts/android-fresh-host-audit.py after `
  --powershell '<同一pwsh.exe>' --expected '<after.json>' `
  --baseline '<原before输出目录>' --output '<全新的after输出目录>'
```

收尾先重新校验原before三份文件的哈希及原空快照结果，核对AVD／端口范围相同；查询后再次检查before和期望身份原字节未变。任何ADB／模拟器、自有PID或传递后代、受保护或自有端口监听均拒绝；PID被复用仍拒绝，不结束新进程。

## 输出与期限

输出保存期望配置、原始两路快照输出和结果；after还绑定三份before文件哈希。控制台只返回计数、原因和哈希。原始文件可能包含本机身份，仅留本地。

退出码0表示通过，1表示核验／采集／证据保存失败，2表示输入、baseline或输出目录错误。输出目录须不存在。每次只执行一次二进制PowerShell采集，保留15秒期限；不重试，超时保留已有部分输出。结果只证明该次快照，不能替代运行期完整身份记录或设备内应用恢复。

```powershell
python -B -m unittest discover -s tauri/scripts -p test_android_host_audit.py -v
python -B -m unittest discover -s tauri/scripts -p test_android_fresh_host_audit.py -v
```

## 实际验证

旧18项及新增16项共34项通过、零跳过。当前Windows真实CLI完成五组对照：空宿主通过；含中文命令行的Python根／子进程和两个conhost共四个真实身份存活时拒绝，自然退出后同一期望通过；声明端口实际监听时拒绝，关闭后通过。所有after使用同一原before证据，未启动ADB、模拟器或应用，没有强制结束进程。

首个诊断脚本假定仅有两个进程，遇到Windows实际创建的conhost而提前拒绝，尚未运行owned-live CLI；该失败和自然退出记录保留。改为捕获完整后代并确认真实计时子进程后，新独立队列通过；CLI规则和源件不变。

见[去标识摘要](rf312-fresh-host-cli-2026-10-17.json)。本项只交付可复用宿主步骤；历史冻结配方不回写。完整行程其他路径、产物／夹具绑定及编排仍需参数化，重复分布、IPC、启动两帧和多端仍待验，不关闭RF-312。
