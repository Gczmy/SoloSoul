from pathlib import Path
import json,os,subprocess,sys,tempfile,unittest
from timeout_streams import preserve_timeout


class TimeoutEvidence(unittest.TestCase):
    def test_partial_bytes_are_preserved_exactly(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix=Path(folder)/'command';data=bytes(range(256))
            value=preserve_timeout(prefix,subprocess.TimeoutExpired(['public-tool'],45,output=data,stderr=b'error\r\n'))
            self.assertEqual(prefix.with_suffix('.partial.stdout').read_bytes(),data)
            self.assertEqual(value['streams']['stdout']['bytes'],256)
            self.assertTrue(value['partialObservedOnly'])

    def test_unavailable_is_not_fabricated_as_empty(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix=Path(folder)/'command'
            value=preserve_timeout(prefix,subprocess.TimeoutExpired(['public-tool'],45))
            self.assertFalse(value['streams']['stdout']['available'])
            self.assertIsNone(value['streams']['stdout']['sha256'])
            self.assertFalse(prefix.with_suffix('.partial.stdout').exists())

    def test_observed_empty_stream_stays_distinct(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix=Path(folder)/'command'
            value=preserve_timeout(prefix,subprocess.TimeoutExpired(['public-tool'],45,output=b''))
            self.assertTrue(value['streams']['stdout']['available'])
            self.assertEqual(prefix.with_suffix('.partial.stdout').read_bytes(),b'')
            self.assertIsNone(value['exitCode'])

    def test_duplicate_receipt_does_not_overwrite_bytes(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix=Path(folder)/'command'
            preserve_timeout(prefix,subprocess.TimeoutExpired(['public-tool'],45,output=b'first'))
            with self.assertRaises(FileExistsError):
                preserve_timeout(prefix,subprocess.TimeoutExpired(['public-tool'],45,output=b'second'))
            self.assertEqual(prefix.with_suffix('.partial.stdout').read_bytes(),b'first')

    def test_text_stream_is_rejected_without_output(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix=Path(folder)/'command'
            with self.assertRaises(ValueError):
                preserve_timeout(prefix,subprocess.TimeoutExpired(['public-tool'],45,output='decoded text'))
            self.assertFalse(prefix.with_suffix('.timeout.json').exists())

    def test_other_exception_is_rejected(self):
        with self.assertRaises(ValueError):preserve_timeout(Path('unused'),ValueError('unrelated'))

    def test_actual_host_subprocess_timeout_is_preserved(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix=Path(folder)/'real command'
            code="import os,time;os.write(1,b'RF312-public-start\\x00\\xff');os.write(2,b'public-stderr\\r\\n');time.sleep(5)"
            with self.assertRaises(subprocess.TimeoutExpired) as caught:
                subprocess.run([sys.executable,'-B','-c',code],capture_output=True,timeout=1)
            value=preserve_timeout(prefix,caught.exception)
            self.assertEqual(prefix.with_suffix('.partial.stdout').read_bytes(),b'RF312-public-start\x00\xff')
            self.assertEqual(prefix.with_suffix('.partial.stderr').read_bytes(),b'public-stderr\r\n')
            self.assertEqual(value['timeoutSeconds'],1)
            self.assertEqual(json.loads(prefix.with_suffix('.timeout.json').read_bytes()),value)


if __name__=='__main__':unittest.main()
