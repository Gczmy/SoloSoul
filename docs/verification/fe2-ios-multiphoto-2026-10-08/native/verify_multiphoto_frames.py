import collections,json,re
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).parent
manifest=json.loads((ROOT/'attachments/manifest.json').read_text())[0]['attachments']
files={a['suggestedHumanReadableName'].split('_0_')[0]:ROOT/'attachments'/a['exportedFileName'] for a in manifest if Path(a['exportedFileName']).suffix in ('.png','.txt')}
frame=re.compile(r'\{\{([\d.-]+), ([\d.-]+)\}, \{([\d.-]+), ([\d.-]+)\}\}')
def bounds(tree,kind,label):
 rows=[]
 for line in tree.splitlines():
  if re.search(r'\b'+kind+r',',line) and "label: '"+label in line:
   m=frame.search(line)
   if m:rows.append(tuple(map(float,m.groups())))
 assert rows,(kind,label)
 return rows

def luminance(rgb):
 v=[x/255 for x in rgb];v=[x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4 for x in v]
 return sum(x*y for x,y in zip(v,[.2126,.7152,.0722]))
def contrast(a,b):
 a,b=sorted([luminance(a),luminance(b)]);return (b+.05)/(a+.05)
def crop(im,rect):
 x,y,w,h=rect;return im.crop((round(x*3),round(y*3),round((x+w)*3),round((y+h)*3)))
def ink_check(im,rect):
 x,y,w,h=rect;bg=im.getpixel((round((x-2)*3),round((y-2)*3)))
 palette=collections.Counter(crop(im,rect).getdata())
 ink=[(c,n) for c,n in palette.most_common() if max(abs(c[i]-bg[i]) for i in range(3))>35]
 assert ink,'No rendered foreground';fg,n=ink[0]
 assert n>=100 and contrast(fg,bg)>=4.5,(fg,bg,n,contrast(fg,bg))
 return {'background_rgb':bg,'dominant_foreground_rgb':fg,'foreground_pixels':n,'contrast':contrast(fg,bg)}

rows=[]
by_stage={}
for mode in ['light','dark']:
 stages=[('first-photo-fit','FE2-public-photo.png'),('native-swipe-second-photo','FE2-public-second.png'),('previous-button-first-photo','FE2-public-photo.png'),('next-button-second-photo','FE2-public-second.png'),('second-photo-zoomed','FE2-public-second.png'),('second-photo-fit-restored','FE2-public-second.png')]
 for suffix,name in stages:
  stage=mode+'-'+suffix
  im=Image.open(files[stage]).convert('RGB');tree=files[stage+'-hierarchy'].read_text();assert im.size==(1206,2622)
  title=[r for r in bounds(tree,'StaticText',name) if 60<=r[1]<130];assert len(title)==1
  img=max(bounds(tree,'Image',name),key=lambda r:r[2]*r[3]);palette=collections.Counter(crop(im,img).getdata())
  colors=[(91,124,153),(196,146,92)] if name=='FE2-public-photo.png' else [(115,91,163),(105,167,136)]
  counts=[sum(n for c,n in palette.items() if max(abs(c[i]-expected[i]) for i in range(3))<=2) for expected in colors]
  assert min(counts)>=2000,(stage,counts)
  controls={}
  for label in ['上一个','下一个','缩小','放大','适应窗口','返回照片集']:
   b=bounds(tree,'Button',label);assert len(b)==1 and b[0][2]>=44 and b[0][3]>=44 and b[0][1]>=60 and b[0][1]+b[0][3]<=840,(stage,label,b);controls[label]=b[0]
  counters=[label for label in re.findall(r"label: '([^']*)'",tree) if re.fullmatch('[12] / 2',label)];assert len(counters)==1
  clockBg=im.getpixel((12,30));clockPixels=sum(min(c)>=250 for c in im.crop((145,75,330,130)).getdata());assert clockPixels>=100 and contrast((255,255,255),clockBg)>=4.5
  row={'stage':stage,'filename':name,'title':ink_check(im,title[0]),'actual_public_color_pixels':counts,'image_rect_points':img,'counter':counters[0],'controls':controls,'native_clock_contrast':contrast((255,255,255),clockBg),'native_clock_foreground_pixels':clockPixels};rows.append(row);by_stage[stage]=row
 first=by_stage[mode+'-first-photo-fit'];swipe=by_stage[mode+'-native-swipe-second-photo'];previous=by_stage[mode+'-previous-button-first-photo'];second=by_stage[mode+'-next-button-second-photo'];zoom=by_stage[mode+'-second-photo-zoomed'];reset=by_stage[mode+'-second-photo-fit-restored']
 assert first['counter']!=swipe['counter'] and first['counter']==previous['counter']
 assert swipe['counter']==second['counter']==zoom['counter']==reset['counter']
 assert zoom['image_rect_points'][2]>second['image_rect_points'][2]*1.1
 assert abs(reset['image_rect_points'][2]-second['image_rect_points'][2])<2
result={'passed':True,'scope':'Twelve stable current-production iPhone17/iOS26.3.1 light/dark multiphoto frames; actual public image/title pixels, counter transitions, button geometry, image zoom dimensions and native clock contrast. Does not establish animation frame rate or physical device performance.','production_binary_sha256':'2e4006a7227f771f442a98c259f84ff6c7ceff7044b21066975365e88d041d7d','frames':rows}
(ROOT/'multiphoto-frame-audit.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print('Twelve stable native frames passed; title minimum',min(r['title']['contrast'] for r in rows),'clock minimum',min(r['native_clock_contrast'] for r in rows))
