"""Owned native frame handshake and terminal evidence, with no UI input."""
from contextlib import contextmanager
import hashlib,json,re,subprocess,time,uuid
import frame_receipt as receipt_reader

APP='com.solosoul.app';PKG='org.solosoul.rf312frames'
COMPONENT=PKG+'/'+PKG+'.FrameCollector'
UI='org.solosoul.rf312observer/org.solosoul.rf312observer.Observer'
sha=lambda raw:hashlib.sha256(raw).hexdigest()

def save(path,value):
    with path.open('x',encoding='utf-8',newline='\n') as stream:json.dump(value,stream,indent=2,allow_nan=False)

def ready_contract(raw,run,app,collector,uid):
    value=receipt_reader.decode(raw);receipt_reader.fields(value,receipt_reader.READY_FIELDS)
    if (type(value['schemaVersion']) is not int or value['schemaVersion']!=1
            or value['scope']!='native-login-frame-commit-probe' or value['runId']!=run
            or value['targetApkSha256']!=app or value['collectorApkSha256']!=collector):
        raise ValueError('Exact frame collector handshake required')
    receipt_reader.process(value['process'],value['process'])
    if value['process']['uid']!=uid:raise ValueError('Original application UID required')
    begin=receipt_reader.integer(value['startedNs']);end=receipt_reader.integer(value['deadlineNs'])
    if end-begin!=receipt_reader.STAGE_NS:raise ValueError('Original native deadline required')
    return value

def terminal(text):
    lines=text.splitlines()
    required=['INSTRUMENTATION_RESULT: frameProbePassed=true','INSTRUMENTATION_RESULT: committedFrames=2','INSTRUMENTATION_CODE: -1']
    if (any(lines.count(line)!=1 for line in required)
            or any(key in text for key in ['INSTRUMENTATION_FAILED','INSTRUMENTATION_ABORTED','frameProbePassed=false','failedStep='])):
        raise ValueError('Exact normal frame collector terminal outcome required')

def refusal_contract(body,marker,token):
    value=receipt_reader.decode(body)
    fields={'schemaVersion','runId','scope','step','errorClass','reason','lastDocument','frames',
            'callbacksDetached','nativeFramesAccepted','performanceSampleAccepted','nativeStatus'}
    receipt_reader.fields(value,fields)
    if (not re.fullmatch(b'[a-f0-9]{64}\n',marker) or sha(body)!=marker.decode().strip()
            or type(value['schemaVersion']) is not int or value['schemaVersion']!=2
            or value['runId']!=token or value['nativeFramesAccepted'] is not False
            or value['performanceSampleAccepted'] is not False or type(value['callbacksDetached']) is not bool
            or value['scope']!='native-login-frame-commit-refusal'):
        raise ValueError('Exact terminal frame refusal required')
    statuses={'not-observed','no-activity','activity-identity','no-webview','view-detached','view-hidden',
              'window-unfocused','software-rendering','unmeasured-view','readonly-script-queued','document-ready','document-not-ready',
              'visual-fence-pending','visual-fence-complete','native-draw-listener-armed','native-draw-observed','native-commit-observed','native-listeners-armed','native-pre-draw-commit-armed'}
    if type(value['nativeStatus']) is not str or value['nativeStatus'] not in statuses:
        raise ValueError('Exact bounded native startup status required')
    for key,pattern,limit in [('step',r'[a-z]+(?:-[a-z]+)*',96),('reason',r'[a-z]+(?:-[a-z]+)*',96),
                              ('errorClass',r'[A-Za-z_$][A-Za-z0-9_.$]*',128)]:
        text=value[key]
        if type(text) is not str or len(text)>limit or re.fullmatch(pattern,text) is None:
            raise ValueError('Bounded fixed-code refusal required')
    if type(value['frames']) is not list or len(value['frames'])>2:raise ValueError('Bounded failed frame prefix required')
    for frame in value['frames']:
        receipt_reader.fields(frame,receipt_reader.COMMIT_FIELDS)
        for key in ['drawOrdinal','drawStartedNs','committedNs']:receipt_reader.integer(frame[key])
        if frame['callbackSource']!=receipt_reader.CALLBACK:raise ValueError('Native commit prefix required')
    document=value['lastDocument']
    if document is not None:
        receipt_reader.fields(document,receipt_reader.DOCUMENT_FIELDS)
        if type(document['schemaVersion']) is not int or document['schemaVersion']!=2:
            raise ValueError('Exact native-window/document-input focus schema required')
        for key in ['mainFrame','localOrigin','loginRoute','enabled','visible','hitPassword','hitSubmit','documentFocused','documentVisible']:
            if type(document[key]) is not bool:raise ValueError('Only boolean document eligibility flags allowed')
        for key in ['schemaVersion','childFrames','passwordInputs','submitButtons']:
            receipt_reader.integer(document[key],0,100000)
        import math
        for key in ['timeOriginMs','atMs']:
            number=document[key]
            if type(number) not in [int,float] or not math.isfinite(number) or number<0:
                raise ValueError('Finite document metadata clock required')
    return value

@contextmanager
def session(adb,read,output,observer,uid,binding,gate):
    package=binding['collector'];app=binding['package'];token=uuid.uuid4().hex
    folder=output/'target-instrumentation';folder.mkdir(exist_ok=False)
    remote='/sdcard/Android/data/'+APP+'/files/rf312-frames-'+token
    process=None;installed=False;stdout=stderr=None;original_error=None
    deadline=min(observer.deadline,time.monotonic()+45)
    accepted=False;ready=None;files={};proof={'accepted':False,'defaultPerformanceSampleAccepted':False}

    def remaining():
        left=deadline-time.monotonic()
        if not 0<left<=45:raise TimeoutError('Original native frame phase deadline exhausted')
        return left
    def call(argv,*,missing=False):
        try:
            result=adb(*argv,timeout=min(5,remaining()),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            code,out,err=result.returncode,result.stdout,result.stderr
        except subprocess.CalledProcessError as error:code,out,err=error.returncode,error.stdout,error.stderr
        remaining()
        if missing and code==1 and out==err==b'':return None
        if code!=0 or err:raise RuntimeError('Native frame SDK operation rejected')
        return out
    def package_hash():
        base=read('shell','pm','path',PKG)
        if not re.fullmatch(r'package:/data/app/(?:[A-Za-z0-9_~+=.-]+/){1,2}base\.apk',base):
            raise RuntimeError('Exact installed frame collector required')
        if read('shell','sha256sum',base.removeprefix('package:')).split()!=[package['sha256'],base.removeprefix('package:')]:
            raise RuntimeError('Installed frame collector changed')
    def document(name):
        if name not in ['ready.json','frames.json']:raise ValueError('Exact native frame metadata name required')
        while call(['shell','test','-f',remote+'/'+name+'.sha256'],missing=True) is None:
            if process.poll() is not None:raise RuntimeError('Frame collector ended before expected metadata')
            time.sleep(min(.025,remaining()))
        marker=call(['exec-out','cat',remote+'/'+name+'.sha256'])
        raw=call(['exec-out','cat',remote+'/'+name])
        if not re.fullmatch(b'[a-f0-9]{64}\n',marker) or sha(raw)!=marker.decode().strip():
            raise RuntimeError('Complete hash-bound native frame metadata required')
        receipt_reader.decode(raw)
        for key,body in [(name,raw),(name+'.sha256',marker)]:
            (folder/key).write_bytes(body);files[key]=sha(body)
        return raw
    class Channel:
        def __init__(self):
            self.deadline=deadline;self.process=ready['process'];self.metadata=None
        def finish(self):
            nonlocal accepted
            if self.metadata is not None:raise RuntimeError('Frame evidence cannot be acknowledged twice')
            raw=document('frames.json')
            self.metadata=receipt_reader.validate(ready,receipt_reader.decode(raw),run_id=token,
                expected_process=self.process,target_apk_sha256=app['sha256'],collector_apk_sha256=package['sha256'],expected_api=31)
            focus=call(['shell','dumpsys','window','displays']).decode()
            if not re.search(r'mCurrentFocus=.*'+re.escape(APP)+'/',focus):
                raise RuntimeError('Actual foreground application window required before acknowledgement')
            if call(['shell','pidof',APP]).decode().strip()!=str(self.process['processId']):
                raise RuntimeError('Original target PID required before acknowledgement')
            stat=call(['exec-out','cat','/proc/'+str(self.process['processId'])+'/stat']).decode()
            if int(stat[stat.rfind(')')+2:].split()[19])!=self.process['startTicks']:
                raise RuntimeError('Original target birth required before acknowledgement')
            gate.check({COMPONENT:APP,UI:UI.split('/')[0]},deadline=deadline,process_ids={APP:self.process['processId']})
            call(['shell','touch',remote+'/accept']);files['accept']=sha(b'')
            process.wait(timeout=remaining());remaining()
            stdout.close();stderr.close()
            if process.returncode!=0 or (folder/'instrumentation.stderr').read_bytes():raise RuntimeError('Frame command failed')
            terminal((folder/'instrumentation.stdout').read_text(encoding='utf-8'))
            save(folder/'metadata-contract.json',self.metadata)
            names=call(['shell','ls','-A',remote]).decode().splitlines()
            if len(names)!=5 or set(names)!=set(files):raise RuntimeError('Exact five owned frame metadata files required')
            if call(['shell','find',remote,'-type','l']):raise RuntimeError('Frame metadata symlinks rejected')
            rows=call(['shell','sha256sum',*[remote+'/'+name for name in sorted(names)]]).decode().splitlines()
            observed={}
            for line in rows:
                match=re.fullmatch(r'([a-f0-9]{64})  '+re.escape(remote)+r'/([^/]+)',line)
                if not match or match[2] in observed:raise RuntimeError('Unambiguous owned frame metadata hashes required')
                observed[match[2]]=match[1]
            if observed!=files:raise RuntimeError('Frame metadata changed before cleanup')
            save(folder/'owned-remote-file-hashes.json',files)
            call(['shell','rm','--',*[remote+'/'+name for name in sorted(names)]])
            call(['shell','rmdir','--',remote])
            accepted=True
            return self.metadata
    try:
        if read('shell','pm','list','packages',PKG):raise RuntimeError('Frame collector already installed')
        adb('install','--no-streaming',package['apk'],stdout=subprocess.PIPE,stderr=subprocess.PIPE);installed=True
        package_hash()
        gate.check({UI:UI.split('/')[0]},deadline=deadline)
        stdout=(folder/'instrumentation.stdout').open('xb');stderr=(folder/'instrumentation.stderr').open('xb')
        command=[binding['host']['adbExecutable'],'-H','127.0.0.1','-P','5057','-s',binding['host']['serial'],
            'shell','am','instrument','-w','-r','-e','runId',token,'-e','targetApkSha256',app['sha256'],
            '-e','privateDataBackedUp','true',COMPONENT]
        process=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
        save(folder/'live.json',dict(pid=process.pid,command=command,originalHostPhaseDeadline=deadline))
        ready=ready_contract(document('ready.json'),token,app['sha256'],package['sha256'],uid)
        identity=ready['process'];pid=identity['processId']
        if call(['shell','pidof',APP]).decode().strip()!=str(pid):raise RuntimeError('Original target process required')
        stat=call(['exec-out','cat','/proc/'+str(pid)+'/stat']).decode()
        if int(stat[stat.rfind(')')+2:].split()[19])!=identity['startTicks']:raise RuntimeError('Target process birth changed')
        gate.check({COMPONENT:APP,UI:UI.split('/')[0]},deadline=deadline,process_ids={APP:pid})
        channel=Channel();yield channel
        if not accepted:raise RuntimeError('External frame evidence was not acknowledged')
    except BaseException as error:
        original_error=error;proof['originalErrorType']=type(error).__name__
        # Preserve all bounded known files, including refusals. Never manufacture a pass.
        for name in ['ready.json','ready.json.sha256','frames.json','frames.json.sha256','failure.json','failure.json.sha256']:
            if not (folder/name).exists():
                try:
                    value=adb('exec-out','cat',remote+'/'+name,timeout=5,stdout=subprocess.PIPE,stderr=subprocess.PIPE).stdout
                    if len(value)>32768:raise ValueError('Oversized refusal evidence')
                    (folder/('retained-'+name)).write_bytes(value)
                except Exception as failure:proof.setdefault('captureErrors',[]).append(type(failure).__name__)
        raise
    finally:
        errors=[]
        if process is not None and process.poll() is None:
            try:process.wait(timeout=15) # Original separate cleanup budget; never accepted measurement time.
            except Exception as error:
                errors.append(type(error).__name__)
                try:
                    adb('shell','am','force-stop',APP,timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    process.wait(timeout=10);proof['forcedOwnTargetStop']=True
                except Exception as stop:errors.append(type(stop).__name__)
        if original_error is not None and process is not None and process.poll() is not None:
            # Pull the refusal AFTER the actor has finished. Retain earlier failed
            # reads independently; only the original marker proves complete bytes.
            try:
                body=adb('exec-out','cat',remote+'/failure.json',timeout=5,stdout=subprocess.PIPE,stderr=subprocess.PIPE).stdout
                marker=adb('exec-out','cat',remote+'/failure.json.sha256',timeout=5,stdout=subprocess.PIPE,stderr=subprocess.PIPE).stdout
                value=refusal_contract(body,marker,token)
                (folder/'terminal-failure.json').write_bytes(body)
                (folder/'terminal-failure.json.sha256').write_bytes(marker)
                proof['completeTerminalRefusalPreserved']=True
            except Exception as capture:proof['terminalRefusalCaptureErrorType']=type(capture).__name__
        for handle in [stdout,stderr]:
            if handle is not None:handle.close()
        if installed:
            try:
                package_hash();adb('uninstall',PKG,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                proof['collectorUninstalled']=not read('shell','pm','list','packages',PKG)
                gate.check({UI:UI.split('/')[0]},deadline=time.monotonic()+10)
                proof['collectorInstrumentationAbsent']=True
            except Exception as error:errors.append(type(error).__name__)
        proof.update(errors=errors,accepted=bool(accepted and not original_error and not errors
            and proof.get('collectorUninstalled') and proof.get('collectorInstrumentationAbsent')))
        save(folder/'cleanup.json',proof)
        if not proof['accepted'] and original_error is None:raise RuntimeError('Frame collector cleanup not verified')
