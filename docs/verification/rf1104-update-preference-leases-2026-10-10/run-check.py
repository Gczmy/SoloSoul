from pathlib import Path
import os, subprocess, time, json, hashlib, datetime, sys

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
cache = root/'build/rf312-maintenance-20261009'
name, *command = sys.argv[1:]
assert name and command and '/' not in name and '\\' not in name
assert not (stage/(name+'.receipt.json')).exists()
files = json.loads((cache/'build-inputs-dtemp.json').read_text())['sourceFiles']
def hashes():
    return {f:hashlib.sha256((root/f).read_bytes()).hexdigest() for f in files}
before = hashes()
(stage/(name+'.sources.json')).write_text(json.dumps(before,indent=2)+'\n',encoding='utf-8')
env = os.environ.copy()
env.update({'TEMP':str(cache/'tmp'), 'TMP':str(cache/'tmp'), 'npm_config_cache':str(cache/'npm-cache'),
    'CARGO_HOME':str(cache/'cargo-home'), 'CARGO_TARGET_DIR':str(cache/'cargo-target'),
    'CARGO_BUILD_JOBS':'1', 'ORT_CACHE_DIR':str(cache/'ort-cache'),
    'PDFIUM_LIBRARY_PATH':str(root/'tauri/src-tauri/resources/pdfium/pdfium.dll')})
env['PATH'] = 'C:/Program Files/Git/usr/bin;'+env.get('PATH','')
started = time.monotonic()
with (stage/(name+'.stdout.log')).open('xb') as out, (stage/(name+'.stderr.log')).open('xb') as err:
    process = subprocess.Popen(command, cwd=root/'tauri',env=env,stdout=out,stderr=err)
    (stage/(name+'.live.json')).write_text(json.dumps({'command':command,'pid':process.pid,
        'at':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n',encoding='utf-8')
    print(f'{name} live PID {process.pid}',flush=True)
    code = process.wait()
receipt = {'command':command,'pid':process.pid,'exitCode':code,'elapsedSeconds':time.monotonic()-started,
    'sourceUnchanged':before==hashes(),'temporaryDirectory':env['TEMP'],
    'at':datetime.datetime.now(datetime.timezone.utc).isoformat()}
(stage/(name+'.receipt.json')).write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
print(json.dumps(receipt),flush=True)
print((stage/(name+'.stdout.log')).read_text(encoding='utf-8',errors='replace')[-2600:],flush=True)
print((stage/(name+'.stderr.log')).read_text(encoding='utf-8',errors='replace')[-1800:],flush=True)
assert receipt['sourceUnchanged'],'Source changed during check'
sys.exit(code)
