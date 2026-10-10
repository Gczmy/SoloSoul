from pathlib import Path
import subprocess, sys, json, hashlib

stage = Path(__file__).parent
runner = stage/'run-check.py'
green = json.loads((stage/'regression-green.receipt.json').read_text())
assert green['exitCode'] == 0 and green['sourceUnchanged']
green_output = (stage/'regression-green.stdout.log').read_text(encoding='utf-8', errors='replace')
assert '15 passed; 0 failed; 0 ignored;' in green_output, 'Require the complete preference suite, without skips'
root = Path('D:/SoloSoul')
expected = json.loads((stage/'regression-green.sources.json').read_text())
def verify_sources():
    for file, digest in expected.items():
        assert hashlib.sha256((root/file).read_bytes()).hexdigest() == digest, 'Changed source: '+file
checks = [
    ('rust-format', ['cargo', 'fmt', '--check']),
    ('rust-clippy-default', ['cargo', 'clippy', '--locked', '--', '-D', 'warnings']),
    ('rust-tests-default', ['cargo', 'test', '--locked', '--verbose']),
    ('rust-clippy-native', ['cargo', 'clippy', '--locked', '--features', 'native-perf', '--all-targets', '--', '-D', 'warnings']),
    ('rust-tests-native', ['cargo', 'test', '--locked', '-p', 'solo_soul', '--lib', '--features', 'native-perf', 'native_perf::', '--', '--nocapture']),
    ('rust-tests-rf905', ['cargo', 'test', '--locked', '-p', 'solo_soul', '--lib', '--features', 'native-perf', 'rf905', '--', '--nocapture']),
    ('rust-tests-preferences-native', ['cargo', 'test', '--locked', '-p', 'solo_soul', '--lib', '--features', 'native-perf', 'commands::update_preferences::tests', '--', '--nocapture']),
]
for name, command in checks:
    verify_sources()
    print('Running '+name, flush=True)
    code = subprocess.call([sys.executable, '-X', 'utf8', str(runner), name, *command])
    if code != 0:
        print(f'{name} stopped with exit {code}; preserve results and repair before new checks', flush=True)
        sys.exit(code)
    verify_sources()
print('Required RF-1104 Rust checks completed', flush=True)
