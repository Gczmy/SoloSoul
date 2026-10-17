"""Replay existing Android memory evidence; report per-process kernel RSS watermarks."""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
WORK = HERE.parents[1]
APP = 'com.solosoul.app'
PROVIDER = 'com.android.webview'
OBSERVER = 'org.solosoul.rf312observer'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate evidence field')
        result[key] = value
    return result


def decode(raw):
    return json.loads(raw, object_pairs_hook=unique)


def digest_value(value):
    if not isinstance(value, str) or re.fullmatch('[a-f0-9]{64}', value) is None:
        raise ValueError('Exact SHA-256 required')
    return value


def status_values(text):
    text = text.replace('\r\n', '\n')
    if '\r' in text or '\0' in text:
        raise ValueError('Unambiguous native status text required')
    result = {}
    for key in ('VmHWM', 'VmRSS'):
        lines = re.findall(r'^' + key + r':([^\r\n]*)$', text, re.M)
        match = re.fullmatch(r'[ \t]*([0-9]+)[ \t]+kB[ \t]*', lines[0]) if len(lines) == 1 else None
        if match is None or not 0 < int(match[1]) <= (2**63 - 1) // 1024:
            raise ValueError('Missing, duplicate or invalid kernel ' + key)
        result[key] = int(match[1]) * 1024
    if result['VmRSS'] > result['VmHWM']:
        raise ValueError('Kernel RSS exceeds its recorded high-water mark')
    return result


def without_collection_times(value):
    result = copy.deepcopy(value)
    result.pop('collectionMs')
    for row in result['processes']:
        row.pop('collectionMs')
    return result


def replay(directory, expected, memory, state):
    directory = Path(directory)
    if not directory.is_absolute() or not directory.is_dir():
        raise ValueError('An existing absolute checkpoint directory is required')
    names = {'checkpoint': directory / 'result.json', 'commands': directory / 'commands.json',
             'sdkCommands': directory.parent / 'commands.json', 'nativeReport': directory.parent / 'report.json'}
    if set(expected) != set(names):
        raise ValueError('All original checkpoint, SDK and native report hashes are required')
    captured = {}

    def read(path, limit=4 * 1024 * 1024):
        if not path.is_file() or path.is_symlink() or path.stat().st_size > limit:
            raise ValueError('Ordinary bounded evidence files required')
        raw = path.read_bytes()
        captured[path] = raw
        return raw

    documents = {}
    for key, path in names.items():
        raw = read(path)
        if sha(raw) != digest_value(expected[key]):
            raise ValueError('Original evidence digest differs: ' + key)
        documents[key] = decode(raw)
    checkpoint, native = documents['checkpoint'], documents['nativeReport']
    if (checkpoint.get('accepted') is not True or checkpoint.get('readOnly') is not True
            or checkpoint.get('targetInstrumentationInjected') is not False
            or checkpoint.get('uiObserverActive') is not False
            or any(native.get(key) is not True for key in
                   ('passed', 'restored', 'originalApksRestored', 'frozenInputsUnchanged'))
            or native.get('targetInstrumentationInjected') is not False):
        raise ValueError('Accepted original default checkpoint and native restoration required')
    snapshot = checkpoint['snapshot']
    roots = [row for row in snapshot['processes'] if row['role'] == 'app' and row['name'] == APP]
    if len(roots) != 1 or snapshot['state'] != 'complete' or snapshot['fullProcessScopeAccepted'] is not True:
        raise ValueError('Complete original main/renderer scope required')
    root = roots[0]
    commands = documents['commands']
    if not isinstance(commands, list) or not 18 <= len(commands) <= 128:
        raise ValueError('Bounded complete checkpoint command sequence required')
    raw_rows = []
    for ordinal, row in enumerate(commands, 1):
        if (type(row.get('exitCode')) is not int or row['exitCode'] != 0 or row.get('timedOut') is not False
                or not isinstance(row.get('command'), list)):
            raise ValueError('Failed or incomplete checkpoint command cannot be reclassified')
        stdout = read(directory / f'{ordinal:03d}.stdout')
        stderr = read(directory / f'{ordinal:03d}.stderr')
        if stderr:
            raise ValueError('Checkpoint stderr cannot be accepted')
        raw_rows.append(dict(command=row['command'], exitCode=0, stdoutSha256=sha(stdout),
                             stderrSha256=sha(stderr), raw=stdout))
    sdk = documents['sdkCommands']
    if not isinstance(sdk, list) or not 1 <= len(sdk) <= 16384:
        raise ValueError('Bounded original SDK command ledger required')
    matches = [start for start in range(len(sdk) - len(raw_rows) + 1)
               if all(all(sdk[start + offset].get(key) == row[key]
                          for key in ('command', 'exitCode', 'stdoutSha256', 'stderrSha256'))
                      and sdk[start + offset].get('timedOut', False) is False
                      for offset, row in enumerate(raw_rows))]
    if len(matches) != 1:
        raise ValueError('Raw checkpoint bytes need one exact original SDK command sequence')
    prefix = [['shell', 'pidof', APP], ['shell', 'dumpsys', 'activity', 'processes'],
              ['shell', 'pm', 'list', 'packages', OBSERVER], ['shell', 'dumpsys', 'window', 'displays']]
    for boundary in (raw_rows[:4], raw_rows[-4:]):
        actual = [row['command'] for row in boundary]
        legacy = copy.deepcopy(prefix)
        legacy[1][-1] = 'instrumentation'
        if actual not in (prefix, legacy) or boundary[0]['raw'].strip() != str(root['pid']).encode():
            raise ValueError('Original checkpoint boundary or main PID differs')
    position = 4
    by_path = {}
    states = []

    def recorded(*command):
        nonlocal position
        if position >= len(raw_rows) - 4 or raw_rows[position]['command'] != list(command):
            raise ValueError('Canonical memory reader requested a different recorded command')
        ordinal = position + 1
        raw = raw_rows[position]['raw']
        position += 1
        text = raw.decode('utf-8', errors='strict').replace('\r\n', '\n')
        if command == ('shell', 'dumpsys', 'activity', 'processes'):
            rows = state.parse(raw)
            if rows:
                raise ValueError('Actual memory-boundary process dump contains active instrumentation')
            states.append(ordinal)
        if command[:2] == ('shell', 'cat'):
            by_path.setdefault(command[2], []).append(dict(ordinal=ordinal, text=text, sha256=sha(raw)))
        return text

    actual = memory.collect_snapshot(recorded, APP, root['uid'], PROVIDER)
    if (position != len(raw_rows) - 4 or len(states) != 2
            or without_collection_times(actual) != without_collection_times(snapshot)):
        raise ValueError('Original strict snapshot does not replay exactly')
    processes = []
    for process in snapshot['processes']:
        stats = by_path.get(f"/proc/{process['pid']}/stat", [])
        statuses = by_path.get(f"/proc/{process['pid']}/status", [])
        if len(stats) != 2 or len(statuses) != 2:
            raise ValueError('Two original kernel identities per owned process required')
        identities = [memory.kernel_identity(s['text'], t['text'], process) for s, t in zip(stats, statuses)]
        identity = {key: process[key] for key in ('pid', 'uid', 'startTicks')}
        if identities != [identity, identity]:
            raise ValueError('Kernel identity differs from the accepted original process')
        before, after = [status_values(row['text']) for row in statuses]
        if after['VmHWM'] < before['VmHWM']:
            raise ValueError('Kernel high-water mark regressed across the checkpoint')
        processes.append(dict(identity, role=process['role'], name=process['name'],
                              kernelRssHighWaterBytes=after['VmHWM'], highWaterBeforeBytes=before['VmHWM'],
                              rssBeforeBytes=before['VmRSS'], rssAfterBytes=after['VmRSS'],
                              statusOrdinals=[row['ordinal'] for row in statuses],
                              statusSha256=[row['sha256'] for row in statuses]))
    if any(path.read_bytes() != raw for path, raw in captured.items()):
        raise ValueError('Evidence changed during offline analysis')
    return dict(accepted=True, scope='kernel-reported-per-surviving-process-rss-high-water',
                evidenceSha256=expected, originalSdkFirstCommandOrdinal=matches[0] + 1,
                rawCommandsReplayed=len(raw_rows), actualEmptyProcessDumps=states,
                legacyInstrumentationReplyUsedAsAbsence=False, processes=processes,
                newDeviceCommands=0, newNativeRun=False, highWaterResetPerformed=False,
                crossProcessPeakBytes=None, pssPeakAccepted=False, wholeJourneyPeakAccepted=False,
                limitations=['Kernel RSS accounting is approximate',
                             'Per-process lifetime until checkpoint; peak time is unknown',
                             'Exited processes and shared WebView/system services are not covered',
                             'Process high-water values cannot be summed into a concurrent PSS/RSS peak'])


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args(argv)
    if not args.input.is_absolute() or not args.output.is_absolute() or args.output.exists():
        parser.error('Absolute input and new absolute output required')
    raw = args.input.read_bytes()
    config = decode(raw)
    if (set(config) != {'schemaVersion', 'checkpoint', 'evidenceSha256', 'readerSha256'}
            or type(config['schemaVersion']) is not int or config['schemaVersion'] != 1):
        parser.error('Explicit complete offline evidence input required')
    paths = {'memory': WORK / 'tauri/scripts/android-process-memory.py',
             'state': HERE / 'rf312_instrumentation_state.py'}
    expected_sources = {key: sha(path.read_bytes()) for key, path in paths.items()}
    if config['readerSha256'] != expected_sources:
        raise ValueError('Canonical reader sources differ from declared originals')
    result = replay(config['checkpoint'], config['evidenceSha256'],
                    module('highwater_canonical_memory', paths['memory']),
                    module('highwater_canonical_state', paths['state']))
    if args.input.read_bytes() != raw or any(sha(path.read_bytes()) != expected_sources[key] for key, path in paths.items()):
        raise ValueError('Explicit input or canonical reader changed')
    result.update(inputSha256=sha(raw), readerSha256=expected_sources,
                  analyzerSha256=sha(Path(__file__).read_bytes()))
    with args.output.open('x', encoding='utf-8', newline='\n') as output:
        output.write(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ('accepted', 'rawCommandsReplayed', 'newDeviceCommands', 'wholeJourneyPeakAccepted')}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
