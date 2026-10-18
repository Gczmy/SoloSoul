import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

_spec = importlib.util.spec_from_file_location('fresh_host_audit', Path(__file__).with_name('android-fresh-host-audit.py'))
host = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(host)


def process():
    return dict(Name='python.exe', ProcessId=200, ParentProcessId=100,
                CreationDate='2026-10-17T04:00:00.123456+01:00',
                ExecutablePath='D:\\公开验证\\python.exe', CommandLine='python.exe owned-public-worker.py')


def expected(phase='before'):
    return dict(schemaVersion=1, baseline=host.BASELINE, ownedProcesses=[] if phase == 'before' else [process()],
                protectedPorts=[5037, 5049], ownedPorts=[5057, 5578, 5579], avd='SoloSoul_public_test')


def snapshot(rows=None, listeners=None):
    return dict(processes=rows or [], listeners=listeners or [])


def encode(value):
    return json.dumps(value, ensure_ascii=False).encode('utf-8')


class FreshAudit(unittest.TestCase):
    def test_complete_empty_snapshots_and_exact_scope_pass(self):
        for phase in ('before', 'after'):
            self.assertTrue(host.audit(expected(phase), snapshot(), phase)['passed'])

    def test_schema_cannot_assume_a_baseline_drop_ports_or_omit_owned_identities(self):
        changes = [lambda s: s.update(schemaVersion=True), lambda s: s.update(oldAdb=None),
                   lambda s: s.pop('avd'), lambda s: s.update(baseline='assumed-empty'),
                   lambda s: s.update(protectedPorts=[5037]), lambda s: s.update(protectedPorts=[5037.0, 5049]),
                   lambda s: s.update(ownedPorts=[True]), lambda s: s.update(ownedPorts=[5057, 5057]),
                   lambda s: s.update(ownedPorts=[5049]), lambda s: s.update(ownedPorts=[]),
                   lambda s: s.update(ownedPorts=[0]), lambda s: s.update(ownedProcesses=[process()]),
                   lambda s: s.update(avd="SoloSoul_test'; Stop-Process -Id 1")]
        for change in changes:
            spec = expected();change(spec)
            with self.subTest(spec=spec), self.assertRaises(ValueError):
                host.validate_spec(spec, 'before')
        with self.assertRaises(ValueError):
            host.validate_spec(expected(), 'after')
        spec = expected('after');spec['ownedProcesses'] *= 2
        with self.assertRaises(ValueError):
            host.validate_spec(spec, 'after')

    def test_any_observed_adb_vm_or_owned_helper_prevents_zero_residue(self):
        for name in ('adb.exe', 'emulator.exe', 'qemu-system-x86_64.exe', 'python.exe'):
            row = dict(process(), Name=name)
            for phase in ('before', 'after'):
                with self.subTest(name=name, phase=phase):
                    result = host.audit(expected(phase), snapshot([row]), phase)
                    self.assertFalse(result['passed'])
                    self.assertIn('adb_vm_or_owned_helpers_present', result['reasons'])

    def test_each_owned_identity_change_and_pid_reuse_remains_a_refusal(self):
        for key, value in dict(Name='other.exe', ParentProcessId=99, CreationDate='2026-10-17T04:00:01.123456+01:00',
                               ExecutablePath='D:\\other\\python.exe', CommandLine='other command').items():
            row = dict(process(), **{key: value})
            with self.subTest(key=key):
                result = host.audit(expected('after'), snapshot([row]), 'after')
                self.assertFalse(result['passed'])
                self.assertIn('owned_identity_changed_or_pid_reused', result['reasons'])

    def test_all_protected_and_owned_listeners_refuse_and_other_scope_is_invalid(self):
        for port in (5037, 5049, 5057, 5578, 5579):
            listener = dict(LocalAddress='127.0.0.1', LocalPort=port, OwningProcess=300)
            self.assertFalse(host.audit(expected(), snapshot(listeners=[listener]), 'before')['passed'])
        listener['LocalPort'] = 6000
        with self.assertRaises(ValueError):
            host.audit(expected(), snapshot(listeners=[listener]), 'before')

    def test_malformed_duplicate_or_nonfinite_snapshots_are_not_empty_evidence(self):
        for value in ({}, dict(processes=None, listeners=[]), snapshot([process(), process()])):
            with self.subTest(value=value), self.assertRaises(ValueError):
                host.audit(expected('after'), value, 'after')
        for raw in (b'\x81', b'{"processes":[],"processes":[],"listeners":[]}', b'{"processes":NaN}'):
            with self.subTest(raw=raw), self.assertRaises((ValueError, UnicodeError)):
                host.identity.decode_json(raw)

    def test_one_binary_cim_query_keeps_original_fifteen_second_budget(self):
        done = subprocess.CompletedProcess([], 0, encode(snapshot()), b'')
        with mock.patch.object(host.subprocess, 'run', return_value=done) as run:
            self.assertIs(host.run_snapshot(Path('pwsh.exe'), expected('after'), 'after'), done)
        args, kwargs = run.call_args
        self.assertEqual(args[0][:3], ['pwsh.exe', '-NoProfile', '-NonInteractive'])
        self.assertTrue(kwargs['capture_output'])
        self.assertEqual(kwargs['timeout'], 15)
        self.assertNotIn('encoding', kwargs)
        self.assertNotIn('text', kwargs)
        self.assertIn('ParentProcessId', args[0][-1])
        self.assertIn('qemu-system-*', args[0][-1])


class FreshCli(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory();self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.pwsh = self.root/'pwsh.exe';self.pwsh.touch()
        self.spec_file = self.root/'spec.json';self.spec_file.write_bytes(encode(expected()))
        self.output = self.root/'before'

    def invoke(self, phase='before', *, output=None, baseline=None, result=None, side_effect=None):
        args = [phase, '--powershell', str(self.pwsh), '--expected', str(self.spec_file), '--output', str(output or self.output)]
        if baseline is not None:args += ['--baseline', str(baseline)]
        completed = result or subprocess.CompletedProcess([], 0, encode(snapshot()), b'')
        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.object(host.identity, 'is_windows', return_value=True), \
                mock.patch.object(host, 'run_snapshot', return_value=completed, side_effect=side_effect) as run, \
                contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = host.main(args)
        return code, stdout.getvalue(), stderr.getvalue(), run

    def before(self):
        code, _, _, _ = self.invoke();self.assertEqual(code, 0)
        self.spec_file.write_bytes(encode(expected('after')))

    def test_before_and_after_bind_original_files_without_raw_console_identity(self):
        self.before()
        code, text, _, run = self.invoke('after', output=self.root/'after', baseline=self.output)
        self.assertEqual(code, 0);self.assertEqual(run.call_count, 1)
        result = json.loads(text)
        self.assertEqual(set(result['baselineFilesSha256']), {'expected.json', 'host-snapshot.json', 'result.json'})
        self.assertNotIn('ProcessId', text)
        self.assertEqual((self.root/'after/expected.json').read_bytes(), self.spec_file.read_bytes())

    def test_after_requires_complete_unmodified_successful_before_evidence(self):
        self.before()
        for name in ('expected.json', 'host-snapshot.json', 'result.json'):
            path = self.output/name;original = path.read_bytes();path.write_bytes(b'changed')
            code, _, _, run = self.invoke('after', output=self.root/'after', baseline=self.output)
            self.assertEqual(code, 2);run.assert_not_called();path.write_bytes(original)
        path = self.output/'result.json';original = path.read_bytes()
        for raw in (b'[]', b'null'):
            path.write_bytes(raw)
            code, _, _, run = self.invoke('after', output=self.root/'after', baseline=self.output)
            self.assertEqual(code, 2);run.assert_not_called()
        path.write_bytes(original)
        spec = expected('after');spec['avd'] = 'SoloSoul_another'
        self.spec_file.write_bytes(encode(spec))
        code, _, _, run = self.invoke('after', output=self.root/'after', baseline=self.output)
        self.assertEqual(code, 2);run.assert_not_called()

    def test_valid_json_but_wrong_before_scope_or_hash_cannot_be_used(self):
        self.before()
        path = self.output/'result.json';original = path.read_bytes()
        value = json.loads(original);value['snapshotSha256'] = '0'*64;path.write_bytes(encode(value))
        code, _, _, run = self.invoke('after', output=self.root/'after', baseline=self.output)
        self.assertEqual(code, 2);run.assert_not_called()
        path.write_bytes(original)
        value = expected('after');value['ownedPorts'] = [5058, 5578, 5579]
        self.spec_file.write_bytes(encode(value))
        code, _, _, run = self.invoke('after', output=self.root/'after', baseline=self.output)
        self.assertEqual(code, 2);run.assert_not_called()

    def test_before_rejects_real_nonempty_snapshot_instead_of_adopting_it(self):
        raw = encode(snapshot([dict(process(), Name='adb.exe')]))
        code, text, _, _ = self.invoke(result=subprocess.CompletedProcess([], 0, raw, b''))
        self.assertEqual(code, 1);self.assertFalse(json.loads(text)['passed'])
        self.assertEqual((self.output/'host-snapshot.json').read_bytes(), raw)

    def test_existing_output_and_missing_or_unexpected_baseline_do_not_query(self):
        self.output.mkdir();sentinel = self.output/'keep';sentinel.write_bytes(b'keep')
        for phase, baseline in [('before', None), ('before', self.root/'missing'), ('after', None)]:
            code, _, _, run = self.invoke(phase, baseline=baseline)
            self.assertEqual(code, 2);run.assert_not_called()
        self.assertEqual(sentinel.read_bytes(), b'keep')

    def test_nonzero_invalid_utf8_or_duplicate_json_retains_original_bytes(self):
        for i, (status, raw) in enumerate([(1, b'partial'), (0, b'\x81'), (0, b'{"processes":[],"processes":[],"listeners":[]}')]):
            output = self.root/f'failure-{i}'
            code, text, _, run = self.invoke(output=output, result=subprocess.CompletedProcess([], status, raw, b'original'))
            self.assertEqual(code, 1);self.assertEqual(run.call_count, 1)
            self.assertFalse(json.loads(text)['passed'])
            self.assertEqual((output/'host-snapshot.json').read_bytes(), raw)
            self.assertEqual((output/'host-snapshot.stderr').read_bytes(), b'original')

    def test_timeout_keeps_partial_outputs_and_never_retries(self):
        error = subprocess.TimeoutExpired('pwsh', 15, output=b'partial', stderr=b'timed out')
        code, text, _, run = self.invoke(side_effect=error)
        self.assertEqual(code, 1);self.assertEqual(run.call_count, 1)
        self.assertIn('15-second', json.loads(text)['error'])
        self.assertEqual((self.output/'host-snapshot.json').read_bytes(), b'partial')

    def test_expected_file_or_before_evidence_changes_during_query_refuse(self):
        for i, before in enumerate((False, True)):
            if before:self.before()
            path = self.output/'result.json' if before else self.spec_file
            def change(*_args):
                path.write_bytes(b'changed')
                return subprocess.CompletedProcess([], 0, encode(snapshot()), b'')
            code, text, _, _ = self.invoke('after' if before else 'before',
                output=self.root/f'changed-{i}', baseline=self.output if before else None, side_effect=change)
            self.assertEqual(code, 1);self.assertIn('changed during collection', json.loads(text)['error'])
            if not before:self.spec_file.write_bytes(encode(expected()))

    def test_evidence_write_error_cannot_return_a_pass(self):
        original = Path.write_bytes
        def fail(path, data):
            if path.name == 'host-snapshot.json':raise PermissionError('read-only test denial')
            return original(path, data)
        with mock.patch.object(Path, 'write_bytes', fail):
            code, text, error, _ = self.invoke()
        self.assertEqual(code, 1);self.assertEqual(text, '')
        self.assertIn('Could not preserve', error)


if __name__ == '__main__':
    unittest.main()
