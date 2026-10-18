from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
import diagnostic_journey as journey
import native_collector as native
from test_native_collector import SDK,Clock,ready,RUN,APK,COLLECTOR


def terminal():
    text=''
    for index,stage in enumerate(journey.PHASES,1):
        text+='INSTRUMENTATION_STATUS: completedStages='+str(index)+'\n'
        text+='INSTRUMENTATION_STATUS: stage='+stage+'\nINSTRUMENTATION_STATUS_CODE: 0\n'
    return text+'INSTRUMENTATION_RESULT: collectorPassed=true\nINSTRUMENTATION_RESULT: completedStages=12\nINSTRUMENTATION_RESULT: runId='+RUN+'\nINSTRUMENTATION_CODE: -1\n'


class DiagnosticJourneyTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)

    def test_terminal_requires_complete_native_sequence_and_custom_success(self):
        text=terminal();journey.terminal(text,RUN);journey.terminal(text.replace('\n','\r\n'),RUN)
        for changed in [text.replace('completedStages=12','completedStages=11'),
                        text.replace('INSTRUMENTATION_CODE: -1','INSTRUMENTATION_CODE: 0'),
                        text+'INSTRUMENTATION_RESULT: collectorPassed=true\n',
                        text.replace('stage=startup','stage=wrong'),text.replace(RUN,'b'*32),text+'Process crashed\n']:
            with self.subTest(text=changed),self.assertRaises(RuntimeError):journey.terminal(changed,RUN)

    def test_active_instrumentation_requires_exact_two_allowlisted_components(self):
        from test_rf312_instrumentation_state import dump
        good=dump().decode();journey.active_instrumentations(good)
        for text in ['', 'Unknown command: instrumentation\n',good.replace('mFinished=false','mFinished=true'),
                     good.replace('org.solosoul.rf312ipc','other.package')]:
            with self.subTest(text=text),self.assertRaises(ValueError):journey.active_instrumentations(text)

    def test_live_process_is_accepted_and_early_death_never_accepted(self):
        process=SimpleNamespace(poll=lambda:None,returncode=None)
        self.assertTrue(journey.available(process,None,self.root,RUN))
        process.poll=lambda:0;process.returncode=0
        self.assertFalse(journey.available(process,SimpleNamespace(attempted=11),self.root,RUN))

    def test_final_exit_only_accepted_with_exact_bound_terminal_and_empty_stderr(self):
        process=SimpleNamespace(poll=lambda:0,returncode=0);channel=SimpleNamespace(attempted=12)
        (self.root/'instrumentation.stdout').write_text(terminal(),encoding='utf-8')
        (self.root/'instrumentation.stderr').write_bytes(b'')
        self.assertTrue(journey.available(process,channel,self.root,RUN))
        (self.root/'instrumentation.stderr').write_bytes(b'error')
        self.assertFalse(journey.available(process,channel,self.root,RUN))
        (self.root/'instrumentation.stderr').write_bytes(b'')
        (self.root/'instrumentation.stdout').write_text(terminal().replace('collectorPassed=true','collectorPassed=false'))
        self.assertFalse(journey.available(process,channel,self.root,RUN))

    def test_sdk_return_after_deadline_cannot_become_native_evidence(self):
        sdk=SDK();clock=Clock()
        client=native.Collector(sdk,lambda:True,self.root/'metadata',ready(),run_id=RUN,
            target_apk_sha256=APK,collector_apk_sha256=COLLECTOR,clock=clock,sleep=clock.sleep)
        def late(argv,timeout):clock.value=46;return (0,b'',b'')
        client.run_sdk=late
        with self.assertRaises(ValueError):client.capture('startup',45)
        self.assertEqual(client.records,[])
        self.assertEqual(client.attempted,1)

    def test_aggregate_deadline_is_not_reset_by_later_phase_deadline(self):
        sdk=SDK();clock=Clock()
        client=native.Collector(sdk,lambda:True,self.root/'metadata',ready(),run_id=RUN,
            target_apk_sha256=APK,collector_apk_sha256=COLLECTOR,clock=clock,sleep=clock.sleep)
        clock.value=241
        with self.assertRaises(ValueError):client.capture('startup',286)
        self.assertEqual(sdk.calls,[])


if __name__=='__main__':unittest.main()
