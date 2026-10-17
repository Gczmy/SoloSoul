from pathlib import Path
import json,shlex,subprocess,tempfile,unittest
from unittest.mock import Mock

import rf312_memory_batches as batches

NONCE='a'*32
def framed(first=b'first\r\n\0raw',second=b'second\n',codes=(0,0),nonce=NONCE):
    return first+b'\0RF312PAIR '+nonce.encode()+b' 1 '+str(codes[0]).encode()+b'\0'+second+b'\0RF312PAIR '+nonce.encode()+b' 2 '+str(codes[1]).encode()+b'\0'


class FrameTests(unittest.TestCase):
    def test_raw_bytes_and_member_offsets_are_preserved(self):
        raw=framed();members=batches.split_pair(raw,NONCE)
        self.assertEqual([raw[m['stdoutOffset']:m['stdoutEnd']] for m in members],[b'first\r\n\0raw',b'second\n'])
        self.assertEqual([m['exitCode'] for m in members],[0,0])

    def test_bad_nonce_extra_frames_truncation_status_and_suffix_are_rejected(self):
        for raw in [b'',framed()[:-1],framed()+b'garbage',framed(nonce='b'*32),framed(codes=(256,0)),
                framed().replace(b' 1 0\0',b' 2 0\0'),framed()+b'\0RF312PAIR '+NONCE.encode()+b' 1 0\0']:
            with self.subTest(raw=raw[-40:]),self.assertRaises(ValueError):batches.split_pair(raw,NONCE)

    def test_member_byte_cap_is_enforced_without_truncation(self):
        with self.assertRaises(ValueError):batches.split_pair(framed(first=b'x'*(batches.LIMIT+1)),NONCE)

    def test_only_independent_inventory_and_kernel_identity_pairs_are_generated(self):
        for first in [batches.PROCESSES,('shell','cat','/proc/123/stat')]:
            pair=batches.pair_for(first);command=batches.physical_pair(pair,NONCE)
            self.assertEqual(command[:6],('shell','-T','-e','none','sh','-c'))
            lines=shlex.split(command[6])[0].splitlines();self.assertEqual(len(lines),8)
            self.assertEqual(tuple(shlex.split(lines[0])),first[1:])
            self.assertEqual(tuple(shlex.split(lines[4])),pair[1][1:])
        for command in [('shell','cat','/proc/0/stat'),('shell','cat','/proc/123;id/stat'),
                ('shell','cat','/proc/9999999999/stat'),('shell','cat','/proc/123/mem'),
                ('shell','dumpsys','meminfo','--local','-s','123')]:self.assertIsNone(batches.pair_for(command))
        with self.assertRaises(ValueError):batches.physical_pair((batches.PROCESSES,('shell','id')),NONCE)
        with self.assertRaises(ValueError):batches.physical_pair((batches.PROCESSES,batches.SERVICES),'a;id')


class AdapterTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.sdk=Mock(side_effect=self.native)
        self.adapter=batches.Adapter(self.sdk,self.root/'native',10105)
        self.codes=(0,0);self.stderr=b''

    def native(self,*command,**kwargs):
        if command[:6]==('shell','-T','-e','none','sh','-c'):
            script=shlex.split(command[6])[0];nonce=script.split('RF312PAIR ')[1].split()[0]
            return subprocess.CompletedProcess(command,0,framed(codes=self.codes,nonce=nonce),framed(first=self.stderr,second=self.stderr,codes=self.codes,nonce=nonce))
        return subprocess.CompletedProcess(command,0,b'raw\r\n\0',self.stderr)

    def execute(self,command):
        return self.adapter.run(*command,timeout=5,stdout=subprocess.PIPE,stderr=subprocess.PIPE)

    def test_two_logical_reads_use_one_sdk_call_and_exact_member_bytes(self):
        first=self.execute(batches.PROCESSES);second=self.execute(batches.SERVICES)
        self.assertEqual(first.stdout,b'first\r\n\0raw');self.assertEqual(second.stdout,b'second\n')
        self.assertEqual(self.sdk.call_count,1);self.assertEqual(second.nativeEvidence['member'],2)
        self.adapter.require_empty();report=self.adapter.close()
        self.assertEqual(report['nativeCalls'],1);self.assertEqual(report['logicalRepliesConsumed'],2)
        self.assertTrue(report['allRepliesConsumed'])

    def test_cached_reply_order_mismatch_never_resends_or_falls_back(self):
        self.execute(batches.PROCESSES)
        with self.assertRaises(RuntimeError):self.execute(('shell','ps','-A','-o','UID,PID,PPID,NAME'))
        self.assertEqual(self.sdk.call_count,1)
        with self.assertRaises(RuntimeError):self.adapter.require_empty()
        report=self.adapter.close();self.assertEqual(len(report['unconsumedPairMembers']),1)
        self.assertTrue((self.root/'native/001.stdout').exists())

    def test_second_member_failure_keeps_exact_exit_and_error_and_is_not_retried(self):
        self.codes=(0,7)
        self.execute(batches.PROCESSES)
        with self.assertRaises(subprocess.CalledProcessError) as error:self.execute(batches.SERVICES)
        self.assertEqual(error.exception.returncode,7)
        report=self.adapter.close();self.assertEqual(report['unconsumedPairMembers'],[])
        self.assertEqual(json.loads((self.root/'native/001.json').read_bytes())['members'][1]['exitCode'],7)
        self.assertEqual(self.sdk.call_count,1)

    def test_per_member_stderr_is_preserved_for_original_caller_to_refuse(self):
        self.stderr=b'native warning'
        value=self.execute(batches.PROCESSES);self.assertEqual(value.stderr,self.stderr)
        self.assertIn(self.stderr,(self.root/'native/001.stderr').read_bytes())

    def test_unframed_physical_sdk_stderr_refuses_all_logical_values(self):
        self.sdk.side_effect=lambda *args,**kwargs:subprocess.CompletedProcess(args,0,framed(),b'unframed host warning')
        with self.assertRaises(ValueError):self.execute(batches.PROCESSES)
        self.assertEqual((self.root/'native/001.stderr').read_bytes(),b'unframed host warning')
        self.assertEqual(self.adapter.consumed,[])

    def test_timeout_preserves_physical_partial_bytes_and_has_no_retry(self):
        self.sdk.side_effect=subprocess.TimeoutExpired('original-sdk',5,output=b'partial-native-pair',stderr=b'partial-error')
        with self.assertRaises(subprocess.TimeoutExpired) as error:self.execute(batches.PROCESSES)
        self.assertEqual(error.exception.nativeEvidence['kind'],'pair')
        self.assertEqual((self.root/'native/001.stdout').read_bytes(),b'partial-native-pair')
        self.assertEqual((self.root/'native/001.stderr').read_bytes(),b'partial-error')
        self.assertEqual(self.sdk.call_count,1)

    def test_stdout_stderr_status_mismatch_cannot_return_logical_values(self):
        def mismatch(*command,**kwargs):
            nonce=shlex.split(command[6])[0].split('RF312PAIR ')[1].split()[0]
            return subprocess.CompletedProcess(command,0,framed(nonce=nonce),framed(first=b'',second=b'',codes=(0,1),nonce=nonce))
        self.sdk.side_effect=mismatch
        with self.assertRaises(RuntimeError):self.execute(batches.PROCESSES)
        self.assertEqual(self.adapter.consumed,[])

    def test_oversized_member_stderr_is_kept_but_refused(self):
        self.stderr=b'x'*(batches.ERROR_LIMIT+1)
        with self.assertRaises(ValueError):self.execute(batches.PROCESSES)
        self.assertIn(self.stderr,(self.root/'native/001.stderr').read_bytes())
        self.assertEqual(self.adapter.consumed,[])

    def test_direct_memory_query_remains_separate_and_byte_preserving(self):
        command=('shell','dumpsys','meminfo','--local','-s','123')
        result=self.execute(command);self.assertEqual(result.stdout,b'raw\r\n\0')
        self.assertEqual(tuple(self.sdk.call_args.args),command)
        self.assertEqual(result.nativeEvidence['kind'],'direct')

    def test_writes_wrong_uid_and_invalid_budgets_send_no_sdk_commands(self):
        for command in [('shell','am','force-stop','com.solosoul.app'),('shell','pm','list','packages','--uid','10106'),
                ('shell','cat','/proc/123/status'),('shell','cat','/proc/123/mem')]:
            with self.subTest(command=command),self.assertRaises(ValueError):self.execute(command)
        for budget in [True,0,11,float('nan')]:
            with self.subTest(budget=budget),self.assertRaises(ValueError):
                self.adapter.run(*batches.PROCESSES,timeout=budget,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        self.sdk.assert_not_called()


if __name__=='__main__':unittest.main()
