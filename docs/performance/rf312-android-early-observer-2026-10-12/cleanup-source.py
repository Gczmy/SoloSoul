from pathlib import Path
import datetime,hashlib,importlib.util,json,tarfile
R=Path('D:/SoloSoul');B=R/'build/android-native-feasibility-20261010';I=B/'rf312-early-observer-implementation-20261012'
O=R/'docs/performance/rf312-android-early-observer-2026-10-12'
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
index=load(O/'index.json');assert index['allMembersReadBackVerified'] and sha(O/'raw.tar.gz')==index['rawSha256']
with tarfile.open(O/'raw.tar.gz') as archive:
    for member,row in zip(archive.getmembers(),index['members'],strict=True):
        assert member.name==row['path']
        data=archive.extractfile(member).read()
        assert len(data)==row['bytes'] and hashlib.sha256(data).hexdigest()==row['sha256']
spec=importlib.util.spec_from_file_location('cleanup_inventory',R/'tauri/scripts/android-home-native-regression.py')
home=importlib.util.module_from_spec(spec);spec.loader.exec_module(home)
cases=[I/'negative-control/baseline-supported-default-release',I/'positive-native/observer-supported',
    I/'positive-native/observer-fallback',I/'java-bridge-probe/late-diagnostic-supported',
    I/'java-bridge-probe-round2/late-diagnostic-supported',I/'positive-native-round2/observer-supported',
    I/'positive-native-round2/observer-fallback',B/'rf312-android-bridge-capabilities-20261012/bridge-supported']
plan=[]
def regular_inside(path):
    assert path.resolve().is_relative_to(B.resolve())
    assert path.is_file() and not path.is_symlink() and not(path.stat().st_file_attributes&0x400)
    cursor=path.parent
    while cursor!=B.parent:
        assert cursor.is_dir() and not cursor.is_symlink() and not(cursor.stat().st_file_attributes&0x400)
        cursor=cursor.parent
for case in cases:
    report=load(case/'report.json')
    assert report['restored'] and report['originalApksRestored'] and report['ownedProbeRemoved']
    assert report['originalInstalledPackages']==report['restoredInstalledPackages']
    before=case/'private-data-before.tar';after=case/'private-data-restored.tar'
    for p in [before,after]:regular_inside(p)
    a=home.inventory(before,True);z=home.inventory(after,True);assert a==z and len(a)==216
    for p in [before,after]:plan.append({'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p),
        'privateEntriesRestoreVerified':len(a),'reportSha256':sha(case/'report.json')})
assert len(plan)==16 and len({r['path'] for r in plan})==16
(O/'cleanup-plan.json').write_text(json.dumps({'files':plan,'allPairsExact':True,'nonRecursive':True},indent=2)+'\n',encoding='utf-8')
for row in plan:
    p=Path(row['path']);regular_inside(p)
    assert p.stat().st_size==row['bytes'] and sha(p)==row['sha256']
    p.unlink();assert not p.exists()
(O/'cleanup.json').write_text(json.dumps({'removedCount':len(plan),'removedFiles':plan,
    'allRemovedCopiesPreviouslyRestoreVerified':True,'allPairs216EntriesExact':True,
    'allAncestorsNonReparse':True,'nonRecursive':True,'originalAvdAndPackagesKept':True,
    'archiveVerifiedBeforeRemoval':True,'at':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n',encoding='utf-8')
(O/'cleanup-source.py').write_bytes(Path(__file__).read_bytes())
print('Removed exactly 16 redundant private backup copies after verified restore/archive; original AVD/private data preserved',flush=True)
