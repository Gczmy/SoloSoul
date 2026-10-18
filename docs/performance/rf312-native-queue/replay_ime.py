from pathlib import Path
import hashlib,json,sys,xml.etree.ElementTree as ET
from queue_context import load_context
ctx=load_context();I=ctx.output;W=ctx.workspace
sys.path[:0]=[str(W/'docs/performance'),str(W/'docs/performance/rf312-default-observer')]
from native_journey_contract import box
read=lambda p:json.loads(p.read_bytes())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
parent=read(ctx.native/'result.json');assert parent['passed'] and parent['restored']
cases=[]
for count in [100,5000]:
    folder=ctx.native/f'journey-{count}/independent-observer';tree=None;history=[];typed=[];refused=0
    for file in sorted(folder.glob('request-*.json')):
        req=read(file);res=read(file.with_name(file.name.replace('request-','response-')))
        if req['action']=='snapshot':tree=ET.fromstring(res['xml'])
        elif req['action']=='inspect-target' and req['intent']=='type-public':
            nodes=[n for n in tree.iter('node')if all(n.get(k)==v for k,v in req['target'].items())]
            parents={c:p for p in tree.iter()for c in p};ancestors=[];cur=nodes[0]if len(nodes)==1 else None
            while cur is not None:
                if cur.get('class')=='android.webkit.WebView':ancestors.append(cur)
                cur=parents.get(cur)
            ready=False
            if len(nodes)==1 and len(ancestors)==2 and res['matched'] and all(res[k]for k in ['targetEnabled','targetVisible','targetFocused','targetFocusable']) and res['imeWindowCount']==1:
                dl,dt,dr,db=box(ancestors[0]);hl,ht,hr,hb=box(ancestors[1]);il,it,ir,ib=box(ET.Element('node',bounds=res['imeBounds'][0]));tl,tt,tr,tb=box(nodes[0])
                dw,dh,hw,hh=dr-dl,db-dt,hr-hl,hb-ht
                ready=min(dw,dh,hw,hh,ir-il,ib-it)>0 and il<=hl<hr<=ir and abs(hb-it)<=1 and hl<=tl<tr<=hr and ht<=tt<tb<=hb and abs(dw*hh-dh*hw)<=hw+hh and abs(dl*hw-hl*dw)<=hw+abs(hl) and abs(dt*hw-ht*dw)<=hw+abs(ht)
                ready=ready and all(a.get('package')=='com.solosoul.app' and a.get('visible-to-user')=='true' and a.get('enabled')=='true'for a in ancestors)
            if not ready:history=[];refused+=1;continue
            signature=(req['target'],tuple(res['imeBounds']),tuple(a.get('bounds')for a in ancestors))
            if history and history[-1]['signature']!=signature:history=[]
            history.append(dict(signature=signature,nativeFinishedAtNs=res['nativeFinishedAtNs'],requestOrdinal=int(file.stem.split('-')[-1])))
        elif req['action']=='type-public':
            assert len(history)>=3 and history[-1]['signature'][0]==req['target']
            assert history[-1]['nativeFinishedAtNs']-history[0]['nativeFinishedAtNs']>=350000000
            assert res['publicTextOnly'] and res['keyEvents']>0 and res['keyEvents']%2==0
            typed.append(dict(requestOrdinal=int(file.stem.split('-')[-1]),stableProbeOrdinals=[v['requestOrdinal']for v in history],geometryReplayed=True,nativeTypingReceiptSha256=sha(file.with_name(file.name.replace('request-','response-')))))
            history=[]
        elif req['action']!='snapshot':history=[]
    assert len(typed)==3
    cases.append(dict(objectCount=count,typingActions=3,staleOrUnreadyProbesRetained=refused,actions=typed))
result=dict(passed=True,cases=cases,allSixNativeTypingActionsReplayed=True,originalFailedQueueChanged=False,inputRetried=False,sdkInputEventsDispatchedByAudit=0)
with(I/'independent-ime-readiness-r1.json').open('x',encoding='utf-8')as f:json.dump(result,f,indent=2)
print(json.dumps(result))
