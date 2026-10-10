from pathlib import Path
import datetime,hashlib,json,os,subprocess,time
root=Path('D:/SoloSoul'); base=Path(__file__).parent.resolve()
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
artifacts=load(base/'theme-current-token-test-build1/current-apks.json')
assert artifacts['accepted'] and not artifacts['deviceAcceptance']
sources=artifacts['sourceFiles']; assert len(sources)==1830
files={row['kind']:row for row in artifacts['files']}
def verify():
    for name,digest in sources.items():assert sha(root/name)==digest,name
    for row in files.values():assert sha(Path(row['copy']))==row['sha256']
verify()
stage=base/'api35-home-neutral-text-round1'; stage.mkdir(exist_ok=False)
def save(name,value):
    with (stage/name).open('x',encoding='utf-8') as out:json.dump(value,out,indent=2)
env=os.environ.copy()
env.update({'ANDROID_HOME':str(base/'sdk'),'ANDROID_SDK_ROOT':str(base/'sdk'),
    'ANDROID_USER_HOME':str(base/'android-user'),'ANDROID_EMULATOR_HOME':str(base/'android-user'),
    'ANDROID_AVD_HOME':str(base/'avd'),'ANDROID_ADB_SERVER_PORT':'5049','ADB_SERVER_SOCKET':'tcp:localhost:5049',
    'TEMP':str(base/'tmp'),'TMP':str(base/'tmp')})
env.pop('ANDROID_SDK_HOME',None)
adb=str(base/'sdk/platform-tools/adb.exe'); serial='emulator-5586'; avd='SoloSoul_RF121_20261010'
def read(args):
    return subprocess.run([adb,'-H','localhost','-P','5049','-s',serial,*args],env=env,
        check=True,capture_output=True,text=True,timeout=60,creationflags=subprocess.CREATE_NO_WINDOW).stdout.strip()
assert read(['get-state'])=='device' and read(['shell','getprop','sys.boot_completed'])=='1'
assert read(['emu','avd','name']).splitlines()[0]==avd
assert read(['shell','getprop','ro.build.version.sdk'])=='35'
save('input-proof.json',{'artifactReceiptSha256':sha(base/'theme-current-token-test-build1/current-apks.json'),
    'serial':serial,'avd':avd,'sourceFiles':sources,'files':artifacts['files'],
    'allBackupsLocalOnly':True,'physicalDeviceAcceptance':False})
rows=[]
for scenario in ['baseline','cold']:
 for mode in ['supported','fallback']:
    verify(); label=f'{scenario}-{mode}'
    settings_before={'blur':read(['shell','settings','get','global','disable_window_blurs']),
        'night':read(['shell','cmd','uimode','night']),'fontScale':read(['shell','settings','get','system','font_scale'])}
    command=['python','scripts/android-home-native-regression.py','--adb',adb,'--serial',serial,'--avd',avd,
        '--apk',files['main']['copy'],'--test-apk',files['test']['copy'],'--output',str(stage/label),
        '--mode',mode,'--scenario',scenario]
    if scenario=='cold':command+=['--motion-preferences']
    began=time.monotonic()
    with (stage/(label+'.stdout.log')).open('xb') as out,(stage/(label+'.stderr.log')).open('xb') as err:
        process=subprocess.Popen(command,cwd=root/'tauri',env=env,stdin=subprocess.DEVNULL,
            stdout=out,stderr=err,creationflags=subprocess.CREATE_NO_WINDOW)
        save(label+'.live.json',{'pid':process.pid,'command':command})
        print(f'API35 home {label} live PID {process.pid}',flush=True)
        code=process.wait()
    verify()
    report=load(stage/label/'report.json') if (stage/label/'report.json').is_file() else {}
    settings_after={'blur':read(['shell','settings','get','global','disable_window_blurs']),
        'night':read(['shell','cmd','uimode','night']),'fontScale':read(['shell','settings','get','system','font_scale'])}
    identity_after=read(['emu','avd','name']).splitlines()[0]==avd
    restored=bool(report.get('restored') and report.get('blurRestored') and
        report.get('notificationPermissionRestored') and settings_before==settings_after and identity_after and
        not any(report.get(k) for k in ['restoreError','blurRestoreError','notificationRestoreError','nightModeRestoreError','fontScaleRestoreError']))
    passed=bool(code==0 and report.get('passed') and restored and report.get('mode')==mode and
        report.get('scenario')==scenario and report.get('api')==35)
    row={'mode':mode,'scenario':scenario,'exitCode':code,'accepted':passed,'elapsedSeconds':time.monotonic()-began,
        'restored':restored,'settingsBefore':settings_before,'settingsAfter':settings_after,
        'sourceAndApkHashesUnchanged':True,'dedicatedAvdIdentityConfirmedAfter':identity_after,
        'reportSha256':sha(stage/label/'report.json') if report else None}
    save(label+'.receipt.json',row);rows.append(row)
    print(f'API35 home {label} terminal exit {code}; accepted={passed}; restored={restored}',flush=True)
    if not restored:
        save('verification.json',{'accepted':False,'results':rows,'stoppedForIncompleteRestoration':True})
        raise SystemExit(1)
accepted=len(rows)==4 and all(row['accepted'] for row in rows)
save('verification.json',{'accepted':accepted,'results':rows,'scope':'API35 baseline/cold in both native glass modes',
    'fullHomeMatrixAccepted':False,'api31Accepted':False,'at':datetime.datetime.now(datetime.timezone.utc).isoformat()})
raise SystemExit(0 if accepted else 1)
