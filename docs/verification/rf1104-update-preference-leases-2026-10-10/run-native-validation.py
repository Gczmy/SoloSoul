from pathlib import Path
import os, subprocess, json, hashlib, time, datetime, shutil, struct, tomllib

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
cache = root/'build/rf312-maintenance-20261009'
assert (stage/'rust-validation.json').is_file(), 'Inspect and verify all completed Rust checks first'
expected = json.loads((stage/'regression-green.sources.json').read_text())

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def verify_sources():
    for file,digest in expected.items():
        assert sha(root/file) == digest, 'Changed source: '+file

def save(name, value):
    path = stage/name
    with path.open('x', encoding='utf-8', newline='\n') as out:
        out.write(json.dumps(value, indent=2)+'\n')

verify_sources()
proof = json.loads((stage/'rust-validation.json').read_text())
assert proof['sourcesManifestSha256'] == sha(stage/'regression-green.sources.json')
assert proof['sourceFileCount'] == len(expected)
for name,result in proof['checks'].items():
    assert result['receipt']['exitCode'] == 0 and result['receipt']['sourceUnchanged'], name
assert proof['checks']['regression-green']['testTotals'] == {'passed':15,'failed':0,'ignored':0}
assert proof['checks']['rust-tests-default']['testTotals'] == {'passed':1871,'failed':0,'ignored':3}
assert proof['checks']['rust-tests-native']['testTotals'] == {'passed':113,'failed':0,'ignored':0}
assert proof['checks']['rust-tests-rf905']['testTotals'] == {'passed':29,'failed':0,'ignored':0}
assert proof['checks']['rust-tests-preferences-native']['testTotals'] == {'passed':15,'failed':0,'ignored':0}
env = os.environ.copy()
env.update({'TEMP':str(cache/'tmp'), 'TMP':str(cache/'tmp'),
    'npm_config_cache':str(cache/'npm-cache'), 'CARGO_HOME':str(cache/'cargo-home'),
    'CARGO_TARGET_DIR':str(cache/'cargo-target'), 'CARGO_BUILD_JOBS':'1',
    'ORT_CACHE_DIR':str(cache/'ort-cache')})
env['PATH'] = 'C:/Program Files/Git/usr/bin;'+env.get('PATH','')
env.pop('PDFIUM_LIBRARY_PATH', None)

def run(name, command):
    assert not (stage/(name+'.receipt.json')).exists()
    started = time.monotonic()
    with (stage/(name+'.stdout.log')).open('xb') as out, (stage/(name+'.stderr.log')).open('xb') as err:
        process = subprocess.Popen(command,cwd=root/'tauri',env=env,stdout=out,stderr=err)
        save(name+'.live.json', {'command':command, 'pid':process.pid,
            'at':datetime.datetime.now(datetime.timezone.utc).isoformat()})
        print(name+' live PID '+str(process.pid),flush=True)
        code = process.wait()
    changed = [file for file,digest in expected.items() if sha(root/file) != digest]
    receipt = {'command':command, 'pid':process.pid, 'exitCode':code,
        'elapsedSeconds':time.monotonic()-started, 'sourceUnchanged':not changed,
        'changedSourceFiles':changed, 'pdfiumOverrideInherited':False,
        'temporaryDirectory':env['TEMP'], 'at':datetime.datetime.now(datetime.timezone.utc).isoformat()}
    save(name+'.receipt.json', receipt)
    print(name+' terminal exit '+str(code),flush=True)
    return receipt

manifest = root/'tauri/src-tauri/Cargo.toml'
manifest_before = manifest.read_bytes()
with (stage/'release-Cargo-before.toml').open('xb') as out:
    out.write(manifest_before)
receipt = run('native-release-build', ['npm.cmd','run','tauri','--','build','--config',
    'src-tauri/tauri.native-perf.conf.json','--features','native-perf','--no-bundle','--ci'])
manifest_after = manifest.read_bytes()
with (stage/'release-Cargo-after.toml').open('xb') as out:
    out.write(manifest_after)
if receipt['changedSourceFiles']:
    assert receipt['changedSourceFiles'] == ['tauri/src-tauri/Cargo.toml'], receipt
    assert manifest_before.replace(b'\r\n',b'\n') == manifest_after.replace(b'\r\n',b'\n')
    assert tomllib.loads(manifest_before.decode('utf-8')) == tomllib.loads(manifest_after.decode('utf-8'))
    save('release-manifest-newline-parity.json', {
        'rawBuildReceiptSourceUnchanged':False,
        'beforeSha256':hashlib.sha256(manifest_before).hexdigest(),
        'afterSha256':hashlib.sha256(manifest_after).hexdigest(),
        'normalizedBytesEqual':True, 'parsedTomlEqual':True,
        'action':'Restore only original Cargo.toml newline bytes; preserve raw build receipt',
    })
    manifest.write_bytes(manifest_before)
verify_sources()
assert receipt['exitCode'] == 0, 'Build failed; preserve diagnostics, do not launch GUI'

binary_dir = stage/'bin'
binary_dir.mkdir(exist_ok=False)
exe = binary_dir/'solo_soul.exe'
shutil.copy2(cache/'cargo-target/release/solo_soul.exe', exe)
data = exe.read_bytes()
offset = struct.unpack_from('<I',data,0x3c)[0]
assert data[offset:offset+4] == b'PE\0\0'
assert struct.unpack_from('<H',data,offset+24+68)[0] == 2, 'Require Windows GUI PE'
for marker in [b'windows-native-auth-attribution', b'windows-native-maintenance-admission']:
    assert marker in data, 'Missing non-default diagnostic marker'
old_package = json.loads((cache/'runtime-package.json').read_text())
assert sha(exe) != old_package['sha256'], 'Require a fresh executable for changed production source'
resources = {}
for config in ['tauri.conf.json', 'tauri.windows.conf.json']:
    resources.update(json.loads((root/'tauri/src-tauri'/config).read_text())['bundle']['resources'])
rows = []
for source,destination in resources.items():
    original = root/'tauri/src-tauri'/source
    target = binary_dir/destination
    assert original.resolve().is_relative_to(root.resolve()), str(original)
    originals = sorted(original.rglob('*')) if original.is_dir() else [original]
    for file in originals:
        assert not file.is_symlink(), str(file)
        if not file.is_file():
            continue
        copy = target/file.relative_to(original) if original.is_dir() else target
        assert copy.resolve().is_relative_to(binary_dir.resolve()), str(copy)
        assert not copy.exists(), str(copy)
        copy.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(file,copy)
        digest = sha(file)
        assert sha(copy) == digest
        rows.append({'source':str(file),'copy':str(copy),'sha256':digest})
assert len(rows) == len(old_package['resources']) == 94
assert {row['source']:row['sha256'] for row in rows} == {
    row['source']:row['sha256'] for row in old_package['resources']}
package = {'exe':str(exe),'sha256':sha(exe),'bytes':len(data),
    'windowsGuiSubsystem':2,'resources':rows,
    'sourceManifestSha256':sha(stage/'regression-green.sources.json')}
save('runtime-package.json', package)
verify_sources()
print('Fresh GUI executable and 94 unchanged resource files verified',flush=True)

fixture_snapshots = {}
for size in [100,5000]:
    fixture = cache/'fixtures'/f'vault{size}'
    assert fixture.is_dir()
    fixture_snapshots[size] = {str(file.relative_to(fixture)):sha(file)
        for file in sorted(fixture.rglob('*')) if file.is_file()}
    assert fixture_snapshots[size]
save('fixture-input-hashes.json', fixture_snapshots)
for size in [100,5000]:
    verify_sources()
    assert sha(exe) == package['sha256']
    for row in rows:
        assert sha(Path(row['copy'])) == row['sha256']
    fixture = cache/'fixtures'/f'vault{size}'
    output = stage/f'auth{size}'
    assert not output.exists(), 'Never overwrite an earlier native group'
    receipt = run(f'native-auth-{size}', ['node','scripts/native-perf-auth-attribution.mjs',
        '--exe',str(exe),'--fixture',str(fixture),'--output',str(output),'--samples','3'])
    verify_sources()
    assert sha(exe) == package['sha256'], 'GUI executable changed during native group'
    for row in rows:
        assert sha(Path(row['copy'])) == row['sha256'], 'Resource changed during native group'
    assert {str(file.relative_to(fixture)):sha(file) for file in sorted(fixture.rglob('*'))
        if file.is_file()} == fixture_snapshots[size], 'Original fixture changed'
    # Preserve all samples and run the other group even when this group fails.
print('Both complete native groups reached terminal results; inspect every sample before acceptance',flush=True)
