"""Describe the registered six-pair sampling cohort without replacing native audits.

Input is a de-identified projection of original receipts and independent replays.
An absent terminal receipt is unavailable evidence, never a failed or live process.
The earlier five-attempt reducer and all measured runtime sources remain unchanged.
"""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import statistics

from rf312_repeat_summary import BINDING_FIELDS as ORIGINAL_BINDINGS
from rf312_repeat_summary import PHASES, digest, is_sha, require


BINDING_FIELDS = ORIGINAL_BINDINGS | {
    "queueSourceSha256", "ownedUiPackageSha256", "ownedUiArtifactSha256",
    "observerApkSha256", "extraLaunchSourceSha256",
    "canonicalMemoryReaderSha256", "canonicalMemoryReaderTestSha256",
}
MODE_ORDER = [["off", "on"], ["on", "off"]] * 3
BUDGETS = dict(observerSeconds=240, phaseSeconds=45, uiStateSdkSeconds=5,
               memoryPointSeconds=20, memorySdkSeconds=10)
PENDING = "awaiting-terminal-evidence"
CASE_STATES = ("accepted", "completed-before-refusal", "refused", "not-reached", PENDING)
FAILURE_CATEGORIES = {"preflight-refused", "native-case-refused", "restore-refused",
                      "replay-refused", "host-finalization-refused"}


def evidence(value, required=()):
    require(isinstance(value, dict) and all(isinstance(k, str) and k and is_sha(v)
                                          for k, v in value.items()),
            "Evidence must contain named SHA-256 digests")
    require(set(required) <= set(value), "Required original evidence is missing")


def number(value, *, optional=False, integer=False, maximum=None):
    if optional and value is None:
        return
    require(type(value) in ((int,) if integer else (int, float))
            and math.isfinite(value) and value > 0
            and (maximum is None or value <= maximum), "Invalid positive measurement")


def memory(value):
    require(isinstance(value, dict), "Accepted sampling-on case needs its replayed trajectory")
    for name in ("points", "acceptedMemoryPoints", "refusedPoints", "missedPeriodicSlots",
                 "plannedPeriodicSlots", "boundaryPoints"):
        require(type(value.get(name)) is int and value[name] >= 0, "Invalid trajectory count")
    require(value["points"] <= 192 and value["boundaryPoints"] == 12
            and value["acceptedMemoryPoints"] >= 12,
            "All twelve acknowledged boundaries are required")
    refusals = value.get("refusedPeriodicCaptures")
    require(isinstance(refusals, list), "Periodic refusals must remain explicit")
    ordinals = []
    for row in refusals:
        require(isinstance(row, dict) and row.get("kind") == "periodic"
                and row.get("numericSnapshotAccepted") is False,
                "Refused captures cannot become numerical samples")
        number(row.get("pointOrdinal"), integer=True, maximum=value["points"])
        require(isinstance(row.get("errorType"), str) and bool(row["errorType"])
                and is_sha(row.get("receiptSha256")), "Refusal evidence is missing")
        ordinals.append(row["pointOrdinal"])
    require(len(set(ordinals)) == len(ordinals), "Duplicate refused captures")
    require(value["refusedPoints"] == value["missedPeriodicSlots"] + len(refusals)
            and value["points"] == 1 + value["acceptedMemoryPoints"] + value["refusedPoints"]
            and value["plannedPeriodicSlots"] == value["acceptedMemoryPoints"] - 12
            + value["missedPeriodicSlots"] + len(refusals), "Sampling gaps or refusals were dropped")
    require(type(value.get("allPeriodicCapturesSucceeded")) is bool
            and value["allPeriodicCapturesSucceeded"] == (not refusals),
            "Actual refusals and missed schedule slots have different meanings")
    require(value.get("workerStopped") is True
            and value.get("allScheduledPeriodicSlotsAccounted") is True,
            "Trajectory termination and complete schedule accounting are required")
    number(value.get("sampledMaximumSerialPssBytes"), integer=True)
    evidence(value.get("evidenceSha256"), ("trace", "independentMemoryReplay"))


def validate_case(case, count, mode):
    require(isinstance(case, dict) and type(case.get("objectCount")) is int
            and case["objectCount"] == count, "Original integer size order required")
    status = case.get("status")
    require(status in CASE_STATES, "Unknown case outcome")
    evidence(case.get("evidenceSha256"))
    phases = case.get("completedPhases")
    require(isinstance(phases, list) and len(phases) <= 12
            and all(isinstance(row, dict) for row in phases), "Invalid completed-stage prefix")
    require([row.get("name") for row in phases] == PHASES[:len(phases)],
            "Original ordered twelve-stage prefix required")
    for row in phases:
        number(row.get("hostObservedUpperBoundMs"), maximum=45000)
    number(case.get("workloadWindowUpperBoundMs"), optional=True)
    number(case.get("postJourneyPssBytes"), optional=True, integer=True)
    complete = status in ("accepted", "completed-before-refusal")
    if complete:
        require(len(phases) == 12 and case.get("workloadWindowUpperBoundMs") is not None
                and case.get("postJourneyPssBytes") is not None, "Complete case lacks measurements")
        evidence(case["evidenceSha256"], ("driver", "supportedState", "checkpoint"))
    if status == "not-reached":
        require(not phases and case.get("workloadWindowUpperBoundMs") is None
                and case.get("postJourneyPssBytes") is None and case.get("journeyMemory") is None
                and not case["evidenceSha256"],
                "Unexecuted cases cannot supply measurements")
    trace = case.get("journeyMemory")
    if mode == "off":
        require(trace is None, "Sampling-off cases cannot acquire journey samples")
    elif trace is not None or complete:
        require(len(phases) == 12, "Complete trajectory requires the complete stage prefix")
        memory(trace)


def distribution(values):
    present = sorted(value for value in values if value is not None)
    return dict(valuesInPairOrder=values, acceptedSamples=len(present),
                median=statistics.median(present) if present else None,
                nearestRankEmpiricalP95=present[math.ceil(.95 * len(present)) - 1] if present else None,
                maximum=max(present) if present else None)


def summarize(data):
    require(isinstance(data, dict) and type(data.get("schemaVersion")) is int
            and data["schemaVersion"] == 1, "Unknown cohort projection schema")
    require(type(data.get("plannedPairs")) is int and data["plannedPairs"] == 6,
            "The registered cohort has six pairs; no replacement attempts")
    require(data.get("modeOrderPerPair") == MODE_ORDER and data.get("objectCounts") == [100, 5000]
            and all(type(v) is int for v in data["objectCounts"]), "Registered mode and size orders required")
    require(data.get("phaseNames") == PHASES
            and data.get("metric") == "hostObservedUpperBoundMs", "Original observed-bound metric required")
    require(data.get("originalBudgets") == BUDGETS
            and all(type(v) is int for v in data["originalBudgets"].values()), "Original budgets required")
    binding = data.get("binding")
    require(isinstance(binding, dict) and set(binding) == BINDING_FIELDS
            and all(is_sha(v) for v in binding.values()), "Complete common cohort provenance required")
    require(data.get("bindingSha256") == digest(binding), "Cohort binding digest mismatch")
    pairs = data.get("pairs")
    require(isinstance(pairs, list) and len(pairs) == 6
            and all(isinstance(row, dict) for row in pairs), "All six planned pair slots must remain")
    modes = {"off": [], "on": []}
    for ordinal, pair in enumerate(pairs, 1):
        require(type(pair.get("pair")) is int and pair["pair"] == ordinal, "Pair order or identity changed")
        rows = pair.get("modes")
        require(isinstance(rows, list) and len(rows) == 2
                and all(isinstance(row, dict) for row in rows)
                and [row.get("mode") for row in rows] == MODE_ORDER[ordinal - 1], "Registered mode order changed")
        for row in rows:
            require(row.get("bindingSha256") == data["bindingSha256"], "Mixed sources, tools or protocols")
            status = row.get("status")
            require(status in ("accepted", "refused", PENDING), "Explicit mode outcome required")
            required = {"accepted": ("nativeResult", "replayResult"),
                        "refused": ("nativeResult", "failure"), PENDING: ()}[status]
            evidence(row.get("evidenceSha256"), required)
            if status == "refused":
                require(isinstance(row.get("failureCategory"), str)
                        and row["failureCategory"] in FAILURE_CATEGORIES, "Explicit refusal category required")
            else:
                require(row.get("failureCategory") is None, "Contradictory mode outcome")
            cases = row.get("cases")
            require(isinstance(cases, list) and len(cases) == 2, "Both planned case outcomes must remain")
            for case, count in zip(cases, (100, 5000)):
                validate_case(case, count, row["mode"])
            states = [case["status"] for case in cases]
            if status in ("accepted", PENDING):
                require(states == [status, status], "Mode and case outcomes disagree")
            else:
                allowed = (["not-reached", "not-reached"], ["refused", "not-reached"],
                           ["completed-before-refusal", "refused"],
                           ["completed-before-refusal", "not-reached"],
                           ["completed-before-refusal", "completed-before-refusal"])
                require(states in allowed, "Failed mode cannot replace or resume a later size")
            modes[row["mode"]].append(row)
    cells = []
    for mode in ("off", "on"):
        for index, count in enumerate((100, 5000)):
            cases = [row["cases"][index] for row in modes[mode]]
            take = lambda getter: distribution([getter(case) if row["status"] == "accepted" else None
                                                for row, case in zip(modes[mode], cases)])
            cell = dict(mode=mode, objectCount=count, plannedSamples=6,
                        caseOutcomeCounts={status: sum(case["status"] == status for case in cases)
                                           for status in CASE_STATES},
                        workloadWindowUpperBoundMs=take(lambda case: case["workloadWindowUpperBoundMs"]),
                        postJourneyPssBytes=take(lambda case: case["postJourneyPssBytes"]),
                        phases=[dict(name=name, **take(lambda case, j=j: case["completedPhases"][j]["hostObservedUpperBoundMs"]))
                                for j, name in enumerate(PHASES)])
            if mode == "on":
                cell["sampledMaximumSerialPssBytes"] = take(lambda case: case["journeyMemory"]["sampledMaximumSerialPssBytes"])
                cell["trajectoryCoverageInPairOrder"] = [copy.deepcopy(case.get("journeyMemory")) for case in cases]
            cells.append(cell)
    paired = []
    for index, count in enumerate((100, 5000)):
        values = {}
        for metric in ("workloadWindowUpperBoundMs", "postJourneyPssBytes"):
            values[metric] = distribution([
                on["cases"][index][metric] - off["cases"][index][metric]
                if on["status"] == off["status"] == "accepted" else None
                for on, off in zip(modes["on"], modes["off"])])
        paired.append(dict(objectCount=count, **values))
    outcomes = {status: sum(row["status"] == status for rows in modes.values() for row in rows)
                for status in ("accepted", "refused", PENDING)}
    return dict(schemaVersion=1, inputProjectionSha256=digest(data), bindingSha256=data["bindingSha256"],
                plannedPairs=6, plannedModeRuns=12, modeOutcomeCounts=outcomes,
                allPlannedModeOutcomesTerminal=outcomes[PENDING] == 0,
                completeAcceptedPairs=sum(on["status"] == off["status"] == "accepted"
                                          for on, off in zip(modes["on"], modes["off"])),
                cells=cells, descriptivePairedOnMinusOff=paired, allPairOutcomes=copy.deepcopy(pairs),
                p95IsSmallSampleEmpiricalNotPopulationTail=True,
                missingTerminalEvidenceDoesNotDetermineProcessLiveness=True,
                onlyCompleteAcceptedModesEnterDistributions=True,
                previousSourceGenerationsPooled=False, nativeReceiptAuditsReplacedByThisReducer=False,
                causalEffectAccepted=False, applicationOnlyLatencyAccepted=False,
                exactContinuousPeakAccepted=False, actualIpcAccepted=False, twoCommittedFramesProven=False,
                performanceDistributionAccepted=False, fullRF312Accepted=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    raw = args.input.read_bytes()
    result = summarize(json.loads(raw.decode("utf-8-sig")))
    result["inputFileSha256"] = hashlib.sha256(raw).hexdigest()
    result["analysisSourceSha256"] = {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in (Path(__file__), Path(__file__).with_name("rf312_repeat_summary.py"))
    }
    with args.output.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(result, stream, ensure_ascii=False, allow_nan=False, indent=2)
        stream.write("\n")
    print(json.dumps({key: result[key] for key in ("modeOutcomeCounts", "completeAcceptedPairs", "fullRF312Accepted")}))


if __name__ == "__main__":
    main()
