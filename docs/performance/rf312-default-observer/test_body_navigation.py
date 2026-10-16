import copy
import unittest
import xml.etree.ElementTree as ET

from body_navigation import APP, main_navigation_top


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


if __name__ == '__main__':
    unittest.main()
