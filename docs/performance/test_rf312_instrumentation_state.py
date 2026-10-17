import unittest

import rf312_instrumentation_state as state

APP='com.solosoul.app'
COLLECTOR='org.solosoul.rf312ipc'
OBSERVER='org.solosoul.rf312observer'
ALLOW={COLLECTOR+'/.Collector':APP,OBSERVER+'/.Observer':OBSERVER}


def block(index,package,entry,target,pid):
    return f'''    Instrumentation #{index}: ActiveInstrumentation{{abc {{{package}/.{entry}}} 1 procs}}
      mClass=ComponentInfo{{{package}/{package}.{entry}}} mFinished=false
      mRunningProcesses:
        #0: ProcessRecord{{def {pid}:{target}/u0a101}}
      mTargetProcesses=[{target}]
      mTargetInfo=ApplicationInfo{{fed {target}}}
        uid=10101 flags=0x0
      mArguments=Bundle[{{public=true}}]
'''


def dump(active=True):
    value=state.HEADER+'\n  All known processes:\n    public synthetic process entries\n'
    if active:
        value+='  Active instrumentation:\n'+block(0,COLLECTOR,'Collector',APP,123)+block(1,OBSERVER,'Observer',OBSERVER,124)
    return (value+'\n  OOM levels:\n    synthetic levels\n').encode()


class InstrumentationStateTests(unittest.TestCase):
    def test_native_section_binds_exact_components_primary_targets_and_process(self):
        rows=state.parse(dump());proof=state.expected(rows,ALLOW,process_ids={APP:123})
        self.assertEqual(proof['activeCount'],2);self.assertTrue(proof['instrumentationStateAccepted'])
        self.assertFalse(proof['actualIpcAccepted']);self.assertFalse(proof['performanceSampleAccepted'])
        self.assertFalse(proof['fullRF312Accepted'])

    def test_absence_requires_complete_unfiltered_supported_section(self):
        self.assertEqual(state.parse(dump(False)),[]);state.expected([], {})
        with self.assertRaises(ValueError):state.expected(state.parse(dump()),{})

    def test_crlf_preserves_native_field_values(self):
        self.assertEqual(state.parse(dump().replace(b'\n',b'\r\n')),state.parse(dump()))

    def test_zero_exit_unknown_command_output_is_never_absence(self):
        for raw in [b'Unknown command: instrumentation\n',b'Instrumentation:\n',b'',b'Permission Denial: x\n']:
            with self.subTest(raw=raw),self.assertRaises(ValueError):state.parse(raw)

    def test_missing_duplicate_or_reordered_section_boundaries_are_rejected(self):
        raw=dump()
        for changed in [raw.replace(state.HEADER.encode(),b'other'),raw.replace(b'  OOM levels:\n',b''),
                        raw.replace(b'  All known processes:\n',b''),raw+state.HEADER.encode()+b'\n',
                        raw.replace(b'  Active instrumentation:',b'  OOM levels:\n  Active instrumentation:')]:
            with self.subTest(raw=changed),self.assertRaises(ValueError):state.parse(changed)

    def test_empty_unscoped_and_truncated_active_sections_are_rejected(self):
        for changed in [dump().replace(b'  Active instrumentation:\n',b''),
                        dump(False).replace(b'  OOM levels:',b'  Active instrumentation:\n  OOM levels:'),
                        dump()[:100]]:
            with self.assertRaises(ValueError):state.parse(changed)

    def test_finished_instrumentation_is_not_a_live_allowed_session(self):
        for changed in [dump().replace(b'mFinished=false',b'mFinished=true'),
                        dump().replace(b'} 1 procs}',b'} FINISHED 1 procs}')]:
            with self.assertRaises(ValueError):state.parse(changed)

    def test_native_component_and_target_disagreement_is_rejected(self):
        for changed in [dump().replace(b'mClass=ComponentInfo{org.solosoul.rf312ipc/org.solosoul.rf312ipc.Collector}',b'mClass=ComponentInfo{evil/evil.Other}'),
                        dump().replace(b'mTargetProcesses=[com.solosoul.app]',b'mTargetProcesses=[*]'),
                        dump().replace(b'mTargetInfo=ApplicationInfo{fed com.solosoul.app}',b'mTargetInfo=ApplicationInfo{fed other.app}')]:
            with self.assertRaises(ValueError):state.parse(changed)

    def test_inconsistent_count_index_or_duplicate_component_rejected(self):
        for changed in [dump().replace(b'1 procs}',b'2 procs}'),dump().replace(b'Instrumentation #1:',b'Instrumentation #2:'),
                        dump().replace(b'org.solosoul.rf312observer/org.solosoul.rf312observer.Observer',b'org.solosoul.rf312ipc/org.solosoul.rf312ipc.Collector'),
                        dump().replace(b'124:',b'123:')]:
            with self.assertRaises(ValueError):state.parse(changed)

    def test_exact_allowlist_rejects_missing_extra_and_replaced_process(self):
        rows=state.parse(dump())
        for components,pids in [({COLLECTOR+'/.Collector':APP},None),({**ALLOW,'evil.app/.Other':'evil.app'},None),
                                (ALLOW,{APP:999}),(ALLOW,{APP:True})]:
            with self.assertRaises(ValueError):state.expected(rows,components,process_ids=pids)

    def test_duplicate_alias_and_implicit_component_are_rejected(self):
        with self.assertRaises(ValueError):state.expected([],{'org.foo/.Bar':'org.foo','org.foo/org.foo.Bar':'org.foo'})
        for raw in [None,'implicit',True]:
            with self.assertRaises(ValueError):state.component(raw)

    def test_wrong_types_control_characters_and_size_overflow_rejected(self):
        for raw in [None,dump().decode(),b'\xff',dump()+b'\0',dump()+b'\r',b'x'*(4*1024*1024+1)]:
            with self.assertRaises(ValueError):state.parse(raw)

    def test_capture_uses_supported_fixed_command_and_saves_raw_before_validation(self):
        events=[];calls=[]
        def sdk(argv,timeout):calls.append((argv,timeout));return 0,dump(),b''
        rows,_=state.capture(sdk,45,events.append,clock=lambda:0)
        self.assertEqual(len(rows),2);self.assertEqual(calls,[(['shell','dumpsys','activity','processes'],5)])
        self.assertEqual(events[0]['stdout'],dump())

    def test_late_nonzero_and_stderr_results_remain_failures(self):
        now=[0];events=[]
        def sdk(argv,timeout):now[0]=46;return 0,dump(),b''
        with self.assertRaises(ValueError):state.capture(sdk,45,events.append,clock=lambda:now[0])
        self.assertEqual(len(events),1)
        for response in [(1,dump(),b''),(0,dump(),b'error'),(False,dump(),b''),(0,b'Unknown command: instrumentation\n',b'')]:
            with self.subTest(response=response),self.assertRaises(ValueError):
                state.capture(lambda argv,timeout:response,45,events.append,clock=lambda:0)

    def test_invalid_or_extended_deadline_cannot_start_sdk_query(self):
        for deadline in [True,46,0,float('nan'),float('inf')]:
            with self.assertRaises(ValueError):state.capture(lambda *a,**k:self.fail('SDK must not run'),deadline,lambda row:None,clock=lambda:0)


if __name__=='__main__':unittest.main()
