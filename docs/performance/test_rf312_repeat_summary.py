"""Failure-retention and provenance regressions for offline RF-312 statistics."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import rf312_repeat_summary as summary


def example():
    binding = {key: "a" * 64 for key in summary.BINDING_FIELDS}
    tag = summary.digest(binding)
    data = dict(schemaVersion=1, plannedAttempts=5, objectCounts=[100, 5000],
                phaseNames=list(summary.PHASES), metric="hostObservedUpperBoundMs",
                binding=binding, bindingSha256=tag, attempts=[])
    for number in range(1, 6):
        cases = []
        for count in [100, 5000]:
            cases.append(dict(objectCount=count, accepted=True,
                              completedPhases=[dict(name=name, hostObservedUpperBoundMs=number * 100)
                                               for name in summary.PHASES],
                              postJourneyPssBytes=number * 1024, failureEvidenceSha256=None))
        data["attempts"].append(dict(attempt=number, accepted=True, bindingSha256=tag,
                                     evidenceSha256={"terminal": "b" * 64}, cases=cases))
    return data


class RepeatSummaryTests(unittest.TestCase):
    def test_five_values_median_and_empirical_tail_keep_order(self):
        data = example()
        data["attempts"][0]["cases"][0]["completedPhases"][0]["hostObservedUpperBoundMs"] = 999
        result = summary.summarize(data)
        values = result["cases"][0]["phases"][0]
        self.assertEqual(values["valuesInAttemptOrder"], [999, 200, 300, 400, 500])
        self.assertEqual((values["median"], values["nearestRankEmpiricalP95"], values["maximum"]), (400, 999, 999))
        self.assertTrue(result["p95IsSmallSampleEmpiricalNotPopulationTail"])

    def test_failed_partial_attempt_is_retained_without_replacement(self):
        data = example()
        attempt = data["attempts"][1]
        attempt["accepted"] = False
        case = attempt["cases"][1]
        case.update(accepted=False, completedPhases=case["completedPhases"][:3],
                    postJourneyPssBytes=None, failureEvidenceSha256="c" * 64)
        result = summary.summarize(data)
        self.assertEqual((result["acceptedAttempts"], result["failedAttempts"]), (4, 1))
        self.assertEqual(result["cases"][1]["phases"][0]["valuesInAttemptOrder"], [100, None, 300, 400, 500])
        self.assertEqual(result["cases"][1]["phases"][0]["median"], 350)
        self.assertEqual(result["cases"][1]["phases"][0]["nearestRankEmpiricalP95"], 500)
        self.assertEqual(result["cases"][0]["postJourneyPssBytes"]["acceptedSamples"], 4)
        self.assertEqual(result["allAttemptOutcomes"][1], attempt)
        self.assertFalse(result["fixedHostObservedSeriesCollected"])

    def test_no_success_is_not_zero_latency(self):
        data = example()
        for attempt in data["attempts"]:
            attempt["accepted"] = False
        result = summary.summarize(data)
        self.assertIsNone(result["cases"][0]["phases"][0]["median"])
        self.assertEqual(result["cases"][0]["phases"][0]["failedAttempts"], 5)

    def test_reject_missing_duplicate_or_extra_attempt(self):
        for edit in [lambda d: d["attempts"].pop(),
                     lambda d: d["attempts"].append(copy.deepcopy(d["attempts"][0])),
                     lambda d: d["attempts"][2].update(attempt=2)]:
            with self.subTest(edit=edit):
                data = example(); edit(data)
                with self.assertRaises(ValueError): summary.summarize(data)

    def test_reject_source_drift_and_forged_binding(self):
        for mutate in [lambda d: d["attempts"][3].update(bindingSha256="e" * 64),
                       lambda d: d["binding"].update(apkSha256="f" * 64),
                       lambda d: d["binding"].pop("fixturesSha256")]:
            with self.subTest(mutate=mutate):
                data = example(); mutate(data)
                with self.assertRaises(ValueError): summary.summarize(data)

    def test_reject_stage_gap_and_incomplete_success(self):
        for mutate in [lambda c: c["completedPhases"].pop(4),
                       lambda c: c.update(postJourneyPssBytes=None),
                       lambda c: c["completedPhases"].reverse()]:
            with self.subTest(mutate=mutate):
                data = example(); mutate(data["attempts"][0]["cases"][0])
                with self.assertRaises(ValueError): summary.summarize(data)

    def test_reject_nonfinite_boolean_and_out_of_budget_times(self):
        for value in [float("nan"), float("inf"), True, -1, 0, 45001]:
            with self.subTest(value=value):
                data = example()
                data["attempts"][0]["cases"][0]["completedPhases"][0]["hostObservedUpperBoundMs"] = value
                with self.assertRaises(ValueError): summary.summarize(data)

    def test_reject_dropped_failure_evidence(self):
        data = example(); data["attempts"][0]["accepted"] = False
        data["attempts"][0]["cases"][0]["accepted"] = False
        with self.assertRaises(ValueError): summary.summarize(data)

    def test_reject_boolean_memory_and_false_success(self):
        data = example(); data["attempts"][0]["cases"][0]["postJourneyPssBytes"] = True
        with self.assertRaises(ValueError): summary.summarize(data)
        data = example()
        data["attempts"][0]["cases"][0].update(accepted=False, failureEvidenceSha256="d" * 64)
        with self.assertRaises(ValueError): summary.summarize(data)

    def test_no_scope_promotion_or_input_mutation(self):
        data = example(); original = copy.deepcopy(data)
        result = summary.summarize(data)
        self.assertEqual(data, original)
        self.assertTrue(result["fixedHostObservedSeriesCollected"])
        for key in ["performanceDistributionAccepted", "applicationOnlyLatencyAttributed",
                    "wholeJourneyOrPeakMemoryAccepted", "actualIpcAccepted",
                    "sameSourceObserverOverheadMeasured", "twoCommittedFramesProven", "fullRF312Accepted"]:
            self.assertIs(result[key], False, key)

    def test_reject_boolean_schema_and_attempt_identity(self):
        for mutate in [lambda d: d.update(schemaVersion=True),
                       lambda d: d["attempts"][0].update(attempt=True),
                       lambda d: d["attempts"][0]["cases"][0].update(objectCount=100.0)]:
            with self.subTest(mutate=mutate):
                data = example(); mutate(data)
                with self.assertRaises(ValueError): summary.summarize(data)

    def test_reject_malformed_rows_with_validation_errors(self):
        for mutate in [lambda d: d["attempts"].__setitem__(0, None),
                       lambda d: d["attempts"][0]["cases"].__setitem__(0, []),
                       lambda d: d["attempts"][0]["cases"][0]["completedPhases"].__setitem__(0, None)]:
            with self.subTest(mutate=mutate):
                data = example(); mutate(data)
                with self.assertRaises(ValueError): summary.summarize(data)

    def test_cli_retains_input_hash_uses_portable_lf_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            source = folder / 'input.json'
            target = folder / 'summary.json'
            raw = (json.dumps(example(), ensure_ascii=False) + '\n').encode('utf-8')
            source.write_bytes(raw)
            command = [sys.executable, '-B', str(Path(summary.__file__).resolve()),
                       '--input', str(source), '--output', str(target)]
            options = dict(stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
            first = subprocess.run(command, **options)
            self.assertEqual(first.returncode, 0, first.stderr.decode('utf-8'))
            original = target.read_bytes()
            self.assertNotIn(b'\r', original, 'Git-archived bytes must retain the recorded file hash')
            self.assertEqual(json.loads(original)['inputFileSha256'], hashlib.sha256(raw).hexdigest())
            second = subprocess.run(command, **options)
            self.assertNotEqual(second.returncode, 0)
            self.assertEqual(target.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
