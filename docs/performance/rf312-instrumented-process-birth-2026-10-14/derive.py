from pathlib import Path
import hashlib,json,math,statistics

R=Path('D:/SoloSoul');B=Path(__file__).parent;OLD=B/'rf312-fixed-current-memory-20261014'
S=B/'rf312-instrumented-process-birth-20261014';S.mkdir(exist_ok=False)
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
raw=load(OLD/'kernel-timebase-read-elevated.json');assert raw[0]['exitCode']==raw[1]['exitCode']==0
hz=int(raw[0]['stdout']);assert hz==100
plan=load(OLD/'cohort-plan.json');assert load(OLD/'cohort-terminal.json')['all10NativeJourneysPassed']
rows=[];seen=set()
for spec in plan['samples']:
    root=Path(spec['root']);native=load(root/'journey/device-files/report.json')
    snapshots=[load(p) for p in sorted((root/'memory-raw').glob('*/result.json'))]
    identities={(p['pid'],p['uid'],p['startTicks']) for z in snapshots if z.get('fullProcessScopeAccepted') for p in z['processes'] if p['role']=='app' and p['pid']==native['processId']}
    assert len(identities)==1;pid,uid,tick=next(iter(identities));assert uid==10105 and (pid,tick) not in seen;seen.add((pid,tick))
    lower=tick*1000/hz;upper=(tick+1)*1000/hz
    began=native['phases'][0]['nativeStartedAtNs']/1e6;ready=native['phases'][0]['nativeFinishedAtNs']/1e6
    assert 0<lower<upper<=began<=ready
    absent=[z['ordinal'] for z in snapshots if z.get('state')=='not-running' and z['deviceClockAfter']['uptimeMs']<lower]
    assert absent,'No original raw absent-app evidence before process birth'
    rows.append(dict(objectCount=spec['objectCount'],ordinal=spec['ordinal'],pid=pid,uid=uid,startTicks=tick,
        processBirthLowerMs=lower,processBirthUpperMsExclusive=upper,activityActionMs=began,loginAcceptedAtMs=ready,
        instrumentedProcessBirthToReadyUpperMs=ready-lower,instrumentedProcessBirthToReadyLowerMs=ready-upper,
        beforeBirthAbsentSnapshotOrdinals=absent,oneBirthIdentityForWholeRun=True))
groups=[]
for count in [100,5000]:
    values=[r['instrumentedProcessBirthToReadyUpperMs'] for r in rows if r['objectCount']==count];assert len(values)==5
    groups.append(dict(objectCount=count,n=5,samples=values,medianUpperMs=statistics.median(values),p95NearestRankUpperMs=sorted(values)[math.ceil(.95*len(values))-1]))
result=dict(task='RF-312',sourceHead=plan['head'],apkSha256=plan['currentMainApk']['sha256'],clockTicksPerSecond=hz,
    birthQuantizationMs=1000/hz,kernelVersion=raw[1]['stdout'].strip(),samples=rows,groups=groups,
    scope='instrumented app process kernel birth to original usable password UI acceptance, including runner initialization, SDK, observer and query work',
    capturedRawBeforeBirthAppAbsentForEverySample=True,kernelClockAndNativeClockBothBoottime=True,
    systemColdOrStorageColdClaimed=False,defaultUninstrumentedProcessStartupClaimed=False,
    originalActivityMetricNotRenamed=True,newDeviceOrApkRunClaimed=False,wholeRF312Accepted=False,
    references=[dict(url='https://github.com/aosp-mirror/kernel_common/blob/f9a66cbe7091/fs/proc/array.c',blobSha='18a4588c35be632449258c2fc5d0e6f4ccd5effd',formula='start_time = nsec_to_clock_t(task->start_boottime);'),
    dict(url='https://github.com/aosp-mirror/platform_system_core/blob/main/libutils/SystemClock.cpp',blobSha='df3a8986520e3b9094fe2ba7189d0c1ff7049793',formula='clock_gettime(CLOCK_BOOTTIME, &ts)'),
    dict(url='https://developer.android.com/reference/android/os/SystemClock',definition='elapsedRealtimeNanos counts time since boot including sleep')],
    sourceEvidenceArchiveSha256=load(R/'docs/performance/rf312-android-current-memory-2026-10-14/index.json')['archiveSha256'])
(S/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
(S/'kernel-timebase-read-elevated.json').write_bytes((OLD/'kernel-timebase-read-elevated.json').read_bytes())
print(json.dumps(dict(scope=result['scope'],groups=groups,defaultUninstrumentedProcessStartupClaimed=False)),flush=True)
