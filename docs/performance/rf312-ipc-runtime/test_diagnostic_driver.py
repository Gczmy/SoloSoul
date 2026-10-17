"""Exercise the public entry from a relocated tree and check runtime globals."""
import builtins
from pathlib import Path
import os
import shutil
import subprocess
import symtable
import sys
import tempfile
import unittest

import diagnostic_binding as binding
import diagnostic_driver as driver
import diagnostic_owner as owner
import diagnostic_journey as journey
import diagnostic_ui_session as ui
import native_collector as native
import diagnostic_exchange as exchange


class PublicEntryTests(unittest.TestCase):
    def test_runtime_entrypoints_have_all_required_global_bindings(self):
        def required(scope):
            result = {symbol.get_name() for symbol in scope.get_symbols()
                      if symbol.is_global() and symbol.is_referenced()}
            for child in scope.get_children():
                result.update(required(child))
            return result
        for module, name in [(driver, 'main'), (owner, 'main'), (journey, 'observe'),
                             (journey, 'target_session'), (journey, 'Journey'),
                             (ui, 'session'), (native, 'Collector'),
                             (binding, 'load'), (binding, 'verify'), (exchange, 'Control'),
                             (exchange, 'Adapter'), (exchange, 'Client'), (exchange, 'binding'),
                             (exchange, 'request_line')]:
            table = symtable.symtable(Path(module.__file__).read_text(encoding='utf-8'),
                                      module.__file__, 'exec')
            entry = next(child for child in table.get_children() if child.get_name() == name)
            with self.subTest(module=module.__name__, entry=name):
                self.assertEqual(required(entry) - set(vars(module)) - set(vars(builtins)), set())

    def test_direct_cli_from_space_path_has_no_private_import_or_sdk_fallback(self):
        with tempfile.TemporaryDirectory(prefix='rf312 public ipc ') as temporary:
            root = Path(temporary)
            public = root / 'relocated performance'
            runtime = public / 'rf312-ipc-runtime'
            runtime.mkdir(parents=True)
            for source in binding.PUBLIC.glob('*.py'):
                shutil.copyfile(source, public / source.name)
            for folder in ['rf312-default-observer', 'rf312-ipc-collector']:
                shutil.copytree(binding.PUBLIC / folder, public / folder)
            for name in binding.LOCAL:
                source = binding.ROOT / name
                shutil.copyfile(source, runtime / source.name)
            environment = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
            environment.pop('PYTHONPATH', None)
            for entry in ['diagnostic_driver.py', 'diagnostic_owner.py']:
                with self.subTest(entry=entry):
                    result = subprocess.run([sys.executable, '-B', str(runtime / entry), '--help'],
                                            cwd=root, env=environment, capture_output=True, timeout=15)
                    self.assertEqual(result.returncode, 0, result.stderr.decode(errors='replace'))
                    self.assertIn(b'--diagnostic-inputs', result.stdout)
                    if entry == 'diagnostic_owner.py':
                        self.assertIn(b'--powershell-sha256', result.stdout)
                    self.assertEqual(result.stderr, b'')
            self.assertFalse(list(root.rglob('__pycache__')))


if __name__ == '__main__':
    unittest.main()
