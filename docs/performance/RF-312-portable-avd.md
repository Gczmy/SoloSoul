# RF-312：显式 SDK/JDK 的新 AVD 准备入口

[prepare_avd.py](rf312-default-observer/prepare_avd.py)只创建本次独占的新 API31 x86_64 AVD，不启动模拟器、ADB或应用。它替代历史配方中固定的SDK、JDK、旧AVD配置路径；后续机器启动、预检及终态编排仍待参数化，完整RF-312保持待验。

准备已安装的Windows SDK、JDK、完整`emulator`包及`system-images;android-31;default;x86_64`。本入口不安装依赖。显式配置如下，SHA须来自独立核验过的工具：

```json
{
  "schemaVersion": 1,
  "sdkRoot": "<当前Android SDK>",
  "jdkRoot": "<当前JDK>",
  "avdName": "SoloSoul_public_test",
  "toolSha256": {
    "avdManager": "<sdk/cmdline-tools/latest/bin/avdmanager.bat的SHA-256>",
    "java": "<jdk/bin/java.exe的SHA-256>",
    "classpath": "<sdk/cmdline-tools/latest/lib/avdmanager-classpath.jar的SHA-256>",
    "imageProperties": "<sdk/system-images/android-31/default/x86_64/source.properties的SHA-256>",
    "emulatorPackage": "<sdk/emulator/package.xml的SHA-256>",
    "emulatorExecutable": "<sdk/emulator/emulator.exe的SHA-256>"
  }
}
```

```powershell
python -B docs/performance/rf312-default-observer/prepare_avd.py `
  --config '<本机配置.json>' --output '<全新的输出目录>'
```

输出父目录须已存在且是普通目录。AVD名以`SoloSoul_`开头；SDK、JDK、输出及其父目录不允许reparse。支持空格；所有传给SDK批处理／JDK的路径必须通过本机Windows代码页往返校验，不能表示的字符在启动SDK前拒绝。本轮en-GB环境中的中文路径不能通过该检查，不能宣称已支持任意Unicode路径。路径中的`% ! ^ & | < >`、引号、换行和空字符同样直接拒绝。命令使用显式系统`cmd.exe`、`/d /s /c`及每个路径的引号，不依赖PATH中的同名程序。

SDK原`create avd`、`pixel_2`、`no`标准输入和120秒预算保持。Java／Android用户目录及临时目录仅在本次输出内，环境只传给子进程。继承的Java附加选项和跳过版本检查开关清空。原SDK/JDK与现有AVD不被迁移、覆盖或清理。

SDK成功生成后，原始配置另存；应用[原样硬件配置](rf312-default-observer/api31-hardware.ini)，SHA为`b6268289e372b91822d745867c06e4ee3c866e295be4202ae65dd3945131e448`，与此前验收配置逐字一致。配置内4核、2G不变；后续启动仍须使用原2核、2048MiB命令覆盖并在设备内验证，不能把准备成功当作运行环境验收。

输出包含`config.json`、实际`command.json`、完整SDK标准输出／错误、SDK原始配置及`avd-preparation.json`。失败输出保留，不重试或改写成通过；已有输出目录拒绝。准备期间配置、入口源、硬件配置和声明的六份工具身份文件改变均拒绝。

六个SHA绑定直接使用的入口／身份文件，不表示整个SDK/JDK依赖闭包已签名。实际迁移验证另对复制工具树做逐文件完整回读，二者的证据范围分别记录。准备成功也不代表完整界面、性能分布、IPC、启动帧或其他平台通过。

回归命令：

```powershell
python -B -m unittest discover -s docs/performance/rf312-default-observer -p test_prepare_avd.py -v
```


## 本轮实际结果（2026-10-17）

新SDK、JDK及输出均处于英文空格路径，原SDK批处理真实创建新AVD成功；SDK生成配置与原硬件配置的SHA均为上述`b626…1448`。最终工具树1,040份复制文件在创建后再次逐字节回读，原十二步运行入口的20份源件未改。18项唯一回归、零跳过；五次实际CLI负向和一次真实junction父目录负向通过，链接移除、目标哨兵保留。

保留三次原生失败：中文JDK安装路径导致DLL加载失败，SDK批处理返回0但无配置仍被拒绝；中文SDK类路径导致类加载失败；不完整SDK缺少emulator包。相同JDK字节在英文空格路径及原安装位置正常。后续入口增加本机代码页拒绝和emulator包身份预检，未跳过Java版本保护。首次单元配置复制位置错误及沙箱拒绝junction创建也保留；后者仅在已授权隔离路径中重新创建并清理。

原SDK的`devices.xml`缺失提示在此前成功配方中也存在，本次原始错误流完整保留。验收依据包含SDK退出码、真实生成的普通配置文件、原始／应用配置哈希、工具前后身份及全部失败证据；不把退出0单独当作准备通过。

本轮未启动模拟器、ADB或应用，未使用APK／Vault，不产生新的界面或性能样本。实际2核／2048MiB覆盖、启动健康检查及整套外层编排继续待验。见[去标识摘要](rf312-portable-avd-2026-10-17.json)。
