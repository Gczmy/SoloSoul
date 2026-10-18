from pathlib import Path
import json
import subprocess
import tempfile
import unittest
from unittest import mock

import rf312_default_memory as checkpoint
from test_rf312_instrumentation_state import dump


class MemoryCheckpoint(unittest.TestCase):
    def test_direct_absence_uses_two_supported_native_boundaries(self):
        result=self.collect()
        self.assertTrue(result['directAbsenceVerified'])
        self.assertEqual(result['supportedStateChecks'],2)
        commands=[call.args for call in self.adb.call_args_list]
        self.assertEqual(commands.count(('shell','dumpsys','activity','processes')),2)
        self.assertNotIn(('shell','dumpsys','activity','instrumentation'),commands)

    def test_zero_exit_unsupported_and_active_state_are_refused(self):
        for raw in [b'Unknown command: instrumentation\n',dump(True)]:
            with self.subTest(raw=raw),tempfile.TemporaryDirectory() as folder:
                def response(*args,**kwargs):
                    if args==('shell','dumpsys','activity','processes'):return subprocess.CompletedProcess(args,0,raw,b'')
                    return self.response(*args,**kwargs)
                with self.assertRaises(ValueError):checkpoint.collect(self.collector,response,Path(folder)/'out',uid=10105,pid=123)
        self.collector.assert_not_called()

    def test_zero_exit_with_stderr_cannot_supply_an_identity(self):
        self.adb.side_effect=lambda *args,**kwargs:subprocess.CompletedProcess(args,0,self.response(*args,**kwargs).stdout,b'unknown warning')
        with self.assertRaisesRegex(RuntimeError,'Unexpected SDK stderr'):self.collect()
        self.collector.assert_not_called()
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.output = Path(self.temporary.name) / 'checkpoint'
        self.clock = mock.Mock(return_value=0)
        self.adb = mock.Mock(side_effect=self.response)
        self.snapshot = dict(fullProcessScopeAccepted=True, state='complete', pssBytes=300000,
                             processes=[dict(role='app', name=checkpoint.APP, pid=123, uid=10105)])
        self.collector = mock.Mock(return_value=self.snapshot)

    def response(self, *command, **kwargs):
        responses = {
            ('shell', 'pidof', checkpoint.APP): b'123\r\n',
            ('shell', 'dumpsys', 'activity', 'processes'): dump(False),
            ('shell', 'pm', 'list', 'packages', checkpoint.OBSERVER): b'',
            ('shell', 'dumpsys', 'window', 'displays'): b'mCurrentFocus=Window{abc u0 com.solosoul.app/.MainActivity}\r\n',
        }
        return subprocess.CompletedProcess(command, 0, responses[command], b'')

    def collect(self, **kwargs):
        return checkpoint.collect(self.collector, self.adb, self.output, uid=10105, pid=123,
                                  clock=self.clock, **kwargs)

    def saved(self):
        return json.loads((self.output / 'result.json').read_text(encoding='utf-8'))

    def test_exact_existing_reader_and_result_are_preserved(self):
        result = self.collect()
        self.assertTrue(result['accepted'])
        self.assertIs(result['snapshot'], self.snapshot)
        self.assertFalse(result['wholeJourneyMemoryDistributionAccepted'])
        self.assertEqual(self.collector.call_args.args[1:], (checkpoint.APP, 10105, checkpoint.PROVIDER))
        self.assertEqual(self.adb.call_count, 8)
        self.assertTrue(all(call.args[0] == 'shell' for call in self.adb.call_args_list))
        self.assertEqual((self.output / '001.stdout').read_bytes(), b'123\r\n')

    def test_single_deadline_shrinks_each_command_without_extension(self):
        now = [0]
        self.clock.side_effect = lambda: now[0]
        def response(*args, **kwargs):
            result = self.response(*args, **kwargs)
            now[0] += 1
            return result
        self.adb.side_effect = response
        result = self.collect(timeout=9)
        self.assertEqual([c.kwargs['timeout'] for c in self.adb.call_args_list], list(range(9, 1, -1)))
        self.assertEqual(result['elapsedMs'], 8000)

    def test_late_command_is_rejected_without_retry(self):
        self.adb.side_effect = lambda *a, **k: (self.clock.configure_mock(return_value=20), self.response(*a, **k))[1]
        with self.assertRaises(TimeoutError):
            self.collect()
        self.assertEqual(self.adb.call_count, 1)
        self.collector.assert_not_called()
        self.assertFalse(self.saved()['accepted'])

    def test_timeout_partial_bytes_and_original_exception_are_retained(self):
        error = subprocess.TimeoutExpired('owned-read', 10, output=b'partial\r\n', stderr=b'error\n')
        self.adb.side_effect = error
        with self.assertRaises(subprocess.TimeoutExpired) as actual:
            self.collect()
        self.assertIs(actual.exception, error)
        self.assertEqual((self.output / '001.stdout').read_bytes(), error.stdout)
        self.assertEqual((self.output / '001.stderr').read_bytes(), error.stderr)
        self.assertEqual(self.adb.call_count, 1)
        self.assertIsNone(json.loads((self.output / 'commands.json').read_text())[0]['exitCode'])

    def test_nonzero_status_is_preserved(self):
        error = subprocess.CalledProcessError(7, 'owned-read', output=b'out', stderr=b'err')
        self.adb.side_effect = error
        with self.assertRaises(subprocess.CalledProcessError) as actual:
            self.collect()
        self.assertIs(actual.exception, error)
        self.assertEqual((self.output / '001.stderr').read_bytes(), b'err')
        self.collector.assert_not_called()

    def test_unknown_failure_propagates_and_is_not_retried(self):
        error = RuntimeError('strict reader refused binding')
        self.collector.side_effect = error
        with self.assertRaises(RuntimeError) as actual:
            self.collect()
        self.assertIs(actual.exception, error)
        self.assertEqual(self.adb.call_count, 4)
        self.assertNotIn('snapshot', self.saved())

    def test_incomplete_or_wrong_main_result_does_not_supply_zero(self):
        for change in [dict(fullProcessScopeAccepted=False), dict(state='not-running'), dict(processes=[]),
                       dict(processes=[dict(role='app', name=checkpoint.APP, pid=124, uid=10105)])]:
            with self.subTest(change=change), tempfile.TemporaryDirectory() as folder:
                self.collector.return_value = dict(self.snapshot, **change)
                with self.assertRaisesRegex(RuntimeError, 'Strict complete'):
                    checkpoint.collect(self.collector, self.adb, Path(folder) / 'checkpoint',
                                       uid=10105, pid=123, clock=self.clock)

    def test_changed_pid_instrumentation_observer_or_focus_refuses(self):
        for number, text in [(1, b'124'), (2, checkpoint.APP.encode()),
                             (2, checkpoint.OBSERVER.encode()), (3, b'package:' + checkpoint.OBSERVER.encode()),
                             (4, b'mCurrentFocus=Window{abc u0 com.other.app/.MainActivity}')]:
            with self.subTest(number=number, text=text), tempfile.TemporaryDirectory() as folder:
                calls = [0]
                def response(*a, **k):
                    calls[0] += 1
                    return subprocess.CompletedProcess(a, 0, text, b'') if calls[0] == number else self.response(*a, **k)
                with self.assertRaises((RuntimeError,ValueError)):
                    checkpoint.collect(self.collector, response, Path(folder) / 'checkpoint', uid=10105, pid=123)

    def test_identity_is_checked_again_after_collection(self):
        def collector(*args):
            self.adb.side_effect = lambda *a, **k: subprocess.CompletedProcess(a, 0, b'124', b'')
            return self.snapshot
        self.collector.side_effect = collector
        with self.assertRaisesRegex(RuntimeError, 'Current main PID'):
            self.collect()
        self.assertFalse(self.saved()['accepted'])

    def test_invalid_configuration_does_not_touch_remote_or_output(self):
        for kwargs in [dict(uid=True, pid=123), dict(uid=100, pid=123), dict(uid=10105, pid=False),
                       dict(uid=10105, pid=123, timeout=0), dict(uid=10105, pid=123, timeout=21),
                       dict(uid=10105, pid=123, timeout=float('nan'))]:
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                checkpoint.collect(self.collector, self.adb, self.output, **kwargs)
        self.adb.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_existing_evidence_is_never_overwritten(self):
        self.output.mkdir()
        (self.output / 'marker').write_bytes(b'keep')
        with self.assertRaises(FileExistsError):
            self.collect()
        self.adb.assert_not_called()
        self.assertEqual((self.output / 'marker').read_bytes(), b'keep')


if __name__ == '__main__':
    unittest.main()
