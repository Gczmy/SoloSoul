import unittest
import xml.etree.ElementTree as ET
from types import SimpleNamespace
from unittest.mock import patch
import rf312_ime_readiness as readiness


def screen(*,host='[0,0][1080,1124]',document='[0,0][1084,1127]',target='[115,506][803,653]'):
    tree=ET.Element('hierarchy')
    attrs={'package':readiness.APP,'visible-to-user':'true','enabled':'true'}
    outer=ET.SubElement(tree,'node',attrs,**{'class':'android.webkit.WebView','bounds':host})
    inner=ET.SubElement(outer,'node',attrs,**{'class':'android.webkit.WebView','bounds':document})
    node=ET.SubElement(inner,'node',attrs,**{'class':'android.widget.EditText','bounds':target,'text':'','focused':'true','password':'true','content-desc':''})
    probe=dict(imeVisible=True,imeWindowCount=1,imeBounds=['[0,1124][1080,1794]'],ready=True)
    return tree,node,probe


class ViewportReadinessTests(unittest.TestCase):
    def test_observed_stale_password_page_is_not_ready_despite_stable_native_target(self):
        tree,node,probe=screen(document='[0,0][1084,1922]',target='[126,693][811,837]')
        self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_observed_resized_password_page_accepts_virtual_pixel_scale(self):
        tree,node,probe=screen()
        self.assertEqual(readiness.viewport_signature(tree,node,probe),('[0,0][1080,1124]','[0,0][1084,1127]',('[0,1124][1080,1794]',)))

    def test_observed_search_keyboard_height_is_not_hard_coded(self):
        tree,node,probe=screen(host='[0,0][1080,1019]',document='[0,0][1084,1022]',target='[49,273][1015,412]')
        probe['imeBounds']=['[0,1019][1080,1794]']
        self.assertIsNotNone(readiness.viewport_signature(tree,node,probe))

    def test_foreign_target_or_webview_cannot_establish_readiness(self):
        for index in range(3):
            tree,node,probe=screen();list(tree.iter('node'))[index].set('package','other.app')
            with self.subTest(index=index):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_missing_or_ambiguous_webview_ancestry_is_rejected(self):
        for extra in [False,True]:
            tree,node,probe=screen();host=list(tree)[0];document=list(host)[0]
            if extra:
                document.remove(node);third=ET.SubElement(document,'node',document.attrib);third.append(node)
            else:host.remove(document);host.append(node)
            with self.subTest(extra=extra):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_hidden_or_disabled_ancestor_is_rejected(self):
        for field in ['visible-to-user','enabled']:
            tree,node,probe=screen();list(tree)[0].set(field,'false')
            with self.subTest(field=field):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_native_viewport_must_reach_the_actual_docked_ime(self):
        for host in ['[0,0][1080,1794]','[0,0][1080,1100]']:
            tree,node,probe=screen(host=host)
            with self.subTest(host=host):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_absent_ambiguous_or_malformed_ime_is_rejected(self):
        for values in [dict(imeVisible=False),dict(imeWindowCount=2),dict(imeBounds=[]),dict(imeBounds=['invalid']),dict(imeBounds=None)]:
            tree,node,probe=screen();probe.update(values)
            with self.subTest(values=values):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_malformed_or_zero_extent_geometry_is_rejected(self):
        for document in ['invalid','[0,0][0,0]','[0,0][1084,0]']:
            tree,node,probe=screen(document=document)
            with self.subTest(document=document):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_virtual_origin_and_both_dimensions_must_agree(self):
        for document in ['[0,20][1084,1147]','[20,0][1104,1127]','[0,0][1084,1200]','[0,0][900,1127]']:
            tree,node,probe=screen(document=document)
            with self.subTest(document=document):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_input_cannot_lie_outside_resized_native_viewport(self):
        for target in ['[-1,500][803,653]','[115,1000][803,1200]','[115,506][1200,653]']:
            tree,node,probe=screen(target=target)
            with self.subTest(target=target):self.assertIsNone(readiness.viewport_signature(tree,node,probe))

    def test_floating_or_noncovering_ime_is_not_assumed_to_resize_host(self):
        tree,node,probe=screen();probe['imeBounds']=['[30,1124][1000,1794]']
        self.assertIsNone(readiness.viewport_signature(tree,node,probe))


class FakeJourney:
    def __init__(self,frames):
        self.frames=frames;self.clock=0;self.snapshots=0;self.probes=0;self.focuses=[]
        self.observer=SimpleNamespace(inspect=self.inspect,target_ready=lambda n,p,i:p['ready'],target=lambda n:dict(n.attrib))
    def remaining(self,deadline):
        if self.clock>=deadline:raise TimeoutError('original phase deadline')
        return deadline-self.clock
    def snapshot(self,deadline):
        self.clock+=.2;self.snapshots+=1
        self.tree,self.node,self.probe=self.frames[min(self.snapshots-1,len(self.frames)-1)]
    def focused(self,package):self.focuses.append(package)
    def inspect(self,node,intent,timeout):
        self.probes+=1
        assert node is self.node and intent=='type-public' and timeout>0
        return self.probe


class StableInputTests(unittest.TestCase):
    def unique(self,journey,deadline=5,candidates=None):
        with patch.object(readiness.time,'monotonic',side_effect=lambda:journey.clock):
            return readiness.unique_input(journey,candidates or(lambda:[journey.node]),deadline)

    def test_stale_viewport_never_counts_as_stable_and_three_fresh_probes_are_required(self):
        stale=screen(document='[0,0][1084,1922]',target='[126,693][811,837]');fresh=screen()
        journey=FakeJourney([stale]*3+[fresh]*3)
        self.assertIs(self.unique(journey),fresh[1]);self.assertEqual(journey.snapshots,6);self.assertEqual(journey.probes,6)
        self.assertEqual(journey.focuses,[readiness.APP]*6)

    def test_geometry_change_resets_three_observations(self):
        a=screen();b=screen(target='[115,507][803,654]')
        journey=FakeJourney([a,a,b,b,b])
        self.assertIs(self.unique(journey),b[1]);self.assertEqual(journey.snapshots,5)

    def test_legacy_native_readiness_failure_does_not_pass_geometry_alone(self):
        frame=screen();frame[2]['ready']=False;journey=FakeJourney([frame])
        with self.assertRaises(TimeoutError):self.unique(journey,deadline=1)
        self.assertEqual(journey.snapshots,5)

    def test_stale_geometry_expires_at_the_original_phase_deadline(self):
        journey=FakeJourney([screen(document='[0,0][1084,1922]')])
        with self.assertRaises(TimeoutError):self.unique(journey,deadline=1)
        self.assertEqual(journey.snapshots,5)

    def test_expired_phase_never_snapshots_or_inspects(self):
        journey=FakeJourney([screen()])
        with self.assertRaises(TimeoutError):self.unique(journey,deadline=0)
        self.assertEqual((journey.snapshots,journey.probes),(0,0))

    def test_ambiguous_candidate_resets_stability_without_inspection(self):
        journey=FakeJourney([screen()]);calls=[0]
        def candidates():
            calls[0]+=1
            return [journey.node,journey.node] if calls[0]==3 else[journey.node]
        self.unique(journey,candidates=candidates)
        self.assertEqual((journey.snapshots,journey.probes),(6,5))

    def test_missing_candidates_preserve_original_eight_scroll_bound(self):
        journey=FakeJourney([screen()]);scrolls=[]
        journey.scroll=lambda deadline:scrolls.append(deadline) or True
        with patch.object(readiness.time,'monotonic',side_effect=lambda:journey.clock),self.assertRaises(TimeoutError):
            readiness.unique_input(journey,lambda:[],3,allow_scroll=True)
        self.assertEqual(len(scrolls),8);self.assertEqual(journey.probes,0)
