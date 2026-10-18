"""Explicit paths and source bindings shared by the native queue and replayers."""
import argparse
import hashlib
import json
from pathlib import Path
from types import SimpleNamespace
import sys

ROOT = Path(__file__).resolve().parent
WORK = ROOT.parents[2]
PERFORMANCE = WORK / 'docs/performance'
HELPER = PERFORMANCE / 'rf312-default-observer'
sys.path[:0] = [str(PERFORMANCE), str(HELPER), str(PERFORMANCE / 'rf312-ipc-runtime')]

import rf312_default_binding as binding
from prepare_avd import ordinary

read = lambda path: json.loads(Path(path).read_bytes())
sha = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require_assertions():
    if sys.flags.optimize:
        raise ValueError('This validation entry requires Python assertions enabled')


def sources():
    return {path.name: sha(ordinary(path)) for path in sorted(ROOT.glob('*.py'))}


def validate_units(path):
    require_assertions()
    path = ordinary(path)
    value = read(path)
    from related_units import collected, case_ids, queue_collected
    expected = case_ids(collected())
    queue_expected = case_ids(queue_collected())
    extras = {name: sha(WORK / name) for name in (
        'tauri/scripts/android-process-memory.py', 'tauri/scripts/test_android_process_memory.py')}
    if (value.get('passed') is not True or value.get('tests') != len(expected)
            or value.get('testIds') != expected or len(expected) != 206
            or value.get('failures') != 0 or value.get('errors') != 0 or value.get('skipped') != 0
            or value.get('sourceSha256') != binding.sources()
            or value.get('externalReaderSourcesSha256') != extras
            or value.get('queueSourceSha256') != sources()
            or value.get('queueTests') != len(queue_expected)
            or value.get('queueTestIds') != queue_expected
            or value.get('queueTestsPassed') is not True
            or value.get('queueSkipped') != 0):
        raise ValueError('Complete passing current-source related and queue regression evidence required')
    log = ordinary(path.parent / value['logFile'])
    if log.parent != path.parent or sha(log) != value['logSha256']:
        raise ValueError('Original regression log changed')
    return value


def load_context():
    """Read a replay-owned context; this function never runs a device command."""
    require_assertions()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--context', required=True, type=Path)
    args = parser.parse_args()
    path = ordinary(args.context)
    value = read(path)
    fields = {'schemaVersion', 'native', 'adapterInputs', 'unitEvidence', 'output',
              'inputSha256', 'sourceSha256', 'queueSourceSha256'}
    if set(value) != fields or type(value['schemaVersion']) is not int or value['schemaVersion'] != 1:
        raise ValueError('Exact replay context required')
    output = ordinary(value['output'], directory=True)
    native = ordinary(value['native'], directory=True)
    if (path.parent != output or native == output or output.is_relative_to(native)
            or native.is_relative_to(output)):
        raise ValueError('A separate replay-owned output directory is required')
    adapter = ordinary(value['adapterInputs'])
    units_file = ordinary(value['unitEvidence'])
    expected = {'adapterInputs': sha(adapter), 'unitEvidence': sha(units_file),
                'nativeResult': sha(native / 'result.json')}
    if (value['inputSha256'] != expected or value['sourceSha256'] != binding.sources()
            or value['queueSourceSha256'] != sources()):
        raise ValueError('Frozen replay sources or inputs changed')
    units = validate_units(units_file)
    loaded = binding.load(adapter)
    parent = read(native / 'result.json')
    if (parent.get('passed') is not True or parent.get('restored') is not True
            or type(parent.get('journeyMemorySamplingEnabled')) is not bool):
        raise ValueError('A complete normally restored native queue is required')
    return SimpleNamespace(workspace=WORK, native=native, output=output, adapter=adapter,
                           units_file=units_file, units=units, loaded=loaded,
                           mode='on' if parent['journeyMemorySamplingEnabled'] else 'off')
