from pathlib import Path
import datetime, hashlib, json, os, subprocess, time
root = Path('D:/SoloSoul')
base = Path(__file__).parent.resolve()
load = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
parent = base / 'theme-current-token-test-build1/current-apks.json'
artifacts = load(parent)
changed = {name: {'before': digest, 'after': sha(root/name)}
           for name, digest in artifacts['sourceFiles'].items() if sha(root/name) != digest}
assert list(changed) == ['tauri/scripts/android-home-native-regression.py'], changed
sources = {name: sha(root/name) for name in artifacts['sourceFiles']}
files = {row['kind']: row for row in artifacts['files']}
stage = base / 'rf1110-utf8-native-round1'
stage.mkdir(exist_ok=False)
def save(name, value):
    (stage/name).write_text(json.dumps(value, indent=2)+'\n', encoding='utf-8')
def verify():
    for name, digest in sources.items(): assert sha(root/name) == digest, name
    for row in files.values(): assert sha(Path(row['copy'])) == row['sha256']
verify()
env = os.environ.copy()
env.update({'ANDROID_HOME': str(base/'sdk'), 'ANDROID_SDK_ROOT': str(base/'sdk'),
            'ANDROID_USER_HOME': str(base/'android-user'), 'ANDROID_EMULATOR_HOME': str(base/'android-user'),
            'ANDROID_AVD_HOME': str(base/'avd'), 'ANDROID_ADB_SERVER_PORT': '5049',
            'ADB_SERVER_SOCKET': 'tcp:localhost:5049', 'TEMP': str(base/'tmp'), 'TMP': str(base/'tmp')})
env.pop('ANDROID_SDK_HOME', None)
env.pop('PYTHONUTF8', None)
env.pop('PYTHONIOENCODING', None)
encoding = subprocess.run(['python', '-c', 'import locale,json,sys; print(json.dumps(dict(encoding=locale.getencoding(),utf8Mode=sys.flags.utf8_mode)))'],
                         env=env, check=True, capture_output=True, text=True, creationflags=subprocess.CREATE_NO_WINDOW)
encoding_proof = json.loads(encoding.stdout)
assert encoding_proof == {'encoding': 'cp1252', 'utf8Mode': 0}, encoding_proof
adb = str(base/'sdk/platform-tools/adb.exe')
serial = 'emulator-5586'; avd = 'SoloSoul_RF121_20261010'
def read(args):
    return subprocess.run([adb, '-H', 'localhost', '-P', '5049', '-s', serial, *args], env=env,
                          check=True, capture_output=True, text=True, timeout=60,
                          creationflags=subprocess.CREATE_NO_WINDOW).stdout.strip()
assert read(['get-state']) == 'device' and read(['shell', 'getprop', 'sys.boot_completed']) == '1'
assert read(['emu', 'avd', 'name']).splitlines()[0] == avd
assert read(['shell', 'getprop', 'ro.build.version.sdk']) == '35'
def settings():
    return dict(blur=read(['shell','settings','get','global','disable_window_blurs']),
                night=read(['shell','cmd','uimode','night']),
                fontScale=read(['shell','settings','get','system','font_scale']))
before = settings()
save('input-proof.json', dict(parentReceiptSha256=sha(parent), changedExternalRunner=changed,
     sourceFiles=sources, files=artifacts['files'], compiledSourcesUnchanged=True, apkBytesUnchanged=True,
     childDefaultEncoding=encoding_proof, serial=serial, avd=avd, privateBackupsLocalOnly=True))
command = ['python', 'scripts/android-home-native-regression.py', '--adb', adb, '--serial', serial,
           '--avd', avd, '--apk', files['main']['copy'], '--test-apk', files['test']['copy'],
           '--output', str(stage/'baseline-supported'), '--mode', 'supported', '--scenario', 'baseline']
start = time.monotonic()
with (stage/'stdout.log').open('xb') as out, (stage/'stderr.log').open('xb') as err:
    process = subprocess.Popen(command, cwd=root/'tauri', env=env, stdin=subprocess.DEVNULL,
                               stdout=out, stderr=err, creationflags=subprocess.CREATE_NO_WINDOW)
    save('live.json', dict(pid=process.pid, command=command))
    print(f'RF1110 default-cp1252 native diagnostic PID {process.pid}', flush=True)
    code = process.wait()
verify()
report = load(stage/'baseline-supported/report.json')
native = (stage/'baseline-supported/native.log').read_bytes()
text = native.decode('utf-8', errors='strict')
device = load(stage/'baseline-supported/device-files/report.json')
after = settings()
identity = read(['emu','avd','name']).splitlines()[0] == avd
restored = bool(report.get('restored') and report.get('blurRestored') and
                report.get('notificationPermissionRestored') and before == after and identity and
                not any(report.get(k) for k in ['restoreError','blurRestoreError',
                'notificationRestoreError','nightModeRestoreError','fontScaleRestoreError']))
utf8 = '\u00b7' in text and b'\xc2\xb7' in native
failure_preserved = code == 1 and report.get('passed') is False
accepted = utf8 and failure_preserved and restored
save('verification.json', dict(accepted=accepted, scope='RF1110 diagnostic UTF-8 only',
     nativeExitCode=code, utf8StrictDecode=True, middleDotPreserved=utf8,
     originalUiFailurePreserved=failure_preserved, fullHomeMatrixAccepted=False,
     restored=restored, settingsBefore=before, settingsAfter=after,
     sourceAndApkHashesUnchanged=True, deviceEvidenceJsonReadable=True,
     nativeLogSha256=sha(stage/'baseline-supported/native.log'),
     reportSha256=sha(stage/'baseline-supported/report.json'),
     elapsedSeconds=time.monotonic()-start, at=datetime.datetime.now(datetime.timezone.utc).isoformat()))
print(f'RF1110 native exit {code}; diagnostic accepted={accepted}; restored={restored}', flush=True)
raise SystemExit(0 if accepted else 1)
