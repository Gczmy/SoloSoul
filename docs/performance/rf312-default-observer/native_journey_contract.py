from pathlib import Path
import hashlib,math,re,struct
from PIL import Image

PHASES=('startup','password-unlock','first-ocr-user-flow','workspace','search-needle','attachments-open',
        'image-preview','text-preview','pdf-text-native','pdf-scanned-native','application-lock','password-reunlock')
FILES={'ocr_test.png','text_only.pdf','scanned.pdf','preview.txt'}
class PublicContentNotReady(ValueError):pass
def labels(nodes):return [n.get(k,'') for n in nodes for k in ('text','content-desc') if n.get(k)]
def box(node):
 m=re.fullmatch(r'\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]',node.get('bounds',''))
 if not m:raise ValueError('Native bounds required')
 return tuple(map(int,m.groups()))
def visible(node):
 try:x,y,r,b=box(node)
 except ValueError:return False
 return r>x and b>y and node.get('visible-to-user')=='true' and node.get('enabled')=='true'
def record_ids(nodes):
 return {int(v) for text in labels(nodes) for v in re.findall(r'\bRecord (\d{8})\b',text)}
def expected_search_ids(count):
 if type(count) is not int or count not in (100,5000):raise ValueError('Public fixed count required')
 return set(range(0,min(count,1000),20))
def public_filename(value):
 if not isinstance(value,str) or not re.fullmatch(r'rf312-ocr-[a-f0-9]{32}\.png',value):
  raise ValueError('Only driver-owned public OCR filename allowed')
 return value
def public_text_present(nodes):
 words=' '.join(labels(nodes)).casefold()
 return all(re.search(r'\b'+re.escape(w)+r'\b',words) for w in ('hello','solosoul','1234567890'))
def file_set(nodes):return {name for name in FILES if any(name in value for value in labels(nodes))}
def complete(phases):
 if [p.get('name') for p in phases]!=list(PHASES):raise ValueError('Exact complete twelve-phase journey required')
 for phase in phases:
  if (phase.get('nativeScreenshotVerified') is not True or not re.fullmatch(r'[a-f0-9]{64}',phase.get('imageSha256',''))
      or type(phase.get('hostObservedUpperBoundMs')) not in (int,float) or not math.isfinite(phase['hostObservedUpperBoundMs'])
      or not 0<phase['hostObservedUpperBoundMs']<=45000):raise ValueError('Every phase needs real bounded native evidence')
 return True
def verify_text_frame(image,receipt,tokens,region=None):
 image=Path(image);data=image.read_bytes()
 if image.is_symlink() or getattr(image.stat(),'st_file_attributes',0)&0x400:raise ValueError('Ordinary PNG required')
 if not data.startswith(b'\x89PNG\r\n\x1a\n') or len(data)<24:raise ValueError('Native PNG required')
 width,height=struct.unpack('>II',data[16:24]);digest=hashlib.sha256(data).hexdigest()
 if (receipt.get('Engine')!='Windows.Media.Ocr' or receipt.get('Language')!='en-GB'
     or receipt.get('ImageSha256')!=digest or receipt.get('PixelWidth')!=width or receipt.get('PixelHeight')!=height):
  raise ValueError('Native image/OCR binding required')
 if region is None:region=(0,0,width,height)
 if (not isinstance(region,tuple) or len(region)!=4 or any(type(x) is not int for x in region)
     or not 0<=region[0]<region[2]<=width or not 0<=region[1]<region[3]<=height):raise ValueError('Physical evidence region required')
 lines=receipt.get('Lines');words=[]
 if not isinstance(lines,list) or not 1<=len(lines)<=256:raise ValueError('Bounded OCR lines required')
 for line in lines:
  members=line.get('Words')
  if not isinstance(members,list) or not 1<=len(members)<=32:raise ValueError('Bounded OCR words required')
  for word in members:
   if not isinstance(word.get('Text'),str) or not 0<len(word['Text'])<=128:raise ValueError('Bounded OCR word required')
   bounds=[word.get(k) for k in ('X','Y','Width','Height')]
   if any(type(x) not in (int,float) or not math.isfinite(x) for x in bounds):raise ValueError('Finite physical OCR bounds required')
   x,y,w,h=bounds
   if not (0<=x<x+w<=width and 0<=y<y+h<=height):raise ValueError('OCR word outside image')
   if region[0]<=x and region[1]<=y and x+w<=region[2] and y+h<=region[3]:
    words.extend(re.findall(r'[a-z0-9]+',word['Text'].casefold()))
 wanted=tuple(tokens)
 if not wanted or not all(isinstance(t,str) and re.fullmatch('[a-z0-9]+',t) for t in wanted):raise ValueError('Exact public token set required')
 if '1234567890' in wanted and any(re.fullmatch(r'\d{10}',w) and w!='1234567890' for w in words):
  raise ValueError('Wrong complete public digit sequence')
 if not set(wanted).issubset(words):raise PublicContentNotReady('Known public content not physically readable in its region')
 with Image.open(image) as bitmap:
  if bitmap.size!=(width,height):raise ValueError('PNG decode dimensions disagree')
  # This public fixture uses the frozen light theme. Blank/solid regions cannot pass.
  pixels=bitmap.convert('RGB').crop(region).getdata()
  dark=sum(max(rgb)<100 for rgb in pixels);light=sum(min(rgb)>235 for rgb in pixels)
 if dark<100 or light<1000:raise PublicContentNotReady('Public text region must contain foreground and background pixels')
 return dict(imageSha256=digest,knownTokens=list(wanted),physicalRegion=list(region),darkPixels=dark,lightPixels=light,
  nativeImageOcrBindingVerified=True,exactDigitsWithoutSubstitution=True)
