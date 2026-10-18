"""Read-only audit of a rejected two-size queue: complete 100, failed 5000.

The original complete-queue gate remains closed. Original per-cell validators are
reused with an explicit single-cell scope; no native receipt is rewritten.
"""
import argparse
import ast
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace

PERFORMANCE = Path(__file__).resolve().parent
WORK = PERFORMANCE.parents[1]
ROOT = PERFORMANCE / 'rf312-native-queue'
sys.path[:0] = [str(ROOT), str(PERFORMANCE), str(PERFORMANCE / 'rf312-default-observer')]
import queue_context as original

read = original.read
sha = original.sha
SCOPE = 'rejected-queue-complete-100-and-pre-tools-5000-prefix'


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def check_failure(parent, small, large, names):
    require(parent.get('passed') is False and parent.get('restored') is True
            and parent.get('allFrozenInputsPreserved') is True
            and parent.get('supportedDefaultStateAccepted') is False
            and parent.get('fullJourneyAccepted') is False, 'An explicitly rejected restored queue is required')
    require(parent.get('journeyMemorySamplingEnabled') is True
            and parent.get('ownedUiTransportEnabled') is True, 'Original sampling and Java UI profile required')
    require([row.get('count') for row in parent.get('outcomes', [])] == [100]
            and parent['outcomes'][0].get('passed') is True, 'Only the actual completed small-cell prefix is supported')
    require(small.get('passed') is True and small.get('defaultStateAdapterAccepted') is True
            and large.get('passed') is False and large.get('defaultStateAdapterAccepted') is False,
            'Cell outcomes must remain unchanged')
    require(large.get('errorType') == 'TimeoutExpired'
            and 'dumpsys' in large.get('error', '') and 'processes' in large['error']
            and 'timed out after 5 seconds' in large['error'], 'Original five-second state refusal required')
    require(names == ['startup', 'password-unlock'], 'The actual two-stage failed prefix is required')
    for report in [small, large]:
        require(all(report.get(key) is True for key in ['restored', 'originalApksRestored',
            'blurRestored', 'ownedProbeRemoved', 'ownedPublicOcrInputRemoved', 'frozenInputsUnchanged']),
            'All original cell restoration gates must remain satisfied')
    require(all(parent.get('cleanup', {}).get(key) is True for key in [
        'ownedPackagesRemoved', 'settingsExactlyUnchanged', 'normalShutdownRequested',
        'launcherExited', 'serverExited', 'freshHostAfterPassed']), 'Original strict outer cleanup required')


def check_state_prefix(ordinals, active, sdk_rows):
    require(ordinals == list(range(1, len(ordinals) + 1)) and len(ordinals) >= 2
            and active == [0] + [1] * (len(active) - 1), 'Every saved state in this interrupted prefix must remain explicit')
    require(len(sdk_rows) == len(ordinals) + 1 and sdk_rows[-1].get('timedOut') is True
            and all(row.get('exitCode') == 0 and row.get('timedOut', False) is False for row in sdk_rows[:-1]),
            'The missing next state needs its actual timed-out SDK receipt')
    return [len(ordinals) + 1]


def replace_once(text, before, after):
    require(text.count(before) == 1, 'Original validator shape changed: ' + before)
    return text.replace(before, after)


def derive(name, count):
    require(name in ['state', 'ime', 'ui', 'memory'] and count in [100, 5000], 'Unknown prefix validator')
    require(count == 100 or name in ['ime', 'ui'], 'Failed state and memory need explicit prefix audits')
    path = ROOT / ('replay_' + name + '.py')
    text = path.read_text(encoding='utf-8')
    require(hashlib.sha256(path.read_bytes()).hexdigest() == original.sources()[path.name], 'Original queue source changed')
    old_tree = ast.parse(text)
    text = replace_once(text, 'from queue_context import load_context',
                        'from rf312_rejected_queue import load_context')
    if name in ['state', 'ime']:
        text = replace_once(text, 'for count in [100,5000]:', f'for count in [{count}]:')
    else:
        text = replace_once(text, 'for count in (100,5000):', f'for count in ({count},):')
    if name == 'state':
        text = replace_once(text, "assert parent['passed'] and parent['supportedDefaultStateAccepted'] and parent['fullJourneyAccepted']",
            "assert parent['passed'] is parent['supportedDefaultStateAccepted'] is parent['fullJourneyAccepted'] is False and parent['restored']")
        text = replace_once(text, 'completeUiStages=24', 'completeUiStages=12')
        text = replace_once(text, 'memoryStateDumps=8', 'memoryStateDumps=4')
    if name == 'ime':
        text = replace_once(text, "assert parent['passed'] and parent['restored']",
                            "assert parent['passed'] is False and parent['restored']")
        text = replace_once(text, 'allSixNativeTypingActionsReplayed', 'allSelectedCellNativeTypingActionsReplayed')
    if name == 'memory':
        text = replace_once(text, 'all24BoundaryPointsReplayed', 'all12CompleteCellBoundaryPointsReplayed')
    text = replace_once(text, 'result=dict(passed=True,',
                        f'result=dict(passed=True,nativeCompleteQueueAccepted=False,auditedObjectCount={count},')
    for before, after in [
        ('default-release-supported-native-state-ui-and-post-memory-validation', 'complete-cell-state-of-rejected-queue'),
        ('current-default-ui-physical-content-and-both-owned-controls', 'selected-cell-ui-of-rejected-queue'),
        ('current-complete-default-two-size-serial-memory-trajectories', 'complete-cell-memory-of-rejected-queue')]:
        if before in text: text = replace_once(text, before, after)
    if count == 5000:
        if name == 'ime':
            text = replace_once(text, 'assert len(typed)==3', 'assert len(typed)==1')
            text = replace_once(text, 'typingActions=3', 'typingActions=1')
        else:
            text = replace_once(text, "read(J/'independent-current-state-r1.json')", "read(J/'partial-state.json')")
            text = replace_once(text, "evidence=read(p/'default-external/supported-state-evidence.json')",
                                "evidence=dict(stages=read(p/'default-external/phase-progress.json'))")
            text = replace_once(text, 'assert content==6', 'assert content==0')
            text = replace_once(text, 'uiStages=12', 'uiStages=2')
    if count == 100:
        old_loop = next(node for node in old_tree.body if isinstance(node, ast.For)
                        and isinstance(node.target, ast.Name) and node.target.id == 'count')
        new_loop = next(node for node in ast.parse(text).body if isinstance(node, ast.For)
                        and isinstance(node.target, ast.Name) and node.target.id == 'count')
        require(ast.dump(ast.Module(body=old_loop.body, type_ignores=[]))
                == ast.dump(ast.Module(body=new_loop.body, type_ignores=[])),
                'Every original complete-cell inner guard must remain identical')
    return text


def guard_subprocesses(allowed):
    def audit(event, arguments):
        if event == 'subprocess.Popen':
            executable, args, _, _ = arguments
            line = args if isinstance(args, str) else subprocess.list2cmdline(args)
            require(executable is None or Path(executable).resolve() == Path(sys.executable).resolve(),
                    'Only the exact local Python readers may run')
            require(any(line == subprocess.list2cmdline(command) for command in allowed),
                    'SDK commands and additional subprocesses are forbidden in this audit')
    sys.addaudithook(audit)


def failure_reports(native):
    return (read(native / 'result.json'), read(native / 'journey-100/report.json'),
            read(native / 'journey-5000/report.json'),
            [row['name'] for row in read(native / 'journey-5000/default-external/phase-progress.json')])


def load_context():
    original.require_assertions()
    guard_subprocesses([])
    parser = argparse.ArgumentParser()
    parser.add_argument('--context', type=Path, required=True)
    path = original.ordinary(parser.parse_args().context)
    value = read(path)
    require(value['scope'] == SCOPE and value['auditorSha256'] == sha(__file__), 'Explicit current failure-audit scope required')
    require(value['sourceSha256'] == original.binding.sources()
            and value['queueSourceSha256'] == original.sources(), 'Original measured sources changed')
    native, output = original.ordinary(value['native'], directory=True), original.ordinary(value['output'], directory=True)
    require(path.parent == output and not output.is_relative_to(native) and not native.is_relative_to(output),
            'Audit output must be separate from original evidence')
    for name, expected in value['derivedSha256'].items():
        require(sha(output / 'readers' / name) == expected, 'Derived reader changed')
    adapter, units_file = original.ordinary(value['adapterInputs']), original.ordinary(value['unitEvidence'])
    require(value['inputSha256'] == dict(adapterInputs=sha(adapter), unitEvidence=sha(units_file),
                                       nativeResult=sha(native / 'result.json')), 'Original input binding changed')
    units = original.validate_units(units_file)
    loaded = original.binding.load(adapter)
    check_failure(*failure_reports(native))
    return SimpleNamespace(workspace=WORK, native=native, output=output, units=units,
                           units_file=units_file, loaded=loaded, mode='on')


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def audit_partial(ctx):
    import rf312_instrumentation_state as state
    import rf312_journey_memory as trajectory
    from native_journey_contract import PHASES
    p = ctx.native / 'journey-5000'
    report, trace = read(p / 'report.json'), read(p / 'journey-memory/result.json')
    phases = read(p / 'default-external/phase-progress.json')
    require(not (p / 'default-external/supported-state-evidence.json').exists()
            and not (p / 'default-external/measured-workload-window.json').exists()
            and not (p / 'post-journey-memory/result.json').exists(), 'Unaccepted total-window and post-memory evidence must not be invented')
    sdk = read(p / 'commands.json')
    failed = [row for row in sdk if row.get('timedOut')]
    require(len(failed) == 1, 'Exactly the actual failed state SDK operation is required')
    row = failed[0]
    require(row['command'] == ['shell', 'dumpsys', 'activity', 'processes']
            and row['timeoutSeconds'] == 5 and row['exitCode'] is None and row['elapsedSeconds'] >= 5,
            'Original five-second refusal must remain a refusal')
    for key in ['stdout', 'stderr']:
        raw = (p / row[key + 'PartialFile']).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == row[key + 'Sha256'] and raw == b'',
                'Empty timed-out bytes must not be parsed as successful absence')
    proofs = sorted((p / 'instrumentation-state').glob('*.proof.json'))
    ordinals, active = [], []
    for file in proofs:
        ordinal = int(file.name.split('.')[0]); prefix = file.with_name(f'{ordinal:03d}')
        proof, command = read(file), read(prefix.with_suffix('.command.json'))
        raw = prefix.with_suffix('.stdout').read_bytes()
        require(command['command'] == row['command'] and command['exitCode'] == 0
                and prefix.with_suffix('.stderr').read_bytes() == b'' and sha(prefix.with_suffix('.stdout')) == proof['stdoutSha256'],
                'All surviving native state bytes must remain bound')
        parsed = state.parse(raw)
        require(parsed == proof['nativeRows'] and proof['ordinal'] == ordinal, 'Original state parser proof changed')
        expected = {} if file == proofs[0] else {'org.solosoul.rf312observer/.Observer': 'org.solosoul.rf312observer'}
        state.expected(parsed, expected, process_ids=None if not expected else {'org.solosoul.rf312observer': trace['observerProcessId']})
        ordinals.append(ordinal); active.append(len(parsed))
    state_rows = [value for value in sdk if value['command'] == row['command']]
    missing = check_state_prefix(ordinals, active, state_rows)
    for file, command in zip(proofs, state_rows[:-1]):
        require(read(file)['stdoutSha256'] == command['stdoutSha256']
                and 0 < command['timeoutSeconds'] <= 5, 'Saved state and original SDK receipt must correspond in order')
    root = Path(ctx.loaded['runtime']['host']['workspaceRoot'])
    restore = module('failed_prefix_restore', root / 'tauri/scripts/android-home-native-regression.py')
    require(restore.inventory(p / 'private-data-before.tar', require_empty=True)
            == restore.inventory(p / 'private-data-restored.tar', require_empty=True), 'Failed cell private data was not restored')
    require(trace['accepted'] is False and trace['workerStopped'] and trace['workerErrorType'] is None
            and trace['hostTraceFinishedAt'] is None and trace['scheduledPeriodicSlotsAccounted'] is False,
            'Incomplete trajectory coverage must remain unaccepted')
    require(trace['requestedBoundaryPhases'] == trace['boundaryPhases'] == list(PHASES[:2])
            and trace['acknowledgedBoundaryPhases'] == trace['boundaryAcknowledgements'] == [],
            'Unacknowledged captures must not become confirmed boundaries')
    require(trace['refusedPoints'] == trace['missedPeriodicSlots'] == 0, 'This entry audits the observed unfailed capture prefix only')
    memory = module('failed_prefix_canonical_memory', root / 'tauri/scripts/android-process-memory.py')
    require(sha(root / 'tauri/scripts/android-process-memory.py') == ctx.loaded['runtime']['canonicalMemoryReaderSha256'], 'Canonical reader changed')
    links = module('failed_prefix_native_links', ROOT / 'native_links.py')
    def numeric(value):
        if isinstance(value, dict): return {k: numeric(v) for k, v in value.items() if k != 'collectionMs'}
        if isinstance(value, list): return [numeric(v) for v in value]
        return value
    points = [read(file) for file in sorted((p / 'journey-memory/points').glob('*/result.json'))]
    intent = read(p / 'journey-memory/intent.json')
    require(len(points) == trace['points'] <= 192 and points[0]['kind'] == 'prelaunch'
            and points[0]['accepted'] and trace['intervalSeconds'] == 2 and trace['maxPoints'] == 192
            and intent['perPointMaximumSeconds'] == 20 and intent['perCommandMaximumSeconds'] == 10
            and intent['originalUiDeadline'] - trace['hostTraceStartedAt'] <= 240, 'Original point, clock and command budgets required')
    source = ast.parse((ROOT / 'replay_memory.py').read_text(encoding='utf-8'))
    loop = next(node for node in ast.walk(source) if isinstance(node, ast.For)
                and isinstance(node.target, ast.Tuple) and [n.id for n in node.target.elts] == ['ordinal', 'point'])
    loop = copy.deepcopy(loop)
    require(isinstance(loop.body[-1], ast.If)
            and ast.unparse(loop.body[-1].test) == "point['kind'] == 'boundary'", 'Original memory loop shape changed')
    unchanged = ast.dump(ast.Module(body=loop.body[:-1], type_ignores=[]))
    loop.body[-1] = ast.parse("if point['kind']=='boundary': audit_unacknowledged(point)").body[0]
    require(ast.dump(ast.Module(body=loop.body[:-1], type_ignores=[])) == unchanged,
            'Every original memory byte, UID, census, state and numerical guard must remain')
    boundary_names = []
    def audit_unacknowledged(point):
        index = len(boundary_names) + 1
        marker = read(p / 'journey-memory' / f'boundary-{index:02d}.request.json')
        require(marker['phase'] == point['phase'] == PHASES[index - 1] and marker['ordinal'] == index
                and marker['uiWaitsForCapture'] is False, 'Original boundary request required')
        require(marker['hostRequestedAt'] <= point['hostStartedAt'] <= point['hostFinishedAt']
                <= marker['captureDeadline'] <= intent['originalUiDeadline']
                and 0 < marker['captureDeadline'] - marker['hostRequestedAt'] <= 45,
                'Original unacknowledged capture deadline required')
        require(not (p / 'journey-memory' / f'boundary-{index:02d}.ack.json').exists(), 'Unexpected boundary confirmation')
        boundary_names.append(point['phase'])
    namespace = dict(points=points, p=p, trace=trace, intent=intent,
        uid=report['originalInstalledPackages']['main']['uid'],
        sdk=read(p / 'journey-memory-sdk/commands.json'), cursor=0, values=[], boundaries=[], due=[], raws=0,
        physical_summary=[], links=links, memory=memory, state=state, trajectory=trajectory,
        numeric=numeric, read=read, sha=sha, audit_unacknowledged=audit_unacknowledged)
    exec(compile(ast.fix_missing_locations(ast.Module(body=[loop], type_ignores=[])),
                 'original-memory-point-guards-with-explicit-unconfirmed-boundaries', 'exec'), namespace)
    require(namespace['cursor'] == len(namespace['sdk'])
            and len(namespace['values']) == trace['acceptedMemoryPoints'] and boundary_names == list(PHASES[:2]),
            'All actual partial memory captures must be replayed')
    require(namespace['due'] == [trace['hostTraceStartedAt'] + 2 * i for i in range(1, len(namespace['due']) + 1)],
            'Observed scheduled prefix changed')
    maximum = max(namespace['values'], key=lambda point: point['snapshot']['pssBytes'])
    require(maximum['ordinal'] == trace['maximumPointOrdinal']
            and maximum['snapshot']['pssBytes'] == trace['sampledMaximumSerialPssBytes'], 'Partial sampled maximum changed')
    result = dict(passed=True, scope='rejected-5000-state-and-memory-prefix-only', completeUiStages=2,
        nativeStateProofs=len(proofs), failedStateOrdinals=missing, timedOutSdkOperations=1,
        originalStateTimeoutSeconds=5, afterObserverAbsenceProven=False,
        memoryCapturePoints=len(points), validNonPrelaunchCaptures=len(namespace['values']),
        capturedBoundaryPoints=2, acknowledgedBoundaryPoints=0, scheduledCoverageAccepted=False,
        completeTrajectoryAccepted=False, originalCanonicalPointGuardsReused=True,
        allOriginalStateRestored=True, nativeCompleteQueueAccepted=False, fullRF312Accepted=False,
        driverReportSha256=sha(p / 'report.json'), phaseProgressSha256=sha(p / 'default-external/phase-progress.json'),
        sdkLedgerSha256=sha(p / 'commands.json'), traceSha256=sha(p / 'journey-memory/result.json'))
    with (ctx.output / 'partial-state.json').open('x', encoding='utf-8') as stream: json.dump(result, stream, indent=2)
    return result


def main():
    original.require_assertions()
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['native', 'adapter-inputs', 'unit-evidence', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    allowed = []
    guard_subprocesses(allowed)
    native = original.ordinary(args.native, directory=True)
    adapter, units_file = original.ordinary(args.adapter_inputs), original.ordinary(args.unit_evidence)
    output = Path(args.output).absolute()
    require(not output.exists() and not output.is_relative_to(native) and not native.is_relative_to(output),
            'New output separate from native evidence is required')
    units = original.validate_units(units_file); loaded = original.binding.load(adapter)
    check_failure(*failure_reports(native))
    parent = read(native / 'result.json')
    require(parent['relatedUnitEvidenceSha256'] == sha(units_file)
            and parent['queueSourceSha256'] == original.sources(), 'Native source and regression binding changed')
    output.mkdir(exist_ok=False); readers = output / 'readers'; readers.mkdir()
    jobs = [('state', 100), ('ime', 100), ('ui', 100), ('memory', 100), ('ime', 5000), ('ui', 5000)]
    for name, count in jobs:
        (readers / f'{name}-{count}.py').write_text(derive(name, count), encoding='utf-8', newline='\n')
    context = dict(scope=SCOPE, native=str(native), output=str(output), adapterInputs=str(adapter), unitEvidence=str(units_file),
        sourceSha256=original.binding.sources(), queueSourceSha256=original.sources(), auditorSha256=sha(__file__),
        inputSha256=dict(adapterInputs=sha(adapter), unitEvidence=sha(units_file), nativeResult=sha(native / 'result.json')),
        derivedSha256={p.name: sha(p) for p in readers.glob('*.py')})
    context_path = output / 'context.json'
    context_path.write_text(json.dumps(context, indent=2), encoding='utf-8')
    commands = [[sys.executable, '-B', str(readers / f'{name}-{count}.py'), '--context', str(context_path)] for name, count in jobs]
    allowed.extend(commands)
    env = dict(os.environ, PYTHONPATH=os.pathsep.join([str(PERFORMANCE), str(ROOT)]), PYTHONDONTWRITEBYTECODE='1', TEMP=str(output), TMP=str(output))
    result = dict(passed=False, nativeCompleteQueueAccepted=False, fullRF312Accepted=False,
                  sdkCommandsExecuted=0, inputEventsDispatched=0)
    try:
        for index, ((name, count), command) in enumerate(zip(jobs, commands)):
            if index == 4:
                partial = audit_partial(SimpleNamespace(native=native, output=output, loaded=loaded))
            prefix = output / f'{name}-{count}'
            with prefix.with_suffix('.stdout').open('xb') as out, prefix.with_suffix('.stderr').open('xb') as err:
                child = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=out, stderr=err, env=env, check=False)
            require(child.returncode == 0, f'Prefix {name}-{count} audit refused; original bytes preserved')
            for file in ['independent-ime-readiness-r1.json', 'independent-ui-r1.json']:
                p = output / file
                if p.exists(): p.rename(output / f'{count}-{file}')
        complete_state = read(output / 'independent-current-state-r1.json')
        result.update(passed=True, scope=SCOPE, completeCell100Accepted=True,
            completeCell100UiStages=12, completeCell100UiStateChecks=complete_state['uiStateChecks'],
            completeCell100PssBytes=complete_state['cases'][0]['pssBytes'], failedCell5000=partial,
            originalQueueCodeAndReadersUnchanged=True, parentResultSha256=sha(native / 'result.json'),
            auditorSha256=sha(__file__), contextSha256=sha(context_path))
    except Exception as error:
        import traceback
        result.update(errorType=type(error).__name__, error=str(error))
        (output / 'failure.traceback').write_text(traceback.format_exc(), encoding='utf-8')
    with (output / 'result.json').open('x', encoding='utf-8') as stream: json.dump(result, stream, indent=2)
    print(json.dumps(result))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
