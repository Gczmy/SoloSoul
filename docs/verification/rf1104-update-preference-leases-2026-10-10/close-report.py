"""Close only RF-1104 after verifying the complete portable evidence archive."""
from pathlib import Path
import gzip
import hashlib
import json
import re

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
index_path = root/'docs/verification/rf1104-update-preference-leases-2026-10-10.json'
index = json.loads(index_path.read_text(encoding='utf-8'))
assert index['task'] == 'RF-1104' and index['validationAccepted'] is True
assert index['native']['accepted'] and len(index['native']['samples']) == 6
assert index['rf312TaskClosed'] is False
for record in index['records']:
    path = root/'docs'/record['path']
    assert path.resolve().is_relative_to((root/'docs/verification').resolve())
    assert path.is_file() and not path.is_symlink()
    stored = path.read_bytes()
    assert len(stored) == record['storedBytes']
    assert hashlib.sha256(stored).hexdigest() == record['storedSha256']
    raw = gzip.decompress(stored) if path.suffix == '.gz' else stored
    assert len(raw) == record['originalBytes']
    assert hashlib.sha256(raw).hexdigest() == record['originalSha256']
expected = json.loads((stage/'regression-green.sources.json').read_text(encoding='utf-8'))
for name, sha in expected.items():
    assert hashlib.sha256((root/name).read_bytes()).hexdigest() == sha, name
data = report.read_bytes()
assert b'\r\n' in data
text = data.decode('utf-8').replace('\r\n', '\n')
rows = lambda value: re.findall(r'^\| (\d+) \| \[(RF-\d+)\]\(#rf-\d+\) \|.*$', value, re.M)
cards = re.findall(r'^### (RF-\d+)\s*$', text, re.M)
assert len(rows(text)) == len(cards) == len(set(cards)) == 294
assert set(item[1] for item in rows(text)) == set(cards)
before = {item[1]:re.search(r'^\| '+item[0]+r' \|.*$', text, re.M).group(0) for item in rows(text)}
assert '[~] 进行中' in before['RF-1104']
text = text.replace('RF-1104 更新源偏好许可范围修复进行中', 'RF-1104 更新源偏好许可范围修复验收完成', 1)
assert text.count('已关闭：**284 / 294**；实际修复（已关闭）：284；排除：0；待验证/阻塞：3（RF-112、RF-121、RF-312）；暂缓：0；进行中：1；待执行：6。') == 1
text = text.replace('已关闭：**284 / 294**；实际修复（已关闭）：284；排除：0；待验证/阻塞：3（RF-112、RF-121、RF-312）；暂缓：0；进行中：1；待执行：6。',
    '已关闭：**285 / 294**；实际修复（已关闭）：285；排除：0；待验证/阻塞：3（RF-112、RF-121、RF-312）；暂缓：0；进行中：0；待执行：6。', 1)
current = '- 当前处理：**RF-1104：网络选源不持有目录许可，结果返回后验证原目录与会话并保护实际写入。RF-112/121/312 与 RF-122～127 前置保持**。'
assert text.count(current) == 1
text = text.replace(current, '- 当前处理：**RF-1104 已验收，下一步继续 RF-312 Windows 首页超时归因与 RF-121 原生材质矩阵；RF-112/121/312 与 RF-122～127 前置保持**。', 1)
text = text.replace(before['RF-1104'], before['RF-1104'].replace('[~] 进行中', '[x] 完成'), 1)
for task, row in before.items():
    if task != 'RF-1104':
        assert row in text, 'Unexpected task state change: '+task
assert '### RF-1104 完整验收' not in text
rust = index['checks']
totals = lambda name: rust[name]['testTotals']
default = totals('rust-tests-default')
native = totals('rust-tests-native')
rf905 = totals('rust-tests-rf905')
prefs = totals('rust-tests-preferences-native')
text += '\n\n### RF-1104 完整验收（2026-10-10）\n\n'
text += '- 网络等待仅保留弱身份快照；同步读取与结果持久化分别取得实际目录许可。迟到结果核对原 owner、Store、账户与会话代次，写入仍由实际许可和会话门闩保护；Core 互斥、SDK 预算及真实 worker 生命周期保持。\n'
text += f'- 真实红测 0 passed / 1 failed（IMPORT_OPERATIONS_ACTIVE，exit 101）；修复后偏好回归 15 passed。默认 workspace {default["passed"]} passed / {default["failed"]} failed / {default["ignored"]} 既有 ignored；非默认诊断 {native["passed"]} passed、RF-905 {rf905["passed"]} passed、非默认偏好 {prefs["passed"]} passed。格式、默认与非默认严格 Clippy 全部 exit 0；1827 个冻结源码输入前后一致。初次测试编译错误不作为缺陷红测。\n'
build = rust['native-release-build']['receipt']
if not build['sourceUnchanged']:
    text += '- 新 GUI Release 构建 exit 0；Tauri CLI 仅改写 Cargo.toml 换行，原始 sourceUnchanged=false 收据保留。规范化字节与解析 TOML 均一致后只恢复原换行，完整源码冻结重新核验通过。\n'
else:
    text += '- 新 GUI Release 构建 exit 0，源码冻结保持；新可执行文件与旧构建不同，94 个运行资源逐字节核验一致。\n'
text += '- 新构建使用原公开夹具，100/5000 对象各三次完整原生行程全部通过；六个独立运行的两次认证流程均结束、维护准入失败均为零，全部样本和原始合同文档保留。诊断不是未插桩性能基线，也不证明旧首页超时根因；RF-312 整项保持待验证。\n'
text += '- [结构化验收与逐字节原件索引](verification/rf1104-update-preference-leases-2026-10-10.json)包含红绿、完整 Rust 检查、新构建与六次原生行程、冻结输入及复核脚本；不归档 Vault、应用缓存、模型或可执行文件。所有归档原始/存储字节与 SHA256 均回读验证。\n'
text += '- 本项按一项一提交独立本地提交（本提交，以 RF-1104 检索），不推送。累计 285/294 关闭、3 待验证、6 待执行；RF-112/121/312 与 RF-122～127 前置保持，整体 goal 继续 active。\n'
report.write_bytes(text.replace('\n', '\r\n').encode('utf-8'))
print(json.dumps({'task':'RF-1104','closed':285,'total':294,'otherTaskRowsUnchanged':True,
    'archiveRecordsVerified':len(index['records'])}))
