"""Offline contract prototype for a future twelve-stage native IPC collector.

Validation of a synthetic receipt is not proof of native observation. The live
driver must bind process/APK identity, physical UI evidence and restoration.
"""
import hashlib
import json
import math
import re

PHASES = ['startup','password-unlock','first-ocr-user-flow','workspace','search-needle',
          'attachments-open','image-preview','text-preview','pdf-text-native','pdf-scanned-native',
          'application-lock','password-reunlock']
SNAPSHOT_FIELDS = {'schemaVersion','scope','runId','valid','invalidReasons','installedAtMs',
                   'timeOriginMs','atMs','maxEvents','observedCount','commands'}
DOCUMENT_FIELDS = {'present','platform','mainFrame','frames','invokeReadonly','observer'}
RECORD_FIELDS = {'stage','process','nativeReadStartedNs','nativeReadFinishedNs','document'}
PROCESS_FIELDS = {'processId','uid','startTicks'}
TRANSPORTS = {'android-java-bridge','android-channel-fetch'}


def require(value, message):
    if not value:raise ValueError(message)


def number(value):
    return type(value) in (int,float) and math.isfinite(value)


def object_fields(value, fields, message):
    require(isinstance(value,dict) and set(value)==fields,message)


def snapshot(document, run_id):
    object_fields(document,DOCUMENT_FIELDS,'Only allowed document metadata may be recorded')
    require(document['present'] is True and document['platform']=='android'
            and document['mainFrame'] is True and type(document['frames']) is int
            and document['frames']==0 and document['invokeReadonly'] is True,
            'Exclusive Android document with readonly invoke required')
    value=document['observer']
    object_fields(value,SNAPSHOT_FIELDS,'Only allowed observer metadata may be recorded')
    require(type(value['schemaVersion']) is int and value['schemaVersion']==1
            and value['scope']=='android-native-ipc-transport-observer','Original transport observer scope required')
    require(value['runId']==run_id and value['valid'] is True and value['invalidReasons']==[],
            'One valid measurement identity required')
    require(type(value['maxEvents']) is int and value['maxEvents']==16384,'Original event cap required')
    require(all(number(value[key]) for key in ['installedAtMs','timeOriginMs','atMs'])
            and value['timeOriginMs']>0 and 0<=value['installedAtMs']<=value['atMs'],
            'Finite original document clocks required')
    events=value['commands']
    require(isinstance(events,list) and type(value['observedCount']) is int
            and 0<value['observedCount']==len(events)<=16384,'Complete positive event count required')
    previous=value['installedAtMs']
    for event in events:
        object_fields(event,{'command','transport','atMs'},'Payloads or extra event fields forbidden')
        require(isinstance(event['command'],str) and re.fullmatch(r'[A-Za-z0-9_.:|/-]{1,128}',event['command']) is not None,
                'Safe command name required')
        require(event['transport'] in TRANSPORTS,'Original actual transport category required')
        require(number(event['atMs']) and previous<=event['atMs']<=value['atMs'],'Ordered bounded event clock required')
        previous=event['atMs']
    return value


def validate(records, *, run_id, expected_process):
    require(isinstance(run_id,str) and re.fullmatch('[a-f0-9]{32}',run_id) is not None,'Fixed 32-hex identity required')
    object_fields(expected_process,PROCESS_FIELDS,'Complete expected process identity required')
    require(all(type(v) is int and v>0 for v in expected_process.values())
            and expected_process['uid']>=10000,'Application UID and process birth required')
    require(isinstance(records,list) and len(records)==12,'All twelve native metadata checkpoints required')
    prior=None;prior_native_end=None;phases=[]
    for index,record in enumerate(records):
        object_fields(record,RECORD_FIELDS,'Only bound native collector fields allowed')
        require(record['stage']==PHASES[index],'Original ordered phase names required')
        object_fields(record['process'],PROCESS_FIELDS,'No extra process metadata allowed')
        require(all(type(value) is int for value in record['process'].values())
                and record['process']==expected_process,'Observed process must preserve PID/UID/birth')
        start,end=record['nativeReadStartedNs'],record['nativeReadFinishedNs']
        require(type(start) is int and type(end) is int and 0<start<=end
                and end-start<=3_000_000_000,'Bounded three-second native metadata read required')
        require(prior_native_end is None or start>=prior_native_end,'Native collection order changed')
        current=snapshot(record['document'],run_id)
        previous_count=0
        if prior is not None:
            require(all(current[key]==prior[key] for key in ['runId','timeOriginMs','installedAtMs']),
                    'Document replacement or rebound observer rejected')
            require(current['atMs']>=prior['atMs'],'Snapshot clock moved backward')
            previous_count=prior['observedCount']
            require(current['observedCount']>=previous_count
                    and current['commands'][:previous_count]==prior['commands'],
                    'Previously observed events were removed, changed or reordered')
            require(all(event['atMs']>=prior['atMs'] for event in current['commands'][previous_count:]),
                    'Late insertion before previous snapshot boundary rejected')
        delta=current['commands'][previous_count:]
        phases.append(dict(name=record['stage'],sendAttempts=len(delta),
            byTransport={name:sum(event['transport']==name for event in delta) for name in sorted(TRANSPORTS)},
            nativeMetadataReadNs=end-start))
        prior=current;prior_native_end=end
    canonical=json.dumps(prior['commands'],sort_keys=True,separators=(',',':'),allow_nan=False).encode()
    return dict(metadataContractAccepted=True,phaseCount=12,sendAttempts=prior['observedCount'],phases=phases,
                commandsSha256=hashlib.sha256(canonical).hexdigest(),metric='actual main-document transport send attempts including retries/channels',
                logicalInvokeOrCompletionCount=False,liveProcessAndApkIdentityAudited=False,
                physicalUiAndRestorationAudited=False,actualIpcAccepted=False,fullRF312Accepted=False)
