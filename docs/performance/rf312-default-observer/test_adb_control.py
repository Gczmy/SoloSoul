import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from adb_control import AdbRecorder


class Receipts(unittest.TestCase):
    def setUp(self):
        temporary_root = Path(tempfile.gettempdir()).resolve()
        self.output = Path(tempfile.mkdtemp(prefix='rf312-adb-receipt-', dir=temporary_root))
        self.assertTrue(self.output.resolve().is_relative_to(temporary_root))
        self.addCleanup(self.remove_files)
        self.recorder = AdbRecorder(sys.executable, 'emulator-5578', self.output)

    def remove_files(self):
        for child in self.output.iterdir():
            child.unlink()
        self.output.rmdir()

    def receipt(self):
        return json.loads((self.output / 'commands.json').read_text(encoding='utf-8'))

    def test_timeout_retains_same_exception_partial_bytes_and_requested_budget(self):
        original = subprocess.TimeoutExpired(['public-fixture-adb'], 7.25,
                                             output=b'partial public stdout', stderr=b'partial public stderr')
        with patch('adb_control.subprocess.run', side_effect=original) as run:
            with self.assertRaises(subprocess.TimeoutExpired) as caught:
                self.recorder.run('shell', 'getprop', timeout=7.25, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, creationflags=0)
        self.assertIs(caught.exception, original)
        self.assertEqual(run.call_args.args[0][1:7], ['-H', '127.0.0.1', '-P', '5057', '-s', 'emulator-5578'])
        self.assertEqual(run.call_args.kwargs['timeout'], 7.25)
        self.assertEqual(run.call_args.kwargs['creationflags'], 0)
        rows = self.receipt()
        self.assertEqual(len(rows), 1)
        self.assertTrue(rows[0]['timedOut'])
        self.assertIsNone(rows[0]['exitCode'])
        for name, data in [('stdout', original.stdout), ('stderr', original.stderr)]:
            self.assertEqual((self.output / rows[0][name + 'PartialFile']).read_bytes(), data)
            self.assertEqual(rows[0][name + 'Sha256'], hashlib.sha256(data).hexdigest())

    def test_success_retains_process_and_command_order(self):
        completed = subprocess.CompletedProcess(['public-fixture-adb'], 0, b'public', b'')
        with patch('adb_control.subprocess.run', return_value=completed):
            self.assertIs(self.recorder.run('version', timeout=3), completed)
            self.assertIs(self.recorder.run('version', timeout=2), completed)
        rows = self.receipt()
        self.assertEqual([row['timeoutSeconds'] for row in rows], [3, 2])
        self.assertEqual(self.recorder.commands, rows)
        self.assertFalse((self.output / 'commands.json.tmp').exists())
        self.assertFalse(list(self.output.glob('*.log')))

    def test_nonzero_exit_retains_stderr_and_called_process_error(self):
        completed = subprocess.CompletedProcess(['public-fixture-adb'], 1, b'', b'public failure')
        with patch('adb_control.subprocess.run', return_value=completed):
            with self.assertRaises(subprocess.CalledProcessError) as caught:
                self.recorder.run('shell', 'test', '-e', 'public-ready')
        self.assertEqual(caught.exception.returncode, 1)
        self.assertEqual(self.receipt()[0]['exitCode'], 1)
        self.assertEqual((self.output / 'failed-command-001.stderr.log').read_bytes(), b'public failure')

    def test_invalid_or_unbounded_budget_cannot_spawn(self):
        for value in [True, None, 0, -1, float('inf'), float('nan'), 241]:
            with self.subTest(value=value), patch('adb_control.subprocess.run') as run:
                with self.assertRaises(ValueError):
                    self.recorder.run('version', timeout=value)
                run.assert_not_called()
        self.assertFalse(list(self.output.iterdir()))

    def test_receipt_failure_does_not_mask_timeout(self):
        original = subprocess.TimeoutExpired(['public-fixture-adb'], 1, output=b'public partial')
        with patch('adb_control.subprocess.run', side_effect=original), patch.object(Path, 'write_bytes', side_effect=OSError('public receipt failure')):
            with self.assertRaises(subprocess.TimeoutExpired) as caught:
                self.recorder.run('version', timeout=1)
        self.assertIs(caught.exception, original)
        self.assertIsInstance(caught.exception.__cause__, OSError)


if __name__ == '__main__':
    unittest.main()
