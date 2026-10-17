import base64
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from persistent_control import Control,read_reply,request_line

def frame(sequence,code=0,out=b'',err=b'',ending=b'\n'):
    return (f'RF312B {sequence} {code} {len(out)} {len(err)}'.encode()+ending+
            b'O '+base64.b64encode(out)+ending+b'E '+base64.b64encode(err)+ending)

class Frames(unittest.TestCase):
    def test_all_binary_bytes_and_content_line_endings_preserved(self):
        out=bytes(range(256))+b'\x00\r\n\n\rpublic'
        for ending in (b'\n',b'\r\n'):
            raw=frame(12,1,out,b'err\r\n',ending);partial=bytearray();stream=io.BytesIO(raw)
            self.assertEqual(read_reply(stream,12,partial),(1,out,b'err\r\n'))
            self.assertEqual(bytes(partial),raw);self.assertEqual(stream.read(),b'')

    def test_sequence_bounds_and_legacy_protocol_refused(self):
        for raw in (b'',b'RF312 1 0 0 0\n',frame(2),b'RF312B 1 0 4194305 0\n',
                    b'RF312B 1 256 0 0\n',b'RF312B 1 0 0 65537\n',b'RF312B 01 0 0 0\n',b'x'*200):
            with self.subTest(raw=raw),self.assertRaises(RuntimeError):read_reply(io.BytesIO(raw),1,bytearray())

    def test_malformed_noncanonical_or_truncated_payload_refused(self):
        for raw in (b'RF312B 1 0 1 0\nO !!!=\nE \n',b'RF312B 1 0 1 0\nO Zh==\nE \n',
                    b'RF312B 1 0 2 0\nO Zg==\nE \n',b'RF312B 1 0 1 0\nX Zg==\nE \n',
                    b'RF312B 1 0 1 0\nO Zg==',b'RF312B 1 0 1 0\nO Z g=\nE \n',
                    b'RF312B 1 0 0 0\nO \n',b'RF312B 1 0 0 0\nO \r\r\nE \n'):
            with self.subTest(raw=raw),self.assertRaises(RuntimeError):read_reply(io.BytesIO(raw),1,bytearray())

    def test_exact_maximum_binary_payload_stays_bounded(self):
        raw=frame(1,out=b'\x00'*4194304,err=b'\xff'*65536)
        code,out,err=read_reply(io.BytesIO(raw),1,bytearray())
        self.assertEqual((code,len(out),len(err)),(0,4194304,65536))

    def test_request_is_ascii_and_binary_payload_encoded(self):
        self.assertEqual(request_line(1,'put','request-001.json',b'\x00\n"'),b'RF312B 1 put request-001.json AAoi\n')
        self.assertEqual(request_line(2,'exists','error'),b'RF312B 2 exists error -\n')

    def test_unsafe_paths_commands_types_and_payloads_refused(self):
        invalid=[(1,'read','../ready',b''),(1,'read','ready;id',b''),(1,'eval','-',b''),
            (1,'read','frame-9999.png',b''),(1,'exists','frame-001.png',b''),
            (1,'put','request-001.json',b'x'*16385),(1,'focus','-',b'x'),(True,'ping','-',b''),
            (16385,'ping','-',b''),(1,'put','request-001.json',b''),(1,'stop','error',b''),
            (1,'read',None,b''),(1,'read','response-٠٠١.json',b'')]
        for args in invalid:
            with self.subTest(args=args[:3]),self.assertRaises(ValueError):request_line(*args)

class Pipe(unittest.TestCase):
    def actor(self,source):
        temporary=tempfile.TemporaryDirectory(prefix='rf312-control-',dir=Path(__file__).parent)
        self.addCleanup(temporary.cleanup)
        process=subprocess.Popen([sys.executable,'-B','-u','-c',source],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        control=Control(process,Path(temporary.name)/'receipts');self.addCleanup(control.abort)
        return control

    def test_empty_exit_one_is_absence_and_channel_stays_usable(self):
        control=self.actor("import sys\nfor line in sys.stdin.buffer:\n p=line.split();seq=p[1];op=p[2];sys.stdout.buffer.write(b'RF312B '+seq+(b' 0 0 0\\r\\nO \\r\\nE \\r\\n' if op==b'close' else b' 1 0 0\\r\\nO \\r\\nE \\r\\n'));sys.stdout.buffer.flush()\n if op==b'close':break")
        with self.assertRaises(subprocess.CalledProcessError) as caught:control.call('exists','error',timeout=3)
        self.assertEqual((caught.exception.returncode,caught.exception.stdout,caught.exception.stderr),(1,b'',b''))
        self.assertFalse(control.broken);control.close();self.assertEqual(control.sequence,2)

    def test_nonempty_error_is_not_absence_or_retried(self):
        control=self.actor("import sys\nsys.stdin.buffer.readline();sys.stdout.buffer.write("+repr(frame(1,1,err=b'fault\r\n'))+");sys.stdout.buffer.flush();sys.stdin.buffer.read()")
        with self.assertRaises(subprocess.CalledProcessError) as caught:control.call('exists','error',timeout=3)
        self.assertEqual(caught.exception.stderr,b'fault\r\n');self.assertEqual(control.sequence,1)

    def test_deadline_covers_blocked_read_and_retains_frame_prefix(self):
        control=self.actor("import sys,time\nsys.stdin.buffer.readline();sys.stdout.buffer.write(b'RF312B 1 0 10 0\\r\\nO YW');sys.stdout.buffer.flush();time.sleep(30)")
        with self.assertRaises(subprocess.TimeoutExpired) as caught:control.call('ping',timeout=.2)
        self.assertEqual(caught.exception.timeout,.2)
        self.assertTrue(caught.exception.stdout.startswith(b'RF312B 1 0 10 0\r\n'))
        self.assertTrue(control.broken);self.assertIsNotNone(control.process.poll())
        with self.assertRaises(RuntimeError):control.call('ping',timeout=1)
        self.assertEqual(control.sequence,1)

    def test_blocked_write_has_same_deadline_and_no_replay(self):
        control=self.actor('import time;time.sleep(30)')
        with self.assertRaises(subprocess.TimeoutExpired):control.call('put','request-001.json',b'x'*16384,timeout=.2)
        self.assertTrue(control.broken);self.assertIsNotNone(control.process.poll())

    def test_invalid_budget_never_sends(self):
        control=self.actor('import sys;sys.stdin.buffer.read()')
        for value in (True,0,-1,None,float('nan'),float('inf'),241):
            with self.assertRaises(ValueError):control.call('ping',timeout=value)
        self.assertEqual(control.sequence,0)

    def test_truncation_poisons_channel_and_keeps_raw_receipt(self):
        raw=b'RF312B 1 0 4 0\nO YWI='
        control=self.actor('import sys;sys.stdin.buffer.readline();sys.stdout.buffer.write('+repr(raw)+');sys.stdout.buffer.flush()')
        with self.assertRaises(RuntimeError):control.call('ping',timeout=3)
        self.assertTrue(control.broken)
        row=json.loads((control.output/'00001.json').read_text());self.assertEqual(row['errorType'],'RuntimeError')
        self.assertEqual((control.output/'00001.reply').read_bytes(),raw)

    def test_transport_stderr_is_drained_before_pipe_exit_and_retained(self):
        control=self.actor('import sys,time;sys.stdin.buffer.readline();sys.stderr.buffer.write(b"transport fault");sys.stderr.buffer.flush();time.sleep(.1);sys.stdout.buffer.write('+repr(frame(1))+');sys.stdout.buffer.flush();time.sleep(30)')
        with self.assertRaisesRegex(RuntimeError,'unexpected stderr'):control.call('ping',timeout=3)
        self.assertEqual((control.output/'transport.stderr').read_bytes(),b'transport fault')
        self.assertTrue(control.broken)

    def test_receipt_failure_preserves_original_timeout(self):
        control=self.actor('import time;time.sleep(30)')
        with patch.object(Path,'write_bytes',side_effect=OSError('public receipt failure')):
            with self.assertRaises(subprocess.TimeoutExpired) as caught:control.call('ping',timeout=.2)
        self.assertIsInstance(caught.exception.__cause__,OSError);self.assertTrue(control.broken)

if __name__=='__main__':unittest.main()
