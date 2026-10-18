import ast
import copy
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import rf312_rejected_off_queue as audit
from test_rf312_rejected_queue import reports as on_reports


def reports():
    parent, small, large, _ = on_reports()
    parent['journeyMemorySamplingEnabled'] = False
    return parent, small, large, list(audit.PHASES[:9])


class RejectedOffQueueTests(unittest.TestCase):
    def test_actual_off_failure_is_accepted_only_as_an_unchanged_prefix(self):
        data = reports(); before = copy.deepcopy(data)
        audit.check_failure(*data)
        self.assertEqual(data, before)

    def test_other_modes_or_failed_stage_prefixes_are_not_substituted(self):
        for change in [lambda d: d[0].update(journeyMemorySamplingEnabled=True),
                       lambda d: d[3].pop(), lambda d: d[3].append(audit.PHASES[9])]:
            data = reports(); change(data)
            with self.assertRaises(ValueError): audit.check_failure(*data)

    def test_complete_or_unrestored_queues_and_changed_budgets_are_rejected(self):
        for change in [lambda d: d[0].update(passed=True),
                       lambda d: d[0]['cleanup'].update(freshHostAfterPassed=False),
                       lambda d: d[1].update(originalApksRestored=False),
                       lambda d: d[2].update(sourceRestoreError='refused'),
                       lambda d: d[2].update(error='dumpsys processes timed out after 10 seconds')]:
            data = reports(); change(data)
            with self.assertRaises(ValueError): audit.check_failure(*data)

    def test_complete_cell_keeps_every_original_inner_validator_guard(self):
        for name in ['state', 'ime', 'ui']:
            with self.subTest(name=name):
                original = (audit.ROOT / f'replay_{name}.py').read_text(encoding='utf-8')
                derived = audit.derive(name, 100)
                def inner(source):
                    loop = next(n for n in ast.parse(source).body if isinstance(n, ast.For)
                                and isinstance(n.target, ast.Name) and n.target.id == 'count')
                    return ast.dump(ast.Module(body=loop.body, type_ignores=[]))
                self.assertEqual(inner(original), inner(derived))
                self.assertIn('nativeCompleteQueueAccepted=False', derived)
                self.assertIn('rf312_rejected_off_queue import load_context', derived)
                compile(derived, 'off-profile-reader', 'exec')

    def test_failed_cell_keeps_two_real_typings_five_proofs_and_nine_stages(self):
        ime, ui = audit.derive('ime', 5000), audit.derive('ui', 5000)
        self.assertIn('assert len(typed)==2', ime)
        self.assertIn('typingActions=2', ime)
        self.assertIn('assert content==5', ui)
        self.assertIn('uiStages=9', ui)
        self.assertIn("read(J/'partial-state.json')", ui)
        for source in [ime, ui]:
            self.assertIn('nativeCompleteQueueAccepted=False', source)
            compile(source, 'failed-off-profile-reader', 'exec')

    def test_failed_state_or_any_memory_trajectory_cannot_enter_complete_readers(self):
        for name, count in [('state', 5000), ('memory', 100), ('memory', 5000)]:
            with self.assertRaises(ValueError): audit.derive(name, count)

    def test_optimized_cli_refuses_before_creating_any_output(self):
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
