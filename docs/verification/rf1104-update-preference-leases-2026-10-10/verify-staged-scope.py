"""Verify the precise RF-1104 staged scope and the actual Git evidence bytes."""
from pathlib import Path
import datetime
import gzip
import hashlib
import json
import re
import subprocess

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
source_path = 'tauri/src-tauri/src/commands/update_preferences.rs'
report_path = 'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
index_path = 'docs/verification/rf1104-update-preference-leases-2026-10-10.json'
archive_prefix = 'docs/verification/rf1104-update-preference-leases-2026-10-10/'

def git(*arguments, data=None):
    return subprocess.run(['git',*arguments],cwd=root,input=data,
        stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True).stdout

def sha(data):
    return hashlib.sha256(data).hexdigest()

index = json.loads((root/index_path).read_text(encoding='utf-8'))
assert git('rev-parse','HEAD').decode().strip() == index['baseHead']
assert git('branch','--show-current').decode().strip() == 'codex/rf1104-update-preference-leases'
assert index['task'] == 'RF-1104' and index['validationAccepted'] is True
assert index['native']['accepted'] is True and index['native']['sampleCount'] == 6
assert index['rf312TaskClosed'] is False
expected = json.loads((stage/'regression-green.sources.json').read_text(encoding='utf-8'))
assert sha((stage/'regression-green.sources.json').read_bytes()) == index['sourceManifestSha256']
assert index['sourceFileCount'] == len(expected) == 1827
for path, digest in expected.items():
    assert sha((root/path).read_bytes()) == digest, path

evidence = {}
for record in index['records']:
    path = 'docs/'+record['path']
    assert path.startswith(archive_prefix)
    file = root/path
    assert file.resolve().is_relative_to((root/archive_prefix).resolve())
    assert file.is_file() and not file.is_symlink()
    raw = file.read_bytes()
    assert len(raw) == record['storedBytes'] and sha(raw) == record['storedSha256']
    recovered = gzip.decompress(raw) if file.suffix == '.gz' else raw
    assert len(recovered) == record['originalBytes'] and sha(recovered) == record['originalSha256']
    assert path not in evidence
    evidence[path] = raw
allowed = {source_path,report_path,index_path,*evidence}
staged = {item.decode('utf-8') for item in git('diff','--cached','--name-only','-z').split(b'\0') if item}
assert staged == allowed, {'unexpected':sorted(staged-allowed),'missing':sorted(allowed-staged)}
status = git('status','--porcelain','-z','--untracked-files=all','--ignore-submodules=none')
for row in status.split(b'\0'):
    if not row:
        continue
    assert row[:2] in (b'M ',b'A '), 'Unstaged/renamed/deleted/unrelated change: '+repr(row)
    assert row[3:].decode('utf-8') in allowed, repr(row)
git('diff','--cached','--check')

attribute_input = b'\0'.join(path.encode('utf-8') for path in sorted(evidence))+b'\0'
attributes = git('check-attr','--cached','-z','text','--stdin',data=attribute_input).split(b'\0')
assert attributes.pop() == b'' and len(attributes) == len(evidence)*3
seen = set()
for offset in range(0,len(attributes),3):
    path, name, value = attributes[offset:offset+3]
    assert name == b'text' and value == b'unset', (path,name,value)
    seen.add(path.decode('utf-8'))
assert seen == set(evidence)
for path, raw in evidence.items():
    assert git('show',':'+path) == raw, 'Git altered evidence bytes: '+path
assert git('show',':'+index_path) == (root/index_path).read_bytes()
for path in (source_path,report_path):
    assert git('show',':'+path).replace(b'\r\n',b'\n') == (root/path).read_bytes().replace(b'\r\n',b'\n'), path

report = (root/report_path).read_text(encoding='utf-8')
rows = re.findall(r'^\| \d+ \| \[(RF-\d+)\]\(#rf-\d+\) \|.*?\| \[([x!~ ])\][^\n]*$',report,re.M)
cards = re.findall(r'^### (RF-\d+)\s*$',report,re.M)
assert len(rows) == len(cards) == len(set(cards)) == 294
assert {task for task,_ in rows} == set(cards)
states = dict(rows)
assert len(states) == 294 and sum(state == 'x' for state in states.values()) == 285
assert states['RF-1104'] == 'x'
pending = {task:state for task,state in states.items() if state != 'x'}
assert pending == {**{task:'!' for task in ('RF-112','RF-121','RF-312')},
    **{f'RF-{number}':' ' for number in range(122,128)}}
proof = {
    'task':'RF-1104','scope':'Actual staged Git objects and unchanged source inputs before the local commit',
    'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'baseHead':index['baseHead'],'sourceFileCount':len(expected),'stagedFileCount':len(staged),
    'rawEvidenceRecordsVerified':len(evidence),'gitEvidenceBytesEqual':True,
    'evidenceTextConversionDisabled':True,'onlyTaskFilesStaged':True,'pendingTasks':pending,
    'stagedObjectSha256':{path:sha(git('show',':'+path)) for path in sorted(staged)},
    'localCommitStillRequired':True,
}
with (stage/'staged-evidence-verification.json').open('x',encoding='utf-8',newline='\n') as output:
    output.write(json.dumps(proof,indent=2)+'\n')
print(json.dumps({key:value for key,value in proof.items() if key != 'stagedObjectSha256'}))
