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
    for y in range(height):
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

reports=[]
for engine in ['chromium','webkit']:
    for state,horizontal in [('all-axes',17),('vertical-only',198),('direct-thumb',17)]:
        path=f'/tmp/solosoul-scroll-{engine}-{state}.png'
        pixel=read_png(path);vertical_rgb=pixel(997,100);horizontal_rgb=pixel(180,697)
        ok=all(abs(x-17)<=1 for x in vertical_rgb) and all(abs(x-horizontal)<=1 for x in horizontal_rgb)
        reports.append({'engine':engine,'state':state,'vertical_pixel':vertical_rgb,'horizontal_pixel':horizontal_rgb,'expected_vertical':17,'expected_horizontal':horizontal,'passed':ok})
report={'scope':'实际浏览器截图滑块像素；合成几何场景，不代表原生客户端窗口恢复验收','passed':all(r['passed'] for r in reports),'samples':reports}
Path('/tmp/solosoul-scroll-region-pixels.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False))
assert report['passed']
