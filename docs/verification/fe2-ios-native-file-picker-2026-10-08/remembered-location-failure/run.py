import hashlib,json,os,plistlib,shutil,stat,subprocess,time
from pathlib import Path
ROOT=Path('/tmp/solosoul-fe2-ios-current-file-picker-public-fixtures-ui-20261008')
UUID='362BC1AE-06B5-4ADE-BAA8-E36935C4AE2C'; APP='com.solosoul.app'
report={'scope':'Current production iOS native file picker and real TXT import; no preview acceptance yet','passed':False,'commands':[],'restored':False}
def command(name,args,allow=(),timeout=60):
 r=subprocess.run(args,capture_output=True,text=True,timeout=timeout)
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
SOURCE_ROOT=Path('/Users/zzc/PycharmProjects/SoloSoul')
source_before=json.loads((SOURCE_ROOT/'docs/verification/fe2-frontend-checkpoint-2026-10-07.json').read_text())['source_sha256']
assert all(hashlib.sha256((SOURCE_ROOT/p).read_bytes()).hexdigest()==h for p,h in source_before.items())
(ROOT/'source-before-execution.json').write_text(json.dumps({'recorded_at':time.time(),'source_sha256':source_before},indent=2)+'\n')
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
  valid=Path(Path('/tmp/solosoul-fe2-ios-current-file-picker-provider-ui-20261008/current-data-container.log').read_text().strip()).resolve()
  assert UUID in str(valid) and '/Containers/Data/Application/' in str(valid)
  original=Path('/tmp/solosoul-fe2-ios-current-file-picker-provider-ui-20261008/private-data-before')
  assert inventory(valid)==inventory(original),'Previously verified data changed while stopped'
  report['stale_data_container_lookup']=str(container);container=valid
 installed=Path(sim('installed-app','get_app_container',UUID,APP,'app'))
 if not (installed/'Info.plist').is_file():
  valid=Path(Path('/tmp/solosoul-fe2-ios-current-file-picker-provider-ui-20261008/current-installed-app.log').read_text().strip())
  assert UUID in str(valid) and '/Containers/Bundle/Application/' in str(valid)
  report['stale_app_container_lookup']=str(installed);installed=valid
 info=plistlib.loads((installed/'Info.plist').read_bytes())
 assert info['CFBundleIdentifier']==APP and info['CFBundleSupportedPlatforms']==['iPhoneSimulator']
 report['production_binary_sha256']=hashlib.sha256((installed/info['CFBundleExecutable']).read_bytes()).hexdigest()
 before=inventory(container);backup=ROOT/'private-data-before'
 shutil.copytree(container,backup,symlinks=True);assert inventory(backup)==before
 report['private_entries_before']=len(before)
 print('Full private backup verified',len(before),flush=True)
 current=Path('/tmp/solosoul-fe2-ios-visual-offset-final-build-20261008')
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
 helper=ROOT/'DerivedData/Build/Products/Debug-iphonesimulator/ProbeHost.app'
 sim('install-fixture-host','install',UUID,str(helper))
 sim('launch-fixture-host','launch',UUID,'com.solosoul.fe2.ios.probehost')
 host=Path(sim('fixture-host-data','get_app_container',UUID,'com.solosoul.fe2.ios.probehost','data'))
 assert UUID in str(host) and host.is_dir()
 docs=host/'Documents'
 for p in (ROOT/'public-fixtures').iterdir():
  for attempt in range(100):
   if (docs/p.name).is_file():break
   time.sleep(0.1)
  assert (docs/p.name).is_file() and hashlib.sha256((docs/p.name).read_bytes()).hexdigest()==hashlib.sha256(p.read_bytes()).hexdigest()
 report['public_fixture_files']=[{'name':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted((ROOT/'public-fixtures').iterdir())]
 sim('terminate-fixture-host','terminate',UUID,'com.solosoul.fe2.ios.probehost')
 sim('launch-current-app','launch',UUID,APP)
 args=['xcodebuild','-project','/tmp/solosoul-fe2-ios-current-file-picker-public-fixtures-ui-20261008/SoloSoulFE2UIProbe.xcodeproj','-scheme','Probe','-destination','platform=iOS Simulator,id='+UUID,'-derivedDataPath','/tmp/solosoul-fe2-ios-current-file-picker-public-fixtures-ui-20261008/DerivedData','-parallel-testing-enabled','NO','-resultBundlePath',str(ROOT/'probe.xcresult'),'test-without-building','CODE_SIGNING_ALLOWED=NO']
 command('native-ui-test',args,timeout=420)
 report['source_differences_after_execution']=[p for p,h in source_before.items() if hashlib.sha256((SOURCE_ROOT/p).read_bytes()).hexdigest()!=h]
 assert not report['source_differences_after_execution']
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
