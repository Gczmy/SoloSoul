from pathlib import Path
import json, hashlib, re

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
expected = json.loads((stage/'regression-green.sources.json').read_text())
checks = [
    'regression-green', 'rust-format', 'rust-clippy-default', 'rust-tests-default',
    'rust-clippy-native', 'rust-tests-native', 'rust-tests-rf905', 'rust-tests-preferences-native',
]
results = {}
commands = {
    'regression-green':['cargo','test','--locked','-p','solo_soul','--lib','commands::update_preferences::tests','--','--nocapture'],
    'rust-format':['cargo','fmt','--check'],
    'rust-clippy-default':['cargo','clippy','--locked','--','-D','warnings'],
    'rust-tests-default':['cargo','test','--locked','--verbose'],
    'rust-clippy-native':['cargo','clippy','--locked','--features','native-perf','--all-targets','--','-D','warnings'],
    'rust-tests-native':['cargo','test','--locked','-p','solo_soul','--lib','--features','native-perf','native_perf::','--','--nocapture'],
    'rust-tests-rf905':['cargo','test','--locked','-p','solo_soul','--lib','--features','native-perf','rf905','--','--nocapture'],
    'rust-tests-preferences-native':['cargo','test','--locked','-p','solo_soul','--lib','--features','native-perf','commands::update_preferences::tests','--','--nocapture'],
}
for name in checks:
    receipt = json.loads((stage/(name+'.receipt.json')).read_text())
    assert receipt['exitCode'] == 0 and receipt['sourceUnchanged'], name
    assert receipt['command'] == commands[name], name+' wrong verification scope'
    assert json.loads((stage/(name+'.sources.json')).read_text()) == expected, name+' changed input'
    output = (stage/(name+'.stdout.log')).read_text(encoding='utf-8', errors='replace')
    totals = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', output, re.M)
    summary = {key:sum(int(row[i]) for row in totals) for i,key in enumerate(['passed','failed','ignored'])}
    assert summary['failed'] == 0
    if 'tests-' in name or name == 'regression-green':
        assert totals, name+' no actual tests'
    results[name] = {'receipt':receipt,'testTotals':summary if totals else None}
assert results['regression-green']['testTotals'] == {'passed':15,'failed':0,'ignored':0}
assert results['rust-tests-preferences-native']['testTotals'] == {'passed':15,'failed':0,'ignored':0}
assert results['rust-tests-rf905']['testTotals'] == {'passed':29,'failed':0,'ignored':0}
assert results['rust-tests-native']['testTotals'] == {'passed':113,'failed':0,'ignored':0}
assert results['rust-tests-default']['testTotals'] == {'passed':1871,'failed':0,'ignored':3}
names = lambda output: set(re.findall(r'^test commands::update_preferences::tests::(\w+) \.\.\. ok$', output, re.M))
expected_names = names((stage/'regression-green.stdout.log').read_text(encoding='utf-8'))
assert len(expected_names) == 15
for name in ['rust-tests-default', 'rust-tests-preferences-native']:
    assert names((stage/(name+'.stdout.log')).read_text(encoding='utf-8')) == expected_names, name+' missing preference regression'
ignored = lambda output: sorted(re.findall(r'^test (.+?) \.\.\. ignored', output, re.M))
baseline_output = (root/'build/rf312-maintenance-20261009/rust-tests-default-dtemp.stdout.log').read_text(encoding='utf-8', errors='replace')
assert len(ignored(baseline_output)) == 3, 'Require named existing ignored tests'
assert ignored((stage/'rust-tests-default.stdout.log').read_text(encoding='utf-8')) == ignored(baseline_output), 'Ignored test identities changed'
for file,digest in expected.items():
    assert hashlib.sha256((root/file).read_bytes()).hexdigest() == digest, 'Changed source: '+file
output = stage/'rust-validation.json'
assert not output.exists()
output.write_text(json.dumps({
    'scope':'RF-1104 required Rust checks; native application journeys still require separate evidence',
    'sourceFileCount':len(expected),
    'sourcesManifestSha256':hashlib.sha256((stage/'regression-green.sources.json').read_bytes()).hexdigest(),
    'existingIgnoredTestNames':ignored(baseline_output),
    'checks':results,
},indent=2)+'\n',encoding='utf-8')
print(json.dumps({name:value['testTotals'] for name,value in results.items()},indent=2))
