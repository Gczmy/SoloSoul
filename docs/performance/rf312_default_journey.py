"""Default Release journey with supported native state checks; no app injection."""
import json,re,subprocess,time
from pathlib import Path
import rf312_default_binding
import default_journey as original
from default_journey import (APP,PICKER,labels,record_ids,expected_search_ids,FILES,file_set,
    public_text_present,public_filename,open_tool_card,wait_for_counter,complete,PHASES)
from persistent_adapter import binding as bind_transport
from rf312_state_gate import Gate
import rf312_default_memory as checkpoint
S=original.S
load=original.load
sha=original.sha
save=original.save
module=original.module


class Journey(original.Journey):
 def __init__(self,*args):
  super().__init__(*args);self.current_phase_deadline=None
 def remaining(self,deadline):
  value=super().remaining(deadline);self.current_phase_deadline=deadline;return value


def observe(adb,read,output,count,uid,public_image_name,inputs):
 public_filename(public_image_name)
 for name,key in [('observer_session.py','journeyObserverSessionSha256'),('process_identity.py','processReaderSha256'),('live_pixel_counter.py','livePixelCounterSha256')]:
  assert sha(S/name)==inputs[key]
 helper=module('independent_journey_session',S/'observer_session.py')
 reader=module('strict_default_process_reader',S/'process_identity.py')
 process_id=None
 def focused(package=APP):
  if package not in (APP,PICKER):raise RuntimeError('Unowned requested focus')
  if not re.search(r'mCurrentFocus=.*'+re.escape(package)+'/',read('shell','dumpsys','window','displays')):raise RuntimeError('Native owner focus required')
  if process_id is not None and reader.read_process_ids(read,APP)!=process_id:raise RuntimeError('Main application process changed')
  gate.check({helper.PKG+'/'+helper.PKG+'.Observer':helper.PKG},
      deadline=min(observer.deadline,journey.current_phase_deadline))
 original_sdk_adb=adb
 gate=Gate(adb,output)
 gate.check({},deadline=time.monotonic()+10)
 with helper.session(adb,read,output,package=inputs['observer'],executable=inputs['host']['adbExecutable'],serial=inputs['host']['serial']) as observer:
  with bind_transport(observer,adb,output,executable=inputs['host']['adbExecutable'],serial=inputs['host']['serial'],expected_executable_sha256=inputs['host']['adbSha256'],script=S/'persistent_control.sh',shell_crlf=True) as (adb,read):
   journey=Journey(observer,focused,output,inputs);j=journey
   adb('shell','am','force-stop',APP,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   assert not reader.read_process_ids(read,APP)
   began=time.monotonic();framework=read('shell','am','start','-W','-n',APP+'/.MainActivity')
   assert re.search(r'^Status:\s*ok\s*$',framework,re.M)
   (j.proof/'framework-start.txt').write_text(framework,encoding='utf-8')
   process_id=reader.read_process_ids(read,APP);assert re.fullmatch(r'[1-9]\d*',process_id)
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
   summary=dict(success=True,scope='default-release-supported-state-complete-ui-journey',objectCount=count,
    stageNames=list(PHASES),stages=j.phases,fullUiJourneyAccepted=True,fullDefaultJourneyAccepted=False,
    performanceDistributionAccepted=False,targetInstrumentationInjected=False,javaScriptInjected=False,
    defaultReleaseNonDebuggable=True,twoCommittedFramesProven=False,scrollGestures=len(j.scrolls),
    ipcAttributed=False,strictAllProcessMemoryCollected=False,sameSourceObserverOverheadMeasured=False)
   save(j.proof/'evidence.json',summary)
 gate.check({},deadline=time.monotonic()+10)
 summary['supportedNativeStateVerified']=True
 summary['supportedStateChecks']=len(gate.proof)
 save(j.proof/'supported-state-ui-evidence.json',summary)
 assert load(output/'independent-observer/cleanup.json')['accepted']
 assert load(output/'persistent-control/cleanup.json')['accepted']
 memory_path=Path(inputs['host']['workspaceRoot'])/'tauri/scripts/android-process-memory.py'
 assert sha(memory_path)==inputs['canonicalMemoryReaderSha256']
 memory=module('original_strict_memory_reader',memory_path)
 assert observer.adb is original_sdk_adb
 result=checkpoint.collect(memory.collect_snapshot,original_sdk_adb,output/'post-journey-memory',uid=uid,pid=int(process_id))
 assert result['directAbsenceVerified'] and result['supportedStateChecks']==2
 summary['postJourneyDirectAbsenceVerified']=True
 save(j.proof/'supported-state-evidence.json',summary)
 return summary
