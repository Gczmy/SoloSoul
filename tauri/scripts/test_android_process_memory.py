"""Ownership, PID reuse and actual meminfo truncation regression contracts."""
import importlib.util
from pathlib import Path
import unittest

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


if __name__ == '__main__':
    unittest.main()
