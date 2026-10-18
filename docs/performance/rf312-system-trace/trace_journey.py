"""Whole-trace wrapper around the unchanged supported-state twelve-phase journey."""
from pathlib import Path
import hashlib
import importlib.util
import json
import subprocess
import time

import trace_binding
import trace_session
from timeout_streams import preserve_timeout

PUBLIC=Path(__file__).resolve().parent.parent
APP='com.solosoul.app'
sha=lambda raw:hashlib.sha256(raw).hexdigest()

def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value

class TraceCalls:
    def __init__(self,adb,folder):self.adb,self.folder,self.rows=adb,folder,[]
    def __call__(self,label,args,*,timeout):
        self.folder.mkdir(exist_ok=True)
        prefix=self.folder/f'{len(self.rows)+1:04d}-{label}'
        try:result=self.adb(*args,timeout=timeout,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        except subprocess.TimeoutExpired as error:
            partial=preserve_timeout(prefix,error)
            self.rows.append(dict(label=label,args=args,timeoutSeconds=timeout,exitCode=None,partial=partial))
            self._save();raise
        except subprocess.CalledProcessError as error:
            self._record(prefix,label,args,timeout,error.returncode,error.stdout,error.stderr);raise
        self._record(prefix,label,args,timeout,result.returncode,result.stdout,result.stderr)
        return result
    def _record(self,prefix,label,args,timeout,code,stdout,stderr):
        digests={}
        for stream,data in [('stdout',stdout),('stderr',stderr)]:
            if not isinstance(data,bytes):raise ValueError('Actual raw SDK output required')
            with prefix.with_suffix('.'+stream).open('xb') as f:f.write(data)
            digests[stream+'Sha256']=sha(data)
        self.rows.append(dict(label=label,args=args,timeoutSeconds=timeout,exitCode=code,**digests));self._save()
    def _save(self):
        (self.folder/'commands.json').write_text(json.dumps(self.rows,indent=2)+'\n',encoding='utf-8')

def require_main_cmdline(raw):
    if (not isinstance(raw,bytes) or not len(APP.encode()) < len(raw) <= 4096
            or not raw.startswith(APP.encode())
            or raw[len(APP.encode()):] != b'\0' * (len(raw)-len(APP.encode()))):
        raise ValueError('Target command-line identity differs')
    return dict(argumentBlockBytes=len(raw),trailingNulBytes=len(raw)-len(APP.encode()))

def observe(adb,read,output,count,uid,public_image_name,inputs,*,trace_inputs,**options):
    trace_binding.verify(trace_inputs)
    base=module('original_supported_journey_for_system_trace',PUBLIC/'rf312_default_journey.py')
    memory=module('original_identity_reader_for_system_trace',Path(inputs['host']['workspaceRoot'])/'tauri/scripts/android-process-memory.py')
    if sha((Path(inputs['host']['workspaceRoot'])/'tauri/scripts/android-process-memory.py').read_bytes())!=inputs['canonicalMemoryReaderSha256']:
        raise ValueError('Canonical birth identity reader differs')
    folder=output/'system-trace';folder.mkdir()
    calls=TraceCalls(adb,folder/'sdk')
    session=trace_session.Session(calls,trace_inputs['inputCases'][str(count)]['root'],folder)
    # Session sees an empty directory before any recording files are created.
    identities=[];phases=[];readiness=[];original_save=base.save;identity=None;capture=None
    def kernel(pid):
        expected=dict(pid=pid,uid=uid)
        def raw(name):
            value=calls('identity-'+name,['exec-out','cat',f'/proc/{pid}/{name}'],timeout=5)
            if value.stderr:raise ValueError('Kernel identity stderr')
            return value.stdout
        stat=raw('stat');status=raw('status');cmdline=raw('cmdline');after=raw('stat')
        require_main_cmdline(cmdline)
        before_id=memory.kernel_identity(stat.decode(),status.decode(),expected)
        after_id=memory.kernel_identity(after.decode(),status.decode(),expected)
        if before_id!=after_id:raise ValueError('Target birth changed during identity read')
        return before_id
    def capture_identity(label,pid):
        nonlocal identity
        current=kernel(pid)
        if identity is None:identity=current
        if current!=identity:raise ValueError('Original target PID/UID/birth changed')
        identities.append(dict(label=label,hostObservedAtNs=time.monotonic_ns(),**current))
        (folder/'identity-progress.json').write_text(json.dumps(identities,indent=2)+'\n',encoding='utf-8')
    def saved(path,value):
        nonlocal capture
        if path.name=='observed-process-start.json':
            if identities:raise ValueError('Original process-start checkpoint repeated')
            capture_identity('original-process-start',value['pid'])
        original_save(path,value)
        if path.name=='measured-workload-window.json':
            if capture is not None:raise ValueError('Workload closed twice')
            capture=session.finish()
    OriginalJourney=base.Journey
    class Journey(OriginalJourney):
        def finish(self,name,started,row,**extra):
            if identity is None:raise ValueError('Original birth checkpoint required')
            capture_identity(name,identity['pid'])
            if name=='startup':
                password=self.field(True,started+45)
                unlock=self.unique(lambda:self.action_candidates(r'Unlock'),started+45)
                self.remaining(started+45)
                readiness.append(dict(scope='host-observed-functional-login-readiness-upper-bound',
                    passwordBounds=password.get('bounds'),unlockBounds=unlock.get('bounds'),
                    hostObservedAtNs=time.monotonic_ns(),originalPhaseDeadline=started+45,
                    nativeTargetHitChecksAndStableState=True,inputDispatched=False,
                    exactFirstInteractiveTimeProven=False,applicationOnlyLatencyAccepted=False))
                (folder/'login-readiness.json').write_text(json.dumps(readiness,indent=2)+'\n',encoding='utf-8')
            result=super().finish(name,started,row,**extra)
            phases.append(dict(name=name,hostStartedAt=started,hostFinishedAtNs=time.monotonic_ns()))
            (folder/'phase-progress.json').write_text(json.dumps(phases,indent=2)+'\n',encoding='utf-8')
            return result
    base.Journey=Journey;base.save=saved
    tick_result=calls('kernel-clock-ticks',['shell','getconf','CLK_TCK'],timeout=5)
    import re
    if tick_result.stderr or not re.fullmatch(rb'[1-9][0-9]*\r?\n',tick_result.stdout):
        raise ValueError('Native kernel clock tick rate unavailable')
    tick_rate=int(tick_result.stdout)
    if not 1<=tick_rate<=10000:raise ValueError('Native kernel clock tick rate out of bound')
    session.start()
    try:
        evidence=base.observe(adb,read,output,count,uid,public_image_name,inputs,**options)
    finally:
        if not session.cleanup_recorded:
            capture=session.finish()
    trace_binding.verify(trace_inputs)
    if capture is None or [p['name'] for p in phases]!=list(base.PHASES) or len(identities)!=13 or len(readiness)!=1:
        raise ValueError('Complete twelve-phase identity and readiness proof required')
    brackets=capture['hostMonotonicBrackets']
    if not all(brackets['activeAfterNs']/1e9<=p['hostStartedAt']<p['hostFinishedAtNs']/1e9<=brackets['stopBeforeNs']/1e9 for p in phases):
        raise ValueError('Trace session does not bracket the complete workload')
    proof=dict(passed=True,scope='host-bracketed-twelve-phase-system-trace-transport',objectCount=count,
        phases=phases,stableMainProcess=identity,identityCheckpoints=len(identities),loginReadiness=readiness[0],
        kernelClockTicksPerSecond=tick_rate,
        originalUiBudgetsAndGatesPreserved=True,hostBracketsVerified=True,
        traceSha256=capture['traceSha256'],traceContentAttributionAccepted=False,
        wholeJourneyMemoryPeakAccepted=False,nativeFramesAccepted=False,
        firstInteractiveTimeProven=False,fullRF312Accepted=False)
    original_save(folder/'whole-workload-binding.json',proof)
    evidence['systemTraceHostBracketsVerified']=True
    return evidence
