"""Derive elapsed control intervals after the unchanged canonical UI replay.

Reads a complete normally restored public-fixture queue. Never starts SDK/UI.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback

import native_timing

HERE = Path(__file__).resolve().parent
QUEUE = HERE.parent / 'rf312-native-queue'
sys.path.insert(0, str(QUEUE))
import replay as canonical
from native_journey_contract import PHASES


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write('\n')


def replay_readers(context_path, output, context):
    receipts = {}
    for name, receipt_name in (
        ('state', 'independent-current-state-r1.json'),
        ('ime', 'independent-ime-readiness-r1.json'),
        ('ui', 'independent-ui-r1.json'),
    ):
        script = QUEUE / f'replay_{name}.py'
        if sha(script) != context['queueSourceSha256'][script.name]:
            raise ValueError('Canonical reader changed')
        command = [sys.executable, '-B', str(script), '--context', str(context_path)]
        save(output/f'canonical-{name}-command.json', dict(argv=command, readerSha256=sha(script)))
        with (output/f'canonical-{name}.stdout').open('xb') as out, (output/f'canonical-{name}.stderr').open('xb') as err:
            child = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                                   env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'),
                                   creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        receipt = context_path.parent / receipt_name
        if child.returncode or native_timing.decode(native_timing.raw_file(receipt)).get('passed') is not True:
            raise ValueError(f'Canonical {name} replay refused; original output retained')
        receipts[name] = sha(receipt)
    return receipts


def main():
    canonical.context.require_assertions()
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('native', 'adapter-inputs', 'unit-evidence', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    parent = canonical.context.ordinary(args.output.absolute().parent, directory=True)
    native = canonical.context.ordinary(args.native, directory=True)
    output = parent / args.output.name
    if (output.exists() or output.is_symlink() or output.is_relative_to(native)
            or native.is_relative_to(output)):
        parser.error('New output outside native evidence required')
    output.mkdir(exist_ok=False)
    result = dict(passed=False, scope='canonically-verified-native-control-method-intervals',
                  sdkCommandsExecuted=0, inputEventsDispatched=0, exclusiveObserverOverheadAccepted=False,
                  applicationOnlyLatencyAccepted=False, firstInteractiveTimeProven=False,
                  nativeFramesAccepted=False, performanceDistributionAccepted=False, fullRF312Accepted=False)
    try:
        context_path, _ = canonical.prepare(native, args.adapter_inputs, args.unit_evidence,
                                            output / 'canonical ui replay')
        context = native_timing.decode(native_timing.raw_file(context_path))
        frozen = {count: native_timing.fingerprint(native/f'journey-{count}/independent-observer',
                    native/f'journey-{count}/default-external/phase-progress.json') for count in (100, 5000)}
        state_files = {count: native/f'journey-{count}/default-external/supported-state-evidence.json'
                       for count in (100, 5000)}
        state_bytes = {count: native_timing.raw_file(path) for count, path in state_files.items()}
        source = {p.name: sha(p) for p in sorted(HERE.glob('*.py'))}
        receipts = replay_readers(context_path, output, context)
        receipt = context_path.parent/'independent-ui-r1.json'
        cases = []
        for count in (100, 5000):
            phase_file = native/f'journey-{count}/default-external/phase-progress.json'
            if native_timing.raw_file(state_files[count]) != state_bytes[count]:
                raise ValueError('Canonical state evidence changed during replay')
            if native_timing.bind_phase_records(native_timing.raw_file(phase_file), state_bytes[count]) != len(PHASES):
                raise ValueError('All original canonical phase records required')
            case = native_timing.collect(native/f'journey-{count}/independent-observer',
                    phase_file, PHASES, frozen[count])
            save(output/f'timing-{count}.json', case)
            cases.append(dict(objectCount=count, requests=case['requests'],
                              nativeChannelSpanNs=case['nativeChannelSpanNs'],
                              nativeOperationDurationSumNs=case['nativeOperationDurationSumNs'],
                              operationPopulations=case['operationPopulations'],
                              matchingXmlCapturePairs=len(case['consecutiveMatchingXmlPairs']),
                              phaseScreenshotObservationWindows=case['phaseScreenshotObservationWindows'],
                              timingSha256=sha(output/f'timing-{count}.json')))
        if (source != {p.name: sha(p) for p in sorted(HERE.glob('*.py'))}
                or native_timing.decode(native_timing.raw_file(context_path)) != context
                or context['sourceSha256'] != canonical.context.binding.sources()
                or context['queueSourceSha256'] != canonical.context.sources()
                or any(native_timing.raw_file(state_files[count]) != raw for count, raw in state_bytes.items())
                or sha(native/'result.json') != context['inputSha256']['nativeResult']):
            raise ValueError('Bound sources or original inputs changed during analysis')
        result.update(passed=True, canonicalUiReplayPassed=True, nativeReplyBytesRevalidated=True,
                      canonicalPhaseRecordsBound=True,
                      supportedStateSha256={str(count): native_timing.digest(raw) for count, raw in state_bytes.items()},
                      sourceSha256=source, canonicalReaderReceiptSha256=receipts, canonicalUiReceiptSha256=sha(receipt),
                      contextSha256=sha(context_path), nativeParentSha256=sha(native/'result.json'),
                      canonicalUiStdoutSha256=sha(output/'canonical-ui.stdout'),
                      canonicalUiStderrSha256=sha(output/'canonical-ui.stderr'), cases=cases)
    except Exception as error:
        result.update(errorType=type(error).__name__, error=str(error))
        traceback.print_exc()
    save(output/'result.json', result)
    print(json.dumps({k: v for k, v in result.items() if k not in ('cases', 'sourceSha256')}))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
