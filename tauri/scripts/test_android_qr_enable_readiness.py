"""Reject UI-only enable evidence; not native runtime proof."""
import unittest
import test_android_qr_native_evidence as qr

class EnableReadinessTests(unittest.TestCase):
    def test_both_phases_need_real_native_permission_and_matching_ui(self):
        self.assertEqual(len(qr.runner.verify_qr_evidence(qr.evidence())),40)
        for key,value in [('nearbyPermissionGranted',False),('explicitEnablePending',False),('syncEnabled',False),
                          ('showReady',False),('realIpc',False),('mocked',True),('path','/login'),('phase','startup')]:
            data=qr.evidence();data['qrEnableReadiness'][0][key]=value
            with self.subTest(key=key),self.assertRaises(RuntimeError):qr.runner.verify_qr_evidence(data)

    def test_missing_duplicate_or_reversed_phases_cannot_replace_complete_readiness(self):
        for mutation in ('missing','duplicate','reverse'):
            data=qr.evidence();rows=data['qrEnableReadiness']
            if mutation=='missing':rows.pop()
            elif mutation=='duplicate':rows[1]=dict(rows[0])
            else:rows.reverse()
            with self.subTest(mutation=mutation),self.assertRaises(RuntimeError):qr.runner.verify_qr_evidence(data)

if __name__=='__main__':unittest.main()
