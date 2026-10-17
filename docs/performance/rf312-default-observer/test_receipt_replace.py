import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

import adb_control


class ReceiptReplace(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.output = Path(self.temporary.name)
        self.recorder = adb_control.AdbRecorder(sys.executable, 'emulator-5578', self.output)
        self.completed = subprocess.CompletedProcess([], 0, b'public-output\n', b'')

    @staticmethod
    def denied(code=5):
        error = PermissionError('public metadata replacement denied')
        error.winerror = code
        return error

    def run_command(self, timeout=1):
        return self.recorder.run('shell', 'pidof', 'com.solosoul.app', timeout=timeout,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def test_known_denial_retries_only_identical_local_payload_not_sdk(self):
        original = Path.replace
        payloads = []
        def replace(path, target):
            payloads.append(path.read_bytes())
            if len(payloads) < 3:
                raise self.denied(32)
            return original(path, target)
        with mock.patch.object(adb_control.subprocess, 'run', return_value=self.completed) as sdk, mock.patch.object(Path, 'replace', replace):
            self.assertIs(self.run_command(), self.completed)
        sdk.assert_called_once()
        self.assertEqual(len(payloads), 3)
        self.assertEqual(len(set(payloads)), 1)
        rows = json.loads((self.output / 'commands.json').read_text())
        self.assertEqual(rows, self.recorder.commands)
        self.assertEqual(len(rows), 1)
        events = [json.loads(x) for x in (self.output / 'commands-replace-events.jsonl').read_text().splitlines()]
        self.assertEqual([x['replaceAttempt'] for x in events], [1, 2])
        self.assertTrue(all(x['winerror'] == 32 and x['commandOrdinal'] == 1 for x in events))

    def test_unknown_or_non_windows_permission_denial_is_not_retried(self):
        for error in [PermissionError('ordinary denial'), self.denied(87), OSError('disk failure')]:
            with self.subTest(error=error), mock.patch.object(adb_control.subprocess, 'run', return_value=self.completed) as sdk, mock.patch.object(Path, 'replace', side_effect=error) as replace:
                with self.assertRaises(type(error)) as actual:
                    self.run_command()
                self.assertIs(actual.exception, error)
                sdk.assert_called_once()
                replace.assert_called_once()
        self.assertFalse((self.output / 'commands-replace-events.jsonl').exists())

    def test_permanent_lock_stops_at_remaining_command_budget(self):
        now = [0.0]
        def sleep(value):
            now[0] += value
        error = self.denied()
        with mock.patch.object(adb_control.subprocess, 'run', return_value=self.completed) as sdk, mock.patch.object(Path, 'replace', side_effect=error), mock.patch.object(adb_control.time, 'monotonic', side_effect=lambda: now[0]), mock.patch.object(adb_control.time, 'sleep', side_effect=sleep):
            with self.assertRaises(PermissionError) as actual:
                self.run_command(timeout=.025)
        self.assertIs(actual.exception, error)
        self.assertAlmostEqual(now[0], .025)
        sdk.assert_called_once()
        self.assertTrue((self.output / 'commands.json.tmp').exists())
        self.assertFalse((self.output / 'commands.json').exists())

    def test_long_command_budget_does_not_expand_local_quarter_second_limit(self):
        now = [0.0]
        error = self.denied(33)
        def sleep(value):
            now[0] += value
        with mock.patch.object(adb_control.subprocess, 'run', return_value=self.completed) as sdk, mock.patch.object(Path, 'replace', side_effect=error) as replace, mock.patch.object(adb_control.time, 'monotonic', side_effect=lambda: now[0]), mock.patch.object(adb_control.time, 'sleep', side_effect=sleep):
            with self.assertRaises(PermissionError):
                self.run_command(timeout=240)
        self.assertLessEqual(now[0], .25)
        self.assertLessEqual(replace.call_count, 26)
        sdk.assert_called_once()

    def test_original_sdk_timeout_is_preserved_when_no_retry_budget_remains(self):
        now = [0.0]
        timeout = subprocess.TimeoutExpired('public-sdk', 1, output=b'public-partial')
        error = self.denied()
        def sdk(*args, **kwargs):
            now[0] = 1
            raise timeout
        with mock.patch.object(adb_control.subprocess, 'run', side_effect=sdk) as execute, mock.patch.object(Path, 'replace', side_effect=error) as replace, mock.patch.object(adb_control.time, 'monotonic', side_effect=lambda: now[0]), mock.patch.object(adb_control.time, 'sleep') as sleep:
            with self.assertRaises(subprocess.TimeoutExpired) as actual:
                self.run_command(timeout=1)
        self.assertIs(actual.exception, timeout)
        self.assertIs(actual.exception.__cause__, error)
        self.assertEqual((self.output / 'timed-out-command-001.stdout.log').read_bytes(), b'public-partial')
        execute.assert_called_once()
        replace.assert_called_once()
        sleep.assert_not_called()

    def test_unrecordable_retry_denial_does_not_hide_failure(self):
        original = Path.open
        error = self.denied()
        def open_file(path, *args, **kwargs):
            if path.name == 'commands-replace-events.jsonl':
                raise OSError('event receipt unavailable')
            return original(path, *args, **kwargs)
        with mock.patch.object(adb_control.subprocess, 'run', return_value=self.completed) as sdk, mock.patch.object(Path, 'replace', side_effect=error) as replace, mock.patch.object(Path, 'open', open_file):
            with self.assertRaises(PermissionError) as actual:
                self.run_command()
        self.assertIs(actual.exception, error)
        self.assertIsInstance(actual.exception.__cause__, OSError)
        sdk.assert_called_once()
        replace.assert_called_once()

    def test_nonzero_sdk_result_remains_failure_after_local_retry(self):
        original = Path.replace
        attempts = [0]
        def replace(path, target):
            attempts[0] += 1
            if attempts[0] == 1:
                raise self.denied()
            return original(path, target)
        completed = subprocess.CompletedProcess([], 7, b'public-out', b'public-err')
        with mock.patch.object(adb_control.subprocess, 'run', return_value=completed) as sdk, mock.patch.object(Path, 'replace', replace):
            with self.assertRaises(subprocess.CalledProcessError) as actual:
                self.run_command()
        self.assertEqual(actual.exception.returncode, 7)
        self.assertEqual(actual.exception.stdout, b'public-out')
        self.assertEqual(actual.exception.stderr, b'public-err')
        sdk.assert_called_once()
        self.assertEqual((self.output / 'failed-command-001.stderr.log').read_bytes(), b'public-err')

    @unittest.skipUnless(os.name == 'nt', 'Requires real Windows read-handle sharing')
    def test_real_windows_read_handle_can_close_without_sdk_replay(self):
        self.recorder._record(dict(public='before'), time.monotonic() + 1)
        handle = (self.output / 'commands.json').open('rb')
        self.addCleanup(lambda: handle.close())
        denied = threading.Event()
        original = Path.replace
        errors = []
        def release():
            if denied.wait(2):
                handle.close()
            else:
                errors.append('no actual sharing denial observed')
        thread = threading.Thread(target=release)
        thread.start()
        def replace(path, target):
            try:
                return original(path, target)
            except PermissionError:
                denied.set()
                raise
        try:
            with mock.patch.object(adb_control.subprocess, 'run', return_value=self.completed) as sdk, mock.patch.object(Path, 'replace', replace):
                self.assertIs(self.run_command(), self.completed)
            sdk.assert_called_once()
        finally:
            thread.join(3)
        self.assertFalse(thread.is_alive())
        self.assertEqual(errors, [])
        self.assertTrue(denied.is_set())
        self.assertEqual(len(json.loads((self.output / 'commands.json').read_text())), 2)


if __name__ == '__main__':
    unittest.main()
