import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import queue_context as context
import related_units
import replay


class QueueAdmission(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='rf312-queue-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.units = self.root / 'units'
        self.units.mkdir()
        (self.units / 'tests.log').write_bytes(b'public unit receipt fixture\n')
        ids = related_units.case_ids(related_units.collected())
        queue_ids = related_units.case_ids(related_units.queue_collected())
        self.receipt = dict(passed=True, tests=len(ids), testIds=ids, failures=0, errors=0,
                            skipped=0, sourceSha256=context.binding.sources(),
                            externalReaderSourcesSha256={name: context.sha(context.WORK / name)
                                for name in ('tauri/scripts/android-process-memory.py',
                                             'tauri/scripts/test_android_process_memory.py')},
                            queueSourceSha256=context.sources(), queueTests=len(queue_ids),
                            queueTestIds=queue_ids, queueTestsPassed=True, queueSkipped=0,
                            logFile='tests.log', logSha256=context.sha(self.units / 'tests.log'))
        self.units_file = self.units / 'result.json'
        self.save_units()
        self.native = self.root / 'native'
        self.native.mkdir()
        self.native_result = dict(passed=True, restored=True, journeyMemorySamplingEnabled=False)
        self.write(self.native / 'result.json', self.native_result)
        self.adapter = self.root / 'adapter.json'
        self.adapter.write_bytes(b'public adapter fixture\n')

    @staticmethod
    def write(path, value):
        path.write_text(json.dumps(value), encoding='utf-8')

    def save_units(self):
        self.write(self.units_file, self.receipt)

    def prepare(self, output=None):
        with mock.patch.object(context.binding, 'load', return_value={'public': 'binding fixture'}):
            return replay.prepare(self.native, self.adapter, self.units_file,
                                  output or self.root / 'replay')

    def load(self, path):
        with mock.patch.object(sys, 'argv', ['replay', '--context', str(path)]), mock.patch.object(
                context.binding, 'load', return_value={'public': 'binding fixture'}):
            return context.load_context()

    def test_complete_receipt_shape_and_original_log_are_bound(self):
        self.assertEqual(context.validate_units(self.units_file), self.receipt)

    def test_changed_runtime_source_is_refused(self):
        first = next(iter(self.receipt['sourceSha256']))
        self.receipt['sourceSha256'][first] = '0' * 64
        self.save_units()
        with self.assertRaises(ValueError):
            context.validate_units(self.units_file)

    def test_changed_canonical_reader_is_refused(self):
        first = next(iter(self.receipt['externalReaderSourcesSha256']))
        self.receipt['externalReaderSourcesSha256'][first] = '0' * 64
        self.save_units()
        with self.assertRaises(ValueError):
            context.validate_units(self.units_file)

    def test_queue_source_change_is_not_hidden_by_same_runtime(self):
        first = next(iter(self.receipt['queueSourceSha256']))
        self.receipt['queueSourceSha256'][first] = '0' * 64
        self.save_units()
        with self.assertRaises(ValueError):
            context.validate_units(self.units_file)

    def test_missing_case_cannot_pass_by_preserving_count(self):
        self.receipt['testIds'].pop()
        self.save_units()
        with self.assertRaises(ValueError):
            context.validate_units(self.units_file)

    def test_changed_original_log_is_refused(self):
        (self.units / 'tests.log').write_bytes(b'changed public fixture\n')
        with self.assertRaises(ValueError):
            context.validate_units(self.units_file)

    def test_owner_rejects_unit_receipt_before_any_process_launch(self):
        spec = importlib.util.spec_from_file_location('queue_owner_under_test', context.ROOT / 'owner.py')
        owner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(owner)
        self.receipt['passed'] = False
        self.save_units()
        argv = ['owner', '--kind', 'default-state', '--preparation', str(self.root / 'unread-avd.json'),
                '--output', str(self.root / 'uncreated-native'), '--adapter-inputs', str(self.adapter),
                '--sampling', 'off', '--unit-evidence', str(self.units_file)]
        with mock.patch.object(sys, 'argv', argv), mock.patch.object(subprocess, 'Popen') as launch:
            with self.assertRaises(ValueError):
                owner.main()
        launch.assert_not_called()
        self.assertFalse((self.root / 'uncreated-native').exists())

    def test_replay_never_reuses_or_changes_existing_output(self):
        output = self.root / 'existing'
        output.mkdir()
        marker = output / 'keep'
        marker.write_bytes(b'public original\n')
        with self.assertRaises(ValueError):
            self.prepare(output)
        self.assertEqual(marker.read_bytes(), b'public original\n')

    def test_owner_rejects_changed_dedicated_serial_before_any_process_launch(self):
        spec = importlib.util.spec_from_file_location('queue_owner_serial_under_test', context.ROOT / 'owner.py')
        owner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(owner)
        argv = ['owner', '--kind', 'default-state', '--preparation', str(self.root / 'unread-avd.json'),
                '--output', str(self.root / 'uncreated-native'), '--adapter-inputs', str(self.adapter),
                '--sampling', 'off', '--unit-evidence', str(self.units_file)]
        binding = {'config': {'runtimeInputs': str(self.root / 'unread-runtime.json')},
                   'runtime': {'host': {'serial': 'emulator-5599'}}}
        with mock.patch.object(sys, 'argv', argv), mock.patch.object(subprocess, 'Popen') as launch, mock.patch.object(
                owner.state_binding, 'load', return_value=binding):
            with self.assertRaisesRegex(ValueError, 'emulator-5578'):
                owner.main()
        launch.assert_not_called()
        self.assertFalse((self.root / 'uncreated-native').exists())

    def test_failed_or_untyped_parent_cannot_start_complete_replay(self):
        for field, value in [('passed', False), ('restored', False),
                             ('journeyMemorySamplingEnabled', 1), ('journeyMemorySamplingEnabled', 'off')]:
            with self.subTest(field=field, value=value):
                original = dict(self.native_result)
                original[field] = value
                self.write(self.native / 'result.json', original)
                with self.assertRaises(ValueError):
                    self.prepare()
                self.assertFalse((self.root / 'replay').exists())

    def test_replay_context_binds_separate_paths_and_does_not_run_sdk(self):
        with mock.patch.object(subprocess, 'Popen') as launch:
            path, sampling = self.prepare()
            loaded = self.load(path)
        launch.assert_not_called()
        self.assertFalse(sampling)
        self.assertEqual(loaded.native, self.native.resolve())
        self.assertEqual(loaded.output, path.parent)
        self.assertEqual(loaded.mode, 'off')

    def test_replay_output_inside_native_is_refused(self):
        with self.assertRaises(ValueError):
            self.prepare(self.native / 'inside')
        self.assertFalse((self.native / 'inside').exists())

    def test_replay_context_detects_changed_adapter(self):
        path, _ = self.prepare()
        self.adapter.write_bytes(b'changed public adapter fixture\n')
        with self.assertRaises(ValueError):
            self.load(path)

    def test_replay_context_rejects_unknown_fields(self):
        path, _ = self.prepare()
        value = context.read(path)
        value['skipStateCheck'] = True
        self.write(path, value)
        with self.assertRaises(ValueError):
            self.load(path)

    def cli(self, name, *args, optimized=False):
        return subprocess.run([sys.executable, '-B', *(['-O'] if optimized else []),
                               str(context.ROOT / name), *map(str, args)], cwd=self.root,
                              stdin=subprocess.DEVNULL, capture_output=True, timeout=20,
                              creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))

    def test_public_cli_help_works_from_an_unrelated_directory(self):
        for name in ('owner.py', 'replay.py', 'related_units.py'):
            with self.subTest(name=name):
                result = self.cli(name, '--help')
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(b'--output', result.stdout)

    def test_optimized_python_cannot_disable_native_or_replay_guards(self):
        for name in ('owner.py', 'replay.py', 'probe-native-pairs-r1.py', 'review-native-preflight.py'):
            with self.subTest(name=name):
                result = self.cli(name, '--help', optimized=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b'assertions enabled', result.stderr)

    def test_wrong_viewed_png_hash_cannot_publish_native_admission(self):
        (self.native / 'preflight.png').write_bytes(b'public non-image refusal fixture\n')
        result = self.cli('review-native-preflight.py', '--native', self.native,
                          '--viewed-png-sha256', '0' * 64)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.native / 'continue-native.json').exists())


if __name__ == '__main__':
    unittest.main()
