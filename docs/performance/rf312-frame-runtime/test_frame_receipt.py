import copy
import json
import unittest
import frame_receipt as reader

def fixture():
    run='a'*32;target='b'*64;collector='c'*64
    process=dict(processId=12345,uid=10032,startTicks=999)
    start=10_000_000_000
    document=dict(schemaVersion=2,mainFrame=True,childFrames=0,localOrigin=True,loginRoute=True,
        passwordInputs=1,submitButtons=1,enabled=True,visible=True,hitPassword=True,hitSubmit=True,documentFocused=False,documentVisible=True,
        timeOriginMs=1000000000.5,atMs=140.25)
    ready=dict(schemaVersion=1,scope='native-login-frame-commit-probe',runId=run,process=copy.deepcopy(process),
        targetApkSha256=target,collectorApkSha256=collector,startedNs=start,deadlineNs=start+reader.STAGE_NS)
    record=dict(schemaVersion=2,scope='native-login-frame-commits-after-visual-fence',runId=run,process=copy.deepcopy(process),
        targetApkSha256=target,api=31,startedNs=start,deadlineNs=start+reader.STAGE_NS,completedNs=start+7_000_000,
        fenceRequest=start+1_000_000,fencePostedNs=start+1_000_000,fenceCompleteNs=start+2_000_000,
        hardwareAccelerated=True,nativeWindowFocused=True,width=1080,height=1731,explicitDrawRequestsDispatched=False,nativeClock=reader.CLOCK,
        forcedInvalidation=False,inputDispatched=False,businessIpcDispatched=False,
        documentBefore=copy.deepcopy(document),documentAfter=copy.deepcopy(document),
        frames=[dict(drawOrdinal=1,drawStartedNs=start+3_000_000,committedNs=start+4_000_000,callbackSource=reader.CALLBACK),
                dict(drawOrdinal=2,drawStartedNs=start+5_000_000,committedNs=start+6_000_000,callbackSource=reader.CALLBACK)],
        callbacksDetached=True,physicalDisplayPresentationProven=False,applicationOnlyLatencyAccepted=False)
    arguments=dict(run_id=run,expected_process=process,target_apk_sha256=target,collector_apk_sha256=collector,expected_api=31)
    return ready,record,arguments

class FrameContractTests(unittest.TestCase):
    def setUp(self):self.ready,self.record,self.arguments=fixture()
    def check(self):return reader.validate(self.ready,self.record,**self.arguments)
    def test_native_window_focus_is_required_without_fabricating_document_input_focus(self):
        self.assertIs(self.record['documentBefore']['documentFocused'],False)
        self.assertTrue(self.check()['frameCommitContractAccepted'])
        self.record['nativeWindowFocused']=False
        with self.assertRaises(ValueError):self.check()
    def test_document_input_focus_remains_explicit_boolean_diagnostic_metadata(self):
        self.record['documentAfter']['documentFocused']='foreign'
        with self.assertRaises(ValueError):self.check()
    def test_old_focus_schema_is_not_reinterpreted_as_native_window_evidence(self):
        self.record['schemaVersion']=1
        with self.assertRaises(ValueError):self.check()
    def test_valid_metadata_does_not_promote_itself_to_a_native_or_latency_result(self):
        value=self.check();self.assertTrue(value['frameCommitContractAccepted']);self.assertEqual(value['committedFrames'],2)
        for key in ['nativeRuntimeAccepted','physicalDisplayPresentationProven','applicationOnlyLatencyAccepted','defaultPerformanceSampleAccepted','fullRF312Accepted']:
            self.assertIs(value[key],False)
    def test_missing_or_extra_fields_rejected(self):
        for key in ['width','extra']:
            self.ready,self.record,self.arguments=fixture()
            if key=='extra':self.record[key]=1
            else:del self.record[key]
            with self.assertRaises(ValueError):self.check()
    def test_animation_callbacks_cannot_stand_in_for_commits(self):
        self.record['frames'][0]['callbackSource']='View.postOnAnimation'
        with self.assertRaises(ValueError):self.check()
    def test_duplicate_draw_ordinal_rejected(self):
        self.record['frames'][1]['drawOrdinal']=1
        with self.assertRaises(ValueError):self.check()
    def test_second_registration_before_first_commit_rejected(self):
        self.record['frames'][1]['drawStartedNs']=self.record['frames'][0]['committedNs']-1
        with self.assertRaises(ValueError):self.check()
    def test_one_commit_cannot_complete_the_contract(self):
        self.record['frames'].pop()
        with self.assertRaises(ValueError):self.check()
    def test_deadline_overrun_or_reset_rejected(self):
        for key in ['completedNs','deadlineNs']:
            self.ready,self.record,self.arguments=fixture()
            self.record[key]=self.record['deadlineNs'] if key=='completedNs' else self.record['deadlineNs']+1
            with self.assertRaises(ValueError):self.check()
    def test_changed_uid_pid_or_birth_rejected(self):
        for key in ['processId','uid','startTicks']:
            self.ready,self.record,self.arguments=fixture();self.record['process'][key]+=1
            with self.assertRaises(ValueError):self.check()
    def test_boolean_is_not_a_native_clock(self):
        self.record['completedNs']=True
        with self.assertRaises(ValueError):self.check()
    def test_software_renderer_or_missing_cleanup_rejected(self):
        for key in ['hardwareAccelerated','nativeWindowFocused','callbacksDetached']:
            self.ready,self.record,self.arguments=fixture();self.record[key]=False
            with self.assertRaises(ValueError):self.check()
    def test_forced_draw_or_input_or_business_ipc_rejected(self):
        for key in ['explicitDrawRequestsDispatched','forcedInvalidation','inputDispatched','businessIpcDispatched']:
            self.ready,self.record,self.arguments=fixture();self.record[key]=True
            with self.assertRaises(ValueError):self.check()
    def test_document_replacement_rejected(self):
        self.record['documentAfter']['timeOriginMs']+=1
        with self.assertRaises(ValueError):self.check()
    def test_covered_or_unfocused_or_nonexclusive_document_rejected(self):
        for key in ['hitPassword','hitSubmit','documentVisible','mainFrame','childFrames']:
            self.ready,self.record,self.arguments=fixture();self.record['documentAfter'][key]=1 if key=='childFrames' else False
            with self.assertRaises(ValueError):self.check()
    def test_unsupported_presentation_or_latency_claim_rejected(self):
        for key in ['physicalDisplayPresentationProven','applicationOnlyLatencyAccepted']:
            self.ready,self.record,self.arguments=fixture();self.record[key]=True
            with self.assertRaises(ValueError):self.check()
    def test_original_utf8_decoder_rejects_duplicate_nonfinite_or_oversized_payload(self):
        for raw in [b'{"x":1,"x":2}',b'{"x":NaN}',b'{"x":"'+b'a'*32768+b'"}']:
            with self.assertRaises((ValueError,json.JSONDecodeError)):reader.decode(raw)
    def test_wrong_run_apk_or_unsupported_api_rejected(self):
        for key in ['runId','targetApkSha256','api']:
            self.ready,self.record,self.arguments=fixture();self.record[key]=28 if key=='api' else 'd'*(32 if key=='runId' else 64)
            with self.assertRaises(ValueError):self.check()

if __name__=='__main__':unittest.main()
