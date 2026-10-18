"""Validate bounded main-thread forensics; never accepts app performance/IPC."""
import re

FIELDS={'schemaVersion','scope','runId','sequence','process','traceStartedNs','nativeReadStartedNs',
        'nativeReadFinishedNs','mainThreadId','threadState','truncated','frames','performanceSampleAccepted'}
FRAME_FIELDS={'className','methodName','fileName','lineNumber','nativeMethod'}
PROCESS_FIELDS={'processId','uid','startTicks'}
STATES={'NEW','RUNNABLE','BLOCKED','WAITING','TIMED_WAITING','TERMINATED'}


def require(value,message):
    if not value:raise ValueError(message)


def text(value,maximum):
    return isinstance(value,str) and 0<len(value)<=maximum and all(ord(char)>=32 for char in value)


def validate(records,*,run_id,expected_process):
    require(isinstance(run_id,str) and re.fullmatch('[a-f0-9]{32}',run_id) is not None,'One fixed trace run identity required')
    require(isinstance(expected_process,dict) and set(expected_process)==PROCESS_FIELDS
            and all(type(value) is int and value>0 for value in expected_process.values())
            and expected_process['uid']>=10000,'Bound app process PID, UID and birth required')
    require(isinstance(records,list) and 1<=len(records)<=80,'Bounded nonempty committed trace prefix required')
    trace_start=thread_id=prior_end=None;states=set();frames=0
    for index,row in enumerate(records,1):
        require(isinstance(row,dict) and set(row)==FIELDS,'Only declared forensic snapshot fields allowed')
        require(type(row['schemaVersion']) is int and row['schemaVersion']==1
                and row['scope']=='forensic-startup-main-thread-stack' and row['runId']==run_id
                and type(row['sequence']) is int and row['sequence']==index
                and row['performanceSampleAccepted'] is False,'Exact forensic prefix and non-performance scope required')
        require(isinstance(row['process'],dict) and set(row['process'])==PROCESS_FIELDS
                and all(type(value) is int for value in row['process'].values())
                and row['process']==expected_process,'Original native process identity required')
        keys=['traceStartedNs','nativeReadStartedNs','nativeReadFinishedNs','mainThreadId']
        require(all(type(row[key]) is int and row[key]>0 for key in keys),'Positive native clocks and main thread required')
        start,end=row['nativeReadStartedNs'],row['nativeReadFinishedNs']
        if trace_start is None:trace_start=row['traceStartedNs'];thread_id=row['mainThreadId']
        require(row['traceStartedNs']==trace_start and row['mainThreadId']==thread_id
                and trace_start<=start<=end<=trace_start+8000000000
                and (prior_end is None or start>=prior_end),'Stable main thread and original eight-second window required')
        require(isinstance(row['threadState'],str) and row['threadState'] in STATES
                and type(row['truncated']) is bool,'Explicit Java state and truncation marker required')
        require(isinstance(row['frames'],list) and len(row['frames'])<=128
                and (not row['truncated'] or len(row['frames'])==128),'Bounded explicit frame list required')
        for frame in row['frames']:
            require(isinstance(frame,dict) and set(frame)==FRAME_FIELDS,'No argument, payload or extra stack fields allowed')
            require(text(frame['className'],512) and text(frame['methodName'],256)
                    and (frame['fileName'] is None or text(frame['fileName'],256)),'Bounded stack symbols required')
            require(type(frame['lineNumber']) is int and -2<=frame['lineNumber']<=2147483647
                    and type(frame['nativeMethod']) is bool and frame['nativeMethod']==(frame['lineNumber']==-2),
                    'Unambiguous native frame and source line required')
        frames+=len(row['frames']);states.add(row['threadState']);prior_end=end
    require(frames>0,'At least one actual main-thread stack frame required')
    return dict(forensicStackPrefixAccepted=True,snapshots=len(records),frames=frames,mainThreadId=thread_id,
                readWindowNs=prior_end-trace_start,states=sorted(states),performanceSampleAccepted=False,
                actualIpcAccepted=False,fullRF312Accepted=False)
