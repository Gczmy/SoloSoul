import json, math, re
from pathlib import Path
from PIL import Image
ROOT = Path(__file__).parent
frames = [
 ('fixed-dark-system-light','2B3A824D-7036-48C6-8711-312850E6D0CC','F9D52BE1-A79D-4060-8044-A379F90459DA','dark','深色 · 森夜'),
 ('fixed-light-system-light','B005E36E-EDB8-4FFB-8E1A-C701F366FDBE','30D95D96-A571-4C9B-AF60-14A7EF214E1B','light','浅色 · 暖石'),
 ('system-mode-system-light','B509BA14-DA19-4C7A-AFA2-A8103FD379F1','6E24DEF5-EABD-4461-B892-E55DD62AB6B4','light','跟随系统'),
 ('system-mode-system-dark','4CDFB106-49AC-4E95-B77E-D9C174A86740','70B97BA4-767B-4A6A-9E56-9B175C808ACA','dark','跟随系统'),
 ('fixed-light-system-dark','721ED640-4052-4C65-A92B-B334FAD07262','70FB3436-2FEF-4600-902B-38324E46DFE8','light','浅色 · 暖石'),
 ('fixed-dark-system-dark','C5DCA01F-0260-46C8-B581-408C39A34B8C','538DBA34-E07A-4265-8E4D-C24B58A1C502','dark','深色 · 森夜'),
 ('system-mode-system-dark-again','A8BD1C7C-EE3C-4DDF-9B86-CAA75D4E2D8F','AD7C0F13-A95B-463A-A5DE-21E03D095DAE','dark','跟随系统'),
]
def luminance(rgb):
 v = [x / 255 for x in rgb]
 v = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in v]
 return sum(c * x for c,x in zip((0.2126,0.7152,0.0722),v))
rows=[]
for stage,png,tree,mode,selected in frames:
 im=Image.open(ROOT/'attachments'/(png+'.png')).convert('RGB')
 assert im.size==(1206,2622), 'Unexpected runtime geometry'
 top,body = im.getpixel((12,30)),im.getpixel((12,600))
 assert top==((253,252,249) if mode=='light' else (36,45,40)), (stage,top)
 assert body==((250,250,246) if mode=='light' else (26,33,29)), (stage,body)
 assert "label: '"+selected+"', value: 1" in (ROOT/'attachments'/(tree+'.txt')).read_text(), stage+' not selected in actual UI'
 # Actual native clock interiors, excluding its surrounding background and antialias edges.
 clock=im.crop((145,75,330,130))
 ink=[p for p in clock.getdata() if (max(p)<=5 if mode=='light' else min(p)>=250)]
 assert len(ink)>=100, (stage,'native clock foreground absent',len(ink))
 fg=tuple(sorted(p[i] for p in ink)[len(ink)//2] for i in range(3))
 a,b=sorted((luminance(fg),luminance(top)))
 contrast=(b+.05)/(a+.05)
 assert contrast>=4.5, (stage,contrast)
 rows.append({'stage':stage,'mode':mode,'selected_control':selected,'top_rgb':top,'body_rgb':body,'clock_foreground_rgb':fg,'clock_solid_foreground_pixels':len(ink),'clock_contrast':contrast})
r={'scope':'Seven actual screenshot frames for current iPhone17/iOS26.3.1 account appearance; native clock foreground and shared palette only, not transitions/performance/other iOS versions','passed':True,'frames':rows,'production_binary_sha256':'5484c123902dede8a2d608207b1a9a99684396aed352eeb1ebbfb74910a03ca8'}
(ROOT/'theme-frame-audit.json').write_text(json.dumps(r,ensure_ascii=False,indent=2)+'\n')
print('7 native theme frames passed; minimum native clock contrast:',min(x['clock_contrast'] for x in rows))
