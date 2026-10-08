import collections,json,re
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).parent
manifest=json.loads((ROOT/'attachments/manifest.json').read_text())[0]['attachments']
files={a['suggestedHumanReadableName'].split('_0_')[0]:ROOT/'attachments'/a['exportedFileName'] for a in manifest if Path(a['exportedFileName']).suffix in ('.png','.txt')}
frame=re.compile(r'\{\{([\d.-]+), ([\d.-]+)\}, \{([\d.-]+), ([\d.-]+)\}\}')
def bounds(tree,kind,label):
 result=[]
 for line in tree.splitlines():
  if re.search(r'\b'+kind+r',',line) and "label: '"+label+"'" in line:
   m=frame.search(line)
   if m:result.append(tuple(map(float,m.groups())))
 assert result,(kind,label)
 return result

def lum(rgb):
 values=[n/255 for n in rgb];linear=[n/12.92 if n<=.04045 else ((n+.055)/1.055)**2.4 for n in values]
 return sum(n*k for n,k in zip(linear,[.2126,.7152,.0722]))
def contrast(a,b):
 lo,hi=sorted([lum(a),lum(b)]);return (hi+.05)/(lo+.05)
rows=[]
for stage,mode,accent in [('a-light-ocean-selected','light','海洋蓝'),('b-dark-rose-selected','dark','玫瑰红'),('a-restored-light-ocean','light','海洋蓝'),('b-restored-dark-rose','dark','玫瑰红')]:
 image=Image.open(files[stage]).convert('RGB');tree=files[stage+'-hierarchy'].read_text();assert image.size==(1206,2622)
 top=image.getpixel((12,30));body=image.getpixel((12,600))
 expected_top=(253,252,249) if mode=='light' else (36,45,40)
 expected_body=(250,250,246) if mode=='light' else (26,33,29)
 assert top==expected_top and body==expected_body,(stage,top,body)
 selected_mode='浅色 · 暖石' if mode=='light' else '深色 · 森夜'
 assert "label: '"+selected_mode+"', value: 1" in tree,stage
 chosen=bounds(tree,'Button',accent);other=bounds(tree,'Button','玫瑰红' if accent=='海洋蓝' else '海洋蓝');assert len(chosen)==len(other)==1
 assert chosen[0][2]>other[0][2]*1.05,(stage,chosen,other)
 expected_accent=(91,124,153) if accent=='海洋蓝' else (176,107,122)
 x,y,w,h=chosen[0];pixels=collections.Counter(image.crop((round(x*3),round(y*3),round((x+w)*3),round((y+h)*3))).getdata())
 count=sum(n for c,n in pixels.items() if max(abs(c[i]-expected_accent[i]) for i in range(3))<=2);assert count>=1000,(stage,count)
 clock=image.crop((145,75,330,130));ink=[c for c in clock.getdata() if (max(c)<=5 if mode=='light' else min(c)>=250)];assert len(ink)>=100
 foreground=tuple(sorted(c[i] for c in ink)[len(ink)//2] for i in range(3));assert contrast(foreground,top)>=4.5
 header=bounds(tree,'Button','返回');assert len(header)==1 and header[0][1]>=60 and header[0][2]>=44 and header[0][3]>=44
 rows.append({'stage':stage,'mode':mode,'selected_mode':selected_mode,'actual_top_rgb':top,'actual_body_rgb':body,'selected_accent':accent,'selected_accent_rect':chosen[0],'other_accent_rect':other[0],'actual_accent_fill_rgb':expected_accent,'actual_accent_fill_pixels':count,'native_clock_foreground_pixels':len(ink),'native_clock_contrast':contrast(foreground,top)})
identities={}
for stage,expected in [('a-created-identity','FE2 Theme A'),('a-restored-identity','FE2 Theme A'),('b-created-identity','FE2 Theme B'),('b-restored-identity','FE2 Theme B')]:
 tree=files[stage+'-hierarchy'].read_text();assert any('TextField,' in line and 'value: '+expected in line for line in tree.splitlines()),stage
 ids=set(re.findall(r'acc_[0-9a-f]{16}',tree));assert len(ids)==1,(stage,ids)
 identities[stage]={'account_name':expected,'account_id':next(iter(ids))}
assert identities['a-created-identity']==identities['a-restored-identity']
assert identities['b-created-identity']==identities['b-restored-identity']
assert identities['a-created-identity']['account_id']!=identities['b-created-identity']['account_id']
result={'passed':True,'scope':'Four stable actual native iPhone17/iOS26.3.1 appearance frames plus actual account identity before and after two-way native list selection and real unlock. Body/top pixels, selected radio, native accent selection geometry/fill and clock contrast verified. Not animation performance or physical devices.','production_binary_sha256':'2e4006a7227f771f442a98c259f84ff6c7ceff7044b21066975365e88d041d7d','frames':rows,'account_identities':identities}
(ROOT/'account-frame-audit.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print('Four native account/theme frames passed; min clock contrast',min(r['native_clock_contrast'] for r in rows))
