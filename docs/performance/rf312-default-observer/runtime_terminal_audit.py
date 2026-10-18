"""Replay original UI, PDF, private-state, ANR and host terminal assertions."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,re,math,sys,xml.etree.ElementTree as ET
from native_journey_contract import PHASES,complete,verify_text_frame,box,FILES,file_set,visible
from preview_scope import preview_controls_present,web_preview_close_candidates,native_pdf_close_candidates
from public_pixel_counter import verified_counter
from scaled_ocr import remap_half_receipt
from owner_inputs import load_plan,verify_plan
from runtime_units import validate as validate_units
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(plan_path,output,args):
 owner=load_plan(plan_path,artifacts=True);S=Path(output);W=Path(owner['runtime']['host']['workspaceRoot'])
 assert re.fullmatch('[a-z0-9-]+',args.host_audit)
 host=load(S/args.host_audit/'result.json');assert host['passed']
 P=S/'native-default-full-journey-r1';master=load(P/'result.json');terminal=load(S/'external-terminal-r2.json')
 assert all(master['cleanup'].get(k) is True for k in ['systemSettingsExactlyRestored','mainUninstalled','testUninstalled',
  'emulatorShutdownRequested','emulatorLauncherExited','ownedServerExited','protectedAdbStateUnchanged'])
 settings=load(P/'system-restoration.json');assert settings['exactlyEqual'] and settings['before']==settings['after'] and len(settings['before'])==9
 verify_plan(owner,artifacts=True)
 unit=load(Path(owner['config']['unitEvidence']));validate_units(unit)
 spec=importlib.util.spec_from_file_location('canonical_private_inventory',W/'tauri/scripts/android-home-native-regression.py')
 home=importlib.util.module_from_spec(spec);spec.loader.exec_module(home)
 def content_proof(out,image,proof):
  receipt=load(out/'journey-native-ocr'/(image.stem+'.json'))
  if 'supplementalOcr' in proof:
   receipt,transform=remap_half_receipt(image,out/'journey-native-ocr'/(image.stem+'.json'),out/'journey-native-ocr'/(image.stem+'-half.png'),out/'journey-native-ocr'/(image.stem+'-half.json'))
   assert transform==proof['supplementalOcr']
  actual=verify_text_frame(image,receipt,proof['knownTokens'],tuple(proof['physicalRegion']))
  assert actual=={k:v for k,v in proof.items() if k!='supplementalOcr'}
  return receipt
 cases=[]
 for row in terminal['outcomes']:
  out=S/('prototype-'+str(row['objectCount']));report=load(out/'report.json');observer=load(out/'independent-observer/cleanup.json')
  before=home.inventory(out/'private-data-before.tar',require_empty=True);after=home.inventory(out/'private-data-restored.tar',require_empty=True)
  assert before==after and before
  assert report['originalInstalledPackages']==report['restoredInstalledPackages']
  assert all(report.get(k) is True for k in ['restored','originalApksRestored','ownedProbeRemoved','blurRestored','ownedPublicOcrInputRemoved','frozenInputsUnchanged'])
  assert not report.get('restoreError') and not report.get('sourceRestoreError')
  assert observer['accepted'] and observer['observerUninstalled'] and observer['observerInstrumentationAbsent'] and not observer['errors']
  assert not observer['targetInstrumentationInjected'] and not observer['javaScriptInjected']
  progress=out/'default-external/phase-progress.json';phases=load(progress) if progress.is_file() else []
  assert [p['name'] for p in phases]==list(PHASES[:len(phases)])
  for phase in phases:
   assert phase['nativeScreenshotVerified'] and sha(out/'independent-observer'/phase['nativeScreenshot'])==phase['imageSha256']
   assert 0<phase['hostObservedUpperBoundMs']<=45000
   response=load(out/'independent-observer'/phase['nativeScreenshot'].replace('frame-','response-').replace('.png','.json'))
   assert response['allowedRoot'] and response['rootPackage']=='com.solosoul.app' and response['screenshot']==phase['nativeScreenshot']
   counter=phase.get('livePixelCounter')
   if counter:
    folder='pixel-counter' if phase['name']=='password-unlock' else 'reunlock-pixel-counter'
    assert re.fullmatch(r'ocr-frame-\d{3}\.json',counter['ocrReceipt']) and counter['count']==row['objectCount']
    actual=verified_counter(out/'independent-observer'/phase['nativeScreenshot'],load(out/folder/counter['ocrReceipt']),row['objectCount'])
    assert all(counter[k]==v for k,v in actual.items()) and counter['originalPhaseBoundKept'] and not counter['accessibilityCounterUsed']
  content_checked=0;pdf_pages_checked=0
  for phase in phases:
   proof=phase.get('knownPublicText') or phase.get('publicContent')
   if proof:
    image=out/'independent-observer'/phase['nativeScreenshot'];receipt=load(out/'journey-native-ocr'/(image.stem+'.json'))
    actual=verify_text_frame(image,receipt,proof['knownTokens'],tuple(proof['physicalRegion']));assert actual==proof;content_checked+=1
   pdf=phase.get('pdf')
   if pdf:
    file_name='text_only.pdf' if phase['name']=='pdf-text-native' else 'scanned.pdf'
    expected_pages=2 if file_name=='text_only.pdf' else 1
    assert pdf['pages']==expected_pages and pdf['decryptedPrivateFileRead'] is False
    for index,key in [(1,'firstPage')]+([(2,'secondPage')] if expected_pages==2 else []):
     proof=pdf[key];images=[p for p in (out/'independent-observer').glob('frame-*.png') if sha(p)==proof['imageSha256']]
     assert images
     image=images[0];receipt=load(out/'journey-native-ocr'/(image.stem+'.json'))
     content_proof(out,image,proof)
     response=load(out/'independent-observer'/image.name.replace('frame-','response-').replace('.png','.json'))
     assert response['allowedRoot'] and response['rootPackage']=='com.solosoul.app'
     nodes=list(ET.fromstring(response['xml']).iter('node'))
     assert any(n.get('view-id','').endswith('/pdfTitle') and n.get('text')==file_name for n in nodes)
     counters=[n for n in nodes if n.get('view-id','')=='com.solosoul.app:id/pdfPageInfo']
     assert len(counters)==1 and visible(counters[0]) and counters[0].get('text')==f'{index} / {expected_pages}'
     area=box(counters[0]);words=[]
     for line in receipt['Lines']:
      for word in line['Words']:
       x,y,w,h=(word[k] for k in ['X','Y','Width','Height'])
       assert all(isinstance(v,(int,float)) and not isinstance(v,bool) and math.isfinite(v) for v in [x,y,w,h])
       if area[0]<=x+w/2<=area[2] and area[1]<=y+h/2<=area[3]:words.append(word['Text'])
     assert ''.join(words).replace(' ','')==f'{index}/{expected_pages}'
     assert any(n.get('view-id','').endswith('/pdfPageImage') and tuple(proof['physicalRegion'])==box(n) for n in nodes)
     content_checked+=1;pdf_pages_checked+=1
  close_names=[]
  for path in sorted((out/'default-external').glob('close-*.json')):
   value=load(path);name=value['fileName'];close_names.append(name)
   assert name in FILES and path.name=='close-'+name+'.json'
   assert value['previewControlsAbsent'] and value['attachmentsRestored']
   assert value['stableCompletionObservations']>=3 and value['stableCompletionElapsedMs']>=350
   assert value['closedLayers'] and len(value['closedLayers'])==len(set(value['closedLayers']))<=2
   assert sorted(v['layer'] for v in value['actions'])==value['closedLayers']
   assert all(v in ['native-pdf','web-preview'] for v in value['closedLayers'])
   assert re.fullmatch(r'frame-\d{3}\.png',value['nativeScreenshot'])
   frame=out/'independent-observer'/value['nativeScreenshot'];assert sha(frame)==value['imageSha256']
   response=load(frame.with_name(frame.name.replace('frame-','response-').replace('.png','.json')))
   assert response['allowedRoot'] and response['rootPackage']=='com.solosoul.app'
   nodes=list(ET.fromstring(response['xml']).iter('node'))
   assert not preview_controls_present(nodes) and file_set([n for n in nodes if visible(n)])==FILES
   for action in value['actions']:
    receipt=action['receipt'];assert receipt['success'] and receipt['singleFingerPair'] and receipt['toolType']=='finger'
    assert receipt['rootPackage']=='com.solosoul.app' and receipt['allowedRoot']
    actual=[p for p in (out/'independent-observer').glob('response-*.json') if load(p)==receipt]
    assert len(actual)==1
    request=load(actual[0].with_name(actual[0].name.replace('response-','request-')))
    assert request['action']=='tap' and request['expectedPackage']=='com.solosoul.app'
    target=request['target'];assert target['bounds']==receipt['bounds'] and target['package']=='com.solosoul.app'
    ordinal=int(actual[0].stem.split('-')[-1]);previous=[]
    for p in (out/'independent-observer').glob('response-*.json'):
     if int(p.stem.split('-')[-1])<ordinal:
      snapshot=load(p)
      if 'xml' in snapshot:previous.append((int(p.stem.split('-')[-1]),snapshot))
    assert previous
    snapshot=max(previous,key=lambda p:p[0])[1];all_nodes=list(ET.fromstring(snapshot['xml']).iter('node'))
    current=[n for n in all_nodes if visible(n)]
    candidates=(native_pdf_close_candidates if action['layer']=='native-pdf' else web_preview_close_candidates)(current,name)
    assert len(candidates)==1 and all(candidates[0].get(k)==v for k,v in target.items())
  if row['passed']:
   assert set(close_names)==FILES and len(close_names)==4
   assert complete(phases)
   evidence=load(out/'default-external/evidence.json');assert evidence['fullUiJourneyAccepted'] and evidence['success']
   assert not evidence['fullDefaultJourneyAccepted'] and not evidence['performanceDistributionAccepted']
   assert evidence['stageNames']==list(PHASES)
  cases.append(dict(objectCount=row['objectCount'],accepted=row['passed'],completedPhaseNames=[p['name'] for p in phases],
   physicallyVerifiedPublicTextStages=content_checked,physicallyVerifiedPdfPages=pdf_pages_checked,privateEntries=len(before),allStateExactlyRestored=True,
   observerRequests=observer['requests'],failureEvidencePreserved=True))
 text=(P/'terminal-health/dropbox.stdout').read_text(encoding='utf-8').replace('\r\n','\n')
 rows=re.findall(r'^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d) (\S+) \(([^\n]*)\)$',text,re.M)
 count=re.search(r'^Drop box contents: (\d+) entries$',text,re.M)
 assert count and int(count[1])==len(rows)<1000 and not any(r[1].endswith('_anr') for r in rows)
 assert '<no ANR has occurred since boot>' in (P/'terminal-health/last-anr.stdout').read_text()
 result=dict(passed=True,fullUiQueueAccepted=terminal['prototypePassed'],fullDefaultJourneyAccepted=False,
  performanceDistributionAccepted=False,cases=cases,completeTerminalDropBoxEntries=len(rows),noAnrEntries=True,
  allSystemAndPrivateStateExactlyRestored=True,hostCleanupPassed=True,newHostBaseline='no-preexisting-adb-or-vm',
  historicalIdentityRejectionPreserved=True,unitTests=unit['tests'],unitResultSha256=sha(Path(owner['config']['unitEvidence'])),
  hostAuditSha256=sha(S/args.host_audit/'result.json'),terminalResultSha256=sha(S/'external-terminal-r2.json'),
  defaultReleasePdfPagesPhysicallyVerified=sum(c['physicallyVerifiedPdfPages'] for c in cases),
  defaultReleasePdfCounterVisibilityAndPhysicalDigitsVerified=any(c['physicallyVerifiedPdfPages'] for c in cases),auditScriptSha256=sha(Path(__file__)))
 destination=S/'independent-terminal-audit.json';assert not destination.exists()
 destination.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');print(json.dumps(result))
 return result

def main(argv=None):
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--plan',required=True,type=Path);parser.add_argument('--output',required=True,type=Path);parser.add_argument('--host-audit',required=True)
 args=parser.parse_args(argv);run(args.plan,args.output,args);return 0
if __name__=='__main__':sys.exit(main())
