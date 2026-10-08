"""Isolated build runner. Run only after the parent sends an explicit code-freeze message."""
import argparse, hashlib, json, os, plistlib, shutil, stat, subprocess, time
from pathlib import Path
SOURCE = Path('/Users/zzc/PycharmProjects/SoloSoul')
OUT = Path('/tmp/solosoul-fe2-macos-scroll-hover-build-20261008-titlebar')
WORKSPACE = Path('/tmp/solosoul-fe2-macos-scroll-hover-build-20261008/workspace')
FIXTURE = Path('/private/var/folders/qx/ppkvfc_x2yb2d2mh38b4wxkm0000gn/T/solosoul-fe2-macos-ab5da5ed6c344e8aba12409781dbd5c6')
PROTECTED = ['tauri/Cargo.lock', 'tauri/Cargo.toml', 'tauri/src-tauri/Cargo.toml', 'tauri/src-tauri/gen/apple/project.yml', 'tauri/src-tauri/gen/apple/solo_soul.xcodeproj', 'tauri/src-tauri/gen/apple/solo_soul_iOS/Info.plist', 'tauri/src-tauri/gen/apple/assets', 'tauri/src-tauri/gen/android/app/src/main/assets', 'tauri/src-tauri/gen/android/app/tauri.properties']

def inventory(root):
    if not root.exists() and not root.is_symlink(): return None
    rows = {}
    for path in [root] + (sorted(root.rglob('*')) if root.is_dir() and not root.is_symlink() else []):
        mode = stat.S_IMODE(path.lstat().st_mode)
        if path.is_symlink(): row = {'type': 'symlink', 'mode': mode, 'target': os.readlink(path)}
        elif path.is_dir(): row = {'type': 'directory', 'mode': mode}
        elif path.is_file(): row = {'type': 'file', 'mode': mode, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        else: raise RuntimeError('Unexpected input type')
        rows['.' if path == root else str(path.relative_to(root))] = row
    return rows

def source_hashes():
    checkpoint = json.loads((SOURCE / 'docs/verification/fe2-frontend-checkpoint-2026-10-07.json').read_text())
    names = {name for name in checkpoint['source_sha256'] if not name.startswith(('tauri/e2e/', 'tauri/native-regression/'))}
    for root in ['tauri/src', 'tauri/src-tauri/src', 'tauri/crates', 'tauri/tools', 'tauri/scripts', 'tauri/public', 'tauri/src-tauri/capabilities', 'tauri/src-tauri/permissions']:
        if not (SOURCE/root).exists(): continue
        names.update(str(path.relative_to(SOURCE)) for path in (SOURCE/root).rglob('*') if path.is_file() and not set(path.relative_to(SOURCE/root).parts) & {'target', 'node_modules', '__pycache__'})
    names.update(['tauri/Cargo.toml', 'tauri/package.json', 'tauri/package-lock.json', 'tauri/src-tauri/build.rs', 'tauri/src-tauri/tauri.conf.json', 'tauri/src-tauri/Info.plist', 'tauri/index.html', 'tauri/tsconfig.json'])
    return {name: hashlib.sha256((SOURCE/name).read_bytes()).hexdigest() for name in sorted(names)}

EXCLUDE_NAMES = {'.git', 'target', 'node_modules', 'dist', 'test-results', 'playwright-report', '.vite', '.gradle', '.idea', '__pycache__', 'build', 'captures'}
def copy_sources():
    destination = WORKSPACE / 'tauri'
    assert (destination/'node_modules').is_dir() and (destination/'target/debug').is_dir(), 'APFS cache preparation missing'
    assert (destination/'src').is_dir(), 'First-round isolated workspace missing'
    for child in sorted((SOURCE/'tauri').iterdir()):
        if child.name in EXCLUDE_NAMES or child.name.startswith('bugreport-'): continue
        copied = destination/child.name
        if child.is_dir():
            if copied.exists(): shutil.rmtree(copied)
            shutil.copytree(child, copied, symlinks=True, ignore=shutil.ignore_patterns(*EXCLUDE_NAMES))
        elif child.is_symlink():
            if copied.exists() or copied.is_symlink(): copied.unlink()
            copied.symlink_to(os.readlink(child))
        elif child.is_file(): shutil.copy2(child, copied)
    market = SOURCE/'SoloSoul_plugin_market'
    output = WORKSPACE/'SoloSoul_plugin_market'
    output.mkdir(exist_ok=True)
    shutil.copy2(market/'registry.json', output/'registry.json')
    for plugin in sorted((market/'plugins').iterdir()):
        if not plugin.is_dir(): continue
        for name in ['manifest.json', 'plugin.wasm']:
            source = plugin/name
            if source.is_file():
                dest = output/'plugins'/plugin.name/name
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, dest)

parser=argparse.ArgumentParser()
parser.add_argument('--code-frozen', action='store_true', help='Parent explicitly confirmed source freeze')
args=parser.parse_args()
assert args.code_frozen, 'Do not build before explicit parent code-freeze message'
assert not (OUT/'report.json').exists(), 'This runner is single use'
report={'scope':'Current source in isolated temporary workspace and APFS cloned target; no window launched', 'passed':False, 'started_at':time.time(), 'workspace':str(WORKSPACE), 'target':str(WORKSPACE/'tauri/target'), 'synthetic_root':str(FIXTURE), 'window_launched':False, 'native_visual_passed':False, 'prior_frozen_bundle_untouched': '/tmp/solosoul-fe2-macos-scroll-hover-build-20261008/SoloSoulFE2Mac.app', 'fixture_root_read_only':True, 'source_scope':'production frontend src/css, Rust sources/crates/build script, runtime scripts/public, Cargo/package/config/capabilities; e2e and standalone native-regression files may still be edited by another agent and are excluded from frozen hashes'}
original_protected={name:inventory(SOURCE/name) for name in PROTECTED}
FIRST = Path('/tmp/solosoul-fe2-macos-scroll-hover-build-20261008')
first_artifacts={name:inventory(FIRST/name) for name in ['SoloSoulFE2Mac.app','report.json','source.json','fixture-preparation.json','build.log','run.py']}
sources=source_hashes()
(OUT/'source.json').write_text(json.dumps({'recorded_at':time.time(), 'source_sha256':sources}, indent=2)+'\n')
try:
    copy_sources()
    assert all(hashlib.sha256((WORKSPACE/name).read_bytes()).hexdigest()==digest for name,digest in sources.items()), 'Snapshot mismatched frozen source'
    env=os.environ.copy()
    env['PATH']='/Users/zzc/.nvm/versions/node/v24.14.0/bin:/Users/zzc/.rustup/toolchains/stable-aarch64-apple-darwin/bin:'+env['PATH']
    env.update({'CARGO_NET_OFFLINE':'true', 'CARGO_TARGET_DIR':str(WORKSPACE/'tauri/target'), 'ORT_LIB_PATH':'/Users/zzc/Library/Caches/ort.pyke.io/dfbin/aarch64-apple-darwin/612739f75438dc0a075461e1fb454226b4a1eb175e60a7271ba966bbbb972cd4'})
    command=['./node_modules/.bin/tauri', 'build', '--debug', '--features', 'macos-ui-regression', '--config', 'src-tauri/tauri.macos-ui-regression.conf.json', '--bundles', 'app']
    report['command']=command
    print('Frozen source snapshot verified; building isolated workspace', flush=True)
    with (OUT/'build.log').open('w') as log:
        built=subprocess.run(command, cwd=WORKSPACE/'tauri', env=env, stdout=log, stderr=subprocess.STDOUT)
    report['exit_code']=built.returncode
    assert built.returncode==0, 'Isolated macOS bundle build failed'
    app=WORKSPACE/'tauri/target/debug/bundle/macos/SoloSoulFE2Mac.app'
    info=plistlib.loads((app/'Contents/Info.plist').read_bytes())
    assert info['CFBundleIdentifier']=='com.solosoul.fe2.macos'
    binary=app/'Contents/MacOS'/info['CFBundleExecutable']
    report.update({'app':str(app), 'bundle_identifier':info['CFBundleIdentifier'], 'bundle_version':info['CFBundleShortVersionString'], 'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()})
    frozen=OUT/'SoloSoulFE2Mac.app'
    shutil.copytree(app, frozen, symlinks=True)
    assert inventory(frozen)==inventory(app), 'Frozen bundle copy mismatched'
    pointer=frozen/'Contents/Resources/fe2-macos-root.txt'
    assert not pointer.exists(), 'Unexpected preexisting fixture binding'
    marker=json.loads((FIXTURE/'fe2-macos-owned.json').read_text())
    assert marker['root']==str(FIXTURE) and marker['identifier']=='com.solosoul.fe2.macos.'+FIXTURE.name.removeprefix('solosoul-fe2-macos-')
    # First-round app may still be running; never alter its root or runtime files.
    for name in ['', 'vault', 'app-data', 'plugins']:
        root_path=FIXTURE/name
        assert root_path.is_dir() and not root_path.is_symlink() and stat.S_IMODE(root_path.stat().st_mode)==0o700
    assert stat.S_IMODE((FIXTURE/'fe2-macos-owned.json').stat().st_mode)==0o600
    assert not any(path.is_symlink() for path in FIXTURE.rglob('*'))
    accounts=json.loads((FIXTURE/'vault/accounts.json').read_text())
    assert len(accounts)==1 and accounts[0]['id']==marker['account_id'] and accounts[0]['name']=='FE2 macOS 合成验收账户'
    pointer.parent.mkdir(parents=True, exist_ok=True)
    with pointer.open('x') as f: f.write(str(FIXTURE)+'\n')
    pointer.chmod(0o600)
    assert pointer.read_text().strip()==str(FIXTURE) and stat.S_IMODE(pointer.stat().st_mode)==0o600
    report.update({'passed':True, 'frozen_app':str(frozen), 'binding_present':True, 'binding_path':str(pointer), 'binding_root':str(FIXTURE), 'fixture_marker':marker})
except Exception as error:
    report['error']=str(error)
    print('ERROR',error,flush=True)
finally:
    report['protected_original_inputs_unchanged']={name:inventory(SOURCE/name)==before for name,before in original_protected.items()}
    report['first_frozen_artifacts_unchanged']={name:inventory(FIRST/name)==before for name,before in first_artifacts.items()}
    after=source_hashes()
    report['source_differences_after_build']=[name for name,digest in sources.items() if after.get(name)!=digest]
    if not all(report['protected_original_inputs_unchanged'].values()) or not all(report['first_frozen_artifacts_unchanged'].values()) or report['source_differences_after_build']: report['passed']=False
    report['finished_at']=time.time()
    (OUT/'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
print(json.dumps({key:report.get(key) for key in ['passed', 'exit_code', 'error', 'frozen_app', 'binding_root', 'source_differences_after_build']}, ensure_ascii=False), flush=True)
raise SystemExit(0 if report['passed'] else 1)
