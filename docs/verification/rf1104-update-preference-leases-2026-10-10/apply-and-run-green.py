from pathlib import Path
import subprocess, sys

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
assert not (stage/'regression-green.live.json').exists()
commands = [
    [sys.executable, '-X', 'utf8', str(stage/'implement.py')],
    [sys.executable, '-X', 'utf8', str(stage/'update-tests.py')],
    ['rustfmt', '--edition', '2021', '--config', 'newline_style=Windows', str(root/'tauri/src-tauri/src/commands/update_preferences.rs')],
    [sys.executable, '-X', 'utf8', str(stage/'run-check.py'), 'regression-green', 'cargo', 'test', '--locked', '-p', 'solo_soul', '--lib', 'commands::update_preferences::tests', '--', '--nocapture'],
]
for command in commands:
    print('Running '+Path(command[0]).name, flush=True)
    code = subprocess.call(command, cwd=root)
    if code:
        sys.exit(code)
