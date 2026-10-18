import hashlib
import unittest
import trace_journey as m

class MainCmdline(unittest.TestCase):
    def test_single_terminator_still_accepted(self):
        self.assertEqual(m.require_main_cmdline(m.APP.encode()+b'\0')['trailingNulBytes'],1)
    def test_known_null_padded_argument_block_matches_original_digest(self):
        raw=m.APP.encode()+b'\0'*83;digest=hashlib.sha256(raw).hexdigest()
        self.assertEqual(digest,'770d246a36c80fb1d31bcf8df7ad4f695957321249f9fc2a009b8d996b878c76')
        self.assertEqual(m.require_main_cmdline(raw),dict(argumentBlockBytes=99,trailingNulBytes=83))
    def test_nonempty_additional_argument_rejected(self):
        for tail in (b'\0other\0',b'\0 \0',b'\0\xff\0',b'\0com.solosoul.app\0'):
            with self.subTest(tail=tail),self.assertRaises(ValueError):m.require_main_cmdline(m.APP.encode()+tail)
    def test_wrong_name_prefix_and_suffix_rejected(self):
        for name in (b'xcom.solosoul.app',b'com.solosoul.app.test',b'com.solosoul.ap',b'com.solosoul.App'):
            with self.subTest(name=name),self.assertRaises(ValueError):m.require_main_cmdline(name+b'\0')
    def test_missing_terminator_rejected(self):
        with self.assertRaises(ValueError):m.require_main_cmdline(m.APP.encode())
    def test_empty_text_and_nonbytes_rejected(self):
        for raw in (b'',b'\0'*99,'com.solosoul.app\0',None,True,bytearray(m.APP.encode()+b'\0')):
            with self.subTest(raw=raw),self.assertRaises(ValueError):m.require_main_cmdline(raw)
    def test_input_bound_kept_at_4096(self):
        raw=m.APP.encode()+b'\0'*(4096-len(m.APP.encode()))
        self.assertEqual(m.require_main_cmdline(raw)['argumentBlockBytes'],4096)
        with self.assertRaises(ValueError):m.require_main_cmdline(raw+b'\0')

if __name__=='__main__':unittest.main()
