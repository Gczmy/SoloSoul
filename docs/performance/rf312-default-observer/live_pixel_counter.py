from pathlib import Path
import hashlib,json,os,re,subprocess,time
import xml.etree.ElementTree as ET
from public_pixel_counter import PixelCountNotReady,verified_counter

S=Path(__file__).parent
APP='com.solosoul.app'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
def save_new(p,value):
    with p.open('x',encoding='utf-8') as stream:json.dump(value,stream,indent=2)

class NativeOcrReader:
    def __init__(self,bindings,evidence_root,*,clock=time.monotonic,runner=subprocess.run):
        self.bindings=bindings;self.clock=clock;self.runner=runner
        self.root=Path(evidence_root).absolute()
        if not self.root.is_dir():raise ValueError('Existing explicit evidence root required')
        self.ordinary(self.root)
        self.root=self.root.resolve(strict=True)
    @staticmethod
    def ordinary(path):
        for item in (path,*path.parents):
            if item.is_symlink() or getattr(item.lstat(),'st_file_attributes',0)&0x400:
                raise ValueError('Ordinary evidence and parents required')
    def owned(self,path,*,existing):
        path=Path(path).absolute()
        if '..' in path.parts or not path.is_relative_to(self.root):
            raise ValueError('Path outside declared evidence root')
        if existing:
            if not path.is_file():raise ValueError('Existing evidence file required')
            self.ordinary(path)
        else:
            if os.path.lexists(path):raise RuntimeError('Fresh native OCR output required')
            self.ordinary(path.parent)
        return path
    def __call__(self,image,receipt,deadline):
        remaining=deadline-self.clock()
        if not 0<remaining<=45:raise TimeoutError('Original pixel phase budget exhausted')
        image=self.owned(image,existing=True);receipt=self.owned(receipt,existing=False)
        assert all(sha(S/name)==digest for name,digest in self.bindings.items()),'Frozen pixel sources changed'
        windows=Path(os.environ.get('SystemRoot',''))
        if not windows.is_absolute():raise ValueError('Explicit Windows SystemRoot required')
        modules=windows/'System32/WindowsPowerShell/v1.0'
        command=[str(modules/'powershell.exe'),'-NoProfile','-NonInteractive',
                 '-File',str(S/'read-public-counter.ps1'),'-ImagePath',str(image),'-OutputPath',str(receipt),
                 '-EvidenceRoot',str(self.root)]
        env=dict(os.environ,TEMP=str(receipt.parent),TMP=str(receipt.parent),PYTHONDONTWRITEBYTECODE='1')
        env['PSModulePath']=str(modules/'Modules')
        remaining=deadline-self.clock()
        if not 0<remaining<=45:raise TimeoutError('Original pixel phase budget exhausted before native call')
        result=self.runner(command,cwd=S,env=env,stdin=subprocess.DEVNULL,capture_output=True,
                           timeout=min(10,remaining),creationflags=subprocess.CREATE_NO_WINDOW)
        receipt.with_suffix('.stdout.log').write_bytes(result.stdout)
        receipt.with_suffix('.stderr.log').write_bytes(result.stderr)
        result.check_returncode()
        if self.clock()>=deadline:raise TimeoutError('Native OCR returned after original phase deadline')
        assert all(sha(S/name)==digest for name,digest in self.bindings.items()),'Pixel sources changed during read'
        return load(receipt)

def visible(node):
    match=re.fullmatch(r'\[(\d+),(\d+)\]\[(\d+),(\d+)\]',node.get('bounds',''))
    return bool(match and int(match[3])>int(match[1]) and int(match[4])>int(match[2])
                and node.get('visible-to-user')=='true' and node.get('enabled')=='true')

def home_context(nodes):
    password=lambda n:n.get('class')=='android.widget.EditText' and n.get('password')=='true'
    words=[n.get(k,'') for n in nodes for k in ('text','content-desc') if n.get(k)]
    category=any(n.get('clickable')=='true' and re.fullmatch(r'Identity (?:0|[1-9]\d*) objects',n.get(k,''))
                 for n in nodes for k in ('text','content-desc'))
    return not any(password(n) for n in nodes) and 'Welcome back, Performance Fixture' in words and category

def wait_for_counter(observer,output,count,deadline,focused,ocr,*,clock=time.monotonic,namespace='pixel-counter'):
    if type(count) is not int or count not in (100,5000):raise ValueError('Frozen public count required')
    if namespace not in ('pixel-counter','reunlock-pixel-counter'):raise ValueError('Bound counter namespace required')
    folder=output/namespace;folder.mkdir(exist_ok=False)
    native_folder=(output/'independent-observer').resolve()
    observations=[]
    def remaining():
        value=deadline-clock()
        if not 0<value<=45:raise TimeoutError('Original 45-second pixel phase bound exhausted')
        return value
    while remaining()>0:
        focused();budget=remaining()
        row=observer.snapshot(budget,screenshot=True)
        if row.get('allowedRoot') is not True or row.get('rootPackage')!=APP:
            raise RuntimeError('Only owned application frames may be counted')
        name=row.get('screenshot','')
        if not re.fullmatch(r'frame-\d{3}\.png',name):raise RuntimeError('Bound native frame name required')
        image=output/'independent-observer'/name
        if image.resolve().parent!=native_folder or not image.is_file() or image.is_symlink():
            raise RuntimeError('Native frame outside owned namespace')
        if type(row.get('nativeStartedAtNs')) is not int or type(row.get('nativeFinishedAtNs')) is not int or row['nativeFinishedAtNs']<row['nativeStartedAtNs']:
            raise RuntimeError('Native frame clock required')
        nodes=[n for n in ET.fromstring(row['xml']).iter('node') if n.get('package')==APP and visible(n)]
        observed=dict(frame=name,imageSha256=sha(image),homeContext=home_context(nodes),ready=False)
        if observed['homeContext']:
            receipt=folder/('ocr-'+name.removesuffix('.png')+'.json')
            result=ocr(image,receipt,deadline)
            remaining()
            try:
                proof=verified_counter(image,result,count)
            except PixelCountNotReady as error:
                observed['notReadyReason']=str(error)
            else:
                focused();remaining()
                observed['ready']=True;observed['count']=proof['count']
                observations.append(observed)
                proof.update(nativeScreenshot=name,nativeStartedAtNs=row['nativeStartedAtNs'],
                    nativeFinishedAtNs=row['nativeFinishedAtNs'],ocrReceipt=receipt.name,
                    ocrReceiptSha256=sha(receipt),homeContextVerified=True,
                    accessibilityCounterUsed=False,originalPhaseBoundKept=True)
                save_new(folder/'accepted-counter.json',proof)
                save_new(folder/'observations.json',observations)
                return row,nodes,proof
        observations.append(observed)
        (folder/'observations-progress.json').write_text(json.dumps(observations,indent=2),encoding='utf-8')
        if len(observations)>512:raise RuntimeError('Original observation request bound exhausted')
