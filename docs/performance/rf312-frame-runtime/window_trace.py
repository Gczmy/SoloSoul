"""Separate passive window metadata; never substitutes for a native frame commit."""
import hashlib,re
import frame_receipt as frame

LIMIT=64
COUNT_LIMIT=1_000_000
CLOCK='System.nanoTime; delivery and UI callbacks only; FrameMetrics clocks unconverted'
FIELDS={'schemaVersion','scope','runId','process','targetApkSha256','collectorApkSha256','api','startedNs','deadlineNs',
        'detachStartedNs','captureStoppedNs','cleanupFinishedNs','lifecycleHookNs','windowHookNs','fenceCompleteNs',
        'targetActivityObserved','lifecycleHookInstalled','windowHookInstalled','hookFrom','fault',
        'callbacksDetached','metricsThreadTerminated','cleanupBudgetMet','webViewTreeCompared','webViewTreeMatchesDecorTree',
        'preDrawCount','drawCount','preDrawAfterFence','drawAfterFence','metricsCallbackCount','metricsCallbacksAfterFence',
        'reportedMetricDrops','lateMetricsCallbacks','lateUiCallbacks','firstPreDrawNs','lastPreDrawNs','firstDrawNs','lastDrawNs',
        'storedMetricLimit','metricPrefixTruncated','metrics','nativeClock','explicitDrawRequestsDispatched','forcedInvalidation',
        'inputDispatched','businessIpcDispatched','explicitDecorCreationRequested','nativeFramesAccepted','applicationOnlyLatencyAccepted',
        'defaultPerformanceSampleAccepted','physicalDisplayPresentationProven','fullRF312Accepted'}
METRIC_FIELDS={'ordinal','deliveredNs','intendedVsyncNs','vsyncNs','totalDurationNs','drawDurationNs','swapDurationNs',
               'firstDrawFlag','droppedSincePrevious'}
FALSE_FLAGS={'explicitDrawRequestsDispatched','forcedInvalidation','inputDispatched','businessIpcDispatched',
             'explicitDecorCreationRequested','nativeFramesAccepted','applicationOnlyLatencyAccepted',
             'defaultPerformanceSampleAccepted','physicalDisplayPresentationProven','fullRF312Accepted'}
BOOL_FLAGS={'targetActivityObserved','lifecycleHookInstalled','windowHookInstalled','callbacksDetached','metricsThreadTerminated',
            'cleanupBudgetMet','webViewTreeCompared','webViewTreeMatchesDecorTree','metricPrefixTruncated'}
COUNTERS={'preDrawCount','drawCount','preDrawAfterFence','drawAfterFence','metricsCallbackCount','metricsCallbacksAfterFence',
          'reportedMetricDrops','lateMetricsCallbacks','lateUiCallbacks'}
FAULTS={'none','setup-rejected','hook-rejected','ui-observation-rejected','metrics-observation-rejected','listener-detach-rejected'}

def validate(body,marker,ready,*,expected_api=31):
    frame.integer(expected_api,29,100)
    frame.fields(ready,frame.READY_FIELDS)
    value=frame.decode(body);frame.fields(value,FIELDS)
    frame.require(type(marker) is bytes and re.fullmatch(b'[a-f0-9]{64}\n',marker) is not None
                  and hashlib.sha256(body).hexdigest()==marker.decode().strip(),'Complete original window trace hash required')
    frame.require(type(value['schemaVersion']) is int and value['schemaVersion']==1
                  and value['scope']=='passive-window-frame-diagnostics','Exact passive trace schema required')
    frame.require(type(value['runId']) is str and value['runId']==ready['runId'] and re.fullmatch('[a-f0-9]{32}',value['runId']) is not None,
                  'Original trace run required')
    for name in ['targetApkSha256','collectorApkSha256']:
        frame.require(type(value[name]) is str and value[name]==ready[name] and re.fullmatch('[a-f0-9]{64}',value[name]) is not None,'Original trace APKs required')
    frame.process(value['process'],ready['process'])
    frame.require(frame.integer(value['api'],29,100)==expected_api,'Actual trace API required')
    for name in ['startedNs','deadlineNs']:
        frame.require(frame.integer(value[name])==frame.integer(ready[name]),'Original native trace deadline required')
    begin,end=value['startedNs'],value['deadlineNs']
    frame.require(end-begin==frame.STAGE_NS and value['nativeClock']==CLOCK,'Unconverted original native window required')
    for name in BOOL_FLAGS:frame.require(type(value[name]) is bool,'Explicit trace boolean required')
    for name in FALSE_FLAGS:frame.require(value[name] is False,'Passive trace cannot claim input or native frame success')
    frame.require(type(value['hookFrom']) is str and type(value['fault']) is str
                  and value['hookFrom'] in ['none','post-created','started','resumed'] and value['fault'] in FAULTS,'Fixed hook and fault codes required')
    for name in COUNTERS:frame.integer(value[name],0,COUNT_LIMIT)
    for name in ['lifecycleHookNs','windowHookNs','fenceCompleteNs']:
        stamp=frame.integer(value[name],0)
        frame.require(stamp==0 or begin<=stamp<end,'Trace hook outside original native window')
    for name in ['detachStartedNs','captureStoppedNs','cleanupFinishedNs']:
        stamp=frame.integer(value[name],0);frame.require(stamp==0 or stamp>=begin,'Trace cleanup clock precedes capture')
    if value['cleanupBudgetMet']:
        frame.require(value['detachStartedNs']>0 and value['detachStartedNs']<=value['captureStoppedNs']<=value['cleanupFinishedNs']
                      <=value['detachStartedNs']+3_000_000_000,'Shared original cleanup budget required')
    frame.require(value['lifecycleHookInstalled']==(value['lifecycleHookNs']>0),'Lifecycle observation marker differs')
    frame.require(value['windowHookInstalled']==(value['windowHookNs']>0),'Window observation marker differs')
    if value['windowHookInstalled']:
        frame.require(value['targetActivityObserved'] and value['lifecycleHookInstalled'] and value['hookFrom']!='none'
                      and value['windowHookNs']>=value['lifecycleHookNs'],'Observed existing target window required')
    else:frame.require(value['hookFrom']=='none','No invented window hook origin accepted')
    frame.require(value['webViewTreeCompared'] or not value['webViewTreeMatchesDecorTree'],'Unknown tree comparison cannot be true')
    fence=value['fenceCompleteNs']
    for count,after,first,last in [('preDrawCount','preDrawAfterFence','firstPreDrawNs','lastPreDrawNs'),
                                   ('drawCount','drawAfterFence','firstDrawNs','lastDrawNs')]:
        frame.require(value[after]<=value[count] and (fence>0 or value[after]==0),'Bounded post-fence UI counts required')
        low,high=frame.integer(value[first],0),frame.integer(value[last],0)
        frame.require((value[count]==0 and low==high==0)
                      or (value[count]>0 and value['windowHookInstalled'] and begin<=low<=high<end),'Exact observed UI time span required')
        if value[after]>0:frame.require(high>=fence,'Post-fence UI clock precedes fence')
        if value['captureStoppedNs']>0:frame.require(high<=value['captureStoppedNs'],'UI event after capture stopped')
    count=value['metricsCallbackCount'];records=value['metrics']
    frame.require(type(value['storedMetricLimit']) is int and value['storedMetricLimit']==LIMIT
                  and type(records) is list and len(records)==min(count,LIMIT)
                  and value['metricPrefixTruncated']==(count>LIMIT),'Complete bounded callback prefix required')
    frame.require(value['metricsCallbacksAfterFence']<=count and (fence>0 or value['metricsCallbacksAfterFence']==0),
                  'Bounded post-fence delivery counts required')
    previous=0;prefix_after=0;prefix_drops=0
    for ordinal,row in enumerate(records,1):
        frame.fields(row,METRIC_FIELDS)
        frame.require(frame.integer(row['ordinal'])==ordinal,'Contiguous native delivery prefix required')
        at=frame.integer(row['deliveredNs'])
        frame.require(value['windowHookInstalled'] and begin<=at<end and at>=previous,'Original ordered native delivery time required')
        previous=at;prefix_after+=int(fence>0 and at>=fence)
        if value['captureStoppedNs']>0:frame.require(at<=value['captureStoppedNs'],'Metric delivery after capture stopped')
        for name in ['intendedVsyncNs','vsyncNs','totalDurationNs','drawDurationNs','swapDurationNs']:frame.integer(row[name],-1)
        frame.integer(row['firstDrawFlag'],-1,1)
        prefix_drops+=frame.integer(row['droppedSincePrevious'],0,COUNT_LIMIT)
    if not value['metricPrefixTruncated']:
        frame.require(prefix_after==value['metricsCallbacksAfterFence'] and prefix_drops==value['reportedMetricDrops'],
                      'Exact complete callback counts required')
    else:
        frame.require(prefix_after<=value['metricsCallbacksAfterFence'] and prefix_drops<=value['reportedMetricDrops'],
                      'Prefix counts cannot exceed observed totals')
    complete=all(value[k] for k in ['targetActivityObserved','lifecycleHookInstalled','windowHookInstalled','callbacksDetached',
                                   'metricsThreadTerminated','cleanupBudgetMet']) and value['fault']=='none'
    return dict(passiveWindowTraceMetadataAccepted=True,observationHooksAndCleanupComplete=bool(complete),
                metricsCallbacks=count,metricsCallbacksAfterFence=value['metricsCallbacksAfterFence'],
                nativeReportedMetricDrops=value['reportedMetricDrops'],prefixTruncated=value['metricPrefixTruncated'],
                earlyPreDrawCallbacks=value['preDrawCount'],earlyDrawCallbacks=value['drawCount'],
                preDrawCallbacksAfterFence=value['preDrawAfterFence'],drawCallbacksAfterFence=value['drawAfterFence'],
                lateMetricsCallbacks=value['lateMetricsCallbacks'],lateUiCallbacks=value['lateUiCallbacks'],
                webViewTreeCompared=value['webViewTreeCompared'],webViewTreeMatchesDecorTree=value['webViewTreeMatchesDecorTree'],
                metricsCompleteCoverageClaimed=False,rawMetricClockComparedToFence=False,
                nativeFramesAccepted=False,physicalDisplayPresentationProven=False,applicationOnlyLatencyAccepted=False,
                defaultPerformanceSampleAccepted=False,fullRF312Accepted=False)
