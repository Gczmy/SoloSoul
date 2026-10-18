import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import prepare_journey as m
import trace_binding


class JourneyPreparation(unittest.TestCase):
    def case(self):
        temporary = tempfile.TemporaryDirectory(prefix='rf312 journey ')
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        runtime, adapter, pwsh = (root / name for name in ('runtime.json', 'state.json', 'pwsh.exe'))
        runtime.write_bytes(b'{"public":true}')
        adapter.write_bytes(b'{}')
        pwsh.write_bytes(b'public test executable, never run')
        state = dict(runtime={'public': True}, ownedUi={'public': True}, config={'runtimeInputs': str(runtime)})
        return root, runtime, adapter, pwsh, state

    def prepared(self):
        root, runtime, adapter, pwsh, state = self.case()
        with patch.object(m, 'load_state', return_value=state):
            path = m.prepare(runtime, adapter, pwsh, root / 'new capture')
        return path, json.loads(path.read_bytes())

    def test_prepares_distinct_bound_sizes_without_launching_executable(self):
        path, plan = self.prepared()
        self.assertEqual(set(plan['inputCases']), {'100', '5000'})
        self.assertEqual(trace_binding.load(path), plan)
        roots = [Path(row['root']) for row in plan['inputCases'].values()]
        self.assertNotEqual(roots[0], roots[1])
        for root in roots:
            body = (root / 'trace-config.pbtxt').read_bytes()
            self.assertIn(b'duration_ms: 240000\n', body)
            self.assertIn(b'max_file_size_bytes: 33554432\n', body)
        self.assertNotEqual((roots[0]/'trace-config.pbtxt').read_bytes(), (roots[1]/'trace-config.pbtxt').read_bytes())

    def test_existing_output_rejected_before_state_loading(self):
        root, runtime, adapter, pwsh, state = self.case()
        with patch.object(m, 'load_state') as load:
            with self.assertRaises(ValueError):
                m.prepare(runtime, adapter, pwsh, root)
            load.assert_not_called()

    def test_runtime_mismatch_refused_before_creating_output(self):
        root, runtime, adapter, pwsh, state = self.case()
        state['runtime'] = {'public': False}
        destination = root / 'never created'
        with patch.object(m, 'load_state', return_value=state), self.assertRaises(ValueError):
            m.prepare(runtime, adapter, pwsh, destination)
        self.assertFalse(destination.exists())

    def test_default_without_owned_ui_refused(self):
        root, runtime, adapter, pwsh, state = self.case()
        state.pop('ownedUi')
        with patch.object(m, 'load_state', return_value=state), self.assertRaises(ValueError):
            m.prepare(runtime, adapter, pwsh, root / 'never created')

    def test_changed_configuration_refused(self):
        path, plan = self.prepared()
        folder = Path(plan['inputCases']['100']['root'])
        (folder / 'trace-config.pbtxt').write_bytes(b'changed')
        with self.assertRaises(ValueError):
            trace_binding.load(path)

    def test_unexpected_input_file_refused(self):
        path, plan = self.prepared()
        (Path(plan['inputCases']['100']['root']) / 'extra').write_bytes(b'public')
        with self.assertRaises(ValueError):
            trace_binding.load(path)

    def test_same_case_root_refused(self):
        path, plan = self.prepared()
        plan['inputCases']['5000'] = copy.deepcopy(plan['inputCases']['100'])
        with self.assertRaises(ValueError):
            trace_binding.verify(plan)

    def test_unexecuted_name_alias_cannot_replace_bound_local_module(self):
        path, plan = self.prepared()
        name = str(m.ROOT / 'trace_journey.py')
        digest = plan['sources'].pop(name)
        alias = path.parent / 'trace_journey.py'
        alias.write_bytes((m.ROOT / 'trace_journey.py').read_bytes())
        plan['sources'][str(alias)] = digest
        with self.assertRaises(ValueError):
            trace_binding.verify(plan)

    def test_unexecuted_original_name_alias_cannot_replace_reader(self):
        path, plan = self.prepared()
        name = str(m.PUBLIC / 'rf312_default_journey.py')
        digest = plan['originalSources'].pop(name)
        alias = path.parent / 'rf312_default_journey.py'
        alias.write_bytes((m.PUBLIC / 'rf312_default_journey.py').read_bytes())
        plan['originalSources'][str(alias)] = digest
        with self.assertRaises(ValueError):
            trace_binding.verify(plan)


if __name__ == '__main__':
    unittest.main()
