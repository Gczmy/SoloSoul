from pathlib import Path
import json

stage = Path(__file__).parent
report = Path('D:/SoloSoul/docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md')
for name in ('rust-format', 'rust-clippy-default'):
    receipt = json.loads((stage/(name+'.receipt.json')).read_text(encoding='utf-8'))
    assert receipt['exitCode'] == 0 and receipt['sourceUnchanged'], name
output = (stage/'rust-tests-default.stdout.log').read_text(encoding='utf-8')
assert 'test result: ok. 790 passed; 0 failed; 0 ignored;' in output
assert not (stage/'rust-tests-default.receipt.json').exists(), 'Use final results when available'
title = '### RF-1104 完整检查进度与收尾设施（2026-10-10）'
data = report.read_bytes()
assert title.encode('utf-8') not in data
entry = '\n\n'+title+'\n\n'
entry += '- 格式与默认严格 Clippy 已取得 exit 0，全部冻结输入前后一致。默认 workspace 主应用库 790 passed / 0 failed / 0 ignored，尚无整个工作区终态；当前实际 Cargo 子进程正在执行 solosoul_core 的 446 项测试，不能把主应用库通过记为完整 R 通过。\n'
entry += '- 准备仅在所有必需检查与六次原生行程验收通过后执行的归档、报告关闭脚本；脚本语法检查通过。归档须保留初次编译错误、真实红测、新构建及全部原生样本，逐字节回读核对，且不收录 Vault、缓存、模型或二进制。此时尚未生成最终验收归档或关闭本项。\n'
entry += '- 现有测试作业保持运行，源码不变；不重复启动、不改变测试预算或跳过用例。RF-1104 仍进行中，284/294 与其余前置保持；尚未提交或推送，goal active。\n'
report.write_bytes(data+entry.replace('\n', '\r\n').encode('utf-8'))
print('Recorded verified RF-1104 check progress without closing the task')
