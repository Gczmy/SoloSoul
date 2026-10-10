from pathlib import Path
import subprocess, sys

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
for script in ['verify-rust-results.py', 'run-native-validation.py']:
    print('Running '+script,flush=True)
    code = subprocess.call([sys.executable,'-X','utf8',str(stage/script)],cwd=root)
    if code != 0:
        print(script+' failed; preserve evidence and stop',flush=True)
        sys.exit(code)
print('Fresh native groups completed; inspect complete per-sample results before acceptance',flush=True)
