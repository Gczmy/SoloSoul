"""Reduce the pre-registered RF-312 five-attempt series without changing its scope.

Input is a de-identified projection of independently audited native receipts.
This reducer validates that projection; it does not replace the native audits.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import statistics

BINDING_FIELDS = {
    "apkSha256", "nativeLibrarySha256", "sourceSnapshotSha256",
    "runtimeSourceSha256", "fixturesSha256", "environmentSha256",
    "measurementPlanSha256",
}
PHASES = [
    "startup", "password-unlock", "first-ocr-user-flow", "workspace",
    "search-needle", "attachments-open", "image-preview", "text-preview",
    "pdf-text-native", "pdf-scanned-native", "application-lock",
    "password-reunlock",
]


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def is_sha(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def digest(value):
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"),
                     ensure_ascii=False, allow_nan=False).encode("utf-8")
    return hashlib.sha256(raw).hexdigest()


def distribution(values):
    present = sorted(value for value in values if value is not None)
    return {
        "valuesInAttemptOrder": values,
        "acceptedSamples": len(present),
        "failedAttempts": len(values) - len(present),
        "median": statistics.median(present) if present else None,
        "nearestRankEmpiricalP95": present[math.ceil(.95 * len(present)) - 1] if present else None,
        "maximum": max(present) if present else None,
    }


def summarize(data):
    require(isinstance(data, dict), "A measurement object is required")
    require(type(data.get("schemaVersion")) is int and data["schemaVersion"] == 1, "Unknown schema")
    require(type(data.get("plannedAttempts")) is int and data["plannedAttempts"] == 5,
            "The registered series has exactly five attempts")
    require(data.get("objectCounts") == [100, 5000], "Original size order required")
    require(all(type(count) is int for count in data["objectCounts"]), "Integer object counts required")
    require(data.get("phaseNames") == PHASES, "Original twelve-stage protocol required")
    require(data.get("metric") == "hostObservedUpperBoundMs", "Do not relabel host bounds as application latency")
    binding = data.get("binding")
    require(isinstance(binding, dict) and set(binding) == BINDING_FIELDS,
            "Complete source, fixture, environment and measurement-plan binding required")
    require(all(is_sha(v) for v in binding.values()), "Invalid binding digest")
    require(data.get("bindingSha256") == digest(binding), "Binding digest mismatch")
    attempts = data.get("attempts")
    require(isinstance(attempts, list) and len(attempts) == 5, "Missing or replacement attempt")
    require(all(isinstance(a, dict) and type(a.get("attempt")) is int for a in attempts),
            "Integer attempt identities required")
    require([a.get("attempt") for a in attempts] == [1, 2, 3, 4, 5], "Attempt order, gaps or duplicates")
    for attempt in attempts:
        require(type(attempt.get("accepted")) is bool, "Explicit attempt outcome required")
        require(attempt.get("bindingSha256") == data["bindingSha256"], "Mixed source or protocol series")
        evidence = attempt.get("evidenceSha256")
        require(isinstance(evidence, dict) and evidence and all(is_sha(v) for v in evidence.values()),
                "Terminal evidence must remain attached to every attempt")
        cases = attempt.get("cases")
        require(isinstance(cases, list) and all(isinstance(c, dict) for c in cases)
                and [c.get("objectCount") for c in cases] == [100, 5000],
                "Both planned case outcomes, including failures, required")
        for case in cases:
            require(type(case.get("objectCount")) is int, "Integer case object count required")
            accepted = case.get("accepted")
            require(type(accepted) is bool, "Explicit case outcome required")
            phases = case.get("completedPhases")
            require(isinstance(phases, list) and len(phases) <= len(PHASES), "Invalid completed-stage list")
            require(all(isinstance(phase, dict) for phase in phases), "Stage objects required")
            require([p.get("name") for p in phases] == PHASES[:len(phases)], "Stage gaps or substitutions")
            for phase in phases:
                value = phase.get("hostObservedUpperBoundMs")
                require(type(value) in (int, float) and math.isfinite(value) and 0 < value <= 45000,
                        "Finite positive stage bound within the unchanged deadline required")
            pss = case.get("postJourneyPssBytes")
            require(pss is None or (type(pss) is int and pss > 0), "Invalid post-journey memory value")
            if accepted:
                require(len(phases) == len(PHASES) and pss is not None, "Accepted case is incomplete")
                require(case.get("failureEvidenceSha256") is None, "Contradictory accepted outcome")
            else:
                require(is_sha(case.get("failureEvidenceSha256")), "Failure evidence cannot be dropped")
        require(not attempt["accepted"] or all(c["accepted"] for c in cases), "Accepted attempt contains a failed case")
    cases = []
    for index, count in enumerate([100, 5000]):
        accepted = [a["accepted"] and a["cases"][index]["accepted"] for a in attempts]
        phases = []
        for phase_index, name in enumerate(PHASES):
            values = [a["cases"][index]["completedPhases"][phase_index]["hostObservedUpperBoundMs"]
                      if ok else None for a, ok in zip(attempts, accepted)]
            phases.append({"name": name, **distribution(values)})
        memory = [a["cases"][index]["postJourneyPssBytes"] if ok else None
                  for a, ok in zip(attempts, accepted)]
        cases.append({"objectCount": count, "phases": phases,
                      "postJourneyPssBytes": distribution(memory)})
    return {
        "schemaVersion": 1, "measurementInputDigestSha256": digest(data),
        "plannedAttempts": 5, "acceptedAttempts": sum(a["accepted"] for a in attempts),
        "failedAttempts": sum(not a["accepted"] for a in attempts),
        "bindingSha256": data["bindingSha256"], "metric": data["metric"], "cases": cases,
        "allAttemptOutcomes": attempts,
        "fixedHostObservedSeriesCollected": all(a["accepted"] for a in attempts),
        "p95IsSmallSampleEmpiricalNotPopulationTail": True,
        "memoryScope": "one unlocked-home post-journey checkpoint per case per accepted attempt",
        "nativeReceiptAuditsReplacedByThisReducer": False,
        "performanceDistributionAccepted": False, "applicationOnlyLatencyAttributed": False,
        "wholeJourneyOrPeakMemoryAccepted": False, "sameSourceObserverOverheadMeasured": False,
        "actualIpcAccepted": False, "twoCommittedFramesProven": False, "fullRF312Accepted": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    raw = args.input.read_bytes()
    result = summarize(json.loads(raw.decode("utf-8-sig")))
    result["inputFileSha256"] = hashlib.sha256(raw).hexdigest()
    with args.output.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(result, stream, ensure_ascii=False, allow_nan=False, indent=2)
        stream.write("\n")
    print(json.dumps({k: result[k] for k in ("acceptedAttempts", "failedAttempts", "fullRF312Accepted")}))


if __name__ == "__main__":
    main()
