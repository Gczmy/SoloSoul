from pathlib import Path
import hashlib,json,os,subprocess,stat
bundle=Path('/tmp/solosoul-fe2-macos-theme-fixture-final-build-20261008/SoloSoulFE2Mac.app')
root=Path((bundle/'Contents/Resources/fe2-macos-root.txt').read_text().strip())
assert root.name == 'solosoul-fe2-macos-bda573b7e6c6401597a06faac48e6ff0'
assert root.is_dir() and not root.is_symlink() and root.stat().st_uid == os.getuid()
assert stat.S_IMODE(root.stat().st_mode) == 0o700
pid=83926
ps=subprocess.run(['ps','-p',str(pid),'-o','pid=,comm='],capture_output=True,text=True)
assert ps.returncode==0 and str(bundle.resolve()/'Contents/MacOS/solo_soul') in ps.stdout
ls=subprocess.run(['lsof','-p',str(pid),'-Fn'],capture_output=True,text=True)
assert ls.returncode==0
paths=[s[1:] for s in ls.stdout.splitlines() if s.startswith('n')]
owned=[s for s in paths if s.startswith(str(root)+'/')]
required=['vault/.lock','app-data/logs/app.log','plugins/.lock']
assert all(str(root/r) in owned for r in required)
formal=[s for s in paths if '/.solosoul/' in s or '/SoloSoul/plugins/' in s]
assert not formal
files=[]
for p in sorted((root/'vault').rglob('*')):
 if p.is_file() and not p.is_symlink() and 'attachments' in p.relative_to(root).parts:
  raw=p.read_bytes()
  files.append({'relative_path':str(p.relative_to(root)),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),'solc_magic':raw[:4]==b'SOLC'})
receipt={'pid':pid,'process_command':ps.stdout.strip(),'binary_sha256':hashlib.sha256((bundle/'Contents/MacOS/solo_soul').read_bytes()).hexdigest(),'macos':subprocess.run(['sw_vers'],capture_output=True,text=True,check=True).stdout.strip(),'owned_required_paths_observed':required,'formal_vault_or_plugin_paths_observed':formal,'external_sandbox_inherited':False,'root_mode':'0700','attachment_ciphertext_files':files,'scope':'Owned frozen macOS test process only; public synthetic account; no decrypted file content collected'}
Path('/tmp/solosoul-fe2-macos-preview-native-20261008/provenance.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(receipt,ensure_ascii=False,indent=2))
