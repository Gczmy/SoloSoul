import copy
import unittest
import xml.etree.ElementTree as ET

from body_navigation import APP, main_navigation_top, open_tool_card, tool_card_candidates


def node(text, bounds, *, clickable='false', description='', kind='android.view.View'):
    return ET.Element('node', {'package': APP, 'class': kind, 'text': text,
                              'content-desc': description, 'bounds': bounds,
                              'clickable': clickable, 'enabled': 'true', 'visible-to-user': 'true'})


class Navigation(unittest.TestCase):
    def setUp(self):
        self.title = node('Objects', '[210,115][391,181]')
        self.group = node('Main navigation', '[28,1569][1052,1782]')
        for name, bounds in [('Home', '[42,1590][294,1761]'),
                             ('Objects', '[288,1590][543,1761]'),
                             ('Tools', '[540,1590][792,1761]'),
                             ('Settings', '[787,1590][1042,1761]')]:
            button = node(name if name == 'Objects' else '', bounds, clickable='true',
                          description=name + (', current page' if name == 'Objects' else ''))
            self.group.append(button)
            button.append(node(name, bounds, kind='android.widget.TextView'))

    def all_nodes(self):
        return [self.title, *self.group.iter('node')]

    def test_recorded_header_collision_uses_actual_navigation_top(self):
        old_top = min(int(n.get('bounds').split(',')[1].split(']')[0])
                      for n in self.all_nodes() if n.get('text') in ('Home', 'Objects', 'Tools', 'Settings'))
        self.assertEqual(old_top, 115)
        self.assertEqual(main_navigation_top(self.all_nodes()), 1569)
        public_record_center = (884 + 1134) / 2
        self.assertGreater(public_record_center, old_top)
        self.assertLess(public_record_center, main_navigation_top(self.all_nodes()))

    def test_unrelated_navigation_words_outside_group_do_not_change_boundary(self):
        for name in ['Home', 'Objects', 'Tools', 'Settings']:
            self.assertEqual(main_navigation_top([node(name, '[1,1][80,40]'), *self.all_nodes()]), 1569)

    def test_missing_navigation_has_no_guessed_boundary(self):
        self.assertIsNone(main_navigation_top([self.title]))

    def test_duplicate_named_groups_are_not_chosen_by_position(self):
        self.assertIsNone(main_navigation_top([*self.all_nodes(), copy.deepcopy(self.group)]))

    def test_duplicate_named_buttons_refuse(self):
        self.group.append(copy.deepcopy(self.group[0]))
        self.assertIsNone(main_navigation_top(self.all_nodes()))

    def test_missing_or_renamed_button_refuses(self):
        self.group.remove(self.group[-1])
        self.assertIsNone(main_navigation_top(self.all_nodes()))
        self.group.append(node('Overview', '[787,1590][1042,1761]', clickable='true'))
        self.assertIsNone(main_navigation_top(self.all_nodes()))

    def test_wrong_owner_hidden_disabled_or_wrong_group_class_refuses(self):
        for target in [self.group, self.group[0]]:
            for attribute, value in [('package', 'unrelated.app'), ('visible-to-user', 'false'),
                                     ('enabled', 'false')]:
                with self.subTest(attribute=attribute, target=target.get('text')):
                    original = target.get(attribute)
                    target.set(attribute, value)
                    self.assertIsNone(main_navigation_top(self.all_nodes()))
                    target.set(attribute, original)
        self.group.set('class', 'android.widget.TextView')
        self.assertIsNone(main_navigation_top(self.all_nodes()))

    def test_invalid_or_outside_button_geometry_refuses(self):
        for bounds in ['[42,1500][294,1761]', '[-1,1590][294,1761]',
                       '[42,1590][42,1761]', 'invalid']:
            with self.subTest(bounds=bounds):
                self.group[0].set('bounds', bounds)
                self.assertIsNone(main_navigation_top(self.all_nodes()))


class Tools(unittest.TestCase):
    def setUp(self):
        self.group = node('Main navigation', '[28,1569][1052,1782]')
        for name, bounds in [('Home', '[42,1590][294,1761]'),
                             ('Objects', '[288,1590][543,1761]'),
                             ('Tools', '[540,1590][792,1761]'),
                             ('Settings', '[787,1590][1042,1761]')]:
            self.group.append(node(name, bounds, clickable='true'))
        self.ocr = node('OCR', '[543,1477][1015,1743]', clickable='true', kind='android.widget.Button')
        self.search = node('Search', '[49,630][525,895]', clickable='true', kind='android.widget.Button')

    def nodes(self):
        return [self.ocr, self.search, *self.group.iter('node')]

    def test_recorded_ocr_center_under_navigation_is_not_selected(self):
        self.assertEqual(tool_card_candidates(self.nodes(), 'OCR'), [])
        self.assertEqual(tool_card_candidates(self.nodes(), 'Search'), [self.search])

    def test_scrolled_card_above_navigation_is_selected(self):
        self.ocr.set('bounds', '[543,1197][1015,1462]')
        self.assertEqual(tool_card_candidates(self.nodes(), 'OCR'), [self.ocr])

    def test_center_exactly_on_navigation_boundary_is_not_selected(self):
        self.ocr.set('bounds', '[543,1469][1015,1669]')
        self.assertEqual(tool_card_candidates(self.nodes(), 'OCR'), [])

    def test_missing_or_duplicate_navigation_has_no_guessed_scope(self):
        self.ocr.set('bounds', '[543,1197][1015,1462]')
        self.assertEqual(tool_card_candidates([self.ocr], 'OCR'), [])
        self.assertEqual(tool_card_candidates([*self.nodes(), copy.deepcopy(self.group)], 'OCR'), [])

    def test_header_text_and_description_only_are_not_tool_buttons(self):
        header = node('OCR', '[50,84][180,190]', clickable='true')
        description = node('Other', '[50,630][300,800]', description='OCR', clickable='true', kind='android.widget.Button')
        self.assertEqual(tool_card_candidates([header, description, *self.nodes()], 'OCR'), [])

    def test_wrong_owner_state_role_or_invalid_geometry_refuses(self):
        self.ocr.set('bounds', '[543,1197][1015,1462]')
        for key, value in [('package', 'other.app'), ('enabled', 'false'), ('visible-to-user', 'false'),
                           ('clickable', 'false'), ('class', 'android.widget.TextView'), ('bounds', 'invalid')]:
            with self.subTest(key=key):
                original = self.ocr.get(key)
                self.ocr.set(key, value)
                self.assertEqual(tool_card_candidates(self.nodes(), 'OCR'), [])
                self.ocr.set(key, original)

    def test_duplicate_visible_cards_are_not_arbitrarily_chosen(self):
        self.ocr.set('bounds', '[543,1197][1015,1462]')
        other = copy.deepcopy(self.ocr)
        self.assertEqual(tool_card_candidates([*self.nodes(), other], 'OCR'), [self.ocr, other])

    def test_unknown_tool_refuses_before_driver_is_used(self):
        with self.assertRaises(ValueError):
            open_tool_card(object(), 'Delete', 45)

    def test_original_readiness_can_scroll_before_one_exact_tap(self):
        test = self
        events = []
        class Observer:
            def tap(self, target, remaining):
                test.assertEqual(target, test.ocr)
                test.assertEqual(remaining, 7)
                events.append('tap')
                return {'originalReceipt': True}
        class Journey:
            nodes = test.nodes()
            observer = Observer()
            def unique(self, candidates, deadline, *, allow_scroll):
                test.assertEqual(candidates(), [])
                test.assertEqual(deadline, 45)
                test.assertTrue(allow_scroll)
                events.append('original-native-scroll-and-readiness')
                test.ocr.set('bounds', '[543,1197][1015,1462]')
                found = candidates()
                test.assertEqual(found, [test.ocr])
                return found[0]
            def remaining(self, deadline):
                test.assertEqual(deadline, 45)
                return 7
        self.assertEqual(open_tool_card(Journey(), 'OCR', 45), {'originalReceipt': True})
        self.assertEqual(events, ['original-native-scroll-and-readiness', 'tap'])

    def test_native_refusal_propagates_without_retry(self):
        self.ocr.set('bounds', '[543,1197][1015,1462]')
        failure = RuntimeError('exact native target changed; input not dispatched')
        taps = []
        class Observer:
            def tap(self, target, remaining):
                taps.append(target)
                raise failure
        class Journey:
            nodes = self.nodes()
            observer = Observer()
            def unique(self, candidates, deadline, *, allow_scroll):
                return candidates()[0]
            def remaining(self, deadline):
                return 7
        with self.assertRaises(RuntimeError) as caught:
            open_tool_card(Journey(), 'OCR', 45)
        self.assertIs(caught.exception, failure)
        self.assertEqual(taps, [self.ocr])


if __name__ == '__main__':
    unittest.main()
