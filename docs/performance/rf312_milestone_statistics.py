"""Statistics for independently verified document milestones, not native verification."""
import argparse,hashlib,json,math
from pathlib import Path

METRICS=('reactMountMs','startupDismissedMs','mountInvocationToHandoffMs','firstPaintMs','firstContentfulPaintMs')
REASONS={'native-rejected','health-rejected','recovery-rejected','source-drift','metadata-unavailable','queue-incomplete'}
FALSE=('nativeVerificationPerformed','nativeFramesAccepted','firstInteractiveTimeProven','applicationOnlyLatencyAccepted',
       'observerOverheadRemoved','defaultPerformanceSampleAccepted','fullRF312Accepted')

def require(value,message):
    if not value:raise ValueError(message)

def fields(value,names):require(type(value) is dict and set(value)==set(names),'Exact cohort fields required')

def digest(value):
    require(type(value) is str and len(value)==64 and all(c in '0123456789abcdef' for c in value),'Exact source/evidence digest required')
    return value

def finite(value):
    require(type(value) in [int,float] and math.isfinite(value) and value>=0,'Finite nonnegative document milliseconds required')
    return value

def decode(raw):
    def pairs(rows):
        result={}
        for name,value in rows:
            require(name not in result,'Duplicate JSON key rejected');result[name]=value
        return result
    def constant(value):raise ValueError('Nonfinite JSON rejected')
    return json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,parse_constant=constant)

def summarize(value,protocol,protocol_sha256):
    fields(protocol,{'schemaVersion','scope','plannedQueues','objectCounts','clock','order','failurePolicy','quantile',
        'samplerSourcesSha256','statisticsSourceSha256','targetApkSha256','collectorApkSha256','exploratoryPairIncluded'})
    require(type(protocol['schemaVersion']) is int and protocol['schemaVersion']==1
        and protocol['scope']=='rf312-document-startup-five-queue-protocol','Exact fixed protocol required')
    require(type(protocol['plannedQueues']) is int and protocol['plannedQueues']==5 and protocol['objectCounts']==[100,5000]
        and protocol['clock']=='document-relative-ms' and protocol['order']=='ascending-slots-then-100-5000'
        and protocol['failurePolicy']=='stop-and-retain-no-replacement' and protocol['quantile']=='nearest-rank-p95'
        and protocol['exploratoryPairIncluded'] is False,'Original prospectively fixed sampling policy required')
    for name in ['samplerSourcesSha256','statisticsSourceSha256','targetApkSha256','collectorApkSha256']:digest(protocol[name])
    fields(value,{'schemaVersion','scope','protocolSha256','queues'})
    require(type(value['schemaVersion']) is int and value['schemaVersion']==1
        and value['scope']=='rf312-document-startup-milestone-observations'
        and digest(value['protocolSha256'])==digest(protocol_sha256),'Exact protocol-bound observations required')
    queues=value['queues'];require(type(queues) is list and len(queues)==5,'All five slots, including failures and unstarted, required')
    values={count:{metric:[] for metric in METRICS} for count in [100,5000]}
    statuses=[];halted=False
    for slot,queue in enumerate(queues,1):
        fields(queue,{'slot','status','reason','evidenceSha256','samplerSourcesSha256','samples'})
        require(type(queue['slot']) is int and queue['slot']==slot,'Original unique ordered slots required')
        status=queue['status'];require(type(status) is str and status in ['accepted','refused','not-started'],'Exact queue outcome required')
        require(not halted or status=='not-started','Cannot replace or continue after a stopped slot')
        samples=queue['samples'];require(type(samples) is list,'Explicit sample list required')
        if status!='accepted':
            require(samples==[],'Failed queue prefixes cannot enter distributions')
            if status=='refused':
                require(type(queue['reason']) is str and queue['reason'] in REASONS,'Fixed refusal reason required')
                digest(queue['evidenceSha256']);digest(queue['samplerSourcesSha256'])
            else:
                require(queue['reason'] is None and queue['evidenceSha256'] is None and queue['samplerSourcesSha256'] is None,
                    'Unstarted slots cannot contain invented evidence')
            halted=True
        else:
            require(queue['reason'] is None and digest(queue['samplerSourcesSha256'])==protocol['samplerSourcesSha256'],
                'All admitted queues require identical sampler source bytes')
            digest(queue['evidenceSha256']);require(len(samples)==2,'Both verified sizes required for an admitted queue')
            for sample,count in zip(samples,[100,5000]):
                fields(sample,{'objectCount','metadataSha256','metrics'})
                require(type(sample['objectCount']) is int and sample['objectCount']==count,'Original 100 then 5000 order required')
                digest(sample['metadataSha256']);metrics=sample['metrics'];fields(metrics,METRICS)
                for name in METRICS:
                    part=metrics[name]
                    if part is None:require(name in ['firstPaintMs','firstContentfulPaintMs'],'Required native-buffer marks cannot be missing')
                    else:values[count][name].append(finite(part))
                require(metrics['startupDismissedMs']>=metrics['reactMountMs'] and math.isclose(
                    metrics['startupDismissedMs']-metrics['reactMountMs'],metrics['mountInvocationToHandoffMs'],rel_tol=0,abs_tol=1e-9),
                    'Handoff interval must use the same document clock')
                if metrics['firstPaintMs'] is not None and metrics['firstContentfulPaintMs'] is not None:
                    require(metrics['firstContentfulPaintMs']>=metrics['firstPaintMs'],'Paint order differs')
        statuses.append({name:queue[name] for name in ['slot','status','reason','evidenceSha256']})
    counts={name:sum(row['status']==name for row in statuses) for name in ['accepted','refused','not-started']}
    result={}
    for count,groups in values.items():
        result[str(count)]={}
        for name,items in groups.items():
            ordered=sorted(items);n=len(items);median=p95=None
            if n>=2:
                median=ordered[n//2] if n%2 else ordered[n//2-1]+(ordered[n//2]-ordered[n//2-1])/2
                p95=ordered[math.ceil(.95*n)-1]
            result[str(count)][name]=dict(availableN=n,missingN=counts['accepted']-n,plannedN=5,
                medianMs=median,p95Ms=p95,distributionComplete=n==5,tailIsMaximumWhenNIsFive=n==5)
    return dict(schemaVersion=1,scope='document-milestone-statistics-only',protocolSha256=protocol_sha256,
        plannedQueues=5,acceptedQueues=counts['accepted'],refusedQueues=counts['refused'],notStartedQueues=counts['not-started'],
        queueStatuses=statuses,statisticsComplete=counts['accepted']==5,metrics=result,**{name:False for name in FALSE})

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--protocol',type=Path,required=True);parser.add_argument('--input',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    protocol_bytes=args.protocol.read_bytes();protocol=decode(protocol_bytes)
    require(protocol.get('statisticsSourceSha256')==hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'Predeclared statistics source changed')
    raw=args.input.read_bytes();result=summarize(decode(raw),protocol,hashlib.sha256(protocol_bytes).hexdigest())
    result['observationsSha256']=hashlib.sha256(raw).hexdigest()
    with args.output.open('x',encoding='utf-8',newline='\n') as f:json.dump(result,f,indent=2,allow_nan=False);f.write('\n')
    print(json.dumps(dict(statisticsComplete=result['statisticsComplete'],acceptedQueues=result['acceptedQueues'],
        refusedQueues=result['refusedQueues'],notStartedQueues=result['notStartedQueues'],nativeFramesAccepted=False)))

if __name__=='__main__':main()
