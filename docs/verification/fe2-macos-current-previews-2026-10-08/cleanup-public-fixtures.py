from pathlib import Path
import hashlib, json, os, stat
manifest_path=Path('/tmp/solosoul-fe2-macos-preview-native-20261008/public-fixtures.json')
manifest=json.loads(manifest_path.read_text())
folder=Path(manifest['folder'])
assert folder.parent == Path('/Users/zzc/Downloads')
assert folder.name == 'SoloSoul-FE2-Preview-8f0265ee35614a5b83195160a6259166'
assert folder.is_dir() and not folder.is_symlink() and folder.stat().st_uid == os.getuid()
assert stat.S_IMODE(folder.stat().st_mode) == 0o700
expected={Path(item['path']).name for item in manifest['files']}
assert {p.name for p in folder.iterdir()} == expected
checked=[]
for item in manifest['files']:
 p=Path(item['path'])
 assert p.parent == folder and p.is_file() and not p.is_symlink()
 assert p.stat().st_uid == os.getuid() and stat.S_IMODE(p.stat().st_mode) == 0o600
 raw=p.read_bytes()
 assert len(raw) == item['size'] and hashlib.sha256(raw).hexdigest() == item['sha256']
 checked.append({'name':p.name, 'size':len(raw), 'sha256':item['sha256'], 'verified_before_removal':True})
for item in manifest['files']: Path(item['path']).unlink()
folder.rmdir()
receipt={'scope':'Only two owned public synthetic fixture copies in exact unique Downloads folder', 'files':checked, 'folder_removed':not folder.exists(), 'existing_user_files_touched':False, 'encrypted_test_vault_imports_preserved':True}
Path('/tmp/solosoul-fe2-macos-preview-native-20261008/public-fixture-cleanup.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(receipt,ensure_ascii=False,indent=2))
