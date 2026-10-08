import hashlib,json,os,plistlib,shutil,stat,subprocess,time,queue,threading
from types import SimpleNamespace
from pathlib import Path
ROOT=Path('/tmp/solosoul-fe2-ios-system-theme-ui-20261008')
UUID='362BC1AE-06B5-4ADE-BAA8-E36935C4AE2C'; APP='com.solosoul.app'
report={'scope':'Current iOS UI account cold restart and real appearance settings theme switching','passed':False,'commands':[],'restored':False}
def command(name,args,allow=(),timeout=60):
 if name=='native-ui-test':
  proc=subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  q=queue.Queue();lines=[];switched=False
  def read_output():
   for line in proc.stdout:q.put(line)
   q.put(None)
  threading.Thread(target=read_output,daemon=True).start()
  deadline=time.monotonic()+timeout
  try:
   while True:
    if time.monotonic()>deadline:raise TimeoutError('Native theme UI exceeded original timeout')
    try:line=q.get(timeout=1)
    except queue.Empty:continue
    if line is None:break
    lines.append(line)
    if 'FE2_SYSTEM_THEME_READY' in line and not switched:
     switched=True
     sim('system-to-dark','ui',UUID,'appearance','dark')
     assert sim('system-dark-readback','ui',UUID,'appearance')=='dark'
     report['system_transition_after_native_settings']=True
   r=SimpleNamespace(returncode=proc.wait(timeout=15),stdout=''.join(lines),stderr='')
   assert switched, 'UI never reached actual system-mode setting'
  except BaseException:
   proc.kill();proc.wait(timeout=15)
   (ROOT/(name+'.log')).write_text(''.join(lines))
   raise
 else:r=subprocess.run(args,capture_output=True,text=True,timeout=timeout)
 (ROOT/(name+'.log')).write_text(r.stdout+r.stderr)
 report['commands'].append({'name':name,'command':args,'exit_code':r.returncode})
 print(name,r.returncode,flush=True)
 if r.returncode not in (0,*allow):raise RuntimeError(name+' failed: '+r.stderr[-1000:])
 return r.stdout.strip()
def sim(name,*args,**kwargs):return command(name,['xcrun','simctl',*args],**kwargs)
def inventory(root):
 rows={'.':{'type':'directory','mode':stat.S_IMODE(root.stat().st_mode)}}
 for p in sorted(root.rglob('*')):
  mode=stat.S_IMODE(p.lstat().st_mode)
  if p.is_symlink():v={'type':'symlink','mode':mode,'target':os.readlink(p)}
  elif p.is_dir():v={'type':'directory','mode':mode}
  elif p.is_file():v={'type':'file','mode':mode,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
  else:raise RuntimeError('Unexpected private special file')
  rows[str(p.relative_to(root))]=v
 return rows
initial=None;container=None;before=None;backup=None;booted=False
try:
 devices=json.loads(sim('device-inventory','list','devices','available','--json'))
 entries=[d for ds in devices['devices'].values() for d in ds if d['udid']==UUID]
 assert len(entries)==1 and entries[0]['name']=='SoloSoul RF201' and entries[0]['isAvailable']
 initial=entries[0]['state']; report['initial_state']=initial
 assert initial=='Shutdown', 'No takeover of an already running simulator'
 sim('boot','boot',UUID);booted=True
 sim('boot-status','bootstatus',UUID,'-b',timeout=180)
 initial_appearance=sim('initial-appearance','ui',UUID,'appearance');report['initial_appearance']=initial_appearance
 container=Path(sim('data-container','get_app_container',UUID,APP,'data')).resolve()
 assert UUID in str(container) and '/Containers/Data/Application/' in str(container)
 if not container.is_dir():
  # The preceding accepted install returned this live sandbox, but reboot lookup is stale.
  valid=Path(Path('/tmp/solosoul-fe2-ios-control-font-ui-20261008/current-data-container.log').read_text().strip()).resolve()
  assert UUID in str(valid) and '/Containers/Data/Application/' in str(valid)
  original=Path('/tmp/solosoul-fe2-ios-control-font-ui-20261008/private-data-before')
  assert inventory(valid)==inventory(original),'Previously verified data changed while stopped'
  report['stale_data_container_lookup']=str(container);container=valid
 installed=Path(sim('installed-app','get_app_container',UUID,APP,'app'))
 if not (installed/'Info.plist').is_file():
  valid=Path(Path('/tmp/solosoul-fe2-ios-control-font-ui-20261008/current-installed-app.log').read_text().strip())
  assert UUID in str(valid) and '/Containers/Bundle/Application/' in str(valid)
  report['stale_app_container_lookup']=str(installed);installed=valid
 info=plistlib.loads((installed/'Info.plist').read_bytes())
 assert info['CFBundleIdentifier']==APP and info['CFBundleSupportedPlatforms']==['iPhoneSimulator']
 report['production_binary_sha256']=hashlib.sha256((installed/info['CFBundleExecutable']).read_bytes()).hexdigest()
 before=inventory(container);backup=ROOT/'private-data-before'
 shutil.copytree(container,backup,symlinks=True);assert inventory(backup)==before
 report['private_entries_before']=len(before)
 print('Full private backup verified',len(before),flush=True)
 current=Path('/tmp/solosoul-fe2-ios-control-font-build-20261008')
 build=json.loads((current/'report.json').read_text());assert build['passed']
 sim('install-current-app','install',UUID,build['frozen_app'])
 installed=Path(sim('current-installed-app','get_app_container',UUID,APP,'app'))
 info=plistlib.loads((installed/'Info.plist').read_bytes())
 assert hashlib.sha256((installed/info['CFBundleExecutable']).read_bytes()).hexdigest()==build['binary_sha256']
 report['previous_binary_sha256']=report['production_binary_sha256'];report['production_binary_sha256']=build['binary_sha256']
 # iOS may move the data container while retaining every byte; follow the verified bundle container.
 new_container=Path(sim('current-data-container','get_app_container',UUID,APP,'data')).resolve()
 assert UUID in str(new_container) and '/Containers/Data/Application/' in str(new_container)
 report['data_container_relocated']=new_container!=container
 container=new_container
 assert inventory(container)==before,'Installed app changed private contents before UI test'
 sim('launch-current-app','launch',UUID,APP)
 args=['xcodebuild','-project','/tmp/solosoul-fe2-ios-system-theme-ui-20261008/SoloSoulFE2UIProbe.xcodeproj','-scheme','Probe','-destination','platform=iOS Simulator,id='+UUID,'-derivedDataPath','/tmp/solosoul-fe2-ios-system-theme-ui-20261008/DerivedData','-parallel-testing-enabled','NO','-resultBundlePath',str(ROOT/'probe.xcresult'),'test-without-building','CODE_SIGNING_ALLOWED=NO']
 command('native-ui-test',args,timeout=420)
 report['passed']=True
except Exception as e:
 report['error']=str(e);print('ERROR',str(e),flush=True)
finally:
 if booted:
  try:
   sim('terminate-tested-app','terminate',UUID,APP,allow=(3,))
   if backup is not None and before is not None:
    for p in container.iterdir():
     if p.is_dir() and not p.is_symlink():shutil.rmtree(p)
     else:p.unlink()
    for p in backup.iterdir():
     dst=container/p.name
     if p.is_symlink():dst.symlink_to(os.readlink(p))
     elif p.is_dir():shutil.copytree(p,dst,symlinks=True)
     else:shutil.copy2(p,dst)
    os.chmod(container,before['.']['mode']);report['restored']=inventory(container)==before
    assert report['restored'],'Private data restoration mismatch'
   if report.get('initial_appearance') in ('light','dark'):
    sim('restore-appearance','ui',UUID,'appearance',report['initial_appearance'])
    report['appearance_restored']=sim('restored-appearance','ui',UUID,'appearance')==report['initial_appearance']
   sim('uninstall-probe-host','uninstall',UUID,'com.solosoul.fe2.ios.probehost',allow=(1,))
   sim('uninstall-probe-runner','uninstall',UUID,'com.solosoul.fe2.ios.probetests.xctrunner',allow=(1,))
  except Exception as e:report['restore_error']=str(e);report['passed']=False
  finally:
   try:sim('shutdown','shutdown',UUID);report['simulator_shutdown_restored']=True
   except Exception as e:report['shutdown_error']=str(e);report['passed']=False
 (ROOT/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:report.get(k) for k in ('passed','restored','error','restore_error','simulator_shutdown_restored')},ensure_ascii=False),flush=True)
raise SystemExit(0 if report['passed'] and report['restored'] and report.get('appearance_restored') and report.get('simulator_shutdown_restored') else 1)
