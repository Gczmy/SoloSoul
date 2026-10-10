from pathlib import Path
import json
import re

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
receipt = json.loads((stage/'rust-tests-default.receipt.json').read_text(encoding='utf-8'))
assert receipt['exitCode'] == 0 and receipt['sourceUnchanged']
assert receipt['command'] == ['cargo','test','--locked','--verbose']
expected = json.loads((stage/'regression-green.sources.json').read_text(encoding='utf-8'))
assert len(expected) == 1827
assert json.loads((stage/'rust-tests-default.sources.json').read_text(encoding='utf-8')) == expected
text = (stage/'rust-tests-default.stdout.log').read_text(encoding='utf-8')
rows = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
counts = {key:sum(int(row[i]) for row in rows) for i,key in enumerate(['passed','failed','ignored'])}
assert counts == {'passed':1871,'failed':0,'ignored':3}
ignored = sorted(re.findall(r'^test (.+?) \.\.\. ignored',text,re.M))
baseline = (root/'build/rf312-maintenance-20261009/rust-tests-default-dtemp.stdout.log').read_text(encoding='utf-8')
assert ignored == sorted(re.findall(r'^test (.+?) \.\.\. ignored',baseline,re.M))
assert len(re.findall(r'^test commands::update_preferences::tests::\w+ \.\.\. ok$',text,re.M)) == 15
title = '### RF-1104 默认完整 Rust 验收（2026-10-10）'
data = report.read_bytes()
assert title.encode('utf-8') not in data
entry = '\n\n'+title+'\n\n'
entry += '- 默认 workspace cargo test --locked --verbose 实际 1871 passed / 0 failed / 3 既有 ignored，exit 0；覆盖主应用、集成、Core/Crypto/Plugin/Sync/Vault 和文档测试，15 项偏好回归均在完整运行中通过。全部 1827 个冻结源码文件前后保持一致，完整命令耗时 3557.39 秒。\n'
entry += '- 三个既有跳过项与原基线名称一致：field::tests::test_field_metadata、field::tests::test_resolve_legacy_unchanged、p025_hold_baseline_large_dataset；未新增跳过，也未因慢测试重启或减少覆盖。\n'
entry += '- 非默认 Windows 严格 Clippy、诊断/许可/偏好回归与新构建六次原生行程仍待完整验收。本项继续 [~]，284/294 和其他前置保持；尚未提交或推送，goal active。\n'
report.write_bytes(data+entry.replace('\n','\r\n').encode('utf-8'))
print(json.dumps({'task':'RF-1104','defaultRust':counts,'exitCode':receipt['exitCode'],
    'sourceUnchanged':receipt['sourceUnchanged'],'taskClosed':False}))
