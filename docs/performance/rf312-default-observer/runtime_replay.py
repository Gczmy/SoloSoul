"""Replay original raw control frames and strict whole memory checkpoints offline."""
from pathlib import Path
import argparse,base64,hashlib,importlib.util,io,json,re,subprocess,sys,xml.etree.ElementTree as ET
from owner_inputs import load_plan
import body_navigation as nav
import memory_checkpoint as checkpoint
SOURCE=Path(__file__).resolve().parent
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def assert_child_arguments(owner,stage,count):
 runtime=owner['runtime'];host=runtime['host']
 actual=load(stage/f'prototype-{count}.live.json')['command']
 expected=[str(SOURCE/'default_driver.py'),'--adb',host['adbExecutable'],'--serial',host['serial'],'--avd',host['avd'],
  '--apk',runtime['release']['apk']['path'],'--restore-apk',runtime['originalDebug']['copy'],'--test-apk',runtime['runtimeTest']['path'],
  '--fixture',runtime['fixedFixtures'][str(count)]['root'],'--inputs',owner['config']['runtimeInputs'],'--object-count',str(count),
  '--mode','supported','--output',str(stage/f'prototype-{count}')]
 assert actual[1:]==expected
def frames(owner,S):
 original=load(S/'independent-terminal-audit.json');assert original['passed']
 spec=importlib.util.spec_from_file_location('frozen_control_frames',SOURCE/'persistent_control.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
 cases=[]
 for item in load(S/'external-terminal-r2.json')['outcomes']:
     out=S/('prototype-'+str(item['objectCount']));control=out/'persistent-control';observer=out/'independent-observer'
     cleanup=load(control/'cleanup.json')
     assert cleanup['accepted'] and cleanup['normalControlClose'] and cleanup['ownedRemoteScratchRemoved'] and cleanup['originalCallbackRestored']
     assert cleanup['controlExitCode']==0 and cleanup['errors']==[] and not cleanup['inputRetried']
     assert (control/'rpc/transport.stderr').read_bytes()==b''
     files=sorted(p for p in (control/'rpc').glob('*.json') if re.fullmatch('[0-9]{5}\\.json',p.name))
     assert len(files)==cleanup['rpcFrames']<=16384
     puts=set();responses=images=errors=0;digests=[]
     for number,path in enumerate(files,1):
         meta=load(path);wire=path.with_suffix('.request').read_bytes();raw=path.with_suffix('.reply').read_bytes()
         assert meta['sequence']==number and meta['requestSha256']==hashlib.sha256(wire).hexdigest() and meta['replySha256']==hashlib.sha256(raw).hexdigest()
         magic,sequence,op,name,data=wire.decode('ascii').strip().split(' ')
         assert magic=='RF312B' and int(sequence)==number
         payload=base64.b64decode(data,validate=True) if op=='put' else b''
         assert module.request_line(number,op,name,payload)==wire
         partial=bytearray();stream=io.BytesIO(raw);code,stdout,stderr=module.read_reply(stream,number,partial)
         assert stream.read()==b'' and bytes(partial)==raw and code==meta['exitCode']
         assert hashlib.sha256(stdout).hexdigest()==meta['stdoutSha256'] and hashlib.sha256(stderr).hexdigest()==meta['stderrSha256']
         if code:errors+=1
         if op=='put':
             assert name not in puts and 1<=int(name[8:11])<=512
             puts.add(name);assert json.loads(payload)==load(observer/name)
         if op=='read' and re.fullmatch('response-[0-9]{3}\\.json',name) and code==0:
             assert (observer/name).read_bytes()==stdout;responses+=1
         if op=='read' and re.fullmatch('frame-[0-9]{3}\\.png',name) and code==0:
             assert (observer/name).read_bytes()==stdout;images+=1
         digests.append([meta['requestSha256'],meta['replySha256']])
     requests=load(observer/'cleanup.json')['requests'];assert 0<len(puts)<=requests<=512
     if item['passed']:assert len(puts)==requests
     assert files and load(files[0])['operation']=='ping' and load(files[-1])['operation']=='close'
     deferrals=[];native_scrolls=0
     for path in sorted(observer.glob('request-*.json')):
         request=load(path);response_path=path.with_name(path.name.replace('request-','response-'))
         if not response_path.is_file():continue
         response=load(response_path)
         if request['action']=='scroll':
             assert response['singleFingerGesture'] and response['direction']==request['direction']
             native_scrolls+=1
         if request['action']=='inspect-target' and request['intent']=='scroll':
             ready=all(response[k] for k in ['matched','targetEnabled','targetVisible','targetScrollable'])
             if not ready:
                 assert response['inputDispatched'] is False
                 number=int(path.stem.split('-')[1]);following=observer/f'request-{number+1:03d}.json'
                 if following.is_file():assert load(following)['action']=='snapshot'
                 deferrals.append(number)
     cases.append(dict(objectCount=item['objectCount'],nativeJourneyAccepted=item['passed'],rpcFrames=len(files),
         unchangedOriginalObserverRequests=requests,exactOriginalRequestPayloads=len(puts),
         readOnlyScrollDeferrals=len(deferrals),deferredChecksNeverDispatchedInput=True,nativeScrollGestures=native_scrolls,
         exactSavedResponseReads=responses,exactSavedPngReads=images,nonzeroExitFramesRetained=errors,
         originalCallbackRestored=True,controlAndObserverCleanupAccepted=True,
         framesDigestSha256=hashlib.sha256(json.dumps(digests,separators=(',',':')).encode()).hexdigest()))
 result=dict(passed=True,cases=cases,fullUiQueueAccepted=original['fullUiQueueAccepted'],
  fullDefaultJourneyAccepted=False,performanceDistributionAccepted=False,rawControlFramesReplayed=True,
  exactOriginalPayloadsAndRepliesBound=True,allInputCapsAndDeadlinesUnchanged=True)
 with (S/'independent-runtime-frame-audit.json').open('x',encoding='utf-8') as stream:json.dump(result,stream,indent=2)
 return result

def memory(owner,S):
 path=Path(owner['runtime']['host']['workspaceRoot'])/'tauri/scripts/android-process-memory.py'
 assert sha(path)==owner['runtime']['canonicalMemoryReaderSha256']
 spec=importlib.util.spec_from_file_location('replayed_canonical_strict_memory',path)
 memory=importlib.util.module_from_spec(spec);spec.loader.exec_module(memory)
 assert load(S/'independent-runtime-frame-audit.json')['passed']
 assert load(S/'independent-terminal-audit.json')['fullUiQueueAccepted']
 def without_times(value):
     if isinstance(value,dict):return {k:without_times(v) for k,v in value.items() if k not in ['collectionMs','elapsedMs']}
     if isinstance(value,list):return [without_times(v) for v in value]
     return value

 cases=[]
 for count in [100,5000]:
     out=S/f'prototype-{count}';report=load(out/'report.json');folder=out/'post-journey-memory'
     actual=load(folder/'result.json');records=load(folder/'commands.json');ledger=load(out/'commands.json')
     assert report['runtimeInputsSha256']==owner['config']['runtimeInputsSha256']
     assert report['runtimeSourcesSha256']==owner['runtime']['fullJourneyFiles']
     assert_child_arguments(owner,S,count)
     assert actual['accepted'] and actual['elapsedMs']<20000 and len(records)==22
     assert all(r['exitCode']==0 and not r['timedOut'] and 0<r['timeoutSeconds']<=10 for r in records)
     wanted=[r['command'] for r in records]
     starts=[i for i in range(len(ledger)-21) if [r['command'] for r in ledger[i:i+22]]==wanted]
     assert len(starts)==1
     for number in range(1,23):
         sdk=ledger[starts[0]+number-1];assert sdk['exitCode']==0
         assert sdk['stdoutSha256']==sha(folder/f'{number:03d}.stdout') and sdk['stderrSha256']==sha(folder/f'{number:03d}.stderr')
         assert (folder/f'{number:03d}.stderr').read_bytes()==b''
     pending=list(enumerate(records,1))
     def replay(*command,**kwargs):
         number,row=pending.pop(0);assert row['command']==list(command)
         assert 0<kwargs['timeout']<=10 and kwargs['stdout']==subprocess.PIPE and kwargs['stderr']==subprocess.PIPE
         return subprocess.CompletedProcess(command,row['exitCode'],(folder/f'{number:03d}.stdout').read_bytes(),(folder/f'{number:03d}.stderr').read_bytes())
     root=next(r for r in actual['snapshot']['processes'] if r['role']=='app' and r['name']==nav.APP)
     assert root['uid']==report['originalInstalledPackages']['main']['uid']
     replayed=checkpoint.collect(memory.collect_snapshot,replay,out/'post-memory-independent-replay',uid=root['uid'],pid=root['pid'])
     assert not pending and without_times(replayed)==without_times(actual)
     observer=out/'independent-observer';requests=[(int(p.stem.split('-')[1]),load(p)) for p in sorted(observer.glob('request-*.json'))]
     tools=[]
     for tool in ['OCR','Search']:
         taps=[(number,r) for number,r in requests if r['action']=='tap' and r.get('target',{}).get('text')==tool
               and r['target']['class']=='android.widget.Button']
         assert len(taps)==1
         ordinal,tap=taps[0]
         start=max(number for number,r in requests if number<ordinal and r['action']=='tap' and r.get('target',{}).get('content-desc')=='Tools' and r['target']['class']=='android.view.View' and r['target']['package']==nav.APP)
         states=[]
         for number,r in requests:
             if not start<number<ordinal or r['action']!='snapshot':continue
             path=observer/f'response-{number:03d}.json';row=load(path)
             assert row['allowedRoot'] and row['rootPackage']==nav.APP
             nodes=list(ET.fromstring(row['xml']).iter('node'))
             states.append(dict(number=number,ready=nav.tools_layout_ready(nodes),sha256=sha(path),
                 reminderVisible=any(n.get('package')==nav.APP and n.get('visible-to-user')=='true'
                     and any(' '.join(v.split()).startswith(nav.BACKUP_REMINDER) for v in nav.labels(n)) for n in nodes)))
         ready=[r for r in states if r['ready']];assert ready
         first=ready[0]['number']
         assert all(r['action']=='snapshot' for number,r in requests if start<number<first)
         assert all(not r['reminderVisible'] for r in ready)
         response=load(observer/f'response-{ordinal:03d}.json');assert response['success']
         tools.append(dict(tool=tool,observedStates=len(states),visibleReminderSnapshots=sum(r['reminderVisible'] for r in states),
             notReadySnapshotsBeforeAdmission=sum(not r['ready'] and r['number']<first for r in states),
             oneExactToolTap=True,noInputBeforeReadOnlyAdmission=True,
             hierarchyBindingsSha256=hashlib.sha256(json.dumps(states,separators=(',',':')).encode()).hexdigest()))
     cases.append(dict(objectCount=count,checkpointAccepted=True,memoryCommands=22,strictWholeCheckpointReplayed=True,
         originalSdkCommandOutputsBound=True,pssBytes=actual['snapshot']['pssBytes'],checkpointElapsedMs=actual['elapsedMs'],tools=tools))
 result=dict(passed=True,cases=cases,twoSizeCheckpointAccepted=True,strictWholeCheckpointReplayed=True,
  originalSdkOutputsBound=True,fullDefaultJourneyAccepted=False,performanceDistributionAccepted=False,
  wholeJourneyMemoryDistributionAccepted=False)
 with (S/'independent-runtime-memory-audit.json').open('x',encoding='utf-8') as stream:json.dump(result,stream,indent=2)
 return result

def main(argv=None):
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--plan',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
 args=parser.parse_args(argv);owner=load_plan(args.plan,artifacts=True)
 first=frames(owner,args.output);second=memory(owner,args.output)
 print(json.dumps(dict(passed=first['passed'] and second['passed'],twoSizeCheckpointAccepted=second['twoSizeCheckpointAccepted'])));return 0
if __name__=='__main__':sys.exit(main())
