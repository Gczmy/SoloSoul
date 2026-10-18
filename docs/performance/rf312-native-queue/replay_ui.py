from pathlib import Path
from collections import Counter
import base64,hashlib,io,json,re,shlex,subprocess,sys,time
from types import SimpleNamespace
from queue_context import load_context
ctx=load_context();R=J=ctx.output;mode=ctx.mode;W=ctx.workspace;P=W/'docs/performance'
sys.path[:0]=[str(P),str(P/'rf312-default-observer'),str(P/'rf312-ipc-runtime')]
import rf312_default_binding as binding
import diagnostic_exchange as exchange
from persistent_control import read_reply
from native_journey_contract import verify_text_frame
from scaled_ocr import remap_half_receipt
from PIL import Image
read=lambda p:json.loads(Path(p).read_bytes())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert read(J/'independent-current-state-r1.json')['passed']
loaded=ctx.loaded;package=loaded['ownedUi'];runtime=loaded['runtime']

def text_proofs(value):
    if isinstance(value,dict):
        if value.get('nativeImageOcrBindingVerified')is True:yield value
        else:
            for child in value.values():yield from text_proofs(child)
    elif isinstance(value,list):
        for child in value:yield from text_proofs(child)

cases=[]
for count in (100,5000):
    p=ctx.native/f'journey-{count}';evidence=read(p/'default-external/supported-state-evidence.json')
    images={sha(file):file for file in (p/'independent-observer').glob('*.png')}
    for image in images.values():
        with Image.open(image)as bitmap:bitmap.load();assert bitmap.size==(1080,1920)
    for phase in evidence['stages']:assert sha(p/'independent-observer'/phase['nativeScreenshot'])==phase['imageSha256']
    content=0
    for proof in text_proofs(evidence['stages']):
        image=images[proof['imageSha256']];receipt=p/'journey-native-ocr'/(image.stem+'.json');raw=read(receipt)
        if 'supplementalOcr'in proof:
            raw,transform=remap_half_receipt(image,receipt,receipt.with_name(image.stem+'-half.png'),receipt.with_name(image.stem+'-half.json'))
            assert transform==proof['supplementalOcr']
        actual=verify_text_frame(image,raw,tuple(proof['knownTokens']),tuple(proof['physicalRegion']))
        assert actual=={key:value for key,value in proof.items()if key!='supplementalOcr'};content+=1
    assert content==6
    exchanges={};reads={};operations=Counter();frames={};initial_absent_pid=0
    for name,allowed in [('persistent-control',{'ping','get-stable','exchange','close'}),
            ('auxiliary-control/persistent-control',{'ping','focus','pid','app-stop','app-start','close'})]:
        folder=p/name;cleanup=read(folder/'cleanup.json');live=read(folder/'live.json')
        assert cleanup['accepted']and cleanup['normalControlClose']and cleanup['ownedRemoteScratchRemoved']and cleanup['originalCallbackRestored']
        if name=='persistent-control':
            assert cleanup['javaFileTransport']and cleanup['stableReadinessRoundTripVerified']
            assert live['scriptSha256']==package['artifactSha256']and (folder/'device-readback.jar').read_bytes()==Path(package['artifact']).read_bytes()
            command=live['command'];args=shlex.split(command[-1])[0].split(' ');token,scratch,uid=args[-3:]
            assert int(uid)==live['observerUid']>=10000 and scratch=='files/'+token+'/owned-ui'
            assert args[:4]==['CLASSPATH='+scratch+'/control.jar','app_process','/system/bin','org.solosoul.rf312transport.OwnedUi']
            assert command[:8]==[runtime['host']['adbExecutable'],'-H','127.0.0.1','-P','5057','-s',runtime['host']['serial'],'shell']
            assert command[-5:-1]==['run-as','org.solosoul.rf312observer','sh','-c']
        else:assert live['scriptSha256']==sha(P/'rf312-default-observer/persistent_control.sh')
        files=sorted((folder/'rpc').glob('*.json'));assert len(files)==cleanup['rpcFrames']
        for ordinal,file in enumerate(files,1):
            row=read(file);request=file.with_suffix('.request').read_bytes();wire=file.with_suffix('.reply').read_bytes()
            assert row['sequence']==ordinal and not row.get('timedOut')and 'errorType'not in row
            assert sha(file.with_suffix('.request'))==row['requestSha256']and sha(file.with_suffix('.reply'))==row['replySha256']and len(wire)==row['replyBytes']
            fields=request.rstrip(b'\n').split(b' ');assert len(fields)==5 and fields[0]==b'RF312B'
            op,label=fields[2].decode(),fields[3].decode();assert op in allowed
            payload=base64.b64decode(fields[4],validate=True)if op=='exchange'else b''
            assert exchange.request_line(ordinal,op,label,payload)==request
            stream=io.BytesIO(wire);partial=bytearray();code,out,err=read_reply(stream,ordinal,partial)
            assert not stream.read()and bytes(partial)==wire and code==row['exitCode']and err==b''
            assert row['stdoutSha256']==hashlib.sha256(out).hexdigest()and row['stderrSha256']==hashlib.sha256(err).hexdigest()
            if code==1:
                assert op=='pid'and operations['pid']==0 and out==b'';initial_absent_pid+=1
            else:assert code==0
            if op=='exchange':
                assert label not in exchanges and out==(p/'independent-observer'/label.replace('request-','response-')).read_bytes()
                assert json.loads(payload)==read(p/'independent-observer'/label);exchanges[label]=(payload,out,row['timeoutSeconds'])
            elif op=='get-stable':
                assert label not in reads and out==(b'ready'if label=='ready'else(p/'independent-observer'/label).read_bytes());reads[label]=out
            elif op=='ping':assert out==b'rf312-control-ready\n'
            elif op=='close':assert out==b''
            operations[op]+=1
        frames[name]=len(files)
    total=read(p/'independent-observer/cleanup.json')['requests'];assert 0<total<=512
    assert list(exchanges)==[f'request-{number:03d}.json'for number in range(1,total+1)]and initial_absent_pid==1
    replay_output=J/f'ui-source-validator-replay-r1-{count}';replay_output.mkdir(exist_ok=False)
    remote='files/'+'a'*32+'/'
    def fake_adb(operation,path,**kwargs):
        assert path.startswith(remote);label=path[len(remote):]
        if operation=='rf312-exchange':
            payload,out,_=exchanges[label];assert kwargs['input']==payload
        else:
            assert operation=='rf312-get-stable';out=reads[label]
        return subprocess.CompletedProcess([operation,path],0,out,b'')
    client=exchange.Client(fake_adb,replay_output,SimpleNamespace(poll=lambda:None),remote)
    client.deadline=time.monotonic()+240;client.composite_bound=True
    for label,(payload,out,timeout)in exchanges.items():
        answer=client.request(json.loads(payload),min(45,timeout));assert answer==json.loads(out)
    assert client.ordinal==total
    cases.append(dict(objectCount=count,uiStages=12,nativePngs=len(images),physicalTextProofs=content,
        uiControlFrames=frames['persistent-control'],auxiliaryControlFrames=frames['auxiliary-control/persistent-control'],observerRequests=total,
        originalSourceClientResponsesReplayed=total,uiInputsDispatchedByAudit=0,originalRpcAndPayloadBytesPreserved=True,
        observerActionCounts=dict(Counter(json.loads(payload)['action']for payload,_,_ in exchanges.values())),controlOperations=dict(operations),javaArtifactSha256=package['artifactSha256']))
result=dict(passed=True,scope='current-default-ui-physical-content-and-both-owned-controls',mode=mode,cases=cases,
    sourceSha256=binding.sources(),sourceClientValidatorsReplayed=True,allBothChannelCleanupsAccepted=True,
    sdkCommandsDispatchedByAudit=0,uiInputsDispatchedByAudit=0,fullRF312Accepted=False)
with(J/'independent-ui-r1.json').open('x',encoding='utf-8')as stream:json.dump(result,stream,indent=2)
print(json.dumps({key:value for key,value in result.items()if key!='sourceSha256'}))
