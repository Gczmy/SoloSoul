from pathlib import Path
import hashlib,importlib.util,json,shutil,subprocess,tempfile,types,unittest
spec=importlib.util.spec_from_file_location('service_transport',Path(__file__).with_name('transport.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
ROOT=Path(__file__).parent
def vi(n):
    b=bytearray()
    while n>=128:b.append((n&127)|128);n>>=7
    b.append(n);return bytes(b)
def field(n,v):return vi(n*8+2)+vi(len(v))+v if isinstance(v,bytes) else vi(n*8)+vi(v)
def state(total=0):
    p=field(1,1)+field(2,b'traced_probes')+field(3,0)
    ds=b''.join(field(2,field(1,field(1,n.encode()))+field(2,1)) for n in sorted(m.REQUIRED))
    return field(1,p)+ds+field(3,total)+field(4,total)

class FakeAdb:
    def __init__(self,fault=None):
        self.calls=[];self.fault=fault;self.trace=field(1,b'public-packet');self.files={}
        self.names=m.remote_paths(json.loads((m.R/'file-plan.json').read_bytes())['nonce'])
        if fault in ['fixture-collision','config-collision','output-collision']:
            self.files[self.names[['fixture-collision','config-collision','output-collision'].index(fault)]]=b'keep-existing'
    def __call__(self,label,args,**kw):
        self.calls.append((label,args,kw));raw=b''
        if label in ['trace-state-before','trace-state-after']:raw=state(1 if self.fault==label else 0)
        elif label.endswith('-reserve'):
            target=args[-1].removeprefix('set -C; : > ')
            if target in self.files:raise subprocess.CalledProcessError(1,args)
            self.files[target]=b''
        elif label.endswith('-push'):
            if self.fault=='push':raise subprocess.CalledProcessError(1,args)
            self.files[args[-1]]=Path(args[-2]).read_bytes()
        elif label.endswith('-readback'):
            raw=self.files[args[-1]]
            if self.fault==label:raw=b'corrupted'
        elif label=='five-second-trace':
            if self.fault=='timeout':raise subprocess.TimeoutExpired(args,45,output=b'public-partial')
            if self.names[2] in self.files:raise subprocess.CalledProcessError(1,args)
            if self.fault=='envelope':self.trace=b'bad'
            if self.fault=='empty':self.trace=b''
            self.files[self.names[2]]=self.trace
        elif label=='trace-remote-sha256':raw=(hashlib.sha256(self.trace).hexdigest()+'  '+args[-1]+'\r\n').encode()
        elif label=='trace-file-pull':
            p=Path(args[-1]);p.write_bytes(b'changed' if self.fault=='hash' else self.trace)
            if self.fault=='oversize':
                with p.open('r+b') as f:f.truncate(32*1024*1024+1)
        elif label=='trace-owned-files-remove':
            if self.fault=='cleanup':raise subprocess.CalledProcessError(1,args)
            for name in args[3:]:self.files.pop(name,None)
        return types.SimpleNamespace(returncode=0,stdout=raw,stderr=b'')

class ServiceTransport(unittest.TestCase):
    def run_case(self,fault=None):
        with tempfile.TemporaryDirectory() as folder:
            previous=m.R;m.R=Path(folder)/'inputs';m.R.mkdir();out=Path(folder)/'output';out.mkdir()
            try:
                base=(ROOT/'base-config.pbtxt').read_bytes();nonce='a'*32
                (m.R/'base-config.pbtxt').write_bytes(base)
                (m.R/'file-plan.json').write_text(json.dumps({'nonce':nonce}),encoding='utf-8')
                (m.R/'trace-config.pbtxt').write_bytes(m.writer_config(base,m.remote_paths(nonce)[2]))
                (m.R/'roundtrip.bin').write_bytes(m.ROUNDTRIP)
                fake=FakeAdb(fault)
                if fault=='destination':(out/'system.trace').write_bytes(b'keep')
                if fault in ['config-tamper','base-tamper','fixture-tamper']:
                    (m.R/{'config-tamper':'trace-config.pbtxt','base-tamper':'base-config.pbtxt','fixture-tamper':'roundtrip.bin'}[fault]).write_bytes(b'changed')
                if fault is None:
                    result=m.run(fake,out);self.assertFalse(result['fullRF312Accepted']);self.assertTrue(result['serviceWriterExclusiveCreate'])
                    self.assertEqual(result['traceSha256'],result['remoteTraceSha256']);self.assertEqual(fake.files,{})
                else:
                    with self.assertRaises((ValueError,subprocess.SubprocessError)):m.run(fake,out)
                    self.assertFalse((out/'trace-transport.json').exists())
                return fake, json.loads((out/'trace-file-cleanup.json').read_bytes()) if (out/'trace-file-cleanup.json').exists() else None
            finally:m.R=previous
    def test_fixed_service_writer_route_and_deadlines(self):
        f,_=self.run_case();by={label:(args,kw) for label,args,kw in f.calls}
        args,kw=by['five-second-trace'];self.assertEqual(args[:4],['shell','-T','perfetto','--txt']);self.assertNotIn('-o',args);self.assertEqual(kw['timeout'],45)
        self.assertEqual(by['trace-file-pull'][1]['timeout'],45);self.assertTrue(all('input' not in kw for _,_,kw in f.calls))
        self.assertTrue(all('mkdir' not in args and 'rmdir' not in args for _,args,_ in f.calls))
        self.assertNotIn('-r',by['trace-owned-files-remove'][0])
    def test_foreign_session_prevents_mutation(self):self.assertEqual([c[0] for c in self.run_case('trace-state-before')[0].calls],['trace-state-before'])
    def test_fixture_collision_preserves_existing(self):
        f,c=self.run_case('fixture-collision');self.assertEqual(f.files[f.names[0]],b'keep-existing');self.assertEqual(c['exactOwnedFiles'],[])
    def test_partial_config_collision_preserves_existing(self):
        f,c=self.run_case('config-collision');self.assertEqual(f.files,{f.names[1]:b'keep-existing'});self.assertEqual(c['exactOwnedFiles'],[f.names[0]])
    def test_output_collision_is_never_deleted(self):
        f,c=self.run_case('output-collision');self.assertEqual(f.files,{f.names[2]:b'keep-existing'});self.assertFalse(c['outputOwnershipProven']);self.assertTrue(c['unclaimedOutputMayRemain'])
    def test_fixture_mismatch_stops_recording(self):self.assertNotIn('five-second-trace',[x[0] for x in self.run_case('trace-fixture-readback')[0].calls])
    def test_config_mismatch_stops_recording(self):self.assertNotIn('five-second-trace',[x[0] for x in self.run_case('trace-config-readback')[0].calls])
    def test_timeout_does_not_claim_output_ownership(self):
        _,c=self.run_case('timeout');self.assertFalse(c['outputOwnershipProven']);self.assertEqual(len(c['exactOwnedFiles']),2)
    def test_pulled_hash_mismatch_rejected(self):self.run_case('hash')
    def test_invalid_envelope_rejected(self):self.run_case('envelope')
    def test_unfinished_session_rejected(self):self.run_case('trace-state-after')
    def test_oversize_trace_rejected(self):self.run_case('oversize')
    def test_existing_destination_prevents_calls(self):self.assertEqual(self.run_case('destination')[0].calls,[])
    def test_invalid_filename_tokens_rejected(self):
        for token in ['../other','a'*31,False,'g'*32]:
            with self.assertRaises(ValueError):m.remote_paths(token)
    def test_config_tamper_prevents_calls(self):self.assertEqual(self.run_case('config-tamper')[0].calls,[])
    def test_base_tamper_prevents_calls(self):self.assertEqual(self.run_case('base-tamper')[0].calls,[])
    def test_fixture_tamper_prevents_calls(self):self.assertEqual(self.run_case('fixture-tamper')[0].calls,[])
    def test_push_failure_removes_only_reserved_fixture(self):
        f,c=self.run_case('push');self.assertEqual(f.files,{});self.assertEqual(c['exactOwnedFiles'],[f.names[0]])
    def test_cleanup_failure_prevents_acceptance(self):self.run_case('cleanup')
    def test_writer_config_preserves_fixed_base(self):
        base=(ROOT/'base-config.pbtxt').read_bytes();output=m.remote_paths('a'*32)[2]
        rendered=m.writer_config(base,output);self.assertTrue(rendered.startswith(base));self.assertEqual(rendered[len(base):],('\nwrite_into_file: true\noutput_path: "'+output+'"\n').encode())
    def test_empty_trace_rejected(self):self.run_case('empty')

if __name__=='__main__':unittest.main()
