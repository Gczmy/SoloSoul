from pathlib import Path
import subprocess, hashlib, json, os, shutil
root=Path('/Users/zzc/PycharmProjects/SoloSoul')
base=Path('/tmp/solosoul-fe2-cold-build-protected-20261008')
base.mkdir(mode=0o700,exist_ok=False)
paths=['tauri/src-tauri/Cargo.toml','tauri/src-tauri/gen/android/app/src/main/assets/SoloSoul_plugin_market/registry.json','tauri/src-tauri/gen/android/app/tauri.properties']
before={p:(root/p).read_bytes() for p in paths}
for i,p in enumerate(paths): (base/f'{i}-{Path(p).name}').write_bytes(before[p])
command=['npm','run','tauri','--','android','build','--debug','--target','aarch64','--split-per-abi','--apk','--ci']
result={'command':command,'exit_code':None,'preserved':{}}
try:
 with open('/tmp/solosoul-fe2-cold-main-build-20261008.log','w') as log:
  result['exit_code']=subprocess.run(command,cwd=root/'tauri',stdout=log,stderr=subprocess.STDOUT).returncode
finally:
 for p,original in before.items():
  if (root/p).read_bytes()!=original: (root/p).write_bytes(original)
  assert (root/p).read_bytes()==original
  result['preserved'][p]=hashlib.sha256(original).hexdigest()
 (base/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
raise SystemExit(result['exit_code'] if result['exit_code'] is not None else 1)
