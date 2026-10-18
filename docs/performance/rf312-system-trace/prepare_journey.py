"""Bind two whole-journey captures to existing declared runtime/state inputs.

This prepares files only. It never starts an SDK, installs an app or drives UI.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import uuid

import trace_binding
import trace_session

ROOT = Path(__file__).resolve().parent
PUBLIC = ROOT.parent


def load_state(path):
    sys.path.insert(0, str(PUBLIC))
    import rf312_default_binding
    return rf312_default_binding.load(path)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write('\n')


def prepare(runtime, adapter, powershell, output, *, profile='default'):
    if type(profile) is not str or profile not in trace_session.PROFILES:
        raise ValueError('Unknown explicit trace profile')
    if sys.flags.optimize:
        raise ValueError('Assertions must remain enabled for the original owner')
    runtime, adapter, powershell, output = (Path(p).absolute() for p in (runtime, adapter, powershell, output))
    for path in (runtime, adapter, powershell):
        if not path.is_file() or path.is_symlink():
            raise ValueError('Ordinary declared input files required')
    if output.exists() or output.is_symlink() or not output.parent.is_dir():
        raise ValueError('New output directory under an existing parent required')
    loaded = load_state(adapter)
    if (not loaded.get('ownedUi') or Path(loaded['config']['runtimeInputs']).resolve() != runtime.resolve()
            or loaded['runtime'] != json.loads(runtime.read_bytes())):
        raise ValueError('Exact runtime and explicit owned UI adapter required')
    base = ROOT / 'base-config.pbtxt'
    if sha(base) != trace_session.BASE_HASH:
        raise ValueError('Declared base configuration changed')
    sources = {str(p): sha(p) for p in sorted(ROOT.glob('*.py'))}
    originals = {str(PUBLIC / name): sha(PUBLIC / name) for name in (
        'rf312_default_state_driver.py', 'rf312_default_journey.py', 'rf312-native-queue/owner.py')}
    frozen = {str(p): sha(p) for p in (runtime, adapter, powershell)}
    output.mkdir(exist_ok=False)
    cases = {}
    for count in (100, 5000):
        folder = output / f'inputs-{count}'
        folder.mkdir()
        nonce = uuid.uuid4().hex
        write_json(folder / 'file-plan.json', {'nonce': nonce})
        (folder / 'base-config.pbtxt').write_bytes(base.read_bytes())
        (folder / 'roundtrip.bin').write_bytes(trace_session.transport.ROUNDTRIP)
        remote = trace_session.transport.remote_paths(nonce)[2]
        (folder / 'trace-config.pbtxt').write_bytes(trace_session.config(base.read_bytes(), remote, profile=profile))
        cases[str(count)] = dict(root=str(folder), files={p.name: sha(p) for p in sorted(folder.iterdir())})
    for path, digest in {**sources, **originals, **frozen}.items():
        if sha(path) != digest:
            raise ValueError('Input changed during preparation; partial output retained')
    plan = dict(schemaVersion=1, adapterInputs=str(adapter), adapterInputsSha256=sha(adapter),
                runtimeInputs=str(runtime), runtimeInputsSha256=sha(runtime), sources=sources,
                originalSources=originals, inputCases=cases,
                powershell=str(powershell), powershellSha256=sha(powershell))
    if profile != 'default':
        plan.update(schemaVersion=2, traceProfile=profile)
    trace_binding.verify(plan)
    destination = output / 'trace-plan.json'
    write_json(destination, plan)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('runtime-inputs', 'adapter-inputs', 'powershell', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--profile', choices=trace_session.PROFILES, default='default')
    args = parser.parse_args()
    try:
        destination = prepare(args.runtime_inputs, args.adapter_inputs, args.powershell, args.output, profile=args.profile)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(json.dumps(dict(prepared=True, tracePlan=str(destination), sdkStarted=False)))


if __name__ == '__main__':
    main()
