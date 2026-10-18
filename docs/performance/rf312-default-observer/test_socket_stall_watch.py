import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from socket_stall_watch import wait_for_worker, windows_capture


class Worker:
    def __init__(self, code, polls=3):
        self.code = code
        self.polls = polls
        self.wait_calls = 0

    def poll(self):
        self.polls -= 1
        return None if self.polls >= 0 else self.code

    def wait(self):
        self.wait_calls += 1
        return self.code


class StallWatch(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.receipt, self.report = self.root / "commands.json", self.root / "report.json"
        self.receipt.write_text("[]", encoding="utf-8")
        os.utime(self.receipt, (0, 0))
        self.output = self.root / "captures"
        self.now = 0
        self.captured = 0

    def sleep(self, seconds):
        self.now += seconds

    def capture(self):
        self.captured += 1
        return 0, b'{"connections":[]}', b""

    def run_watch(self, code=7, capture=None, polls=3):
        worker = Worker(code, polls)
        result = wait_for_worker(worker, self.output, [(self.receipt, self.report)], capture or self.capture,
                                 clock=lambda: self.now, wall_clock=lambda: 100 + self.now, sleep=self.sleep)
        self.assertEqual(worker.wait_calls, 1)
        return result

    def test_stale_receipt_captures_once_and_preserves_failure_exit(self):
        code, summary = self.run_watch(code=7)
        self.assertEqual(code, 7)
        self.assertEqual(summary["captures"], 1)
        self.assertEqual(self.captured, 1)
        self.assertFalse(summary["workerResultChanged"])
        self.assertEqual((self.output / "capture-001.stdout").read_bytes(), b'{"connections":[]}')

    def test_capture_timeout_does_not_abort_or_restart_worker(self):
        def failed():
            raise subprocess.TimeoutExpired("read-only capture", 10)
        code, summary = self.run_watch(code=23, capture=failed)
        self.assertEqual(code, 23)
        self.assertIn("TimeoutExpired", summary["captureErrors"])

    def test_nonzero_capture_retains_output_and_backend_result(self):
        code, summary = self.run_watch(code=0, capture=lambda: (5, b"partial", b"capture denied"))
        self.assertEqual(code, 0)
        self.assertIn("capture_exit_5", summary["captureErrors"])
        self.assertEqual((self.output / "capture-001.stderr").read_bytes(), b"capture denied")

    def test_finished_case_does_not_end_process_waiting(self):
        self.report.write_text(json.dumps({"finishedAt": 99}), encoding="utf-8")
        code, summary = self.run_watch(code=2)
        self.assertEqual(code, 2)
        self.assertEqual(summary["captures"], 0)
        self.assertEqual(self.now, .75)

    def test_missing_receipt_does_not_trigger_capture(self):
        self.receipt.unlink()
        code, summary = self.run_watch(code=0)
        self.assertEqual(code, 0)
        self.assertEqual(summary["captures"], 0)

    def test_existing_output_is_preserved_and_backend_is_waited(self):
        self.output.mkdir()
        marker = self.output / "existing.json"
        marker.write_text("original", encoding="utf-8")
        code, summary = self.run_watch(code=9)
        self.assertEqual(code, 9)
        self.assertFalse(summary["enabled"])
        self.assertEqual(marker.read_text(), "original")

    def test_corrupt_case_metadata_remains_advisory(self):
        self.report.write_text("{", encoding="utf-8")
        code, summary = self.run_watch(code=1)
        self.assertEqual(code, 1)
        self.assertIn("JSONDecodeError", summary["captureErrors"])

    def test_capture_is_rate_bounded(self):
        code, summary = self.run_watch(code=0, polls=25)
        self.assertEqual(code, 0)
        self.assertEqual(summary["captures"], 2)

    def test_windows_capture_refuses_unverified_process_ids(self):
        for value in [True, -1, 0, 1.5, "1"]:
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    windows_capture("powershell", value, 2)

    def test_windows_capture_is_bounded_hidden_and_read_only(self):
        calls = []
        def runner(argv, **kwargs):
            calls.append((argv, kwargs))
            return subprocess.CompletedProcess(argv, 0, b"{}", b"")
        self.assertEqual(windows_capture("powershell", 1, 2, runner=runner)(), (0, b"{}", b""))
        argv, kwargs = calls[0]
        self.assertEqual(kwargs["timeout"], 10)
        self.assertIn("Get-NetTCPConnection", argv[-1])
        self.assertIn("Get-CimInstance", argv[-1])
        self.assertIn("5057", argv[-1])
        for forbidden in ["Stop-Process", "kill-server", "Restart", "SendInput", "-H 127"]:
            self.assertNotIn(forbidden, argv[-1])


if __name__ == "__main__":
    unittest.main()
