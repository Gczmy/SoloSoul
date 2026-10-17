# RF-312 Android 插桩状态的有效来源

## 实际问题

API31 的 `dumpsys activity instrumentation` 实际返回退出码 0，但正文是 `Unknown command: instrumentation`。旧诊断代码无法取得期望的组件；旧默认行程及内存工具仅判断回复中没有应用包名，也不能据此证明插桩缺席。

逐字节回放既有默认包五轮中的 10 个内存检查点，前后共 20 次状态查询均为该错误正文。原始数值、回执和失败保留；没有将这些回复补写成有效身份状态。

## 新读取器

`rf312_instrumentation_state.py` 只查询无包名过滤的 `dumpsys activity processes`。Android 12 的 [命令分派](https://github.com/aosp-mirror/platform_frameworks_base/blob/android12-release/services/core/java/com/android/server/am/ActivityManagerService.java#L8323)、[进程与插桩区段顺序](https://github.com/aosp-mirror/platform_frameworks_base/blob/android12-release/services/core/java/com/android/server/am/ProcessList.java#L3780)及[插桩记录格式](https://github.com/aosp-mirror/platform_frameworks_base/blob/android-12.0.0_r1/services/core/java/com/android/server/am/ActiveInstrumentation.java#L99)提供了对应的系统来源。

读取器要求完整进程标题、已知进程区段及位于插桩区段之后的 OOM 边界。每条记录的头部、组件、未完成状态、单一目标应用／进程和实际运行 PID 必须一致；重复、缺失、通配目标、已结束会话、未知错误和截断均拒绝。只有完整边界之间确实没有插桩区段，才返回空列表。声明的组件集合必须与实际列表完全一致，测量期间还绑定原目标 PID；没有把错误正文或不存在的字段转换成缺席。

```python
from rf312_instrumentation_state import capture, expected

# sdk(argv, timeout=...) 返回实际退出码、原始 stdout 和 stderr；
# record 在解析前保存原始输出，不修改 SDK 的回复。
rows, stdout_sha256 = capture(sdk, phase_deadline, record)
proof = expected(rows, component_to_target, process_ids=bound_pids)
```

SDK 查询及本地记录完成后仍检查原阶段期限，未知结果不重发界面输入。原严格包读取器、默认观察器、标记保护、源文件及既有测量回执不改。本轮独立诊断适配器在开始前检查全局为空、启动采集器前只允许自身 UI 观察器、实际操作期间只允许两个明确组件、清理后再次核实缺席。旧默认工具尚需同样接入有效来源，已有回复不算直接缺席证据。

```powershell
python -B -m unittest discover -s docs/performance -p test_rf312_instrumentation_state.py -v
```

## 实际验证

15 项读取器回归和 35 项诊断适配回归通过，零跳过。原不支持命令的实际控制回复及全部 151 帧原始传输已回放，新读取器拒绝；历史 20 次错误回复也全部拒绝。公开采集器 83 份来源保持，独立适配共绑定 96 份来源；未重新编译或替换产品库，采集器的可选堆栈参数缺省关闭。

新 API31／x86_64、2 CPU／2048 MiB、SwiftShader 隔离环境完成固定顺序 100→5000 的全部十二阶段。每档包括启动、密码解锁、首次 OCR、工作区、搜索、附件、图片、文本、两类原生 PDF、锁定及重新解锁。实际累计传输尝试分别为 235 和 240；这是早期观察器记录的发送尝试，不是成功完成的后端命令数。

171／172 次有效系统状态读取逐项回放，确认空状态→独立观察器→两个指定组件→独立观察器→空状态，观察器原生 PID 稳定，目标 PID 与原生握手一致。24 个阶段元数据及请求／响应摘要、两档私有数据内容／链接／权限和原 APK／UID／设置恢复、各三类会话清理均通过。每档只清理精确核验的 50 个元数据文件，没有堆栈文件。正常关闭 SDK 后，原 45 秒内第 7 次只读严格审计归零，原后验离线重放通过。

本轮接受的是明确插桩诊断包中的完整界面行程与实际 IPC 元数据；每档只有一次功能运行，没有接受默认性能分布或同源观察开销。旧阶段耗时与本轮还包含不同状态读取开销，不能相减推导应用本体开销。本轮无 ANR，此前 FocusEvent 超时的根因仍未证明。

下一步补齐默认包有效状态接入，再完成同源观察开销、应用本体时延、全部归属进程及全程／峰值内存、启动两帧与多端。RF-312 保持未完成，365/374 不变。见[去标识摘要](rf312-instrumentation-state-2026-10-17.json)；原始系统回复、进程标识、截图、APK、私有数据及本机路径仅留本地。


后续已完成[默认包有效状态入口与旧 PSS 边界补证](RF-312-default-supported-state.md)：在原严格内存读取器内部另存的 20 份有效进程转储中核实了缺席，原不支持回复仍无效。本文件对应的初次发现摘要保持原样。
