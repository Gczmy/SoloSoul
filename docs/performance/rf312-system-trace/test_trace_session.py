from pathlib import Path
import hashlib
import json
import subprocess
import tempfile
import types
import unittest
from unittest.mock import patch

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
        self.atrace_reads=0
        self.trace=field(1,b'public-native-packet')
    def __call__(self,label,args,*,timeout):
        self.calls.append((label,args,timeout));out=err=b'';code=0
        if label.startswith('session-state-'):
            count=1 if self.active else 0
            if self.fault=='foreign-before' and label=='session-state-before':count=1
            if self.fault=='wrong-active' and label=='session-state-active':count=2
            out=state(count)
        elif label.startswith('session-atrace-'):
            mask=64 if self.active else 0
            if label.endswith('-active'):
                self.atrace_reads+=1
                if self.fault=='atrace-late-active' and self.atrace_reads<3:mask=0
            if self.fault=='atrace-before' and label.endswith('-before'):mask=32
            if self.fault=='atrace-after' and label.endswith('-after'):mask=64
            if self.fault=='atrace-active' and label.endswith('-active'):mask=0
            count=1 if self.fault=='atrace-empty-slot' and label.endswith('-active') else 0
            out=(str(mask)+'\n'+str(count)+'\n\n'+str(count)+'\n'+str(mask)+'\n').encode()
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
    def case(self,fault=None,profile='default'):
        folder=tempfile.TemporaryDirectory();self.addCleanup(folder.cleanup)
        root=Path(folder.name);inputs=root/'inputs';inputs.mkdir();out=root/'out';out.mkdir()
        base=(ROOT/'base-config.pbtxt').read_bytes();names=m.transport.remote_paths('c'*32)
        (inputs/'base-config.pbtxt').write_bytes(base)
        (inputs/'file-plan.json').write_text(json.dumps({'nonce':'c'*32}),encoding='utf-8')
        (inputs/'roundtrip.bin').write_bytes(m.transport.ROUNDTRIP)
        (inputs/'trace-config.pbtxt').write_bytes(m.config(base,names[2],profile=profile))
        device=Device(names,fault);session=m.Session(device,inputs,out,profile=profile)
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

    def test_default_configuration_golden_and_no_atrace_queries(self):
        base=(ROOT/'base-config.pbtxt').read_bytes()
        self.assertEqual(hashlib.sha256(m.config(base,m.transport.remote_paths('d'*32)[2])).hexdigest(),
                         'd92de1d19cbd53d52a1ce3d2894c4a54e1ca268f198d0ae8e242deb9ec806081')
        s,d,_,_=self.case();s.start();s.finish()
        self.assertFalse(any(label.startswith('session-atrace-')for label,_,_ in d.calls))

    def test_startup_am_profile_binds_tags_and_disables_after_stop(self):
        s,d,out,_=self.case(profile='startup-am');s.start();result=s.finish()
        self.assertEqual(result['traceProfile'],'startup-am')
        self.assertEqual({k:v['tagMask']for k,v in result['atraceStates'].items()},dict(before=0,active=64,after=0))
        self.assertEqual(sum(label.startswith('session-atrace-')for label,_,_ in d.calls),3)
        self.assertEqual(d.files,{})
        self.assertTrue(json.loads((out/'session-cleanup.json').read_bytes())['passed'])

    def test_occupied_atrace_before_start_never_stops_foreign_state(self):
        s,d,out,_=self.case('atrace-before',profile='startup-am')
        with self.assertRaises(ValueError):s.start()
        self.assertNotIn('session-start',[label for label,_,_ in d.calls])
        self.assertNotIn('session-stop',[label for label,_,_ in d.calls])
        self.assertEqual(d.files,{})

    def test_inactive_am_profile_is_refused_and_owned_session_restored(self):
        s,d,out,_=self.case('atrace-active',profile='startup-am')
        with patch.object(m.time,'sleep'),self.assertRaises(ValueError):s.start()
        self.assertFalse(d.active);self.assertEqual(d.files,{})
        self.assertTrue(json.loads((out/'session-cleanup.json').read_bytes())['passed'])

    def test_atrace_not_disabled_after_stop_refuses_trace_without_retry(self):
        s,d,out,_=self.case('atrace-after',profile='startup-am');s.start()
        with self.assertRaises(ValueError):s.finish()
        self.assertEqual(sum(label=='session-stop'for label,_,_ in d.calls),1)
        self.assertFalse((out/'session-transport.json').exists())
        self.assertFalse(json.loads((out/'session-cleanup.json').read_bytes())['passed'])

    def test_profile_cannot_silently_consume_other_configuration(self):
        s,d,out,inputs=self.case(profile='startup-am')
        with self.assertRaises(ValueError):m.Session(d,inputs,out)
        self.assertEqual(d.calls,[])

    def test_unknown_profile_refused_without_custom_config_injection(self):
        base=(ROOT/'base-config.pbtxt').read_bytes()
        for profile in (None,True,'unknown','am\natrace_apps: "*"'):
            with self.subTest(profile=profile),self.assertRaises(ValueError):
                m.config(base,m.transport.remote_paths('d'*32)[2],profile=profile)

    def test_atrace_property_parser_refuses_missing_extra_foreign_or_invalid_values(self):
        s,d,_,_=self.case(profile='startup-am')
        for raw in (b'0\n',b'0\n0\nextra\n',b'bad\n0\n\n0\n0\n',b'-1\n0\n\n0\n0\n',
                    b'0\n1\n\n1\n0\n',b'64\n0\n\n0\n64\n',b'0'*257+b'\n0\n\n0\n0\n'):
            s.raw_call=lambda *args,**kwargs:types.SimpleNamespace(returncode=0,stdout=raw,stderr=b'')
            with self.subTest(raw=raw),self.assertRaises(ValueError):s._atrace_state('before',0)
        self.assertEqual(d.calls,[])

    def test_disabled_unset_and_hex_property_forms_are_read_only_equivalents(self):
        s,_,_,_=self.case(profile='startup-am');calls=[]
        for stage,raw,mask in (('before',b'\n\n\n\n\n',0),('active',b'0x40\n0\n\n0\n0x40\n',64),
                               ('after',b'0x0\r\n0\r\n\r\n0\r\n0x0\r\n',0)):
            def call(label,args,*,timeout):
                calls.append(args)
                return types.SimpleNamespace(returncode=0,stdout=raw,stderr=b'')
            s.raw_call=call;s._atrace_state(stage,mask)
        self.assertEqual({k:v['tagMask']for k,v in s.atrace_states.items()},dict(before=0,active=64,after=0))
        self.assertTrue(all(args[:3]==['exec-out','sh','-c']and 'setprop'not in args[-1]for args in calls))

    def test_delayed_am_readiness_preserves_zero_probes_and_requires_real_64(self):
        s,d,_,_=self.case('atrace-late-active',profile='startup-am');s.start();result=s.finish()
        self.assertTrue(result['atraceReadiness']['withinBudget'])
        self.assertEqual(len(result['atraceReadiness']['disabledProbesRetained']),2)
        self.assertTrue(all(v['tagMask']==0 for v in result['atraceReadiness']['disabledProbesRetained']))
        self.assertEqual(result['atraceStates']['active']['tagMask'],64)
        self.assertTrue(all(0<timeout<=5 for label,_,timeout in d.calls if label=='session-atrace-active'))

    def test_ready_reply_after_total_budget_is_not_accepted(self):
        s,d,out,_=self.case(profile='startup-am')
        with patch.object(m.time,'monotonic',side_effect=[0,0.1,5.1]),self.assertRaises(ValueError):s.start()
        self.assertFalse(s.atrace_ready);self.assertFalse(d.active);self.assertEqual(d.files,{})
        self.assertEqual(sum(label=='session-stop'for label,_,_ in d.calls),1)

    def test_only_active_profile_reads_can_use_a_shorter_remaining_timeout(self):
        s,d,_,_=self.case(profile='startup-am')
        for label,timeout in (('session-state-before',4.0),('session-atrace-active',5.1),
                              ('session-atrace-active',45),('session-atrace-active',0),('session-atrace-active',True)):
            with self.subTest(label=label,timeout=timeout),self.assertRaises(ValueError):s.call(label,[],timeout=timeout)
        self.assertEqual(d.calls,[])

    def test_single_empty_app_slot_is_verified_without_hiding_the_raw_count(self):
        s,d,out,_=self.case('atrace-empty-slot',profile='startup-am');s.start();result=s.finish()
        active=result['atraceStates']['active']
        self.assertEqual(active['appCount'],1)
        self.assertEqual(active['appCountReadbacks'],[1,1])
        self.assertTrue(active['emptyAppEntryVerified']);self.assertTrue(active['stableReadback'])
        self.assertEqual(result['atraceStates']['before']['appCount'],0)
        self.assertEqual(result['atraceStates']['after']['appCount'],0)
        self.assertTrue(json.loads((out/'session-cleanup.json').read_bytes())['passed'])

    def test_actual_app_patterns_counts_and_unproven_old_replies_remain_refused(self):
        s,_,_,_=self.case(profile='startup-am')
        raws=[b'0x40\n1\n'+entry+b'\n1\n0x40\n'
              for entry in (b'*',b'com.solosoul.app',b'com.*',b'?',b' ',b'\0')]
        raws.extend((b'64\n2\n\n2\n64\n',b'64\n1\n',b'64\n0\n',b'64\n0\n\n1\n64\n'))
        for raw in raws:
            s.raw_call=lambda *args,**kwargs:types.SimpleNamespace(returncode=0,stdout=raw,stderr=b'')
            with self.subTest(raw=raw),self.assertRaises(ValueError):s._atrace_state('active',64)

    def test_empty_slot_does_not_relax_original_zero_count_before_or_after(self):
        s,_,_,_=self.case(profile='startup-am')
        s.raw_call=lambda *args,**kwargs:types.SimpleNamespace(returncode=0,stdout=b'0\n1\n\n1\n0\n',stderr=b'')
        for stage in ('before','after'):
            with self.subTest(stage=stage),self.assertRaises(ValueError):s._atrace_state(stage,0)

    def test_transition_during_property_read_is_pending_until_fresh_stable_64(self):
        s,_,_,_=self.case(profile='startup-am')
        for raw in (b'0\n0\n\n1\n64\n',b'64\n0\n\n1\n64\n',b'0\n1\n\n1\n0\n'):
            s.raw_call=lambda *args,**kwargs:types.SimpleNamespace(returncode=0,stdout=raw,stderr=b'')
            self.assertFalse(s._atrace_state('active',64,allow_starting=True))
            self.assertNotIn('active',s.atrace_states)
        s.raw_call=lambda *args,**kwargs:types.SimpleNamespace(returncode=0,stdout=b'64\n1\n\n1\n64\n',stderr=b'')
        self.assertTrue(s._atrace_state('active',64,allow_starting=True))
        self.assertEqual(len(s.atrace_pending),3)

    def test_unknown_tag_or_app_filter_is_rejected_even_while_starting(self):
        s,_,_,_=self.case(profile='startup-am')
        for raw in (b'32\n0\n\n0\n64\n',b'0\n1\n*\n1\n0\n',b'0\n2\n\n2\n0\n'):
            s.raw_call=lambda *args,**kwargs:types.SimpleNamespace(returncode=0,stdout=raw,stderr=b'')
            with self.subTest(raw=raw),self.assertRaises(ValueError):s._atrace_state('active',64,allow_starting=True)

if __name__=='__main__':unittest.main()
