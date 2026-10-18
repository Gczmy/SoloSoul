from pathlib import Path
import argparse,hashlib,json,re

parser=argparse.ArgumentParser()
parser.add_argument('--native',type=Path,required=True)
parser.add_argument('--viewed-png-sha256',required=True)
from queue_context import require_assertions
require_assertions()
args=parser.parse_args();p=args.native.resolve(strict=True)
sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
assert re.fullmatch('[0-9a-f]{64}',args.viewed_png_sha256)
assert args.viewed_png_sha256==sha(p/'preflight.png')
files={label:next(p.glob('*preflight-'+label+'.stdout')) for label in ('window','anr','dropbox')}
texts={key:file.read_text(encoding='utf-8') for key,file in files.items()}
assert '<no ANR has occurred since boot>' in texts['anr']
rows=re.findall(r'^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d (\S+) \(',texts['dropbox'],re.M)
declared=re.search(r'^Drop box contents: (\d+) entries$',texts['dropbox'],re.M)
assert declared and len(rows)==int(declared[1]) and 0<len(rows)<1000
assert not any(tag.endswith('_anr') or tag.endswith('_crash') for tag in rows)
focus=re.search(r'^\s*mCurrentFocus=(.+)$',texts['window'],re.M)
focused=re.search(r'^\s*mFocusedApp=(.+)$',texts['window'],re.M)
assert focus and focused and 'com.android.launcher3/' in focus[1] and 'com.android.launcher3/' in focused[1]
ready=json.loads((p/'preflight-ready.json').read_bytes())
assert ready['nativeTestsStarted']==0 and ready['pngSha256']==sha(p/'preflight.png')
assert not (p/'result.json').exists()
value=dict(pngSha256=sha(p/'preflight.png'),screenInspected=True,healthReviewed=True,
    nativeTestsStarted=0,preflightImageActuallyViewed=True,launcherVisible=True,
    healthEntries=len(rows),noAnr=True,noCrashTags=True,
    healthSha256={key:sha(file) for key,file in files.items()},
    fixedObjectOrder=[100,5000],originalBudgetsPreserved=True)
# Validate the exact outer-owner consumer fields before publishing atomically
# visible admission. This does not stand in for actually viewing the PNG.
assert value['pngSha256']==ready['pngSha256'] and value['screenInspected'] and value['healthReviewed']
probe=p/'metadata-pair-probe/probe.json'
assert json.loads(probe.read_bytes())['passed']
value['metadataPairProbeSha256']=sha(probe)
with (p/'continue-native.json').open('x',encoding='utf-8') as out:out.write(json.dumps(value,indent=2)+'\n')
print(json.dumps(value))
