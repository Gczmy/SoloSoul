"""Independently replay one complete native queue without running SDK or UI input."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import traceback

import queue_context as context


def prepare(native, adapter, units, output):
    context.require_assertions()
    native = context.ordinary(native, directory=True)
    adapter = context.ordinary(adapter)
    units = context.ordinary(units)
    parent = context.ordinary(Path(output).absolute().parent, directory=True)
    output = parent / Path(output).name
    if output.exists() or output.is_symlink():
        raise ValueError('New replay output required')
    context.validate_units(units)
    context.binding.load(adapter)
    value = context.read(native / 'result.json')
    if (value.get('passed') is not True or value.get('restored') is not True
            or type(value.get('journeyMemorySamplingEnabled')) is not bool):
        raise ValueError('Complete restored native queue required; preserve failed queues separately')
    if output.is_relative_to(native) or native.is_relative_to(output):
        raise ValueError('Replay output must be separate from native evidence')
    output.mkdir(exist_ok=False)
    frozen = dict(schemaVersion=1, native=str(native), adapterInputs=str(adapter),
                  unitEvidence=str(units), output=str(output),
                  inputSha256=dict(adapterInputs=context.sha(adapter),
                                   unitEvidence=context.sha(units),
                                   nativeResult=context.sha(native / 'result.json')),
                  sourceSha256=context.binding.sources(), queueSourceSha256=context.sources())
    path = output / 'context.json'
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(frozen, stream, indent=2)
    return path, value['journeyMemorySamplingEnabled']


def main():
    context.require_assertions()
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('native', 'adapter-inputs', 'unit-evidence', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    path, sampling = prepare(args.native, args.adapter_inputs, args.unit_evidence, args.output)
    output = path.parent
    result = dict(passed=False, sdkCommandsExecuted=0, inputEventsDispatched=0,
                  applicationOnlyLatencyAccepted=False, performanceDistributionAccepted=False,
                  fullRF312Accepted=False)
    try:
        steps = ['state', 'ime', 'ui'] + (['memory'] if sampling else [])
        for name in steps:
            with (output / (name + '.stdout')).open('xb') as stdout, (output / (name + '.stderr')).open('xb') as stderr:
                child = subprocess.run([sys.executable, '-B', str(context.ROOT / ('replay_' + name + '.py')),
                                        '--context', str(path)], stdin=subprocess.DEVNULL,
                                       stdout=stdout, stderr=stderr, check=False)
            if child.returncode:
                raise RuntimeError('Independent ' + name + ' replay refused; original output retained')
        state = context.read(output / 'independent-current-state-r1.json')
        ime = context.read(output / 'independent-ime-readiness-r1.json')
        ui = context.read(output / 'independent-ui-r1.json')
        assert state['passed'] and ime['passed'] and ui['passed']
        parent = context.read(args.native / 'result.json')
        if sampling:
            memory = context.read(output / 'independent-memory-r1.json')
            assert memory['passed']
            result['memory'] = memory
        else:
            for count in [100, 5000]:
                child = args.native / ('journey-' + str(count))
                assert not (child / 'journey-memory').exists() and not (child / 'journey-memory-sdk').exists()
        assert context.read(path)['queueSourceSha256'] == context.sources()
        assert context.read(path)['sourceSha256'] == context.binding.sources()
        assert context.sha(args.native / 'result.json') == context.read(path)['inputSha256']['nativeResult']
        result.update(passed=True, samplingEnabled=sampling, completeUiStages=24,
                      sourceSha256=context.binding.sources(), queueSourceSha256=context.sources(),
                      unitTests=206, uiStateChecks=state['uiStateChecks'], memoryStateDumps=8,
                      cases=ui['cases'], allOriginalStateRestored=True, strictHostAfterReplayed=True,
                      nativeParentSha256=context.sha(args.native / 'result.json'),
                      contextSha256=context.sha(path), originalNativeEvidenceChanged=False)
    except Exception as error:
        result.update(errorType=type(error).__name__, error=str(error))
        (output / 'failure.traceback').write_text(traceback.format_exc(), encoding='utf-8')
    finally:
        with (output / 'result.json').open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(result, stream, indent=2)
        print(json.dumps({key: value for key, value in result.items()
                          if key not in ('memory', 'cases', 'sourceSha256', 'queueSourceSha256')}))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
