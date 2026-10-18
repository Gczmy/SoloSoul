"""Strict metadata contract; passing it is not proof that a native run happened."""
import json
import math
import re

STAGE_NS = 45_000_000_000
READY_FIELDS = {'schemaVersion','scope','runId','process','targetApkSha256','collectorApkSha256','startedNs','deadlineNs'}
PROCESS_FIELDS = {'processId','uid','startTicks'}
DOCUMENT_FIELDS = {'schemaVersion','mainFrame','childFrames','localOrigin','loginRoute','passwordInputs','submitButtons',
                   'enabled','visible','hitPassword','hitSubmit','documentFocused','documentVisible','timeOriginMs','atMs'}
FRAME_FIELDS = {'schemaVersion','scope','runId','process','targetApkSha256','api','startedNs','deadlineNs','completedNs',
                'fenceRequest','fencePostedNs','fenceCompleteNs','hardwareAccelerated','nativeWindowFocused','width','height',
                'explicitDrawRequestsDispatched','nativeClock','forcedInvalidation','inputDispatched','businessIpcDispatched',
                'documentBefore','documentAfter','frames','callbacksDetached','physicalDisplayPresentationProven','applicationOnlyLatencyAccepted'}
COMMIT_FIELDS = {'drawOrdinal','drawStartedNs','committedNs','callbackSource'}
CLOCK = 'System.nanoTime; same target process; not converted to document clock'
CALLBACK = 'ViewTreeObserver.registerFrameCommitCallback'

def require(value, message):
    if not value:
        raise ValueError(message)

def fields(value, expected):
    require(type(value) is dict and set(value) == expected, 'Exact frame metadata fields required')

def integer(value, minimum=1, maximum=(1 << 63) - 1):
    require(type(value) is int and minimum <= value <= maximum, 'Bounded integer required')
    return value

def process(value, expected):
    for item in [value, expected]:
        fields(item, PROCESS_FIELDS)
        for key, part in item.items(): integer(part, 10000 if key == 'uid' else 1)
    require(value == expected, 'Stable process UID, PID and birth required')

def document(value):
    fields(value, DOCUMENT_FIELDS)
    for key, expected in [('schemaVersion',2),('childFrames',0),('passwordInputs',1),('submitButtons',1)]:
        require(integer(value[key],0) == expected, 'Exclusive usable password form required')
    for key in ['mainFrame','localOrigin','loginRoute','enabled','visible','hitPassword','hitSubmit','documentVisible']:
        require(value[key] is True, 'Visible, focused and unobscured login document required')
    require(type(value['documentFocused']) is bool, 'Explicit diagnostic document input-focus flag required')
    for key in ['timeOriginMs','atMs']:
        part=value[key]
        require(type(part) in [int,float] and math.isfinite(part) and part >= 0, 'Finite document clock required')
    require(value['timeOriginMs'] > 0, 'Positive document clock origin required')

def decode(raw):
    require(type(raw) is bytes and 0 < len(raw) <= 32768, 'Bounded original UTF-8 receipt required')
    def pairs(items):
        result={}
        for key,value in items:
            require(key not in result, 'Duplicate JSON field rejected')
            result[key]=value
        return result
    def constant(value):
        raise ValueError('Nonfinite JSON constant rejected')
    return json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,parse_constant=constant)

def validate(ready, record, *, run_id, expected_process, target_apk_sha256, collector_apk_sha256, expected_api):
    require(type(run_id) is str and re.fullmatch('[a-f0-9]{32}',run_id) is not None, 'Explicit measurement identity required')
    for digest in [target_apk_sha256,collector_apk_sha256]:
        require(type(digest) is str and re.fullmatch('[a-f0-9]{64}',digest) is not None, 'Exact APK digest required')
    integer(expected_api,29,100)
    fields(ready,READY_FIELDS);fields(record,FRAME_FIELDS)
    require(type(ready['schemaVersion']) is int and ready['schemaVersion']==1, 'Ready schema rejected')
    require(type(record['schemaVersion']) is int and record['schemaVersion']==2, 'Frame schema rejected')
    require(ready['scope']=='native-login-frame-commit-probe', 'Explicit frame actor required')
    require(record['scope']=='native-login-frame-commits-after-visual-fence', 'Visual fence frame scope required')
    require(ready['runId']==record['runId']==run_id, 'Frame run changed')
    require(ready['targetApkSha256']==record['targetApkSha256']==target_apk_sha256
            and ready['collectorApkSha256']==collector_apk_sha256, 'Frame APK identity changed')
    process(ready['process'],expected_process);process(record['process'],expected_process)
    require(integer(record['api'],29,100)==expected_api, 'Actual supported API required')
    for key in ['startedNs','deadlineNs']:
        require(integer(ready[key])==integer(record[key]), 'Original native deadline changed')
    require(record['deadlineNs']-record['startedNs']==STAGE_NS, 'Original 45-second phase limit required')
    for key in ['completedNs','fenceRequest','fencePostedNs','fenceCompleteNs']: integer(record[key])
    require(record['fenceRequest']==record['fencePostedNs'], 'Exact visual state callback request required')
    require(record['startedNs'] <= record['fencePostedNs'] <= record['fenceCompleteNs']
            <= record['completedNs'] < record['deadlineNs'], 'Ordered original native clock required')
    require(record['nativeClock']==CLOCK, 'No native/document clock conversion accepted')
    for key in ['hardwareAccelerated','nativeWindowFocused','callbacksDetached']:
        require(record[key] is True, 'Hardware commits and callback cleanup required')
    for key in ['explicitDrawRequestsDispatched','forcedInvalidation','inputDispatched','businessIpcDispatched',
                'physicalDisplayPresentationProven','applicationOnlyLatencyAccepted']:
        require(record[key] is False, 'Forced actions or unsupported evidence claims rejected')
    integer(record['width'],1,16384);integer(record['height'],1,16384)
    document(record['documentBefore']);document(record['documentAfter'])
    require(record['documentBefore']['timeOriginMs']==record['documentAfter']['timeOriginMs']
            and record['documentAfter']['atMs']>=record['documentBefore']['atMs'], 'Document identity or clock changed')
    commits=record['frames'];require(type(commits) is list and len(commits)==2, 'Two real commit callbacks required')
    previous=None
    for ordinal,commit in enumerate(commits,1):
        fields(commit,COMMIT_FIELDS)
        require(integer(commit['drawOrdinal'])==ordinal, 'One separate registered draw per commit required')
        draw=integer(commit['drawStartedNs']);end=integer(commit['committedNs'])
        require(commit['callbackSource']==CALLBACK, 'Animation callbacks do not prove a native frame commit')
        require(record['fenceCompleteNs'] <= draw <= end <= record['completedNs'], 'Commit outside the fenced native window')
        if previous is not None:
            require(draw > previous['committedNs'] and end > previous['committedNs'], 'Second callback must follow a separate post-commit draw')
        previous=commit
    return dict(frameCommitContractAccepted=True,committedFrames=2,
                observedCommitSeparationNs=commits[1]['committedNs']-commits[0]['committedNs'],
                nativeRuntimeAccepted=False,physicalDisplayPresentationProven=False,
                applicationOnlyLatencyAccepted=False,defaultPerformanceSampleAccepted=False,fullRF312Accepted=False)
