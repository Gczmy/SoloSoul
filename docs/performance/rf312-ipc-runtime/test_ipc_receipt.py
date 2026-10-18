import copy
import unittest

import ipc_receipt as receipt

ID='a'*32
PROCESS=dict(processId=123,uid=10001,startTicks=456)


def sample():
    records=[];events=[]
    for index,name in enumerate(receipt.PHASES):
        events.append(dict(command='login' if index==1 else 'ui_get_preferences',
                           transport='android-java-bridge',atMs=index*10+2))
        value=dict(schemaVersion=1,scope='android-native-ipc-transport-observer',runId=ID,
                   valid=True,invalidReasons=[],installedAtMs=0,timeOriginMs=100000,atMs=index*10+3,
                   maxEvents=16384,observedCount=len(events),commands=copy.deepcopy(events))
        document=dict(present=True,platform='android',mainFrame=True,frames=0,invokeReadonly=True,observer=value)
        records.append(dict(stage=name,process=dict(PROCESS),nativeReadStartedNs=index*100+1,
                            nativeReadFinishedNs=index*100+2,document=document))
    return records


class CollectorReceiptTests(unittest.TestCase):
    def validate(self,records):return receipt.validate(records,run_id=ID,expected_process=PROCESS)

    def test_complete_monotonic_prefix_counts_real_transport_categories(self):
        result=self.validate(sample())
        self.assertEqual(result['sendAttempts'],12)
        self.assertEqual(sum(row['sendAttempts'] for row in result['phases']),12)
        self.assertTrue(all(row['sendAttempts']==1 for row in result['phases']))

    def test_zero_new_events_is_a_valid_phase_not_missing_data(self):
        records=sample();last=records[-1]['document']['observer'];last['commands'].pop();last['observedCount']-=1
        self.assertEqual(self.validate(records)['phases'][-1]['sendAttempts'],0)

    def test_all_stages_and_exact_order_required(self):
        for mutation in [lambda r:r.pop(),lambda r:r.reverse(),lambda r:r[5].update(stage=r[4]['stage'])]:
            with self.subTest(mutation=mutation):
                records=sample();mutation(records)
                with self.assertRaises(ValueError):self.validate(records)

    def test_process_replacement_or_boolean_identity_rejected(self):
        for key,value in [('processId',124),('uid',10002),('startTicks',457),('processId',True)]:
            with self.subTest(key=key):
                records=sample();records[5]['process'][key]=value
                with self.assertRaises(ValueError):self.validate(records)

    def test_document_replacement_and_rebinding_rejected(self):
        for key,value in [('timeOriginMs',100001),('installedAtMs',1),('runId','b'*32)]:
            with self.subTest(key=key):
                records=sample();records[5]['document']['observer'][key]=value
                with self.assertRaises(ValueError):self.validate(records)

    def test_payloads_and_extra_document_or_native_fields_rejected(self):
        for mutation in [lambda r:r[0].update(payload='forbidden'),
                         lambda r:r[0]['document'].update(url='forbidden'),
                         lambda r:r[0]['document']['observer']['commands'][0].update(payload='forbidden')]:
            with self.subTest(mutation=mutation):
                records=sample();mutation(records)
                with self.assertRaises(ValueError):self.validate(records)

    def test_prefix_edit_deletion_or_reordering_rejected(self):
        for mutation in [lambda v:v['commands'][0].update(command='changed'),
                         lambda v:v['commands'].__setitem__(slice(None,2),list(reversed(v['commands'][:2]))),
                         lambda v:v.update(commands=v['commands'][1:],observedCount=v['observedCount']-1)]:
            with self.subTest(mutation=mutation):
                records=sample();mutation(records[5]['document']['observer'])
                with self.assertRaises(ValueError):self.validate(records)

    def test_inserted_event_before_prior_boundary_rejected(self):
        records=sample();records[5]['document']['observer']['commands'][-1]['atMs']=42
        with self.assertRaises(ValueError):self.validate(records)

    def test_invalid_count_overflow_and_observer_invalidity_rejected(self):
        for mutation in [lambda v:v.update(observedCount=0),lambda v:v.update(observedCount=True),
                         lambda v:v.update(maxEvents=16385),lambda v:v.update(valid=False),
                         lambda v:v.update(invalidReasons=['event-overflow'])]:
            with self.subTest(mutation=mutation):
                records=sample();mutation(records[0]['document']['observer'])
                with self.assertRaises(ValueError):self.validate(records)

    def test_extra_frame_or_mutable_invoke_rejected(self):
        for key,value in [('mainFrame',False),('frames',1),('frames',True),('invokeReadonly',False)]:
            with self.subTest(key=key):
                records=sample();records[5]['document'][key]=value
                with self.assertRaises(ValueError):self.validate(records)

    def test_nonfinite_or_backward_clocks_and_expired_native_read_rejected(self):
        for mutation in [lambda r:r[5]['document']['observer'].update(atMs=float('nan')),
                         lambda r:r[5]['document']['observer'].update(atMs=1),
                         lambda r:r[5].update(nativeReadFinishedNs=3_000_000_502),
                         lambda r:r[5].update(nativeReadStartedNs=True)]:
            with self.subTest(mutation=mutation):
                records=sample();mutation(records)
                with self.assertRaises(ValueError):self.validate(records)

    def test_synthetic_contract_success_never_promotes_actual_native_acceptance(self):
        result=self.validate(sample())
        for key in ['actualIpcAccepted','liveProcessAndApkIdentityAudited','physicalUiAndRestorationAudited','fullRF312Accepted']:
            self.assertIs(result[key],False,key)


if __name__=='__main__':unittest.main()
