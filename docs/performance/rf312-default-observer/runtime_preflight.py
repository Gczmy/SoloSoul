"""Collect independent fresh-boot evidence; require explicit inspection of both Launcher images."""
from pathlib import Path
import argparse,datetime as dt,hashlib,json,os,re,subprocess,sys
from owner_inputs import load_plan,verify_plan
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
save=lambda p,v:p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8')
def fresh_anr_rows(inventory,last,birth):
 rows=re.findall(r'^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d) (\S+) \(([^\n]*)\)$',inventory,re.M)
 count=re.search(r'^Drop box contents: (\d+) entries$',inventory,re.M)
 assert count and len(rows)==int(count[1]) and rows
 assert re.search(r'^Max entries: 1000$',inventory,re.M)
 assert [r[0] for r in rows]==sorted(r[0] for r in rows)
 prebirth=lambda stamp:dt.datetime.strptime(stamp,'%Y-%m-%d %H:%M:%S').replace(tzinfo=birth.tzinfo)<birth
 assert len(rows)<1000, 'A complete uncapped fresh inventory is required'
 assert not prebirth(rows[0][0]), 'Fresh AVD should contain only current-boot records'
 assert not any(tag.endswith('_anr') for _,tag,_ in rows), 'No ANR of any age may occur in fresh AVD'
 assert '<no ANR has occurred since boot>' in last
 return rows

def collect(plan_path,output):
 owner=load_plan(plan_path,artifacts=True);runtime=owner['runtime'];S=Path(output);P=S/'native-default-full-journey-r1'
 assert load(S/'owner-binding.json')['planSha256']==sha(plan_path)
 assert not (P/'continue-native.json').exists() and not (P/'reject-native.json').exists()
 out=P/'preflight-fresh-independent';out.mkdir(exist_ok=False)
 live=load(P/'live.json')
 shell=owner['config']['powershell']
 script='''$ErrorActionPreference='Stop'
 [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
 @(Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -eq SERVER_PID -or ($_.Name -like 'qemu-system-*' -and $_.ParentProcessId -eq LAUNCHER_PID) } | ForEach-Object {
  [pscustomobject]@{Name=$_.Name;ProcessId=$_.ProcessId;ParentProcessId=$_.ParentProcessId;CreationDate=$_.CreationDate.ToString('yyyy-MM-ddTHH:mm:ss.ffffffzzz');ExecutablePath=$_.ExecutablePath;CommandLine=$_.CommandLine}
 }) | ConvertTo-Json -Depth 4
 '''.replace('SERVER_PID',str(live['serverPid'])).replace('LAUNCHER_PID',str(live['emulatorLauncherPid']))
 (out/'capture-owned-host.ps1').write_text(script,encoding='utf-8')
 result=subprocess.run([shell,'-NoLogo','-NoProfile','-NonInteractive','-File',str(out/'capture-owned-host.ps1')],
     capture_output=True,timeout=45,creationflags=subprocess.CREATE_NO_WINDOW)
 (out/'host.stdout').write_bytes(result.stdout);(out/'host.stderr').write_bytes(result.stderr)
 result.check_returncode()
 owners=json.loads(result.stdout.decode('utf-8-sig'))
 assert len(owners)==2 and sum(r['ProcessId']==live['serverPid'] for r in owners)==1
 vm=next(r for r in owners if r['Name'].startswith('qemu-system-'))
 assert vm['ParentProcessId']==live['emulatorLauncherPid']
 assert Path(vm['ExecutablePath']).resolve().is_relative_to((Path(runtime['host']['sdkRoot'])/'emulator').resolve())
 birth=dt.datetime.fromisoformat(vm['CreationDate'])
 save(P/'fresh-owned-host-processes.json',owners)
 adb=Path(runtime['host']['adbExecutable'])
 def call(label,args):
     r=subprocess.run([str(adb),'-H','127.0.0.1','-P','5057','-s',runtime['host']['serial'],*args],
         capture_output=True,stdin=subprocess.DEVNULL,timeout=45,creationflags=subprocess.CREATE_NO_WINDOW)
     (out/(label+'.stdout')).write_bytes(r.stdout);(out/(label+'.stderr')).write_bytes(r.stderr)
     r.check_returncode();return r.stdout
 assert call('avd',['emu','avd','name']).decode().splitlines()[0]==runtime['host']['avd']
 assert call('api',['shell','getprop','ro.build.version.sdk']).strip()==b'31'
 assert call('abi',['shell','getprop','ro.product.cpu.abi']).strip()==b'x86_64'
 provider=call('webview',['shell','dumpsys','webviewupdate']).decode()
 assert '91.0.4472.114' in provider
 for package in ['com.solosoul.app','com.solosoul.app.test','com.solosoul.app.releaseprobe','org.solosoul.rf312observer']:
     assert not call('absent-'+package,['shell','pm','list','packages',package]).strip()
 inventory=call('dropbox',['shell','dumpsys','dropbox']).decode('utf-8').replace('\r\n','\n')
 call('system-app-anr',['shell','dumpsys','dropbox','--print','system_app_anr'])
 last=call('last-anr',['shell','dumpsys','activity','lastanr']).decode('utf-8')
 rows=fresh_anr_rows(inventory,last,birth)
 prepared=load(Path(owner['config']['avdPreparation']))
 assert prepared['createdNewDedicatedAvd'] and prepared['freshSdkUserdata'] and not prepared['oldUserdataCopied']
 assert prepared['name']==runtime['host']['avd']
 verify_plan(owner,artifacts=True)
 windows=call('windows',['shell','dumpsys','window']).decode('utf-8')
 focus=[r.strip() for r in windows.splitlines() if 'mCurrentFocus=' in r]
 assert len(focus)==1 and 'com.android.launcher3' in focus[0]
 png=call('screenshot',['exec-out','screencap','-p'])
 assert png.startswith(b'\x89PNG\r\n\x1a\n')
 (out/'preflight.png').write_bytes(png)
 assert sha(P/'preflight.png')==load(P/'environment-preflight.json')['pngSha256']
 save(out/'result.json',dict(passed=True,currentVmBirth=vm['CreationDate'],currentBootAnrAbsent=True,
     completeDropBoxEntries=len(rows),allFourProductObserverPackagesAbsent=True,focus=focus,
     originalPngSha256=sha(P/'preflight.png'),latestPngSha256=sha(out/'preflight.png'),
     freshSdkUserdataVerified=True,originalCompleteFreshAnrRuleRetained=True,screenshotInspectionStillRequired=True,api=31,abi='x86_64',webView='91.0.4472.114'))

 print(json.dumps(dict(passed=True,entries=len(rows),screenInspectionStillRequired=True)))
 return 0
def review(plan_path,output,original,latest,result,inspected):
 owner=load_plan(plan_path,artifacts=True);S=Path(output);P=S/'native-default-full-journey-r1';out=P/'preflight-fresh-independent'
 assert inspected and load(S/'owner-binding.json')['planSha256']==sha(plan_path)
 evidence=load(out/'result.json');assert evidence['passed'] and evidence['currentBootAnrAbsent']
 assert original==sha(P/'preflight.png')==evidence['originalPngSha256']
 assert latest==sha(out/'preflight.png')==evidence['latestPngSha256'] and result==sha(out/'result.json')
 assert not (P/'continue-native.json').exists() and not (P/'reject-native.json').exists()
 value=dict(pngSha256=original,latestPngSha256=latest,independentResultSha256=result,screenInspected=True,
  blockingSystemErrorDialogAbsent=True,systemAnrEvidenceReviewed=True)
 with (P/'continue-native.json').open('x',encoding='utf-8') as stream:stream.write(json.dumps(value,indent=2)+'\n')
 print(json.dumps(dict(passed=True,screenshotInspectionRecorded=True)));return 0
def main(argv=None):
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('action',choices=['collect','review'])
 parser.add_argument('--plan',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
 parser.add_argument('--original-png-sha256');parser.add_argument('--latest-png-sha256');parser.add_argument('--independent-result-sha256')
 parser.add_argument('--screens-inspected',action='store_true');args=parser.parse_args(argv)
 if args.action=='collect':return collect(args.plan,args.output)
 return review(args.plan,args.output,args.original_png_sha256,args.latest_png_sha256,args.independent_result_sha256,args.screens_inspected)
if __name__=='__main__':sys.exit(main())
