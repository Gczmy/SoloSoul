"""Archive RF-1104 only after every required Rust/native gate is accepted."""
from pathlib import Path
import datetime
import gzip
import hashlib
import json
import re

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
destination = root/'docs/verification/rf1104-update-preference-leases-2026-10-10'
index_path = destination.with_suffix('.json')

def digest(data):
    return hashlib.sha256(data).hexdigest()

def load(path):
    return json.loads(path.read_text(encoding='utf-8'))

def save_new(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(json.dumps(value, indent=2, ensure_ascii=False)+'\n')

assert not destination.exists() and not index_path.exists(), 'Never overwrite earlier evidence'
manifest_path = stage/'regression-green.sources.json'
expected = load(manifest_path)
assert len(expected) == 1827, 'Require the full frozen RF-1104 input scope'
for name, sha in expected.items():
    assert digest((root/name).read_bytes()) == sha, 'Changed source: '+name
rust = load(stage/'rust-validation.json')
native = load(stage/'native-validation.json')
scope = load(stage/'acceptance-scope-review.json')
assert scope['sourceSha256'] == expected[scope['sourceFile']]
assert scope['targetedTestCount'] == 15
assert scope['fullRustAndFreshNativeJourneyAcceptanceRequiredSeparately'] is True
assert rust['sourceFileCount'] == len(expected)
assert rust['sourcesManifestSha256'] == digest(manifest_path.read_bytes())
assert native['sourceManifestSha256'] == rust['sourcesManifestSha256']
assert native['accepted'] is True and native['sampleCount'] == 6
assert native['rf312TaskClosed'] is False and native['oldHomeTimeoutRootCauseEstablished'] is False
assert [(item['objects'], item['index']) for item in native['samples']] == [
    (size, index) for size in (100, 5000) for index in (1, 2, 3)]
assert all(item['accepted'] and not item['maintenanceFailures'] for item in native['samples'])
for name, check in rust['checks'].items():
    assert check['receipt']['exitCode'] == 0 and check['receipt']['sourceUnchanged'], name

red = load(stage/'regression-red-fixed.receipt.json')
assert red['exitCode'] == 101 and red['sourceUnchanged']
assert '0 passed; 1 failed; 0 ignored;' in (stage/'regression-red-fixed.stdout.log').read_text(encoding='utf-8')
assert 'IMPORT_OPERATIONS_ACTIVE' in (stage/'regression-red-fixed.stderr.log').read_text(encoding='utf-8')
assert load(stage/'red-input-audit.json')['productionMatchesHead'] is True
build = load(stage/'native-release-build.receipt.json')
assert build['exitCode'] == 0
if not build['sourceUnchanged']:
    parity = load(stage/'release-manifest-newline-parity.json')
    assert build['changedSourceFiles'] == ['tauri/src-tauri/Cargo.toml']
    assert parity['normalizedBytesEqual'] and parity['parsedTomlEqual']
    assert parity['rawBuildReceiptSourceUnchanged'] is False
runtime = load(stage/'runtime-package.json')
assert digest(Path(runtime['exe']).read_bytes()) == runtime['sha256'] == native['exeSha256']
assert runtime['windowsGuiSubsystem'] == 2 and len(runtime['resources']) == 94
for resource in runtime['resources']:
    assert digest(Path(resource['copy']).read_bytes()) == resource['sha256']

checks = dict(rust['checks'])
checks['regression-red'] = {'receipt':load(stage/'regression-red.receipt.json'),
    'scope':'Initial test compilation error; not the defect red regression'}
checks['regression-red-fixed'] = {'receipt':red,'testTotals':{'passed':0,'failed':1,'ignored':0}}
checks['native-release-build'] = {'receipt':build}
for size in (100, 5000):
    name = 'native-auth-'+str(size)
    receipt = load(stage/(name+'.receipt.json'))
    assert receipt['exitCode'] == 0 and receipt['sourceUnchanged']
    assert receipt['pdfiumOverrideInherited'] is False
    checks[name] = {'receipt':receipt}

selected = set()
for name in checks:
    for suffix in ('.receipt.json', '.sources.json', '.stdout.log', '.stderr.log'):
        source = stage/(name+suffix)
        if source.is_file():
            selected.add(source)
for source in stage.iterdir():
    if source.is_file() and source.suffix in ('.py', '.mjs'):
        selected.add(source)
for name in ('red-input-audit.json', 'acceptance-scope-review.json', 'rust-validation.json', 'native-validation.json',
    'runtime-package.json', 'fixture-input-hashes.json', 'release-Cargo-before.toml',
    'release-Cargo-after.toml', 'release-manifest-newline-parity.json'):
    source = stage/name
    if source.is_file():
        selected.add(source)
for directory in ('before', 'initial'):
    for source in (stage/directory).rglob('*.rs'):
        selected.add(source)
for size in (100, 5000):
    group = stage/f'auth{size}'
    candidates = list(group.iterdir())
    samples = sorted(group.glob('sample-[0-9][0-9][0-9]'))
    assert len(samples) == 3
    for sample in samples:
        assert sample.is_dir() and not sample.is_symlink()
        candidates.extend(sample.iterdir())
    for source in candidates:
        if source.is_file() and source.suffix in ('.json', '.log') and (
            source.name.startswith(('native-perf-', 'sample-')) or source.suffix == '.log'):
            selected.add(source)

toolchain = load(root/'build/rf312-maintenance-20261009/toolchain.json')
save_new(stage/'final-source-proof.json', {
    'scope':'All frozen RF-1104 source inputs; report and evidence are separately reviewed',
    'sourceManifestSha256':digest(manifest_path.read_bytes()),
    'sourceFileCount':len(expected), 'allInputsStillMatch':True,
    'at':datetime.datetime.now(datetime.timezone.utc).isoformat(), 'toolchain':toolchain,
})
selected.add(stage/'final-source-proof.json')
destination.mkdir(parents=True)
records = []
attributes = destination/'.gitattributes'
attribute_bytes = (b'# Preserve original evidence bytes across Windows/Unix checkouts.\n'
    b'* -text whitespace=blank-at-eol,blank-at-eof,space-before-tab,cr-at-eol\n')
with attributes.open('xb') as stream:
    stream.write(attribute_bytes)
assert attributes.read_bytes() == attribute_bytes
records.append({'path':attributes.relative_to(root/'docs').as_posix(),
    'source':'generated: Git evidence byte-preservation rule',
    'originalBytes':len(attribute_bytes), 'originalSha256':digest(attribute_bytes),
    'storedBytes':len(attribute_bytes), 'storedSha256':digest(attribute_bytes)})
for source in sorted(selected):
    assert source.is_file() and not source.is_symlink()
    assert source.resolve().is_relative_to(stage.resolve())
    data = source.read_bytes()
    relative = source.relative_to(stage)
    target = destination/relative
    if source.suffix in ('.log', '.rs') or len(data) > 100000:
        target = target.with_name(target.name+'.gz')
        stored = gzip.compress(data, mtime=0)
    else:
        stored = data
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open('xb') as stream:
        stream.write(stored)
    actual = target.read_bytes()
    recovered = gzip.decompress(actual) if target.suffix == '.gz' else actual
    assert recovered == data
    records.append({'path':target.relative_to(root/'docs').as_posix(),
        'source':relative.as_posix(), 'originalBytes':len(data), 'originalSha256':digest(data),
        'storedBytes':len(actual), 'storedSha256':digest(actual)})
save_new(index_path, {
    'schemaVersion':1, 'task':'RF-1104', 'date':'2026-10-10',
    'baseHead':load(stage/'red-input-audit.json')['head'],
    'scope':'Update preference activity lifetime, stale-session persistence and Windows native re-unlock regression',
    'validationAccepted':True, 'commitStillRequired':True,
    'sourceManifestSha256':rust['sourcesManifestSha256'],
    'sourceFileCount':len(expected), 'toolchain':toolchain,
    'checks':checks, 'targetedAcceptanceScope':scope, 'native':native, 'records':records,
    'rf312TaskClosed':False, 'oldHomeTimeoutRootCauseEstablished':False,
    'excludedArtifacts':['Executables', 'DLLs', 'models', 'Vault databases', 'application caches'],
})
print(json.dumps({'task':'RF-1104','index':str(index_path),'records':len(records),
    'nativeSamples':len(native['samples']),'validationAccepted':True}))
