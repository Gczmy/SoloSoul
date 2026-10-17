from contextlib import contextmanager
from pathlib import Path
import base64,hashlib,json,re,shlex,subprocess,time
import diagnostic_exchange as exchange

@contextmanager
def binding(observer,original_adb,output,*,executable,serial,expected_executable_sha256,script,shell_crlf):
    if not isinstance(observer,exchange.Client)or observer.composite_bound:raise ValueError('Fresh explicit composite client required')
    executable=Path(executable);script=Path(script)
    sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
    if sha(executable)!=expected_executable_sha256:raise ValueError('Frozen SDK differs')
    if serial!='emulator-5578'or not re.fullmatch(r'files/[a-f0-9]{32}/',observer.remote):raise ValueError('Exact endpoint and private namespace required')
    folder=Path(output)/'persistent-control';folder.mkdir(exist_ok=False)
    token=observer.remote.split('/')[1];scratch=observer.remote+'owned-ui'
    argv=[str(executable),'-H','127.0.0.1','-P','5057','-s',serial]
    previous=observer.adb;control=process=None;created=False;original_error=None
    report=dict(accepted=False,inputRetried=False,originalObserverApkChanged=False,
        compositeExchange=True,ownedUidControl=True,javaFileTransport=True,originalReadersChanged=False)
    def sdk(*args,timeout=10,**kwargs):
        return original_adb(*args,timeout=timeout,stdout=subprocess.PIPE,stderr=subprocess.PIPE,**kwargs)
    def owned(*args,**kwargs):return sdk('shell','run-as',exchange.PKG,*args,**kwargs)
    try:
        uid=owned('id','-u').stdout.strip()
        if not re.fullmatch(rb'[1-9][0-9]*',uid)or not 10000<=int(uid)<=999999999:raise RuntimeError('Explicit Observer UID required')
        owned('sh','-c',shlex.quote('umask 077; mkdir '+scratch));created=True
        assert owned('stat','-c','%a',scratch).stdout.strip()==b'700'
        assert owned('stat','-c','%u',scratch).stdout.strip()==uid
        sdk('shell','-T','-e','none','run-as',exchange.PKG,'sh','-c',shlex.quote('umask 077; base64 -d > '+scratch+'/control.jar'),input=base64.b64encode(script.read_bytes()))
        owned('chmod','444',scratch+'/control.jar')
        assert owned('sha256sum',scratch+'/control.jar').stdout.decode().split()==[sha(script),scratch+'/control.jar']
        artifact_bytes=sdk('exec-out','run-as',exchange.PKG,'cat',scratch+'/control.jar').stdout
        (folder/'device-readback.jar').write_bytes(artifact_bytes)
        assert artifact_bytes==script.read_bytes()
        body='CLASSPATH='+scratch+'/control.jar app_process /system/bin org.solosoul.rf312transport.OwnedUi '+token+' '+scratch+' '+uid.decode()
        command=argv+['shell','-T','-e','none','run-as',exchange.PKG,'sh','-c',shlex.quote(body)]
        process=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        (folder/'live.json').write_text(json.dumps(dict(pid=process.pid,command=command,scriptSha256=sha(script),observerUid=int(uid)),indent=2),encoding='utf-8')
        control=exchange.Control(process,folder/'rpc')
        assert control.call('ping',timeout=10).stdout==b'rf312-control-ready\n'
        budget=min(10,observer.deadline-time.monotonic())
        if budget<=0:raise TimeoutError('Original observer deadline exhausted')
        assert control.call('get-stable','ready',timeout=budget).stdout==b'ready'
        report['stableReadinessRoundTripVerified']=True
        adapter=exchange.Adapter(control,observer.remote,argv,observer.deadline,shell_crlf=shell_crlf)
        observer.adb=adapter.run;observer.composite_bound=True
        yield adapter.run,adapter.read
    except BaseException as error:
        original_error=error;report['originalErrorType']=type(error).__name__;raise
    finally:
        observer.composite_bound=False;observer.adb=previous
        errors=[]
        try:
            if control is not None:
                broken=control.broken;control.close()
                report.update(controlExitCode=process.returncode,rpcFrames=control.sequence,
                    normalControlClose=not broken and not control.stderr and process.returncode==0)
            elif process is not None and process.poll()is None:
                process.terminate();process.wait(timeout=10)
            if created:
                assert owned('stat','-c','%a',scratch).stdout.strip()==b'700'
                assert owned('stat','-c','%u',scratch).stdout.strip()==uid
                if owned('ls','-A',scratch).stdout.decode().strip()!='control.jar':raise RuntimeError('Unexpected owned UID scratch; retaining')
                assert owned('sha256sum',scratch+'/control.jar').stdout.decode().split()==[sha(script),scratch+'/control.jar']
                owned('rm','--',scratch+'/control.jar');owned('rmdir','--',scratch)
                report['ownedRemoteScratchRemoved']=True
        except BaseException as error:errors.append(dict(type=type(error).__name__,message=str(error)))
        report.update(errors=errors,originalCallbackRestored=observer.adb is previous)
        report['accepted']=bool(not errors and report.get('normalControlClose')and report.get('ownedRemoteScratchRemoved')and report.get('stableReadinessRoundTripVerified'))
        (folder/'cleanup.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        if not report['accepted']and original_error is None:raise RuntimeError('Owned UID cleanup not accepted')
