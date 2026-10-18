"""One physical startup UI checkpoint plus native frame commits; no unlock or input."""
import hashlib,json,re,subprocess,time
from pathlib import Path
import frame_binding
import frame_ui_session as ui
import frame_session as native
from rf312_state_gate import Gate
import default_journey as original
import process_identity as reader
from diagnostic_exchange import binding as bind_transport

APP=original.APP;S=original.S
sha=original.sha;save=original.save;labels=original.labels

def observe(adb,read,output,count,uid,public_image_name,inputs,binding):
    for name,key in [('observer_session.py','journeyObserverSessionSha256'),('process_identity.py','processReaderSha256'),('live_pixel_counter.py','livePixelCounterSha256')]:
        if sha(S/name)!=inputs[key]:raise RuntimeError('Frozen native startup dependencies changed')
    original_adb,original_read=adb,read
    gate=Gate(adb,output);gate.check({},deadline=time.monotonic()+10)
    process_id=None;stage_deadline=None
    def focused(package=APP):
        if package!=APP:raise RuntimeError('Startup proof only admits actual application focus')
        if not re.search(r'mCurrentFocus=.*'+re.escape(APP)+'/',read('shell','dumpsys','window','displays')):
            raise RuntimeError('Native application focus required')
        if reader.read_process_ids(read,APP)!=process_id:raise RuntimeError('Startup process identity changed')
        gate.check({native.COMPONENT:APP,native.UI:ui.PKG},deadline=stage_deadline,process_ids={APP:int(process_id)})
    host=binding['host']
    with ui.session(adb,read,output,package=inputs['observer'],executable=host['adbExecutable'],serial=host['serial'],gate=gate) as observer:
        with bind_transport(observer,adb,output,executable=host['adbExecutable'],serial=host['serial'],
                expected_executable_sha256=host['adbSha256'],script=frame_binding.LEGACY/'exchange_control.sh',shell_crlf=True) as (adb,read):
            adb('shell','am','force-stop',APP,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            if reader.read_process_ids(read,APP):raise RuntimeError('Startup requires a stopped target')
            with native.session(original_adb,original_read,output,observer,uid,binding,gate) as collector:
                stage_deadline=collector.deadline
                j=original.Journey(observer,focused,output,inputs)
                began=time.monotonic();framework=read('shell','am','start','-W','-n',APP+'/.MainActivity')
                if not re.search(r'^Status:\s*ok\s*$',framework,re.M):raise RuntimeError('Native startup failed')
                (j.proof/'framework-start.txt').write_text(framework,encoding='utf-8')
                process_id=reader.read_process_ids(read,APP)
                if process_id!=str(collector.process['processId']):raise RuntimeError('Native startup process differs')
                save(j.proof/'observed-process-start.json',collector.process)
                login=lambda:any(n.get('class')=='android.widget.EditText' and n.get('password')=='true' for n in j.nodes) and any(f'acc_rf312_{count}' in text for text in labels(j.nodes))
                row=j.wait(login,stage_deadline,capture=True)
                password=j.field(True,stage_deadline)
                button=j.unique(lambda:j.action_candidates(r'Unlock'),stage_deadline)
                row=j.wait(login,stage_deadline,capture=True)
                focused(APP)
                j.finish('startup',began,row,twoCommittedFramesProven=False,nativeFrameContractPending=True)
                metadata=collector.finish()
                summary=dict(success=True,scope='default-apk-target-instrumented-native-startup-frame-evidence',objectCount=count,
                    stageNames=['startup'],stages=j.phases,metadataContractAccepted=metadata['frameCommitContractAccepted'],
                    committedFrames=metadata['committedFrames'],targetInstrumentationInjected=True,metadataJavaScriptEvaluation=True,
                    defaultReleaseNonDebuggable=True,physicalStartupScreenshot=True,independentPasswordAndButtonHitChecks=True,
                    physicalDisplayPresentationProven=False,applicationOnlyLatencyAccepted=False,defaultPerformanceSampleAccepted=False,
                    fullUiJourneyAccepted=False,fullRF312Accepted=False,actualIpcAccepted=False,
                    uiInputDispatched=False,supportedNativeInstrumentationStateVerified=True,nativeInstrumentationStateChecks=len(gate.proof))
    gate.check({},deadline=time.monotonic()+10)
    for folder in ['target-instrumentation','independent-observer','persistent-control']:
        if json.loads((output/folder/'cleanup.json').read_text())['accepted'] is not True:
            raise RuntimeError('Complete native startup cleanup required')
    summary['nativeInstrumentationStateChecks']=len(gate.proof)
    save(output/'default-external/evidence.json',summary)
    return summary
