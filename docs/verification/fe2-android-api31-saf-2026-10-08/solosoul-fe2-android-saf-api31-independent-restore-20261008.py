from pathlib import Path
import importlib.util, subprocess, json, tarfile, os, sys

root = Path(sys.argv[1]).resolve()
assert root.parent == Path('/tmp').resolve() and root.name.startswith('solosoul-fe2-android-saf-')
spec = importlib.util.spec_from_file_location('runner', '/Users/zzc/PycharmProjects/SoloSoul/tauri/scripts/android-home-native-regression.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
prefix = ['/Users/zzc/Library/Android/sdk/platform-tools/adb', '-s', 'emulator-5588']
assert subprocess.check_output(prefix + ['emu', 'avd', 'name'], text=True).splitlines()[0] == 'SoloSoul_FE2_API31'
live = root / 'private-data-independent.tar'
with live.open('xb') as f:
    os.chmod(live, 0o600)
    subprocess.run(prefix + ['exec-out', 'run-as', 'com.solosoul.app', 'tar', '-cf', '-', '.'], check=True, stdout=f, stderr=subprocess.PIPE)
before = m.inventory(root / 'private-data-before.tar', True)
assert before == m.inventory(root / 'private-data-restored.tar', True) == m.inventory(live, True)
with tarfile.open(root / 'private-data-before.tar') as t:
    mode = t.getmember('.').mode
assert int(subprocess.check_output(prefix + ['shell', 'run-as', 'com.solosoul.app', 'stat', '-c', '%a', '.'], text=True).strip(), 8) == mode
result = {'passed': True, 'serial': 'emulator-5588', 'avd': 'SoloSoul_FE2_API31', 'actual_private_entries': len(before), 'contents_links_modes_and_actual_root_equal': True}
(root / 'independent-restoration.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
