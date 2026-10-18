import copy,hashlib,json,unittest
import startup_milestones as reader
import frame_receipt as frames
from frame_session import ready_contract as old_ready,terminal as old_terminal,refusal_contract as old_refusal
from test_frame_receipt import fixture as frame_fixture

def fixture():
    ready,old,args=frame_fixture();ready['scope']=reader.READY_SCOPE
    before=copy.deepcopy(old['documentBefore']);before.update(timeOriginMs=10000,atMs=500)
    after=copy.deepcopy(before);after['atMs']=800
    group=lambda *values:dict(count=len(values),entries=list(values))
    doc=dict(schemaVersion=1,scope='document-startup-milestones',mainFrame=True,childFrames=0,localOrigin=True,
        loginRoute=True,documentVisible=True,timeOriginMs=10000,observedAtMs=600,entryQueryAvailable=True,
        supportedEntryTypesAvailable=True,markEntrySupported=True,paintEntrySupported=True,navigationEntrySupported=True,
        reactMount=group(200),startupDismissed=group(400),firstPaint=group(30),firstContentfulPaint=group(40),
        navigation=group(dict(startTimeMs=0,durationMs=150,domInteractiveMs=80,domContentLoadedEndMs=100,loadEventEndMs=150)))
    record={name:copy.deepcopy(old[name]) for name in ['runId','process','targetApkSha256','api','startedNs','deadlineNs',
        'completedNs','hardwareAccelerated','nativeWindowFocused','width','height','callbacksDetached']}
    record.update(schemaVersion=1,scope=reader.SCOPE,collectorApkSha256=ready['collectorApkSha256'],nativeClock=reader.NATIVE_CLOCK,
        documentClock=reader.DOCUMENT_CLOCK,documentBefore=before,documentAfter=after,milestones=doc)
    record.update({name:False for name in reader.FALSE})
    return ready,record,args

class StartupMilestoneTests(unittest.TestCase):
    def setUp(self):self.ready,self.record,self.args=fixture()
    def check(self):return reader.validate(self.ready,self.record,**self.args)
    def test_unique_existing_marks_use_document_clock_and_keep_all_claim_limits(self):
        result=self.check();self.assertEqual(result['metrics']['mountInvocationToHandoffMs'],200)
        self.assertTrue(result['requiredMilestonesAvailable'])
        for name in ['firstInteractiveTimeProven','nativeFramesAccepted','physicalDisplayPresentationProven',
                     'applicationOnlyLatencyAccepted','defaultPerformanceSampleAccepted','fullRF312Accepted','observerOverheadRemoved']:
            self.assertIs(result[name],False)
    def test_missing_paint_and_navigation_remain_null(self):
        for name in ['firstPaint','firstContentfulPaint','navigation']:self.record['milestones'][name]=dict(count=0,entries=[])
        result=self.check();self.assertIsNone(result['metrics']['firstPaintMs']);self.assertIsNone(result['metrics']['navigation'])
        self.assertTrue(result['requiredMilestonesAvailable'])
    def test_missing_mark_is_unavailable_not_zero_or_synthetic_timing(self):
        self.record['milestones']['startupDismissed']=dict(count=0,entries=[])
        result=self.check();self.assertFalse(result['requiredMilestonesAvailable'])
        self.assertIsNone(result['metrics']['startupDismissedMs']);self.assertIsNone(result['metrics']['mountInvocationToHandoffMs'])
    def test_duplicate_or_truncated_marks_are_ambiguous(self):
        for count in [2,5,1000001,True]:
            self.setUp();self.record['milestones']['reactMount']=dict(count=count,entries=[100,200] if count==2 else [1,2,3,4])
            with self.assertRaises(ValueError):self.check()
    def test_unknown_fields_and_account_payload_cannot_enter_protocol(self):
        for target in [self.record,self.record['milestones'],self.record['milestones']['reactMount'],self.record['milestones']['navigation']['entries'][0]]:
            target['account']='foreign'
            with self.assertRaises(ValueError):self.check()
            del target['account']
    def test_wrong_document_or_native_identity_rejected(self):
        for name in ['runId','targetApkSha256','collectorApkSha256','scope']:
            self.setUp();self.record[name]='foreign'
            with self.assertRaises(ValueError):self.check()
        for name in ['processId','uid','startTicks']:
            self.setUp();self.record['process'][name]+=1
            with self.assertRaises(ValueError):self.check()
    def test_document_origin_order_and_future_timestamps_rejected(self):
        for name,value in [('timeOriginMs',11000),('observedAtMs',499),('observedAtMs',801)]:
            self.setUp();self.record['milestones'][name]=value
            with self.assertRaises(ValueError):self.check()
        self.setUp();self.record['milestones']['startupDismissed']['entries']=[601]
        with self.assertRaises(ValueError):self.check()
    def test_marks_cannot_be_reordered(self):
        self.record['milestones']['startupDismissed']['entries']=[199]
        with self.assertRaises(ValueError):self.check()
    def test_nonfinite_boolean_and_negative_numbers_rejected(self):
        for value in [float('nan'),float('inf'),-1,True,None]:
            self.setUp();self.record['milestones']['reactMount']['entries']=[value]
            with self.assertRaises(ValueError):self.check()
    def test_native_budget_focus_geometry_and_all_claim_flags_remain_required(self):
        for name,value in [('completedNs',self.record['deadlineNs']),('nativeWindowFocused',False),('hardwareAccelerated',False),
                           ('callbacksDetached',False),('width',0),('documentClock','host clock'),*[(name,True) for name in reader.FALSE]]:
            self.setUp();self.record[name]=value
            with self.assertRaises(ValueError):self.check()
    def test_unavailable_api_cannot_invent_support_or_entries(self):
        self.record['milestones']['supportedEntryTypesAvailable']=False
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.record['milestones']['entryQueryAvailable']=False
        with self.assertRaises(ValueError):self.check()
    def test_new_ready_and_terminal_are_rejected_by_old_frame_consumers(self):
        raw=json.dumps(self.ready).encode()
        reader.ready_contract(raw,self.args['run_id'],self.args['target_apk_sha256'],self.args['collector_apk_sha256'],self.ready['process']['uid'])
        with self.assertRaises(ValueError):old_ready(raw,self.args['run_id'],self.args['target_apk_sha256'],self.args['collector_apk_sha256'],self.ready['process']['uid'])
        text='INSTRUMENTATION_RESULT: startupMilestoneProbeCompleted=true\nINSTRUMENTATION_RESULT: committedFrames=0\nINSTRUMENTATION_CODE: -1\n'
        reader.terminal(text)
        with self.assertRaises(ValueError):old_terminal(text)
        with self.assertRaises(ValueError):reader.terminal(text+'INSTRUMENTATION_RESULT: frameProbePassed=true\n')
    def test_no_clock_conversion_or_navigation_url_is_accepted(self):
        self.record['milestones']['navigation']['entries'][0]['url']='http://foreign/'
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.record['nativeClock']='host clock'
        with self.assertRaises(ValueError):self.check()
    def test_navigation_unknown_zero_is_kept_without_claiming_interactive(self):
        self.record['milestones']['navigation']['entries'][0].update(domInteractiveMs=0,domContentLoadedEndMs=0,loadEventEndMs=0,durationMs=0)
        result=self.check();self.assertEqual(result['metrics']['navigation']['domInteractiveMs'],0)
        self.assertIs(result['firstInteractiveTimeProven'],False)
    def test_navigation_and_paint_order_rejected(self):
        self.record['milestones']['navigation']['entries'][0]['domContentLoadedEndMs']=79
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.record['milestones']['firstContentfulPaint']['entries']=[29]
        with self.assertRaises(ValueError):self.check()
    def test_separate_hashed_refusal_cannot_be_used_as_frame_evidence(self):
        value=dict(schemaVersion=2,runId=self.args['run_id'],scope='native-login-startup-milestone-refusal',
            step='startup-milestone-query',errorClass='java.lang.IllegalStateException',reason='milestone-query-rejected',
            lastDocument=self.record['documentBefore'],frames=[],callbacksDetached=True,nativeFramesAccepted=False,
            performanceSampleAccepted=False,nativeStatus='document-ready')
        raw=json.dumps(value).encode();marker=(hashlib.sha256(raw).hexdigest()+'\n').encode()
        self.assertEqual(reader.refusal_contract(raw,marker,self.args['run_id']),value)
        with self.assertRaises(ValueError):old_refusal(raw,marker,self.args['run_id'])
        with self.assertRaises(ValueError):reader.refusal_contract(raw,b'0'*64+b'\n',self.args['run_id'])

if __name__=='__main__':unittest.main()
