from pathlib import Path
import importlib.util
import json
import subprocess
import tempfile
import types
import unittest
from unittest.mock import patch

import trace_journey as m
import trace_binding

PHASES=('startup','password-unlock','first-ocr-user-flow','workspace','search-needle','attachments-open',
        'image-preview','text-preview','pdf-text-native','pdf-scanned-native','application-lock','password-reunlock')

class TraceAdapter(unittest.TestCase):
    def test_raw_success_logs_exact_output(self):
        with tempfile.TemporaryDirectory() as folder:
            result=types.SimpleNamespace(returncode=0,stdout=b'\0\xff\r\n',stderr=b'log\n')
            call=m.TraceCalls(lambda *a,**kw:result,Path(folder))
            self.assertIs(call('public',['exec-out','public'],timeout=5),result)
            self.assertEqual((Path(folder)/'0001-public.stdout').read_bytes(),result.stdout)
            self.assertEqual(json.loads((Path(folder)/'commands.json').read_bytes())[0]['exitCode'],0)
    def test_failure_preserves_actual_returncode_and_streams(self):
        with tempfile.TemporaryDirectory() as folder:
            def failed(*args,**kwargs):raise subprocess.CalledProcessError(2,args,b'',b'raw error')
            call=m.TraceCalls(failed,Path(folder))
            with self.assertRaises(subprocess.CalledProcessError):call('public',[],timeout=5)
            self.assertEqual((Path(folder)/'0001-public.stderr').read_bytes(),b'raw error')
            self.assertEqual(call.rows[0]['exitCode'],2)
    def test_timeout_retains_available_not_fabricated_streams(self):
        with tempfile.TemporaryDirectory() as folder:
            def failed(*args,**kwargs):raise subprocess.TimeoutExpired(args,5,output=b'raw\0')
            call=m.TraceCalls(failed,Path(folder))
            with self.assertRaises(subprocess.TimeoutExpired):call('public',[],timeout=5)
            self.assertEqual((Path(folder)/'0001-public.partial.stdout').read_bytes(),b'raw\0')
            self.assertFalse((Path(folder)/'0001-public.partial.stderr').exists())
    def flow(self,*,changed_birth=False,missing_phase=False,early_failure=False):
        temp=tempfile.TemporaryDirectory();self.addCleanup(temp.cleanup);out=Path(temp.name)
        calls=[];saved=[];identity_count=0
        class BaseJourney:
            def __init__(self):self.proof=out/'default-external';self.proof.mkdir()
            def finish(self,name,started,row,**kw):calls.append(name)
            def field(self,password,deadline):return {'bounds':'[1,1][2,2]'}
            def action_candidates(self,pattern):return []
            def unique(self,candidates,deadline):return {'bounds':'[3,3][4,4]'}
            def remaining(self,deadline):return 1
        def save(path,value):
            saved.append(path.name)
            with path.open('x',encoding='utf-8') as f:json.dump(value,f)
        fake_base=types.SimpleNamespace(Journey=BaseJourney,save=save,PHASES=PHASES)
        def observe(*args,**kw):
            j=fake_base.Journey()
            fake_base.save(j.proof/'observed-process-start.json',{'pid':123,'uid':10101})
            if early_failure:raise RuntimeError('original phase failed')
            import time
            for phase in PHASES[:-1] if missing_phase else PHASES:j.finish(phase,time.monotonic()-0.01,{})
            fake_base.save(j.proof/'measured-workload-window.json',{})
            return {'success':True}
        fake_base.observe=observe
        def kernel(stat,status,expected):
            nonlocal identity_count
            identity_count+=1
            return dict(expected,startTicks=99 if changed_birth and identity_count>2 else 88)
        memory=types.SimpleNamespace(kernel_identity=kernel)
        class Session:
            def __init__(self,*args):self.cleanup_recorded=False
            def start(self):calls.append('trace-start')
            def finish(self):
                self.cleanup_recorded=True;calls.append('trace-stop')
                return dict(traceSha256='a'*64,hostMonotonicBrackets=dict(activeAfterNs=0,stopBeforeNs=10**22))
        def adb(*args,**kw):
            raw=b'com.solosoul.app\0' if args[-1].endswith('/cmdline') else b'public'
            if args==('shell','getconf','CLK_TCK'):raw=b'100\n'
            return types.SimpleNamespace(returncode=0,stdout=raw,stderr=b'')
        source=m.PUBLIC/'rf312_default_journey.py'
        memory_file=out/'tauri/scripts/android-process-memory.py';memory_file.parent.mkdir(parents=True);memory_file.write_bytes(b'public-memory-reader')
        plan=dict(inputCases={'100':{'root':str(out)}})
        inputs=dict(host={'workspaceRoot':str(out)},canonicalMemoryReaderSha256=m.sha(memory_file.read_bytes()))
        def load_module(name,path):return fake_base if Path(path)==source else memory
        with patch.object(m.trace_binding,'verify'),patch.object(m,'module',side_effect=load_module),patch.object(m.trace_session,'Session',Session):
            try:result=m.observe(adb,None,out,100,10101,'public',inputs,trace_inputs=plan)
            except (RuntimeError,ValueError) as error:return out,calls,error
        return out,calls,result
    def test_twelve_phases_bind_13_identities_and_keep_acceptance_limits(self):
        out,calls,result=self.flow()
        self.assertTrue(result['systemTraceHostBracketsVerified'])
        proof=json.loads((out/'system-trace/whole-workload-binding.json').read_bytes())
        self.assertEqual(proof['identityCheckpoints'],13)
        self.assertEqual([p['name'] for p in proof['phases']],list(PHASES))
        self.assertFalse(proof['traceContentAttributionAccepted']);self.assertFalse(proof['fullRF312Accepted'])
        self.assertEqual(calls[0],'trace-start');self.assertEqual(calls[-1],'trace-stop')
    def test_reused_pid_changed_birth_stops_and_refuses(self):
        out,calls,result=self.flow(changed_birth=True)
        self.assertIsInstance(result,ValueError);self.assertEqual(calls[-1],'trace-stop')
        self.assertFalse((out/'system-trace/whole-workload-binding.json').exists())
    def test_incomplete_journey_not_accepted(self):
        out,calls,result=self.flow(missing_phase=True)
        self.assertIsInstance(result,ValueError);self.assertEqual(calls[-1],'trace-stop')
    def test_original_workload_error_is_preserved_after_stop(self):
        out,calls,result=self.flow(early_failure=True)
        self.assertIsInstance(result,RuntimeError);self.assertEqual(str(result),'original phase failed')
        self.assertEqual(calls[-1],'trace-stop')
    def test_duplicate_binding_json_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'plan.json';path.write_bytes(b'{"schemaVersion":1,"schemaVersion":1}')
            with self.assertRaises(ValueError):trace_binding.load(path)

if __name__=='__main__':unittest.main()
