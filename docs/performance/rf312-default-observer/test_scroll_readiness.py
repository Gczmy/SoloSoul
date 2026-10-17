from types import SimpleNamespace
import unittest
import xml.etree.ElementTree as ET

from scroll_readiness import scroll_if_ready

APP = 'com.solosoul.app'


def node(bounds):
    return ET.Element('node', {'package': APP, 'bounds': bounds, 'scrollable': 'true'})


class Observer:
    def __init__(self, readiness):
        self.readiness = iter(readiness)
        self.calls = []
        self.receipt = {'singleFingerGesture': True}
        self.inspection_error = self.input_error = None

    def inspect(self, target, intent, timeout):
        self.calls.append(('inspect', target, intent, timeout))
        if self.inspection_error is not None:
            raise self.inspection_error
        return next(self.readiness)

    def target_ready(self, target, probe, intent):
        self.calls.append(('ready', target, probe, intent))
        return probe['ready']

    def scroll(self, target, direction, timeout):
        self.calls.append(('scroll', target, direction, timeout))
        if self.input_error is not None:
            raise self.input_error
        return self.receipt


class ScrollReadiness(unittest.TestCase):
    def journey(self, readiness, budgets=(5.5, 3.25)):
        observer = Observer(readiness)
        values = iter(budgets)
        seen = []

        def remaining(deadline):
            seen.append(deadline)
            value = next(values)
            if value <= 0:
                raise TimeoutError('Original phase expired')
            return value

        journey = SimpleNamespace(observer=observer, focused=lambda owner: self.assertEqual(owner, APP),
                                  remaining=remaining, scrolls=[])
        return journey, observer, seen

    def test_recorded_zero_match_is_deferred_without_gesture(self):
        probe = {'ready': False, 'matched': False, 'matchCount': 0, 'inputDispatched': False,
                 'candidateBounds': ['[0,231][1084,1572]']}
        journey, observer, _ = self.journey([probe])
        self.assertFalse(scroll_if_ready(journey, node('[0,511][1084,1572]'), 'forward', 12))
        self.assertEqual([call[0] for call in observer.calls], ['inspect', 'ready'])
        self.assertEqual(journey.scrolls, [])

    def test_next_observation_supplies_a_fresh_target_before_single_gesture(self):
        journey, observer, seen = self.journey([{'ready': False}, {'ready': True}], (8, 6, 5))
        old, fresh = node('[0,511][1084,1572]'), node('[0,231][1084,1572]')
        self.assertFalse(scroll_if_ready(journey, old, 'forward', 15))
        self.assertTrue(scroll_if_ready(journey, fresh, 'forward', 15))
        inputs = [call for call in observer.calls if call[0] == 'scroll']
        self.assertEqual(len(inputs), 1)
        self.assertIs(inputs[0][1], fresh)
        self.assertEqual(seen, [15, 15, 15])

    def test_repeated_read_only_not_ready_never_dispatches_or_records_scroll(self):
        journey, observer, _ = self.journey([{'ready': False}] * 3, (3, 2, 1))
        for _ in range(3):
            self.assertFalse(scroll_if_ready(journey, node('[0,231][1084,1572]'), 'forward', 10))
        self.assertEqual(len([c for c in observer.calls if c[0] == 'inspect']), 3)
        self.assertFalse(any(c[0] == 'scroll' for c in observer.calls))
        self.assertEqual(journey.scrolls, [])

    def test_focus_refusal_propagates_before_inspection(self):
        journey, observer, _ = self.journey([])
        error = RuntimeError('Owner focus differs')
        def refuse(_):
            raise error
        journey.focused = refuse
        with self.assertRaises(RuntimeError) as caught:
            scroll_if_ready(journey, node('[0,231][1084,1572]'), 'forward', 10)
        self.assertIs(caught.exception, error)
        self.assertEqual(observer.calls, [])

    def test_inspection_error_is_preserved_without_retry(self):
        journey, observer, _ = self.journey([])
        error = observer.inspection_error = RuntimeError('Protocol failed')
        with self.assertRaises(RuntimeError) as caught:
            scroll_if_ready(journey, node('[0,231][1084,1572]'), 'forward', 10)
        self.assertIs(caught.exception, error)
        self.assertEqual([c[0] for c in observer.calls], ['inspect'])

    def test_native_input_refusal_is_preserved_and_not_retried(self):
        journey, observer, _ = self.journey([{'ready': True}])
        error = observer.input_error = RuntimeError('Exact native target moved before input')
        with self.assertRaises(RuntimeError) as caught:
            scroll_if_ready(journey, node('[0,231][1084,1572]'), 'forward', 10)
        self.assertIs(caught.exception, error)
        self.assertEqual(len([c for c in observer.calls if c[0] == 'scroll']), 1)
        self.assertEqual(journey.scrolls, [])

    def test_expired_budget_prevents_inspection(self):
        journey, observer, _ = self.journey([], (0,))
        with self.assertRaises(TimeoutError):
            scroll_if_ready(journey, node('[0,231][1084,1572]'), 'forward', 10)
        self.assertEqual(observer.calls, [])

    def test_budget_expiring_after_readiness_prevents_input(self):
        journey, observer, _ = self.journey([{'ready': True}], (1, 0))
        with self.assertRaises(TimeoutError):
            scroll_if_ready(journey, node('[0,231][1084,1572]'), 'forward', 10)
        self.assertEqual([c[0] for c in observer.calls], ['inspect', 'ready'])
        self.assertEqual(journey.scrolls, [])

    def test_success_uses_original_remaining_budgets_and_receipt_once(self):
        journey, observer, seen = self.journey([{'ready': True}])
        target = node('[0,231][1084,1572]')
        self.assertTrue(scroll_if_ready(journey, target, 'forward', 20))
        self.assertEqual(observer.calls[0], ('inspect', target, 'scroll', 5.5))
        self.assertEqual(observer.calls[-1], ('scroll', target, 'forward', 3.25))
        self.assertEqual(seen, [20, 20])
        self.assertEqual(journey.scrolls, [observer.receipt])

    def test_backward_direction_is_passed_to_original_native_actor(self):
        journey, observer, _ = self.journey([{'ready': True}])
        self.assertTrue(scroll_if_ready(journey, node('[0,231][1084,1572]'), 'backward', 20))
        self.assertEqual(observer.calls[-1][2], 'backward')


if __name__ == '__main__':
    unittest.main()
