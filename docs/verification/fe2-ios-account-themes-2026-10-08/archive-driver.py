import hashlib,json,os,shutil,stat
from pathlib import Path
REPO=Path('/Users/zzc/PycharmProjects/SoloSoul')
DEST=REPO/'docs/verification/fe2-ios-native-file-picker-2026-10-08'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def write(p,v): p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def inventory(root):
    rows={'.':{'type':'directory','mode':stat.S_IMODE(root.stat().st_mode)}}
    for p in sorted(root.rglob('*')):
        mode=stat.S_IMODE(p.lstat().st_mode)
        if p.is_symlink():v={'type':'symlink','mode':mode,'target':os.readlink(p)}
        elif p.is_dir():v={'type':'directory','mode':mode}
        elif p.is_file():v={'type':'file','mode':mode,'sha256':sha(p)}
        else:raise RuntimeError('Unexpected special file')
        rows[str(p.relative_to(root))]=v
    return rows
def archive(src,name):
    out=DEST/name;out.mkdir(parents=True,exist_ok=False)
    r=json.loads((src/'report.json').read_text())
    assert r['restored'] and r['appearance_restored'] and r['simulator_shutdown_restored']
    assert inventory(src/'private-data-before')==inventory(Path('/tmp/solosoul-fe2-ios-current-account-20261008/private-data-before'))
    for p in src.iterdir():
        if p.is_file() and p.suffix in ('.json','.log','.py','.yml'):shutil.copy2(p,out/p.name)
    for name in ['Host','Tests','public-fixtures']:
        shutil.copytree(src/name,out/name)
    manifest=json.loads((src/'attachments/manifest.json').read_text())
    selected=[]; stages=[];d=out/'attachments';d.mkdir()
    for test in manifest:
        for a in test['attachments']:
            p=src/'attachments'/a['exportedFileName'];n=a['suggestedHumanReadableName']
            if p.suffix=='.png' or (p.suffix=='.txt' and '-hierarchy_' in n):
                shutil.copy2(p,d/p.name);selected.append(a)
                if p.suffix=='.png':stages.append(n.split('_0_')[0])
    write(d/'manifest.json',[{'testIdentifier':manifest[0]['testIdentifier'],'attachments':selected}])
    write(out/'archive-scope.json',{'native_passed':r['passed'],'screenshots':len(stages),'screenshot_stages':stages,'private_before_backup_matches_earliest':True,'private_backup_not_exported':True,'full_bundles_and_xcresult_not_exported':True,'attachment_manifest_filtered_to_png_and_stage_hierarchies':True})
    print(name,'archived',len(stages),'PNG; driver restoration and baseline backup equality checked')
if __name__=='__main__':
    archive(Path('/tmp/solosoul-fe2-ios-current-file-picker-ui-20261008'),'initial-localization-failure')
