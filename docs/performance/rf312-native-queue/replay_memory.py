from pathlib import Path
import hashlib,importlib.util,json,sys
from queue_context import load_context,ROOT
ctx=load_context();assert ctx.mode=='on'
R=J=ctx.output;native=ctx.native;W=ctx.workspace;P=W/'docs/performance'
sys.path[:0]=[str(P),str(P/'rf312-default-observer'),str(P/'rf312-ipc-runtime')]
import rf312_default_binding as binding
import rf312_instrumentation_state as state
import rf312_journey_memory as trajectory
from native_journey_contract import PHASES
read=lambda p:json.loads(Path(p).read_bytes())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path);value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value
assert read(J/'independent-current-state-r1.json')['passed']
# Physical UI replay is independent; the four-cell composer requires both audits.
runtime=ctx.loaded['runtime'];root=Path(runtime['host']['workspaceRoot'])
assert sha(root/'tauri/scripts/android-process-memory.py')==runtime['canonicalMemoryReaderSha256']
memory=module('current_serial_canonical_memory',root/'tauri/scripts/android-process-memory.py')
links=module('current_serial_native_links',ROOT/'native_links.py')
def numeric(value):
    if isinstance(value,dict):return {k:numeric(v)for k,v in value.items()if k!='collectionMs'}
    if isinstance(value,list):return [numeric(v)for v in value]
    return value
cases=[]
for count in (100,5000):
    p=native/f'journey-{count}'
    evidence=read(p/'default-external/supported-state-evidence.json')
    trace=read(p/'journey-memory/result.json');intent=read(p/'journey-memory/intent.json')
    assert trace==evidence['journeyMemory'] and trace['accepted'] and trace['workerStopped']
    assert trace['requestedBoundaryPhases']==trace['boundaryPhases']==trace['acknowledgedBoundaryPhases']==list(PHASES)
    assert trace['completeBoundaryCoverage'] and trace['scheduledPeriodicSlotsAccounted']
    assert trace['intervalSeconds']==2 and trace['maxPoints']==192
    for key in ('targetInstrumentationInjected','javaScriptInjected','inputRetried','exactContinuousPeakAccepted','applicationOnlyLatencyAccepted','performanceDistributionAccepted','fullRF312Accepted'):
        assert trace[key] is False
    assert trace['inputEventsSent']==0 and trace['workerErrorType'] is None
    assert intent['perPointMaximumSeconds']==20 and intent['perCommandMaximumSeconds']==10
    assert intent['originalUiDeadline']-trace['hostTraceStartedAt']<=240
    uid=read(p/'report.json')['originalInstalledPackages']['main']['uid']
    points=[read(file) for file in sorted((p/'journey-memory/points').glob('*/result.json'))]
    assert len(points)==trace['points']<=192 and points[0]['kind']=='prelaunch' and points[0]['accepted']
    sdk=read(p/'journey-memory-sdk/commands.json');cursor=0;values=[];boundaries=[];due=[];raws=0;physical_summary=[]
    for ordinal,point in enumerate(points,1):
        assert point['ordinal']==ordinal and point['inputRetried'] is False
        folder=p/'journey-memory/points'/f'{ordinal:03d}'
        if point['kind']=='missed-periodic':
            assert not point['accepted'] and point['commands']==[]
        else:
            assert read(folder/'commands.json')==point['commands']
        cursor,link_result=links.audit_links(folder,point,sdk,cursor,uid)
        physical_summary.append(link_result);raws+=link_result.get('logicalRawFiles',0)
        if point['kind'] in ('periodic','missed-periodic'):due.append(point['plannedDueAt'])
        if not point['accepted']:continue
        assert point['hostStartedAt']<=point['hostFinishedAt']<=point['deadline']<=intent['originalUiDeadline']
        assert point['deadline']-point['hostStartedAt']<=20
        commands=point['commands'];offset=1 if point['kind']=='prelaunch' else 0
        if offset:
            assert commands[0]['command']==['shell','pm','list','packages','--uid',str(uid)]
            assert (folder/'001.stdout').read_bytes().splitlines()==[f'package:com.solosoul.app uid:{uid}'.encode()]
        ps=['shell','ps','-A','-o','UID,PID,PPID,NAME']
        assert commands[offset]['command']==commands[-1]['command']==ps
        position=[offset+1]
        def replay(*args):
            index=position[0];assert list(args)==commands[index]['command'];position[0]+=1
            return (folder/f'{index+1:03d}.stdout').read_bytes().decode('utf-8').replace('\r\n','\n')
        parsed=memory.collect_snapshot(replay,'com.solosoul.app',uid,'com.android.webview')
        assert position[0]==len(commands)-1 and numeric(parsed)==numeric(point['snapshot'])
        owned={row['pid']:row['uid'] for row in parsed['processes']};uids={uid}|set(owned.values())
        for index in (offset,len(commands)-1):
            census=trajectory.census((folder/f'{index+1:03d}.stdout').read_bytes())
            assert {pid:row['uid'] for pid,row in census.items() if row['uid'] in uids}==owned
            assert not any(row['parentPid'] in owned and pid not in owned for pid,row in census.items())
        states=[]
        for index,command in enumerate(commands,1):
            assert command['exitCode']==0 and command['timedOut'] is False
            assert (folder/f'{index:03d}.stderr').read_bytes()==b''
            if command['command']==['shell','dumpsys','activity','processes']:
                rows=state.parse((folder/f'{index:03d}.stdout').read_bytes())
                state.expected(rows,{'org.solosoul.rf312observer/.Observer':'org.solosoul.rf312observer'},process_ids={'org.solosoul.rf312observer':trace['observerProcessId']})
                states.append(dict(commandOrdinal=index,nativeRows=rows,stdoutSha256=sha(folder/f'{index:03d}.stdout')))
        assert len(states)==2 and states==point['stateProofs']
        if offset:assert parsed['state']=='not-running' and parsed['pssBytes'] is None and not parsed['processes']
        else:
            main=[row for row in parsed['processes'] if row['role']=='app'];assert len(main)==1
            assert {key:main[0][key] for key in ('pid','uid','startTicks')}==trace['mainProcessIdentity']
            values.append(point)
        if point['kind']=='boundary':
            boundaries.append(point['phase']);phase=evidence['stages'][len(boundaries)-1]
            assert phase['name']==point['phase']
            marker=read(p/'journey-memory'/f"boundary-{len(boundaries):02d}.request.json")
            ack=read(p/'journey-memory'/f"boundary-{len(boundaries):02d}.ack.json")
            assert marker==phase['memoryBoundaryRequest'] and marker['phase']==point['phase'] and marker['ordinal']==len(boundaries)
            assert marker['uiWaitsForCapture'] is False
            assert marker['hostRequestedAt']<=point['hostStartedAt']<=point['hostFinishedAt']<=marker['captureDeadline']<=intent['originalUiDeadline']
            assert 0<marker['captureDeadline']-marker['hostRequestedAt']<=45
            assert ack==trace['boundaryAcknowledgements'][len(boundaries)-1]
            assert ack['phase']==point['phase'] and ack['captureDeadlineVerified'] and ack['synchronousAckRequired'] is False
            assert ack['memoryReceipt']==phase['memoryBoundary']
            assert point['hostFinishedAt']<=ack['hostAcknowledgedAt']<=intent['originalUiDeadline']

            assert phase['memoryBoundary']==dict(pointOrdinal=ordinal,receiptSha256=sha(folder/'result.json'),pssBytes=parsed['pssBytes'],hostStartedAt=point['hostStartedAt'],hostFinishedAt=point['hostFinishedAt'])
    assert cursor==len(sdk) and len(values)==trace['acceptedMemoryPoints'] and boundaries==list(PHASES)
    planned=[];slot=trace['hostTraceStartedAt']+2
    while slot<=trace['hostTraceFinishedAt']:planned.append(slot);slot+=2
    assert due==planned and len(set(due))==len(due)
    maximum=max(values,key=lambda point:point['snapshot']['pssBytes'])
    assert maximum['ordinal']==trace['maximumPointOrdinal'] and maximum['snapshot']['pssBytes']==trace['sampledMaximumSerialPssBytes']
    assert sum(not point['accepted'] for point in points)==trace['refusedPoints']
    assert sum(point['kind']=='missed-periodic' for point in points)==trace['missedPeriodicSlots']
    refusals=[]
    for point,physical in zip(points,physical_summary):
        if not point['accepted'] and point['kind']!='missed-periodic':
            assert point['kind']=='periodic'
            refusals.append(dict(pointOrdinal=point['ordinal'],kind=point['kind'],
                errorType=point.get('errorType'),numericSnapshotAccepted=False,
                provisionalSnapshotPresent='snapshot'in point,
                unconsumedPairMembers=physical['unusedMembers'],
                receiptSha256=sha(p/'journey-memory/points'/f"{point['ordinal']:03d}"/'result.json')))
    cases.append(dict(objectCount=count,uiStages=12,points=len(points),acceptedMemoryPoints=len(values),refusedPoints=trace['refusedPoints'],missedPeriodicSlots=trace['missedPeriodicSlots'],plannedPeriodicSlots=len(planned),logicalMemoryReads=sum(row['logicalReads']for row in physical_summary),physicalMemorySdkCalls=len(sdk),pairedSdkCalls=sum(row['pairedSdkCalls']for row in physical_summary),nativeRawFilesVerified=sum(row['nativeRawFiles']for row in physical_summary),unusedPairMembersRetained=sum(row['unusedMembers']for row in physical_summary),logicalRawFilesVerified=raws,boundaryPoints=12,canonicalMemoryAndUidCensusReplayed=True,sampledMaximumSerialPssBytes=trace['sampledMaximumSerialPssBytes'],maximumPointOrdinal=trace['maximumPointOrdinal'],traceSha256=sha(p/'journey-memory/result.json'),sdkLedgerSha256=sha(p/'journey-memory-sdk/commands.json')))
    cases[-1].update(refusedPeriodicCaptures=refusals,allPeriodicCapturesSucceeded=not refusals)
result=dict(passed=True,scope='current-complete-default-two-size-serial-memory-trajectories',sourceFiles=97,relatedTests=206,sourceSha256=binding.sources(),cases=cases,all24BoundaryPointsReplayed=True,allScheduledPeriodicSlotsAccounted=True,originalCanonicalMemoryAndStateParsersReplayed=True,actualPhysicalSdkFramesReplayed=True,inputRetried=False,exactContinuousPeakAccepted=False,applicationOnlyLatencyAccepted=False,performanceDistributionAccepted=False,fullRF312Accepted=False)
result['allPeriodicCapturesSucceeded']=all(row['allPeriodicCapturesSucceeded']for row in cases)
with(J/'independent-memory-r1.json').open('x',encoding='utf-8')as stream:json.dump(result,stream,indent=2)
print(json.dumps({k:v for k,v in result.items()if k!='sourceSha256'}))
