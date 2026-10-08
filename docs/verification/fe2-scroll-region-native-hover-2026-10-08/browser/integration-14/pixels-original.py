import struct,zlib,json
from pathlib import Path

def read_png(path):
    raw=Path(path).read_bytes(); assert raw[:8]==b'\x89PNG\r\n\x1a\n'
    pos=8; compressed=b''
    while pos<len(raw):
        size=struct.unpack('>I',raw[pos:pos+4])[0]; kind=raw[pos+4:pos+8]; data=raw[pos+8:pos+8+size];pos+=size+12
        if kind==b'IHDR':
            width,height,depth,color,_,_,interlace=struct.unpack('>IIBBBBB',data);assert depth==8 and color in (2,6) and interlace==0
        if kind==b'IDAT':compressed+=data
    channels=3 if color==2 else 4; stride=width*channels; stream=zlib.decompress(compressed); previous=bytearray(stride); rows=[]
    for y in range(min(height,240)):
        offset=y*(stride+1); flag=stream[offset]; row=bytearray(stream[offset+1:offset+1+stride])
        for x in range(stride):
            left=row[x-channels] if x>=channels else 0; up=previous[x]; upper_left=previous[x-channels] if x>=channels else 0
            if flag==1:add=left
            elif flag==2:add=up
            elif flag==3:add=(left+up)//2
            elif flag==4:
                p=left+up-upper_left;a=abs(p-left);b=abs(p-up);c=abs(p-upper_left)
                add=left if a<=b and a<=c else up if b<=c else upper_left
            else:assert flag==0;add=0
            row[x]=(row[x]+add)&255
        rows.append(row);previous=row
    return lambda x,y:list(rows[y][x*channels:x*channels+3])


reports=[]; cache={}
for sample_file in Path('/tmp/solosoul-scroll-region-hover-integration-results').rglob('paint-samples.json'):
 for row in json.loads(sample_file.read_text()):
  path=row['path']
  if path not in cache:cache[path]=read_png(path)
  pixel=cache[path];actual=pixel(row['x'],row['y']);background=pixel(row['x']-8,row['y'])
  expected=[255 if row['theme']=='dark' else 17]*3 if row['active'] else [round(128*.45+b*.55) for b in background]
  ok=all(abs(a-b)<=2 for a,b in zip(actual,expected))
  reports.append({**row,'actual':actual,'background':background,'expected':expected,'passed':ok})
report={'scope':'连续三轮正文/侧栏/工具/顶栏往返，浅深主题，两种浏览器实际像素；不是原生客户端目测','passed':bool(reports) and all(r['passed'] for r in reports),'count':len(reports),'samples':reports}
Path('/tmp/solosoul-scroll-region-hover-integration-pixels.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'passed':report['passed'],'count':len(reports),'failures':[r for r in reports if not r['passed']][:8]},ensure_ascii=False))
assert report['passed']
