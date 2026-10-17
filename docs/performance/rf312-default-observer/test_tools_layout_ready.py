import copy
import unittest

from body_navigation import BACKUP_REMINDER, open_tool_card, tools_layout_ready
from test_body_navigation import node


def ready_nodes():
    group = node('Main navigation', '[28,1569][1052,1782]')
    for label, box in [('Home', '[42,1590][294,1761]'), ('Objects', '[288,1590][543,1761]'),
                       ('Tools', '[540,1590][792,1761]'), ('Settings', '[787,1590][1042,1761]')]:
        group.append(node(label, box, clickable='true',
                          description=label + (', current page' if label == 'Tools' else '')))
    return [*group.iter('node'),
            node('Organize, connect, explore', '[112,336][952,420]'),
            node('Your everyday tools, together in one place.', '[112,447][952,510]'),
            node('OCR', '[543,1197][1015,1462]', clickable='true', kind='android.widget.Button')]


class ToolsLayout(unittest.TestCase):
    def setUp(self):
        self.nodes = ready_nodes()

    def test_owned_tools_body_without_reminder_is_ready(self):
        self.assertTrue(tools_layout_ready(self.nodes))

    def test_selected_navigation_and_title_without_loaded_body_are_not_ready(self):
        nodes = [n for n in self.nodes if n.get('text') not in (
            'Organize, connect, explore', 'Your everyday tools, together in one place.')]
        nodes.append(node('Tools', '[49,115][181,181]'))
        self.assertFalse(tools_layout_ready(nodes))

    def test_visible_reminder_blocks_even_if_disabled_or_not_clickable(self):
        for enabled, clickable in [('true', 'true'), ('false', 'true'), ('true', 'false')]:
            with self.subTest(enabled=enabled, clickable=clickable):
                reminder = node(BACKUP_REMINDER + ' Back Up Now Close', '[42,252][1042,493]',
                                clickable=clickable, kind='android.widget.Button')
                reminder.set('enabled', enabled)
                self.assertFalse(tools_layout_ready([*self.nodes, reminder]))

    def test_hidden_or_foreign_reminder_does_not_block_owned_layout(self):
        reminder = node(BACKUP_REMINDER, '[42,252][1042,493]')
        reminder.set('visible-to-user', 'false')
        self.assertTrue(tools_layout_ready([*self.nodes, reminder]))
        reminder.set('visible-to-user', 'true')
        reminder.set('package', 'unrelated.app')
        self.assertTrue(tools_layout_ready([*self.nodes, reminder]))

    def test_missing_wrong_owned_hidden_or_duplicate_body_is_not_ready(self):
        for attribute, value in [('package', 'unrelated.app'), ('visible-to-user', 'false'),
                                 ('enabled', 'false'), ('bounds', 'invalid'), ('bounds', '[1,1570][90,1700]')]:
            nodes = copy.deepcopy(self.nodes)
            next(n for n in nodes if n.get('text') == 'Organize, connect, explore').set(attribute, value)
            with self.subTest(attribute=attribute, value=value):
                self.assertFalse(tools_layout_ready(nodes))
        duplicate = node('Organize, connect, explore', '[112,336][952,420]')
        self.assertFalse(tools_layout_ready([*self.nodes, duplicate]))

    def test_another_selected_page_is_not_tools_readiness(self):
        next(n for n in self.nodes if n.get('content-desc') == 'Tools, current page').set('content-desc', 'Tools')
        next(n for n in self.nodes if n.get('text') == 'Home').set('content-desc', 'Home, current page')
        self.assertFalse(tools_layout_ready(self.nodes))

    def test_original_deadline_refusal_precedes_scroll_or_tap(self):
        error = TimeoutError('original phase deadline')
        class Journey:
            def wait(self, predicate, deadline):
                raise error
            def unique(self, *args, **kwargs):
                raise AssertionError('must not admit input after wait refusal')
        with self.assertRaises(TimeoutError) as actual:
            open_tool_card(Journey(), 'OCR', 45)
        self.assertIs(actual.exception, error)

    def test_wait_uses_unfiltered_hierarchy_then_original_unique_and_one_tap(self):
        ready = self.nodes
        reminder = node(BACKUP_REMINDER + ' Back Up Now Close', '[42,252][1042,493]')
        reminder.set('enabled', 'false')
        events = []
        test = self
        class Observer:
            def tap(self, target, remaining):
                test.assertEqual(target.get('text'), 'OCR')
                test.assertEqual(remaining, 7)
                events.append('tap')
                return {'original': True}
        class Journey:
            all = [*ready, reminder]
            nodes = ready
            observer = Observer()
            def wait(self, predicate, deadline):
                test.assertEqual(deadline, 45)
                test.assertFalse(predicate())
                events.append('unready-snapshot')
                self.all = ready
                test.assertTrue(predicate())
                events.append('ready-snapshot')
            def unique(self, candidates, deadline, *, allow_scroll):
                test.assertTrue(allow_scroll)
                test.assertEqual(deadline, 45)
                events.append('original-unique')
                found = candidates()
                test.assertEqual(len(found), 1)
                return found[0]
            def remaining(self, deadline):
                return 7
        self.assertEqual(open_tool_card(Journey(), 'OCR', 45), {'original': True})
        self.assertEqual(events, ['unready-snapshot', 'ready-snapshot', 'original-unique', 'tap'])


if __name__ == '__main__':
    unittest.main()
