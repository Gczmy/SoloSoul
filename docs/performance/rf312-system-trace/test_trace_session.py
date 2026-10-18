from pathlib import Path
import hashlib
import json
import subprocess
import tempfile
import types
import unittest

import trace_session as m

ROOT=Path(__file__).resolve().parent
def vi(n):
    b=bytearray()
    while n>=128:b.append((n&127)|128);n>>=7
    b.append(n);return bytes(b)
def field(n,v):return vi(n*8+2)+vi(len(v))+v if isinstance(v,bytes) else vi(n*8)+vi(v)
def state(total):
    p=field(1,1)+field(2,b'traced_probes')+field(3,0)
    ds=b''.join(field(2,field(1,field(1,n.encode()))+field(2,1)) for n in sorted(m.transport.REQUIRED))
    return field(1,p)+ds+field(3,total)+field(4,total)
def log(message):return b'[100.000] perfetto_cmd.cc:792     '+message+b'\n'

class Device:
    def __init__(self,names,fault=None):
        self.names=names;self.fault=fault;self.active=False;self.files={};self.calls=[]
        self.trace=field(1,b'public-native-packet')
    def __call__(self,label,args,*,timeout):
        self.calls.append((label,args,timeout));out=err=b'';code=0
        if label.startswith('session-state-'):
            count=1 if self.active else 0
            if self.fault=='foreign-before' and label=='session-state-before':count=1
            if self.fault=='wrong-active' and label=='session-state-active':count=2
            out=state(count)
        elif label.endswith('-reserve'):
            path=args[-1].removeprefix('set -C; : > ')
            if self.fault=='nonzero-reserve':
                self.files[path]=b'foreign';code=1
            else:self.files[path]=b''
        elif label.endswith('-push'):self.files[args[-1]]=Path(args[-2]).read_bytes()
        elif label.endswith('-readback'):
            out=self.files[args[-1]]
            if self.fault=='readback':out=b'changed'
        elif label=='session-start':
            if self.fault=='service-error-zero':
                self.files[self.names[2]]=field(1,b'foreign-output')
                err=log(b'Connected to the Perfetto traced service, TTL: 240s')+log(b'Service error: cannot create output')
            else:
                self.active=True;self.files[self.names[2]]=self.trace
                err=log(b'Connected to the Perfetto traced service, TTL: 240s')
                if self.fault=='start-timeout':raise subprocess.TimeoutExpired(args,5,stderr=err)
        elif label=='session-key-state':
            if not self.active:code=2
        elif label=='session-stop':
            if self.fault=='stop-timeout':raise subprocess.TimeoutExpired(args,45)
            self.active=False;err=log(b'Trace written into the output file')
            if self.fault=='stop-warning':err+=log(b'Final flush unsuccessful.')
        elif label=='session-trace-hash':out=(hashlib.sha256(self.trace).hexdigest()+'  '+self.names[2]+'\n').encode()
        elif label=='session-trace-pull':Path(args[-1]).write_bytes(b'changed' if self.fault=='hash' else self.trace)
        elif label=='session-owned-files-remove':
            if self.fault=='cleanup':raise subprocess.CalledProcessError(1,args)
            for path in args[3:]:self.files.pop(path,None)
        return types.SimpleNamespace(returncode=code,stdout=out,stderr=err)

class WholeJourneySession(unittest.TestCase):
    def case(self,fault=None):
        folder=tempfile.TemporaryDirectory();self.addCleanup(folder.cleanup)
        root=Path(folder.name);inputs=root/'inputs';inputs.mkdir();out=root/'out';out.mkdir()
        base=(ROOT/'base-config.pbtxt').read_bytes();names=m.transport.remote_paths('c'*32)
        (inputs/'base-config.pbtxt').write_bytes(base)
        (inputs/'file-plan.json').write_text(json.dumps({'nonce':'c'*32}),encoding='utf-8')
        (inputs/'roundtrip.bin').write_bytes(m.transport.ROUNDTRIP)
        (inputs/'trace-config.pbtxt').write_bytes(m.config(base,names[2]))
        device=Device(names,fault);session=m.Session(device,inputs,out)
        return session,device,out,inputs
    def test_one_session_brackets_work_and_stops_exact_key(self):
        s,d,out,_=self.case();s.start();self.assertTrue(d.active)
        result=s.finish();self.assertFalse(d.active);self.assertEqual(d.files,{})
        self.assertFalse(result['wholeJourneyCoverageAccepted']);self.assertFalse(result['fullRF312Accepted'])
        b=result['hostMonotonicBrackets'];self.assertLessEqual(b['startBeforeNs'],b['activeAfterNs']);self.assertLessEqual(b['activeAfterNs'],b['stopBeforeNs'])
        self.assertEqual(sum(label=='session-stop' for label,_,_ in d.calls),1)
        self.assertTrue(all(timeout in (5,45) for _,_,timeout in d.calls))
        self.assertFalse(any('-o' in args or 'mkdir' in args or '-r' in args for _,args,_ in d.calls))
        stop=next(args for label,args,_ in d.calls if label=='session-stop');self.assertEqual(stop[-2:],[f'--attach={s.key}','--stop'])
    def test_original_base_fixed_except_declared_duration_and_size(self):
        s,_,_,_=self.case();self.assertIn(b'duration_ms: 240000\n',s.body)
        self.assertIn(b'max_file_size_bytes: 33554432\n',s.body)
        self.assertIn(b'flush_period_ms: 500\n',s.body)
    def test_foreign_before_prevents_reservation(self):
        s,d,out,_=self.case('foreign-before')
        with self.assertRaises(ValueError):s.start()
        self.assertEqual([x[0] for x in d.calls],['session-state-before']);self.assertFalse(d.files)
    def test_nonzero_reservation_does_not_claim_foreign_file(self):
        s,d,out,_=self.case('nonzero-reserve')
        with self.assertRaises(subprocess.CalledProcessError):s.start()
        self.assertEqual(d.files,{s.names[0]:b'foreign'})
        self.assertEqual(json.loads((out/'session-cleanup.json').read_bytes())['exactOwnedFiles'],[])
    def test_readback_mismatch_never_starts(self):
        s,d,_,_=self.case('readback')
        with self.assertRaises(ValueError):s.start()
        self.assertFalse(d.files);self.assertNotIn('session-start',[x[0] for x in d.calls])
    def test_zero_exit_error_preserves_unclaimed_output(self):
        s,d,out,_=self.case('service-error-zero')
        with self.assertRaises(ValueError):s.start()
        self.assertEqual(d.files,{s.names[2]:field(1,b'foreign-output')})
        self.assertTrue(json.loads((out/'session-cleanup.json').read_bytes())['unclaimedOutputMayRemain'])
    def test_start_timeout_recovers_only_own_key_without_claiming_output(self):
        s,d,out,_=self.case('start-timeout')
        with self.assertRaises(subprocess.TimeoutExpired):s.start()
        self.assertFalse(d.active);self.assertIn(s.names[2],d.files)
        c=json.loads((out/'session-cleanup.json').read_bytes());self.assertTrue(c['sessionStopCompleted']);self.assertFalse(c['passed'])
    def test_wrong_active_count_stops_only_owned_key(self):
        s,d,_,_=self.case('wrong-active')
        with self.assertRaises(ValueError):s.start()
        self.assertFalse(d.active);self.assertFalse(d.files)
    def test_expired_session_is_not_retried_or_accepted(self):
        s,d,out,_=self.case();s.start();d.active=False
        with self.assertRaises(ValueError):s.finish()
        self.assertNotIn('session-stop',[x[0] for x in d.calls]);self.assertFalse((out/'session-transport.json').exists())
        self.assertFalse(json.loads((out/'session-cleanup.json').read_bytes())['passed'])
    def test_stop_timeout_does_not_retry(self):
        s,d,out,_=self.case('stop-timeout');s.start()
        with self.assertRaises(subprocess.TimeoutExpired):s.finish()
        self.assertEqual(sum(label=='session-stop' for label,_,_ in d.calls),1)
        self.assertFalse(json.loads((out/'session-cleanup.json').read_bytes())['sessionStopCompleted'])
    def test_stop_warning_refuses_trace(self):
        s,d,out,_=self.case('stop-warning');s.start()
        with self.assertRaises(ValueError):s.finish()
        self.assertNotIn('session-trace-pull',[x[0] for x in d.calls]);self.assertFalse((out/'session-transport.json').exists())
    def test_hash_mismatch_refuses_trace(self):
        s,d,out,_=self.case('hash');s.start()
        with self.assertRaises(ValueError):s.finish()
        self.assertFalse((out/'session-transport.json').exists());self.assertFalse(d.files)
    def test_changed_inputs_recovered_without_continuing_journey(self):
        s,d,out,i=self.case();s.start();(i/'roundtrip.bin').write_bytes(b'changed')
        with self.assertRaises(ValueError):s.finish()
        self.assertFalse(d.active);self.assertFalse(d.files)
    def test_cleanup_failure_not_accepted(self):
        s,d,out,_=self.case('cleanup');s.start()
        with self.assertRaises(ValueError):s.finish()
        self.assertFalse(json.loads((out/'session-cleanup.json').read_bytes())['passed'])
    def test_no_duplicate_start_or_finish(self):
        s,d,_,_=self.case();s.start()
        with self.assertRaises(ValueError):s.start()
        s.finish()
        with self.assertRaises(ValueError):s.finish()

if __name__=='__main__':unittest.main()
