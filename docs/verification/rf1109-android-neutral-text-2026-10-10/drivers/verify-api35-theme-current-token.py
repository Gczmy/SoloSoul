"""Run existing strict native instrumentation against the owned API35 AVD.

Never create acceptance from boot readiness, skips, partial counts or a failed
restoration. Both glass lanes use the same frozen default-product APK pair.
"""
from pathlib import Path
import datetime, hashlib, json, os, subprocess, time

root = Path('D:/SoloSoul')
base = Path(__file__).parent.resolve()
load = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
artifacts = load(base / 'theme-current-token-test-build1/current-apks.json')
assert artifacts['accepted'] and not artifacts['deviceAcceptance']
sources = artifacts['sourceFiles']
assert len(sources) == 1830
files = {row['kind']: row for row in artifacts['files']}
assert set(files) == {'main', 'test'}

def verify():
    for name, digest in sources.items():
        assert sha(root / name) == digest, name
    for row in files.values():
        assert sha(Path(row['copy'])) == row['sha256']

verify()
stage = base / 'api35-native-remaining-seven-current-token-round2'
stage.mkdir(exist_ok=False)

def save(name, data):
    with (stage / name).open('x', encoding='utf-8', newline='\n') as out:
        out.write(json.dumps(data, indent=2) + '\n')

env = os.environ.copy()
env.update({'ANDROID_HOME': str(base / 'sdk'), 'ANDROID_SDK_ROOT': str(base / 'sdk'),
    'ANDROID_USER_HOME': str(base / 'android-user'), 'ANDROID_EMULATOR_HOME': str(base / 'android-user'),
    'ANDROID_AVD_HOME': str(base / 'avd'), 'ANDROID_ADB_SERVER_PORT': '5049',
    'ADB_SERVER_SOCKET': 'tcp:localhost:5049', 'TEMP': str(base / 'tmp'), 'TMP': str(base / 'tmp')})
env.pop('ANDROID_SDK_HOME', None)
adb = str(base / 'sdk/platform-tools/adb.exe')
serial = 'emulator-5586'
avd = 'SoloSoul_RF121_20261010'

def read(args):
    return subprocess.run([adb, '-H', 'localhost', '-P', '5049', '-s', serial, *args],
        check=True, capture_output=True, text=True, env=env, timeout=60,
        creationflags=subprocess.CREATE_NO_WINDOW).stdout.strip()

assert read(['get-state']) == 'device'
assert read(['shell', 'getprop', 'sys.boot_completed']) == '1'
assert read(['emu', 'avd', 'name']).splitlines()[0] == avd
assert read(['shell', 'getprop', 'ro.build.version.sdk']) == '35'
assert read(['shell', 'getprop', 'ro.product.cpu.abi']) == 'x86_64'
save('input-proof.json', {'artifactReceiptSha256': sha(base / 'theme-current-token-test-build1/current-apks.json'),
    'serial': serial, 'avd': avd, 'api': 35, 'abi': 'x86_64', 'sourceFiles': sources,
    'files': artifacts['files'], 'privateAdbSocket': 'tcp:localhost:5049',
    'physicalGpuPerformanceEvidence': False, 'appAcceptance': False})

results = []
for mode, expected in [('supported', 7), ('fallback', 7)]:
    verify()
    settings_before = {'disableWindowBlurs': read(['shell', 'settings', 'get', 'global', 'disable_window_blurs']),
        'nightMode': read(['shell', 'cmd', 'uimode', 'night'])}
    output = stage / mode
    command = ['node', 'D:\\SoloSoul\\build\\android-native-feasibility-20261010\\native-remaining-seven.mjs', '--adb', adb,
        '--serial', serial, '--avd', avd, '--apk', files['main']['copy'],
        '--test-apk', files['test']['copy'], '--output', str(output), '--mode', mode]
    began = time.monotonic()
    with (stage / (mode + '.stdout.log')).open('xb') as out, (stage / (mode + '.stderr.log')).open('xb') as err:
        p = subprocess.Popen(command, cwd=root / 'tauri', env=env, stdin=subprocess.DEVNULL,
            stdout=out, stderr=err, creationflags=subprocess.CREATE_NO_WINDOW)
        save(mode + '.live.json', {'pid': p.pid, 'command': command})
        print(f'API35 {mode} native regression live PID {p.pid}', flush=True)
        code = p.wait()
    verify()
    report = load(output / 'report.json') if (output / 'report.json').is_file() else {}
    passed = bool(code == 0 and report.get('passed') and report.get('count') == expected
        and report.get('successes') == expected and report.get('api') == 35
        and report.get('serial') == serial and report.get('avd') == avd and report.get('mode') == mode)
    settings_after = {'disableWindowBlurs': read(['shell', 'settings', 'get', 'global', 'disable_window_blurs']),
        'nightMode': read(['shell', 'cmd', 'uimode', 'night'])}
    identity_after = read(['emu', 'avd', 'name']).splitlines()[0] == avd
    restored = (not any(report.get(name) for name in ['restoreError', 'cleanupError'])
        and bool(report) and settings_before == settings_after and identity_after)
    passed = passed and restored
    row = {'mode': mode, 'command': command, 'exitCode': code, 'accepted': passed,
        'elapsedSeconds': time.monotonic() - began, 'expectedTests': expected,
        'observedTests': report.get('count'), 'successes': report.get('successes'),
        'restorationErrorsAbsent': restored, 'sourceAndApkHashesUnchanged': True,
        'settingsBefore': settings_before, 'settingsAfter': settings_after,
        'dedicatedAvdIdentityConfirmedAfter': identity_after,
        'reportSha256': sha(output / 'report.json') if report else None}
    save(mode + '.receipt.json', row)
    results.append(row)
    print(f'API35 {mode} terminal exit {code}; accepted={passed}', flush=True)
    # A failed capability lane may be followed by the independent fallback lane;
    # an incomplete device restoration must stop further mutation immediately.
    if not restored:
        break
accepted = len(results) == 2 and all(row['accepted'] for row in results)
save('verification.json', {'accepted': accepted, 'results': results,
    'scope': 'Independent last seven methods per mode; original full groups remain failed on WebGL',
    'homeMatrixAccepted': False, 'api31Accepted': False, 'physicalDeviceAccepted': False,
    'at': datetime.datetime.now(datetime.timezone.utc).isoformat()})
raise SystemExit(0 if accepted else 1)
