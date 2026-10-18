import os
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent


class IndependentEntries(unittest.TestCase):
    def test_all_entry_points_reach_help_with_standalone_imports(self):
        for name in ('trace_native_owner.py', 'trace_driver.py', 'prepare_journey.py'):
            with self.subTest(entry=name):
                result = subprocess.run([sys.executable, '-B', str(ROOT/name), '--help'],
                    cwd=ROOT, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'),
                    capture_output=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stderr, b'')
                self.assertIn(b'--', result.stdout)

    def test_original_driver_exception_and_restoration_tail_preserved(self):
        original = (ROOT.parent/'rf312_default_state_driver.py').read_bytes()
        current = (ROOT/'trace_driver.py').read_bytes()
        marker = b'    except Exception as error:'
        self.assertIn(marker, original)
        self.assertEqual(current[current.index(marker):], original[original.index(marker):])

    def test_original_host_shutdown_block_preserved(self):
        original = (ROOT.parent/'rf312-native-queue/owner.py').read_bytes()
        current = (ROOT/'trace_native_owner.py').read_bytes()
        start, end = b'        if emulator is not None:', b"        record['cleanup']=cleanup"
        self.assertEqual(current[current.index(start):current.index(end)],
                         original[original.index(start):original.index(end)])


if __name__ == '__main__':
    unittest.main()
