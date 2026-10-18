"""One five-second platform trace; no application or input is started."""
from pathlib import Path
import hashlib,json,re
R=Path(__file__).resolve().parent
REQUIRED={'android.surfaceflinger.frametimeline','linux.ftrace','linux.process_stats'}
ROUNDTRIP=b'RF312-input-roundtrip\x00'+bytes(range(256))+b'\r\n'
def varint(raw,index,*,protozero_length=False):
    start=index;value=0
    for shift in range(0,70,7):
        if index>=len(raw):raise ValueError('Truncated protobuf varint')
        b=raw[index];index+=1
        if shift==63 and b>1:raise ValueError('Overflowing protobuf varint')
        value|=(b&127)<<shift
        if not b&128:
            # ProtoZero reserves four bytes for nested-message lengths.
            # Tags and scalar values remain canonical; all old bounds apply.
            if index-start>1 and b==0 and not (protozero_length and index-start==4):
                raise ValueError('Unsupported redundant protobuf varint')
            return value,index
    raise ValueError('Unterminated protobuf varint')
def fields(raw,limit=262144,max_fields=4096):
    if not isinstance(raw,bytes) or len(raw)>limit:raise ValueError('Protobuf exceeds bound')
    rows={};index=count=0
    while index<len(raw):
        tag,index=varint(raw,index);number,wire=tag>>3,tag&7
        if not 1<=number<2**29 or wire not in [0,1,2,5]:raise ValueError('Unsupported protobuf field')
        if wire==0:value,index=varint(raw,index)
        else:
            if wire==2:size,index=varint(raw,index,protozero_length=True)
            else:size=8 if wire==1 else 4
            if size>len(raw)-index:raise ValueError('Truncated protobuf payload')
            value=raw[index:index+size];index+=size
        count+=1
        if count>max_fields:raise ValueError('Too many protobuf fields')
        rows.setdefault(number,[]).append((wire,value))
    return rows
def one(rows,key,wire):
    values=rows.get(key,[])
    if len(values)!=1 or values[0][0]!=wire:raise ValueError('Missing or ambiguous protobuf field')
    return values[0][1]
def service_state(raw):
    top=fields(raw);total=one(top,3,0);started=one(top,4,0)
    if total>1000 or started>total:raise ValueError('Invalid current session counts')
    producers={}
    for wire,value in top.get(1,[]):
        if wire!=2:raise ValueError('Producer protobuf wire mismatch')
        p=fields(value);pid=one(p,1,0);uid=one(p,3,0);name=one(p,2,2).decode('utf-8')
        if not 0<pid<2**31 or uid>2**31 or not 0<len(name)<=1024 or '\0' in name or pid in producers:raise ValueError('Invalid producer identity')
        producers[pid]=dict(name=name,uid=uid)
    sources=[]
    for wire,value in top.get(2,[]):
        if wire!=2:raise ValueError('Datasource protobuf wire mismatch')
        d=fields(value);pid=one(d,2,0);desc=fields(one(d,1,2));name=one(desc,1,2).decode('utf-8')
        if pid not in producers or not 0<len(name)<=256 or '\0' in name or (pid,name) in sources:raise ValueError('Invalid datasource binding')
        sources.append((pid,name))
    if not REQUIRED<={name for _,name in sources}:raise ValueError('Required trace sources absent')
    return dict(totalSessions=total,currentlyStartedSessions=started,producerCount=len(producers),dataSourceRows=len(sources),dataSourceNames=sorted(set(name for _,name in sources)))
def remote_paths(nonce):
    if not isinstance(nonce, str) or not re.fullmatch('[0-9a-f]{32}', nonce):
        raise ValueError('Invalid frozen filename token')
    stem = 'rf312-public-' + nonce
    return ['/data/misc/perfetto-configs/' + stem + '-fixture.bin',
            '/data/misc/perfetto-configs/' + stem + '-config.pbtxt',
            '/data/misc/perfetto-traces/' + stem + '-output.trace']


def writer_config(base, output):
    if hashlib.sha256(base).hexdigest() != 'de6be90adb296dcaa2cb168e141fb76d5eb6ffe0ed15340c174bab544d40a518':
        raise ValueError('Fixed five-second base configuration changed')
    return base + ('\nwrite_into_file: true\noutput_path: "' + output + '"\n').encode('ascii')


def require_completed_trace(result):
    """v15 may return zero for a service error; require the complete log protocol."""
    if (type(result.returncode) is not int or result.returncode != 0
            or result.stdout != b'' or not isinstance(result.stdout, bytes)
            or not isinstance(result.stderr, bytes) or len(result.stderr) > 16384
            or not result.stderr.endswith(b'\n')):
        raise ValueError('Unproven Perfetto command completion')
    messages = []
    for line in result.stderr.splitlines():
        match = re.fullmatch(rb'\[[ 0-9]+\.[0-9]+\] +perfetto_cmd\.cc:[1-9][0-9]* +(.+)', line)
        if match is None:
            raise ValueError('Unrecognized Perfetto completion log')
        messages.append(match[1])
    if messages != [b'Connected to the Perfetto traced service, TTL: 5s',
                    b'Trace written into the output file']:
        raise ValueError('Missing or contradictory Perfetto completion log')


def run(call, out):
    out = Path(out)
    path = out / 'system.trace'
    if path.exists() or path.is_symlink() or (out / 'trace-transport.json').exists():
        raise ValueError('Trace destination is not fresh')
    plan = json.loads((R / 'file-plan.json').read_bytes())
    if set(plan) != {'nonce'}:
        raise ValueError('Unexpected file plan fields')
    names = remote_paths(plan['nonce'])
    body = writer_config((R / 'base-config.pbtxt').read_bytes(), names[2])
    config, fixture = R / 'trace-config.pbtxt', R / 'roundtrip.bin'
    if config.read_bytes() != body or fixture.read_bytes() != ROUNDTRIP:
        raise ValueError('Frozen configuration or fixture changed')
    owned, trace_attempted, trace_owned = [], False, False

    def cleanup():
        errors = []
        if owned:
            try:
                call('trace-owned-files-remove', ['shell', 'rm', '-f', *owned], timeout=5)
            except Exception as error:
                errors.append(dict(errorType=type(error).__name__, error=str(error)))
        unknown_output = trace_attempted and not trace_owned
        receipt = dict(passed=not errors and not unknown_output, exactOwnedFiles=list(owned),
                       outputOwnershipProven=trace_owned, unclaimedOutputMayRemain=unknown_output,
                       recursiveDeletionUsed=False, directoriesCreated=False, errors=errors)
        with (out / 'trace-file-cleanup.json').open('x', encoding='utf-8') as stream:
            json.dump(receipt, stream, indent=2)
            stream.write('\n')
        return receipt

    try:
        before = call('trace-state-before', ['exec-out', 'perfetto', '--query-raw'], timeout=5)
        if before.stderr:
            raise ValueError('Service state stderr')
        b = service_state(before.stdout)
        if b['totalSessions'] or b['currentlyStartedSessions']:
            raise ValueError('Foreign trace session exists')
        call('trace-directory-policy', ['shell', 'ls', '-ldZ', '/data/misc/perfetto-configs', '/data/misc/perfetto-traces'], timeout=5)
        for role, local, remote in [('fixture', fixture, names[0]), ('config', config, names[1])]:
            # One remote shell command; the validated literal path has no shell metacharacters.
            # Ownership follows successful O_EXCL/noclobber creation, never an existence check.
            reserved = call('trace-' + role + '-reserve', ['shell', '-T', 'set -C; : > ' + remote], timeout=5)
            owned.append(remote)
            if reserved.stdout or reserved.stderr:
                raise ValueError('Unexpected exclusive reservation output')
            call('trace-' + role + '-push', ['push', str(local), remote], timeout=5)
            checked = call('trace-' + role + '-readback', ['exec-out', 'cat', remote], timeout=5)
            expected = ROUNDTRIP if role == 'fixture' else body
            if checked.stdout != expected or checked.stderr:
                raise ValueError('Raw file-transfer readback mismatch')
        trace_attempted = True
        # v15 service output_path uses O_EXCL. --out must be omitted in this mode.
        completed = call('five-second-trace', ['shell', '-T', 'perfetto', '--txt', '-c', names[1]], timeout=45)
        require_completed_trace(completed)
        trace_owned = True
        owned.append(names[2])
        remote_hash = call('trace-remote-sha256', ['shell', 'sha256sum', names[2]], timeout=5)
        parts = remote_hash.stdout.decode('ascii').split()
        if (remote_hash.stderr or len(parts) != 2 or parts[1] != names[2]
                or not re.fullmatch('[0-9a-f]{64}', parts[0])):
            raise ValueError('Remote trace hash identity mismatch')
        call('trace-file-pull', ['pull', names[2], str(path)], timeout=45)
        if path.is_symlink() or not path.is_file() or not 1 <= path.stat().st_size <= 32 * 1024 * 1024:
            raise ValueError('Trace output absent or too large')
        captured = path.read_bytes()
        digest = hashlib.sha256(captured).hexdigest()
        if digest != parts[0]:
            raise ValueError('Pulled trace hash mismatch')
        envelope = fields(captured, 32 * 1024 * 1024, 500000)
        if set(envelope) != {1} or not envelope[1] or any(w != 2 or not v for w, v in envelope[1]):
            raise ValueError('Trace packet envelope rejected')
        after = call('trace-state-after', ['exec-out', 'perfetto', '--query-raw'], timeout=5)
        if after.stderr:
            raise ValueError('Final service state stderr')
        a = service_state(after.stdout)
        if a['totalSessions'] or a['currentlyStartedSessions']:
            raise ValueError('Trace session did not finish')
    except BaseException:
        try:
            cleanup()
        except Exception as cleanup_error:
            with (out / 'trace-cleanup-record-error.txt').open('x', encoding='utf-8') as stream:
                stream.write(repr(cleanup_error))
        raise
    clean = cleanup()
    if not clean['passed']:
        raise ValueError('Owned trace file cleanup failed')
    result = dict(passed=True, scope='five-second-platform-trace-transport-only',
                  configSha256=hashlib.sha256(body).hexdigest(), traceSha256=digest,
                  remoteTraceSha256=parts[0], traceBytes=len(captured), topLevelPackets=len(envelope[1]),
                  serviceBefore=b, serviceAfter=a, rawFileRoundtripBytes=len(ROUNDTRIP),
                  rawFileRoundtripSha256=hashlib.sha256(ROUNDTRIP).hexdigest(),
                  cliCompletionProtocolVerified=True,
                  serviceWriterExclusiveCreate=True, traceRecordingStarted=True,
                  traceRecordingCompleted=True, ownedRemoteFilesRemoved=True,
                  productPackagesInstalled=False, applicationDataAccepted=False, nativeFramesAccepted=False,
                  firstInteractiveTimeProven=False, fullRF312Accepted=False)
    with (out / 'trace-transport.json').open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(result, stream, indent=2)
        stream.write('\n')
    return result
