"""Ordered metadata control for the explicit non-default native collector.

No UI inputs, app invokes, payload extraction, or default acceptance flags.
Physical UI/APK/process/restoration evidence must be audited separately.
"""
import hashlib
import json
from pathlib import Path
import re
import time

import ipc_receipt


def require(value,message):
    if not value:raise ValueError(message)


def decode(raw):
    def pairs(items):
        result={}
        for key,value in items:
            require(key not in result,'Duplicate metadata field')
            result[key]=value
        return result
    def bad(_):raise ValueError('Nonfinite metadata value')
    return json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,parse_constant=bad)


def ready(raw,*,run_id,target_apk_sha256,collector_apk_sha256):
    require(re.fullmatch('[a-f0-9]{32}',run_id) is not None,'One explicit run identity required')
    require(all(re.fullmatch('[a-f0-9]{64}',v) is not None for v in [target_apk_sha256,collector_apk_sha256]),'Exact APK digests required')
    value=decode(raw)
    require(isinstance(value,dict) and set(value)=={'schemaVersion','runId','process','targetApkSha256','collectorApkSha256','phaseCount'},'Exact native ready fields required')
    require(type(value['schemaVersion']) is int and value['schemaVersion']==1 and value['runId']==run_id
            and value['targetApkSha256']==target_apk_sha256 and value['collectorApkSha256']==collector_apk_sha256
            and type(value['phaseCount']) is int and value['phaseCount']==12,'Native run and both package identities must match')
    process=value['process']
    require(isinstance(process,dict) and set(process)==ipc_receipt.PROCESS_FIELDS
            and all(type(v) is int and v>0 for v in process.values()) and process['uid']>=10000,'Native PID, app UID and birth required')
    return value


def push_progress(argv,stdout,stderr):
    require(len(argv)==3 and argv[0]=='push','One declared metadata file push required')
    path=Path(argv[1]);require(path.is_file(),'Declared metadata source file required')
    expected=(re.escape(str(path)).encode()+rb': 1 file pushed, 0 skipped\. [0-9]+(?:\.[0-9]+)? [KMGT]?B/s \('
              +str(path.stat().st_size).encode()+rb' bytes in [0-9]+(?:\.[0-9]+)?s\)\r?\n')
    require((stdout==stderr==b'') or (stdout==b'' and re.fullmatch(expected,stderr) is not None)
            or (stderr==b'' and re.fullmatch(expected,stdout) is not None),
            'Only the exact successful one-file SDK push receipt may appear')


class Collector:
    def __init__(self,run_sdk,alive,output,handshake,*,run_id,target_apk_sha256,collector_apk_sha256,
                 clock=time.monotonic,sleep=time.sleep):
        self.ready=ready(handshake,run_id=run_id,target_apk_sha256=target_apk_sha256,collector_apk_sha256=collector_apk_sha256)
        self.run_sdk=run_sdk;self.alive=alive;self.clock=clock;self.sleep=sleep
        self.run_id=run_id;self.process=self.ready['process'];self.records=[];self.attempted=0
        self.root='/sdcard/Android/data/com.solosoul.app/files/rf312-ipc-'+run_id
        self.output=Path(output).absolute()
        require(not self.output.exists() and not self.output.is_symlink(),'Fresh metadata output required')
        require(self.output.parent.is_dir(),'Existing metadata parent required')
        require(not any(p.is_symlink() or getattr(p.lstat(),'st_file_attributes',0)&0x400 for p in self.output.parents),'Ordinary metadata parents required')
        self.output.mkdir(exist_ok=False)
        self.started=self.clock();self.deadline=self.started+240
        self.proof=[]

    def remaining(self,deadline):
        value=min(deadline,self.deadline)-self.clock()
        require(0<value<=45,'Original metadata/physical phase deadline expired')
        require(self.alive(),'Native collector process ended')
        return value

    def command(self,argv,deadline,*,missing=False):
        result=self.run_sdk(argv,timeout=min(5,self.remaining(deadline)))
        self.remaining(deadline)
        require(isinstance(result,tuple) and len(result)==3,'SDK result tuple required')
        code,stdout,stderr=result
        require(type(code) is int and isinstance(stdout,bytes) and isinstance(stderr,bytes),'Exact SDK status types required')
        if missing and code==1 and stdout==b'' and stderr==b'':return None
        if argv[:1]==['push'] and code==0:
            push_progress(argv,stdout,stderr);return b''
        require(code==0 and stderr==b'','SDK metadata operation failed')
        return stdout

    def capture(self,stage,phase_deadline):
        require(self.attempted==len(self.records),'Failed metadata request cannot be retried or replaced')
        require(self.attempted<12 and stage==ipc_receipt.PHASES[self.attempted],'Exactly ordered original phase names required')
        self.remaining(phase_deadline);self.attempted+=1;index=self.attempted
        name=f'request-{index:03d}.json';request=self.output/name
        raw=(json.dumps(dict(schemaVersion=1,runId=self.run_id,sequence=index,stage=stage),separators=(',',':'))+'\n').encode()
        with request.open('xb') as stream:stream.write(raw)
        marker=request.with_name(name+'.sha256')
        with marker.open('xb') as stream:stream.write((hashlib.sha256(raw).hexdigest()+'\n').encode())
        self.command(['push',str(request),self.root+'/'+name],phase_deadline)
        self.command(['push',str(marker),self.root+'/'+name+'.sha256.part'],phase_deadline)
        self.command(['shell','mv',self.root+'/'+name+'.sha256.part',self.root+'/'+name+'.sha256'],phase_deadline)
        response=f'phase-{index:03d}.json'
        while True:
            present=self.command(['shell','test','-f',self.root+'/'+response+'.sha256'],phase_deadline,missing=True)
            if present is None:
                self.sleep(min(.025,self.remaining(phase_deadline)));continue
            digest=self.command(['exec-out','cat',self.root+'/'+response+'.sha256'],phase_deadline)
            if digest==b'':
                self.sleep(min(.025,self.remaining(phase_deadline)));continue
            require(re.fullmatch(b'[a-f0-9]{64}\n?',digest) is not None,'Complete native response digest required')
            payload=self.command(['exec-out','cat',self.root+'/'+response],phase_deadline)
            require(0<len(payload)<=4000000 and hashlib.sha256(payload).hexdigest()==digest.decode().strip(),'Native metadata payload binding failed')
            record=decode(payload);self.validate_record(record,stage)
            self.remaining(phase_deadline)
            with (self.output/response).open('xb') as stream:stream.write(payload)
            self.records.append(record)
            self.proof.append(dict(stage=stage,sequence=index,requestSha256=hashlib.sha256(raw).hexdigest(),responseSha256=hashlib.sha256(payload).hexdigest()))
            return record

    def validate_record(self,record,stage):
        require(isinstance(record,dict) and set(record)==ipc_receipt.RECORD_FIELDS,'Only bound native record fields allowed')
        require(record['stage']==stage and record['process']==self.process
                and all(type(v) is int for v in record['process'].values()),'Original stage and native process required')
        start,end=record['nativeReadStartedNs'],record['nativeReadFinishedNs']
        require(type(start) is int and type(end) is int and 0<start<=end and end-start<=3000000000,'Original three-second native read budget required')
        current=ipc_receipt.snapshot(record['document'],self.run_id)
        if self.records:
            prior=self.records[-1];old=prior['document']['observer'];count=old['observedCount']
            require(start>=prior['nativeReadFinishedNs'],'Native read order changed')
            require(all(current[k]==old[k] for k in ['runId','timeOriginMs','installedAtMs']),'Original document identity changed')
            require(current['atMs']>=old['atMs'] and current['observedCount']>=count
                    and current['commands'][:count]==old['commands'],'Cumulative event prefix changed')
            require(all(event['atMs']>=old['atMs'] for event in current['commands'][count:]),'Backdated insertion rejected')

    def finish(self):
        require(self.attempted==12 and len(self.records)==12,'All twelve native checkpoints required')
        result=ipc_receipt.validate(self.records,run_id=self.run_id,expected_process=self.process)
        result.update(orderedHostRequestsBound=True,requestResponseDigests=list(self.proof))
        require(result['actualIpcAccepted'] is False and result['fullRF312Accepted'] is False,'Metadata collection cannot accept physical UI or restoration')
        return result
