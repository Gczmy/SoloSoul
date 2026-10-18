"""Failure retention, pairing and provenance tests for offline cohort statistics."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import rf312_sampling_cohort as cohort


def example():
    binding = {key: "a" * 64 for key in cohort.BINDING_FIELDS}
    tag = cohort.digest(binding)
    data = dict(schemaVersion=1, plannedPairs=6, modeOrderPerPair=copy.deepcopy(cohort.MODE_ORDER),
                objectCounts=[100, 5000], phaseNames=list(cohort.PHASES),
                metric="hostObservedUpperBoundMs", originalBudgets=dict(cohort.BUDGETS),
                binding=binding, bindingSha256=tag, pairs=[])
    for ordinal, order in enumerate(cohort.MODE_ORDER, 1):
        modes = []
        for mode in order:
            cases = []
            for count in (100, 5000):
                trace = None
                if mode == "on":
                    trace = dict(points=20, acceptedMemoryPoints=18, refusedPoints=1,
                                 missedPeriodicSlots=1, plannedPeriodicSlots=7, boundaryPoints=12,
                                 refusedPeriodicCaptures=[], allPeriodicCapturesSucceeded=True,
                                 workerStopped=True, allScheduledPeriodicSlotsAccounted=True,
                                 sampledMaximumSerialPssBytes=400000,
                                 evidenceSha256=dict(trace="b" * 64, independentMemoryReplay="c" * 64))
                cases.append(dict(objectCount=count, status="accepted",
                                  completedPhases=[dict(name=name, hostObservedUpperBoundMs=ordinal * 100)
                                                   for name in cohort.PHASES],
                                  workloadWindowUpperBoundMs=100000 + ordinal * 1000 + (200 if mode == "on" else 0),
                                  postJourneyPssBytes=200000 + ordinal * 1024 - (1024 if mode == "on" else 0),
                                  journeyMemory=trace,
                                  evidenceSha256=dict(driver="d" * 64, supportedState="e" * 64,
                                                      checkpoint="f" * 64)))
            modes.append(dict(mode=mode, status="accepted", bindingSha256=tag,
                              failureCategory=None, evidenceSha256=dict(nativeResult="b" * 64,
                                                                       replayResult="c" * 64), cases=cases))
        data["pairs"].append(dict(pair=ordinal, modes=modes))
    return data


def mode_at(data, pair, mode):
    return next(row for row in data["pairs"][pair - 1]["modes"] if row["mode"] == mode)


def empty_case(case, status):
    case.update(status=status, completedPhases=[], workloadWindowUpperBoundMs=None,
                postJourneyPssBytes=None, journeyMemory=None, evidenceSha256={})


def refuse_before_install(data, pair, mode):
    row = mode_at(data, pair, mode)
    row.update(status="refused", failureCategory="preflight-refused",
               evidenceSha256=dict(nativeResult="d" * 64, failure="e" * 64))
    for case in row["cases"]:
        empty_case(case, "not-reached")


class SamplingCohortTests(unittest.TestCase):
    def test_six_pairs_preserve_order_median_tail_and_signed_paired_difference(self):
        data = example()
        mode_at(data, 1, "off")["cases"][0]["completedPhases"][0]["hostObservedUpperBoundMs"] = 999
        result = cohort.summarize(data)
        phase = result["cells"][0]["phases"][0]
        self.assertEqual(phase["valuesInPairOrder"], [999, 200, 300, 400, 500, 600])
        self.assertEqual((phase["median"], phase["nearestRankEmpiricalP95"], phase["maximum"]), (450, 999, 999))
        self.assertEqual(result["descriptivePairedOnMinusOff"][0]["workloadWindowUpperBoundMs"]["median"], 200)
        self.assertEqual(result["descriptivePairedOnMinusOff"][0]["postJourneyPssBytes"]["median"], -1024)
        self.assertEqual(result["completeAcceptedPairs"], 6)
        self.assertTrue(result["allPlannedModeOutcomesTerminal"])

    def test_preflight_failure_retains_both_unexecuted_sizes_and_excludes_pair(self):
        data = example()
        refuse_before_install(data, 2, "on")
        result = cohort.summarize(data)
        cell = result["cells"][2]
        self.assertEqual(cell["caseOutcomeCounts"]["not-reached"], 1)
        self.assertEqual(cell["workloadWindowUpperBoundMs"]["acceptedSamples"], 5)
        self.assertIsNone(cell["workloadWindowUpperBoundMs"]["valuesInPairOrder"][1])
        self.assertEqual(result["cells"][0]["postJourneyPssBytes"]["acceptedSamples"], 6)
        self.assertIsNone(result["descriptivePairedOnMinusOff"][0]["postJourneyPssBytes"]["valuesInPairOrder"][1])
        self.assertEqual(result["modeOutcomeCounts"]["refused"], 1)
        self.assertEqual(result["allPairOutcomes"][1], data["pairs"][1])

    def test_failed_later_size_keeps_completed_prefix_but_does_not_pool_it(self):
        data = example()
        row = mode_at(data, 1, "on")
        row.update(status="refused", failureCategory="native-case-refused")
        row["evidenceSha256"]["failure"] = "e" * 64
        row["cases"][0]["status"] = "completed-before-refusal"
        case = row["cases"][1]
        case.update(status="refused", completedPhases=case["completedPhases"][:3],
                    postJourneyPssBytes=None, journeyMemory=None)
        result = cohort.summarize(data)
        self.assertEqual(result["cells"][2]["workloadWindowUpperBoundMs"]["acceptedSamples"], 5)
        self.assertEqual(len(result["allPairOutcomes"][0]["modes"][1]["cases"][0]["completedPhases"]), 12)
        self.assertEqual(result["cells"][3]["caseOutcomeCounts"]["refused"], 1)

    def test_restore_or_replay_failure_after_both_sizes_cannot_supply_successes(self):
        data = example()
        row = mode_at(data, 1, "off")
        row.update(status="refused", failureCategory="replay-refused")
        row["evidenceSha256"]["failure"] = "e" * 64
        for case in row["cases"]:
            case["status"] = "completed-before-refusal"
        result = cohort.summarize(data)
        self.assertEqual(result["cells"][0]["postJourneyPssBytes"]["acceptedSamples"], 5)
        self.assertEqual(result["cells"][1]["caseOutcomeCounts"]["completed-before-refusal"], 1)

    def test_pending_is_neither_failed_nor_zero_measurement_nor_liveness_claim(self):
        data = example()
        for pair in data["pairs"]:
            for row in pair["modes"]:
                row.update(status=cohort.PENDING, evidenceSha256={})
                for case in row["cases"]:
                    empty_case(case, cohort.PENDING)
        result = cohort.summarize(data)
        self.assertEqual(result["modeOutcomeCounts"], {"accepted": 0, "refused": 0, cohort.PENDING: 12})
        self.assertIsNone(result["cells"][0]["workloadWindowUpperBoundMs"]["median"])
        self.assertFalse(result["allPlannedModeOutcomesTerminal"])
        self.assertTrue(result["missingTerminalEvidenceDoesNotDetermineProcessLiveness"])

    def test_missing_slots_are_distinct_from_actual_refusals(self):
        result = cohort.summarize(example())
        trace = result["cells"][2]["trajectoryCoverageInPairOrder"][0]
        self.assertEqual(trace["missedPeriodicSlots"], 1)
        self.assertTrue(trace["allPeriodicCapturesSucceeded"])
        data = example()
        trace = mode_at(data, 1, "on")["cases"][0]["journeyMemory"]
        trace.update(points=21, refusedPoints=2, plannedPeriodicSlots=8,
                     allPeriodicCapturesSucceeded=False,
                     refusedPeriodicCaptures=[dict(kind="periodic", pointOrdinal=9,
                                                   numericSnapshotAccepted=False, errorType="TimeoutExpired",
                                                   receiptSha256="e" * 64)])
        result = cohort.summarize(data)
        kept = result["cells"][2]["trajectoryCoverageInPairOrder"][0]
        self.assertEqual((kept["missedPeriodicSlots"], len(kept["refusedPeriodicCaptures"])), (1, 1))
        self.assertFalse(kept["allPeriodicCapturesSucceeded"])

    def test_reject_missing_pairs_duplicate_identity_and_wrong_mode_or_size_order(self):
        edits = [lambda d: d["pairs"].pop(),
                 lambda d: d["pairs"][1].update(pair=1),
                 lambda d: d["pairs"][0]["modes"].reverse(),
                 lambda d: mode_at(d, 1, "off")["cases"].reverse(),
                 lambda d: d.update(plannedPairs=5)]
        for edit in edits:
            with self.subTest(edit=edit):
                data = example(); edit(data)
                with self.assertRaises(ValueError): cohort.summarize(data)

    def test_reject_source_artifact_and_protocol_mixing(self):
        edits = [lambda d: d["binding"].update(apkSha256="f" * 64),
                 lambda d: mode_at(d, 2, "on").update(bindingSha256="f" * 64),
                 lambda d: d["binding"].pop("queueSourceSha256"),
                 lambda d: d["originalBudgets"].update(phaseSeconds=46),
                 lambda d: d.update(metric="applicationOnlyLatencyMs")]
        for edit in edits:
            with self.subTest(edit=edit):
                data = example(); edit(data)
                with self.assertRaises(ValueError): cohort.summarize(data)

    def test_reject_nonfinite_boolean_nonpositive_or_out_of_budget_phase(self):
        for value in (float("nan"), float("inf"), True, 0, -1, 45001):
            with self.subTest(value=value):
                data = example()
                mode_at(data, 1, "off")["cases"][0]["completedPhases"][0]["hostObservedUpperBoundMs"] = value
                with self.assertRaises(ValueError): cohort.summarize(data)

    def test_reject_stage_gaps_false_successes_and_unexecuted_values(self):
        edits = [lambda c: c["completedPhases"].pop(4),
                 lambda c: c.update(postJourneyPssBytes=None),
                 lambda c: c.update(postJourneyPssBytes=True),
                 lambda c: c.update(status="not-reached"),
                 lambda c: c["evidenceSha256"].pop("checkpoint")]
        for edit in edits:
            with self.subTest(edit=edit):
                data = example(); edit(mode_at(data, 1, "off")["cases"][0])
                with self.assertRaises(ValueError): cohort.summarize(data)

    def test_reject_dropped_failure_evidence_and_resumed_size_after_failure(self):
        data = example(); refuse_before_install(data, 1, "on")
        mode_at(data, 1, "on")["evidenceSha256"].pop("failure")
        with self.assertRaises(ValueError): cohort.summarize(data)
        data = example()
        row = mode_at(data, 1, "off")
        row.update(status="refused", failureCategory="native-case-refused")
        row["evidenceSha256"]["failure"] = "e" * 64
        row["cases"][0]["status"] = "refused"
        row["cases"][1]["status"] = "completed-before-refusal"
        with self.assertRaises(ValueError): cohort.summarize(data)

    def test_reject_sampling_mode_substitution_and_unaccounted_gaps(self):
        edits = [lambda c: c.update(journeyMemory=None),
                 lambda c: c["journeyMemory"].update(missedPeriodicSlots=0),
                 lambda c: c["journeyMemory"].update(workerStopped=False),
                 lambda c: c["journeyMemory"].update(allPeriodicCapturesSucceeded=False),
                 lambda c: c["journeyMemory"].update(boundaryPoints=11),
                 lambda c: c["journeyMemory"].update(points=193, acceptedMemoryPoints=191,
                                                   plannedPeriodicSlots=180)]
        for edit in edits:
            with self.subTest(edit=edit):
                data = example(); edit(mode_at(data, 1, "on")["cases"][0])
                with self.assertRaises(ValueError): cohort.summarize(data)
        data = example()
        mode_at(data, 1, "off")["cases"][0]["journeyMemory"] = mode_at(data, 1, "on")["cases"][0]["journeyMemory"]
        with self.assertRaises(ValueError): cohort.summarize(data)

    def test_reject_boolean_identities_and_malformed_rows(self):
        edits = [lambda d: d.update(schemaVersion=True),
                 lambda d: d["pairs"][0].update(pair=True),
                 lambda d: d["pairs"][0]["modes"].__setitem__(0, None),
                 lambda d: mode_at(d, 1, "off")["cases"][0].update(objectCount=100.0)]
        for edit in edits:
            with self.subTest(edit=edit):
                data = example(); edit(data)
                with self.assertRaises(ValueError): cohort.summarize(data)

    def test_no_scope_promotion_and_no_input_or_result_alias_mutation(self):
        data = example(); original = copy.deepcopy(data)
        result = cohort.summarize(data)
        self.assertEqual(data, original)
        for key in ("causalEffectAccepted", "applicationOnlyLatencyAccepted", "exactContinuousPeakAccepted",
                    "actualIpcAccepted", "twoCommittedFramesProven", "performanceDistributionAccepted",
                    "fullRF312Accepted", "nativeReceiptAuditsReplacedByThisReducer"):
            self.assertIs(result[key], False, key)
        result["allPairOutcomes"][0]["pair"] = 99
        self.assertEqual(data, original)

    def test_cli_binds_input_and_analyzer_sources_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, output = root / "input.json", root / "summary.json"
            raw = (json.dumps(example()) + "\n").encode("utf-8")
            source.write_bytes(raw)
            command = [sys.executable, "-B", str(Path(cohort.__file__).resolve()),
                       "--input", str(source), "--output", str(output)]
            options = dict(stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
            first = subprocess.run(command, **options)
            self.assertEqual(first.returncode, 0, first.stderr.decode("utf-8"))
            saved = output.read_bytes()
            value = json.loads(saved)
            self.assertEqual(value["inputFileSha256"], hashlib.sha256(raw).hexdigest())
            self.assertEqual(value["analysisSourceSha256"]["rf312_sampling_cohort.py"],
                             hashlib.sha256(Path(cohort.__file__).read_bytes()).hexdigest())
            self.assertNotIn(b"\r", saved)
            second = subprocess.run(command, **options)
            self.assertNotEqual(second.returncode, 0)
            self.assertEqual(output.read_bytes(), saved)


if __name__ == "__main__":
    unittest.main()
