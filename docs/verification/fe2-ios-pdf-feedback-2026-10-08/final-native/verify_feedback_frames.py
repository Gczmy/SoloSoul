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
 stage=mode+'-pdf-external-open-error-feedback';im=Image.open(files[stage]).convert('RGB');tree=files[stage+'-hierarchy'].read_text();assert im.size==(1206,2622)
 error=bounds(tree,'StaticText','无法打开文件');assert len(error)==1
 assert 'FE2-public-document.pdf' in tree and error[0][1]>=62 and error[0][1]+error[0][3]<=840
 ink=ink_check(im,error[0]);name=bounds(tree,'StaticText','FE2-public-document.pdf');assert len(name)==1
 upload=bounds(tree,'Button','上传');assert len(upload)==1 and upload[0][2]>=44 and upload[0][3]>=44
 rows.append({'stage':stage,'error_text_bounds_points':error[0],'error_text_actual_pixels':ink,'attachment_name_bounds_points':name[0],'upload_bounds_points':upload[0]})
result={'passed':True,'scope':'Two stable iPhone17/iOS26.3.1 PDF external-opener failure feedback frames, actual readable text and retained attachment UI; this does not establish PDF rendering or opening support','production_binary_sha256':'2e4006a7227f771f442a98c259f84ff6c7ceff7044b21066975365e88d041d7d','frames':rows}
(ROOT/'feedback-frame-audit.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print('Two actual error feedback frames passed; minimum text contrast',min(r['error_text_actual_pixels']['contrast'] for r in rows))
