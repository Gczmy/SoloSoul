"""Strict, separate document-clock milestone metadata; never a native frame or TTI."""
import hashlib,json,math,re
import frame_receipt as frame

READY_SCOPE='native-login-startup-milestone-probe'
SCOPE='native-login-startup-milestone-observation'
NATIVE_CLOCK='System.nanoTime; same target process; not converted to document clock'
DOCUMENT_CLOCK='performance.timeOrigin and performance.now; no cross-clock conversion'
DOCUMENT_FIELDS={'schemaVersion','scope','mainFrame','childFrames','localOrigin','loginRoute','documentVisible',
    'timeOriginMs','observedAtMs','entryQueryAvailable','supportedEntryTypesAvailable','markEntrySupported',
    'paintEntrySupported','navigationEntrySupported','reactMount','startupDismissed','firstPaint','firstContentfulPaint','navigation'}
FIELDS={'schemaVersion','scope','runId','process','targetApkSha256','collectorApkSha256','api','startedNs','deadlineNs','completedNs',
    'nativeClock','documentClock','hardwareAccelerated','nativeWindowFocused','width','height','documentBefore','documentAfter','milestones',
    'callbacksDetached','explicitDrawRequestsDispatched','forcedInvalidation','inputDispatched','businessIpcDispatched',
    'nativeFramesAccepted','applicationOnlyLatencyAccepted','physicalDisplayPresentationProven','defaultPerformanceSampleAccepted','fullRF312Accepted'}
FALSE={'explicitDrawRequestsDispatched','forcedInvalidation','inputDispatched','businessIpcDispatched','nativeFramesAccepted',
       'applicationOnlyLatencyAccepted','physicalDisplayPresentationProven','defaultPerformanceSampleAccepted','fullRF312Accepted'}
NAVIGATION_FIELDS={'startTimeMs','durationMs','domInteractiveMs','domContentLoadedEndMs','loadEventEndMs'}

def number(value):
    frame.require(type(value) in [int,float] and math.isfinite(value) and value>=0,'Finite nonnegative document clock required')
    return value

def ready_contract(raw,run,app,collector,uid):
    value=frame.decode(raw);frame.fields(value,frame.READY_FIELDS)
    frame.require(type(value['schemaVersion']) is int and value['schemaVersion']==1 and value['scope']==READY_SCOPE,
                  'Separate milestone ready scope required')
    for key,expected,pattern in [('runId',run,'[a-f0-9]{32}'),('targetApkSha256',app,'[a-f0-9]{64}'),
                                  ('collectorApkSha256',collector,'[a-f0-9]{64}')]:
        frame.require(type(value[key]) is str and value[key]==expected and re.fullmatch(pattern,value[key]) is not None,
                      'Original milestone identity required')
    frame.process(value['process'],value['process'])
    frame.require(value['process']['uid']==frame.integer(uid,10000),'Original milestone UID required')
    frame.require(frame.integer(value['deadlineNs'])-frame.integer(value['startedNs'])==frame.STAGE_NS,'Original 45s deadline required')
    return value

def document(value):
    frame.fields(value,DOCUMENT_FIELDS)
    frame.require(type(value['schemaVersion']) is int and value['schemaVersion']==1 and value['scope']=='document-startup-milestones',
                  'Exact document milestone schema required')
    frame.require(type(value['childFrames']) is int and value['childFrames']==0,'Single main document required')
    for key in ['mainFrame','localOrigin','loginRoute','documentVisible']:frame.require(value[key] is True,'Usable local login required')
    for key in ['entryQueryAvailable','supportedEntryTypesAvailable','markEntrySupported','paintEntrySupported','navigationEntrySupported']:
        frame.require(type(value[key]) is bool,'Explicit API availability required')
    if not value['supportedEntryTypesAvailable']:
        frame.require(not any(value[k] for k in ['markEntrySupported','paintEntrySupported','navigationEntrySupported']),
                      'Unknown API support cannot be invented')
    now=number(value['observedAtMs']);frame.require(number(value['timeOriginMs'])>0,'Positive document time origin required');result={}
    for name in ['reactMount','startupDismissed','firstPaint','firstContentfulPaint','navigation']:
        group=value[name];frame.fields(group,{'count','entries'})
        count=frame.integer(group['count'],0,1_000_000);entries=group['entries']
        frame.require(type(entries) is list and len(entries)==min(count,4),'Exact bounded entry prefix required')
        frame.require(count<=1,'Missing or unique native-buffer entry required; duplicates are ambiguous')
        frame.require(value['entryQueryAvailable'] or count==0,'Unavailable query cannot contain fabricated entries')
        if name=='navigation':
            for entry in entries:
                frame.fields(entry,NAVIGATION_FIELDS)
                for part in entry.values():frame.require(number(part)<=now,'Navigation event beyond observation')
                frame.require(entry['startTimeMs']==0,'Same-document navigation origin required')
                for earlier,later in [('domInteractiveMs','domContentLoadedEndMs'),('domContentLoadedEndMs','loadEventEndMs')]:
                    if entry[later]>0:frame.require(entry[earlier]<=entry[later],'Navigation event order differs')
            result[name]=entries[0] if entries else None
        else:
            for at in entries:frame.require(number(at)<=now,'Milestone cannot occur after observation')
            result[name+'Ms']=entries[0] if entries else None
    mount,dismiss=result['reactMountMs'],result['startupDismissedMs']
    if mount is not None and dismiss is not None:frame.require(dismiss>=mount,'Handoff must follow mount invocation')
    fp,fcp=result['firstPaintMs'],result['firstContentfulPaintMs']
    if fp is not None and fcp is not None:frame.require(fcp>=fp,'Contentful paint precedes first paint')
    result['requiredMilestonesAvailable']=mount is not None and dismiss is not None
    result['mountInvocationToHandoffMs']=dismiss-mount if result['requiredMilestonesAvailable'] else None
    return result

def validate(ready,record,*,run_id,expected_process,target_apk_sha256,collector_apk_sha256,expected_api):
    ready_contract(json.dumps(ready).encode(),run_id,target_apk_sha256,collector_apk_sha256,expected_process['uid'])
    frame.fields(record,FIELDS);frame.process(record['process'],expected_process);frame.process(ready['process'],expected_process)
    frame.require(type(record['schemaVersion']) is int and record['schemaVersion']==1 and record['scope']==SCOPE,
                  'Separate native milestone observation required')
    for name in ['runId','targetApkSha256','collectorApkSha256','startedNs','deadlineNs']:
        frame.require(type(record[name]) is type(ready[name]) and record[name]==ready[name],'Original milestone binding required')
    frame.require(frame.integer(record['api'],29,100)==frame.integer(expected_api,29,100),'Original native API required')
    frame.require(record['startedNs']<=frame.integer(record['completedNs'])<record['deadlineNs'],'Original native stage exceeded')
    frame.require(record['nativeClock']==NATIVE_CLOCK and record['documentClock']==DOCUMENT_CLOCK,'Separate unconverted clocks required')
    for name in FALSE:frame.require(record[name] is False,'Milestones cannot claim input, native frames or application-only latency')
    for name in ['hardwareAccelerated','nativeWindowFocused','callbacksDetached']:frame.require(record[name] is True,'Native identity/focus/cleanup required')
    for name in ['width','height']:frame.integer(record[name],1,16384)
    before,after=record['documentBefore'],record['documentAfter'];frame.document(before);frame.document(after)
    value=record['milestones'];metrics=document(value)
    frame.require(before['timeOriginMs']==value['timeOriginMs']==after['timeOriginMs']
                  and before['atMs']<=value['observedAtMs']<=after['atMs'],'One ordered document clock required')
    return dict(startupMilestoneMetadataAccepted=True,requiredMilestonesAvailable=metrics.pop('requiredMilestonesAvailable'),
        metrics=metrics,timeOriginMs=value['timeOriginMs'],observedAtMs=value['observedAtMs'],
        milestoneDefinition='document mount invocation and startup overlay handoff; neither first-interactive nor physical presentation',
        targetInstrumentationInjected=True,metadataJavaScriptEvaluation=True,observerOverheadRemoved=False,
        firstInteractiveTimeProven=False,nativeFramesAccepted=False,physicalDisplayPresentationProven=False,
        applicationOnlyLatencyAccepted=False,defaultPerformanceSampleAccepted=False,fullRF312Accepted=False)

def terminal(text):
    lines=text.splitlines()
    required=['INSTRUMENTATION_RESULT: startupMilestoneProbeCompleted=true','INSTRUMENTATION_RESULT: committedFrames=0','INSTRUMENTATION_CODE: -1']
    frame.require(all(lines.count(row)==1 for row in required) and not any(part in text for part in
        ['INSTRUMENTATION_FAILED','INSTRUMENTATION_ABORTED','frameProbePassed=','startupMilestoneProbeCompleted=false','failedStep=']),
        'Exact separate milestone terminal required')

def refusal_contract(body,marker,token):
    value=frame.decode(body)
    frame.fields(value,{'schemaVersion','runId','scope','step','errorClass','reason','lastDocument','frames',
        'callbacksDetached','nativeFramesAccepted','performanceSampleAccepted','nativeStatus'})
    frame.require(type(marker) is bytes and re.fullmatch(b'[a-f0-9]{64}\n',marker) is not None
                  and hashlib.sha256(body).hexdigest()==marker.decode().strip(),'Original milestone refusal hash required')
    frame.require(type(value['schemaVersion']) is int and value['schemaVersion']==2
                  and value['scope']=='native-login-startup-milestone-refusal' and value['runId']==token
                  and re.fullmatch('[a-f0-9]{32}',token) is not None,'Exact milestone refusal identity required')
    frame.require(type(value['frames']) is list and value['frames']==[]
                  and value['nativeFramesAccepted'] is False and value['performanceSampleAccepted'] is False
                  and type(value['callbacksDetached']) is bool,'Refusal cannot claim a frame or cleanup success')
    statuses={'not-observed','no-activity','activity-identity','no-webview','view-detached','view-hidden','window-unfocused',
        'software-rendering','unmeasured-view','readonly-script-queued','document-ready','document-not-ready'}
    frame.require(type(value['nativeStatus']) is str and value['nativeStatus'] in statuses,'Exact milestone progress code required')
    for name,pattern,limit in [('step',r'[a-z]+(?:-[a-z]+)*',96),('reason',r'[a-z]+(?:-[a-z]+)*',96),
                              ('errorClass',r'[A-Za-z_$][A-Za-z0-9_.$]*',128)]:
        part=value[name];frame.require(type(part) is str and len(part)<=limit and re.fullmatch(pattern,part) is not None,
                                      'Bounded fixed-code refusal required')
    doc=value['lastDocument']
    if doc is not None:
        frame.fields(doc,frame.DOCUMENT_FIELDS)
        frame.require(type(doc['schemaVersion']) is int and doc['schemaVersion']==2,'Exact document refusal schema required')
        for name in ['mainFrame','localOrigin','loginRoute','enabled','visible','hitPassword','hitSubmit','documentFocused','documentVisible']:
            frame.require(type(doc[name]) is bool,'Diagnostic document flags must be explicit booleans')
        for name in ['childFrames','passwordInputs','submitButtons']:frame.integer(doc[name],0,100000)
        for name in ['timeOriginMs','atMs']:number(doc[name])
    return value
