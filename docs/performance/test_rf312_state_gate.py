from pathlib import Path
import hashlib,json,subprocess,tempfile,time,unittest
from unittest.mock import patch
import rf312_state_gate as gate
import rf312_instrumentation_state as state
from test_rf312_instrumentation_state import dump


class StateGateTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.calls=[]

    def sdk(self,*args,**kwargs):
        self.calls.append((args,kwargs));return subprocess.CompletedProcess(args,0,dump(False),b'')

    def test_real_raw_bytes_and_summary_are_bound_before_acceptance(self):
        value=gate.Gate(self.sdk,self.root);proof=value.check({},deadline=time.monotonic()+10)
        raw=(value.output/'001.stdout').read_bytes();self.assertEqual(raw,dump(False))
        self.assertEqual(proof['stdoutSha256'],hashlib.sha256(raw).hexdigest())
        self.assertEqual(self.calls[0][0],('shell','dumpsys','activity','processes'))
        self.assertEqual(proof['activeCount'],0);self.assertEqual(value.proof,[proof])

    def test_existing_output_and_failed_proofs_are_not_overwritten(self):
        value=gate.Gate(self.sdk,self.root)
        with self.assertRaises(FileExistsError):gate.Gate(self.sdk,self.root)
        with self.assertRaises(ValueError):value.check({'org.foo/.Other':'org.foo'},deadline=time.monotonic()+10)
        self.assertEqual(value.proof,[]);self.assertTrue((value.output/'001.stdout').is_file())
        self.assertFalse((value.output/'001.proof.json').exists())

    def test_sdk_error_and_raw_zero_exit_error_body_are_preserved(self):
        for code,out,err in [(7,b'partial',b'error'),(0,b'Unknown command: instrumentation\n',b'')]:
            with self.subTest(code=code),tempfile.TemporaryDirectory() as folder:
                def sdk(*args,**kwargs):
                    if code:raise subprocess.CalledProcessError(code,args,output=out,stderr=err)
                    return subprocess.CompletedProcess(args,code,out,err)
                value=gate.Gate(sdk,folder)
                with self.assertRaises(ValueError):value.check({},deadline=time.monotonic()+10)
                self.assertEqual((value.output/'001.stdout').read_bytes(),out)
                self.assertEqual((value.output/'001.stderr').read_bytes(),err)
                self.assertEqual(value.proof,[])

    def test_late_response_is_saved_and_never_becomes_an_accepted_proof(self):
        now=[0];real=state.capture
        def sdk(*args,**kwargs):now[0]=46;return subprocess.CompletedProcess(args,0,dump(False),b'')
        value=gate.Gate(sdk,self.root)
        with patch.object(state,'capture',side_effect=lambda sdk,deadline,record:real(sdk,deadline,record,clock=lambda:now[0])):
            with self.assertRaises(ValueError):value.check({},deadline=45)
        self.assertTrue((value.output/'001.stdout').is_file());self.assertFalse(value.proof)
