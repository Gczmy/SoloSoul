from pathlib import Path
import hashlib,io,json,re,subprocess,tarfile,tomllib
R=Path('D:/SoloSoul');B=Path(__file__).parent;I=B/'rf312-early-observer-implementation-20261012'
folder='docs/performance/rf312-android-early-observer-2026-10-12'
def git(*args):return subprocess.run(['git',*args],cwd=R,check=True,capture_output=True).stdout
def staged(p):return git('show',':'+p)
def load(data):return json.loads(data.decode('utf-8-sig'))
sha=lambda data:hashlib.sha256(data).hexdigest()
production={'tauri/src-tauri/Cargo.toml','tauri/src-tauri/src/lib.rs','tauri/src-tauri/src/android_native_perf.rs',
    'tauri/src-tauri/src/android_native_perf/observer.js','tauri/src-tauri/src/android_native_perf/observer.node.test.mjs',
    'tauri/src-tauri/gen/android/app/src/androidTest/java/com/solosoul/app/AndroidEarlyIpcObserverInstrumentedTest.kt'}
allowed=production|{'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md',
    'docs/performance/RF-312-android-early-ipc-observer.md','docs/performance/rf312-android-early-observer-2026-10-12.json'}
paths=git('diff','--cached','--name-only').decode().splitlines()
assert production.issubset(paths) and all(p in allowed or p.startswith(folder+'/') for p in paths)
index=load(staged(folder+'/index.json'));raw=staged(folder+'/raw.tar.gz')
assert sha(raw)==index['rawSha256'] and len(raw)==index['rawBytes']
members={}
with tarfile.open(fileobj=io.BytesIO(raw),mode='r:gz') as archive:
    for member in archive:
        assert member.isfile() and not member.name.startswith('/') and '..' not in Path(member.name).parts
        assert member.name not in members
        members[member.name]=archive.extractfile(member).read()
assert len(members)==index['memberCount']
for row in index['members']:assert len(members[row['path']])==row['bytes'] and sha(members[row['path']])==row['sha256']
summary=load(staged('docs/performance/rf312-android-early-observer-2026-10-12.json'))
assert summary['acceptedEarlyTransportObserver'] and summary['nodeTests']==12
assert summary['nativePositive']['nativeTests']==2 and summary['nativePositive']['failures']==0
assert summary['nativePositive']['skipped']==0 and len(summary['nativePositive']['cases'])==2
assert summary['negativeControl']['acceptedAbsenceControl']
assert not any(summary[k] for k in ['fixedVault1005000NativePerformanceAccepted',
    'physicalArmPerformanceAccepted','fullRF121Accepted','fullRF312Accepted','taskClosed'])
sources=summary['sourceProof']['currentSourceFiles'];assert len(sources)==1843
assert all(sha((R/n).read_bytes())==h for n,h in sources.items())
for name in production:
    assert sha(staged(name))==sources[name]
    assert members['implementation/executed-sources/'+name]==staged(name)
manifest=tomllib.loads(staged('tauri/src-tauri/Cargo.toml').decode())
assert manifest['features']['default']==['custom-protocol'] and manifest['features']['android-native-perf']==[]
kt='tauri/src-tauri/gen/android/app/src/androidTest/java/com/solosoul/app/AndroidHomeInstrumentedTest.kt'
assert sha((R/kt).read_bytes())=='ff485fd1a2de8cb1e99a6c1524d9256d0d49f81fbe83ae0905b3b0255cb07ca7'
for case in summary['nativePositive']['cases']:
    assert case['invokeReadonly'] and case['transportCount']>0 and len(case['completeHomeStages'])==12
    assert case['privateEntriesExactlyRestored']>0 and case['originalPackageBytesAndUidRestored']
    assert len(case['pngs'])==11
    for png in case['pngs']:
        member='implementation/positive-native-round2/observer-'+case['mode']+'/device-files/'+png['name']
        assert sha(members[member])==png['sha256']
assert summary['emulatorNormalStop']['ownedProcessExited'] and not summary['emulatorNormalStop']['forced']
assert summary['windowsChecks']['counts']=={'passed':1874,'failed':0,'ignored':3,'measured':0,'filtered':0}
assert len(summary['failedMethodShadowNative']['results'])==2
assert summary['lateDiagnosticWrongCommand']['failures']==1
assert summary['lateDiagnosticCorrected']['acceptedLateDiagnostic']
cleanup=load(staged(folder+'/cleanup.json'))
assert cleanup['allRemovedCopiesPreviouslyRestoreVerified'] and cleanup['nonRecursive']
assert all(not Path(row['path']).exists() for row in cleanup['removedFiles'])
report=staged('docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md').decode('utf-8')
rows=[l for l in report.splitlines() if re.match(r'^\|\s*\d+\s*\|\s*\[RF-\d+\]',l)]
assert len(rows)==322 and sum('[x]' in l for l in rows)==310
assert sum('[!]' in l for l in rows)==6 and sum('[ ]' in l for l in rows)==6 and not any('[~]' in l for l in rows)
doc=staged('docs/performance/RF-312-android-early-ipc-observer.md').decode('utf-8')
for link in re.findall(r'\]\(([^)]+)\)',doc):
    if '://' in link or link.startswith('#'):continue
    assert (R/'docs/performance'/link.split('#')[0]).is_file(),link
result={'acceptedStagedScope':True,'stagedPaths':paths,'archiveMembers':len(members),'sourceHashes':1843,
    'nativePositiveTests':2,'failures':0,'skipped':0,'nodeTests':12,'defaultDependenciesAndFeaturesKept':True,
    'allFailuresPreserved':True,'all22FinalPngsBound':True,'privateCopiesAbsent':True,
    'canonicalClosed':310,'total':322,'waiting':6,'pending':6,'fullRF312Accepted':False,'commitCreated':False}
(R/folder/'staged-verification.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print('Staged early-observer source and native success/failure evidence verified; full performance remains unclosed',flush=True)
