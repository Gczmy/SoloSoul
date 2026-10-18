from pathlib import Path
import hashlib,json,re
import rf312_memory_batches as pairs

read=lambda p:json.loads(p.read_bytes())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def audit_links(folder,point,sdk,cursor,uid):
    logical=point['commands'];transport=folder/'native-sdk'
    if point['kind']=='missed-periodic':
        assert not transport.exists() and logical==[]
        return cursor,dict(logicalReads=0,physicalSdkCalls=0,pairedSdkCalls=0,nativeRawFiles=0,unusedMembers=0)
    files=sorted(transport.glob('[0-9][0-9][0-9].json'));records={};raws={};paired=0;native_raw_files=0
    for ordinal,file in enumerate(files,1):
        record=read(file);assert record['ordinal']==ordinal
        actual=sdk[cursor];cursor+=1
        for key in ['command','timeoutSeconds','exitCode','timedOut','stdoutSha256','stderrSha256']:
            default=False if key=='timedOut' else None
            assert record.get(key,default)==actual.get(key,default),(point['ordinal'],ordinal,key)
        assert 0<record['timeoutSeconds']<=10
        assert point['hostStartedAt']<=record['hostStartedAt']<=record['hostFinishedAt']<=point['hostFinishedAt']
        outputs={}
        for stream in ['stdout','stderr']:
            path=file.with_suffix('.'+stream)
            if stream+'Sha256' in record:
                assert sha(path)==record[stream+'Sha256'];outputs[stream]=path.read_bytes();native_raw_files+=1
            else:assert not path.exists();outputs[stream]=None
        members=tuple(tuple(row) for row in record['logicalMembers']);command=tuple(record['command'])
        if record['nonce'] is not None:
            paired+=1;assert pairs.physical_pair(members,record['nonce'])==command
            if 'members' in record:
                out=pairs.split_pair(outputs['stdout'],record['nonce'])
                err=pairs.split_pair(outputs['stderr'],record['nonce'],limit=pairs.ERROR_LIMIT)
                assert [row['exitCode'] for row in out]==[row['exitCode'] for row in err]
                combined=[dict(a,stderrOffset=b['stdoutOffset'],stderrEnd=b['stdoutEnd'],stderrSha256=b['stdoutSha256']) for a,b in zip(out,err)]
                assert combined==record['members']
        else:
            assert members==(command,)
            assert (command==('shell','pm','list','packages','--uid',str(uid))
                or command==('shell','ps','-A','-o','UID,PID,PPID,NAME')
                or (len(command)==6 and command[:5]==('shell','dumpsys','meminfo','--local','-s')
                    and re.fullmatch('[1-9][0-9]{0,9}',command[5]) and int(command[5])<=2**31-1))
        records[ordinal]=record;raws[ordinal]=outputs
    consumed=[];logical_raws=0
    for index,command in enumerate(logical,1):
        evidence=command.get('nativeEvidence')
        assert 0<command['timeoutSeconds']<=10
        if evidence is None:
            assert not point['accepted'] and command['logicalReplyAvailable'] is False
            continue
        number=evidence['nativeOrdinal'];member=evidence['member'];native=records[number];output=raws[number]
        assert evidence['nativeRecord']==f'{number:03d}.json'
        assert command['command']==native['logicalMembers'][member-1]
        assert evidence['kind']==('pair' if native['nonce'] is not None else 'direct')
        if member==1:assert command['timeoutSeconds']==native['timeoutSeconds']
        if native['nonce'] is None:
            out,err=output['stdout'],output['stderr'];code=native['exitCode']
        elif 'members' in native:
            field=native['members'][member-1];code=field['exitCode']
            out=output['stdout'][field['stdoutOffset']:field['stdoutEnd']]
            err=output['stderr'][field['stderrOffset']:field['stderrEnd']]
        else:
            out=err=code=None
        available=command.get('logicalReplyAvailable') is True
        if available:
            assert not command['timedOut'] and command['exitCode']==code
            assert isinstance(out,bytes) and isinstance(err,bytes)
            assert (folder/f'{index:03d}.stdout').read_bytes()==out
            assert (folder/f'{index:03d}.stderr').read_bytes()==err
            consumed.append(evidence)
        else:
            assert not point['accepted']
            if native['nonce'] is not None:
                assert 'stdoutSha256' not in command and 'stderrSha256' not in command
        for stream in ['stdout','stderr']:
            path=folder/f'{index:03d}.{stream}'
            if stream+'Sha256' in command:
                assert sha(path)==command[stream+'Sha256'];logical_raws+=1
                if not available:
                    assert native['nonce'] is None
                    assert path.read_bytes()==output[stream]
            else:assert not path.exists()
    expected_unused=[];used={(row['nativeOrdinal'],row['member']) for row in consumed}
    for number,native in records.items():
        for member in range(1,len(native['logicalMembers'])+1):
            if (number,member) not in used:
                expected_unused.append(dict(nativeOrdinal=number,nativeRecord=f'{number:03d}.json',member=member,kind='pair' if native['nonce'] is not None else 'direct'))
    summary=read(transport/'result.json');assert summary==point['nativeTransport']
    assert summary['nativeCalls']==len(records) and summary['consumed']==consumed
    assert summary['logicalRepliesConsumed']==len(consumed) and summary['unconsumedPairMembers']==expected_unused
    assert summary['allRepliesConsumed']==(not expected_unused)
    assert summary['inputRetried'] is summary['rawBytesTransformed'] is False
    if point['accepted']:
        assert not expected_unused and len(consumed)==len(logical)
        processes=len(point['snapshot']['processes'])
        assert len(logical)==(7 if point['kind']=='prelaunch' else 6+5*processes)
        assert len(records)==(5 if point['kind']=='prelaunch' else 4+3*processes)
        assert paired==(2 if point['kind']=='prelaunch' else 2+2*processes)
    return cursor,dict(logicalReads=len(logical),physicalSdkCalls=len(records),pairedSdkCalls=paired,
        nativeRawFiles=native_raw_files,logicalRawFiles=logical_raws,unusedMembers=len(expected_unused))
