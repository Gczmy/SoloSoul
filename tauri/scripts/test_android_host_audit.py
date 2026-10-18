"""Read-only host attestation: encoding, exact identity and failure evidence."""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


loader = importlib.util.spec_from_file_location("host_audit", Path(__file__).with_name("android-host-audit.py"))
host = importlib.util.module_from_spec(loader)
loader.loader.exec_module(host)


def process(pid=100, name="adb.exe", command=None):
    return dict(Name=name, ProcessId=pid, ParentProcessId=10,
                CreationDate="2026-10-16T10:00:00.123456+01:00",
                ExecutablePath="D:\\公开验证\\" + name,
                CommandLine=command or '"D:\\公开验证\\adb.exe" -L tcp:localhost:5049 server nodaemon')


def spec():
    return dict(schemaVersion=1, oldAdb=process(), ownedProcesses=[],
                ownedPorts=[5057, 5578, 5579], avd="SoloSoul_public_test")


def snapshot(rows=None, listeners=None):
    return dict(processes=[process()] if rows is None else rows,
                listeners=[] if listeners is None else listeners)


def encode(value):
    return json.dumps(value, ensure_ascii=False).encode("utf-8")


class HostAuditTests(unittest.TestCase):
    def test_utf8_snapshot_preserves_chinese_and_passes_complete_identity(self):
        raw = encode(snapshot())
        with self.assertRaises(UnicodeDecodeError):
            raw.decode("cp1252")
        self.assertTrue(host.audit(spec(), host.decode_json(raw))["passed"])
        self.assertTrue(host.audit(spec(), host.decode_json(b"\xef\xbb\xbf" + raw))["passed"])

    def test_malformed_duplicate_and_nonfinite_json_are_rejected(self):
        for raw in (b"\x81", b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}', b'{"a":'):
            with self.subTest(raw=raw), self.assertRaises((ValueError, UnicodeError)):
                host.decode_json(raw)

    def test_protected_server_requires_loopback_and_separate_owned_scope(self):
        changes = [lambda s: s.update(schemaVersion=True),
                   lambda s: s["oldAdb"].update(ProcessId=True),
                   lambda s: s["oldAdb"].update(ExecutablePath="relative\\adb.exe"),
                   lambda s: s["oldAdb"].update(CommandLine="adb.exe -L tcp:0.0.0.0:5049 server nodaemon"),
                   lambda s: s["oldAdb"].update(CommandLine="adb.exe -L tcp:localhost:0 server nodaemon"),
                   lambda s: s.update(ownedProcesses=[process()]),
                   lambda s: s.update(ownedPorts=[5049]),
                   lambda s: s.update(ownedPorts=[5057, 5057]),
                   lambda s: s.update(ownedPorts=[True]),
                   lambda s: s.update(ownedPorts=[]),
                   lambda s: s.update(avd="SoloSoul_test'; Stop-Process -Id 1; '"),
                   lambda s: s.update(extra=True)]
        for change in changes:
            value = spec()
            change(value)
            with self.subTest(value=value), self.assertRaises(ValueError):
                host.validate_spec(value)

    def test_birth_requires_valid_offset_and_microsecond_precision(self):
        for value in ("2026-10-16T10:00:00", "2026-10-16T10:00:00.1234567Z", "2026-99-16T10:00:00Z"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                host.birth(value)
        actual = process()
        actual["CreationDate"] = "2026-10-16T09:00:00.123456Z"
        actual["ExecutablePath"] = "d:\\公开验证\\ADB.EXE"
        self.assertTrue(host.audit(spec(), snapshot([actual]))["passed"])

    def test_every_protected_identity_component_and_absence_reject(self):
        changes = dict(Name="other.exe", ProcessId=101, ParentProcessId=11,
                       CreationDate="2026-10-16T10:00:00.123457+01:00",
                       ExecutablePath="D:\\other\\adb.exe", CommandLine="different command")
        for key, value in changes.items():
            actual = process()
            actual[key] = value
            with self.subTest(key=key):
                result = host.audit(spec(), snapshot([actual]))
                self.assertFalse(result["passed"])
                self.assertIn("protected_old_adb_missing_or_changed", result["reasons"])
        self.assertFalse(host.audit(spec(), snapshot([]))["passed"])

    def test_live_owned_root_descendant_and_orphan_avd_all_reject(self):
        expected = spec()
        root = process(200, "emulator.exe", "emulator -avd SoloSoul_public_test")
        expected["ownedProcesses"] = [root]
        child = process(201, "qemu-system-x86_64.exe", "qemu --public")
        child["ParentProcessId"] = 200
        orphan = process(202, "qemu-system-x86_64.exe", "qemu -avd SoloSoul_public_test")
        for row in (root, child, orphan):
            with self.subTest(name=row["Name"]):
                result = host.audit(expected, snapshot([process(), row]))
                self.assertFalse(result["passed"])
                self.assertEqual(result["ownedProcessRows"], 1)

    def test_pid_reuse_cannot_prove_old_descendants_are_gone(self):
        expected = spec()
        root = process(200)
        expected["ownedProcesses"] = [root]
        reused = dict(root, CreationDate="2026-10-16T10:00:01.123456+01:00")
        result = host.audit(expected, snapshot([process(), reused]))
        self.assertFalse(result["passed"])
        self.assertIn("owned_identity_changed_or_pid_reused", result["reasons"])

    def test_owned_listener_rejects_even_with_no_owned_processes(self):
        listener = dict(LocalAddress="127.0.0.1", LocalPort=5057, OwningProcess=200)
        result = host.audit(spec(), snapshot(listeners=[listener]))
        self.assertFalse(result["passed"])
        self.assertEqual(result["ownedListeners"], 1)
        listener["LocalPort"] = 5049
        with self.assertRaisesRegex(ValueError, "outside declared scope"):
            host.audit(spec(), snapshot(listeners=[listener]))

    def test_incomplete_or_duplicate_snapshot_cannot_attest_cleanup(self):
        incomplete = process()
        incomplete.pop("CreationDate")
        for value in ({}, snapshot([incomplete]), snapshot([process(), process()]),
                      dict(processes=process(), listeners=[]),
                      snapshot(listeners=[dict(LocalAddress="::", LocalPort=5057, OwningProcess=0)])):
            with self.subTest(value=value), self.assertRaises(ValueError):
                host.audit(spec(), value)

    def test_collector_uses_binary_capture_and_original_short_budget(self):
        completed = subprocess.CompletedProcess([], 0, encode(snapshot()), b"")
        with mock.patch.object(host.subprocess, "run", return_value=completed) as run:
            self.assertIs(host.run_snapshot(Path("pwsh.exe"), spec()), completed)
        args, kwargs = run.call_args
        self.assertEqual(args[0][:3], ["pwsh.exe", "-NoProfile", "-NonInteractive"])
        self.assertTrue(kwargs["capture_output"])
        self.assertNotIn("text", kwargs)
        self.assertNotIn("encoding", kwargs)
        self.assertEqual(kwargs["timeout"], 15)
        self.assertEqual(kwargs["stdin"], subprocess.DEVNULL)


class HostAuditCliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.pwsh = self.root / "pwsh.exe"
        self.pwsh.touch()
        self.expected = self.root / "expected.json"
        self.expected.write_bytes(encode(spec()))
        self.output = self.root / "evidence"
        self.args = ["--powershell", str(self.pwsh), "--expected", str(self.expected), "--output", str(self.output)]

    def run_cli(self, completed=None, side_effect=None):
        if completed is None:
            completed = subprocess.CompletedProcess([], 0, encode(snapshot()), b"")
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(host, "is_windows", return_value=True), \
                mock.patch.object(host, "run_snapshot", return_value=completed, side_effect=side_effect) as run, \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = host.main(self.args)
        return status, out.getvalue(), err.getvalue(), run

    def test_success_preserves_exact_bytes_and_hashes_without_raw_console_identity(self):
        raw = encode(snapshot())
        status, stdout, _, run = self.run_cli()
        self.assertEqual(status, 0)
        self.assertEqual(run.call_count, 1)
        self.assertEqual((self.output / "host-snapshot.json").read_bytes(), raw)
        result = json.loads(stdout)
        self.assertEqual(result["snapshotSha256"], hashlib.sha256(raw).hexdigest())
        self.assertTrue(result["readOnly"])
        self.assertNotIn("ProcessId", stdout)
        self.assertNotIn("公开验证", stdout)
        self.assertEqual(json.loads((self.output / "result.json").read_text(encoding="utf-8")), result)

    def test_existing_output_is_never_overwritten_or_queried(self):
        self.output.mkdir()
        sentinel = self.output / "keep.txt"
        sentinel.write_bytes(b"keep")
        status, _, _, run = self.run_cli()
        self.assertEqual(status, 2)
        run.assert_not_called()
        self.assertEqual(sentinel.read_bytes(), b"keep")

    def test_invalid_spec_and_non_windows_do_not_collect_or_create_evidence(self):
        self.expected.write_bytes(b'{"schemaVersion":1}')
        status, _, _, run = self.run_cli()
        self.assertEqual(status, 2)
        run.assert_not_called()
        self.assertFalse(self.output.exists())
        with mock.patch.object(host, "is_windows", return_value=False), \
                mock.patch.object(host, "run_snapshot") as run, contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(host.main(self.args), 2)
        run.assert_not_called()

    def test_nonzero_collection_retains_failed_stdout_and_stderr(self):
        status, stdout, _, _ = self.run_cli(subprocess.CompletedProcess([], 1, b"partial", b"original error"))
        self.assertEqual(status, 1)
        self.assertFalse(json.loads(stdout)["passed"])
        self.assertEqual((self.output / "host-snapshot.json").read_bytes(), b"partial")
        self.assertEqual((self.output / "host-snapshot.stderr").read_bytes(), b"original error")

    def test_timeout_retains_partial_bytes_and_does_not_retry(self):
        error = subprocess.TimeoutExpired("pwsh", 15, output=b"partial", stderr=b"timeout detail")
        status, stdout, _, run = self.run_cli(side_effect=error)
        self.assertEqual(status, 1)
        self.assertEqual(run.call_count, 1)
        self.assertIn("15-second", json.loads(stdout)["error"])
        self.assertEqual((self.output / "host-snapshot.json").read_bytes(), b"partial")

    def test_invalid_utf8_snapshot_is_preserved_and_rejected(self):
        status, stdout, _, _ = self.run_cli(subprocess.CompletedProcess([], 0, b"\x81", b""))
        self.assertEqual(status, 1)
        self.assertFalse(json.loads(stdout)["passed"])
        self.assertEqual((self.output / "host-snapshot.json").read_bytes(), b"\x81")

    def test_changed_expected_identity_file_rejects_whole_snapshot(self):
        def change_file(*_args):
            self.expected.write_bytes(b"changed")
            return subprocess.CompletedProcess([], 0, encode(snapshot()), b"")
        status, stdout, _, _ = self.run_cli(side_effect=change_file)
        self.assertEqual(status, 1)
        self.assertIn("changed during collection", json.loads(stdout)["error"])

    def test_evidence_write_failure_cannot_return_pass(self):
        original = Path.write_bytes
        def fail_snapshot(path, data):
            if path.name == "host-snapshot.json":
                raise PermissionError("test write refusal")
            return original(path, data)
        with mock.patch.object(Path, "write_bytes", fail_snapshot):
            status, stdout, stderr, _ = self.run_cli()
        self.assertEqual(status, 1)
        self.assertEqual(stdout, "")
        self.assertIn("Could not preserve", stderr)


if __name__ == "__main__":
    unittest.main()
