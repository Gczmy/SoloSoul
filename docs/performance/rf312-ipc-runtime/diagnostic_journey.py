"""Explicit target diagnostic journey; default observe/session are never called."""
from contextlib import contextmanager
from pathlib import Path
import hashlib
import json
import re
import subprocess
import time
import uuid

import diagnostic_binding
import default_journey as original
from default_journey import (APP,PICKER,labels,record_ids,expected_search_ids,FILES,file_set,
    public_text_present,public_filename,open_tool_card,wait_for_counter,complete,PHASES)
from diagnostic_exchange import binding as bind_transport
import process_identity as reader
import diagnostic_ui_session as ui
import native_collector as native
import ipc_receipt
import rf312_instrumentation_state as native_state
from rf312_state_gate import Gate

S=original.S
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
save=original.save
COLLECTOR='org.solosoul.rf312ipc'
CLASS=COLLECTOR+'.Collector'


def active_instrumentations(text):
    return native_state.expected(native_state.parse(text.encode('utf-8')),
        {COLLECTOR+'/'+CLASS:APP,ui.PKG+'/'+ui.PKG+'.Observer':ui.PKG})


def terminal(text,token):
    text=text.replace('\r\n','\n');lines=text.splitlines()
    required=['INSTRUMENTATION_RESULT: collectorPassed=true',
              'INSTRUMENTATION_RESULT: completedStages=12',
              'INSTRUMENTATION_RESULT: runId='+token,'INSTRUMENTATION_CODE: -1']
    if (any(lines.count(value)!=1 for value in required)
            or re.findall(r'^INSTRUMENTATION_STATUS: stage=(.+)$',text,re.M)!=list(PHASES)
            or [int(value) for value in re.findall(r'^INSTRUMENTATION_STATUS: completedStages=(\d+)$',text,re.M)]!=list(range(1,13))
            or re.findall(r'^INSTRUMENTATION_STATUS_CODE: (-?\d+)$',text,re.M)!=['0']*12
            or re.search(r'INSTRUMENTATION_FAILED|INSTRUMENTATION_ABORTED|Process crashed|collectorPassed=false',text)):
        raise RuntimeError('Exact successful twelve-stage collector terminal output required')


def available(process,channel,folder,token):
    if process.poll() is None:return True
    if process.returncode!=0 or channel is None or channel.attempted!=12:return False
    try:terminal((folder/'instrumentation.stdout').read_text(encoding='utf-8'),token)
    except (ValueError,RuntimeError,OSError):return False
    return (folder/'instrumentation.stderr').read_bytes()==b''


class Journey(original.Journey):
    def __init__(self,observer,focused,output,inputs,collector):
        super().__init__(observer,focused,output,inputs)
        destination=output/'diagnostic-external'
        if destination.exists():raise RuntimeError('Fresh diagnostic proof directory required')
        self.proof.rename(destination);self.proof=destination
        self.collector=collector
        self.current_phase_deadline=None

    def remaining(self,deadline):
        value=super().remaining(deadline)
        self.current_phase_deadline=deadline
        return value

    def finish(self,name,started,row,**extra):
        super().finish(name,started,row,**extra)
        stamp=time.monotonic()
        record=self.collector.capture(name,started+45)
        self.remaining(started+45)
        self.phases[-1].update(metadataResponseSha256=self.collector.proof[-1]['responseSha256'],
            nativeMetadataReadNs=record['nativeReadFinishedNs']-record['nativeReadStartedNs'],
            metadataHostReadUpperBoundMs=(time.monotonic()-stamp)*1000)
        (self.proof/'phase-progress.json').write_text(json.dumps(self.phases,indent=2),encoding='utf-8')


@contextmanager
def target_session(adb,read,output,observer,uid,diagnostic,gate):
    receipt=diagnostic['collector'];app=diagnostic['package'];token=uuid.uuid4().hex
    folder=output/'target-instrumentation';folder.mkdir(exist_ok=False)
    remote='/sdcard/Android/data/'+APP+'/files/rf312-ipc-'+token
    process=None;installed=False;channel=None;stdout=stderr=None;error=None
    proof=dict(accepted=False,targetInstrumentationInjected=True,metadataJavaScriptEvaluation=True,
               defaultPerformanceSampleAccepted=False,collectorApkSha256=receipt['sha256'],targetApkSha256=app['sha256'])
    overall=observer.deadline

    def remaining(deadline):
        value=min(deadline,overall)-time.monotonic()
        if not 0<value<=45:raise TimeoutError('Original diagnostic phase/aggregate budget exhausted')
        return value

    def sdk(argv,*,timeout):
        try:
            result=adb(*argv,timeout=timeout,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            return result.returncode,result.stdout,result.stderr
        except subprocess.CalledProcessError as problem:
            return problem.returncode,problem.stdout,problem.stderr

    def checked(argv,deadline,*,missing=False):
        try:code,out,err=sdk(argv,timeout=min(5,remaining(deadline)))
        except subprocess.CalledProcessError as problem:
            code,out,err=problem.returncode,problem.stdout,problem.stderr
        remaining(deadline)
        if missing and code==1 and out==err==b'':return None
        if code!=0 or err!=b'':raise RuntimeError('Diagnostic handshake SDK operation failed')
        return out

    def package_hash():
        base=read('shell','pm','path',COLLECTOR)
        if not re.fullmatch(r'package:/data/app/(?:[A-Za-z0-9_~+=.-]+/){1,2}base\.apk',base):
            raise RuntimeError('One installed owned collector APK required')
        if read('shell','sha256sum',base.removeprefix('package:')).split()!=[receipt['sha256'],base.removeprefix('package:')]:
            raise RuntimeError('Installed collector APK changed')

    try:
        if read('shell','pm','list','packages',COLLECTOR):raise RuntimeError('Collector already installed; refusing replacement')
        adb('install','--no-streaming',receipt['apk'],stdout=subprocess.PIPE,stderr=subprocess.PIPE);installed=True
        package_hash()
        gate.check({ui.PKG+'/'+ui.PKG+'.Observer':ui.PKG},deadline=min(overall,time.monotonic()+10))
        stdout=(folder/'instrumentation.stdout').open('xb');stderr=(folder/'instrumentation.stderr').open('xb')
        command=[diagnostic['host']['adbExecutable'],'-H','127.0.0.1','-P','5057',
            '-s',diagnostic['host']['serial'],'shell','am','instrument','-w','-r',
            '-e','runId',token,'-e','targetApkSha256',app['sha256'],'-e','privateDataBackedUp','true',COLLECTOR+'/'+CLASS]
        process=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
        save(folder/'live.json',dict(pid=process.pid,command=command))
        deadline=min(overall,time.monotonic()+45)
        while checked(['shell','test','-f',remote+'/ready.json.sha256'],deadline,missing=True) is None:
            if process.poll() is not None:raise RuntimeError('Collector ended before native handshake')
            time.sleep(min(.025,remaining(deadline)))
        marker=checked(['exec-out','cat',remote+'/ready.json.sha256'],deadline)
        raw=checked(['exec-out','cat',remote+'/ready.json'],deadline)
        if not re.fullmatch(b'[a-f0-9]{64}\n',marker) or hashlib.sha256(raw).hexdigest()!=marker.decode().strip():
            raise RuntimeError('Complete bound native ready receipt required')
        (folder/'ready.json').write_bytes(raw)
        channel=native.Collector(sdk,lambda:available(process,channel,folder,token),output/'ipc-metadata',raw,
            run_id=token,target_apk_sha256=app['sha256'],collector_apk_sha256=receipt['sha256'])
        channel.deadline=min(channel.deadline,overall)
        pid=channel.process['processId']
        if channel.process['uid']!=uid or read('shell','pidof',APP)!=str(pid):raise RuntimeError('Original app UID/PID required')
        stat=checked(['exec-out','cat','/proc/'+str(pid)+'/stat'],deadline).decode()
        if int(stat[stat.rfind(')')+2:].split()[19])!=channel.process['startTicks']:raise RuntimeError('Original native process birth required')
        yield channel
        metadata=channel.finish();save(folder/'metadata-contract.json',metadata)
        process.wait(timeout=min(15,max(.001,overall-time.monotonic())))
        if time.monotonic()>overall:raise TimeoutError('Collector terminal exceeded original aggregate deadline')
        stdout.close();stderr.close()
        if process.returncode!=0 or (folder/'instrumentation.stderr').read_bytes()!=b'':raise RuntimeError('Native collector command failed')
        terminal((folder/'instrumentation.stdout').read_text(encoding='utf-8'),token)
        proof['allTwelveNativeCheckpoints']=True
        expected={'ready.json':hashlib.sha256(raw).hexdigest(),'ready.json.sha256':hashlib.sha256(marker).hexdigest()}
        for name in channel.output.iterdir():
            expected[name.name]=sha(name)
        for row in channel.proof:
            name='phase-'+str(row['sequence']).zfill(3)+'.json.sha256'
            expected[name]=hashlib.sha256((row['responseSha256']+'\n').encode()).hexdigest()
        names=read('shell','ls','-A',remote).splitlines()
        if len(expected)!=50 or len(names)!=50 or set(names)!=set(expected):raise RuntimeError('Exact owned metadata directory required')
        if read('shell','find',remote,'-type','l'):raise RuntimeError('Owned metadata cannot contain symbolic links')
        hashes=read('shell','sha256sum',*[remote+'/'+name for name in sorted(names)])
        observed={}
        for line in hashes.splitlines():
            match=re.fullmatch(r'([a-f0-9]{64})  '+re.escape(remote)+r'/([^/]+)',line)
            if not match:raise RuntimeError('Unambiguous owned metadata hashes required')
            observed[match[2]]=match[1]
        if observed!=expected:raise RuntimeError('Owned metadata changed before removal')
        save(folder/'owned-remote-file-hashes.json',expected)
        adb('shell','rm','--',*[remote+'/'+name for name in sorted(names)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        adb('shell','rmdir','--',remote,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        proof['ownedMetadataRemoved']=True
    except BaseException as problem:
        error=problem;proof['originalErrorType']=type(problem).__name__
        if channel is not None:
            try:
                diagnostic_log=adb('shell','logcat','-d','--pid='+str(channel.process['processId']),timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                (folder/'failure-app-logcat.stdout').write_bytes(diagnostic_log.stdout)
                (folder/'failure-app-logcat.stderr').write_bytes(diagnostic_log.stderr)
            except Exception as capture_error:proof['failureLogcatErrorType']=type(capture_error).__name__
        raise
    finally:
        errors=[]
        if process is not None and process.poll() is None:
            try:
                adb('shell','touch',remote+'/abort',timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                process.wait(timeout=15)
            except Exception as problem:
                errors.append(repr(problem))
                try:
                    adb('shell','am','force-stop',APP,timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    process.wait(timeout=10);proof['forcedOwnTargetStop']=True
                except Exception as stop_error:errors.append(repr(stop_error))
        for handle in [stdout,stderr]:
            if handle is not None:handle.close()
        if installed:
            try:
                package_hash();adb('uninstall',COLLECTOR,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                proof['collectorUninstalled']=not read('shell','pm','list','packages',COLLECTOR)
                gate.check({ui.PKG+'/'+ui.PKG+'.Observer':ui.PKG},deadline=time.monotonic()+10)
                proof['collectorInstrumentationAbsent']=True
                proof['supportedNativeStateVerified']=True
            except Exception as problem:errors.append(repr(problem))
        proof.update(errors=errors,accepted=bool(proof.get('allTwelveNativeCheckpoints') and proof.get('ownedMetadataRemoved')
            and proof.get('collectorUninstalled') and proof.get('collectorInstrumentationAbsent') and not errors and not error))
        save(folder/'cleanup.json',proof)
        if not proof['accepted'] and error is None:raise RuntimeError('Explicit diagnostic collector cleanup not verified')


def observe(adb,read,output,count,uid,public_image_name,inputs,diagnostic):
 public_filename(public_image_name)
 for name,key in [('observer_session.py','journeyObserverSessionSha256'),('process_identity.py','processReaderSha256'),('live_pixel_counter.py','livePixelCounterSha256')]:
  assert sha(S/name)==inputs[key]
 process_id=None
 def focused(package=APP):
  if package not in (APP,PICKER):raise RuntimeError('Unowned requested focus')
  if not re.search(r'mCurrentFocus=.*'+re.escape(package)+'/',read('shell','dumpsys','window','displays')):raise RuntimeError('Native owner focus required')
  if process_id is not None and reader.read_process_ids(read,APP)!=process_id:raise RuntimeError('Main application process changed')
  gate.check({COLLECTOR+'/'+CLASS:APP,ui.PKG+'/'+ui.PKG+'.Observer':ui.PKG},
      deadline=min(observer.deadline,journey.current_phase_deadline),process_ids={APP:int(process_id)})
 original_sdk_adb,original_sdk_read=adb,read
 gate=Gate(adb,output)
 gate.check({},deadline=time.monotonic()+10)
 host=diagnostic['host']
 with ui.session(adb,read,output,package=inputs['observer'],executable=host['adbExecutable'],serial=host['serial']) as observer:
  with bind_transport(observer,adb,output,executable=host['adbExecutable'],serial=host['serial'],expected_executable_sha256=host['adbSha256'],script=Path(__file__).parent/'exchange_control.sh',shell_crlf=True) as (adb,read):
   adb('shell','am','force-stop',APP,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   assert not reader.read_process_ids(read,APP)
   with target_session(original_sdk_adb,original_sdk_read,output,observer,uid,diagnostic,gate) as collector:
    journey=Journey(observer,focused,output,inputs,collector);j=journey
    began=time.monotonic();framework=read('shell','am','start','-W','-n',APP+'/.MainActivity')
    assert re.search(r'^Status:\s*ok\s*$',framework,re.M)
    (j.proof/'framework-start.txt').write_text(framework,encoding='utf-8')
    process_id=reader.read_process_ids(read,APP);assert process_id==str(collector.process['processId'])
    save(j.proof/'observed-process-start.json',dict(pid=int(process_id),uid=uid))
    login=lambda:any(n.get('class')=='android.widget.EditText' and n.get('password')=='true' for n in j.nodes) and any(f'acc_rf312_{count}' in v for v in labels(j.nodes))
    row=j.wait(login,began+45,capture=True);j.finish('startup',began,row,twoCommittedFramesProven=False)
    def unlock(name,namespace):
     began=time.monotonic();deadline=began+45;j.type_fixture(True,deadline);j.tap(r'Unlock',deadline)
     row,nodes,proof=wait_for_counter(observer,output,count,deadline,lambda:focused(APP),j.ocr,namespace=namespace)
     j.finish(name,began,row,objectCount=count,livePixelCounter=proof)
    unlock('password-unlock','pixel-counter')
    began=time.monotonic();deadline=began+45
    j.tap(r'Tools',deadline);open_tool_card(j,'OCR',deadline);j.wait(lambda:'OCR Scan' in labels(j.nodes),deadline)
    j.tap(r'Select Image',deadline,body=True,allow_scroll=True)
    j.wait(lambda:j.row['rootPackage']==PICKER,deadline)
    j.pick_file(public_image_name,deadline)
    j.wait(lambda:j.row['rootPackage']==APP and public_text_present(j.all),deadline)
    row,proof=j.content(('hello','solosoul','1234567890'),deadline,allow_scroll=True);j.finish('first-ocr-user-flow',began,row,knownPublicText=proof)
    j.home(time.monotonic()+45)
    began=time.monotonic();row=j.workspace(began+45,count);j.finish('workspace',began,row,renderedRecordIds=sorted(record_ids(j.all)))
    began=time.monotonic();deadline=began+45
    j.tap(r'Tools',deadline);open_tool_card(j,'Search',deadline);j.type_fixture(False,deadline)
    wanted=expected_search_ids(count)
    row=j.wait(lambda:record_ids(j.all)==wanted and bool(record_ids(j.nodes)),deadline)
    # Hide only the owned search IME before the physical result screenshot.
    if any(n.get('class')=='android.widget.EditText' and n.get('focused')=='true' for n in j.nodes):observer.back(APP,j.remaining(deadline))
    row=j.wait(lambda:record_ids(j.all)==wanted and bool(record_ids(j.nodes)),deadline,capture=True)
    j.finish('search-needle',began,row,expectedPublicRecordIds=sorted(wanted))
    j.home(time.monotonic()+45);j.workspace(time.monotonic()+45,count)
    began=time.monotonic();deadline=began+45
    j.tap(r'Record 00000000(?: .*)?',deadline,body=True)
    j.tap(r'Attachments(?: \(4\))?',deadline)
    row=j.attachments(deadline);j.finish('attachments-open',began,row,knownAttachmentNames=sorted(FILES))
    for phase,name in [('image-preview','ocr_test.png'),('text-preview','preview.txt')]:
     began=time.monotonic();deadline=began+45;j.open_preview(name,deadline)
     row,proof=j.content(('hello','solosoul','1234567890'),deadline)
     j.finish(phase,began,row,publicContent=proof);j.close_preview(name,time.monotonic()+45)
    for phase,name,pages in [('pdf-text-native','text_only.pdf',2),('pdf-scanned-native','scanned.pdf',1)]:
     began=time.monotonic();deadline=began+45;j.open_preview(name,deadline)
     pdf=lambda: name in labels(j.nodes) and f'1 / {pages}' in labels(j.nodes) and any(n.get('view-id','').endswith('/pdfPageImage') for n in j.nodes)
     j.wait(pdf,deadline)
     tokens=('hello','solosoul','ocr','pdf','text','layer','test','page','one') if pages==2 else ('hello','solosoul','1234567890')
     row,proof=j.content(tokens,deadline,region=j.pdf_region,native_pdf=True)
     evidence=dict(firstPage=proof,pages=pages,sourcePublicFixtureVerified=True,decryptedPrivateFileRead=False)
     if pages==2:
      j.tap(r'Next page',deadline);j.wait(lambda:'2 / 2' in labels(j.nodes),deadline)
      row,second=j.content(('this','is','page','two','of','the','text','only','pdf','fixture'),deadline,region=j.pdf_region,native_pdf=True);evidence['secondPage']=second
     j.finish(phase,began,row,pdf=evidence);j.close_preview(name,time.monotonic()+45)
    observer.back(APP,45);j.wait(lambda:not file_set(j.nodes),time.monotonic()+45)
    j.tap(r'Close',time.monotonic()+45)
    began=time.monotonic();deadline=began+45;j.tap(r'Lock Vault',deadline)
    row=j.wait(login,deadline,capture=True);j.finish('application-lock',began,row,passwordLoginRestored=True)
    unlock('password-reunlock','reunlock-pixel-counter');complete(j.phases)
    summary=dict(success=True,scope='explicit-target-diagnostic-independent-complete-ui-and-ipc-journey',objectCount=count,
     stageNames=list(PHASES),stages=j.phases,fullUiJourneyAccepted=True,fullDefaultJourneyAccepted=False,
     performanceDistributionAccepted=False,targetInstrumentationInjected=True,javaScriptInjected=True,metadataJavaScriptEvaluation=True,
     defaultReleaseNonDebuggable=True,twoCommittedFramesProven=False,scrollGestures=len(j.scrolls),
     metadataContractAccepted=collector.finish()['metadataContractAccepted'],actualIpcAccepted=False,fullRF312Accepted=False,strictAllProcessMemoryCollected=False,sameSourceObserverOverheadMeasured=False)

 gate.check({},deadline=time.monotonic()+10)
 summary['supportedNativeInstrumentationStateVerified']=True
 summary['nativeInstrumentationStateChecks']=len(gate.proof)
 assert json.loads((output/'target-instrumentation/cleanup.json').read_text())['accepted']
 assert json.loads((output/'independent-observer/cleanup.json').read_text())['accepted']
 assert json.loads((output/'persistent-control/cleanup.json').read_text())['accepted']
 save(j.proof/'evidence.json',summary)
 return summary
