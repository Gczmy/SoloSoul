"""Verify exactly one RF-1123 change and all archived members at index or HEAD."""
from pathlib import Path
import hashlib, io, json, subprocess, sys, tarfile

r = Path('D:/SoloSoul')
mode = sys.argv[1]
assert mode in ['staged', 'head']
base = '0e7059c8ee8df5856c049be33e161728e3f5404d'
stem = 'docs/verification/rf1123-preference-write-attribution-2026-10-11'
code = {
    'tauri/scripts/run-node-tests.mjs',
    'tauri/scripts/native-perf-preference-contract.mjs',
    'tauri/scripts/native-perf-preference.node.test.mjs',
    'tauri/src-tauri/src/commands/settings.rs',
    'tauri/src-tauri/src/native_perf.rs',
    'tauri/src-tauri/src/native_perf/sdk_surface_appearance.rs',
    'tauri/src-tauri/src/native_perf/preference_trace.rs',
}
expected = code | {
    'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md',
    'docs/performance/RF-121-preference-write-attribution.md',
    stem + '.json', stem + '/.gitattributes', stem + '/index.json', stem + '/raw.tar.gz',
    stem + '/git-verification.py', stem + '/git-verification-first-crlf-failure.json',
}
def git(*args):
    return subprocess.check_output(['git', *args], cwd=r)
def blob(path):
    return git('show', (':' if mode == 'staged' else 'HEAD:') + path)
def load(path):
    return json.loads(blob(path).decode('utf-8-sig'))
if mode == 'staged':
    assert git('rev-parse', 'HEAD').decode().strip() == base
    changed = git('diff', '--cached', '--name-only', '--ignore-submodules', base, '--').decode().splitlines()
else:
    assert git('rev-parse', 'HEAD^').decode().strip() == base
    assert 'RF-1123' in git('show', '-s', '--format=%s', 'HEAD').decode()
    changed = git('diff', '--name-only', '--ignore-submodules', base, 'HEAD', '--').decode().splitlines()
assert set(changed) == expected and len(changed) == 15, changed
record = load(stem + '.json')
assert record['task'] == 'RF-1123' and record['status'] == 'verification-complete'
assert record['base'] == base and record['sourceCount'] == 1837
checks = record['checks']
assert checks['success'] and checks['nativeTests'] == 136
assert checks['workspace']['passed'] == 1874 and checks['workspace']['failed'] == 0
assert checks['workspace']['ignored'] == 3 and checks['node']['pass'] == 236
assert checks['node']['fail'] == 0 and checks['node']['skipped'] == 1
assert record['originalFailuresRetained']
assert record['queue']['rf1123DiagnosticEligible'] and len(record['groups']) == 4
assert not record['fullRF121Accepted'] and not record['fullRF312Accepted']
ledger = blob('docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md').decode('utf-8-sig')
assert '已关闭：**303 / 313**；实际修复（已关闭）：303；' in ledger
assert '暂缓：0；进行中：0；待执行：6。' in ledger
assert '| 无（RF-121额外写入来源未明） | [x] 完成 |' in ledger
index = load(stem + '/index.json')
raw = blob(stem + '/raw.tar.gz')
digest = hashlib.sha256(raw).hexdigest()
assert digest == index['archiveSha256'] == record['archive']['sha256']
assert len(index['members']) == record['archive']['members']
assert blob(stem + '/.gitattributes').replace(b'\r\n', b'\n') == b'* -text -diff\n'
with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
    assert len(archive.getmembers()) == len(index['members'])
    assert set(archive.getnames()) == {item['path'] for item in index['members']}
    for item in index['members']:
        member = archive.getmember(item['path'])
        assert member.isfile() and not member.issym() and not member.islnk()
        data = archive.extractfile(member).read()
        assert len(data) == item['size'] and hashlib.sha256(data).hexdigest() == item['sha256']
    for path in code:
        archived = archive.extractfile('final-sources/' + path).read()
        assert archived.replace(b'\r\n', b'\n') == blob(path).replace(b'\r\n', b'\n')
print(json.dumps(dict(mode=mode, paths=15, members=len(index['members']),
    archiveSha256=digest, sourceCount=1837, allMembersVerified=True,
    fullRF121Accepted=False, fullRF312Accepted=False)))
