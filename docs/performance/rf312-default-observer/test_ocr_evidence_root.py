import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

from live_pixel_counter import NativeOcrReader
from default_journey import Journey

S = Path(__file__).parent


class EvidenceRoot(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory();self.addCleanup(temp.cleanup)
        self.folder = Path(temp.name).resolve()
        self.root = self.folder/'public evidence';self.root.mkdir()
        self.image = self.root/'image.png';self.image.write_bytes(b'public test image')
        self.receipt = self.root/'receipt.json'
        self.bindings = {'read-public-counter.ps1': hashlib.sha256((S/'read-public-counter.ps1').read_bytes()).hexdigest()}
        self.calls = []
        self.windows = self.folder/'Windows'
        self.environment = mock.patch.dict(os.environ, {'SystemRoot': str(self.windows), 'PSModulePath': 'parent-modules'})
        self.environment.start();self.addCleanup(self.environment.stop)

    def capture(self, command, **kwargs):
        self.calls.append((command, kwargs))
        self.receipt.write_text(json.dumps({'native': True}), encoding='utf-8')
        return subprocess.CompletedProcess(command, 0, b'original stdout', b'')

    def reader(self, *, runner=None, clock=lambda: 0):
        return NativeOcrReader(self.bindings, self.root, clock=clock, runner=runner or self.capture)

    def test_root_is_mandatory_existing_directory_without_a_default(self):
        with self.assertRaises(TypeError):NativeOcrReader(self.bindings)
        for root in (self.folder/'missing', self.image):
            with self.subTest(root=root), self.assertRaises(ValueError):NativeOcrReader(self.bindings, root)

    def test_exact_root_is_forwarded_to_native_script_with_system_winrt_host(self):
        self.assertEqual(self.reader()(self.image, self.receipt, 45), {'native': True})
        command, options = self.calls[0]
        modules = self.windows/'System32/WindowsPowerShell/v1.0'
        self.assertEqual(command[0], str(modules/'powershell.exe'))
        self.assertEqual(command[-2:], ['-EvidenceRoot', str(self.root)])
        self.assertEqual(options['env']['PSModulePath'], str(modules/'Modules'))
        self.assertEqual(options['env']['TEMP'], str(self.root))
        self.assertEqual(options['env']['TMP'], str(self.root))
        self.assertEqual(options['timeout'], 10)
        self.assertEqual(os.environ['PSModulePath'], 'parent-modules')

    def test_image_outside_root_is_refused_before_native_read(self):
        outside = self.folder/'outside.png';outside.write_bytes(b'public outside')
        with self.assertRaisesRegex(ValueError, 'outside'):self.reader()(outside, self.receipt, 45)
        self.assertEqual(self.calls, [])

    def test_sibling_with_same_text_prefix_is_not_inside_root(self):
        sibling = self.folder/'public evidence sibling';sibling.mkdir()
        outside = sibling/'image.png';outside.write_bytes(b'public sibling')
        with self.assertRaisesRegex(ValueError, 'outside'):self.reader()(outside, self.receipt, 45)
        self.assertEqual(self.calls, [])

    def test_output_outside_root_cannot_create_logs_or_receipt(self):
        outside = self.folder/'outside.json'
        with self.assertRaisesRegex(ValueError, 'outside'):self.reader()(self.image, outside, 45)
        self.assertFalse(outside.exists());self.assertEqual(self.calls, [])

    def test_dotdot_alias_is_not_accepted_as_an_owned_path(self):
        alias = self.root/'unused'/'..'/'receipt.json'
        with self.assertRaisesRegex(ValueError, 'outside'):self.reader()(self.image, alias, 45)
        self.assertEqual(self.calls, [])

    def test_existing_output_is_preserved_and_not_passed_to_runner(self):
        self.receipt.write_bytes(b'keep original')
        with self.assertRaisesRegex(RuntimeError, 'Fresh'):self.reader()(self.image, self.receipt, 45)
        self.assertEqual(self.receipt.read_bytes(), b'keep original');self.assertEqual(self.calls, [])

    def test_source_binding_refusal_precedes_native_read(self):
        self.bindings['read-public-counter.ps1'] = '0'*64
        with self.assertRaisesRegex(AssertionError, 'Frozen'):self.reader()(self.image, self.receipt, 45)
        self.assertEqual(self.calls, [])

    def test_missing_system_root_has_no_powershell_or_drive_fallback(self):
        with mock.patch.dict(os.environ, {'SystemRoot': ''}):
            with self.assertRaisesRegex(ValueError, 'SystemRoot'):self.reader()(self.image, self.receipt, 45)
        self.assertEqual(self.calls, [])

    def test_remaining_phase_time_caps_the_native_call(self):
        self.reader()(self.image, self.receipt, 2.5)
        self.assertEqual(self.calls[0][1]['timeout'], 2.5)

    def test_expired_root_validation_does_not_start_native_work(self):
        ticks = iter([0, 46])
        with self.assertRaises(TimeoutError):self.reader(clock=lambda: next(ticks))(self.image, self.receipt, 45)
        self.assertEqual(self.calls, [])

    def test_native_nonzero_keeps_raw_output_and_is_not_retried(self):
        def failed(command, **kwargs):
            self.calls.append((command, kwargs))
            return subprocess.CompletedProcess(command, 1, b'partial native output', b'original failure')
        with self.assertRaises(subprocess.CalledProcessError):self.reader(runner=failed)(self.image, self.receipt, 45)
        self.assertEqual(len(self.calls), 1)
        self.assertEqual(self.receipt.with_suffix('.stdout.log').read_bytes(), b'partial native output')
        self.assertEqual(self.receipt.with_suffix('.stderr.log').read_bytes(), b'original failure')
        self.assertFalse(self.receipt.exists())

    def test_late_native_result_does_not_become_an_accepted_receipt(self):
        ticks = iter([0, 0, 46])
        with self.assertRaises(TimeoutError):self.reader(clock=lambda: next(ticks))(self.image, self.receipt, 45)
        self.assertEqual(len(self.calls), 1)
        self.assertTrue(self.receipt.exists())

    def test_journey_supplies_its_own_actual_output_root(self):
        journey = Journey(None, None, self.root, {'fullJourneyFiles': {}, 'pixelInputFiles': self.bindings})
        self.assertEqual(journey.ocr.root, self.root)


if __name__ == '__main__':
    unittest.main()
