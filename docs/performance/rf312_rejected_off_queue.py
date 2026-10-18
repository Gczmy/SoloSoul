"""Read-only audit of the actual sampling-off queue rejected after nine stages.

Keep the prior sampling-on audit and all measured readers unchanged. This entry
does not accept a complete queue or invent post-failure state or memory evidence.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace

import rf312_rejected_queue as base
from native_journey_contract import PHASES

original = base.original
read, sha, require = base.read, base.sha, base.require
PERFORMANCE, ROOT, WORK = base.PERFORMANCE, base.ROOT, base.WORK
SCOPE = 'rejected-off-queue-complete-100-and-nine-stage-5000-prefix'


def check_failure(parent, small, large, names):
    require(parent.get('passed') is False and parent.get('restored') is True
            and parent.get('allFrozenInputsPreserved') is True
            and parent.get('supportedDefaultStateAccepted') is False
            and parent.get('fullJourneyAccepted') is False, 'An explicitly rejected restored queue is required')
    require(parent.get('journeyMemorySamplingEnabled') is False
            and parent.get('ownedUiTransportEnabled') is True, 'Original sampling-off Java UI profile required')
    require([row.get('count') for row in parent.get('outcomes', [])] == [100]
            and parent['outcomes'][0].get('passed') is True, 'Only the actual completed small-cell prefix is supported')
    require(small.get('passed') is True and small.get('defaultStateAdapterAccepted') is True
            and large.get('passed') is False and large.get('defaultStateAdapterAccepted') is False,
            'Cell outcomes must remain unchanged')
    require(large.get('errorType') == 'TimeoutExpired'
            and 'dumpsys' in large.get('error', '') and 'processes' in large['error']
            and 'timed out after 5 seconds' in large['error'], 'Original five-second state refusal required')
    require(names == list(PHASES[:9]), 'The actual nine-stage failed prefix is required')
    for report in [small, large]:
        require(all(report.get(key) is True for key in ['restored', 'originalApksRestored',
            'blurRestored', 'ownedProbeRemoved', 'ownedPublicOcrInputRemoved', 'frozenInputsUnchanged'])
            and report.get('restoreError') is report.get('sourceRestoreError') is None,
            'Every original cell restoration gate must remain satisfied')
    require(all(parent.get('cleanup', {}).get(key) is True for key in [
        'ownedPackagesRemoved', 'settingsExactlyUnchanged', 'normalShutdownRequested',
        'launcherExited', 'serverExited', 'freshHostAfterPassed']), 'Original strict outer cleanup required')


def derive(name, count):
    require(name in ['state', 'ime', 'ui'] and count in [100, 5000]
            and (count == 100 or name != 'state'), 'Only the explicit off-profile prefix readers are supported')
    text = base.derive(name, count)
    text = base.replace_once(text, 'from rf312_rejected_queue import load_context',
                             'from rf312_rejected_off_queue import load_context')
    if count == 5000:
        if name == 'ime':
            text = base.replace_once(text, 'assert len(typed)==1', 'assert len(typed)==2')
            text = base.replace_once(text, 'typingActions=1', 'typingActions=2')
        else:
            text = base.replace_once(text, 'assert content==0', 'assert content==5')
            text = base.replace_once(text, 'uiStages=2', 'uiStages=9')
    return text


def load_context():
    original.require_assertions()
    base.guard_subprocesses([])
    parser = argparse.ArgumentParser(); parser.add_argument('--context', type=Path, required=True)
    path = original.ordinary(parser.parse_args().context); value = read(path)
    require(value['scope'] == SCOPE and value['auditorSha256'] == sha(__file__)
            and value['baseAuditorSha256'] == sha(base.__file__), 'Both actual auditor generations must remain bound')
    require(value['sourceSha256'] == original.binding.sources()
            and value['queueSourceSha256'] == original.sources(), 'Original measured sources changed')
    native = original.ordinary(value['native'], directory=True)
    output = original.ordinary(value['output'], directory=True)
    require(path.parent == output and not output.is_relative_to(native) and not native.is_relative_to(output),
            'Separate audit and native evidence required')
    for name, expected in value['derivedSha256'].items():
        require(sha(output / 'readers' / name) == expected, 'Derived reader changed')
    adapter, units_file = original.ordinary(value['adapterInputs']), original.ordinary(value['unitEvidence'])
    require(value['inputSha256'] == dict(adapterInputs=sha(adapter), unitEvidence=sha(units_file),
        nativeResult=sha(native / 'result.json')), 'Original input binding changed')
    units = original.validate_units(units_file); loaded = original.binding.load(adapter)
    check_failure(*base.failure_reports(native))
    return SimpleNamespace(workspace=WORK, native=native, output=output, units=units,
                           units_file=units_file, loaded=loaded, mode='off')


def audit_partial(ctx):
    import rf312_instrumentation_state as state
    p = ctx.native / 'journey-5000'
    require(not (p / 'journey-memory').exists(), 'Sampling-off failure must not acquire a trajectory')
    for name in ['default-external/supported-state-evidence.json',
                 'default-external/measured-workload-window.json', 'post-journey-memory/result.json']:
        require(not (p / name).exists(), 'Missing complete state, window or post-memory proof must remain absent')
    sdk = read(p / 'commands.json'); failed = [row for row in sdk if row.get('timedOut')]
    require(len(failed) == 1, 'Exactly the actual failed state SDK operation required')
    row = failed[0]
    require(row['command'] == ['shell', 'dumpsys', 'activity', 'processes']
            and row['timeoutSeconds'] == 5 and row['exitCode'] is None and row['elapsedSeconds'] >= 5,
            'Original five-second refusal must remain a refusal')
    for key in ['stdout', 'stderr']:
        raw = (p / row[key + 'PartialFile']).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == row[key + 'Sha256'] and raw == b'',
                'Empty timed-out bytes cannot prove absence')
    proofs = sorted((p / 'instrumentation-state').glob('*.proof.json'))
    ordinals, active, observer_pid = [], [], None
    for file in proofs:
        ordinal = int(file.name.split('.')[0]); prefix = file.with_name(f'{ordinal:03d}')
        proof, command = read(file), read(prefix.with_suffix('.command.json'))
        raw = prefix.with_suffix('.stdout').read_bytes()
        require(command['command'] == row['command'] and command['exitCode'] == 0
                and prefix.with_suffix('.stderr').read_bytes() == b''
                and sha(prefix.with_suffix('.stdout')) == proof['stdoutSha256'], 'Original state bytes required')
        parsed = state.parse(raw)
        require(parsed == proof['nativeRows'] and proof['ordinal'] == ordinal, 'Original state parser proof changed')
        expected = {} if file == proofs[0] else {'org.solosoul.rf312observer/.Observer': 'org.solosoul.rf312observer'}
        state.expected(parsed, expected)
        for native_row in parsed:
            if observer_pid is None: observer_pid = native_row['processId']
            require(native_row['processId'] == observer_pid, 'Stable actual observer process required')
        ordinals.append(ordinal); active.append(len(parsed))
    rows = [v for v in sdk if v['command'] == row['command']]
    missing = base.check_state_prefix(ordinals, active, rows)
    for file, command in zip(proofs, rows[:-1]):
        require(read(file)['stdoutSha256'] == command['stdoutSha256']
                and 0 < command['timeoutSeconds'] <= 5, 'Saved state and actual SDK receipt must correspond in order')
    root = Path(ctx.loaded['runtime']['host']['workspaceRoot'])
    restore = base.module('failed_off_prefix_restore', root / 'tauri/scripts/android-home-native-regression.py')
    require(restore.inventory(p / 'private-data-before.tar', require_empty=True)
            == restore.inventory(p / 'private-data-restored.tar', require_empty=True), 'Failed cell data not restored')
    result = dict(passed=True, scope='rejected-off-5000-state-prefix-only', completeUiStages=9,
        nativeStateProofs=len(proofs), failedStateOrdinals=missing, timedOutSdkOperations=1,
        originalStateTimeoutSeconds=5, afterObserverAbsenceProven=False, stableObserverPidProven=True,
        samplingEnabled=False, completeTrajectoryAccepted=False, allOriginalStateRestored=True,
        workloadWindowUpperBoundMs=None, postJourneyPssBytes=None,
        nativeCompleteQueueAccepted=False, fullRF312Accepted=False,
        driverReportSha256=sha(p / 'report.json'), phaseProgressSha256=sha(p / 'default-external/phase-progress.json'),
        sdkLedgerSha256=sha(p / 'commands.json'))
    with (ctx.output / 'partial-state.json').open('x', encoding='utf-8') as stream: json.dump(result, stream, indent=2)
    return result


def main():
    original.require_assertions()
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['native', 'adapter-inputs', 'unit-evidence', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args(); allowed = []; base.guard_subprocesses(allowed)
    native = original.ordinary(args.native, directory=True)
    adapter, units_file = original.ordinary(args.adapter_inputs), original.ordinary(args.unit_evidence)
    output = Path(args.output).absolute()
    require(not output.exists() and not output.is_relative_to(native) and not native.is_relative_to(output),
            'New output separate from native evidence required')
    original.validate_units(units_file); original.binding.load(adapter)
    check_failure(*base.failure_reports(native))
    parent = read(native / 'result.json')
    require(parent['relatedUnitEvidenceSha256'] == sha(units_file)
            and parent['queueSourceSha256'] == original.sources(), 'Native source and regression binding changed')
    output.mkdir(exist_ok=False); readers = output / 'readers'; readers.mkdir()
    jobs = [('state', 100), ('ime', 100), ('ui', 100), ('ime', 5000), ('ui', 5000)]
    for name, count in jobs: (readers / f'{name}-{count}.py').write_text(derive(name, count), encoding='utf-8', newline='\n')
    context = dict(scope=SCOPE, native=str(native), output=str(output), adapterInputs=str(adapter), unitEvidence=str(units_file),
        sourceSha256=original.binding.sources(), queueSourceSha256=original.sources(), auditorSha256=sha(__file__),
        baseAuditorSha256=sha(base.__file__), inputSha256=dict(adapterInputs=sha(adapter), unitEvidence=sha(units_file),
        nativeResult=sha(native / 'result.json')), derivedSha256={p.name: sha(p) for p in readers.glob('*.py')})
    context_path = output / 'context.json'; context_path.write_text(json.dumps(context, indent=2), encoding='utf-8')
    commands = [[sys.executable, '-B', str(readers / f'{name}-{count}.py'), '--context', str(context_path)] for name, count in jobs]
    allowed.extend(commands)
    env = dict(os.environ, PYTHONPATH=os.pathsep.join([str(PERFORMANCE), str(ROOT)]),
        PYTHONDONTWRITEBYTECODE='1', TEMP=str(output), TMP=str(output))
    result = dict(passed=False, nativeCompleteQueueAccepted=False, fullRF312Accepted=False,
        sdkCommandsExecuted=0, inputEventsDispatched=0)
    try:
        for index, ((name, count), command) in enumerate(zip(jobs, commands)):
            if index == 3:
                partial = audit_partial(SimpleNamespace(native=native, output=output, loaded=original.binding.load(adapter)))
            prefix = output / f'{name}-{count}'
            with prefix.with_suffix('.stdout').open('xb') as out, prefix.with_suffix('.stderr').open('xb') as err:
                child = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=out, stderr=err, env=env)
            require(child.returncode == 0, f'Off-prefix {name}-{count} audit refused; original bytes preserved')
            for file in ['independent-ime-readiness-r1.json', 'independent-ui-r1.json']:
                p = output / file
                if p.exists(): p.rename(output / f'{count}-{file}')
        complete = read(output / 'independent-current-state-r1.json')
        result.update(passed=True, scope=SCOPE, completeCell100Accepted=True, completeCell100UiStages=12,
            completeCell100UiStateChecks=complete['uiStateChecks'], completeCell100PssBytes=complete['cases'][0]['pssBytes'],
            failedCell5000=partial, originalQueueCodeAndReadersUnchanged=True,
            parentResultSha256=sha(native / 'result.json'), auditorSha256=sha(__file__),
            baseAuditorSha256=sha(base.__file__), contextSha256=sha(context_path))
    except Exception as error:
        import traceback
        result.update(errorType=type(error).__name__, error=str(error))
        (output / 'failure.traceback').write_text(traceback.format_exc(), encoding='utf-8')
    with (output / 'result.json').open('x', encoding='utf-8') as stream: json.dump(result, stream, indent=2)
    print(json.dumps(result)); return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
