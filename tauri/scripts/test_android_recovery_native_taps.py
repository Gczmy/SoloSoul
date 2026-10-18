"""Reader rejection tests for native frame/coordinate/event binding, no device."""
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('native_tap_reader',Path(__file__).with_name('android-home-native-regression.py'))
runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)

def evidence():
    plans=[];events=[];states=[]
    names=['Host Address','PIN','Host fingerprint','Close']
    for index in range(8):
        theme='light' if index<4 else 'dark';label=index%4!=3
        identity=f'{theme}-{index%4}' if label else None
        width,height=(400,800) if label else (800,400)
        geometry=dict(x=200,y=100,width=width,viewportHeight=height,theme=theme,hittable=True,id=identity,associated=label)
        native=dict(webViewWidthPx=width*2,webViewHeightPx=height*2,webViewScreenX=0,webViewScreenY=0,imeVisible=False)
        plans.append(dict(index=index,caption=names[index%4],label=label,frameCommittedAndGeometryStable=True,
            injectedTapCount=1,injectedSource=4098,injectedToolType=1,preparationFrames=1,beforeScreenshot=f'native-tap-{index}-before.png',
            geometry=geometry,nativeViewport=native,expectedRestoredHeight=height*2,screenX=400,screenY=200,
            activeAfter=identity or '',cardPresentAfter=label))
        events.append(dict(trusted=True,pointerType='touch',inCard=True,theme=theme,clientX=200,clientY=100,
            width=width,height=height,labelFor=identity,buttonLabel='Close' if not label else None))
        if label:states.append(dict(stage=f'recovery-{theme}-label-{index%4}-ime',activeId=identity,expectedActiveId=identity))
    return dict(recoveryTapPlans=plans,recoveryPointerEvents=events,recoveryRecords=states)

class NativeTapReaderTests(unittest.TestCase):
    def test_complete_committed_frames_native_mapping_and_actual_events_are_accepted(self):
        self.assertEqual(runner.verify_recovery_taps(evidence()),8)
        report=dict(nativePassed=True,restored=True,blurRestored=True,recoveryCardsRequested=True,
                    recoveryCardsPassed=True,cameraPermissionRestored=True,nativeTapCount=8)
        self.assertTrue(runner.verification_passed(report,False))
        for count in (None,0,7,9):
            with self.subTest(count=count):self.assertFalse(runner.verification_passed(dict(report,nativeTapCount=count),False))

    def test_stale_geometry_wrong_focus_or_untrusted_non_touch_events_are_rejected(self):
        for field,value in [('frameCommittedAndGeometryStable',False),('injectedTapCount',2),('injectedSource',8194),('injectedToolType',0),('preparationFrames',4),
                            ('beforeScreenshot','other.png'),('activeAfter','other'),('cardPresentAfter',False),
                            ('screenY',300),('expectedRestoredHeight',1000)]:
            data=evidence();data['recoveryTapPlans'][0][field]=value
            with self.subTest(field=field),self.assertRaises(RuntimeError):runner.verify_recovery_taps(data)
        for field,value in [('trusted',False),('pointerType','mouse'),('inCard',False),('theme','dark'),
                            ('clientY',150),('height',400),('labelFor','other')]:
            data=evidence();data['recoveryPointerEvents'][0][field]=value
            with self.subTest(field=field),self.assertRaises(RuntimeError):runner.verify_recovery_taps(data)
        for field,value in [('y',-1),('width',float('nan')),('associated',False)]:
            data=evidence();data['recoveryTapPlans'][0]['geometry'][field]=value
            with self.subTest(field=field),self.assertRaises(RuntimeError):runner.verify_recovery_taps(data)
        data=evidence();data['recoveryTapPlans'][0]['nativeViewport']['imeVisible']=True
        with self.assertRaises(RuntimeError):runner.verify_recovery_taps(data)

    def test_all_single_touches_must_match_their_own_focus_captures_and_real_close(self):
        for mutation in ('missing-plan','extra-event','wrong-order','missing-state','foreign-focus-state','close-not-dismissed','close-wrong-hit'):
            data=evidence()
            if mutation=='missing-plan':data['recoveryTapPlans'].pop()
            elif mutation=='extra-event':data['recoveryPointerEvents'].append(dict(data['recoveryPointerEvents'][0]))
            elif mutation=='wrong-order':data['recoveryTapPlans'][0],data['recoveryTapPlans'][1]=data['recoveryTapPlans'][1],data['recoveryTapPlans'][0]
            elif mutation=='missing-state':data['recoveryRecords'].pop()
            elif mutation=='foreign-focus-state':data['recoveryRecords'][0]['activeId']='other'
            elif mutation=='close-not-dismissed':data['recoveryTapPlans'][3]['cardPresentAfter']=True
            else:data['recoveryPointerEvents'][3]['buttonLabel']='Other'
            with self.subTest(mutation=mutation),self.assertRaises(RuntimeError):runner.verify_recovery_taps(data)

if __name__=='__main__':unittest.main()
