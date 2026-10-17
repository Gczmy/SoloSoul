"""Run the unchanged two-size child queue for one explicit owner plan."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,subprocess,sys,time
from native_journey_contract import PHASES
from owner_inputs import load_plan
SOURCE=Path(__file__).resolve().parent
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
save=lambda p,v:p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(plan_path,output):
 owner=load_plan(plan_path,artifacts=True);inputs=owner['runtime'];S=Path(output);W=Path(inputs['host']['workspaceRoot'])
 plan=inputs['host']
 spec=importlib.util.spec_from_file_location('canonical_empty_backup_audit',W/'tauri/scripts/android-home-native-regression.py')
 home=importlib.util.module_from_spec(spec);spec.loader.exec_module(home)
 result=dict(scope='default-release-independent-twelve-step-ui-journey',prototypePassed=False,outcomes=[],
  fullDefaultJourneyAccepted=False,performanceDistributionAccepted=False)
 for count in (100,5000):
  out=S/f'prototype-{count}';command=[sys.executable,str(SOURCE/'default_driver.py'),
   '--adb',inputs['host']['adbExecutable'],'--serial',plan['serial'],'--avd',plan['avd'],
   '--apk',inputs['release']['apk']['path'],'--restore-apk',inputs['originalDebug']['copy'],
   '--test-apk',inputs['runtimeTest']['path'],'--fixture',inputs['fixedFixtures'][str(count)]['root'],
   '--inputs',owner['config']['runtimeInputs'],'--object-count',str(count),'--mode','supported','--output',str(out)]
  started=time.monotonic()
  with (S/f'prototype-{count}.stdout.log').open('xb') as stdout,(S/f'prototype-{count}.stderr.log').open('xb') as stderr:
   child=subprocess.Popen(command,cwd=W,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
   save(S/f'prototype-{count}.live.json',dict(pid=child.pid,command=command))
   code=child.wait()
  row=dict(objectCount=count,exitCode=code,elapsedSeconds=time.monotonic()-started,allOriginalStateRestored=False,passed=False)
  try:
   report=load(out/'report.json')
   before=home.inventory(out/'private-data-before.tar',require_empty=True)
   after=home.inventory(out/'private-data-restored.tar',require_empty=True)
   row['originalPrivateEntries']=len(before)
   row['allOriginalStateRestored']=bool(before==after and report.get('restored') and report.get('originalApksRestored')
    and report.get('blurRestored') and report.get('ownedProbeRemoved') and report.get('ownedPublicOcrInputRemoved')
    and report.get('frozenInputsUnchanged') and not report.get('restoreError') and not report.get('sourceRestoreError'))
   evidence=load(out/'default-external/evidence.json');cleanup=load(out/'independent-observer/cleanup.json')
   row['passed']=bool(code==0 and report['passed'] and evidence['success'] and cleanup['accepted']
    and row['allOriginalStateRestored'] and evidence['objectCount']==count
    and evidence['stageNames']==list(PHASES) and evidence['fullUiJourneyAccepted'] is True
    and not evidence['targetInstrumentationInjected'] and not evidence['javaScriptInjected']
    and not evidence['fullDefaultJourneyAccepted'] and not evidence['performanceDistributionAccepted'])
  except Exception as error:row['auditError']=repr(error)
  result['outcomes'].append(row);save(S/'external-progress.json',result)
  print(json.dumps(row),flush=True)
  if not row['passed']:break
 result['prototypePassed']=len(result['outcomes'])==2 and all(r['passed'] for r in result['outcomes'])
 save(S/'external-terminal-r2.json',result)
 print(json.dumps(result),flush=True)

 return 0 if result['prototypePassed'] else 1
def main(argv=None):
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--plan',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
 args=parser.parse_args(argv);return run(args.plan,args.output)
if __name__=='__main__':sys.exit(main())
