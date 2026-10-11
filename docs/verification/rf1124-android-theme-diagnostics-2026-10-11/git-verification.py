"""Verify the single RF1124 commit and immutable archived sources/results."""
from pathlib import Path
import hashlib,io,json,subprocess,sys,tarfile
r=Path('D:/SoloSoul');mode=sys.argv[1];assert mode in ('staged','head')
base='c714a71962c1336d999d5696f4ed91895baf7e71';stem='docs/verification/rf1124-android-theme-diagnostics-2026-10-11'
source='tauri/src-tauri/gen/android/app/src/androidTest/java/com/solosoul/app/AndroidThemeInstrumentedTest.kt'
expected={source,'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md','docs/performance/RF-121-android-theme-diagnostics.md',stem+'.json'}
expected|={stem+'/'+name for name in ['.gitattributes','index.json','raw.tar.gz','git-verification.py','cleanup.json','cleanup-plan.json','cleanup.ps1']}
assert len(expected)==11
def git(*args):return subprocess.check_output(['git',*args],cwd=r)
def blob(p):return git('show',(':' if mode=='staged' else 'HEAD:')+p)
def load(p):return json.loads(blob(p).decode('utf-8-sig'))
if mode=='staged':
    assert git('rev-parse','HEAD').decode().strip()==base
    changed=git('diff','--cached','--name-only','--ignore-submodules',base,'--').decode().splitlines()
else:
    assert git('rev-parse','HEAD^').decode().strip()==base
    assert 'RF-1124' in git('show','-s','--format=%s','HEAD').decode()
    changed=git('diff','--name-only','--ignore-submodules',base,'HEAD','--').decode().splitlines()
assert set(changed)==expected and len(changed)==11,changed
record=load(stem+'.json');assert record['task']=='RF-1124' and record['base']==base and record['status']=='diagnostic-verification-complete'
assert record['sourceCount']==1837 and record['checks']['budgetSeconds']==45
assert record['checks']['unit']==dict(tests=10,failures=0,errors=0,skipped=0)
assert record['checks']['originalThemeAcceptanceBodyUnchanged'] and record['checks']['originalLightDarkLightCommandsAndAssertionsUnchanged']
assert record['nativeThemeTests']==dict(api31Passed=0,api31Failed=2,api35Passed=2,api35Failed=0,skipped=0)
assert record['originalFailuresRetained'] and record['driverExternalOwnerErrorRecovered'] and record['faultBoundaryEstablished']
for field in ['uniqueRootCauseEstablished','productionBugFixed','fullRF121Accepted','fullRF312Accepted','performanceAccepted','physicalDeviceAccepted']:
    assert record[field] is False,field
assert len(record['observations'])==4
cleanup=load(stem+'/cleanup.json');plan=load(stem+'/cleanup-plan.json')
assert cleanup['task']=='RF-1124' and cleanup['removedFiles']==len(cleanup['removed'])==39 and not cleanup['recursiveRemoval']
assert cleanup['userProcessesTerminated']==0 and cleanup['ownedEmulatorsExited'] and len(cleanup['privatePairs'])==7
assert cleanup['removed']==plan['targets'] and all(x['contentsLinksAndModesMatched'] for x in cleanup['privatePairs'])
index=load(stem+'/index.json');raw=blob(stem+'/raw.tar.gz');digest=hashlib.sha256(raw).hexdigest()
assert digest==record['archive']['sha256']==index['archiveSha256']==cleanup['archiveSha256']==plan['archiveSha256']
assert len(index['members'])==record['archive']['members']==154
assert blob(stem+'/.gitattributes').replace(b'\r\n',b'\n')==b'* -text -diff\n'
with tarfile.open(fileobj=io.BytesIO(raw),mode='r:gz') as tf:
    assert len(tf.getmembers())==154 and set(tf.getnames())=={x['path'] for x in index['members']}
    for row in index['members']:
        member=tf.getmember(row['path']);assert member.isfile() and not member.issym() and not member.islnk()
        data=tf.extractfile(member).read();assert len(data)==row['size'] and hashlib.sha256(data).hexdigest()==row['sha256']
        assert not row['path'].endswith(('.tar','.apk')) and '/external-before/' not in row['path']
    final=tf.extractfile('final-source/'+source).read()
    assert hashlib.sha256(final).hexdigest()==record['finalSourceSha256']
    assert final.replace(b'\r\n',b'\n')==blob(source).replace(b'\r\n',b'\n')
ledger=blob('docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md').decode('utf-8-sig')
assert '已关闭：**304 / 314**；实际修复（已关闭）：304；' in ledger
assert '暂缓：0；进行中：0；待执行：6。' in ledger
assert '| 无（API31主题切换失败缺少分层证据） | [x] 完成 |' in ledger
assert '截图绘制同步尚未验收' in ledger
print(json.dumps(dict(mode=mode,paths=11,members=154,archiveSha256=digest,allMembersVerified=True,productionBugFixed=False,fullRF121Accepted=False)))
