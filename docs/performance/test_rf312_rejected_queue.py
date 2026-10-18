import ast
import copy
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import rf312_rejected_queue as audit


def reports():
    parent = dict(passed=False, restored=True, allFrozenInputsPreserved=True,
        supportedDefaultStateAccepted=False, fullJourneyAccepted=False,
        journeyMemorySamplingEnabled=True, ownedUiTransportEnabled=True,
        outcomes=[dict(count=100, passed=True)], cleanup={key: True for key in [
            'ownedPackagesRemoved', 'settingsExactlyUnchanged', 'normalShutdownRequested',
            'launcherExited', 'serverExited', 'freshHostAfterPassed']})
    base = {key: True for key in ['restored', 'originalApksRestored', 'blurRestored',
                                 'ownedProbeRemoved', 'ownedPublicOcrInputRemoved', 'frozenInputsUnchanged']}
    small = dict(base, passed=True, defaultStateAdapterAccepted=True)
    large = dict(base, passed=False, defaultStateAdapterAccepted=False, errorType='TimeoutExpired',
                 error="dumpsys activity processes timed out after 5 seconds")
    return parent, small, large, ['startup', 'password-unlock']


class RejectedQueueTests(unittest.TestCase):
    def test_explicit_failure_scope_accepts_actual_shape_without_mutation(self):
        data = reports(); previous = copy.deepcopy(data)
        audit.check_failure(*data)
        self.assertEqual(data, previous)

    def test_complete_queue_cannot_enter_rejected_audit(self):
        data = reports(); data[0]['passed'] = True
        with self.assertRaises(ValueError): audit.check_failure(*data)

    def test_restore_frozen_source_and_lifecycle_failures_remain_closed(self):
        for key in ['restored', 'allFrozenInputsPreserved', 'journeyMemorySamplingEnabled', 'ownedUiTransportEnabled']:
            with self.subTest(key=key):
                data = reports(); data[0][key] = False
                with self.assertRaises(ValueError): audit.check_failure(*data)
        data = reports(); data[0]['cleanup']['freshHostAfterPassed'] = False
        with self.assertRaises(ValueError): audit.check_failure(*data)

    def test_other_completed_cells_or_failed_phase_prefixes_are_not_substituted(self):
        for mutate in [lambda d: d[0]['outcomes'].append(dict(count=5000, passed=True)),
                       lambda d: d[3].append('first-ocr-user-flow'),
                       lambda d: d[2].update(error='timed out after 10 seconds'),
                       lambda d: d[2].update(passed=True)]:
            data = reports(); mutate(data)
            with self.assertRaises(ValueError): audit.check_failure(*data)

    def test_complete_cell_derivatives_keep_all_original_inner_guards(self):
        for name in ['state', 'ime', 'ui', 'memory']:
            with self.subTest(name=name):
                source = (audit.ROOT / f'replay_{name}.py').read_text(encoding='utf-8')
                derived = audit.derive(name, 100)
                def body(text):
                    node = next(n for n in ast.parse(text).body if isinstance(n, ast.For)
                                and isinstance(n.target, ast.Name) and n.target.id == 'count')
                    return ast.dump(ast.Module(body=node.body, type_ignores=[]))
                self.assertEqual(body(source), body(derived))
                self.assertIn('nativeCompleteQueueAccepted=False', derived)
                self.assertIn('auditedObjectCount=100', derived)
                compile(derived, 'scoped-reader', 'exec')

    def test_failed_cell_scopes_typing_and_physical_proofs_without_full_claim(self):
        ime, ui = audit.derive('ime', 5000), audit.derive('ui', 5000)
        self.assertIn('assert len(typed)==1', ime)
        self.assertIn('typingActions=1', ime)
        self.assertIn('assert content==0', ui)
        self.assertIn('uiStages=2', ui)
        self.assertIn("read(J/'partial-state.json')", ui)
        for source in [ime, ui]:
            self.assertIn('nativeCompleteQueueAccepted=False', source)
            compile(source, 'failed-cell-reader', 'exec')
        for name in ['state', 'memory']:
            with self.assertRaises(ValueError): audit.derive(name, 5000)

    def test_validator_rewrite_refuses_missing_or_duplicate_source_anchor(self):
        for value in ['different', 'anchor anchor']:
            with self.assertRaises(ValueError): audit.replace_once(value, 'anchor', 'replacement')

    def test_interrupted_state_prefix_has_no_invented_final_absence(self):
        sdk = [dict(exitCode=0)] * 3 + [dict(exitCode=None, timedOut=True)]
        self.assertEqual(audit.check_state_prefix([1, 2, 3], [0, 1, 1], sdk), [4])
        with self.assertRaises(ValueError): audit.check_state_prefix([1, 2, 3], [0, 1, 0], sdk)

    def test_failed_state_slot_requires_its_real_sdk_timeout_and_no_renumbering(self):
        sdk = [dict(exitCode=0, timedOut=False)] * 3 + [dict(exitCode=None, timedOut=True)]
        with self.assertRaises(ValueError): audit.check_state_prefix([1, 2, 4], [0, 1, 1], sdk)
        with self.assertRaises(ValueError): audit.check_state_prefix([1, 2, 3], [0, 1, 1], sdk[:-1])

    def test_optimized_cli_refuses_before_output_or_native_admission(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'new-output'
            result = subprocess.run([sys.executable, '-B', '-O', str(Path(audit.__file__).resolve()),
                '--native', directory, '--adapter-inputs', 'missing', '--unit-evidence', 'missing', '--output', str(output)],
                capture_output=True, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b'assertions enabled', result.stderr)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
