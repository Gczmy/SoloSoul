"""Read API31 instrumentation from the supported, unfiltered process dump.

The former instrumentation subcommand can return exit zero with an error body.
Only a complete section bounded by the process header and later OOM section is
evidence of presence or absence. This module never starts instrumentation.
"""
import hashlib
import math
import re
import time

HEADER='ACTIVITY MANAGER RUNNING PROCESSES (dumpsys activity processes)'
COMPONENT=r'([A-Za-z][A-Za-z0-9_.]*)/(\.?[A-Za-z_$][A-Za-z0-9_.$]*)'


def require(value,message):
    if not value:raise ValueError(message)


def component(value):
    require(isinstance(value,str),'An explicit component string is required')
    match=re.fullmatch(COMPONENT,value)
    require(match is not None,'An exact package/class component is required')
    package,entry=match.groups()
    return package+'/'+(package+entry if entry.startswith('.') else entry)


def parse(raw):
    require(isinstance(raw,bytes) and 0<len(raw)<=4*1024*1024,'A bounded complete native process dump is required')
    text=raw.decode('utf-8').replace('\r\n','\n')
    require('\r' not in text and '\0' not in text,'Unambiguous native text is required')
    lines=text.splitlines()
    require(lines and lines[0]==HEADER and lines.count(HEADER)==1,'The supported unfiltered process command is required')
    require(lines.count('  All known processes:')==1 and lines.count('  OOM levels:')==1,'Complete process and post-instrumentation boundaries are required')
    end=lines.index('  OOM levels:')
    require(0<lines.index('  All known processes:')<end,'Ordered native process boundaries are required')
    require(not re.search(r'(?im)^(?:Unknown command:|Bad activity command|Permission Denial:|DUMP TIMEOUT|Exception occurred)',text),
            'SDK command errors are not absence evidence')
    head=[i for i,line in enumerate(lines) if line=='  Active instrumentation:']
    require(len(head)<=1 and (not head or head[0]<end),'One complete active instrumentation section is required')
    if not head:
        require(not any('Instrumentation #' in line or re.match(r'\s*mClass=',line) for line in lines[:end]),
                'Unscoped instrumentation data cannot prove absence')
        return []
    section=lines[head[0]+1:end]
    starts=[i for i,line in enumerate(section) if line.startswith('    Instrumentation #')]
    require(starts and starts[0]==0,'A populated, explicit instrumentation section is required')
    result=[]
    for ordinal,start in enumerate(starts):
        block=section[start:(starts[ordinal+1] if ordinal+1<len(starts) else len(section))]
        top=re.fullmatch(r'    Instrumentation #([0-9]+): ActiveInstrumentation\{[a-f0-9]+ \{('+COMPONENT+r')\} ([1-9][0-9]*) procs\}',block[0])
        require(top is not None and int(top[1])==ordinal,'Ordered, unfinished native instrumentation headers are required')
        top_component=component(top[2]);count=int(top[5])
        classes=[re.fullmatch(r'      mClass=ComponentInfo\{('+COMPONENT+r')\} mFinished=false',line) for line in block if line.startswith('      mClass=')]
        targets=[re.fullmatch(r'      mTargetProcesses=\[([A-Za-z][A-Za-z0-9_.]*)\]',line) for line in block if line.startswith('      mTargetProcesses=')]
        infos=[re.fullmatch(r'      mTargetInfo=ApplicationInfo\{[a-f0-9]+ ([A-Za-z][A-Za-z0-9_.]*)\}',line) for line in block if line.startswith('      mTargetInfo=')]
        require(len(classes)==len(targets)==len(infos)==1 and classes[0] and targets[0] and infos[0],
                'Exact class, finished state and one target process/application are required')
        require(component(classes[0][1])==top_component and targets[0][1]==infos[0][1],
                'Native component and primary target declarations must agree')
        require(block.count('      mRunningProcesses:')==1,'One explicit native running process list is required')
        running=[]
        for line in block:
            if line.startswith('        #'):
                match=re.fullmatch(r'        #([0-9]+): ProcessRecord\{[a-f0-9]+ ([1-9][0-9]*):([A-Za-z][A-Za-z0-9_.]*)/[^\s{}]+\}',line)
                require(match is not None and int(match[1])==len(running) and match[3]==targets[0][1],
                        'Only the declared primary native process may be instrumented')
                running.append(int(match[2]))
        require(count==len(running)==1,'Exactly one running primary process per instrumentation is required')
        result.append(dict(component=top_component,target=targets[0][1],processId=running[0]))
    require(len({row['component'] for row in result})==len(result) and len({row['processId'] for row in result})==len(result),
            'Distinct native instrumentation components and processes are required')
    return result


def expected(rows,components,*,process_ids=None):
    require(isinstance(components,dict),'An exact component-to-target allowlist is required')
    normalized={component(key):value for key,value in components.items()}
    require(len(normalized)==len(components) and all(isinstance(v,str) and re.fullmatch(r'[A-Za-z][A-Za-z0-9_.]*',v) for v in normalized.values())
            and len(set(normalized.values()))==len(normalized),
            'Distinct explicit native targets are required')
    require(len(rows)==len(normalized) and {row['component']:row['target'] for row in rows}==normalized,
            'Only the exact declared instrumentation may be active')
    if process_ids is not None:
        require(isinstance(process_ids,dict) and all(type(pid) is int and pid>0 for pid in process_ids.values()),
                'Explicit bound native PIDs are required')
        observed={row['target']:row['processId'] for row in rows}
        require(all(observed.get(name)==pid for name,pid in process_ids.items()),'Original native instrumented process identity is required')
    return dict(instrumentationStateAccepted=True,activeCount=len(rows),nativeRows=rows,
                performanceSampleAccepted=False,actualIpcAccepted=False,fullRF312Accepted=False)


def capture(sdk,deadline,record,*,clock=time.monotonic):
    require(type(deadline) in (int,float) and math.isfinite(deadline) and 0<deadline-clock()<=45,
            'The existing finite phase deadline is required')
    code,out,err=sdk(['shell','dumpsys','activity','processes'],timeout=min(5,deadline-clock()))
    record(dict(exitCode=code,stdout=out,stderr=err,performanceSampleAccepted=False))
    require(clock()<deadline,'Native state arrived after the original phase deadline')
    require(type(code) is int and code==0 and err==b'','A successful native process command with empty stderr is required')
    rows=parse(out)
    return rows,hashlib.sha256(out).hexdigest()
