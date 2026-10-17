import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import rf312_kernel_highwater as highwater
import rf312_instrumentation_state as state
import test_rf312_instrumentation_state as state_fixture

legacy = highwater.module('highwater_original_memory_tests',
                          highwater.WORK / 'tauri/scripts/test_android_process_memory.py')


class Runner(legacy.Runner):
    def __call__(self, *command):
        text = super().__call__(*command)
        if command == ('shell', 'dumpsys', 'activity', 'processes'):
            return state.HEADER + '\n  All known processes:\n' + text + '\n  OOM levels:\n'
        if command[:2] == ('shell', 'cat') and command[-1].endswith('/status'):
            return text + 'VmHWM:\t1000 kB\nVmRSS:\t200 kB\n'
        return text


class ReplayTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.directory = self.root / 'post-journey-memory'
        self.directory.mkdir()
        runner = Runner()
        self.body = []

        def record(*command):
            text = runner(*command)
            self.body.append((list(command), text.encode()))
            return text

        snapshot = legacy.memory.collect_snapshot(record, legacy.APP, 10105, legacy.PROVIDER)
        self.checkpoint = dict(accepted=True, readOnly=True, targetInstrumentationInjected=False,
                               uiObserverActive=False, snapshot=snapshot)
        self.native = dict(passed=True, restored=True, originalApksRestored=True,
                           frozenInputsUnchanged=True, targetInstrumentationInjected=False)
        self.dump = (state.HEADER + '\n  All known processes:\n' + legacy.REGISTRY + '\n  OOM levels:\n').encode()
        self.boundary = [(['shell', 'pidof', legacy.APP], b'10559\n'),
                         (['shell', 'dumpsys', 'activity', 'instrumentation'], b'Unknown command: instrumentation\n'),
                         (['shell', 'pm', 'list', 'packages', highwater.OBSERVER], b''),
                         (['shell', 'dumpsys', 'window', 'displays'], b'mCurrentFocus=com.solosoul.app/.MainActivity\n')]
        self.rows = self.boundary + self.body + self.boundary
        self.commands = []
        self.sdk = []
        for ordinal, (command, raw) in enumerate(self.rows, 1):
            self.commands.append(dict(command=command, timedOut=False, exitCode=0))
            (self.directory / f'{ordinal:03d}.stdout').write_bytes(raw)
            (self.directory / f'{ordinal:03d}.stderr').write_bytes(b'')
            self.sdk.append(dict(command=command, exitCode=0, stdoutSha256=highwater.sha(raw),
                                 stderrSha256=highwater.sha(b'')))
        self.save_documents()

    def save_documents(self):
        for file, value in [(self.directory / 'result.json', self.checkpoint),
                            (self.directory / 'commands.json', self.commands),
                            (self.root / 'commands.json', self.sdk), (self.root / 'report.json', self.native)]:
            file.write_text(json.dumps(value), encoding='utf-8')
        self.expected = {key: highwater.sha(path.read_bytes()) for key, path in {
            'checkpoint': self.directory / 'result.json', 'commands': self.directory / 'commands.json',
            'sdkCommands': self.root / 'commands.json', 'nativeReport': self.root / 'report.json'}.items()}

    def edit_stdout(self, ordinal, raw, update_sdk=True):
        (self.directory / f'{ordinal:03d}.stdout').write_bytes(raw)
        if update_sdk:
            self.sdk[ordinal - 1]['stdoutSha256'] = highwater.sha(raw)
            self.save_documents()

    def replay(self):
        return highwater.replay(self.directory, self.expected, legacy.memory, state)

    def test_complete_original_bytes_replay_and_are_never_summed(self):
        result = self.replay()
        self.assertEqual(result['rawCommandsReplayed'], 22)
        self.assertEqual(len(result['actualEmptyProcessDumps']), 2)
        self.assertEqual([p['kernelRssHighWaterBytes'] for p in result['processes']], [1024000, 1024000])
        self.assertIsNone(result['crossProcessPeakBytes'])
        self.assertFalse(result['pssPeakAccepted'])
        self.assertFalse(result['wholeJourneyPeakAccepted'])
        self.assertFalse(result['legacyInstrumentationReplyUsedAsAbsence'])
        self.assertEqual(result['newDeviceCommands'], 0)

    def test_raw_watermark_tampering_must_match_the_original_sdk_digest(self):
        self.edit_stdout(8, (self.directory / '008.stdout').read_bytes().replace(b'1000 kB', b'2000 kB'), False)
        with self.assertRaisesRegex(ValueError, 'original SDK'):
            self.replay()

    def test_missing_extra_and_wrong_artifact_hashes_are_rejected(self):
        for expected in [{}, {**self.expected, 'extra': 'a' * 64}, {**self.expected, 'sdkCommands': 'a' * 64}]:
            with self.subTest(expected=expected), self.assertRaises(ValueError):
                highwater.replay(self.directory, expected, legacy.memory, state)

    def test_failed_native_restoration_and_incomplete_checkpoint_are_rejected(self):
        for key in ['passed', 'restored', 'originalApksRestored', 'frozenInputsUnchanged']:
            with self.subTest(key=key):
                self.native[key] = False
                self.save_documents()
                with self.assertRaises(ValueError):
                    self.replay()
                self.native[key] = True
        self.checkpoint['accepted'] = False
        self.save_documents()
        with self.assertRaises(ValueError):
            self.replay()

    def test_nonzero_timeout_and_stderr_never_become_success(self):
        for change in [dict(exitCode=1), dict(exitCode=False), dict(timedOut=True)]:
            original = copy.deepcopy(self.commands[7])
            self.commands[7].update(change)
            self.save_documents()
            with self.assertRaises(ValueError):
                self.replay()
            self.commands[7] = original
        self.save_documents()
        (self.directory / '008.stderr').write_bytes(b'unknown')
        with self.assertRaisesRegex(ValueError, 'stderr'):
            self.replay()

    def test_duplicate_sdk_sequences_are_ambiguous(self):
        self.sdk += copy.deepcopy(self.sdk)
        self.save_documents()
        with self.assertRaisesRegex(ValueError, 'one exact'):
            self.replay()

    def test_old_unknown_state_body_is_not_absence_evidence(self):
        self.edit_stdout(5, b'Unknown command: instrumentation\n')
        with self.assertRaises(ValueError):
            self.replay()

    def test_actual_active_instrumentation_is_rejected(self):
        active = state_fixture.dump(True).replace(b'    public synthetic process entries\n', legacy.REGISTRY.encode())
        self.edit_stdout(5, active)
        with self.assertRaisesRegex(ValueError, 'active instrumentation'):
            self.replay()

    def test_pid_reuse_is_still_rejected_by_the_original_reader(self):
        self.edit_stdout(10, legacy.stat(10559, 12698152).encode())
        with self.assertRaises(legacy.memory.MemoryEvidenceError):
            self.replay()

    def test_changed_accepted_snapshot_cannot_replace_the_original_values(self):
        self.checkpoint['snapshot']['processes'][0]['pssBytes'] += 1024
        self.save_documents()
        with self.assertRaisesRegex(ValueError, 'snapshot does not replay'):
            self.replay()

    def test_high_water_regression_and_missing_value_rejected(self):
        original = (self.directory / '011.stdout').read_bytes()
        for changed in [original.replace(b'1000 kB', b'999 kB'), original.replace(b'VmHWM:', b'Other:')]:
            self.edit_stdout(11, changed)
            with self.assertRaises(ValueError):
                self.replay()

    def test_complete_cli_works_outside_repository_and_refuses_overwrite(self):
        config = dict(schemaVersion=1, checkpoint=str(self.directory), evidenceSha256=self.expected,
                      readerSha256={key: highwater.sha(path.read_bytes()) for key, path in {
                          'memory': highwater.WORK / 'tauri/scripts/android-process-memory.py',
                          'state': highwater.HERE / 'rf312_instrumentation_state.py'}.items()})
        source = self.root / 'explicit input.json'
        source.write_text(json.dumps(config), encoding='utf-8')
        output = self.root / 'new output.json'
        command = [sys.executable, '-B', str(Path(highwater.__file__)), '--input', str(source), '--output', str(output)]
        run = subprocess.run(command, capture_output=True, cwd=self.root)
        self.assertEqual(run.returncode, 0, run.stderr.decode())
        self.assertTrue(json.loads(output.read_bytes())['accepted'])
        original = output.read_bytes()
        rejected = subprocess.run(command, capture_output=True, cwd=self.root)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertEqual(output.read_bytes(), original)


class StatusTests(unittest.TestCase):
    def test_kernel_values_and_crlf_keep_binary_units(self):
        result = highwater.status_values('VmHWM:\t2048 kB\r\nVmRSS: 1024 kB\r\n')
        self.assertEqual(result, {'VmHWM': 2097152, 'VmRSS': 1048576})

    def test_missing_duplicate_wrong_units_negative_zero_and_overflow_rejected(self):
        for changed in ['', 'VmHWM: 10 kB\nVmHWM: 10 kB\nVmRSS: 1 kB\n',
                        'VmHWM: 1 MB\nVmRSS: 1 kB\n', 'VmHWM: -1 kB\nVmRSS: 1 kB\n',
                        'VmHWM: 0 kB\nVmRSS: 0 kB\n', 'VmHWM: 1 kB\nVmRSS: 2 kB\n',
                        f'VmHWM: {2**63} kB\nVmRSS: 1 kB\n',
                        'VmHWM: 10 kB\rVmRSS: 1 kB\n', 'VmHWM: 10 kB\0\nVmRSS: 1 kB\n']:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                highwater.status_values(changed)

    def test_duplicate_json_fields_are_rejected(self):
        with self.assertRaises(ValueError):
            highwater.decode(b'{"accepted":false,"accepted":true}')


if __name__ == '__main__':
    unittest.main()
