from pathlib import Path
import datetime, hashlib, json, os, socket, subprocess, time
base=Path(__file__).parent.resolve(); stage=base/'api35-host-gpu-round1'
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
save=lambda name,data:(stage/name).open('x',encoding='utf-8').write(json.dumps(data,indent=2))
old=next(p for p in load(base/'private-device-process-identities.json')['processes'] if p['Name']=='emulator.exe')
probe=subprocess.run(['pwsh','-NoProfile','-NonInteractive','-Command',
    'Get-CimInstance Win32_Process -Filter "ProcessId = 26600" | Select-Object Name,ProcessId,CreationDate,ExecutablePath,CommandLine | ConvertTo-Json'],
    check=True,capture_output=True,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
live=json.loads(probe.stdout)
assert live['ProcessId']==old['ProcessId']==26600 and live['ExecutablePath']==old['ExecutablePath']
assert datetime.datetime.fromisoformat(live['CreationDate'])==datetime.datetime.fromisoformat(old['CreationDate'])
assert live['CommandLine']==old['CommandLine']
assert load(base/'api35-native-remaining-seven-round1/verification.json')['results'][-1]['restorationErrorsAbsent']
save('stop-preflight.json',{'live':live,'previousNativeRestorationConfirmed':True})
env=os.environ.copy()
env.update({'ANDROID_HOME':str(base/'sdk'),'ANDROID_SDK_ROOT':str(base/'sdk'),
    'ANDROID_USER_HOME':str(base/'android-user'),'ANDROID_EMULATOR_HOME':str(base/'android-user'),
    'ANDROID_AVD_HOME':str(base/'avd'),'ANDROID_ADB_SERVER_PORT':'5049','ADB_SERVER_SOCKET':'tcp:localhost:5049',
    'TEMP':str(base/'tmp'),'TMP':str(base/'tmp')})
env.pop('ANDROID_SDK_HOME',None)
adb=str(base/'sdk/platform-tools/adb.exe'); serial='emulator-5586'; avd='SoloSoul_RF121_20261010'
def call(args,timeout=20):
    return subprocess.run([adb,'-H','localhost','-P','5049','-s',serial,*args],env=env,
        capture_output=True,timeout=timeout,creationflags=subprocess.CREATE_NO_WINDOW)
def read(args):
    result=call(args); assert result.returncode==0,(args,result.stderr.decode(errors='replace'))
    return result.stdout.decode().strip()
assert read(['emu','avd','name']).splitlines()[0]==avd
before={'blur':read(['shell','settings','get','global','disable_window_blurs']),
        'night':read(['shell','cmd','uimode','night'])}
assert before=={'blur':'null','night':'Night mode: no'}
config=base/'avd'/f'{avd}.avd'/'config.ini'
config_sha=hashlib.sha256(config.read_bytes()).hexdigest()
stopped=call(['emu','kill'])
save('previous-emulator-stop.json',{'exitCode':stopped.returncode,'stdout':stopped.stdout.decode(errors='replace'),
    'stderr':stopped.stderr.decode(errors='replace'),'verifiedPreviousPid':26600})
assert stopped.returncode==0
deadline=time.monotonic()+30
while True:
    free=True
    for port in [5586,5587]:
        try:
            with socket.socket() as stream: stream.bind(('127.0.0.1',port))
        except OSError: free=False
    if free:break
    assert time.monotonic()<deadline,'Dedicated emulator ports did not close'
    time.sleep(1)
command=[str(base/'sdk/emulator/emulator.exe'),'-avd',avd,'-port','5586','-accel','on','-gpu','host',
    '-memory','2048','-cores','2','-no-window','-no-snapshot','-no-audio','-no-boot-anim']
began=time.monotonic()
with (stage/'emulator.stdout.log').open('xb') as out,(stage/'emulator.stderr.log').open('xb') as err:
    process=subprocess.Popen(command,cwd=base,env=env,stdin=subprocess.DEVNULL,stdout=out,stderr=err,
        creationflags=subprocess.CREATE_NO_WINDOW)
    save('emulator.live.json',{'pid':process.pid,'command':command,'avd':avd,'serial':serial,
        'applicationAcceptance':False,'systemSecuritySettingsChanged':False})
    print(f'Owned API35 host-GPU emulator live PID {process.pid}',flush=True)
    attempts=[]; booted=False
    while time.monotonic()-began<240:
        if process.poll() is not None:break
        result=call(['shell','getprop','sys.boot_completed'])
        value=result.stdout.decode(errors='replace').strip()
        attempts.append({'exitCode':result.returncode,'value':value,'elapsedSeconds':time.monotonic()-began})
        if result.returncode==0 and value=='1':booted=True;break
        time.sleep(2)
    record={'booted':booted,'attempts':attempts,'elapsedSeconds':time.monotonic()-began,
        'pid':process.pid,'command':command,'applicationAcceptance':False,
        'physicalDevicePerformanceAccepted':False,'previousFailuresPreserved':True}
    if booted:
        assert read(['emu','avd','name']).splitlines()[0]==avd
        assert read(['shell','getprop','ro.build.version.sdk'])=='35'
        assert read(['shell','getprop','ro.product.cpu.abi'])=='x86_64'
        after={'blur':read(['shell','settings','get','global','disable_window_blurs']),
               'night':read(['shell','cmd','uimode','night'])}
        assert before==after
        record.update({'settingsBefore':before,'settingsAfter':after,'dedicatedAvdIdentityConfirmed':True})
        for label,args in [('surfaceflinger',['shell','dumpsys','SurfaceFlinger']),('webview',['shell','dumpsys','webviewupdate'])]:
            result=call(args)
            (stage/(label+'.log')).write_bytes(result.stdout)
            assert result.returncode==0
        assert hashlib.sha256(config.read_bytes()).hexdigest()==config_sha
        record['avdConfigUnchanged']=True
    else:record['emulatorExitCode']=process.poll()
    record['at']=datetime.datetime.now(datetime.timezone.utc).isoformat()
    save('boot.receipt.json',record)
    print(f'Host-GPU boot readiness={booted}; actual WebGL acceptance still required',flush=True)
    raise SystemExit(0 if booted else 1)
