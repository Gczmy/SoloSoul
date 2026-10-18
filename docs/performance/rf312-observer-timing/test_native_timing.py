import json
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

import native_timing as m

spec = importlib.util.spec_from_file_location('native_timing_entry', Path(__file__).with_name('read-native-timing.py'))
entry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(entry)


class NativeControlTiming(unittest.TestCase):
    def fixture(self, base=0):
        temporary = tempfile.TemporaryDirectory(prefix='rf312 native timing ')
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        observer = root/'observer'
        observer.mkdir()
        for ordinal, (action, image, start, end) in enumerate((
            ('snapshot', False, 100, 120), ('snapshot', True, 130, 180),
            ('tap', False, 190, 200), ('snapshot', False, 210, 260),
            ('snapshot', True, 270, 280)), 1):
            request = dict(action=action)
            response = dict(action=action, success=True, nativeStartedAtNs=base+start,
                            nativeFinishedAtNs=base+end, rootPackage='com.solosoul.app')
            if action == 'snapshot':
                request['screenshot'] = image
                response['xml'] = '<hierarchy/>'
            if image:
                response['screenshot'] = f'frame-{ordinal:03d}.png'
            for prefix, value in (('request', request), ('response', response)):
                (observer/f'{prefix}-{ordinal:03d}.json').write_text(json.dumps(value), encoding='utf-8')
        phases = root/'phases.json'
        phases.write_text(json.dumps([dict(name='one', nativeScreenshot='frame-002.png'),
                                      dict(name='two', nativeScreenshot='frame-005.png')]), encoding='utf-8')
        return observer, phases

    def collect(self, observer, phases, frozen=None):
        return m.collect(observer, phases, ('one', 'two'),
                         m.fingerprint(observer, phases) if frozen is None else frozen)

    def mutate(self, path, **changes):
        value=json.loads(path.read_bytes());value.update(changes)
        path.write_text(json.dumps(value), encoding='utf-8')

    def test_exact_intervals_and_negative_capture_pair_are_retained(self):
        observer, phases = self.fixture()
        result = self.collect(observer, phases)
        self.assertEqual(result['requests'], 5)
        self.assertEqual(result['nativeOperationDurationSumNs'], 140)
        self.assertEqual(result['nativeChannelSpanNs'], 180)
        self.assertEqual([p['incrementNs'] for p in result['consecutiveMatchingXmlPairs']], [30, -40])
        self.assertEqual([p['responseOrdinal'] for p in result['phaseScreenshotObservationWindows']], [2, 5])
        for name in ('exclusiveObserverOverheadAccepted', 'applicationOnlyLatencyAccepted',
                     'firstInteractiveTimeProven', 'nativeFramesAccepted', 'performanceDistributionAccepted',
                     'fullRF312Accepted', 'canonicalUiValidationPerformedByThisLibrary'):
            self.assertIs(result[name], False)

    def test_epoch_above_javascript_integer_precision_stays_exact(self):
        observer, phases = self.fixture(base=2**53+1)
        result = self.collect(observer, phases)
        self.assertEqual(result['rows'][0]['nativeStartedAtNs'], 2**53+101)
        self.assertEqual(result['nativeOperationDurationSumNs'], 140)

    def test_exact_half_nanosecond_median_and_nearest_rank(self):
        result = m.statistics(list(range(1, 21)))
        self.assertEqual((result['medianNumeratorNs'], result['medianDenominator']), (21, 2))
        self.assertEqual(result['nearestRankP95Ns'], 19)

    def test_canonical_phase_records_accept_only_equivalent_typed_json(self):
        phases = [dict(name='one', nativeScreenshot='frame-002.png', hostObservedUpperBoundMs=1)]
        raw = json.dumps(dict(stages=phases)).encode()
        self.assertEqual(m.bind_phase_records(json.dumps(phases, indent=2).encode(), raw), 1)

    def test_valid_screenshot_retarget_cannot_rebind_a_canonical_phase(self):
        observer, phases = self.fixture()
        canonical = json.loads(phases.read_bytes())
        self.mutate(observer/'request-003.json', action='snapshot', screenshot=True)
        self.mutate(observer/'response-003.json', action='snapshot', screenshot='frame-003.png', xml='<hierarchy/>')
        retargeted = [dict(name='one', nativeScreenshot='frame-003.png'), canonical[1]]
        phases.write_text(json.dumps(retargeted), encoding='utf-8')
        # The previous timing reader accepts this real, ordered response, but it
        # belongs to a different image than the canonical state/UI evidence.
        self.assertEqual([r['responseOrdinal'] for r in self.collect(observer, phases)['phaseScreenshotObservationWindows']], [3, 5])
        with self.assertRaisesRegex(ValueError, 'canonically verified stages'):
            m.bind_phase_records(phases.read_bytes(), json.dumps(dict(stages=canonical)).encode())

    def test_phase_number_boolean_and_float_substitution_refused(self):
        canonical = [dict(name='one', hostObservedUpperBoundMs=1)]
        for number in (True, 1.0):
            with self.subTest(number=number), self.assertRaises(ValueError):
                m.bind_phase_records(json.dumps([dict(name='one', hostObservedUpperBoundMs=number)]).encode(),
                                     json.dumps(dict(stages=canonical)).encode())

    def test_changed_bytes_after_verification_refused_without_overwrite(self):
        observer, phases = self.fixture()
        frozen = m.fingerprint(observer, phases)
        path = observer/'response-002.json'
        original = path.read_bytes();path.write_bytes(original+b' ')
        with self.assertRaises(ValueError):
            self.collect(observer, phases, frozen)
        self.assertEqual(path.read_bytes(), original+b' ')

    def test_clock_boolean_float_zero_reverse_and_overflow_refused(self):
        for value in (True, 1.0, 0, -1, 2**63):
            with self.subTest(value=value):
                observer, phases = self.fixture()
                self.mutate(observer/'response-001.json', nativeStartedAtNs=value)
                with self.assertRaises(ValueError):self.collect(observer, phases)
        observer, phases = self.fixture()
        self.mutate(observer/'response-001.json', nativeFinishedAtNs=99)
        with self.assertRaises(ValueError):self.collect(observer, phases)

    def test_cross_request_overlap_refused(self):
        observer, phases = self.fixture()
        self.mutate(observer/'response-002.json', nativeStartedAtNs=119)
        with self.assertRaises(ValueError):self.collect(observer, phases)

    def test_unsuccessful_original_receipt_not_counted_as_valid_sample(self):
        observer, phases = self.fixture()
        self.mutate(observer/'response-002.json', success=False)
        before=(observer/'response-002.json').read_bytes()
        with self.assertRaises(ValueError):self.collect(observer, phases)
        self.assertEqual((observer/'response-002.json').read_bytes(), before)

    def test_action_mismatch_and_unknown_action_refused(self):
        observer, phases = self.fixture()
        self.mutate(observer/'response-003.json', action='back')
        with self.assertRaises(ValueError):self.collect(observer, phases)
        self.mutate(observer/'request-003.json', action='unknown')
        self.mutate(observer/'response-003.json', action='unknown')
        with self.assertRaises(ValueError):self.collect(observer, phases)

    def test_protocol_gap_missing_reply_and_request_count_bound(self):
        observer, phases = self.fixture()
        (observer/'response-003.json').unlink()
        with self.assertRaises(ValueError):self.collect(observer, phases)
        observer, phases = self.fixture()
        with patch.object(m, 'MAX_REQUESTS', 4), self.assertRaises(ValueError):
            self.collect(observer, phases)

    def test_duplicate_nonfinite_and_oversized_json_refused(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}'):
            with self.assertRaises(ValueError):m.decode(raw)
        observer, phases = self.fixture()
        with patch.object(m, 'MAX_BYTES', 3), self.assertRaises(ValueError):
            m.raw_file(phases)

    def test_screenshot_receipt_identity_and_boolean_refused(self):
        observer, phases = self.fixture()
        self.mutate(observer/'response-002.json', screenshot='../other.png')
        with self.assertRaises(ValueError):self.collect(observer, phases)
        observer, phases = self.fixture()
        self.mutate(observer/'request-002.json', screenshot='true')
        with self.assertRaises(ValueError):self.collect(observer, phases)
        observer, phases = self.fixture()
        value=json.loads((observer/'response-002.json').read_bytes());value.pop('screenshot')
        (observer/'response-002.json').write_text(json.dumps(value), encoding='utf-8')
        with self.assertRaises(ValueError):self.collect(observer, phases)

    def test_phase_order_and_reused_observation_refused(self):
        observer, phases = self.fixture()
        phases.write_text(json.dumps([dict(name='two'),dict(name='one')]), encoding='utf-8')
        with self.assertRaises(ValueError):self.collect(observer, phases)
        observer, phases = self.fixture()
        phases.write_text(json.dumps([dict(name='one',nativeScreenshot='frame-002.png'),
                                      dict(name='two',nativeScreenshot='frame-002.png')]), encoding='utf-8')
        with self.assertRaises(ValueError):self.collect(observer, phases)

    def test_xml_mismatch_preserved_but_not_declared_a_matching_pair(self):
        observer, phases = self.fixture()
        self.mutate(observer/'response-002.json', xml='<different/>')
        self.assertEqual(len(self.collect(observer, phases)['consecutiveMatchingXmlPairs']), 1)

    def test_canonical_state_and_ime_precede_ui(self):
        observer, _ = self.fixture()
        output = observer.parent/'output'
        output.mkdir()
        context_dir = output/'context'
        context_dir.mkdir()
        names = ('state', 'ime', 'ui')
        receipts = ('independent-current-state-r1.json', 'independent-ime-readiness-r1.json', 'independent-ui-r1.json')
        context = dict(queueSourceSha256={f'replay_{name}.py': entry.sha(entry.QUEUE/f'replay_{name}.py') for name in names})
        called = []
        def run(command, **kwargs):
            name = Path(command[2]).stem.removeprefix('replay_')
            self.assertEqual(name, names[len(called)])
            called.append(name)
            (context_dir/receipts[names.index(name)]).write_text('{"passed":true}', encoding='utf-8')
            self.assertIs(kwargs['stdin'], entry.subprocess.DEVNULL)
            return SimpleNamespace(returncode=0)
        with patch.object(entry.subprocess, 'run', side_effect=run):
            self.assertEqual(list(entry.replay_readers(context_dir/'context.json', output, context)), list(names))
        self.assertEqual(called, list(names))

    def test_refused_state_prevents_ime_ui_and_timing(self):
        observer, _ = self.fixture()
        output = observer.parent/'output'
        output.mkdir()
        context = dict(queueSourceSha256={'replay_state.py': entry.sha(entry.QUEUE/'replay_state.py')})
        with patch.object(entry.subprocess, 'run', return_value=SimpleNamespace(returncode=1)) as run:
            with self.assertRaisesRegex(ValueError, 'state replay refused'):
                entry.replay_readers(output/'context.json', output, context)
        self.assertEqual(run.call_count, 1)
        self.assertFalse((output/'canonical-ui-command.json').exists())
        self.assertEqual(list(output.glob('timing-*.json')), [])


if __name__ == '__main__':
    unittest.main()
