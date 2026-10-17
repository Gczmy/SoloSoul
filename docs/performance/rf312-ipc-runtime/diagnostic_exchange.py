"""Single owned UI exchange; canonical validators, deadlines and raw frames stay intact."""
from contextlib import contextmanager
from pathlib import Path
import base64,hashlib,json,math,queue,re,shlex,subprocess,time
import xml.etree.ElementTree as ET
import diagnostic_binding
import observer_session as original_session
import persistent_control as original_control
from persistent_adapter import Adapter as OriginalAdapter

PKG=original_session.PKG
APP=original_session.APP
ALLOWED=original_session.ALLOWED
save=original_session.save


def request_line(sequence,operation,name='-',payload=b''):
    if operation not in ('exchange','get-stable'):
        return original_control.request_line(sequence,operation,name,payload)
    if type(sequence) is not int or not 1<=sequence<=16384 or not isinstance(name,str) or not isinstance(payload,bytes):
        raise ValueError('Explicit bounded composite protocol types required')
    if operation=='exchange':
        match=re.fullmatch(r'request-([0-9]{3})\.json',name)
        valid=match is not None and 1<=int(match[1])<=512 and 0<len(payload)<=16384
    else:
        valid=re.fullmatch(r'(?:ready|error|stopped|response-[0-9]{3}\.json|frame-[0-9]{3}\.png)',name) is not None and payload==b''
    if not valid:raise ValueError('Only exact owned exchange/read names and bounded payloads allowed')
    body=base64.b64encode(payload).decode() if operation=='exchange' else '-'
    return f'RF312B {sequence} {operation} {name} {body}\n'.encode('ascii')


class Control(original_control.Control):
    def call(self, operation, name='-', payload=b'', *, timeout):
        if isinstance(timeout, bool) or not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or not 0 < timeout <= 240:
            raise ValueError('Original finite positive budget up to 240 seconds required')
        if not self.lock.acquire(blocking=False):
            raise RuntimeError('Concurrent control calls refused')
        started = time.monotonic()
        try:
            if self.broken:
                raise RuntimeError('Broken control channel cannot be reused or replayed')
            line = request_line(self.sequence + 1, operation, name, payload)
            self.sequence += 1
            partial = bytearray()
            row = dict(sequence=self.sequence, operation=operation, name=name, timeoutSeconds=timeout,
                       requestSha256=hashlib.sha256(line).hexdigest())
            answer = queue.Queue(maxsize=1)
            failure = None
            try:
                self.exchanges.put_nowait((line, self.sequence, partial, answer))
                remaining = timeout - (time.monotonic() - started)
                if remaining <= 0:
                    raise queue.Empty
                success, reply = answer.get(timeout=remaining)
                if not success:
                    raise reply
                if time.monotonic() - started > timeout:
                    raise queue.Empty
                if self.stderr_overflow or self.stderr_error or self.stderr:
                    raise RuntimeError('Control transport emitted unexpected stderr')
                code, out, err = reply
                row.update(exitCode=code, stdoutSha256=hashlib.sha256(out).hexdigest(),
                           stderrSha256=hashlib.sha256(err).hexdigest())
            except BaseException as error:
                row.update(errorType=type(error).__name__, timedOut=isinstance(error, queue.Empty))
                try:
                    self.abort()
                except BaseException as cleanup_error:
                    row['ownedPipeCleanupErrorType'] = type(cleanup_error).__name__
                if isinstance(error, queue.Empty):
                    failure = subprocess.TimeoutExpired(self.process.args, timeout, output=bytes(partial), stderr=bytes(self.stderr))
                    raise failure from error
                failure = error
                raise
            finally:
                raw = bytes(partial)
                prefix = self.output / f'{self.sequence:05d}'
                try:
                    prefix.with_suffix('.request').write_bytes(line)
                    prefix.with_suffix('.reply').write_bytes(raw)
                    row.update(elapsedSeconds=time.monotonic()-started, replyBytes=len(raw),
                               replySha256=hashlib.sha256(raw).hexdigest())
                    prefix.with_suffix('.json').write_text(json.dumps(row, indent=2)+'\n', encoding='utf-8')
                except OSError as receipt_error:
                    if failure is not None:
                        raise failure from receipt_error
                    self.abort()
                    raise
            result = subprocess.CompletedProcess([operation, name], code, out, err)
            result.check_returncode()
            return result
        finally:
            self.lock.release()



class Adapter(OriginalAdapter):
    def run(self,*command,**kwargs):
        if not command or command[0] not in ('rf312-exchange','rf312-get-stable'):
            return super().run(*command,**kwargs)
        if (len(command)!=2 or not isinstance(command[1],str) or not command[1].startswith(self.remote)
                or set(kwargs)-{'timeout','stdout','stderr','input'}
                or kwargs.get('stdout')!=subprocess.PIPE or kwargs.get('stderr')!=subprocess.PIPE):
            raise ValueError('Exact owned captured composite operation required')
        budget=kwargs.get('timeout',240)
        if type(budget) not in (int,float) or not math.isfinite(budget) or not 0<budget<=240:
            raise ValueError('Original finite positive budget required')
        budget=min(budget,self.deadline-time.monotonic())
        if budget<=0:raise TimeoutError('Original observer aggregate deadline exhausted')
        operation='exchange' if command[0]=='rf312-exchange' else 'get-stable'
        name=command[1][len(self.remote):];payload=kwargs.get('input',b'')
        request_line(1,operation,name,payload)
        try:
            value=self.control.call(operation,name,payload=payload,timeout=budget)
            code,out,err=value.returncode,value.stdout,value.stderr
        except subprocess.CalledProcessError as error:code,out,err=error.returncode,error.stdout,error.stderr
        if code==0 and err:raise RuntimeError('Composite helper stderr cannot be accepted')
        result=subprocess.CompletedProcess(self.argv+list(command),code,out,err)
        result.check_returncode();return result


class Client(original_session.Client):
    composite_bound=False
    def get(self,name,deadline):
        if not self.composite_bound:return super().get(name,deadline)
        if not isinstance(name,str) or re.fullmatch(r'(?:ready|error|stopped|response-[0-9]{3}\.json|frame-[0-9]{3}\.png)',name) is None:
            raise ValueError('Invalid protocol name')
        raw=self.call('rf312-get-stable',self.remote+name,deadline=deadline).stdout
        if not 0<len(raw)<=4*1024*1024:raise RuntimeError('Invalid bounded stable response size')
        return raw
    def request(self,value,timeout):
        if not self.composite_bound:raise RuntimeError('Explicit composite binding required for measurement')
        if not 0<timeout<=45:raise ValueError('Per-phase observer budget must be within 45 seconds')
        self.ordinal+=1
        if self.ordinal>512:raise RuntimeError('Bounded observer request count exhausted')
        name=f'{self.ordinal:03d}';deadline=min(self.deadline,time.monotonic()+timeout)
        payload=json.dumps(value,ensure_ascii=True,separators=(',',':')).encode()
        if len(payload)>16384:raise ValueError('Bounded public request too large')
        save(self.output/('request-'+name+'.json'),value)
        if self.process.poll() is not None:raise RuntimeError('Independent observer exited early')
        raw=self.call('rf312-exchange',self.remote+'request-'+name+'.json',input=payload,deadline=deadline).stdout
        response='response-'+name+'.json'
        result=json.loads(raw)
        if result.get('success') is not True or result.get('action')!=value['action']:
            raise RuntimeError('Independent observer action mismatch')
        if type(result.get('nativeStartedAtNs')) is not int or type(result.get('nativeFinishedAtNs')) is not int or result['nativeFinishedAtNs']<result['nativeStartedAtNs']:
            raise RuntimeError('Invalid native observation clock')
        if type(result.get('allowedRoot')) is not bool or result['allowedRoot']!=(result.get('rootPackage') in ALLOWED):
            raise RuntimeError('Invalid root ownership receipt')
        if value['action']=='snapshot':
            xml=result.get('xml')
            if not isinstance(xml,str) or '<!DOCTYPE' in xml.upper():raise RuntimeError('Invalid hierarchy receipt')
            tree=ET.fromstring(xml)
            if tree.tag!='hierarchy' or len(list(tree.iter('node')))>5000:raise RuntimeError('Invalid bounded hierarchy')
            if not result['allowedRoot'] and list(tree):raise RuntimeError('Unrelated hierarchy contents refused')
        else:
            if not result['allowedRoot'] or result.get('rootPackage')!=value['expectedPackage']:
                raise RuntimeError('Input receipt owner differs')
            if value['action']=='inspect-target':
                count=result.get('matchCount');matched=result.get('matched')
                if (type(count) is not int or not 0<=count<=5000 or type(matched) is not bool
                        or matched!=(count==1) or result.get('inputDispatched') is not False or result.get('intent')!=value['intent']):
                    raise RuntimeError('Read-only exact target inspection receipt required')
                if any(type(result.get(k)) is not bool for k in ('targetEnabled','targetVisible','targetFocused','targetFocusable','targetClickable')):
                    raise RuntimeError('Native target readiness flags required')
                boxes=result.get('candidateBounds');compatible=result.get('compatibleCount')
                if (not isinstance(boxes,list) or len(boxes)>16 or type(compatible) is not int or not len(boxes)<=compatible<=5000
                        or any(not isinstance(b,str) or not re.fullmatch(r'\[\d+,\d+\]\[\d+,\d+\]',b) for b in boxes)):
                    raise RuntimeError('Bounded native candidate geometry required')
                ime=result.get('imeBounds');ime_count=result.get('imeWindowCount')
                if (not isinstance(ime,list) or type(ime_count) is not int or not 0<=ime_count==len(ime)<=16
                        or type(result.get('imeVisible')) is not bool or result['imeVisible']!=(ime_count>0)
                        or any(not isinstance(b,str) or not re.fullmatch(r'\[\d+,\d+\]\[\d+,\d+\]',b) for b in ime)):
                    raise RuntimeError('Bounded native IME geometry required')
                if value['intent']=='pick-public-file' and type(result.get('targetPickerFileTile')) is not bool:
                    raise RuntimeError('Native public picker tile probe required')
            if value['action']=='tap' and (result.get('singleFingerPair') is not True or result.get('toolType')!='finger'
                    or result.get('bounds')!=value['target']['bounds']):
                raise RuntimeError('Single exact native finger receipt required')
            if value['action']=='type-public' and (result.get('publicTextOnly') is not True
                    or type(result.get('keyEvents')) is not int or result['keyEvents']<=0 or result['keyEvents']%2):
                raise RuntimeError('Public native keyboard receipt required')
            if value['action']=='scroll' and (result.get('singleFingerGesture') is not True
                    or result.get('toolType')!='finger' or type(result.get('events')) is not int or result['events']!=14
                    or result.get('bounds')!=value['target']['bounds'] or result.get('direction')!=value['direction']):
                raise RuntimeError('Single bounded native finger scroll receipt required')
            if value['action']=='pick-public-file' and (result.get('publicPickerFileOnly') is not True
                    or result.get('fileName')!=value['fileName'] or result.get('previewControlExcluded') is not True
                    or result.get('singleFingerPair') is not True or result.get('toolType')!='finger'
                    or result.get('bounds')!=value['target']['bounds']):
                raise RuntimeError('Exact public picker file finger receipt required')
        if time.monotonic()>deadline:raise TimeoutError('Observer result arrived after phase deadline')
        (self.output/response).write_bytes(raw)
        if 'screenshot' in result:
            frame='frame-'+name+'.png'
            if result['screenshot']!=frame or result.get('rootPackage') not in ALLOWED:raise RuntimeError('Invalid screenshot ownership')
            image=self.get(frame,deadline)
            if not image.startswith(b'\x89PNG\r\n\x1a\n'):raise RuntimeError('Native screenshot is not PNG')
            (self.output/frame).write_bytes(image)
        return result


@contextmanager
def binding(observer,original_adb,output,*,executable,serial,expected_executable_sha256,script,shell_crlf):
    if not isinstance(observer,Client) or observer.composite_bound:raise ValueError('Fresh explicit composite client required')
    executable=Path(executable);script=Path(script)
    sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    if sha(executable)!=expected_executable_sha256:raise ValueError('Frozen SDK executable differs')
    if serial!='emulator-5578' or not re.fullmatch(r'files/[a-f0-9]{32}/',observer.remote):
        raise ValueError('Original explicit endpoint and observer namespace required')
    folder=Path(output)/'persistent-control';folder.mkdir(exist_ok=False)
    token=observer.remote.split('/')[1];scratch='/data/local/tmp/rf312-control-'+token
    argv=[str(executable),'-H','127.0.0.1','-P','5057','-s',serial]
    control=None;process=None;created=False;original_error=None;previous=observer.adb
    report=dict(accepted=False,inputRetried=False,originalObserverApkChanged=False,originalReadersChanged=False,compositeExchange=True)
    def sdk(*command,timeout=10,**kwargs):
        return original_adb(*command,timeout=timeout,stdout=subprocess.PIPE,stderr=subprocess.PIPE,**kwargs)
    try:
        uid=sdk('shell','run-as',PKG,'id','-u').stdout.strip()
        if not re.fullmatch(rb'[1-9][0-9]*',uid) or int(uid)<10000:raise RuntimeError('Owned observer UID required')
        sdk('shell','sh','-c',shlex.quote('umask 077; mkdir '+scratch));created=True
        if sdk('shell','stat','-c','%a',scratch).stdout.strip()!=b'700':raise RuntimeError('Private control scratch mode differs')
        sdk('push',str(script),scratch+'/control.sh')
        if sdk('shell','sha256sum',scratch+'/control.sh').stdout.decode().split()!=[sha(script),scratch+'/control.sh']:
            raise RuntimeError('Actual device helper bytes differ')
        command=argv+['shell','-T','-e','none','sh',scratch+'/control.sh',token,scratch]
        process=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=subprocess.CREATE_NO_WINDOW)
        (folder/'live.json').write_text(json.dumps(dict(pid=process.pid,command=command,scriptSha256=sha(script)),indent=2),encoding='utf-8')
        control=Control(process,folder/'rpc')
        if control.call('ping',timeout=10).stdout!=b'rf312-control-ready\n':raise RuntimeError('Fixed control handshake differs')
        ready_budget=min(10,observer.deadline-time.monotonic())
        if ready_budget<=0:raise TimeoutError('Original observer aggregate deadline exhausted')
        if control.call('get-stable','ready',timeout=ready_budget).stdout!=b'ready':
            raise RuntimeError('Exact stable observer readiness bytes required')
        report['stableReadinessRoundTripVerified']=True
        adapter=Adapter(control,observer.remote,argv,observer.deadline,shell_crlf=shell_crlf)
        observer.adb=adapter.run;observer.composite_bound=True
        yield adapter.run,adapter.read
    except BaseException as error:
        original_error=error;report.update(originalErrorType=type(error).__name__);raise
    finally:
        # Restore original callbacks before original observer stop/whole-data
        # recovery. A failed measurement is never sent again on another link.
        observer.composite_bound=False;observer.adb=previous
        errors=[]
        try:
            if control is not None:
                was_broken=control.broken
                control.close();report.update(controlExitCode=process.returncode,rpcFrames=control.sequence,
                    normalControlClose=not was_broken and not control.stderr and process.returncode==0)
            elif process is not None and process.poll() is None:process.terminate();process.wait(timeout=10)
            if created:
                if sdk('shell','ls','-A',scratch).stdout.decode().strip()!='control.sh':
                    raise RuntimeError('Control scratch has unexpected/live residual files; retaining')
                if sdk('shell','sha256sum',scratch+'/control.sh').stdout.decode().split()!=[sha(script),scratch+'/control.sh']:
                    raise RuntimeError('Cleanup helper identity differs; retaining')
                sdk('shell','rm','--',scratch+'/control.sh');sdk('shell','rmdir','--',scratch)
                report['ownedRemoteScratchRemoved']=True
        except BaseException as error:errors.append(dict(type=type(error).__name__,message=str(error)))
        report.update(errors=errors,originalCallbackRestored=observer.adb is previous,
            accepted=not errors and bool(report.get('normalControlClose')) and bool(report.get('ownedRemoteScratchRemoved'))
                and bool(report.get('stableReadinessRoundTripVerified')))
        (folder/'cleanup.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
        if not report['accepted'] and original_error is None:raise RuntimeError('Persistent control cleanup not accepted')
