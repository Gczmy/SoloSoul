from pathlib import Path
import json,base64,sys
from PIL import Image
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).parent
report=json.loads((root/'results.json').read_text());rows=[]
for suite in report['suites']:
 if suite.get('file')!='zoom.spec.ts':continue
 for spec in suite.get('specs',[]):
  for test in spec['tests']:
   engine=test['projectName'];theme=spec['title'].split()[-1]
   metadata=json.loads(base64.b64decode(test['results'][0]['attachments'][0]['body']))
   folder=root/'results'/f'zoom-photo-toolbar-drawing-through-zoom-{theme}-{engine}'
   images={m['step']:Image.open(folder/(m['step']+'.png')).convert('RGB') for m in metadata}
   b=metadata[0]['bounds'];x,y,w,h=[round(b[k]) for k in ['x','y','width','height']]
   icon_regions=[(x+15,y+9,x+43,y+37),(x+115,y+9,x+143,y+37),(x+153,y+9,x+181,y+37)]
   icon_counts={s:[sum(1 for p in im.crop(rect).get_flattened_data() if min(p)>150) for rect in icon_regions] for s,im in images.items()}
   backgrounds={s:im.getpixel((x+w//2,y+6)) for s,im in images.items()}
   changed={s:sum(1 for yy in range(y,y+h) for xx in range(x,x+w) if not x+50<=xx<x+110 and im.getpixel((xx,yy))!=images['initial'].getpixel((xx,yy))) for s,im in images.items()}
   quadrants=[(230,55,47),(30,177,95),(40,109,225),(237,184,39)]
   photo_counts={}
   for step,im in images.items():
    counts=[0]*4
    for pixel in im.get_flattened_data():
     for i,color in enumerate(quadrants):
      if all(abs(pixel[j]-color[j])<=3 for j in range(3)):counts[i]+=1
    photo_counts[step]=counts
   passed=(all(min(c)>20 for c in icon_counts.values()) and all(n==0 for n in changed.values()) and len(set(backgrounds.values()))==1 and all(min(c)>500 for c in photo_counts.values()))
   rows.append({'engine':engine,'theme':theme,'public_image_size':[320,240],'image_size':images['initial'].size,'bounds':b,'metadata':metadata,'svg_bright_pixel_counts':icon_counts,'background_rgb':backgrounds,'changed_pixels_outside_percentage':changed,'quadrant_color_pixel_counts':photo_counts,'passed':passed})
record={'passed':len(rows)==4 and all(r['passed'] for r in rows),'scope':'320x240 public fixture; headless Playwright browser screenshots only, not WKWebView/AppKit or physical display proof','rows':rows}
(root/'pixel-analysis.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'passed':record['passed'],'groups':len(rows),'group_results':[{'engine':r['engine'],'theme':r['theme'],'passed':r['passed'],'svg_counts':r['svg_bright_pixel_counts'],'quadrant_counts':r['quadrant_color_pixel_counts']} for r in rows]}))
assert record['passed']
