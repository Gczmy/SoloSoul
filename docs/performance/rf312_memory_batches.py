"""Batch only independent native metadata pairs; retain every actual SDK reply."""
from pathlib import Path
import hashlib,json,math,re,shlex,subprocess,time,uuid

APP='com.solosoul.app'
LIMIT=4*1024*1024
ERROR_LIMIT=65536
PROCESSES=('shell','dumpsys','activity','processes')
SERVICES=('shell','dumpsys','activity','services',APP)
SHELL_PREFIX=('shell','-T','-e','none','sh','-c')


def pair_for(command):
    if command==PROCESSES:return (PROCESSES,SERVICES)
    if len(command)==3 and command[:2]==('shell','cat'):
        match=re.fullmatch('/proc/([1-9][0-9]{0,9})/stat',command[2])
        if match and int(match[1])<=2**31-1:
            return (command,('shell','cat','/proc/'+match[1]+'/status'))
    return None


def physical_pair(commands,nonce):
    if (not isinstance(commands,tuple) or len(commands)!=2 or pair_for(commands[0])!=commands
            or not isinstance(nonce,str) or re.fullmatch('[0-9a-f]{32}',nonce) is None):
        raise ValueError('Exactly one fixed independent metadata pair and nonce required')
    lines=[]
    for ordinal,command in enumerate(commands,1):
        lines.append(shlex.join(command[1:]))
        lines.append('rf312_rc=$?')
        marker="printf '\\000RF312PAIR "+nonce+' '+str(ordinal)+" %s\\000' \"$rf312_rc\""
        lines.extend([marker,marker+' >&2'])
    # Use the original non-PTY SDK shell transport with separate output/error
    # pipes. The fixed script is one quoted argument; no user text is evaluated.
    return SHELL_PREFIX+(shlex.quote('\n'.join(lines)),)


def split_pair(raw,nonce,*,limit=LIMIT):
    if (type(limit) is not int or not 0<limit<=LIMIT or not isinstance(raw,bytes) or not 0<len(raw)<=2*limit+256
            or not isinstance(nonce,str) or re.fullmatch('[0-9a-f]{32}',nonce) is None):
        raise ValueError('Bounded complete native pair bytes required')
    prefix=b'\0RF312PAIR '+nonce.encode()+b' '
    matches=list(re.finditer(re.escape(prefix)+rb'([12]) ([0-9]{1,3})\x00',raw))
    if (len(matches)!=2 or raw.count(prefix)!=2 or matches[0][1]!=b'1' or matches[1][1]!=b'2'
            or matches[-1].end()!=len(raw)):
        raise ValueError('Exactly two ordered complete native pair frames required')
    result=[];offset=0
    for match in matches:
        code=int(match[2]);end=match.start()
        if code>255 or end-offset>limit:raise ValueError('Original member byte/exit-code bound exceeded')
        result.append(dict(exitCode=code,stdoutOffset=offset,stdoutEnd=end,
                           stdoutSha256=hashlib.sha256(raw[offset:end]).hexdigest()))
        offset=match.end()
    return result


class Adapter:
    def __init__(self,sdk,output,uid):
        if type(uid) is not int or not 10000<=uid<20000:raise ValueError('Original app UID required')
        self.sdk,self.uid=sdk,uid;self.output=Path(output);self.output.mkdir(exist_ok=False)
        self.records=[];self.pending=None;self.consumed=[]

    def _native(self,command,timeout,members,nonce=None):
        ordinal=len(self.records)+1;prefix=self.output/f'{ordinal:03d}'
        record=dict(ordinal=ordinal,command=list(command),logicalMembers=[list(c) for c in members],
            timeoutSeconds=timeout,hostStartedAt=time.monotonic(),nonce=nonce)
        self.records.append(record)
        evidence=dict(nativeOrdinal=ordinal,nativeRecord=str(ordinal).zfill(3)+'.json',member=1,
                      kind='pair' if nonce is not None else 'direct')
        try:
            value=self.sdk(*command,timeout=timeout,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            record.update(exitCode=value.returncode,timedOut=False)
            out,err=value.stdout,value.stderr
        except BaseException as error:
            out,err=getattr(error,'stdout',None),getattr(error,'stderr',None)
            record.update(exitCode=getattr(error,'returncode',None),timedOut=isinstance(error,subprocess.TimeoutExpired),errorType=type(error).__name__)
            error.nativeEvidence=evidence
            raise
        finally:
            record['hostFinishedAt']=time.monotonic()
            for name,raw in [('stdout',locals().get('out')),('stderr',locals().get('err'))]:
                if raw is not None:
                    if not isinstance(raw,bytes):raise ValueError('Native SDK must return actual bytes')
                    prefix.with_suffix('.'+name).write_bytes(raw);record[name+'Sha256']=hashlib.sha256(raw).hexdigest()
            prefix.with_suffix('.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
        return value,record,evidence

    def run(self,*command,**kwargs):
        timeout=kwargs.pop('timeout',None)
        if (type(timeout) not in (int,float) or not math.isfinite(timeout) or not 0<timeout<=10
                or kwargs!={'stdout':subprocess.PIPE,'stderr':subprocess.PIPE}):
            raise ValueError('Captured native command within original ten-second cap required')
        if self.pending is not None:
            expected,value,evidence=self.pending
            if command!=expected:raise RuntimeError('Metadata pair consumption order changed; no fallback')
            self.pending=None
            return self._deliver(value,evidence)
        pair=pair_for(command)
        direct=(command==('shell','pm','list','packages','--uid',str(self.uid))
                or command==('shell','ps','-A','-o','UID,PID,PPID,NAME')
                or (len(command)==6 and command[:5]==('shell','dumpsys','meminfo','--local','-s')
                    and re.fullmatch('[1-9][0-9]{0,9}',command[5]) is not None and int(command[5])<=2**31-1))
        if pair is None and not direct:raise ValueError('Unsupported logical memory command; no fallback')
        nonce=uuid.uuid4().hex if pair else None
        physical=physical_pair(pair,nonce) if pair else command
        native,record,evidence=self._native(physical,timeout,pair or (command,),nonce)
        try:
            if native.returncode!=0:
                raise RuntimeError('Native SDK failed; no logical values accepted')
            if pair:
                outputs=split_pair(native.stdout,nonce);errors=split_pair(native.stderr,nonce,limit=ERROR_LIMIT)
                if [row['exitCode'] for row in outputs]!=[row['exitCode'] for row in errors]:
                    raise RuntimeError('Native stdout/stderr exit statuses differ')
                members=[dict(out,stderrOffset=err['stdoutOffset'],stderrEnd=err['stdoutEnd'],stderrSha256=err['stdoutSha256'])
                         for out,err in zip(outputs,errors)]
                record['members']=members
                values=[]
                for index,member in enumerate(members,1):
                    receipt={**evidence,'member':index}
                    value=subprocess.CompletedProcess(list(pair[index-1]),member['exitCode'],
                        native.stdout[member['stdoutOffset']:member['stdoutEnd']],native.stderr[member['stderrOffset']:member['stderrEnd']])
                    value.nativeEvidence=receipt;values.append((value,receipt))
                self.pending=(pair[1],*values[1]);value,evidence=values[0]
            else:
                if (not isinstance(native.stdout,bytes) or not isinstance(native.stderr,bytes)
                        or len(native.stdout)>LIMIT or len(native.stderr)>ERROR_LIMIT):raise ValueError('Bounded direct native reply required')
                value=subprocess.CompletedProcess(list(command),0,native.stdout,native.stderr);value.nativeEvidence=evidence
        except BaseException as error:
            error.nativeEvidence=evidence;record['logicalDecodeErrorType']=type(error).__name__;raise
        finally:
            (self.output/f"{record['ordinal']:03d}.json").write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
        return self._deliver(value,evidence)

    def _deliver(self,value,evidence):
        self.consumed.append(evidence)
        try:value.check_returncode()
        except subprocess.CalledProcessError as error:
            error.nativeEvidence=evidence;error.logicalReplyAvailable=True;raise
        return value

    def require_empty(self):
        if self._unused():raise RuntimeError('Unconsumed metadata reply cannot form a complete point')

    def _unused(self):
        consumed={(row['nativeOrdinal'],row['member']) for row in self.consumed}
        return [dict(nativeOrdinal=row['ordinal'],nativeRecord=f"{row['ordinal']:03d}.json",member=index,
                     kind='pair' if row['nonce'] is not None else 'direct')
                for row in self.records for index in range(1,len(row['logicalMembers'])+1)
                if (row['ordinal'],index) not in consumed]

    def close(self):
        unused=self._unused()
        result=dict(nativeCalls=len(self.records),logicalRepliesConsumed=len(self.consumed),
            consumed=self.consumed,unconsumedPairMembers=unused,allRepliesConsumed=not unused,
            inputRetried=False,rawBytesTransformed=False)
        (self.output/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
        return result
