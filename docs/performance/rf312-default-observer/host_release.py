"""Observe verified SDK shutdown descendants within the original exit budget."""
import copy
import json
import math
from pathlib import Path,PureWindowsPath
import re
import subprocess
import time


class ReleaseState:
    def __init__(self,fresh,spec,sdk_root,system_root):
        fresh.validate_spec(spec,'after');self.fresh=fresh;self.identity=fresh.identity
        self.base=copy.deepcopy(spec);self.sdk=PureWindowsPath(str(sdk_root));self.system=PureWindowsPath(str(system_root))
        if not self.sdk.is_absolute() or not self.system.is_absolute():raise ValueError('Explicit absolute SDK and system roots required')
        self.known={row['ProcessId']:dict(row) for row in spec['ownedProcesses']};self.kinds={pid:'root' for pid in self.known}
        self.original_ids=set(self.known)
        self.qemu={pid:row for pid,row in self.known.items() if row['Name'].startswith('qemu-system-')
                   and PureWindowsPath(row['ExecutablePath']).is_relative_to(self.sdk/'emulator/qemu')}
        for row in self.known.values():
            path=PureWindowsPath(row['ExecutablePath'])
            if row['Name']=='adb.exe':
                if path!=self.sdk/'platform-tools/adb.exe' or not row['CommandLine'].endswith('-L tcp:localhost:5057 server nodaemon'):
                    raise ValueError('Original owned server identity required')
            elif row['Name']=='emulator.exe':
                if path!=self.sdk/'emulator/emulator.exe':raise ValueError('Original owned launcher identity required')
            elif row['ProcessId'] not in self.qemu:raise ValueError('Only original SDK roots may initialize shutdown observation')
            if row['Name']!='adb.exe' and (path.name.casefold()!=row['Name'].casefold()
                    or not re.search(r'(?:^|\s)-avd\s+"?'+re.escape(spec['avd'])+r'"?(?:\s|$)',row['CommandLine'])):
                raise ValueError('Original AVD identity required for emulator roots')
        for row in self.qemu.values():
            parent=self.known.get(row['ParentProcessId'])
            if parent is None or parent['Name']!='emulator.exe' or self.identity.birth(row['CreationDate'])<self.identity.birth(parent['CreationDate']):
                raise ValueError('Original owned VM ancestry required')

    def spec(self):
        result=copy.deepcopy(self.base);result['ownedProcesses']=[dict(row) for row in self.known.values()]
        return result

    def target(self,command,kind):
        if not isinstance(command,str) or any(c in command for c in '\x00\r\n'):return None
        pattern=(r'/C ""([^"]+)" -kill ([1-9][0-9]*) -sleep 20 >nul 2>&1"' if kind=='cmd'
                 else r'"([^"]+)"[ \t]+-kill ([1-9][0-9]*) -sleep 20[ \t]*')
        match=re.fullmatch(pattern,command)
        if match is None or PureWindowsPath(match[1]) not in {self.sdk/'emulator/emulator',self.sdk/'emulator/emulator.exe'}:return None
        pid=int(match[2]);return pid if pid in self.qemu else None

    def observe(self,snapshot):
        self.fresh.audit(self.spec(),snapshot,'after')  # Keep canonical schema, port scope and identity validation.
        current={row['ProcessId']:row for row in snapshot['processes']}
        for pid,row in current.items():
            if pid in self.known and not self.identity.same_identity(row,self.known[pid]):
                raise RuntimeError('Recorded SDK or shutdown descendant PID was reused')
        graph={**self.known,**current};pending={}

        def classify(pid,seen=()):
            if pid in self.kinds:return self.kinds[pid]
            if pid in pending:return pending[pid]
            if pid in seen or len(seen)>=16 or pid not in graph:raise RuntimeError('Unverified shutdown ancestry')
            row=graph[pid];parent_id=row['ParentProcessId'];parent=graph.get(parent_id)
            if parent is None:raise RuntimeError('Shutdown parent not captured')
            kind=classify(parent_id,seen+(pid,))
            if self.identity.birth(row['CreationDate'])<self.identity.birth(parent['CreationDate']):
                raise RuntimeError('Shutdown child predates its recorded parent')
            path=PureWindowsPath(row['ExecutablePath']);role=None
            if row['Name']=='cmd.exe' and path==self.system/'System32/cmd.exe':
                target=self.target(row['CommandLine'],'cmd')
                if target is not None and parent_id==target:role='sdk-shutdown-cmd'
            elif row['Name']=='emulator.exe' and path==self.sdk/'emulator/emulator.exe':
                target=self.target(row['CommandLine'],'emulator')
                if target is not None and kind=='sdk-shutdown-cmd' and self.target(parent['CommandLine'],'cmd')==target:
                    role='sdk-shutdown-helper'
            elif row['Name']=='conhost.exe' and path==self.system/'System32/conhost.exe':
                command=row['CommandLine'];parts=command.rsplit(' ',1)
                if (len(parts)==2 and re.fullmatch(r'0x[0-9a-fA-F]+',parts[1])
                        and PureWindowsPath(parts[0].removeprefix('\\??\\'))==path
                        and kind in ('root','sdk-shutdown-cmd','sdk-shutdown-helper')):role='sdk-console'
            if role is None:raise RuntimeError('Unrecognized shutdown process; no ownership granted')
            pending[pid]=role;return role

        for pid in current:classify(pid)
        for listener in snapshot['listeners']:
            if (listener['LocalPort'] in self.base['protectedPorts'] or listener['OwningProcess'] not in current
                    or listener['OwningProcess'] not in self.original_ids):
                raise RuntimeError('Protected or unverified listener appeared during shutdown')
        for pid,kind in pending.items():self.known[pid]=dict(current[pid]);self.kinds[pid]=kind
        if len(self.known)>128:raise RuntimeError('Original bounded identity scope exceeded')
        return not snapshot['processes'] and not snapshot['listeners']


def wait_for_release(state,capture,deadline,record,*,clock=time.monotonic,sleep=time.sleep):
    if type(deadline) not in (int,float) or not math.isfinite(deadline) or not 0<deadline-clock()<=45:
        raise ValueError('Only the remaining original 45-second emulator exit budget may be used')
    ordinal=0
    while True:
        remaining=deadline-clock()
        if remaining<=0:raise TimeoutError('Original emulator exit budget exhausted with pending host descendants')
        snapshot=capture(state.spec(),timeout=min(15,remaining));ordinal+=1
        remaining=deadline-clock()
        if remaining<=0:
            record(ordinal,dict(accepted=False,deadlineExpired=True))
            raise TimeoutError('Host snapshot exceeded the original remaining exit budget')
        accepted=state.observe(snapshot)
        record(ordinal,dict(accepted=accepted,remainingSeconds=remaining,trackedKinds=state.kinds,
                            inputDispatched=False,processTerminationRequested=False))
        if accepted:return state.spec()['ownedProcesses']
        sleep(min(.5,remaining))


def windows_capture(fresh,powershell,output,*,runner=subprocess.run):
    output=Path(output);output.mkdir(exist_ok=False);ordinal=0
    def capture(spec,*,timeout):
        nonlocal ordinal
        if type(timeout) not in (int,float) or not 0<timeout<=15:raise ValueError('Original bounded host snapshot timeout required')
        ordinal+=1;prefix=output/f'{ordinal:03d}'
        prefix.with_suffix('.expected.json').write_text(json.dumps(spec,indent=2)+'\n',encoding='utf-8')
        try:
            result=runner([str(powershell),'-NoProfile','-NonInteractive','-Command',fresh.powershell_command(spec,'after')],
                stdin=subprocess.DEVNULL,capture_output=True,timeout=timeout,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        except subprocess.TimeoutExpired as error:
            if error.stdout is not None:prefix.with_suffix('.stdout').write_bytes(error.stdout)
            if error.stderr is not None:prefix.with_suffix('.stderr').write_bytes(error.stderr)
            raise
        prefix.with_suffix('.stdout').write_bytes(result.stdout);prefix.with_suffix('.stderr').write_bytes(result.stderr)
        result.check_returncode()
        return fresh.identity.decode_json(result.stdout)
    return capture
