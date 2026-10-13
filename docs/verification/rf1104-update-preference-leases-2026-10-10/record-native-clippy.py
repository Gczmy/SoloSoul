from pathlib import Path
import json

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
receipt = json.loads((stage/'rust-clippy-native.receipt.json').read_text(encoding='utf-8'))
command = ['cargo','clippy','--locked','--features','native-perf','--all-targets','--','-D','warnings']
assert receipt['command'] == command
assert receipt['exitCode'] == 0 and receipt['sourceUnchanged']
expected = json.loads((stage/'regression-green.sources.json').read_text(encoding='utf-8'))
assert len(expected) == 1827
assert json.loads((stage/'rust-clippy-native.sources.json').read_text(encoding='utf-8')) == expected
title = '### RF-1104 非默认 Windows 严格 Clippy（2026-10-10）'
data = report.read_bytes()
assert title.encode('utf-8') not in data
entry = '\n\n'+title+'\n\n'
entry += '- cargo clippy --locked --features native-perf --all-targets -- -D warnings 已取得 exit 0，覆盖非默认诊断功能及全部目标；全部 1827 个冻结输入前后一致，与定向绿测及默认完整 R 使用同一源码。\n'
entry += '- 继续非默认 native_perf、RF-905 与完整偏好回归，随后才运行新 GUI 构建和100/5000各三次完整原生行程。本项仍进行中，284/294 和其他前置不变；尚未提交或推送，goal active。\n'
report.write_bytes(data+entry.replace('\n','\r\n').encode('utf-8'))
print(json.dumps({'task':'RF-1104','nativeClippyExitCode':receipt['exitCode'],
    'sourceUnchanged':receipt['sourceUnchanged'],'taskClosed':False}))
