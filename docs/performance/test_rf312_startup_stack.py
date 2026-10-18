import copy
import unittest
import rf312_startup_stack as trace

RUN='a'*32
PROCESS=dict(processId=123,uid=10001,startTicks=111)


def rows():
    return [dict(schemaVersion=1,scope='forensic-startup-main-thread-stack',runId=RUN,sequence=i,
        process=dict(PROCESS),traceStartedNs=100,nativeReadStartedNs=100+i*100000000,
        nativeReadFinishedNs=101+i*100000000,mainThreadId=1,threadState='RUNNABLE',truncated=False,
        frames=[dict(className='android.webkit.WebViewFactory',methodName='getProvider',fileName='WebViewFactory.java',lineNumber=100,nativeMethod=False)],
        performanceSampleAccepted=False) for i in range(1,4)]


class StartupTraceTests(unittest.TestCase):
    def validate(self,value):return trace.validate(value,run_id=RUN,expected_process=PROCESS)

    def test_partial_forensic_prefix_does_not_accept_performance_or_ipc(self):
        result=self.validate(rows());self.assertEqual(result['snapshots'],3)
        self.assertTrue(result['forensicStackPrefixAccepted']);self.assertFalse(result['performanceSampleAccepted'])
        self.assertFalse(result['actualIpcAccepted']);self.assertFalse(result['fullRF312Accepted'])

    def test_gap_process_replacement_and_performance_promotion_rejected(self):
        for mutate in [lambda v:v[1].update(sequence=3),lambda v:v[1]['process'].update(processId=999),
                       lambda v:v[0].update(performanceSampleAccepted=True),lambda v:v[0].update(runId='b'*32)]:
            value=rows();mutate(value)
            with self.subTest(value=value),self.assertRaises(ValueError):self.validate(value)

    def test_clock_overrun_regression_and_thread_replacement_rejected(self):
        for mutate in [lambda v:v[2].update(nativeReadFinishedNs=8000000101),
                       lambda v:v[1].update(nativeReadStartedNs=1),lambda v:v[1].update(mainThreadId=2),
                       lambda v:v[1].update(traceStartedNs=101),lambda v:v[0].update(mainThreadId=True)]:
            value=rows();mutate(value)
            with self.subTest(value=value),self.assertRaises(ValueError):self.validate(value)

    def test_payloads_unbounded_symbols_and_wrong_native_frames_rejected(self):
        for mutate in [lambda v:v[0]['frames'][0].update(arguments='forbidden'),
                       lambda v:v[0]['frames'][0].update(className='x'*513),
                       lambda v:v[0]['frames'][0].update(fileName='bad\nname'),
                       lambda v:v[0]['frames'][0].update(nativeMethod=True)]:
            value=rows();mutate(value)
            with self.subTest(value=value),self.assertRaises(ValueError):self.validate(value)

    def test_frame_and_snapshot_caps_and_explicit_truncation(self):
        value=rows();frame=value[0]['frames'][0];value[0].update(frames=[copy.deepcopy(frame) for _ in range(128)],truncated=True)
        self.validate(value)
        value[0]['frames'].append(copy.deepcopy(frame))
        with self.assertRaises(ValueError):self.validate(value)
        for value in [[],rows()*27]:
            with self.assertRaises(ValueError):self.validate(value)

    def test_native_frame_and_null_filename_supported(self):
        value=rows();value[0]['frames'][0].update(fileName=None,lineNumber=-2,nativeMethod=True)
        self.validate(value)

    def test_wrong_json_types_are_rejected_without_coercion(self):
        for update in [dict(threadState=[]),dict(nativeReadStartedNs=True),dict(frames='not frames'),
                       dict(performanceSampleAccepted=0),dict(truncated=1)]:
            value=rows();value[0].update(update)
            with self.subTest(update=update),self.assertRaises(ValueError):self.validate(value)


if __name__=='__main__':unittest.main()
