import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import rf312_forensic_inventory as module


class ForensicStreamingTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.path=self.root/'disk.img'

    def test_empty_and_multiple_blocks_have_exact_digest(self):
        for raw in [b'',b'public fixture',b'a'*(module.BLOCK+31)]:
            self.path.write_bytes(raw)
            self.assertEqual(module.digest_file(self.path),hashlib.sha256(raw).hexdigest())

    def test_every_read_is_bounded_and_no_read_bytes_call_is_used(self):
        self.path.write_bytes(b'x'*(module.BLOCK+17));original=Path.open;calls=[]
        class Reader:
            def __init__(self,stream):self.stream=stream
            def __enter__(self):return self
            def __exit__(self,*args):self.stream.close()
            def fileno(self):return self.stream.fileno()
            def readable(self):return True
            def readinto(self,buffer):
                if not 0<len(buffer)<=module.BLOCK:raise AssertionError('Unbounded read')
                calls.append(len(buffer));return self.stream.readinto(buffer)
        def opened(path,*args,**kwargs):
            stream=original(path,*args,**kwargs)
            return Reader(stream) if path==self.path else stream
        with patch.object(Path,'open',opened),patch.object(Path,'read_bytes',side_effect=AssertionError('Whole-file read forbidden')):
            module.digest_file(self.path)
        self.assertGreater(len(calls),1)
        self.assertLessEqual(max(calls),module.BLOCK)

    def test_change_during_hash_is_rejected(self):
        self.path.write_bytes(b'x'*(module.BLOCK+17));original=Path.open;target=self.path
        class Reader:
            def __init__(self,stream):self.stream=stream;self.first=True
            def __enter__(self):return self
            def __exit__(self,*args):self.stream.close()
            def fileno(self):return self.stream.fileno()
            def readable(self):return True
            def readinto(self,buffer):
                size=self.stream.readinto(buffer)
                if self.first:
                    self.first=False
                    with original(target,'ab') as writer:writer.write(b'changed')
                return size
        def opened(path,*args,**kwargs):
            stream=original(path,*args,**kwargs)
            return Reader(stream) if path==target and args==('rb',) else stream
        with patch.object(Path,'open',opened),self.assertRaises(ValueError):module.digest_file(self.path)

    def test_inventory_preserves_complete_names_directories_and_empty_files(self):
        (self.root/'nested').mkdir();(self.root/'empty').write_bytes(b'');(self.root/'nested/file').write_bytes(b'fixture')
        value=module.inventory(self.root)
        self.assertEqual(value['directories'],['nested'])
        self.assertEqual(value['files'],{'empty':hashlib.sha256(b'').hexdigest(),'nested/file':hashlib.sha256(b'fixture').hexdigest()})

    def test_executable_filename_and_handle_modes_bind_independently(self):
        path=self.root/'tool.exe';path.write_bytes(b'public executable fixture')
        self.assertEqual(module.digest_file(path),hashlib.sha256(b'public executable fixture').hexdigest())


if __name__=='__main__':unittest.main()
