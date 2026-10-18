import copy,json,unittest
from rf312_milestone_statistics import summarize,decode

def fixture():
    protocol=dict(schemaVersion=1,scope='rf312-document-startup-five-queue-protocol',plannedQueues=5,objectCounts=[100,5000],
        clock='document-relative-ms',order='ascending-slots-then-100-5000',failurePolicy='stop-and-retain-no-replacement',
        quantile='nearest-rank-p95',samplerSourcesSha256='a'*64,statisticsSourceSha256='b'*64,
        targetApkSha256='c'*64,collectorApkSha256='d'*64,exploratoryPairIncluded=False)
    queues=[]
    for slot,interval in enumerate([300,100,200,500,400],1):
        samples=[]
        for count in [100,5000]:
            samples.append(dict(objectCount=count,metadataSha256='e'*64,metrics=dict(reactMountMs=100,
                startupDismissedMs=100+interval,mountInvocationToHandoffMs=interval,firstPaintMs=20,firstContentfulPaintMs=30)))
        queues.append(dict(slot=slot,status='accepted',reason=None,evidenceSha256='f'*64,samplerSourcesSha256='a'*64,samples=samples))
    return dict(schemaVersion=1,scope='rf312-document-startup-milestone-observations',protocolSha256='1'*64,queues=queues),protocol

class MilestoneStatisticsTests(unittest.TestCase):
    def setUp(self):self.value,self.protocol=fixture()
    def check(self):return summarize(self.value,self.protocol,'1'*64)
    def stop(self,slot,status='refused'):
        for index,q in enumerate(self.value['queues'][slot-1:],slot):
            q.update(status=status if index==slot else 'not-started',reason='native-rejected' if index==slot and status=='refused' else None,
                evidenceSha256='f'*64 if index==slot and status=='refused' else None,
                samplerSourcesSha256='a'*64 if index==slot and status=='refused' else None,samples=[])
    def test_five_values_have_expected_median_and_nearest_rank_tail(self):
        r=self.check();m=r['metrics']['100']['mountInvocationToHandoffMs']
        self.assertEqual((m['medianMs'],m['p95Ms'],m['availableN']),(300,500,5));self.assertTrue(r['statisticsComplete'])
        for key in ['nativeVerificationPerformed','nativeFramesAccepted','firstInteractiveTimeProven','applicationOnlyLatencyAccepted',
                    'observerOverheadRemoved','defaultPerformanceSampleAccepted','fullRF312Accepted']:self.assertIs(r[key],False)
    def test_one_exploratory_sized_prefix_cannot_produce_median_or_tail(self):
        self.stop(2,status='not-started');r=self.check();m=r['metrics']['100']['reactMountMs']
        self.assertFalse(r['statisticsComplete']);self.assertIsNone(m['medianMs']);self.assertIsNone(m['p95Ms']);self.assertEqual(m['availableN'],1)
    def test_failed_queue_and_unstarted_slots_are_retained_without_replacement(self):
        self.stop(3);r=self.check();self.assertEqual((r['acceptedQueues'],r['refusedQueues'],r['notStartedQueues']),(2,1,2))
        self.assertEqual(r['queueStatuses'][2]['evidenceSha256'],'f'*64);self.assertFalse(r['statisticsComplete'])
    def test_missing_paint_is_null_not_a_zero_sample(self):
        for slot in [2,4]:
            for sample in self.value['queues'][slot-1]['samples']:sample['metrics'].update(firstPaintMs=None,firstContentfulPaintMs=None)
        m=self.check()['metrics']['100']['firstPaintMs'];self.assertEqual((m['availableN'],m['missingN']),(3,2));self.assertFalse(m['distributionComplete'])
    def test_duplicate_reordered_and_extra_slots_are_rejected(self):
        self.value['queues'][1]['slot']=1
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.value['queues'].reverse()
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.value['queues'].append(copy.deepcopy(self.value['queues'][0]))
        with self.assertRaises(ValueError):self.check()
    def test_resume_after_a_refusal_is_a_protocol_violation(self):
        saved=copy.deepcopy(self.value['queues'][4]);self.stop(3);self.value['queues'][4]=saved
        with self.assertRaises(ValueError):self.check()
    def test_source_drift_and_reused_exploratory_protocol_are_rejected(self):
        self.value['queues'][2]['samplerSourcesSha256']='2'*64
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.protocol['exploratoryPairIncluded']=True
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.value['protocolSha256']='3'*64
        with self.assertRaises(ValueError):self.check()
    def test_failure_prefix_cannot_smuggle_numeric_samples(self):
        saved=copy.deepcopy(self.value['queues'][2]['samples']);self.stop(3);self.value['queues'][2]['samples']=saved
        with self.assertRaises(ValueError):self.check()
    def test_required_marks_and_handoff_difference_must_remain_valid(self):
        self.value['queues'][0]['samples'][0]['metrics']['reactMountMs']=None
        with self.assertRaises(ValueError):self.check()
        self.setUp();self.value['queues'][0]['samples'][0]['metrics']['mountInvocationToHandoffMs']+=1
        with self.assertRaises(ValueError):self.check()
    def test_nonfinite_bool_negative_and_private_payloads_are_rejected(self):
        for number in [float('nan'),float('inf'),True,-1]:
            self.setUp();self.value['queues'][0]['samples'][0]['metrics']['firstPaintMs']=number
            with self.assertRaises(ValueError):self.check()
        self.setUp();self.value['queues'][0]['samples'][0]['account']='foreign'
        with self.assertRaises(ValueError):self.check()
    def test_unstarted_slot_cannot_claim_an_evidence_hash(self):
        self.stop(1,status='not-started');self.value['queues'][0]['evidenceSha256']='f'*64
        with self.assertRaises(ValueError):self.check()
    def test_aggregation_does_not_mutate_or_sort_original_observations(self):
        before=copy.deepcopy(self.value);self.check();self.assertEqual(self.value,before)
    def test_duplicate_json_and_nonfinite_json_are_rejected(self):
        for raw in [b'{"slot":1,"slot":2}',b'{"time":NaN}',b'{"time":Infinity}']:
            with self.assertRaises(ValueError):decode(raw)
    def test_invalid_evidence_digest_cannot_be_counted(self):
        self.value['queues'][1]['evidenceSha256']='foreign'
        with self.assertRaises(ValueError):self.check()
    def test_even_median_remains_finite_for_large_finite_inputs(self):
        self.stop(5,status='not-started')
        for queue in self.value['queues'][:4]:
            for sample in queue['samples']:
                sample['metrics'].update(reactMountMs=1e308,startupDismissedMs=1e308,mountInvocationToHandoffMs=0)
        self.assertEqual(self.check()['metrics']['100']['reactMountMs']['medianMs'],1e308)

if __name__=='__main__':unittest.main()
