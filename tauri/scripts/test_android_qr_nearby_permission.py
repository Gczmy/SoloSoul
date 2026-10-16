"""Nearby Devices identity/restoration rejection tests; no device evidence."""
import copy
import unittest
import test_android_qr_native_evidence as qr

runner = qr.runner

def prompt():
    return dict(controllerPackage='com.android.permissioncontroller', controllerIsSystemApp=True, appLabel='SoloSoul',
                message='Allow SoloSoul to find, connect to, and determine the relative position of nearby devices?',
                allowLabel='ALLOW', allowEnabled=True, allowClickable=True, permission='android.permission.NEARBY_WIFI_DEVICES',
                phase='qr-enable-sync', explicitEnablePending=True, permissionGrantedBefore=False, permissionGrantedAfter=True,
                cameraPermissionUnchanged=True, grantedByNativeUi=True, nativeActionCount=1,
                screenshot='nearby-permission-0.png', tree=[dict(text='public')])

class NearbyPermissionTests(unittest.TestCase):
    def test_parser_requires_one_exact_permission_and_preserves_all_flags(self):
        text=' android.permission.NEARBY_WIFI_DEVICES: granted=false, flags=[USER_SET| USER_SENSITIVE_WHEN_GRANTED]\n'
        self.assertEqual(runner.nearby_permission(text), dict(granted=False, flags=['USER_SENSITIVE_WHEN_GRANTED','USER_SET']))
        for bad in ('',text+text,text.replace('NEARBY_WIFI_DEVICES','ACCESS_FINE_LOCATION'),text.replace('false','unknown')):
            with self.subTest(text=bad),self.assertRaises(RuntimeError):runner.nearby_permission(bad)

    def test_restore_recovers_grant_and_exact_user_flags_without_touching_camera(self):
        for original in (dict(granted=False,flags=['USER_SENSITIVE_WHEN_DENIED']),dict(granted=True,flags=['USER_FIXED','USER_SET'])):
            state=dict(granted=not original['granted'],flags=list(set(original['flags'])|{'USER_SET'}))
            calls=[]
            def read(*args):
                return ' android.permission.NEARBY_WIFI_DEVICES: granted='+str(state['granted']).lower()+', flags=['+'|'.join(state['flags'])+']\n'
            def adb(*args,**kwargs):
                calls.append(args)
                self.assertEqual(args[:2],('shell','pm'))
                self.assertEqual(args[3:5],(runner.APP,'android.permission.NEARBY_WIFI_DEVICES'))
                if args[2] in ('grant','revoke'):state['granted']=args[2]=='grant'
                elif args[2]=='clear-permission-flags':state['flags']=[f for f in state['flags'] if f not in ('USER_SET','USER_FIXED')]
                elif args[2]=='set-permission-flags':state['flags'] += [f.upper().replace('-','_') for f in args[5:]]
                else:self.fail('Unknown permission mutation')
            runner.restore_nearby_permission(adb,read,original)
            self.assertEqual(runner.nearby_permission(read()),original)
            self.assertGreaterEqual(len(calls),2)

    def test_failed_or_unrestored_permission_and_unknown_flags_cannot_pass(self):
        def adb(*args,**kwargs):pass
        def read(*args):return ' android.permission.NEARBY_WIFI_DEVICES: granted=false, flags=[USER_FIXED]\n'
        with self.assertRaisesRegex(RuntimeError,'differ'):runner.restore_nearby_permission(adb,read,dict(granted=False,flags=[]))
        good=dict(nativePassed=True,restored=True,blurRestored=True,qrCardsRequested=True,qrCardsPassed=True,
                  cameraPermissionRestored=True,nearbyPermissionRestored=True,qrTapCount=8)
        self.assertTrue(runner.verification_passed(good,False))
        for failure in (dict(nearbyPermissionRestored=False),dict(nearbyRestoreError='not restored'),dict(restoreError='data failed')):
            self.assertFalse(runner.verification_passed(dict(good,**failure),False))

    def test_exact_native_controller_phase_and_unchanged_camera_proof_are_required(self):
        row=prompt()
        self.assertEqual(runner.verify_qr_nearby_prompts(dict(qrNearbyPrompts=[row])),1)
        self.assertEqual(runner.verify_qr_nearby_prompts(dict(qrNearbyPrompts=[])),0)
        for key,value in [('controllerPackage','third.party'),('controllerIsSystemApp',False),('appLabel','Other App'),
                          ('message','Allow SoloSoul notifications?'),('allowLabel','DON’T ALLOW'),('allowEnabled',False),
                          ('allowClickable',False),('permission','android.permission.ACCESS_FINE_LOCATION'),('phase','startup'),
                          ('explicitEnablePending',False),('permissionGrantedBefore',True),('permissionGrantedAfter',False),
                          ('cameraPermissionUnchanged',False),('grantedByNativeUi',False),('nativeActionCount',2),
                          ('screenshot','../other.png'),('tree',[])]:
            data=dict(row,**{key:value})
            with self.subTest(key=key),self.assertRaises(RuntimeError):runner.verify_qr_nearby_prompts(dict(qrNearbyPrompts=[data]))
        with self.assertRaises(RuntimeError):runner.verify_qr_nearby_prompts({})
        with self.assertRaises(RuntimeError):runner.verify_qr_nearby_prompts(dict(qrNearbyPrompts=[row,row,row]))

    def test_nearby_button_must_be_exact_allow_and_a_single_real_action(self):
        for caption in ('Allow','ALLOW','allow'):
            self.assertEqual(runner.verify_qr_nearby_prompts(dict(qrNearbyPrompts=[dict(prompt(),allowLabel=caption)])),1)
        for caption in ('Allow all','ONLY THIS TIME',None,1,'Dont allow'):
            with self.subTest(caption=caption),self.assertRaises(RuntimeError):runner.verify_qr_nearby_prompts(dict(qrNearbyPrompts=[dict(prompt(),allowLabel=caption)]))
        with self.assertRaises(RuntimeError):runner.verify_qr_nearby_prompts(dict(qrNearbyPrompts=[dict(prompt(),nativeActionCount=True)]))

    def test_api_below_33_and_horizontal_content_overflow_are_rejected_without_location_grants(self):
        data=qr.evidence();data['runtime']['api']=32
        with self.assertRaises(RuntimeError):runner.verify_qr_evidence(data)
        for key,value in [('cardClientWidth',None),('cardScrollWidth',float('nan')),('cardClientWidth',0),('cardScrollWidth',352)]:
            data=qr.evidence();data['qrRecords'][2][key]=value
            with self.subTest(key=key),self.assertRaises(RuntimeError):runner.verify_qr_evidence(data)

if __name__=='__main__':unittest.main()
