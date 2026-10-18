# RF-312：物理OCR使用显式证据根目录

2026-10-17，将本机配方中的物理读取组件放入[观察器目录](rf312-default-observer/README.md)，解除原脚本只接受固定构建目录的问题。Windows.Media.Ocr的en-GB引擎、原图前后哈希、原生异步操作及期限不变；[计数读取器](rf312-default-observer/public_pixel_counter.py)和[完整内容断言](rf312-default-observer/native_journey_contract.py)原字节保持，不推断或替换数字。

## 使用

需要Windows系统PowerShell 5.1及可用的en-GB原生识别器；Python调用还需要Pillow。证据根目录须已存在，包含本次公开图片和输出目录，目录及所有父级不得是symlink／junction等reparse point。

```powershell
& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" `
  -NoProfile -NonInteractive `
  -File docs/performance/rf312-default-observer/read-public-counter.ps1 `
  -ImagePath '<证据根目录内的公开PNG>' `
  -OutputPath '<同一根目录内的新JSON>' `
  -EvidenceRoot '<已存在的本次证据根目录>'
```

`EvidenceRoot`为必填参数，没有历史盘符或目录回退。检查使用完整目录边界，名称前缀相同的相邻目录不属于该根。图片必须是普通文件；输出必须不存在，父目录须已存在且普通。原生写出使用`CreateNew`，不覆盖已有文件。JSON保留原引擎、语言、原图哈希、尺寸、原文和词坐标。

Python的`NativeOcrReader(bindings, evidence_root)`也必须显式传根目录；在运行系统识别器前检查输入／输出归属、普通路径和源件绑定。系统PowerShell及模块目录从`SystemRoot`派生，不依赖父进程的`PSModulePath`；缺失SystemRoot直接拒绝。执行前重算原阶段余量，单次进程不超过10秒及原余量；超时、非零、迟到或源件改变均不当作有效结果，没有重试。

`default_journey.Journey`现在将自己的实际输出目录传给读取器；其余十二步动作、三次／350ms就绪、45／240秒及512请求上限不变。调用方仍需重新冻结新源件和哈希；历史队列及其输入清单不回写。

```powershell
python -B -m unittest discover `
  -s docs/performance/rf312-default-observer -p test_ocr_evidence_root.py -v
```

## 验证及范围

14项回归通过、零跳过。原固定前缀之外、带空格和中文的新证据根实际完成三次系统OCR：公开图片的hello／solosoul／1234567890，以及保留的100／5000对象首页截图。三次原图哈希和精确文字／计数断言均通过。两张首页图是既有截图的新位置重读，不是新界面行程或性能样本。

另八次直接原生负向调用均拒绝：缺失根、根外图片、相同文本前缀的兄弟目录、根外输出、已有输出、图片父级junction、输出父级junction和根自身junction。Python预检也实际拒绝三种junction路径，未启动识别器；已有输出字节保持。测试自建junction只删除链接，原公共目标目录／图片保留。未运行ADB或操作应用窗口。

见[去标识记录](rf312-portable-ocr-2026-10-17.json)。本项只验收物理读取组件的目录参数化；SDK／工作区／APK／夹具的完整编排、重复分布、IPC、启动两帧和多端仍待验，RF-312保持未完成。
