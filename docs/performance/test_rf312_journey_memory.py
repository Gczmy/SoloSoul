import importlib.util
import json
import re
import shlex
from pathlib import Path
import subprocess
import tempfile
import threading
import time
import unittest
from unittest.mock import Mock, patch

import rf312_default_binding
import rf312_default_journey as journey
import rf312_journey_memory as memory
import rf312_instrumentation_state as state
from native_journey_contract import PHASES
from test_rf312_instrumentation_state import block

spec = importlib.util.spec_from_file_location('stream_original_memory_tests',
    Path(__file__).resolve().parents[2] / 'tauri/scripts/test_android_process_memory.py')
legacy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(legacy)


class Sdk:
    def __init__(self):
        self.running = False
        self.calls = []
        self.extra = False
        self.stderr = False
        self.exit_code = 0
        self.observer_pid = 88
        self.birth = 12698151
        self.package_reply = b'package:com.solosoul.app uid:10105\r\n'
        self.wrong_uid = False
        self.bad_meminfo = False

    def __call__(self, *args, **kwargs):
        self.calls.append((args, kwargs))
        assert 0 < kwargs['timeout'] <= 10
        assert kwargs['stdout'] == kwargs['stderr'] == subprocess.PIPE
        if args[:6] == ('shell','-T','-e','none','sh','-c'):
            script = shlex.split(args[6])[0];lines = script.splitlines()
            assert len(lines) == 8
            nonce = re.search(r'RF312PAIR ([0-9a-f]{32}) 1',lines[2])[1].encode()
            values = [self.native('shell',*shlex.split(lines[index])) for index in (0,4)]
            raw = b'';errors = b''
            for index,value in enumerate(values,1):
                raw += value.stdout + b'\0RF312PAIR '+nonce+b' '+str(index).encode()+b' '+str(value.returncode).encode()+b'\0'
                errors += value.stderr + b'\0RF312PAIR '+nonce+b' '+str(index).encode()+b' '+str(value.returncode).encode()+b'\0'
            return subprocess.CompletedProcess(args,0,raw,errors)
        return self.native(*args)

    def native(self,*args):
        if args == ('shell', 'pm', 'list', 'packages', '--uid', '10105'):
            raw = self.package_reply
        elif args == ('shell', 'ps', '-A', '-o', 'UID,PID,PPID,NAME'):
            text = 'UID PID PPID NAME\n0 1 0 init\n10080 10609 1 shared-provider\n'
            if self.running:
                text += '10105 10559 1 com.solosoul.app\n99082 10626 1 webview-renderer\n'
            if self.extra:
                text += '10105 10627 1 unregistered-helper\n'
            raw = text.encode()
        elif args == ('shell', 'dumpsys', 'activity', 'processes'):
            registry = legacy.REGISTRY if self.running else legacy.REGISTRY.splitlines()[0] + '\n'
            raw = (state.HEADER + '\n  All known processes:\n' + registry + '  Active instrumentation:\n'
                   + block(0, memory.OBSERVER, 'Observer', memory.OBSERVER, self.observer_pid)
                   + '\n  OOM levels:\n').encode()
        elif args == ('shell', 'dumpsys', 'activity', 'services', memory.APP):
            raw = (legacy.SERVICES if self.running else
                   'ACTIVITY MANAGER SERVICES (dumpsys activity services)\n  (nothing)\n').encode()
        elif args[:2] == ('shell', 'cat'):
            pid = int(args[-1].split('/')[2])
            raw = (legacy.stat(pid, self.birth) if args[-1].endswith('/stat') else
                   legacy.status(2000 if self.wrong_uid else (10105 if pid == 10559 else 99082))).encode()
        elif args[:5] == ('shell', 'dumpsys', 'meminfo', '--local', '-s'):
            pid = int(args[-1])
            raw = (legacy.meminfo(pid, memory.APP if pid == 10559 else legacy.NAME) if not self.bad_meminfo else 'invalid native memory values').encode()
        else:
            raise AssertionError('Unexpected command: ' + repr(args))
        return subprocess.CompletedProcess(args, self.exit_code, raw, b'unknown native error' if self.stderr else b'')


class StreamTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.sdk = Sdk()
        self.stream = memory.Stream(self.sdk, legacy.memory.collect_snapshot, self.root / 'trace',
            uid=10105, deadline=time.monotonic() + 20, phases=PHASES)
        self.addCleanup(self.shutdown)

    def shutdown(self):
        if not (self.stream.output / 'result.json').exists():
            try:
                self.stream.close()
            except RuntimeError:
                pass

    def point(self):
        self.sdk.running = True
        self.stream.expected_pid = 10559
        return self.stream._capture('boundary', time.monotonic() + 5, phase=PHASES[0])

    def test_complete_point_reuses_canonical_memory_and_both_real_state_proofs(self):
        point = self.point()
        self.assertTrue(point['accepted'], point.get('error'))
        self.assertEqual(point['snapshot']['pssBytes'], 204800)
        self.assertEqual(len(point['commands']), 16)
        self.assertEqual(point['nativeTransport']['nativeCalls'], 10)
        self.assertEqual(len(self.sdk.calls), 10)
        self.assertEqual(len(point['stateProofs']), 2)
        self.assertTrue(point['uidCensusScopeVerified'])
        self.assertEqual([x['role'] for x in point['snapshot']['processes']], ['app', 'renderer'])

    def test_kernel_identity_is_still_validated_before_any_meminfo(self):
        self.sdk.wrong_uid = True
        point = self.point();self.assertFalse(point['accepted'])
        self.assertFalse(any(args[:3]==('shell','dumpsys','meminfo') for args,kwargs in self.sdk.calls))

    def test_invalid_memory_values_do_not_prefetch_post_read_identity(self):
        self.sdk.bad_meminfo = True
        point = self.point();self.assertFalse(point['accepted'])
        records=[json.loads(file.read_bytes()) for file in sorted((self.root/'trace/points/001/native-sdk').glob('[0-9][0-9][0-9].json'))]
        pairs=[row for row in records if row['logicalMembers'][0]==['shell','cat','/proc/10559/stat']]
        self.assertEqual(len(pairs),1)

    def test_unused_pair_reply_is_retained_after_early_state_rejection(self):
        self.assertTrue(self.point()['accepted']);self.sdk.observer_pid+=1
        point=self.point();self.assertFalse(point['accepted'])
        self.assertEqual(len(point['nativeTransport']['unconsumedPairMembers']),1)
        member=point['nativeTransport']['unconsumedPairMembers'][0]
        self.assertEqual(member['member'],2)
        folder=self.root/'trace/points/002/native-sdk';record=json.loads((folder/member['nativeRecord']).read_bytes())
        self.assertEqual(record['logicalMembers'][1],['shell','dumpsys','activity','services',memory.APP])
        self.assertIn(legacy.SERVICES.encode(),(folder/f"{member['nativeOrdinal']:03d}.stdout").read_bytes())

    def test_failed_second_member_keeps_its_own_stderr_and_logical_identity(self):
        original=self.sdk.native
        def fail_services(*args):
            result=original(*args)
            if args==('shell','dumpsys','activity','services',memory.APP):
                return subprocess.CompletedProcess(args,7,result.stdout,b'owned service read failed')
            return result
        with patch.object(self.sdk,'native',side_effect=fail_services):point=self.point()
        self.assertFalse(point['accepted'])
        command=point['commands'][-1]
        self.assertEqual(command['command'],['shell','dumpsys','activity','services',memory.APP])
        self.assertEqual(command['exitCode'],7);self.assertTrue(command['logicalReplyAvailable'])
        self.assertEqual(command['nativeEvidence']['member'],2)
        path=self.root/f"trace/points/001/{len(point['commands']):03d}.stderr"
        self.assertEqual(path.read_bytes(),b'owned service read failed')

    def test_unregistered_native_helper_is_not_silently_omitted(self):
        self.sdk.extra = True
        point = self.point()
        self.assertFalse(point['accepted'])
        self.assertIn('unregistered', point['error'])
        self.assertTrue((self.stream.output / 'points/001/001.stdout').exists())

    def test_stderr_nonzero_and_timeout_keep_refused_raw_evidence(self):
        for field, value in [('stderr', True), ('exit_code', 7)]:
            setattr(self.sdk, field, value)
            point = self.point()
            self.assertFalse(point['accepted'])
            setattr(self.sdk, field, False if field == 'stderr' else 0)
        self.stream.sdk = Mock(side_effect=subprocess.TimeoutExpired('frozen-adb', .01, output=b'partial', stderr=b'timeout'))
        point = self.point()
        self.assertFalse(point['accepted'])
        command = point['commands'][0]
        self.assertTrue(command['timedOut'])
        self.assertEqual((self.stream.output / f"points/{point['ordinal']:03d}/001.stdout").read_bytes(), b'partial')
        self.stream.sdk.assert_called_once()

    def test_observer_restart_and_app_birth_change_are_rejected(self):
        first = self.point()
        self.assertTrue(first['accepted'])
        self.sdk.observer_pid += 1
        changed = self.point()
        self.assertFalse(changed['accepted'])
        self.sdk.observer_pid -= 1
        self.sdk.birth += 1
        changed = self.point()
        self.assertFalse(changed['accepted'])
        self.assertIn('lifetime', changed['error'])

    def test_prelaunch_proves_absence_without_recording_zero_pss(self):
        point = self.stream._capture('prelaunch', time.monotonic() + 5)
        self.assertTrue(point['accepted'], point.get('error'))
        self.assertIsNone(point['snapshot']['pssBytes'])
        self.assertEqual(point['snapshot']['state'], 'not-running')
        self.assertTrue(point['uniqueAppUidVerified'])

    def test_uid_reply_keeps_exact_package_uid_and_unique_membership(self):
        for raw in [b'package:com.solosoul.app\n', b'package:com.solosoul.app uid:10106\n',
                    b'package:other.app uid:10105\n',
                    b'package:com.solosoul.app uid:10105\npackage:other.app uid:10105\n']:
            self.sdk.package_reply = raw
            with self.subTest(raw=raw):
                point = self.stream._capture('prelaunch', time.monotonic() + 5)
                self.assertFalse(point['accepted'])

    def test_all_twelve_boundaries_finish_and_stop_worker_before_restore(self):
        self.stream.start()
        self.sdk.running = True
        self.stream.bind(10559)
        for phase in PHASES:
            value = self.stream.boundary(phase, time.monotonic() + 5)
            self.assertEqual(value['pssBytes'], 204800)
        report = self.stream.close()
        self.assertTrue(report['accepted'])
        self.assertTrue(report['workerStopped'])
        self.assertEqual(report['boundaryPhases'], list(PHASES))
        self.assertEqual(report['acknowledgedBoundaryPhases'], list(PHASES))
        self.assertEqual(report['sampledMaximumSerialPssBytes'], 204800)
        self.assertFalse(report['exactContinuousPeakAccepted'])
        self.assertFalse(report['applicationOnlyLatencyAccepted'])

    def test_periodic_observations_are_planned_and_failed_points_are_not_replaced(self):
        with patch.object(memory, 'INTERVAL', .02):
            self.stream.start()
            self.sdk.running = True
            self.stream.bind(10559)
            self.sdk.extra = True
            time.sleep(.06)
            self.sdk.extra = False
            for phase in PHASES:
                self.stream.boundary(phase, time.monotonic() + 5)
            report = self.stream.close()
        refused = [x for x in self.stream.points if x['kind'] == 'periodic' and not x['accepted']]
        self.assertGreaterEqual(len(refused), 1)
        self.assertEqual(report['refusedPoints'], sum(not x['accepted'] for x in self.stream.points))
        self.assertTrue(report['accepted'])
        self.assertTrue(all(not point['inputRetried'] for point in self.stream.points))

    def test_bad_phase_rebinding_and_incomplete_close_never_accept(self):
        self.stream.start()
        self.sdk.running = True
        self.stream.bind(10559)
        with self.assertRaises(RuntimeError):
            self.stream.bind(10559)
        with self.assertRaises(ValueError):
            self.stream.boundary(PHASES[1], time.monotonic() + 5)
        with self.assertRaises(RuntimeError):
            self.stream.close()
        self.assertFalse(json.loads((self.stream.output / 'result.json').read_bytes())['accepted'])

    def test_expired_point_sends_no_command(self):
        point = self.stream._capture('boundary', time.monotonic() - 1, phase=PHASES[0])
        self.assertFalse(point['accepted'])
        self.assertEqual(self.sdk.calls, [])

    def test_failed_boundary_is_not_retried_and_worker_stops(self):
        self.stream.start()
        self.sdk.running = True
        self.stream.bind(10559)
        self.sdk.extra = True
        with self.assertRaises(RuntimeError):
            self.stream.boundary(PHASES[0], time.monotonic() + 5)
        with self.assertRaises(RuntimeError):
            self.stream.close()
        self.assertEqual(sum(p['kind'] == 'boundary' for p in self.stream.points), 1)
        self.assertFalse(self.stream.thread.is_alive())

    def test_missed_slots_keep_each_declared_time_without_sdk_calls(self):
        self.stream.next_due = 1
        self.stream._retain_missed(7)
        self.assertEqual([p['plannedDueAt'] for p in self.stream.points], [1, 3, 5])
        self.stream._retain_missed(8, final=True)
        self.assertEqual([p['plannedDueAt'] for p in self.stream.points], [1, 3, 5, 7])
        self.assertTrue(all(p['kind'] == 'missed-periodic' and not p['accepted'] for p in self.stream.points))
        self.assertEqual(self.sdk.calls, [])

    def test_observation_cap_refuses_before_another_device_command(self):
        with patch.object(memory, 'MAX_POINTS', 1):
            self.assertTrue(self.stream._capture('prelaunch', time.monotonic() + 5)['accepted'])
            calls = len(self.sdk.calls)
            with self.assertRaisesRegex(RuntimeError, 'cap exhausted'):
                self.stream._capture('periodic', time.monotonic() + 5)
            self.assertEqual(len(self.sdk.calls), calls)


    def test_all_async_boundaries_queue_without_blocking_ui_and_are_acknowledged(self):
        self.stream.start();self.sdk.running=True;self.stream.bind(10559)
        entered=threading.Event();release=threading.Event();capture=self.stream._capture
        def blocked(kind,*args,**kwargs):
            if kind=='boundary' and kwargs.get('phase')==PHASES[0]:
                entered.set()
                if not release.wait(5):raise TimeoutError('test capture was not released')
            return capture(kind,*args,**kwargs)
        with patch.object(self.stream,'_capture',side_effect=blocked):
            try:
                markers=[self.stream.submit_boundary(phase,time.monotonic()+5) for phase in PHASES]
                self.assertTrue(entered.wait(1))
                self.assertFalse(self.stream.pending[PHASES[0]][1].done())
                self.assertEqual(self.stream.acknowledged,[])
                self.assertTrue(all(marker['uiWaitsForCapture'] is False for marker in markers))
            finally:release.set()
            receipts=self.stream.complete_boundaries()
        self.assertEqual(list(receipts),list(PHASES))
        report=self.stream.close();self.assertTrue(report['accepted'])
        self.assertEqual(report['requestedBoundaryPhases'],list(PHASES))
        self.assertTrue(all(not ack['synchronousAckRequired'] for ack in report['boundaryAcknowledgements']))

    def test_deferred_ack_checks_capture_deadline_and_keeps_original_sync_rule(self):
        self.stream.start();self.sdk.running=True;self.stream.bind(10559)
        deadline=time.monotonic()+2;marker=self.stream.submit_boundary(PHASES[0],deadline)
        point=self.stream.pending[PHASES[0]][1].result(timeout=2)
        self.assertLessEqual(point['hostFinishedAt'],deadline)
        with patch.object(self.stream,'clock',return_value=deadline+.5):
            with self.assertRaises(RuntimeError):self.stream.receive_boundary(PHASES[0])
            receipt=self.stream.receive_boundary(PHASES[0],require_timely_ack=False)
        self.assertEqual(receipt['pointOrdinal'],point['ordinal'])
        self.assertEqual(marker['captureDeadline'],deadline)

    def test_late_capture_is_refused_even_if_acknowledgement_is_deferred(self):
        self.stream.start();self.sdk.running=True;self.stream.bind(10559)
        marker=self.stream.submit_boundary(PHASES[0],time.monotonic()+2)
        point=self.stream.pending[PHASES[0]][1].result(timeout=2)
        late=memory.Future();late.set_result({**point,'hostFinishedAt':marker['captureDeadline']+.001})
        self.stream.pending[PHASES[0]]=(marker['captureDeadline'],late,marker)
        with self.assertRaises(RuntimeError):self.stream.receive_boundary(PHASES[0],require_timely_ack=False)
        self.assertEqual(self.stream.acknowledged,[])

    def test_missing_or_duplicate_async_phase_requests_cannot_complete(self):
        self.stream.start();self.sdk.running=True;self.stream.bind(10559)
        with self.assertRaises(RuntimeError):self.stream.complete_boundaries()
        self.stream.submit_boundary(PHASES[0],time.monotonic()+2)
        with self.assertRaises(ValueError):self.stream.submit_boundary(PHASES[0],time.monotonic()+2)
        with self.assertRaises(RuntimeError):self.stream.complete_boundaries()
        self.assertEqual(len(self.stream.requested),1)


class ProtocolTests(unittest.TestCase):
    def test_census_is_numeric_unfiltered_unique_and_crlf_safe(self):
        self.assertEqual(memory.census(b' UID PID PPID NAME\r\n10105 123 1 com.solosoul.app\r\n')[123]['uid'], 10105)
        for raw in [b'', b'UID PID PPID NAME\n', b'USER PID PPID NAME\n10105 123 1 app\n',
                    b'UID PID PPID NAME\nu0_a105 123 1 app\n',
                    b'UID PID PPID NAME\n10105 123 1 app\n10105 123 1 duplicate\n']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                memory.census(raw)

    def test_same_uid_children_and_extra_descendants_must_be_in_snapshot(self):
        base = {1: dict(pid=1, uid=10105, parentPid=0), 2: dict(pid=2, uid=99000, parentPid=0)}
        snapshot = dict(processes=[dict(pid=1, uid=10105), dict(pid=2, uid=99000)])
        memory.verify_census(base, snapshot, 10105)
        for child in [dict(pid=3, uid=10105, parentPid=0), dict(pid=3, uid=99000, parentPid=0), dict(pid=3, uid=2000, parentPid=1)]:
            with self.subTest(child=child), self.assertRaises(RuntimeError):
                memory.verify_census({**base, 3: child}, snapshot, 10105)

    def test_disabled_session_has_no_sdk_or_output_side_effect(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'must-not-exist'
            with memory.session(False, {}, path, uid=10105, deadline=0, phases=PHASES) as stream:
                self.assertIsNone(stream)
            self.assertFalse(path.exists())

    def test_journey_keeps_original_finish_and_remaining_budget(self):
        value = journey.Journey.__new__(journey.Journey)
        value.phases = []
        value.memory_stream = Mock()
        value.memory_stream.submit_boundary.return_value = {'ordinal': 1}
        with patch.object(journey.original.Journey, 'finish', return_value='original') as original:
            self.assertEqual(value.finish(PHASES[0], 10, {'native': True}), 'original')
        value.memory_stream.submit_boundary.assert_called_once_with(PHASES[0], 55)
        original.assert_called_once_with(PHASES[0], 10, {'native': True}, memoryBoundaryRequest={'ordinal': 1})
        with self.assertRaises(RuntimeError):
            value.finish(PHASES[1], 10, {})


if __name__ == '__main__':
    unittest.main()
