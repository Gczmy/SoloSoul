from pathlib import Path
import hashlib, json, os, shutil, subprocess, time, datetime

root = Path('D:/SoloSoul')
base = Path(__file__).parent.resolve()
load = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
original = load(base / 'card-visual-state-test-build3/current-apks.json')
sources = original['sourceFiles'].copy()
changed = [name for name, digest in sources.items() if sha(root / name) != digest]
test = 'tauri/src-tauri/gen/android/app/src/androidTest/java/com/solosoul/app/AndroidThemeInstrumentedTest.kt'
home = 'tauri/src-tauri/gen/android/app/src/androidTest/java/com/solosoul/app/AndroidHomeInstrumentedTest.kt'
assert set(changed) == {test, home}, changed
for name in changed: sources[name] = sha(root / name)
original.pop("testOnlyChange", None)
stage = base / 'theme-current-token-test-build1'
stage.mkdir(exist_ok=False)
def save(name, value):
    with (stage / name).open('x', encoding='utf-8', newline='\n') as out:
        json.dump(value, out, indent=2)
def verify():
    for name, digest in sources.items(): assert sha(root / name) == digest, name
    for row in original['files']: assert sha(Path(row['copy'])) == row['sha256']
verify()
save('input-proof.json', {'originalReceiptSha256': sha(base / 'card-visual-state-test-build3/current-apks.json'),
    'changedFiles': changed, 'oldTestSha256': original['sourceFiles'][test],
    'newTestSha256': sources[test], 'sourceFiles': sources,
    'originalNativeFailuresPreserved': True, 'productSourcesUnchanged': True})
sdk = base / 'sdk'
jdk = Path('C:/Users/40299571/ss-android/jdk')
gradle = root / 'tauri/src-tauri/gen/android'
env = os.environ.copy()
env.update({'JAVA_HOME': str(jdk), 'ANDROID_HOME': str(sdk), 'ANDROID_SDK_ROOT': str(sdk),
    'NDK_HOME': str(sdk / 'ndk/27.0.12077973'), 'ANDROID_NDK_HOME': str(sdk / 'ndk/27.0.12077973'),
    'ANDROID_USER_HOME': str(base / 'android-user'), 'ANDROID_EMULATOR_HOME': str(base / 'android-user'),
    'ANDROID_AVD_HOME': str(base / 'avd'), 'ANDROID_ADB_SERVER_PORT': '5049',
    'ADB_SERVER_SOCKET': 'tcp:localhost:5049', 'RUSTUP_HOME': str(base / 'rustup'),
    'RUSTUP_TOOLCHAIN': '1.96.0', 'CARGO_HOME': str(base / 'cargo-home'),
    'CARGO_TARGET_DIR': str(base / 'cargo-target'), 'CARGO_BUILD_JOBS': '1',
    'GRADLE_USER_HOME': str(base / 'gradle-home'), 'ORT_CACHE_DIR': str(base / 'ort-cache'),
    'TEMP': str(base / 'tmp'), 'TMP': str(base / 'tmp'),
    'JAVA_TOOL_OPTIONS': f'-Duser.home={base / "java-home"} -Djava.io.tmpdir={base / "tmp"}'})
for key in ['ANDROID_SDK_HOME', 'PDFIUM_LIBRARY_PATH', 'SOLOSOUL_KEYSTORE_PATH',
    'SOLOSOUL_KEYSTORE_PASSWORD', 'SOLOSOUL_KEY_ALIAS', 'SOLOSOUL_KEY_PASSWORD', 'SOLOSOUL_REGISTRY_PUBKEY']:
    env.pop(key, None)
env['PATH'] = str(base / 'cargo-home/bin') + ';' + str(jdk / 'bin') + ';' + env.get('PATH', '')
def run(name, command, cwd):
    began = time.monotonic()
    with (stage / (name+'.stdout.log')).open('xb') as out, (stage / (name+'.stderr.log')).open('xb') as err:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
            stdout=out, stderr=err, creationflags=subprocess.CREATE_NO_WINDOW)
        save(name+'.live.json', {'pid': process.pid, 'command': command})
        print(f'{name} live PID {process.pid}', flush=True)
        code = process.wait()
    verify()
    save(name+'.receipt.json', {'exitCode': code, 'elapsedSeconds': time.monotonic()-began,
        'sourceUnchanged': True, 'command': command})
    print(f'{name} terminal exit {code}', flush=True)
    assert code == 0
run('test-build', ['cmd.exe', '/d', '/c', str(gradle / 'gradlew.bat'), ':app:assembleX86_64DebugAndroidTest',
    '-x', ':app:rustBuildX86_64Debug', '--init-script', str(base / 'current-apks-recovery2/private-packaging-assets.init.gradle'),
    '--no-daemon', '--max-workers', '2'], gradle)
test_apk = gradle / 'app/build/outputs/apk/androidTest/x86_64/debug/app-x86_64-debug-androidTest.apk'
copied = stage / test_apk.name
shutil.copy2(test_apk, copied)
run('test-badging', [str(sdk / 'build-tools/35.0.0/aapt2.exe'), 'dump', 'badging', str(copied)], stage)
assert "package: name='com.solosoul.app.test'" in (stage / 'test-badging.stdout.log').read_text()
run('test-signature', [str(jdk / 'bin/java.exe'), '-jar', str(sdk / 'build-tools/35.0.0/lib/apksigner.jar'),
    'verify', '--verbose', str(copied)], stage)
files = [dict(row) for row in original['files']]
for row in files:
    if row['kind'] == 'test':
        row.update({'source': str(test_apk), 'copy': str(copied), 'sha256': sha(copied), 'bytes': copied.stat().st_size})
assert files[0]['kind'] == 'main' and files[0]['sha256'] == original['files'][0]['sha256']
save('current-apks.json', {**original, 'files': files, 'sourceFiles': sources,
    'scope': 'Unchanged full product APK plus rebuilt current neutral-text theme/home instrumentation',
    'testOnlyChangedFiles': changed, 'appAcceptance': False, 'deviceAcceptance': False,
    'at': datetime.datetime.now(datetime.timezone.utc).isoformat()})
print('Test-only APK rebuilt; full product APK remains byte-identical', flush=True)
