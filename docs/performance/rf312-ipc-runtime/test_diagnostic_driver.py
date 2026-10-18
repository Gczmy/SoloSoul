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
    def test_optimized_clis_refuse_before_help_receipts_or_output_creation(self):
        with tempfile.TemporaryDirectory(prefix='rf1187 optimized CLI ') as temporary:
            root = Path(temporary)
            arguments = {
                'diagnostic_owner.py': ['--kind', 'diagnostic-full', '--preparation', str(root / 'missing-avd.json'),
                    '--diagnostic-inputs', str(root / 'missing-diagnostic.json'), '--powershell', str(root / 'missing-pwsh.exe'),
                    '--powershell-sha256', 'a' * 64],
                'diagnostic_driver.py': ['--adb', str(root / 'platform-tools/adb.exe'), '--serial', 'emulator-5578',
                    '--avd', 'SoloSoul_RF1187_PublicGuard', '--apk', str(root / 'missing-app.apk'),
                    '--test-apk', str(root / 'missing-test.apk'), '--mode', 'supported',
                    '--restore-apk', str(root / 'missing-original.apk'), '--object-count', '100',
                    '--fixture', str(root / 'missing-public-fixture'), '--inputs', str(root / 'missing-runtime.json'),
                    '--diagnostic-inputs', str(root / 'missing-diagnostic.json')],
            }
            for entry, declared in arguments.items():
                for optimize, option, inherited in [
                    ('flag-O', ['-O'], None), ('flag-OO', ['-OO'], None),
                    ('env-1', [], '1'), ('env-2', [], '2'),
                ]:
                    for help_only in [True, False]:
                        with self.subTest(entry=entry, optimize=optimize, help=help_only):
                            output = root / (entry + optimize + ('-help' if help_only else '-admission'))
                            cli = ['--help'] if help_only else [*declared, '--output', str(output)]
                            environment = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
                            environment.pop('PYTHONOPTIMIZE', None)
                            if inherited is not None:
                                environment['PYTHONOPTIMIZE'] = inherited
                            result = subprocess.run([sys.executable, '-B', *option, str(binding.ROOT / entry), *cli],
                                cwd=root, env=environment,
                                capture_output=True, timeout=15,
                                creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
                            self.assertNotEqual(result.returncode, 0)
                            self.assertEqual(result.stdout, b'')
                            self.assertIn(b'Python assertions enabled', result.stderr)
                            self.assertFalse(output.exists())
            self.assertFalse(list(root.rglob('__pycache__')))

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
