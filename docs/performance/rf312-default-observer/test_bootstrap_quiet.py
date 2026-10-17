import copy
import unittest

from bootstrap_quiet import wait_for_quiet


class BootstrapQuiet(unittest.TestCase):
    def setUp(self):
        self.now = 0
        self.server = dict(Name='adb.exe', ProcessId=10, ParentProcessId=5,
                           CreationDate='2026-01-01T00:00:00+00:00',
                           ExecutablePath='X:/sdk/adb.exe', CommandLine='owned server')
        self.emulator = dict(self.server, Name='emulator.exe', ProcessId=11,
                             ExecutablePath='X:/sdk/emulator.exe', CommandLine='owned emulator')
        self.rows = [self.server, self.emulator]
        self.helper = dict(self.server, ProcessId=12, CommandLine='unverified shell logcat -G 2M')
        self.events = []
        self.budgets = []

    def snapshot(self, *, timeout):
        self.budgets.append(timeout)
        return copy.deepcopy(self.rows)

    def sleep(self, duration):
        self.now += duration

    def wait(self, snapshot=None, deadline=20, record=None):
        return wait_for_quiet(snapshot or self.snapshot, self.server, self.emulator, deadline,
                              record or (lambda n, row: self.events.append(row)),
                              clock=lambda: self.now, sleep=self.sleep)

    def test_two_transient_empty_samples_are_not_enough(self):
        self.assertEqual(self.wait(), self.rows)
        self.assertEqual(self.now, 5)
        self.assertEqual(len(self.events), 11)
        self.assertTrue(self.events[-1]['accepted'])
        self.assertFalse(any(r['accepted'] for r in self.events[:-1]))

    def test_late_helper_resets_quiet_window_without_accepting_ownership(self):
        def snapshot(**kwargs):
            rows = self.snapshot(**kwargs)
            return rows + [self.helper] if 1 <= self.now < 2 else rows
        self.wait(snapshot)
        self.assertEqual(self.now, 7)
        self.assertTrue(any(r['helpers'] for r in self.events))
        self.assertFalse(any(r['helperOwnershipGranted'] for r in self.events))

    def test_unknown_persistent_helper_exhausts_original_deadline(self):
        self.rows.append(self.helper)
        with self.assertRaises(TimeoutError):
            self.wait(deadline=3)
        self.assertEqual(self.now, 3)
        self.assertFalse(any(r['accepted'] for r in self.events))

    def test_missing_or_reused_root_is_refused_immediately(self):
        for changed in [[], [dict(self.server, CreationDate='2026-01-02T00:00:00+00:00'), self.emulator],
                        self.rows + [self.server]]:
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                self.wait(lambda **kwargs: changed)
        self.assertEqual(self.events, [])

    def test_callback_failure_keeps_original_exception(self):
        error = RuntimeError('CIM read refused')
        def failed(**kwargs):
            raise error
        with self.assertRaises(RuntimeError) as actual:
            self.wait(failed)
        self.assertIs(actual.exception, error)
        self.assertEqual(self.events, [])

    def test_slow_snapshot_cannot_extend_original_deadline(self):
        def snapshot(**kwargs):
            self.now = 20
            return self.rows
        with self.assertRaises(TimeoutError):
            self.wait(snapshot)
        self.assertEqual(self.events, [])

    def test_each_snapshot_uses_remaining_original_budget(self):
        self.wait(deadline=7)
        self.assertEqual(self.budgets, [7, 6.5, 6, 5.5, 5, 4.5, 4, 3.5, 3, 2.5, 2])

    def test_recording_failure_never_returns_acceptance(self):
        error = OSError('proof unavailable')
        def failed(*args):
            raise error
        with self.assertRaises(OSError) as actual:
            self.wait(record=failed)
        self.assertIs(actual.exception, error)

    def test_invalid_deadline_or_incomplete_root_precedes_snapshot(self):
        for deadline in [0, -1, 241, float('inf'), float('nan'), True]:
            with self.subTest(deadline=deadline), self.assertRaises(ValueError):
                self.wait(deadline=deadline)
        self.server['ExecutablePath'] = None
        with self.assertRaises(ValueError):
            self.wait()
        self.server['ExecutablePath'] = 'X:/sdk/adb.exe'
        del self.server['ParentProcessId']
        with self.assertRaises(ValueError):
            self.wait()
        self.assertEqual(self.budgets, [])


if __name__ == '__main__':
    unittest.main()
