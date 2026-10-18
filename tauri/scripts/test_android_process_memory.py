"""Ownership, PID reuse and actual meminfo truncation regression contracts."""
import importlib.util
from pathlib import Path
import unittest
from unittest import mock
import subprocess
import tempfile
import io
import sys

spec = importlib.util.spec_from_file_location('memory', Path(__file__).with_name('android-process-memory.py'))
memory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(memory)

APP = 'com.solosoul.app'
PROVIDER = 'com.android.webview'
NAME = PROVIDER + ':sandboxed_process0:org.chromium.content.app.SandboxedProcessService0:0'
ROOT = 'ProcessRecord{308a815 10559:com.solosoul.app/u0a105}'
RENDERER = 'ProcessRecord{def37a2 10626:' + NAME + '/u0a105i82}'
REGISTRY = ('  *APP* UID 10080 ProcessRecord{c56773e 10609:com.android.webview:webview_service/u0a80}\n'
            '  *APP* UID 10105 ' + ROOT + '\n  *APP* UID 99082 ' + RENDERER + '\n')
SERVICES = ('ACTIVITY MANAGER SERVICES (dumpsys activity services)\n  User 0 active services:\n'
            '  * ServiceRecord{ec471c u0 com.solosoul.app/org.chromium.content.app.SandboxedProcessService0:0}\n'
            '    app=' + RENDERER + '\n    isolatedProc=' + RENDERER + '\n'
            '    recentCallingPackage=com.solosoul.app\n    recentCallingUid=10105\n'
            '    Bindings:\n      * Client AppBindRecord{a99f779 ' + ROOT + '}\n'
            '  Connection bindings to services:\n')


def stat(pid, birth=12698151):
    return f'{pid} (name with ) spaces) ' + ' '.join(['S'] + ['0'] * 18 + [str(birth)] + ['0'] * 30)


def status(uid):
    return 'Name:\tpublic\nUid:\t' + '\t'.join([str(uid)] * 4) + '\n'


def meminfo(pid, name, pss=100, rss=200):
    return f'** MEMINFO in pid {pid} [{name}] **\n TOTAL PSS: {pss} TOTAL RSS: {rss} TOTAL SWAP (KB): 0\n'


class Runner:
    def __init__(self):
        self.calls = []
        self.birth_changes = False
        self.truncate = False
        self.registry = REGISTRY
        self.services = SERVICES

    def __call__(self, *command):
        self.calls.append(command)
        if command == ('shell', 'dumpsys', 'activity', 'processes'):
            return self.registry
        if command == ('shell', 'dumpsys', 'activity', 'services', APP):
            return self.services
        if command[:2] == ('shell', 'cat'):
            _, _, path = command
            pid = int(path.split('/')[2])
            if path.endswith('/stat'):
                changes = self.birth_changes and self.calls.count(command) == 2
                return stat(pid, 12698151 + int(changes))
            return status(10105 if pid == 10559 else 99082)
        if command[:5] == ('shell', 'dumpsys', 'meminfo', '--local', '-s'):
            pid = int(command[-1])
            name = APP if pid == 10559 else NAME
            if self.truncate:
                return f'** MEMINFO in pid {pid} [{name}] **\n'
            return meminfo(pid, name)
        raise AssertionError('Unexpected command, especially remote meminfo: ' + repr(command))


class MemoryTests(unittest.TestCase):
    def collect(self, run):
        return memory.collect_snapshot(run, APP, 10105, PROVIDER)

    def test_main_and_bound_renderer_sum_excludes_shared_provider(self):
        run = Runner()
        result = self.collect(run)
        self.assertTrue(result['fullProcessScopeAccepted'])
        self.assertEqual(result['state'], 'complete')
        self.assertEqual(result['pssBytes'], 200 * 1024)
        self.assertEqual([row['pid'] for row in result['processes']], [10559, 10626])
        self.assertTrue(all('--local' in command for command in run.calls if 'meminfo' in command))
        self.assertNotIn(('shell', 'dumpsys', 'meminfo', '--local', '-s', '10609'), run.calls)

    def test_real_header_only_renderer_output_is_not_zero_or_success(self):
        run = Runner()
        run.truncate = True
        with self.assertRaisesRegex(memory.MemoryEvidenceError, 'incomplete'):
            self.collect(run)

    def test_kernel_pid_reuse_rejects_the_entire_sample(self):
        run = Runner()
        run.birth_changes = True
        with self.assertRaisesRegex(memory.MemoryEvidenceError, 'identity changed'):
            self.collect(run)

    def test_absent_app_is_not_a_zero_memory_sample(self):
        run = Runner()
        run.registry = REGISTRY.splitlines()[0] + '\n'
        run.services = 'ACTIVITY MANAGER SERVICES (dumpsys activity services)\n  (nothing)\n'
        result = self.collect(run)
        self.assertEqual(result['state'], 'not-running')
        self.assertIsNone(result['pssBytes'])
        self.assertFalse(result['fullProcessScopeAccepted'])

    def test_main_only_before_renderer_is_explicitly_incomplete(self):
        run = Runner()
        run.registry = '\n'.join(REGISTRY.splitlines()[:2]) + '\n'
        run.services = 'ACTIVITY MANAGER SERVICES (dumpsys activity services)\n  (nothing)\n'
        result = self.collect(run)
        self.assertEqual(result['state'], 'main-only-before-renderer')
        self.assertIsNone(result['pssBytes'])

    def test_pid_name_alone_does_not_prove_renderer_ownership(self):
        for value in [SERVICES.replace(ROOT, ROOT.replace('10559:', '10560:')),
                      SERVICES.replace('recentCallingUid=10105', 'recentCallingUid=10106'),
                      SERVICES.replace('recentCallingPackage=' + APP, 'recentCallingPackage=other.app'),
                      SERVICES.replace('    isolatedProc=' + RENDERER, '    isolatedProc=null')]:
            with self.subTest(value=value):
                with self.assertRaises(memory.MemoryEvidenceError):
                    memory.owned_processes(REGISTRY, value, APP, 10105, PROVIDER)

    def test_no_silent_loss_of_owned_isolated_process(self):
        with self.assertRaisesRegex(memory.MemoryEvidenceError, 'not covered'):
            memory.owned_processes(REGISTRY, SERVICES.split('  * ServiceRecord')[0], APP, 10105, PROVIDER)

    def test_shared_uid_or_wrong_provider_rejected(self):
        for registry, provider in [(REGISTRY.replace('10559:' + APP, '10559:foreign.app'), PROVIDER),
                                   (REGISTRY, 'com.google.android.webview')]:
            with self.assertRaises(memory.MemoryEvidenceError):
                memory.owned_processes(registry, SERVICES, APP, 10105, provider)

    def test_duplicate_pid_wrong_kernel_uid_and_birth_are_rejected(self):
        with self.assertRaisesRegex(memory.MemoryEvidenceError, 'Duplicate'):
            memory.process_records(REGISTRY + REGISTRY)
        expected = dict(pid=10559, uid=10105)
        for proc_stat, proc_status in [(stat(10560), status(10105)), (stat(10559, 0), status(10105)),
                                        (stat(10559), status(10106)), (stat(10559), 'Uid:\t10105\n')]:
            with self.subTest(proc_stat=proc_stat, proc_status=proc_status):
                with self.assertRaises(memory.MemoryEvidenceError):
                    memory.kernel_identity(proc_stat, proc_status, expected)

    def test_memory_pid_name_and_totals_are_checked(self):
        expected = dict(pid=10559, name=APP)
        for text in [meminfo(10560, APP), meminfo(10559, 'foreign.app'), meminfo(10559, APP, 0),
                     meminfo(10559, APP, 201, 200), meminfo(10559, APP) * 2]:
            with self.assertRaises(memory.MemoryEvidenceError):
                memory.memory_values(text, expected)

    def test_swap_pss_summary_keeps_proportional_swap_separate(self):
        text = meminfo(10559, APP, 74134, 187864).replace('TOTAL SWAP (KB): 0', 'TOTAL SWAP PSS: 120')
        result = memory.memory_values(text, dict(pid=10559, name=APP))
        self.assertEqual(result['pssBytes'], 74134 * 1024)
        self.assertEqual(result['rssBytes'], 187864 * 1024)
        self.assertIsNone(result['swapBytes'])
        self.assertEqual(result['swapPssBytes'], 120 * 1024)

    def test_legacy_swap_summary_retains_its_exact_contract(self):
        text = meminfo(10559, APP).replace('TOTAL SWAP (KB): 0', 'TOTAL SWAP (KB): 7')
        self.assertEqual(memory.memory_values(text, dict(pid=10559, name=APP)),
                         dict(pssBytes=100 * 1024, rssBytes=200 * 1024, swapBytes=7 * 1024))

    def test_mixed_swap_formats_collect_one_complete_owned_snapshot(self):
        run = Runner()

        def mixed(*command):
            value = run(*command)
            if command == ('shell', 'dumpsys', 'meminfo', '--local', '-s', '10626'):
                return value.replace('TOTAL SWAP (KB): 0', 'TOTAL SWAP PSS: 0')
            return value

        result = self.collect(mixed)
        self.assertTrue(result['fullProcessScopeAccepted'])
        self.assertEqual(result['pssBytes'], 200 * 1024)
        renderer = next(row for row in result['processes'] if row['role'] == 'renderer')
        self.assertIsNone(renderer['swapBytes'])
        self.assertEqual(renderer['swapPssBytes'], 0)

    def test_ambiguous_or_extended_total_rows_are_rejected(self):
        text = meminfo(10559, APP)
        pss = text.replace('TOTAL SWAP (KB): 0', 'TOTAL SWAP PSS: 1')
        for invalid in [text + pss.splitlines()[-1] + '\n',
                        text.rstrip('\n') + ' TOTAL SWAP PSS: 1\n',
                        pss.rstrip('\n') + ' TOTAL SWAP (KB): 0\n',
                        pss.rstrip('\n') + ' trailing-data\n']:
            with self.subTest(text=invalid), self.assertRaises(memory.MemoryEvidenceError):
                memory.memory_values(invalid, dict(pid=10559, name=APP))

    def test_incomplete_or_unknown_swap_pss_summary_is_rejected(self):
        text = meminfo(10559, APP).replace('TOTAL SWAP (KB): 0', 'TOTAL SWAP PSS: 1')
        for invalid in [text.split(' TOTAL RSS:')[0], text.replace('TOTAL RSS:', 'RSS:'),
                        text.replace('SWAP PSS:', 'SWAP RSS:'), text.replace('SWAP PSS: 1', 'SWAP PSS: NaN'),
                        text.replace('SWAP PSS: 1', 'SWAP PSS: -1')]:
            with self.subTest(text=invalid), self.assertRaises(memory.MemoryEvidenceError):
                memory.memory_values(invalid, dict(pid=10559, name=APP))

    def test_swap_pss_does_not_relax_pid_name_or_nonzero_total_validation(self):
        for text in [meminfo(10560, APP), meminfo(10559, 'foreign.app'), meminfo(10559, APP, 0),
                     meminfo(10559, APP, 201, 200), meminfo(10559, APP) * 2]:
            with self.subTest(text=text), self.assertRaises(memory.MemoryEvidenceError):
                memory.memory_values(text.replace('TOTAL SWAP (KB): 0', 'TOTAL SWAP PSS: 1'),
                                     dict(pid=10559, name=APP))

    def test_complete_native_swap_forms_accept_crlf_and_preserve_units(self):
        for label in ['SWAP (KB)', 'SWAP PSS']:
            text = meminfo(10559, APP).replace('SWAP (KB)', label).replace('\n', '\r\n')
            result = memory.memory_values(text, dict(pid=10559, name=APP))
            self.assertEqual(result['pssBytes'], 100 * 1024)
            self.assertEqual(result['swapBytes'] if label == 'SWAP (KB)' else result['swapPssBytes'], 0)

    def test_target_validation_precedes_any_remote_command(self):
        run = Runner()
        for package in [APP + ';id', '', '-h']:
            with self.assertRaises(memory.MemoryEvidenceError):
                memory.collect_snapshot(run, package, 10105, PROVIDER)
        self.assertEqual(run.calls, [])

    def test_owned_set_change_during_collection_rejects_sample(self):
        run = Runner()
        def changed(*command):
            if command == ('shell', 'dumpsys', 'activity', 'processes') and command in run.calls:
                return REGISTRY.replace('308a815', '308a816')
            return run(*command)
        with self.assertRaises(memory.MemoryEvidenceError):
            self.collect(changed)



class AdbEndpointTests(unittest.TestCase):
    avd = 'SoloSoul_Endpoint_Test'
    serial = 'emulator-5578'

    def response(self, text=None, code=0):
        return subprocess.CompletedProcess([], code, (text or self.avd + '\nOK\n').encode(), b'')

    def test_custom_endpoint_applies_to_first_identity_and_following_commands(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'diagnostic'
            with mock.patch.object(memory.subprocess, 'run', return_value=self.response()) as execute:
                run = memory.AdbRunner(sys.executable, self.serial, self.avd, output,
                                       host='127.0.0.1', port=5057)
                run('shell', 'id')
            self.assertEqual(len(execute.call_args_list), 2)
            for call in execute.call_args_list:
                self.assertEqual(call.args[0][:7], [str(Path(sys.executable).resolve()), '-H', '127.0.0.1', '-P', '5057', '-s', self.serial])
                self.assertEqual(call.kwargs['timeout'], 10)
            self.assertEqual(execute.call_args_list[0].args[0][7:], ['emu', 'avd', 'name'])
            self.assertEqual(execute.call_args_list[1].args[0][7:], ['shell', 'id'])
            self.assertTrue((output / 'commands.json').is_file())

    def test_legacy_default_endpoint_is_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            with mock.patch.object(memory.subprocess, 'run', return_value=self.response()) as execute:
                memory.AdbRunner(sys.executable, self.serial, self.avd, Path(temporary) / 'diagnostic')
            self.assertEqual(execute.call_args.args[0][1:5], ['-H', 'localhost', '-P', '5049'])

    def test_invalid_endpoint_precedes_filesystem_and_remote_commands(self):
        invalid = [('remote.example', 5057), ('0.0.0.0', 5057), ('127.0.0.1;id', 5057),
                   (None, 5057), ('localhost', 0), ('localhost', 65536), ('localhost', True),
                   ('localhost', '5057'), ('localhost', 5057.0)]
        with tempfile.TemporaryDirectory() as temporary:
            for host, port in invalid:
                with self.subTest(host=host, port=port):
                    output = Path(temporary) / 'diagnostic'
                    with mock.patch.object(memory.subprocess, 'run') as execute:
                        with self.assertRaises(memory.MemoryEvidenceError):
                            memory.AdbRunner(sys.executable, self.serial, self.avd, output, host=host, port=port)
                    execute.assert_not_called()
                    self.assertFalse(output.exists())

    def test_invalid_emulator_identity_still_precedes_remote_commands(self):
        with tempfile.TemporaryDirectory() as temporary:
            for serial, avd in [('physical-device', self.avd), (self.serial, 'Unowned_AVD')]:
                with self.subTest(serial=serial, avd=avd):
                    output = Path(temporary) / 'diagnostic'
                    with mock.patch.object(memory.subprocess, 'run') as execute:
                        with self.assertRaises(memory.MemoryEvidenceError):
                            memory.AdbRunner(sys.executable, serial, avd, output, host='127.0.0.1', port=5057)
                    execute.assert_not_called()
                    self.assertFalse(output.exists())

    def test_custom_endpoint_does_not_bypass_wrong_avd_rejection(self):
        with tempfile.TemporaryDirectory() as temporary:
            with mock.patch.object(memory.subprocess, 'run', return_value=self.response('SoloSoul_Other\nOK\n')):
                with self.assertRaisesRegex(memory.MemoryEvidenceError, 'identity differs'):
                    memory.AdbRunner(sys.executable, self.serial, self.avd, Path(temporary) / 'diagnostic',
                                     host='127.0.0.1', port=5057)

    def test_failed_identity_command_is_logged_and_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'diagnostic'
            with mock.patch.object(memory.subprocess, 'run', return_value=self.response(code=1)):
                with self.assertRaisesRegex(memory.MemoryEvidenceError, 'command failed'):
                    memory.AdbRunner(sys.executable, self.serial, self.avd, output, host='127.0.0.1', port=5057)
            self.assertTrue((output / '001.stderr.txt').is_file())
            self.assertIn('"exitCode": 1', (output / 'commands.json').read_text(encoding='utf-8'))

    def test_cli_forwards_requested_endpoint_before_collection(self):
        with tempfile.TemporaryDirectory() as temporary:
            arguments = ['android-process-memory.py', '--adb', sys.executable, '--serial', self.serial,
                         '--avd', self.avd, '--package', APP, '--provider', PROVIDER, '--uid', '10105',
                         '--output', temporary, '--adb-host', '127.0.0.1', '--adb-port', '5057']
            run = mock.Mock(output=Path(temporary))
            with mock.patch.object(sys, 'argv', arguments), mock.patch.object(memory, 'AdbRunner', return_value=run) as create, mock.patch.object(memory, 'collect_snapshot', return_value={'fullProcessScopeAccepted': True}) as collect, mock.patch('sys.stdout', new_callable=io.StringIO):
                self.assertEqual(memory.main(), 0)
            create.assert_called_once_with(sys.executable, self.serial, self.avd, temporary, host='127.0.0.1', port=5057)
            collect.assert_called_once_with(run, APP, 10105, PROVIDER)


if __name__ == '__main__':
    unittest.main()
