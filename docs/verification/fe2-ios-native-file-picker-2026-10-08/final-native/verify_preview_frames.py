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
for mode in ['light','dark']:
 for kind in ['text','photo-viewer']:
  stage=mode+'-public-'+('text-preview' if kind=='text' else 'photo-viewer')
  im=Image.open(files[stage]).convert('RGB');tree=files[stage+'-hierarchy'].read_text();assert im.size==(1206,2622)
  name='FE2-public-note.txt' if kind=='text' else 'FE2-public-photo.png'
  title=[r for r in bounds(tree,'StaticText',name) if 60<=r[1]<130];assert len(title)==1
  row={'stage':stage,'title':ink_check(im,title[0])}
  if kind=='text':
   body=bounds(tree,'StaticText','SoloSoul FE2 public preview fixture')[0]
   assert 'Readable text in light and dark themes.' in tree and 'No real account information.' in tree
   row['body']=ink_check(im,body)
   back=[r for r in bounds(tree,'Button','返回') if r[0]==14 and r[1]==72];assert len(back)==1
  else:
   img=max(bounds(tree,'Image',name),key=lambda r:r[2]*r[3]);palette=collections.Counter(crop(im,img).getdata())
   colors=[(91,124,153),(196,146,92)];counts=[sum(n for c,n in palette.items() if max(abs(c[i]-expected[i]) for i in range(3))<=2) for expected in colors]
   assert min(counts)>=2000,(stage,counts);row['public_image_color_pixels']=dict(zip(['blue','amber'],counts));row['image_rect_points']=img
   back=bounds(tree,'Button','返回照片集');assert len(back)==1
   controls={}
   for label in ['缩小','放大','适应窗口']:
    b=bounds(tree,'Button',label);assert len(b)==1 and b[0][2]>=44 and b[0][3]>=44 and b[0][1]+b[0][3]<=840;controls[label]=b[0]
   row['zoom_controls']=controls
  assert back[0][1]>=60 and back[0][2]>=44 and back[0][3]>=44;row['actual_preview_back_rect_points']=back[0]
  bg=im.getpixel((12,30));p=[c for c in im.crop((145,75,330,130)).getdata() if min(c)>=250];assert len(p)>=100
  assert contrast((255,255,255),bg)>=4.5;row['clock_background_rgb']=bg;row['clock_foreground_pixels']=len(p);row['clock_contrast']=contrast((255,255,255),bg)
  rows.append(row)
r={'passed':True,'scope':'Four captured current-package iPhone17/iOS26.3.1 TXT/viewer frames: actual public content pixels, title/text/native clock contrast and toolbar geometry; not transitions or all overlays','production_binary_sha256':'2e4006a7227f771f442a98c259f84ff6c7ceff7044b21066975365e88d041d7d','frames':rows}
(ROOT/'preview-frame-audit.json').write_text(json.dumps(r,ensure_ascii=False,indent=2)+'\n')
print('4 native preview frames verified; minimum title contrast',min(x['title']['contrast'] for x in rows),'minimum clock contrast',min(x['clock_contrast'] for x in rows))
