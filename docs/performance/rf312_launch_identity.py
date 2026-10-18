"""Wait for complete observed identities of the two SDK children we launched.

No missing value is filled from the launch request. The existing bootstrap
quiescence/ownership readers receive only actual complete OS process rows.
"""
import copy
import datetime
import math
import os
from pathlib import PureWindowsPath
import time

FIELDS={'Name','ProcessId','ParentProcessId','CreationDate','ExecutablePath','CommandLine'}
SPEC_FIELDS={'Name','ProcessId','ParentProcessId','ExecutablePath','CommandLine','BornNotBefore'}


def birth(value):
    if not isinstance(value,str):raise ValueError('Offset-aware observed process birth required')
    result=datetime.datetime.fromisoformat(value.replace('Z','+00:00'))
    if result.tzinfo is None:raise ValueError('Offset-aware observed process birth required')
    return result


def pid(value):
    return type(value) is int and 0<value<2**32


def wait_for_roots(snapshot,expected,alive,deadline,record,*,clock=time.monotonic,sleep=time.sleep):
    expected=copy.deepcopy(expected)
    if (not isinstance(expected,list) or len(expected)!=2
            or any(not isinstance(row,dict) or set(row)!=SPEC_FIELDS for row in expected)
            or any(not isinstance(row['Name'],str) for row in expected)
            or {row['Name'] for row in expected}!={'adb.exe','emulator.exe'}):
        raise ValueError('Two explicit SDK child launch specifications required')
    for row in expected:
        if (not pid(row['ProcessId']) or row['ProcessId']==os.getpid() or row['ParentProcessId']!=os.getpid()
                or type(row['ParentProcessId']) is not int or not isinstance(row['ExecutablePath'],str)
                or not PureWindowsPath(row['ExecutablePath']).is_absolute()
                or PureWindowsPath(row['ExecutablePath']).name.casefold()!=row['Name']
                or not isinstance(row['CommandLine'],str) or not row['CommandLine']):
            raise ValueError('Exact own SDK PID, parent, executable and command required')
        birth(row['BornNotBefore'])
    if len({row['ProcessId'] for row in expected})!=2:
        raise ValueError('Distinct SDK child PIDs required')
    if (type(deadline) not in (int,float) or not math.isfinite(deadline)
            or not 0<deadline-clock()<=240):
        raise ValueError('Existing finite 240-second boot deadline required')
    known={row['ProcessId']:{} for row in expected};ordinal=0

    def remaining():
        value=deadline-clock()
        if value<=0:raise TimeoutError('Original boot deadline exhausted while binding SDK roots')
        if alive() is not True:raise RuntimeError('Owned SDK launcher ended before complete identity binding')
        return value

    while True:
        rows=snapshot(timeout=min(15,remaining()));remaining()
        if not isinstance(rows,list) or any(not isinstance(row,dict) for row in rows):
            raise ValueError('Complete current OS process rows required')
        selected=[];complete=True
        for spec in expected:
            matches=[row for row in rows if row.get('ProcessId')==spec['ProcessId']]
            if len(matches)>1:raise RuntimeError('Ambiguous SDK process identity')
            if not matches:
                if known[spec['ProcessId']]:raise RuntimeError('Previously observed SDK root disappeared')
                complete=False;continue
            row=matches[0]
            if set(row)!=FIELDS:raise ValueError('Exact observed OS process fields required')
            if (row['Name']!=spec['Name'] or not pid(row['ProcessId'])
                    or type(row['ParentProcessId']) is not int or row['ParentProcessId']!=spec['ParentProcessId']):
                raise RuntimeError('SDK name, PID or parent changed')
            for key in ['CreationDate','ExecutablePath','CommandLine']:
                value=row[key];previous=known[spec['ProcessId']].get(key)
                if value is None:
                    if previous is not None:raise RuntimeError('Previously observed SDK identity field disappeared')
                    complete=False;continue
                if not isinstance(value,str) or not value:raise ValueError('Unambiguous observed SDK identity strings required')
                if key=='CreationDate':
                    if birth(value)<birth(spec['BornNotBefore']):raise RuntimeError('SDK PID belongs to an older process')
                    same=previous is None or birth(value)==birth(previous)
                elif key=='ExecutablePath':
                    if PureWindowsPath(value)!=PureWindowsPath(spec[key]):raise RuntimeError('Observed SDK executable differs from launch')
                    same=previous is None or PureWindowsPath(value)==PureWindowsPath(previous)
                else:
                    if value!=spec[key]:raise RuntimeError('Observed SDK command differs from launch')
                    same=previous is None or value==previous
                if not same:raise RuntimeError('SDK process identity changed during binding')
                known[spec['ProcessId']][key]=value
            selected.append(copy.deepcopy(row))
        ordinal+=1
        record(ordinal,dict(observedRoots=selected,identityRowsComplete=complete,
                            missingValuesInferred=False,ownershipGranted=False))
        remaining()
        if complete and len(selected)==2:return selected
        sleep(min(.1,remaining()))
