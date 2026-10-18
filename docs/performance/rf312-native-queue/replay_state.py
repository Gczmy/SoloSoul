from pathlib import Path
import collections,copy,hashlib,importlib.util,json,sys
from queue_context import load_context
ctx=load_context();W=ctx.workspace;R=J=ctx.output;H=W/'docs/performance/rf312-default-observer'
sys.path[:0]=[str(W/'docs/performance'),str(H)]
import rf312_default_binding as binding
import rf312_instrumentation_state as state
from native_journey_contract import PHASES,complete
read=lambda f:json.loads(Path(f).read_bytes())
sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path);value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value
P=ctx.native;parent=read(P/'result.json');b=ctx.loaded;runtime=b['runtime'];unit=ctx.units
assert parent['passed'] and parent['supportedDefaultStateAccepted'] and parent['fullJourneyAccepted']
assert parent['actualIpcAccepted'] is parent['performanceSampleAccepted'] is parent['fullRF312Accepted'] is False
assert parent['targetInstrumentationInjected'] is parent['metadataJavaScriptEvaluation'] is False
assert unit['passed'] and unit['sourceSha256']==binding.sources()
assert parent['allFrozenInputsPreserved']
for file,value in parent['frozenInputs'].items():assert sha(file)==value
root=Path(runtime['host']['workspaceRoot']);restore=module('independent_default_state_restore',root/'tauri/scripts/android-home-native-regression.py')
assert restore.inventory(P/'private-data-before.tar',require_empty=True)==restore.inventory(P/'private-data-restored.tar',require_empty=True)
memory=module('independent_original_strict_memory',root/'tauri/scripts/android-process-memory.py')
assert sha(root/'tauri/scripts/android-process-memory.py')==runtime['canonicalMemoryReaderSha256']
def numeric_snapshot(value):
    if isinstance(value,dict):return {k:numeric_snapshot(v) for k,v in value.items() if k!='collectionMs'}
    if isinstance(value,list):return [numeric_snapshot(v) for v in value]
    return value
cases=[]
for count in [100,5000]:
    p=P/f'journey-{count}';report=read(p/'report.json');evidence=read(p/'default-external/supported-state-evidence.json')
    assert report['passed'] and report['defaultStateAdapterAccepted'] and report['restored']
    assert report['originalApksRestored'] and report['blurRestored'] and report['ownedProbeRemoved'] and report['ownedPublicOcrInputRemoved'] and report['frozenInputsUnchanged']
    assert report.get('restoreError') is report.get('sourceRestoreError') is None
    assert report['targetInstrumentationInjected'] is report['javaScriptInjected'] is report['fullRF312Accepted'] is False
    assert evidence['objectCount']==count and evidence['stageNames']==list(PHASES);complete(evidence['stages'])
    assert evidence['success'] and evidence['fullUiJourneyAccepted'] and evidence['supportedNativeStateVerified'] and evidence['postJourneyDirectAbsenceVerified']
    assert evidence['targetInstrumentationInjected'] is evidence['javaScriptInjected'] is evidence['ipcAttributed'] is False
    before=restore.inventory(p/'private-data-before.tar',require_empty=True);after=restore.inventory(p/'private-data-restored.tar',require_empty=True);assert before==after
    proofs=sorted((p/'instrumentation-state').glob('*.proof.json'));active=[];observer_pid=None
    for number,file in enumerate(proofs,1):
        prefix=file.with_name(str(number).zfill(3));raw=prefix.with_suffix('.stdout').read_bytes();saved=read(file);command=read(prefix.with_suffix('.command.json'))
        assert command['command']==['shell','dumpsys','activity','processes'] and command['exitCode']==0
        assert prefix.with_suffix('.stderr').read_bytes()==b'' and sha(prefix.with_suffix('.stdout'))==saved['stdoutSha256']
        rows=state.parse(raw);assert rows==saved['nativeRows'] and saved['ordinal']==number
        expected={} if number in [1,len(proofs)] else {'org.solosoul.rf312observer/.Observer':'org.solosoul.rf312observer'}
        state.expected(rows,expected);active.append(len(rows))
        for row in rows:
            if observer_pid is None:observer_pid=row['processId']
            assert row['processId']==observer_pid
    assert active==[0]+[1]*(len(active)-2)+[0]
    point=p/'post-journey-memory';checkpoint=read(point/'result.json');commands=read(point/'commands.json')
    assert checkpoint['accepted'] and checkpoint['directAbsenceVerified'] and checkpoint['supportedStateChecks']==2 and checkpoint['readOnly']
    assert checkpoint['targetInstrumentationInjected'] is checkpoint['uiObserverActive'] is checkpoint['wholeJourneyMemoryDistributionAccepted'] is False
    uid=report['originalInstalledPackages']['main']['uid'];start=read(p/'default-external/observed-process-start.json')
    app=[row for row in checkpoint['snapshot']['processes'] if row['role']=='app'];assert len(app)==1 and app[0]['pid']==start['pid'] and app[0]['uid']==uid==start['uid']
    states=[]
    for number,row in enumerate(commands,1):
        assert row['exitCode']==0 and row['timedOut'] is False
        raw=(point/f'{number:03d}.stdout').read_bytes();assert (point/f'{number:03d}.stderr').read_bytes()==b''
        if row['command']==['shell','dumpsys','activity','processes']:
            parsed=state.parse(raw);state.expected(parsed,{});states.append(dict(ordinal=number,stdoutSha256=sha(point/f'{number:03d}.stdout'),active=0))
        assert row['command']!=['shell','dumpsys','activity','instrumentation']
    assert len(states)==4 and len(commands)==12+5*len(checkpoint['snapshot']['processes'])
    cursor=[4]
    def replay(*args):
        number=cursor[0];row=commands[number];assert list(args)==row['command'];cursor[0]+=1
        return (point/f'{number+1:03d}.stdout').read_bytes().decode('utf-8').replace('\r\n','\n')
    reparsed=memory.collect_snapshot(replay,'com.solosoul.app',uid,'com.android.webview')
    assert cursor[0]==len(commands)-4 and numeric_snapshot(reparsed)==numeric_snapshot(checkpoint['snapshot'])
    sdk=read(p/'commands.json')
    for entry in states:
        matches=[v for v in sdk if v['command']==['shell','dumpsys','activity','processes'] and v.get('stdoutSha256')==entry['stdoutSha256'] and v['exitCode']==0]
        assert len(matches)>=1
    assert report['ownedUiTransportEnabled'] is evidence['ownedUiTransportEnabled'] is True
    assert report['ownedUiPackageSha256']==b['config']['ownedUiPackageSha256']
    for folder in ['independent-observer','persistent-control','auxiliary-control/persistent-control']:assert read(p/folder/'cleanup.json')['accepted']
    cases.append(dict(objectCount=count,uiStages=12,uiStateChecks=len(active),stableObserverPidProven=True,
        mandatoryMemoryAbsenceChecks=2,actualMemoryProcessStateDumps=4,strictMemoryCommands=len(commands),
        canonicalMemoryParserReplayed=True,replayTimingCompared=False,pssBytes=checkpoint['snapshot']['pssBytes'],
        processRoles=[row['role'] for row in checkpoint['snapshot']['processes']],
        privateEntriesRestored=len(before),allOriginalStateRestored=True,
        checkpointResultSha256=sha(point/'result.json'),driverResultSha256=sha(p/'report.json'),stateProofsSha256=hashlib.sha256(json.dumps([sha(file) for file in proofs]).encode()).hexdigest(),memoryStateProofs=states))
after=P/parent['cleanup']['hostAfterReceipt'];canonical=module('independent_default_state_after',root/'tauri/scripts/android-fresh-host-audit.py')
audit=canonical.audit(read(after.parent/'expected.json'),read(after.parent/'host-snapshot.json'),'after')
assert audit['passed'] and audit['ownedProcessRows']==audit['listenerRows']==0
for key in ['ownedPackagesRemoved','settingsExactlyUnchanged','normalShutdownRequested','launcherExited','serverExited','freshHostAfterPassed']:assert parent['cleanup'][key] is True
assert '<no ANR has occurred since boot>' in next(P.glob('*terminal-health.stdout')).read_text(encoding='utf-8')
result=dict(passed=True,scope='default-release-supported-native-state-ui-and-post-memory-validation',sourceFiles=len(binding.sources()),sourceSha256=binding.sources(),unitTests=unit['tests'],skipped=unit['skipped'],cases=cases,
    completeUiStages=24,uiStateChecks=sum(row['uiStateChecks'] for row in cases),memoryStateDumps=8,
    targetInstrumentationInjected=False,javaScriptInjected=False,actualIpcAccepted=False,performanceDistributionAccepted=False,fullRF312Accepted=False,
    originalUnsupportedSubcommandNotReinterpreted=True,
    originalNumericalSamplesChanged=False,
    sourceInputsPreserved=True,allOriginalStateRestored=True,normalSdkShutdown=True,hostAfterReplayed=True,hostAfterSdkRows=0,hostAfterListeners=0,
    previousSourceGenerationsPooled=False,parentReceiptSha256=sha(P/'result.json'),hostAfterReceiptSha256=sha(after))
out=J/'independent-current-state-r1.json';assert not out.exists();out.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:v for k,v in result.items() if k not in ['cases','sourceSha256']}));print(json.dumps([{k:row[k] for k in ['objectCount','pssBytes','uiStateChecks','actualMemoryProcessStateDumps']} for row in cases]))
