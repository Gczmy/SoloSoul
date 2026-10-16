"""Synthetic reader rejection tests only; not native runtime evidence."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('recovery_home_runner',Path(__file__).with_name('android-home-native-regression.py'))
runner=importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

def evidence():
    rows=[]
    for theme in ('light','dark'):
        for suffix in ('portrait','next-portrait','label-0-ime','label-1-ime','label-2-ime',
                       'landscape','next-landscape','close-landscape','returned-home'):
            stage=f'recovery-{theme}-{suffix}'
            if suffix=='returned-home':
                rows.append(dict(stage=stage,theme=theme,nativeCloseSucceeded=True,unlocked=True))
                continue
            landscape='landscape' in suffix
            ime=suffix.startswith('label-')
            width,height=(844,390) if landscape else (390,300 if ime else 844)
            x,y,w,h=16,16,min(350,width-32),height-56
            ids=['host','pin','fingerprint']
            names=['Host Address','PIN','Host fingerprint']
            targets=[dict(tag='LABEL',text=n,htmlFor=v) for n,v in zip(names,ids)]
            targets += [dict(tag='INPUT',id=v) for v in ids]
            targets += [dict(tag='BUTTON',text='Next',hittable=True),dict(tag='BUTTON',aria='Close',hittable=True)]
            active=ids[int(suffix.split('-')[1])] if ime else None
            rows.append(dict(stage=stage,theme=theme,dark=theme=='dark',path='/login',platform='android',
                bounded=True,realIpc=True,screenshotStateConfirmed=True,mocked=False,documentOverflow=False,
                overflowY='auto',ime=ime,card=dict(x=x,y=y,width=w,height=h,right=x+w,bottom=y+h),
                width=width,height=height,screenshotWidth=1600 if landscape else 720,screenshotHeight=720 if landscape else 1600,
                scrollHeight=900,clientHeight=h,targets=targets,activeId=active,expectedActiveId=active,
                nativeViewport=dict(imeVisible=ime,webViewWidthPx=1600 if landscape else 720,webViewScreenY=0,
                    screenHeightPx=720 if landscape else 1600,imeHeightPx=800 if ime else 0,
                    systemBars=dict(bottom=48),windowBounds=dict(left=0,top=0,right=1600 if landscape else 720,bottom=720 if landscape else 1600))))
    return dict(recoveryCardsRequested=True,scenario='baseline',runtime=dict(api=35,webViewPackage='com.android.webview',webViewVersion='124'),recoveryRecords=rows)

class RecoveryEvidenceTests(unittest.TestCase):
    def test_complete_native_geometry_and_ime_shallow_css_viewport_are_accepted(self):
        self.assertEqual(len(runner.verify_recovery_evidence(evidence())),18)

    def test_exact_flag_scenario_order_and_runtime_are_required(self):
        for key,value in [('recoveryCardsRequested',False),('scenario','keyboard'),('runtime',{})]:
            data=evidence();data[key]=value
            with self.subTest(key=key),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)
        for mutation in ('drop','duplicate','swap'):
            data=evidence();rows=data['recoveryRecords']
            if mutation=='drop':rows.pop()
            elif mutation=='duplicate':rows.append(copy.deepcopy(rows[-1]))
            else:rows[0],rows[1]=rows[1],rows[0]
            with self.subTest(mutation=mutation),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)

    def test_boolean_claims_cannot_replace_real_state_or_native_keyboard(self):
        for key,value in [('bounded',1),('realIpc',False),('mocked',True),('screenshotStateConfirmed',False),
                          ('documentOverflow',True),('theme','dark'),('ime',True),('path','/fixture')]:
            data=evidence();data['recoveryRecords'][0][key]=value
            with self.subTest(key=key),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)
        data=evidence();data['recoveryRecords'][2]['nativeViewport']['imeVisible']=False
        with self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)

    def test_clipped_nonfinite_inconsistent_and_native_occluded_bounds_are_rejected(self):
        for key,value in [('x',-1),('y',0),('bottom',10000),('right',400),('width',float('nan')),('height',0)]:
            data=evidence();data['recoveryRecords'][0]['card'][key]=value
            with self.subTest(key=key),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)
        data=evidence();data['recoveryRecords'][2]['nativeViewport']['imeHeightPx']=1500
        with self.assertRaisesRegex(RuntimeError,'obscured'):runner.verify_recovery_evidence(data)

    def test_label_association_and_actual_native_focus_are_required(self):
        for mutation in ('duplicate-id','wrong-label','empty-id','missing-label','wrong-focus'):
            data=evidence();row=data['recoveryRecords'][2];targets=row['targets']
            if mutation=='duplicate-id':targets[4]['id']=targets[3]['id']
            elif mutation=='wrong-label':targets[0]['htmlFor']='pin'
            elif mutation=='empty-id':targets[3]['id']=''
            elif mutation=='missing-label':targets.pop(0)
            else:row['activeId']='pin'
            with self.subTest(mutation=mutation),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)

    def test_shallow_case_requires_real_rotation_and_internal_scroll(self):
        for mutation in ('orientation','missing-bounds','no-scroll'):
            data=evidence();row=data['recoveryRecords'][5]
            if mutation=='orientation':row['nativeViewport']['windowBounds']['bottom']=2000
            elif mutation=='missing-bounds':row['nativeViewport'].pop('windowBounds')
            else:row['scrollHeight']=row['clientHeight']
            with self.subTest(mutation=mutation),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)

    def test_both_terminal_buttons_and_returned_home_are_independently_required(self):
        for index in (1,6,7):
            data=evidence();data['recoveryRecords'][index]['targets'][-2 if index!=7 else -1]['hittable']=False
            with self.subTest(index=index),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)
        for key in ('unlocked','nativeCloseSucceeded'):
            data=evidence();data['recoveryRecords'][-1][key]=False
            with self.subTest(key=key),self.assertRaises(RuntimeError):runner.verify_recovery_evidence(data)

    def test_new_case_never_bypasses_original_restore_or_native_failure(self):
        passed=dict(nativePassed=True,restored=True,blurRestored=True,recoveryCardsRequested=True,recoveryCardsPassed=True,cameraPermissionRestored=True,nativeTapCount=8)
        self.assertTrue(runner.verification_passed(passed,False))
        for key in ('nativePassed','restored','blurRestored','recoveryCardsPassed','cameraPermissionRestored'):
            data=dict(passed);data[key]=False
            with self.subTest(key=key):self.assertFalse(runner.verification_passed(data,False))
        data=dict(passed,error='failed baseline');self.assertFalse(runner.verification_passed(data,False))

    def test_camera_parser_requires_unique_actual_camera_state(self):
        raw='  android.permission.CAMERA: granted=false, flags=[USER_SET| USER_SENSITIVE_WHEN_GRANTED]\n'
        self.assertEqual(runner.camera_permission(raw),dict(granted=False,flags=['USER_SENSITIVE_WHEN_GRANTED','USER_SET']))
        for invalid in ('',raw+raw,raw.replace('CAMERA','POST_NOTIFICATIONS'),raw.replace('granted=false','granted=unknown')):
            with self.subTest(invalid=invalid),self.assertRaises(RuntimeError):runner.camera_permission(invalid)

    def test_camera_restore_recovers_grant_and_user_flags_exactly(self):
        for original in [dict(granted=False,flags=['USER_SENSITIVE_WHEN_DENIED']),dict(granted=True,flags=['USER_FIXED','USER_SET'])]:
            state=dict(granted=not original['granted'],flags=list(set(original['flags'])|{'USER_SET'}))
            calls=[]
            def read(*args):
                return ' android.permission.CAMERA: granted='+str(state['granted']).lower()+', flags=['+'|'.join(state['flags'])+']\n'
            def adb(*args,**kwargs):
                calls.append(args);self.assertEqual(args[:2],('shell','pm'));self.assertEqual(args[3:5],(runner.APP,'android.permission.CAMERA'))
                if args[2] in ('grant','revoke'):state['granted']=args[2]=='grant'
                elif args[2]=='clear-permission-flags':state['flags']=[f for f in state['flags'] if f not in ('USER_SET','USER_FIXED')]
                elif args[2]=='set-permission-flags':state['flags']+= [f.upper().replace('-','_') for f in args[5:]]
                else:self.fail('Unknown mutation')
            runner.restore_camera_permission(adb,read,original)
            self.assertEqual(runner.camera_permission(read()),original)
            self.assertGreaterEqual(len(calls),2)

    def test_camera_restore_refuses_unrestored_unknown_flags_and_failures(self):
        def adb(*args,**kwargs):pass
        def read(*args):return ' android.permission.CAMERA: granted=false, flags=[USER_FIXED]\n'
        with self.assertRaisesRegex(RuntimeError,'differ'):runner.restore_camera_permission(adb,read,dict(granted=False,flags=[]))
        passed=dict(nativePassed=True,restored=True,blurRestored=True,recoveryCardsRequested=True,recoveryCardsPassed=True,cameraPermissionRestored=True,nativeTapCount=8,cameraRestoreError='not restored')
        self.assertFalse(runner.verification_passed(passed,False))

    def test_camera_prompt_evidence_rejects_other_apps_requests_and_unproven_denial(self):
        row=dict(controllerPackage='com.android.permissioncontroller',controllerIsSystemApp=True,appLabel='SoloSoul',message='Allow SoloSoul to take pictures and record video?',
                 denyLabel="Don't allow",permission='android.permission.CAMERA',deniedByNativeUi=True,permissionDeniedAfter=True,screenshot='camera-permission-0.png',tree=[dict(text='public')])
        self.assertEqual(runner.verify_camera_prompts(dict(recoveryCameraPrompts=[row])),1)
        for key,value in [('controllerPackage','third.party'),('controllerIsSystemApp',False),('appLabel','Other App'),('message','Allow SoloSoul notifications?'),
                          ('denyLabel','Allow'),('permission','android.permission.RECORD_AUDIO'),('deniedByNativeUi',False),
                          ('permissionDeniedAfter',False),('screenshot','../other.png'),('tree',[])]:
            candidate=dict(row);candidate[key]=value
            with self.subTest(key=key),self.assertRaises(RuntimeError):runner.verify_camera_prompts(dict(recoveryCameraPrompts=[candidate]))

    def test_camera_deny_caption_accepts_actual_native_uppercase_but_never_allow_actions(self):
        row=dict(controllerPackage='com.android.permissioncontroller',controllerIsSystemApp=True,appLabel='SoloSoul',
                 message='Allow SoloSoul to take pictures and record video?',permission='android.permission.CAMERA',
                 deniedByNativeUi=True,permissionDeniedAfter=True,screenshot='camera-permission-0.png',tree=[dict(text='public')])
        for text in ("Don't allow","DON'T ALLOW",'DON\u2019T ALLOW'):
            with self.subTest(text=text):self.assertEqual(runner.verify_camera_prompts(dict(recoveryCameraPrompts=[dict(row,denyLabel=text)])),1)
        for text in ('ALLOW','ONLY THIS TIME','WHILE USING THE APP',"DON'T ALLOW OTHER",None,1):
            with self.subTest(text=text),self.assertRaises(RuntimeError):runner.verify_camera_prompts(dict(recoveryCameraPrompts=[dict(row,denyLabel=text)]))

if __name__=='__main__':unittest.main()
