"""Independent UI control during explicit target diagnostics; no default acceptance."""
from contextlib import contextmanager
from pathlib import Path
import hashlib,json,os,re,shlex,subprocess,time,uuid
import diagnostic_binding
import observer_session as original
S=original.S
PKG=original.PKG
APP=original.APP
Client=original.Client
sha=original.sha
save=original.save
@contextmanager
def session(adb,read,output,*,package,executable,serial):
    apk=Path(package['apk'])
    assert package['passed'] and package['instrumentationTarget']==PKG and not package['targetInstrumentationInjected']
    assert sha(apk)==package['sha256']
    assert all(sha(S/n)==h for n,h in package['sourceSha256'].items())
    assert not read('shell','pm','list','packages',PKG)
    folder=output/'independent-observer';folder.mkdir(exist_ok=False)
    token=uuid.uuid4().hex;remote='files/'+token+'/'
    installed=False;process=None;stdout=stderr=None;client=None;original_error=None
    cleanup=dict(accepted=False,scope='independent-ui-observer-during-explicit-target-diagnostic',targetInstrumentationInjected=True,javaScriptInjected=True,metadataJavaScriptEvaluation=True,ownObserverInstrumentsOnlySelf=True,originalObserverAbsent=True)
    try:
        adb('install','--no-streaming',str(apk),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        installed=True
        base=read('shell','pm','path',PKG)
        assert re.fullmatch(r'package:/data/app/(?:[A-Za-z0-9_~+=.-]+/){1,2}base\.apk',base)
        actual=read('shell','sha256sum',base.removeprefix('package:')).split()
        assert actual==[package['sha256'],base.removeprefix('package:')]
        assert APP not in read('shell','dumpsys','activity','instrumentation')
        stdout=(folder/'instrumentation.stdout.log').open('xb');stderr=(folder/'instrumentation.stderr.log').open('xb')
        command=[str(executable),'-H','127.0.0.1','-P','5057','-s',serial,
                 'shell','am','instrument','-w','-r','-e','token',token,PKG+'/'+PKG+'.Observer']
        process=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
        save(folder/'live.json',dict(pid=process.pid,command=command,observerApkSha256=package['sha256']))
        client=Client(adb,folder,process,remote);deadline=time.monotonic()+15
        while not client.exists('ready',deadline):
            client.alive(deadline);time.sleep(.025)
        cleanup['connectedBeforeTargetStart']=True
        yield client
    except BaseException as error:
        original_error=error;cleanup['originalError']=repr(error);raise
    finally:
        errors=[]
        if client is not None:
            deadline=time.monotonic()+10
            # Cleanup gets its own original ten-second stop budget even after
            # measurement expiry; it cannot extend accepted measurement time.
            client.deadline=deadline
            try:
                if process.poll() is None:
                    client.call('shell','run-as',PKG,'touch',remote+'stop',deadline=deadline)
                    cleanup['commandExitCode']=process.wait(timeout=max(.001,deadline-time.monotonic()))
                else:cleanup['commandExitCode']=process.returncode
                cleanup['normalStopReceipt']=client.exists('stopped',deadline)
                if client.exists('error',deadline):cleanup['observerError']=client.get('error',deadline).decode()
            except Exception as error:
                errors.append(repr(error))
                if process.poll() is None:
                    adb('shell','am','force-stop',PKG,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    process.wait(timeout=10);cleanup['forcedOwnObserverStop']=True
            cleanup['requests']=client.ordinal
        for handle in (stdout,stderr):
            if handle is not None:handle.close()
        if installed:
            try:
                current=read('shell','pm','path',PKG)
                assert current==base
                assert read('shell','sha256sum',current.removeprefix('package:')).split()==[package['sha256'],current.removeprefix('package:')]
                adb('uninstall',PKG,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                cleanup['observerUninstalled']=not read('shell','pm','list','packages',PKG)
                cleanup['observerInstrumentationAbsent']=PKG not in read('shell','dumpsys','activity','instrumentation')
            except Exception as error:errors.append(repr(error))
        cleanup['errors']=errors
        cleanup['accepted']=bool(cleanup.get('normalStopReceipt') and cleanup.get('observerUninstalled')
            and cleanup.get('observerInstrumentationAbsent') and cleanup.get('commandExitCode')==0
            and not cleanup.get('observerError') and not cleanup.get('forcedOwnObserverStop') and not errors)
        save(folder/'cleanup.json',cleanup)
        if not cleanup['accepted'] and original_error is None:raise RuntimeError('Independent observer cleanup not verified')
