"""Strict existing-call adapter; setup/cleanup use the caller's original SDK."""
from contextlib import contextmanager
import hashlib,json,math,re,shlex,subprocess,time
from pathlib import Path
from persistent_control import Control

PKG='org.solosoul.rf312observer'
APP='com.solosoul.app'

class Adapter:
    def __init__(self,control,remote,argv,deadline,*,shell_crlf):
        if not re.fullmatch(r'files/[a-f0-9]{32}/',remote):raise ValueError('Exact observer namespace required')
        if type(shell_crlf) is not bool:raise ValueError('Explicit verified SDK shell newline profile required')
        self.control,self.remote,self.argv,self.deadline,self.shell_crlf=control,remote,list(argv),deadline,shell_crlf

    def run(self,*command,**kwargs):
        if set(kwargs)-{'timeout','stdout','stderr','input'}:raise ValueError('Unsupported existing SDK call options')
        if kwargs.get('stdout')!=subprocess.PIPE or kwargs.get('stderr')!=subprocess.PIPE:
            raise ValueError('Exact captured-output call contract required')
        timeout=kwargs.get('timeout',240)
        if isinstance(timeout,bool) or not isinstance(timeout,(int,float)) or not math.isfinite(timeout) or not 0<timeout<=240:
            raise ValueError('Original finite SDK budget required')
        payload=kwargs.get('input',b'');name='-';operation=None
        if command[:3]==('shell','run-as',PKG):
            tail=command[3:]
            if len(tail)==3 and tail[:2]==('test','-e'):operation,name='exists',tail[2]
            elif len(tail)==4 and tail[:3]==('stat','-c','%s'):operation,name='size',tail[3]
            elif len(tail)==2 and tail[0]=='touch':operation,name='stop',tail[1]
            if operation:
                if not name.startswith(self.remote):raise ValueError('Observer file namespace differs')
                name=name[len(self.remote):]
        elif len(command)==5 and command[:4]==('exec-out','run-as',PKG,'cat'):
            if not command[4].startswith(self.remote):raise ValueError('Observer file namespace differs')
            operation,name='read',command[4][len(self.remote):]
        elif len(command)==2 and command[0]=='shell':
            match=re.search(r'request-([0-9]{3})\.tmp',command[1])
            if match:
                name='request-'+match[1]+'.json';tmp=self.remote+'request-'+match[1]+'.tmp';final=self.remote+name
                expected='run-as '+PKG+' sh -c '+shlex.quote('umask 077; cat > '+tmp+' && mv '+tmp+' '+final)
                if command[1]!=expected:raise ValueError('Atomic request command differs')
                operation='put'
        if operation is None:
            operation={('shell','dumpsys','window','displays'):'focus',('shell','pidof',APP):'pid',
                ('shell','dumpsys','activity','instrumentation'):'instrumentation',
                ('shell','am','force-stop',APP):'app-stop',('shell','am','start','-W','-n',APP+'/.MainActivity'):'app-start'}.get(command)
        if operation is None:raise ValueError('Unsupported measurement command; no SDK fallback or retry')
        remaining=min(timeout,self.deadline-time.monotonic())
        if remaining<=0:raise TimeoutError('Original observer aggregate deadline exhausted')
        try:
            value=self.control.call(operation,name,payload=payload,timeout=remaining)
            code,out,err=value.returncode,value.stdout,value.stderr
        except subprocess.CalledProcessError as error:
            code,out,err=error.returncode,error.stdout,error.stderr
        # Reproduce only the verified SDK shell serialization. exec-out file
        # bytes remain untouched, including JSON, PNG and literal CR/LF bytes.
        if command[0]=='shell' and self.shell_crlf:
            out=out.replace(b'\n',b'\r\n');err=err.replace(b'\n',b'\r\n')
        result=subprocess.CompletedProcess(self.argv+list(command),code,out,err)
        result.check_returncode();return result

    def read(self,*command):
        return self.run(*command,stdout=subprocess.PIPE,stderr=subprocess.PIPE).stdout.decode().strip()

class SnapshotCadence:
    def __init__(self,*,clock=time.monotonic,sleep=time.sleep):
        self.clock,self.sleep,self.last=clock,sleep,None
    def before(self,deadline):
        now=self.clock()
        if now>=deadline:raise TimeoutError('Original snapshot phase deadline exhausted')
        due=now if self.last is None else self.last+.175
        if due>now:self.sleep(min(due-now,deadline-now))
        now=self.clock()
        if now>=deadline:raise TimeoutError('Original snapshot phase deadline exhausted')
        if now+1e-9<due:raise RuntimeError('Snapshot cadence sleep returned prematurely')
        self.last=now

@contextmanager
def binding(observer,original_adb,output,*,executable,serial,expected_executable_sha256,script,shell_crlf):
    executable=Path(executable);script=Path(script)
    sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    if sha(executable)!=expected_executable_sha256:raise ValueError('Frozen SDK executable differs')
    if serial!='emulator-5578' or not re.fullmatch(r'files/[a-f0-9]{32}/',observer.remote):
        raise ValueError('Original explicit endpoint and observer namespace required')
    folder=Path(output)/'persistent-control';folder.mkdir(exist_ok=False)
    token=observer.remote.split('/')[1];scratch='/data/local/tmp/rf312-control-'+token
    argv=[str(executable),'-H','127.0.0.1','-P','5057','-s',serial]
    control=None;process=None;created=False;original_error=None;previous=observer.adb
    report=dict(accepted=False,inputRetried=False,originalObserverApkChanged=False,originalReadersChanged=False)
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
        adapter=Adapter(control,observer.remote,argv,observer.deadline,shell_crlf=shell_crlf)
        observer.adb=adapter.run
        yield adapter.run,adapter.read
    except BaseException as error:
        original_error=error;report.update(originalErrorType=type(error).__name__);raise
    finally:
        # Restore original callbacks before original observer stop/whole-data
        # recovery. A failed measurement is never sent again on another link.
        observer.adb=previous
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
            accepted=not errors and bool(report.get('normalControlClose')) and bool(report.get('ownedRemoteScratchRemoved')))
        (folder/'cleanup.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
        if not report['accepted'] and original_error is None:raise RuntimeError('Persistent control cleanup not accepted')
