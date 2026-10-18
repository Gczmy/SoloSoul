import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import ipc_receipt
import native_collector as native

RUN='a'*32;APK='b'*64;COLLECTOR='c'*64
PROCESS=dict(processId=101,uid=10001,startTicks=123)


def ready():
    return json.dumps(dict(schemaVersion=1,runId=RUN,process=PROCESS,targetApkSha256=APK,
                           collectorApkSha256=COLLECTOR,phaseCount=12)).encode()


def record(index):
    return dict(stage=ipc_receipt.PHASES[index-1],process=dict(PROCESS),
        nativeReadStartedNs=index*1000000000,nativeReadFinishedNs=index*1000000000+1000000,
        document=dict(present=True,platform='android',mainFrame=True,frames=0,invokeReadonly=True,
            observer=dict(schemaVersion=1,scope='android-native-ipc-transport-observer',runId=RUN,valid=True,
                invalidReasons=[],installedAtMs=0,timeOriginMs=1000,atMs=index*10,maxEvents=16384,observedCount=index,
                commands=[dict(command='get_objects',transport='android-java-bridge' if n%2 else 'android-channel-fetch',atMs=n*10-5)
                          for n in range(1,index+1)])))


class Clock:
    def __init__(self):self.value=0
    def __call__(self):return self.value
    def sleep(self,seconds):self.value+=seconds


class SDK:
    def __init__(self):
        self.calls=[];self.index=0;self.records={i:record(i) for i in range(1,13)}
        self.first_missing=True;self.corrupt_digest=False;self.error=False
    def __call__(self,argv,timeout):
        self.calls.append(list(argv))
        if self.error:return (1,b'',b'transport failure')
        if argv[0]=='push' and argv[-1].endswith('.json'):
            value=json.loads(Path(argv[1]).read_bytes());self.index=value['sequence']
        if argv[:3]==['shell','test','-f']:
            if self.first_missing:self.first_missing=False;return (1,b'',b'')
            return (0,b'',b'')
        if argv[:2]==['exec-out','cat']:
            raw=json.dumps(self.records[self.index]).encode()
            if argv[-1].endswith('.sha256'):
                return (0,('0'*64 if self.corrupt_digest else hashlib.sha256(raw).hexdigest()).encode()+b'\n',b'')
            return (0,raw,b'')
        return (0,b'',b'')


class NativeCollectorTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.sdk=SDK();self.clock=Clock()
        self.client=native.Collector(self.sdk,lambda:True,Path(self.temp.name)/'metadata',ready(),
            run_id=RUN,target_apk_sha256=APK,collector_apk_sha256=COLLECTOR,clock=self.clock,sleep=self.clock.sleep)

    def capture(self,index):return self.client.capture(ipc_receipt.PHASES[index-1],self.clock()+45)

    def test_all_twelve_bind_order_prefix_hashes_and_count_attempts_without_accepting_native_ui(self):
        for index in range(1,13):self.capture(index)
        result=self.client.finish()
        self.assertEqual(result['sendAttempts'],12);self.assertEqual(result['phaseCount'],12)
        self.assertTrue(result['orderedHostRequestsBound']);self.assertFalse(result['actualIpcAccepted'])
        self.assertFalse(result['fullRF312Accepted']);self.assertFalse(result['logicalInvokeOrCompletionCount'])
        self.assertEqual(len(result['requestResponseDigests']),12)
        self.assertFalse(any(argv[0] in ['input','am','monkey'] for argv in self.sdk.calls))

    def test_request_marker_is_committed_after_data_and_partial_marker_push(self):
        self.capture(1)
        self.assertEqual([row[0] for row in self.sdk.calls[:3]],['push','push','shell'])
        self.assertTrue(self.sdk.calls[1][-1].endswith('.sha256.part'))
        self.assertEqual(self.sdk.calls[2][:2],['shell','mv'])
        data=(self.client.output/'request-001.json').read_bytes()
        self.assertEqual((self.client.output/'request-001.json.sha256').read_text().strip(),hashlib.sha256(data).hexdigest())

    def test_skipped_phase_or_replayed_completed_phase_is_rejected_before_sdk_input(self):
        with self.assertRaises(ValueError):self.capture(2)
        self.assertEqual(self.sdk.calls,[])
        self.capture(1);count=len(self.sdk.calls)
        with self.assertRaises(ValueError):self.capture(1)
        self.assertEqual(len(self.sdk.calls),count)

    def test_incomplete_capture_cannot_be_retried_or_reported_as_complete(self):
        self.sdk.error=True
        with self.assertRaises(ValueError):self.capture(1)
        count=len(self.sdk.calls);self.sdk.error=False
        with self.assertRaises(ValueError):self.capture(1)
        self.assertEqual(len(self.sdk.calls),count)
        with self.assertRaises(ValueError):self.client.finish()

    def test_wrong_digest_cannot_become_a_record(self):
        self.sdk.corrupt_digest=True
        with self.assertRaises(ValueError):self.capture(1)
        self.assertEqual(self.client.records,[])

    def test_rejects_removed_or_changed_prior_event(self):
        self.capture(1)
        self.sdk.records[2]['document']['observer']['commands'][0]['command']='different_command'
        with self.assertRaises(ValueError):self.capture(2)
        self.assertEqual(len(self.client.records),1)

    def test_rejects_process_swap_payload_and_original_native_read_budget_violation(self):
        for mutate in [lambda value:value['process'].update(uid=10002),
                       lambda value:value['document']['observer']['commands'][0].update(payload='forbidden'),
                       lambda value:value.update(nativeReadFinishedNs=value['nativeReadStartedNs']+3000000001)]:
            value=record(1);mutate(value)
            with self.subTest(value=value),self.assertRaises(ValueError):self.client.validate_record(value,'startup')

    def test_expired_phase_has_no_sdk_side_effect_and_stderr_is_not_file_absence(self):
        with self.assertRaises(ValueError):self.client.capture('startup',0)
        self.assertEqual(self.sdk.calls,[])
        self.sdk.error=True
        with self.assertRaises(ValueError):self.client.command(['shell','test','-f','file'],45,missing=True)

    def test_zero_delta_phases_are_valid_without_inventing_sends(self):
        first=record(1)['document']['observer']['commands']
        for value in self.sdk.records.values():
            value['document']['observer'].update(commands=copy.deepcopy(first),observedCount=1)
        for index in range(1,13):self.capture(index)
        result=self.client.finish()
        self.assertEqual(result['sendAttempts'],1)
        self.assertEqual([row['sendAttempts'] for row in result['phases']],[1]+[0]*11)

    def test_sdk_success_progress_on_stderr_does_not_reject_a_bound_metadata_push(self):
        def reporting(argv,timeout):
            result=self.sdk(argv,timeout)
            if argv[0]=='push':
                path=Path(argv[1]);message=(str(path)+': 1 file pushed, 0 skipped. 0.3 MB/s ('+str(path.stat().st_size)+' bytes in 0.000s)\r\n').encode()
                return (0,b'',message)
            return result
        self.client.run_sdk=reporting
        for index in range(1,13):self.capture(index)
        self.assertEqual(self.client.finish()['phaseCount'],12)

    def test_progress_must_match_one_exact_source_size_and_zero_skips(self):
        path=Path(self.temp.name)/'request.json';path.write_bytes(b'public synthetic request')
        good=(str(path)+': 1 file pushed, 0 skipped. 1.0 MB/s ('+str(path.stat().st_size)+' bytes in 0.001s)\r\n').encode()
        argv=['push',str(path),'/owned/request.json']
        native.push_progress(argv,b'',good);native.push_progress(argv,good,b'')
        for bad in [good.replace(b'1 file pushed',b'0 files pushed'),good.replace(b'0 skipped',b'1 skipped'),
                    good.replace(str(path.stat().st_size).encode()+b' bytes',b'999 bytes'),good+b'error: transport failed\n',
                    good.replace(path.name.encode(),b'wrong.json')]:
            with self.subTest(bad=bad),self.assertRaises(ValueError):native.push_progress(argv,b'',bad)

    def test_failed_push_and_non_push_stderr_are_never_success_progress(self):
        self.client.run_sdk=lambda argv,timeout:(1,b'',b'1 file pushed, 0 skipped.\n')
        with self.assertRaises(ValueError):self.capture(1)
        self.client.run_sdk=lambda argv,timeout:(0,b'',b'successful progress\n')
        with self.assertRaises(ValueError):self.client.command(['exec-out','cat','/owned/result'],45)
        self.assertEqual(self.client.records,[])

    def test_progress_on_both_streams_is_ambiguous_and_rejected(self):
        path=Path(self.temp.name)/'file';path.write_bytes(b'fixture')
        good=(str(path)+': 1 file pushed, 0 skipped. 1.0 MB/s (7 bytes in 0.001s)\n').encode()
        with self.assertRaises(ValueError):native.push_progress(['push',str(path),'/owned/file'],good,good)


if __name__=='__main__':unittest.main()
