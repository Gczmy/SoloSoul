"""Complete twelve-step default Release UI journey; metrics/IPC acceptance remains separate."""
from pathlib import Path
import hashlib,importlib.util,json,re,subprocess,time
import xml.etree.ElementTree as ET
from native_journey_contract import PHASES,FILES,labels,box,visible,record_ids,expected_search_ids,public_filename,public_text_present,file_set,complete,verify_text_frame,PublicContentNotReady
from live_pixel_counter import NativeOcrReader,wait_for_counter
from body_navigation import main_navigation_top,open_tool_card
from preview_scope import close_preview as close_fixture_preview
from scaled_ocr import create_half_png,remap_half_receipt
from persistent_adapter import binding as bind_transport,SnapshotCadence
from scroll_readiness import scroll_if_ready
S=Path(__file__).parent;APP='com.solosoul.app';PICKER='com.android.documentsui'
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,value):
 with p.open('x',encoding='utf-8') as stream:json.dump(value,stream,indent=2)
def module(name,path):
 spec=importlib.util.spec_from_file_location(name,path);value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value

class Journey:
 def __init__(self,observer,focused,output,inputs):
  self.cadence=SnapshotCadence()
  self.observer=observer;self.focused=focused;self.output=output;self.inputs=inputs
  self.bindings=dict(inputs['fullJourneyFiles'])
  self.ocr=NativeOcrReader(inputs['pixelInputFiles'],output);self.phases=[];self.scrolls=[]
  self.proof=output/'default-external';self.proof.mkdir(exist_ok=False)
  self.ocr_folder=output/'journey-native-ocr';self.ocr_folder.mkdir(exist_ok=False)
 def remaining(self,deadline):
  value=deadline-time.monotonic()
  if not 0<value<=45:raise TimeoutError('Original 45-second phase budget exhausted')
  return value
 def snapshot(self,deadline,capture=False):
  self.cadence.before(deadline)
  self.row=self.observer.snapshot(self.remaining(deadline),screenshot=capture)
  self.tree=ET.fromstring(self.row['xml']);self.parents={c:p for p in self.tree.iter() for c in p}
  self.all=[n for n in self.tree.iter('node') if n.get('package')==self.row['rootPackage']]
  self.nodes=[n for n in self.all if visible(n)]
  return self.row
 def wait(self,predicate,deadline,*,capture=False):
  while self.remaining(deadline)>0:
   self.snapshot(deadline)
   if self.row['allowedRoot'] and predicate():
    self.focused(self.row['rootPackage'])
    if capture:
     self.snapshot(deadline,True)
     if not predicate():raise RuntimeError('Native state changed during screenshot')
    return self.row
 def action_candidates(self,pattern,*,body=False):
  limit=main_navigation_top(self.nodes) if body else None
  if body and limit is None:return []
  candidates=[]
  for n in self.nodes:
   if not any(re.fullmatch(pattern,value) for value in labels([n])):continue
   node=n
   while node is not None and node.tag=='node' and node.get('clickable')!='true' and node.get('class')!='android.widget.EditText':node=self.parents.get(node)
   if node is None or node.tag!='node' or not visible(node):continue
   if body and (box(node)[1]+box(node)[3])/2>=limit:continue
   if not any(node is old for old in candidates):candidates.append(node)
  return candidates
 def unique(self,candidates,deadline,*,intent='tap',allow_scroll=False):
  stable=None;first=0;observations=0;scrolls=0
  while self.remaining(deadline)>0:
   self.snapshot(deadline);found=candidates()
   if len(found)!=1:
    stable=None;observations=0
    if not found and allow_scroll and scrolls<8:
     if self.scroll(deadline):scrolls+=1
    continue
   node=found[0];self.focused(node.get('package'))
   probe=self.observer.inspect(node,intent,self.remaining(deadline))
   if not self.observer.target_ready(node,probe,intent):stable=None;observations=0;continue
   current=(self.observer.target(node),tuple(probe['imeBounds']) if intent=='type-public' else ())
   now=time.monotonic()
   if current!=stable:stable=current;first=now;observations=1
   else:observations+=1
   if observations>=3 and now-first>=.350:return node
 def scroll(self,deadline,direction='forward'):
  candidates=[n for n in self.nodes if n.get('scrollable')=='true' and box(n)[3]-box(n)[1]>=300]
  webviews=[n for n in candidates if n.get('class')=='android.webkit.WebView']
  chosen=webviews if len(webviews)==1 else candidates
  if not chosen:return False
  if len(chosen)!=1:raise RuntimeError('Exactly one visible native scroll container required')
  node=chosen[0]
  return scroll_if_ready(self,node,direction,deadline)
 def tap(self,pattern,deadline,*,body=False,allow_scroll=False):
  node=self.unique(lambda:self.action_candidates(pattern,body=body),deadline,allow_scroll=allow_scroll)
  return self.observer.tap(node,self.remaining(deadline))
 def field(self,password,deadline):
  return self.unique(lambda:[n for n in self.nodes if n.get('class')=='android.widget.EditText'
   and n.get('password')==str(password).lower() and n.get('text','')==''],deadline)
 def type_fixture(self,password,deadline):
  node=self.field(password,deadline);self.observer.tap(node,self.remaining(deadline))
  node=self.unique(lambda:[n for n in self.nodes if n.get('class')=='android.widget.EditText'
   and n.get('password')==str(password).lower() and n.get('focused')=='true' and n.get('text','')==''],deadline,intent='type-public')
  value='perf-baseline-only-password' if password else 'needle'
  self.observer.type_public(node,value,self.remaining(deadline))
  self.wait(lambda:any(n.get('class')=='android.widget.EditText' and len(n.get('text',''))==len(value) for n in self.nodes),deadline)
 def pick_file(self,filename,deadline):
  public_filename(filename)
  def candidates():
   return [n for n in self.nodes if n.get('package')==PICKER and n.get('class')=='android.widget.LinearLayout'
     and n.get('focusable')=='true' and n.get('text','')=='' and n.get('content-desc','').startswith(filename+', ')]
  node=self.unique(candidates,deadline,intent='pick-public-file',allow_scroll=True)
  return self.observer.pick_file(node,filename,self.remaining(deadline))
 def finish(self,name,started,row,**extra):
  if name!=PHASES[len(self.phases)]:raise RuntimeError('Full journey phase order mismatch')
  if not all(sha(S/n)==h for n,h in self.bindings.items()):raise RuntimeError('Frozen full journey sources changed')
  self.remaining(started+45)
  filename=row.get('screenshot','')
  if not re.fullmatch(r'frame-\d{3}\.png',filename) or row.get('rootPackage')!=APP or not row.get('allowedRoot'):
   raise RuntimeError('Owned native phase screenshot required')
  image=self.output/'independent-observer'/filename
  record=dict(name=name,hostObservedUpperBoundMs=(time.monotonic()-started)*1000,nativeScreenshot=filename,
   imageSha256=sha(image),nativeScreenshotVerified=True,**extra)
  self.phases.append(record)
  (self.proof/'phase-progress.json').write_text(json.dumps(self.phases,indent=2),encoding='utf-8')
 def content(self,tokens,deadline,*,region=None,allow_scroll=False,native_pdf=False):
  scrolls=0;observations=[]
  while self.remaining(deadline)>0:
   self.snapshot(deadline,True);row=self.row
   if row['rootPackage']!=APP:raise RuntimeError('Only application content may be verified')
   area=region() if callable(region) else region
   image=self.output/'independent-observer'/row['screenshot'];receipt=self.ocr_folder/(image.stem+'.json')
   self.focused(APP);data=self.ocr(image,receipt,deadline)
   try:proof=verify_text_frame(image,data,tokens,area)
   except PublicContentNotReady as error:
    if native_pdf:
     if area!=self.pdf_region():raise RuntimeError('Exact native PDF body region required')
     half_image=self.ocr_folder/(image.stem+'-half.png');half_receipt=self.ocr_folder/(image.stem+'-half.json')
     self.remaining(deadline);create_half_png(image,half_image);self.remaining(deadline)
     self.ocr(half_image,half_receipt,deadline)
     mapped,transform=remap_half_receipt(image,receipt,half_image,half_receipt)
     try:proof=verify_text_frame(image,mapped,tokens,area)
     except PublicContentNotReady as half_error:error=half_error
     else:
      proof['supplementalOcr']=transform
      self.focused(APP);self.remaining(deadline);return row,proof
    observations.append(dict(frame=image.name,imageSha256=sha(image),notReady=str(error)))
    (self.ocr_folder/'loading-observations.json').write_text(json.dumps(observations,indent=2),encoding='utf-8')
    if allow_scroll and scrolls<8 and self.scroll(deadline):scrolls+=1
    continue
   self.focused(APP);self.remaining(deadline);return row,proof
 def home(self,deadline):
  self.tap(r'Home(?:, current page)?',deadline)
  return self.wait(lambda:'Welcome back, Performance Fixture' in labels(self.nodes),deadline)
 def workspace(self,deadline,count):
  self.tap(r'Identity \d+ objects',deadline,body=True,allow_scroll=True)
  self.wait(lambda:any(re.fullmatch(r'All(?: \d+)?',v) for v in labels(self.nodes)),deadline)
  self.tap(r'All(?: \d+)?',deadline,body=True)
  return self.wait(lambda:0 in record_ids(self.nodes) and len(record_ids(self.all))==min(count,50),deadline,capture=True)
 def attachments(self,deadline):
  return self.wait(lambda:file_set(self.nodes)==FILES,deadline,capture=True)
 def open_preview(self,name,deadline):
  self.tap(re.escape('Attachment actions: '+name),deadline,body=True,allow_scroll=True)
  self.tap(r'Preview',deadline)
 def close_preview(self,name,deadline):
  def receipt(value):
   value['imageSha256']=sha(self.output/'independent-observer'/value['nativeScreenshot'])
   save(self.proof/('close-'+name+'.json'),value)
  return close_fixture_preview(self,name,deadline,lambda:file_set(self.nodes)==FILES,receipt)
 def pdf_region(self):
  nodes=[n for n in self.nodes if n.get('view-id','').endswith('/pdfPageImage')]
  if len(nodes)!=1:raise RuntimeError('Exact native PDF image view required')
  return box(nodes[0])

def observe(adb,read,output,count,uid,public_image_name):
 public_filename(public_image_name);inputs=load(S/'inputs.json')
 for name,key in [('journey-observer-session.py','journeyObserverSessionSha256'),('android-home-native-regression-fixed.py','processReaderSha256'),('live_pixel_counter.py','livePixelCounterSha256')]:
  assert sha(S/name)==inputs[key]
 helper=module('independent_journey_session',S/'journey-observer-session.py')
 reader=module('strict_default_process_reader',S/'android-home-native-regression-fixed.py')
 process_id=None
 def focused(package=APP):
  if package not in (APP,PICKER):raise RuntimeError('Unowned requested focus')
  if not re.search(r'mCurrentFocus=.*'+re.escape(package)+'/',read('shell','dumpsys','window','displays')):raise RuntimeError('Native owner focus required')
  if process_id is not None and reader.read_process_ids(read,APP)!=process_id:raise RuntimeError('Main application process changed')
  if APP in read('shell','dumpsys','activity','instrumentation'):raise RuntimeError('Target instrumentation forbidden')
 original_sdk_adb=adb
 with helper.session(adb,read,output) as observer:
  with bind_transport(observer,adb,output,executable=helper.BASE/'sdk/platform-tools/adb.exe',serial='emulator-5578',expected_executable_sha256='b4a6b455702684652cccf7b46258b29e653538904359a58fd4931cf3ef286b3f',script=S/'control.sh',shell_crlf=True) as (adb,read):
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
   summary=dict(success=True,scope='default-release-independent-complete-ui-journey',objectCount=count,
    stageNames=list(PHASES),stages=j.phases,fullUiJourneyAccepted=True,fullDefaultJourneyAccepted=False,
    performanceDistributionAccepted=False,targetInstrumentationInjected=False,javaScriptInjected=False,
    defaultReleaseNonDebuggable=True,twoCommittedFramesProven=False,scrollGestures=len(j.scrolls),
    ipcAttributed=False,strictAllProcessMemoryCollected=False,sameSourceObserverOverheadMeasured=False)
   save(j.proof/'evidence.json',summary)
 assert load(output/'independent-observer/cleanup.json')['accepted']
 assert load(output/'persistent-control/cleanup.json')['accepted']
 checkpoint=module('post_ui_memory_checkpoint',S/'memory_checkpoint.py')
 memory=module('original_strict_memory_reader',S/'android-process-memory.py')
 assert observer.adb is original_sdk_adb
 checkpoint.collect(memory.collect_snapshot,original_sdk_adb,output/'post-journey-memory',uid=uid,pid=int(process_id))
 return summary
