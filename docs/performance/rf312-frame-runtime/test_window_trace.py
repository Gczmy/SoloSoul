import copy,hashlib,json,unittest
import window_trace as trace

def fixture():
    ready=dict(schemaVersion=1,scope='native-login-frame-commit-probe',runId='a'*32,
        process=dict(processId=123,uid=10001,startTicks=456),targetApkSha256='b'*64,collectorApkSha256='c'*64,
        startedNs=100,deadlineNs=45_000_000_100)
    v=dict(schemaVersion=1,scope='passive-window-frame-diagnostics',runId=ready['runId'],process=copy.deepcopy(ready['process']),
        targetApkSha256=ready['targetApkSha256'],collectorApkSha256=ready['collectorApkSha256'],api=31,startedNs=100,
        deadlineNs=ready['deadlineNs'],detachStartedNs=ready['deadlineNs']+9,captureStoppedNs=ready['deadlineNs']+10,cleanupFinishedNs=ready['deadlineNs']+20,
        lifecycleHookNs=110,windowHookNs=120,fenceCompleteNs=150,targetActivityObserved=True,lifecycleHookInstalled=True,
        windowHookInstalled=True,hookFrom='post-created',fault='none',callbacksDetached=True,metricsThreadTerminated=True,
        cleanupBudgetMet=True,webViewTreeCompared=True,webViewTreeMatchesDecorTree=True,preDrawCount=0,drawCount=0,
        preDrawAfterFence=0,drawAfterFence=0,metricsCallbackCount=0,metricsCallbacksAfterFence=0,reportedMetricDrops=0,
        lateMetricsCallbacks=0,lateUiCallbacks=0,firstPreDrawNs=0,lastPreDrawNs=0,firstDrawNs=0,lastDrawNs=0,
        storedMetricLimit=64,metricPrefixTruncated=False,metrics=[],nativeClock=trace.CLOCK)
    v.update({k:False for k in trace.FALSE_FLAGS})
    return ready,v

def metric(ordinal,at):
    return dict(ordinal=ordinal,deliveredNs=at,intendedVsyncNs=1,vsyncNs=2,totalDurationNs=3,
                drawDurationNs=4,swapDurationNs=5,firstDrawFlag=0,droppedSincePrevious=0)

class WindowTraceTests(unittest.TestCase):
    def setUp(self):self.ready,self.value=fixture()
    def check(self):
        body=json.dumps(self.value).encode()
        return trace.validate(body,(hashlib.sha256(body).hexdigest()+'\n').encode(),self.ready)
    def two(self):
        self.value.update(metricsCallbackCount=2,metrics=[metric(1,140),metric(2,160)],metricsCallbacksAfterFence=1)
    def test_zero_observations_remain_only_passive_metadata(self):
        result=self.check();self.assertTrue(result['observationHooksAndCleanupComplete'])
        for key in ['nativeFramesAccepted','physicalDisplayPresentationProven','applicationOnlyLatencyAccepted',
                    'defaultPerformanceSampleAccepted','fullRF312Accepted']:self.assertIs(result[key],False)
    def test_window_callbacks_are_not_two_native_commits(self):
        self.two();result=self.check();self.assertEqual(result['metricsCallbacks'],2)
        self.assertEqual(result['metricsCallbacksAfterFence'],1);self.assertIs(result['nativeFramesAccepted'],False)
    def test_unavailable_metrics_are_retained_without_clock_conversion(self):
        self.two()
        for name in ['intendedVsyncNs','vsyncNs','totalDurationNs','drawDurationNs','swapDurationNs','firstDrawFlag']:
            self.value['metrics'][0][name]=-1
        before=copy.deepcopy(self.value);self.check();self.assertEqual(self.value,before)
    def test_native_drop_counts_must_match_complete_prefix(self):
        self.two();self.value['metrics'][0]['droppedSincePrevious']=3;self.value['reportedMetricDrops']=3;self.check()
        self.value['reportedMetricDrops']=2
        with self.assertRaises(ValueError):self.check()
    def test_incomplete_hash_and_wrong_marker_rejected(self):
        body=json.dumps(self.value).encode()
        for marker in [b'0'*64+b'\n',hashlib.sha256(body).hexdigest().encode()]:
            with self.assertRaises(ValueError):trace.validate(body,marker,self.ready)
    def test_run_apk_process_and_original_deadline_cannot_be_replayed(self):
        for key,new in [('runId','d'*32),('targetApkSha256','d'*64),('collectorApkSha256','d'*64),('deadlineNs',1234),('api',35)]:
            self.setUp();self.value[key]=new
            with self.assertRaises(ValueError):self.check()
        for key in ['processId','uid','startTicks']:
            self.setUp();self.value['process'][key]+=1
            with self.assertRaises(ValueError):self.check()
    def test_arbitrary_content_and_unknown_fields_rejected(self):
        self.value['content']='foreign'
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.two();self.value['metrics'][0]['text']='foreign'
        with self.assertRaises(ValueError):self.check()
    def test_boolean_negative_and_overflow_counters_rejected(self):
        for part in [True,-1,1_000_001]:
            self.setUp();self.value['preDrawCount']=part
            with self.assertRaises(ValueError):self.check()
    def test_delivery_cannot_escape_native_stage_or_stop_boundary(self):
        self.two();self.value['metrics'][1]['deliveredNs']=self.value['deadlineNs']
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.two();self.value['captureStoppedNs']=155;self.value['cleanupFinishedNs']=156
        with self.assertRaises(ValueError):self.check()
    def test_no_forced_actions_or_presentation_claims_can_be_accepted(self):
        for key in trace.FALSE_FLAGS:
            self.setUp();self.value[key]=True
            with self.assertRaises(ValueError):self.check()
    def test_truncated_callback_prefix_is_explicit_and_bounded(self):
        self.value.update(metricsCallbackCount=65,metrics=[metric(n,140+n) for n in range(1,65)],
                          metricsCallbacksAfterFence=56,metricPrefixTruncated=True)
        result=self.check();self.assertIs(result['prefixTruncated'],True);self.assertFalse(result['nativeFramesAccepted'])
        self.value['metrics'].append(metric(65,205))
        with self.assertRaises(ValueError):self.check()
    def test_duplicate_ordinals_and_false_truncation_rejected(self):
        self.two();self.value['metrics'][1]['ordinal']=1
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.value['metricPrefixTruncated']=True
        with self.assertRaises(ValueError):self.check()
    def test_incomplete_cleanup_never_claims_complete_observation(self):
        self.value['metricsThreadTerminated']=False
        self.assertFalse(self.check()['observationHooksAndCleanupComplete'])
        self.value['cleanupFinishedNs']=self.value['captureStoppedNs']+3_000_000_001
        with self.assertRaises(ValueError):self.check()
    def test_missing_window_hook_and_unknown_tree_are_explicit(self):
        self.value.update(windowHookInstalled=False,windowHookNs=0,hookFrom='none',webViewTreeCompared=False,
                          webViewTreeMatchesDecorTree=False)
        self.assertFalse(self.check()['observationHooksAndCleanupComplete'])
        self.value['webViewTreeMatchesDecorTree']=True
        with self.assertRaises(ValueError):self.check()
    def test_callback_during_close_lock_wait_uses_actual_capture_stop(self):
        self.value.update(detachStartedNs=145,captureStoppedNs=170,cleanupFinishedNs=180,
                          metricsCallbackCount=1,metrics=[metric(1,160)],metricsCallbacksAfterFence=1)
        result=self.check();self.assertTrue(result['observationHooksAndCleanupComplete'])
        self.assertIs(result['nativeFramesAccepted'],False)
    def test_duplicate_json_and_nonfinite_metadata_rejected(self):
        body=json.dumps(self.value).encode().replace(b'"api": 31',b'"api": 31, "api": 31')
        with self.assertRaises(ValueError):trace.validate(body,(hashlib.sha256(body).hexdigest()+'\n').encode(),self.ready)
        self.value['firstDrawNs']=float('nan')
        with self.assertRaises(ValueError):self.check()

if __name__=='__main__':unittest.main()
