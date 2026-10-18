from contextlib import contextmanager
from pathlib import Path
import hashlib,json,os,re,shlex,subprocess,time,uuid
import xml.etree.ElementTree as ET

S=Path(__file__).parent
PKG='org.solosoul.rf312observer'
APP='com.solosoul.app'
ALLOWED={APP,'com.android.documentsui'}
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
def save(p,v):
    with p.open('x',encoding='utf-8') as f:json.dump(v,f,indent=2)

class Client:
    def __init__(self,adb,output,process,remote):
        self.adb=adb;self.output=output;self.process=process;self.remote=remote;self.ordinal=0
        self.deadline=time.monotonic()+240
    def call(self,*args,deadline,**kwargs):
        remaining=min(deadline,self.deadline)-time.monotonic()
        if remaining<=0:raise TimeoutError('Original full/phase observation budget exhausted')
        return self.adb(*args,timeout=remaining,stdout=subprocess.PIPE,stderr=subprocess.PIPE,**kwargs)
    def exists(self,name,deadline):
        if not re.fullmatch(r'(?:ready|error|stopped|response-\d{3}\.json)',name):raise ValueError('Invalid protocol name')
        try:self.call('shell','run-as',PKG,'test','-e',self.remote+name,deadline=deadline);return True
        except subprocess.CalledProcessError as e:
            if e.returncode==1 and e.stdout==e.stderr==b'':return False
            raise
    def get(self,name,deadline):
        if not re.fullmatch(r'(?:ready|error|stopped|response-\d{3}\.json|frame-\d{3}\.png)',name):raise ValueError('Invalid protocol name')
        size=self.call('shell','run-as',PKG,'stat','-c','%s',self.remote+name,deadline=deadline).stdout.strip()
        if not re.fullmatch(rb'[1-9]\d*',size) or int(size)>4*1024*1024:raise RuntimeError('Invalid bounded response size')
        data=self.call('exec-out','run-as',PKG,'cat',self.remote+name,deadline=deadline).stdout
        if len(data)!=int(size):raise RuntimeError('Response size changed')
        return data
    def alive(self,deadline):
        if self.exists('error',deadline):raise RuntimeError('Independent observer: '+self.get('error',deadline).decode())
        if self.process.poll() is not None:raise RuntimeError('Independent observer exited early')
    def request(self,value,timeout):
        if not 0<timeout<=45:raise ValueError('Per-phase observer budget must be within 45 seconds')
        self.ordinal+=1
        if self.ordinal>512:raise RuntimeError('Bounded observer request count exhausted')
        name=f'{self.ordinal:03d}';deadline=min(self.deadline,time.monotonic()+timeout)
        payload=json.dumps(value,ensure_ascii=True,separators=(',',':')).encode()
        if len(payload)>16384:raise ValueError('Bounded public request too large')
        save(self.output/('request-'+name+'.json'),value)
        temporary=self.remote+'request-'+name+'.tmp';final=self.remote+'request-'+name+'.json'
        command='run-as '+PKG+' sh -c '+shlex.quote('umask 077; cat > '+temporary+' && mv '+temporary+' '+final)
        self.alive(deadline)
        self.call('shell',command,input=payload,deadline=deadline)
        response='response-'+name+'.json'
        while not self.exists(response,deadline):
            self.alive(deadline);time.sleep(min(.025,max(0,deadline-time.monotonic())))
        raw=self.get(response,deadline);result=json.loads(raw)
        if result.get('success') is not True or result.get('action')!=value['action']:
            raise RuntimeError('Independent observer action mismatch')
        if type(result.get('nativeStartedAtNs')) is not int or type(result.get('nativeFinishedAtNs')) is not int or result['nativeFinishedAtNs']<result['nativeStartedAtNs']:
            raise RuntimeError('Invalid native observation clock')
        if type(result.get('allowedRoot')) is not bool or result['allowedRoot']!=(result.get('rootPackage') in ALLOWED):
            raise RuntimeError('Invalid root ownership receipt')
        if value['action']=='snapshot':
            xml=result.get('xml')
            if not isinstance(xml,str) or '<!DOCTYPE' in xml.upper():raise RuntimeError('Invalid hierarchy receipt')
            tree=ET.fromstring(xml)
            if tree.tag!='hierarchy' or len(list(tree.iter('node')))>5000:raise RuntimeError('Invalid bounded hierarchy')
            if not result['allowedRoot'] and list(tree):raise RuntimeError('Unrelated hierarchy contents refused')
        else:
            if not result['allowedRoot'] or result.get('rootPackage')!=value['expectedPackage']:
                raise RuntimeError('Input receipt owner differs')
            if value['action']=='inspect-target':
                count=result.get('matchCount');matched=result.get('matched')
                if (type(count) is not int or not 0<=count<=5000 or type(matched) is not bool
                        or matched!=(count==1) or result.get('inputDispatched') is not False or result.get('intent')!=value['intent']):
                    raise RuntimeError('Read-only exact target inspection receipt required')
                if any(type(result.get(k)) is not bool for k in ('targetEnabled','targetVisible','targetFocused','targetFocusable','targetClickable')):
                    raise RuntimeError('Native target readiness flags required')
                boxes=result.get('candidateBounds');compatible=result.get('compatibleCount')
                if (not isinstance(boxes,list) or len(boxes)>16 or type(compatible) is not int or not len(boxes)<=compatible<=5000
                        or any(not isinstance(b,str) or not re.fullmatch(r'\[\d+,\d+\]\[\d+,\d+\]',b) for b in boxes)):
                    raise RuntimeError('Bounded native candidate geometry required')
                ime=result.get('imeBounds');ime_count=result.get('imeWindowCount')
                if (not isinstance(ime,list) or type(ime_count) is not int or not 0<=ime_count==len(ime)<=16
                        or type(result.get('imeVisible')) is not bool or result['imeVisible']!=(ime_count>0)
                        or any(not isinstance(b,str) or not re.fullmatch(r'\[\d+,\d+\]\[\d+,\d+\]',b) for b in ime)):
                    raise RuntimeError('Bounded native IME geometry required')
                if value['intent']=='pick-public-file' and type(result.get('targetPickerFileTile')) is not bool:
                    raise RuntimeError('Native public picker tile probe required')
            if value['action']=='tap' and (result.get('singleFingerPair') is not True or result.get('toolType')!='finger'
                    or result.get('bounds')!=value['target']['bounds']):
                raise RuntimeError('Single exact native finger receipt required')
            if value['action']=='type-public' and (result.get('publicTextOnly') is not True
                    or type(result.get('keyEvents')) is not int or result['keyEvents']<=0 or result['keyEvents']%2):
                raise RuntimeError('Public native keyboard receipt required')
            if value['action']=='scroll' and (result.get('singleFingerGesture') is not True
                    or result.get('toolType')!='finger' or type(result.get('events')) is not int or result['events']!=14
                    or result.get('bounds')!=value['target']['bounds'] or result.get('direction')!=value['direction']):
                raise RuntimeError('Single bounded native finger scroll receipt required')
            if value['action']=='pick-public-file' and (result.get('publicPickerFileOnly') is not True
                    or result.get('fileName')!=value['fileName'] or result.get('previewControlExcluded') is not True
                    or result.get('singleFingerPair') is not True or result.get('toolType')!='finger'
                    or result.get('bounds')!=value['target']['bounds']):
                raise RuntimeError('Exact public picker file finger receipt required')
        if time.monotonic()>deadline:raise TimeoutError('Observer result arrived after phase deadline')
        (self.output/response).write_bytes(raw)
        if 'screenshot' in result:
            frame='frame-'+name+'.png'
            if result['screenshot']!=frame or result.get('rootPackage') not in ALLOWED:raise RuntimeError('Invalid screenshot ownership')
            image=self.get(frame,deadline)
            if not image.startswith(b'\x89PNG\r\n\x1a\n'):raise RuntimeError('Native screenshot is not PNG')
            (self.output/frame).write_bytes(image)
        return result
    def snapshot(self,timeout,*,screenshot=False):
        return self.request(dict(action='snapshot',screenshot=screenshot),timeout)
    @staticmethod
    def target(node):
        fields=('package','class','text','content-desc','password','bounds')
        row={k:node.attrib[k] for k in fields}
        if row['package'] not in ALLOWED or node.get('enabled')!='true' or node.get('visible-to-user')!='true':
            raise RuntimeError('Target outside allowed visible hierarchy')
        return row
    def inspect(self,node,intent,timeout):
        if intent not in ('tap','type-public','scroll','pick-public-file'):raise ValueError('Bound native input intent required')
        request=dict(action='inspect-target',expectedPackage=node.get('package'),target=self.target(node),intent=intent)
        if intent=='pick-public-file':request['fileName']=self.picker_name(node)
        return self.request(request,timeout)
    @staticmethod
    def target_ready(node,probe,intent):
        if not (probe['matched'] and probe['targetEnabled'] and probe['targetVisible']):return False
        if intent=='scroll':return probe.get('targetScrollable') is True and node.get('scrollable')=='true'
        if intent=='pick-public-file':return probe.get('targetPickerFileTile') is True and node.get('focusable')=='true'
        if intent=='type-public':
            return node.get('class')=='android.widget.EditText' and node.get('text','')=='' and probe['targetFocused'] and probe['targetFocusable'] and probe['imeVisible'] and probe['imeWindowCount']==1
        return probe['targetClickable'] or node.get('class')=='android.widget.EditText' and probe['targetFocusable']
    def tap(self,node,timeout):
        return self.request(dict(action='tap',expectedPackage=node.get('package'),target=self.target(node)),timeout)
    def type_public(self,node,value,timeout):
        if value not in ('perf-baseline-only-password','needle'):raise ValueError('Only public fixture text allowed')
        if node.get('package')!=APP:raise RuntimeError('Public text requires application owner')
        return self.request(dict(action='type-public',expectedPackage=APP,target=self.target(node),text=value),timeout)
    def back(self,package,timeout):
        if package not in ALLOWED:raise ValueError('Unowned back target')
        return self.request(dict(action='back',expectedPackage=package),timeout)
    def scroll(self,node,direction,timeout):
        if direction not in ('forward','backward') or node.get('scrollable')!='true':
            raise ValueError('Visible native scrollable target and bounded direction required')
        return self.request(dict(action='scroll',expectedPackage=node.get('package'),target=self.target(node),direction=direction),timeout)
    @staticmethod
    def picker_name(node):
        match=re.match(r'^(rf312-ocr-[a-f0-9]{32}\.png), ',node.get('content-desc',''))
        if (not match or node.get('package')!='com.android.documentsui' or node.get('class')!='android.widget.LinearLayout'
                or node.get('focusable')!='true' or node.get('password')!='false' or node.get('text','')!=''):
            raise ValueError('Only owned delegated public picker file tile allowed')
        return match[1]
    def pick_file(self,node,filename,timeout):
        if self.picker_name(node)!=filename:raise ValueError('Public picker filename mismatch')
        return self.request(dict(action='pick-public-file',expectedPackage='com.android.documentsui',target=self.target(node),fileName=filename),timeout)

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
    cleanup=dict(accepted=False,targetInstrumentationInjected=False,javaScriptInjected=False,originalObserverAbsent=True)
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
