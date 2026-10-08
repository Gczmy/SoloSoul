import json, math, re
from pathlib import Path
from PIL import Image
ROOT = Path(__file__).parent
frames = [('fixed-dark-system-light', 'A9C104ED-1D99-4769-9F1F-15FADDE5FED2', '17017270-38A9-4B67-84E9-6663F5E2702B', 'dark', '深色 · 森夜'), ('fixed-light-system-light', '5D92AD56-2342-4D17-9B7D-31FCBD2C2CD8', '1AAD2D76-3619-47B9-AD47-125077ECB546', 'light', '浅色 · 暖石'), ('system-mode-system-light', 'F77FBCC0-8C68-4126-A939-3186FC8CA2A2', 'D49ABE7B-2AC8-472B-8824-8C5A0ABFBEEF', 'light', '跟随系统'), ('system-mode-system-dark', 'BD4FA8DC-8FBB-41C2-8238-F720BA241F75', '2C17CA08-ACF5-444D-9F1E-82E042D1DE0F', 'dark', '跟随系统'), ('fixed-light-system-dark', '185808C1-71BC-4869-A53A-E08EB4E0067E', 'EF50F769-02A4-4DAC-9E93-BC200F68ED28', 'light', '浅色 · 暖石'), ('fixed-dark-system-dark', '5D4BF4AF-B9FE-4411-92AD-09E3854D4DF1', '19BEF6B7-A674-4832-8BC2-39491B3B4733', 'dark', '深色 · 森夜'), ('system-mode-system-dark-again', '5D415CFB-026E-4590-ACA8-B937A4322BE5', '0508447B-449F-4E49-84AA-B8DA3E0191A8', 'dark', '跟随系统')]
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
r={'scope':'Seven actual screenshot frames for current iPhone17/iOS26.3.1 account appearance; native clock foreground and shared palette only, not transitions/performance/other iOS versions','passed':True,'frames':rows,'production_binary_sha256':'98280597fff53310efb49a8849556b5820d0a218a5262a429ae5c4d85f7f277f'}
(ROOT/'theme-frame-audit.json').write_text(json.dumps(r,ensure_ascii=False,indent=2)+'\n')
print('7 native theme frames passed; minimum native clock contrast:',min(x['clock_contrast'] for x in rows))
