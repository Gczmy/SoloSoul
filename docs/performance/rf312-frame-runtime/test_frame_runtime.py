"""Native frame handshake must reject aliases, replays and terminal false positives."""
import copy,json,unittest
from frame_session import ready_contract,terminal,refusal_contract,sha
from test_frame_receipt import fixture

class FrameRuntimeTests(unittest.TestCase):
    def setUp(self):self.ready,self.record,self.arguments=fixture()
    def check(self):
        return ready_contract(json.dumps(self.ready).encode(),self.arguments['run_id'],
            self.arguments['target_apk_sha256'],self.arguments['collector_apk_sha256'],self.arguments['expected_process']['uid'])
    def test_accepts_only_the_separate_native_frame_handshake(self):self.assertEqual(self.check(),self.ready)
    def test_old_phase_handshake_not_promoted(self):
        self.ready['phaseCount']=12
        with self.assertRaises(ValueError):self.check()
    def test_native_clock_boolean_is_rejected(self):
        self.ready['startedNs']=True
        with self.assertRaises(ValueError):self.check()
    def test_exact_original_deadline_cannot_be_reset(self):
        self.ready['deadlineNs']+=1
        with self.assertRaises(ValueError):self.check()
    def test_process_identity_and_uid_are_required(self):
        self.ready['process']['uid']+=1
        with self.assertRaises(ValueError):self.check()
    def test_run_apk_and_scope_cannot_be_replayed(self):
        for key in ['runId','targetApkSha256','collectorApkSha256','scope']:
            self.setUp();self.ready[key]='foreign'
            with self.assertRaises(ValueError):self.check()
    def test_success_requires_the_native_terminal_bundle_and_code(self):
        terminal('INSTRUMENTATION_RESULT: frameProbePassed=true\nINSTRUMENTATION_RESULT: committedFrames=2\nINSTRUMENTATION_CODE: -1\n')
    def test_junit_or_ipc_outcome_is_not_native_frame_success(self):
        for text in ['OK (1 test)','INSTRUMENTATION_RESULT: collectorPassed=true\nINSTRUMENTATION_CODE: -1',
                     'INSTRUMENTATION_RESULT: frameProbePassed=true\nINSTRUMENTATION_CODE: -1']:
            with self.assertRaises(ValueError):terminal(text)
    def test_duplicate_or_aborted_or_refused_terminal_rejected(self):
        base='INSTRUMENTATION_RESULT: frameProbePassed=true\nINSTRUMENTATION_RESULT: committedFrames=2\nINSTRUMENTATION_CODE: -1\n'
        for extra in ['INSTRUMENTATION_CODE: -1\n','INSTRUMENTATION_FAILED: failure','INSTRUMENTATION_ABORTED: failure',
                      'INSTRUMENTATION_RESULT: frameProbePassed=false','INSTRUMENTATION_RESULT: failedStep=login']:
            with self.assertRaises(ValueError):terminal(base+extra)

class RefusalEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.run='a'*32
        self.value=dict(schemaVersion=2,runId=self.run,scope='native-login-frame-commit-refusal',
            nativeStatus='document-not-ready',step='usable-login-document',errorClass='java.lang.IllegalStateException',reason='frame-contract-rejected',
            lastDocument=fixture()[1]['documentBefore'],frames=[],callbacksDetached=True,
            nativeFramesAccepted=False,performanceSampleAccepted=False)
    def check(self):
        raw=json.dumps(self.value).encode();return refusal_contract(raw,(sha(raw)+'\n').encode(),self.run)
    def test_false_eligibility_is_diagnostic_and_never_success(self):
        self.value['lastDocument']['localOrigin']=False
        result=self.check();self.assertIs(result['nativeFramesAccepted'],False)
    def test_account_values_or_extra_document_fields_rejected(self):
        self.value['lastDocument']['password']='foreign'
        with self.assertRaises(ValueError):self.check()
    def test_incomplete_hash_or_wrong_run_rejected(self):
        raw=json.dumps(self.value).encode()
        with self.assertRaises(ValueError):refusal_contract(raw,b'0'*64+b'\n',self.run)
        self.value['runId']='b'*32
        with self.assertRaises(ValueError):self.check()
    def test_refusal_cannot_claim_native_success_or_contain_unbounded_error_text(self):
        self.value['nativeFramesAccepted']=True
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.value['reason']='arbitrary account text'
        with self.assertRaises(ValueError):self.check()
    def test_passive_progress_codes_preserve_refusal_and_do_not_claim_a_frame(self):
        for status in ['visual-fence-pending','visual-fence-complete','native-draw-listener-armed',
                       'native-draw-observed','native-commit-observed']:
            self.setUp();self.value['nativeStatus']=status
            result=self.check()
            self.assertIs(result['nativeFramesAccepted'],False)
            self.assertIs(result['performanceSampleAccepted'],False)
            self.assertEqual(result['frames'],[])
    def test_pre_draw_registration_progress_is_never_a_committed_frame(self):
        for status in ['native-listeners-armed','native-pre-draw-commit-armed']:
            self.setUp();self.value['nativeStatus']=status
            result=self.check()
            self.assertIs(result['nativeFramesAccepted'],False)
            self.assertIs(result['performanceSampleAccepted'],False)
            self.assertEqual(result['frames'],[])
    def test_old_refusal_schema_cannot_claim_the_new_native_diagnostic_fields(self):
        self.value['schemaVersion']=1
        with self.assertRaises(ValueError):self.check()
    def test_native_status_is_an_exact_code_and_cannot_contain_account_data(self):
        self.value['nativeStatus']='arbitrary account text'
        with self.assertRaises(ValueError):self.check()

if __name__=='__main__':unittest.main()
