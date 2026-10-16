import copy
import unittest
import types
import xml.etree.ElementTree as ET

from body_navigation import APP
from preview_scope import (close_preview, native_pdf_close_candidates, native_pdf_present,
                           preview_controls_present, web_preview_close_candidates)


def node(text, bounds, kind='android.widget.Button', clickable='true', view_id=''):
    return ET.Element('node', {'package': APP, 'class': kind, 'text': text, 'content-desc': '',
                              'bounds': bounds, 'enabled': 'true', 'visible-to-user': 'true',
                              'clickable': clickable, 'view-id': view_id})


class Preview(unittest.TestCase):
    def setUp(self):
        self.background = [node('Close', '[858,181][987,309]'), node('Close', '[863,391][992,519]')]
        self.header = node('', '[0,0][1084,244]', 'android.view.View')
        for child in [node('Back', '[34,86][165,217]'),
                      node('ocr_test.png', '[189,128][396,175]', 'android.widget.TextView', 'false'),
                      node('Edit Attachment Attributes', '[766,86][895,217]'),
                      node('Close', '[918,86][1047,217]')]:
            self.header.append(child)

    def nodes(self):
        return [*self.background, *self.header.iter('node')]

    def test_recorded_three_close_nodes_select_current_file_toolbar_only(self):
        self.assertEqual(sum(n.get('text') == 'Close' for n in self.nodes()), 3)
        found = web_preview_close_candidates(self.nodes(), 'ocr_test.png')
        self.assertEqual(found, [self.header[-1]])
        self.assertTrue(preview_controls_present(self.nodes()))

    def test_wrong_or_nonfixture_filename_cannot_select_close(self):
        self.assertEqual(web_preview_close_candidates(self.nodes(), 'preview.txt'), [])
        with self.assertRaises(ValueError):
            web_preview_close_candidates(self.nodes(), '../unknown.png')

    def test_duplicate_matching_toolbar_refuses_even_if_one_disabled(self):
        duplicate = copy.deepcopy(self.header)
        duplicate.set('enabled', 'false')
        self.assertEqual(web_preview_close_candidates([*self.nodes(), duplicate], 'ocr_test.png'), [])

    def test_missing_back_edit_or_close_has_no_generic_fallback(self):
        for index in [0, 2, 3]:
            with self.subTest(index=index):
                group = copy.deepcopy(self.header)
                group.remove(group[index])
                self.assertEqual(web_preview_close_candidates([*self.background, group], 'ocr_test.png'), [])

    def test_wrong_owner_state_or_outside_geometry_refuses(self):
        for target in [self.header, *self.header]:
            for attribute, value in [('package', 'unrelated.app'), ('enabled', 'false'),
                                     ('visible-to-user', 'false'), ('bounds', '[0,0][0,0]')]:
                with self.subTest(attribute=attribute, target=target.get('text')):
                    original = target.get(attribute)
                    target.set(attribute, value)
                    self.assertEqual(web_preview_close_candidates(self.nodes(), 'ocr_test.png'), [])
                    target.set(attribute, original)
        self.header[-1].set('bounds', '[918,86][1047,300]')
        self.assertEqual(web_preview_close_candidates(self.nodes(), 'ocr_test.png'), [])

    def test_native_pdf_requires_exact_owned_ids_and_filename(self):
        title = node('text_only.pdf', '[160,20][800,100]', 'android.widget.TextView', 'false', APP + ':id/pdfTitle')
        close = node('Close', '[900,20][1000,100]', 'android.widget.ImageButton', 'true', APP + ':id/pdfCloseButton')
        self.assertEqual(native_pdf_close_candidates([title, close], 'text_only.pdf'), [close])
        self.assertEqual(native_pdf_close_candidates([title, close], 'scanned.pdf'), [])
        self.assertEqual(native_pdf_close_candidates([title, close, copy.deepcopy(close)], 'text_only.pdf'), [])
        self.assertTrue(native_pdf_present([title, close]))

    def test_presence_does_not_disappear_when_close_is_hidden_or_disabled(self):
        self.header[-1].set('enabled', 'false')
        self.header[-1].set('visible-to-user', 'false')
        self.assertEqual(web_preview_close_candidates(self.nodes(), 'ocr_test.png'), [])
        self.assertTrue(preview_controls_present(self.nodes()))
        self.header[1].set('text', 'unexpected filename')
        self.assertTrue(preview_controls_present(self.nodes()))

    def test_background_filenames_and_one_generic_close_are_not_a_preview(self):
        nodes = [self.background[0], *[node(name, '[10,400][500,450]', 'android.widget.TextView', 'false')
                                      for name in ['ocr_test.png', 'preview.txt', 'text_only.pdf', 'scanned.pdf']]]
        self.assertEqual(web_preview_close_candidates(nodes, 'ocr_test.png'), [])
        self.assertFalse(preview_controls_present(nodes))


class Lifecycle(unittest.TestCase):
    def journey(self, *, stale=False, native_first=False):
        fixture = Preview()
        fixture.setUp()
        header = fixture.header
        title = node('ocr_test.png', '[160,20][800,100]', 'android.widget.TextView', 'false', APP + ':id/pdfTitle')
        native_close = node('Close', '[900,20][1000,100]', 'android.widget.ImageButton', 'true', APP + ':id/pdfCloseButton')
        state = types.SimpleNamespace(now=0.0, layer='native-pdf' if native_first else 'web-preview',
                                      pending=None, left=0, taps=[], frames=0, captures=0, saved=[])
        class Journey:
            def remaining(self, deadline):
                if state.now >= deadline:
                    raise TimeoutError('Original synthetic close deadline exhausted')
                return deadline - state.now
            def snapshot(self, deadline, capture=False):
                self.remaining(deadline)
                state.now += .15
                state.frames += 1
                if state.pending is not None:
                    state.left -= 1
                    if state.left <= 0:
                        state.layer, state.pending = state.pending, None
                self.all = [title, native_close] if state.layer == 'native-pdf' else list(header.iter('node')) if state.layer == 'web-preview' else []
                self.nodes = self.all
                if capture:
                    state.captures += 1
                return dict(rootPackage=APP, allowedRoot=True, screenshot='public-fixture-frame.png')
            def focused(self, package):
                if package != APP:
                    raise AssertionError('Wrong owner')
            def unique(self, candidates, deadline):
                self.remaining(deadline)
                values = candidates()
                if len(values) != 1:
                    raise AssertionError('Unique scoped target required')
                return values[0]
        journey = Journey()
        def tap(target, remaining):
            state.taps.append(state.layer)
            if not stale:
                state.pending = 'web-preview' if state.layer == 'native-pdf' else 'absent'
                state.left = 2
            return dict(action='tap', singleFingerPair=True, toolType='finger')
        journey.observer = types.SimpleNamespace(tap=tap)
        return journey, state

    def test_stale_toolbar_cannot_complete_or_receive_repeated_close(self):
        journey, state = self.journey(stale=True)
        with self.assertRaises(TimeoutError):
            close_preview(journey, 'ocr_test.png', 3.0, lambda: True, state.saved.append, clock=lambda: state.now)
        self.assertEqual(state.taps, ['web-preview'])
        self.assertEqual(state.saved, [])
        self.assertEqual(state.captures, 0)

    def test_native_then_web_layer_both_close_before_completion(self):
        journey, state = self.journey(native_first=True)
        close_preview(journey, 'ocr_test.png', 5.0, lambda: True, state.saved.append, clock=lambda: state.now)
        self.assertEqual(state.taps, ['native-pdf', 'web-preview'])
        self.assertEqual(state.layer, 'absent')
        self.assertEqual(len(state.saved), 1)
        self.assertTrue(state.saved[0]['previewControlsAbsent'])
        self.assertGreaterEqual(state.saved[0]['stableCompletionObservations'], 3)
        self.assertGreaterEqual(state.saved[0]['stableCompletionElapsedMs'], 350)
        self.assertEqual(state.captures, 1)


if __name__ == '__main__':
    unittest.main()
