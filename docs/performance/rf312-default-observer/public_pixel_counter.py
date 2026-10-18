from pathlib import Path
import hashlib,json,math,re,struct

class PixelCountNotReady(ValueError):
    pass

def verified_counter(image,receipt,expected):
    if type(expected) is not int or expected not in (100,5000):raise ValueError('Only frozen public fixture counts')
    image=Path(image);data=image.read_bytes()
    if image.is_symlink() or image.stat().st_file_attributes&0x400:raise ValueError('Ordinary native PNG required')
    if not data.startswith(b'\x89PNG\r\n\x1a\n') or len(data)<24:raise ValueError('Native PNG required')
    width,height=struct.unpack('>II',data[16:24]);digest=hashlib.sha256(data).hexdigest()
    if (receipt.get('Engine')!='Windows.Media.Ocr' or receipt.get('Language')!='en-GB'
        or receipt.get('ImageSha256')!=digest or receipt.get('PixelWidth')!=width or receipt.get('PixelHeight')!=height):
        raise ValueError('Native image/recognizer binding mismatch')
    lines=receipt.get('Lines')
    if not isinstance(lines,list) or not 1<=len(lines)<=256:raise ValueError('Bounded OCR lines required')
    def rect(word):
        values=[word.get(k) for k in ('X','Y','Width','Height')]
        if any(type(x) not in (int,float) or not math.isfinite(x) for x in values):raise ValueError('Finite physical bounds required')
        x,y,w,h=values
        if not (0<=x<width and 0<=y<height and w>0 and h>0 and x+w<=width and y+h<=height):raise ValueError('Word outside physical image')
        return x,y,w,h
    def union(words):
        boxes=[rect(w) for w in words];left=min(r[0] for r in boxes);top=min(r[1] for r in boxes)
        return left,top,max(r[0]+r[2] for r in boxes)-left,max(r[1]+r[3] for r in boxes)-top
    for line in lines:
        if not isinstance(line.get('Text'),str) or len(line['Text'])>512:raise ValueError('Bounded OCR line text required')
        if not isinstance(line.get('Words'),list) or not 1<=len(line['Words'])<=32:raise ValueError('Bounded words required')
        for word in line['Words']:
            if not isinstance(word.get('Text'),str) or not 0<len(word['Text'])<=128:raise ValueError('Bounded OCR word required')
            rect(word)
    title=[line for line in lines if re.sub(r'\s+',' ',line['Text'].strip()).casefold()=='my vault']
    footer=[line for line in lines if re.sub(r'\s+',' ',line['Text'].strip()).casefold().startswith('local vault ')]
    if len(title)!=1 or len(footer)!=1:raise PixelCountNotReady('Unique vault title and footer required')
    top=union(title[0]['Words']);bottom=union(footer[0]['Words'])
    if not (top[0]<width*.25 and abs(bottom[0]-top[0])<width*.05 and top[1]+top[3]<bottom[1]):raise ValueError('Vault anchors do not agree')
    words=[word for line in lines for word in line['Words'] if word['Y']>=top[1]+top[3] and word['Y']+word['Height']<=bottom[1]]
    numbers=[w for w in words if re.fullmatch(r'[0-9]+',w['Text'])]
    if not numbers:raise PixelCountNotReady('Physical full vault counter not yet readable')
    if len(numbers)!=1 or numbers[0]['Text']!=str(expected):raise ValueError('Exact single full vault count required')
    count=numbers[0];x,y,w,h=rect(count)
    if not (h>=2*top[3] and abs(x-top[0])<width*.05):raise ValueError('Prominent vault counter required')
    units=[item for item in words if item['Text']=='objects' and item['X']>x+w
           and item['X']-(x+w)<width*.2 and abs(item['Y']+item['Height']-(y+h))<=h*.35]
    if len(units)!=1:raise ValueError('Single aligned object unit required')
    return dict(count=expected,imageSha256=digest,engine='Windows.Media.Ocr',language='en-GB',
                counterBounds=dict(x=x,y=y,width=w,height=h),titleAndFooterVerified=True,
                exactDigitsWithoutSubstitution=True,alignedObjectUnitVerified=True)
