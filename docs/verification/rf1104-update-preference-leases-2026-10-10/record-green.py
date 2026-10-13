from pathlib import Path
import json, re

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
receipt = json.loads((stage/'regression-green.receipt.json').read_text())
format_receipt = json.loads((stage/'rust-format.receipt.json').read_text())
assert receipt['exitCode'] == format_receipt['exitCode'] == 0
assert receipt['sourceUnchanged'] and format_receipt['sourceUnchanged']
output = (stage/'regression-green.stdout.log').read_text(encoding='utf-8')
assert '15 passed; 0 failed; 0 ignored;' in output
names = re.findall(r'^test commands::update_preferences::tests::(\w+) \.\.\. ok$', output, re.M)
assert len(set(names)) == 15
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
text = report.read_text(encoding='utf-8')
assert '### RF-1104 定向红绿验证' not in text
text += '''

### RF-1104 定向红绿验证（2026-10-10）

- 真正红测 0 passed / 1 failed（IMPORT_OPERATIONS_ACTIVE，exit 101）后，修复后的完整更新源偏好回归实际 15 passed / 0 failed / 0 ignored，exit 0；既有选源、缓存与持久化用例均保留。原 Store 强引用仍在的同账户重解锁可取得维护许可，网络结果返回后不写旧账户或公共缓存；实际文件写入被文件锁延迟时，维护仍拒绝准入，写完才恢复。
- 新增账户切换的逐字节与版本比较、原目录仍存活的 root 身份校验、同路径新 owner、服务释放后的真实目录重开和并发双通道合并通过。格式检查 exit 0，全部 1827 个冻结源码文件前后 SHA 一致；默认严格 Clippy 与后续必需 Rust 检查已顺序启动。
- 本记录只证明定向回归与格式，不冒充完整 Rust 或新 GUI 构建/原生行程验收。RF-1104 继续 [~]，其余状态和计数保持；尚未提交或推送，goal active。
'''
report.write_bytes(text.replace('\n','\r\n').encode('utf-8'))
print('Recorded 15 actual passing regressions and mandatory checks still pending')
