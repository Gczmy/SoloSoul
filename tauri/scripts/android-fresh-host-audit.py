#!/usr/bin/env python3
"""Read-only Windows before/after attestation for an initially empty Android host."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

_path = Path(__file__).with_name('android-host-audit.py')
_spec = importlib.util.spec_from_file_location('android_host_identity', _path)
identity = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(identity)
FIELDS = {'schemaVersion', 'baseline', 'ownedProcesses', 'protectedPorts', 'ownedPorts', 'avd'}
BASELINE = 'no-existing-adb-or-emulator'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def validate_spec(value, phase):
    if (not isinstance(value, dict) or set(value) != FIELDS
            or type(value['schemaVersion']) is not int or value['schemaVersion'] != 1
            or value['baseline'] != BASELINE or phase not in ('before', 'after')):
        raise ValueError('Explicit fresh-host schema and phase required')
    if value['protectedPorts'] != [5037, 5049] or any(type(p) is not int for p in value['protectedPorts']):
        raise ValueError('Both original protected ports must be declared')
    ports = value['ownedPorts']
    if not isinstance(ports, list) or not 1 <= len(ports) <= 16:
        raise ValueError('Explicit bounded owned port list required')
    for port in ports:
        identity._integer(port, 1, 65535, 'owned port')
    if len(set(ports)) != len(ports) or set(ports) & set(value['protectedPorts']):
        raise ValueError('Owned ports overlap or include protected ports')
    rows = value['ownedProcesses']
    if not isinstance(rows, list) or len(rows) > 128:
        raise ValueError('Bounded complete owned identities required')
    ids = [identity.identity(row)['ProcessId'] for row in rows]
    if len(ids) != len(set(ids)) or (phase == 'before' and rows) or (phase == 'after' and not rows):
        raise ValueError('Before must be empty; after requires distinct recorded identities')
    if not isinstance(value['avd'], str) or not identity.AVD.fullmatch(value['avd']):
        raise ValueError('Explicit dedicated SoloSoul AVD required')
    return value


def audit(spec, snapshot, phase):
    validate_spec(spec, phase)
    identity.validate_snapshot(snapshot)
    ports = spec['protectedPorts'] + spec['ownedPorts']
    if any(row['LocalPort'] not in ports for row in snapshot['listeners']):
        raise ValueError('Unexpected listener outside declared scope')
    owners = {row['ProcessId']: row for row in spec['ownedProcesses']}
    reasons = []
    if any(row['ProcessId'] in owners and not identity.same_identity(row, owners[row['ProcessId']])
           for row in snapshot['processes']):
        reasons.append('owned_identity_changed_or_pid_reused')
    if snapshot['processes']:
        reasons.append('adb_vm_or_owned_helpers_present')
    if snapshot['listeners']:
        reasons.append('protected_or_owned_ports_listening')
    return dict(passed=not reasons, baseline=BASELINE, phase=phase, readOnly=True,
                ownedProcessRows=len(snapshot['processes']), listenerRows=len(snapshot['listeners']), reasons=reasons)


def powershell_command(spec, phase):
    validate_spec(spec, phase)
    roots = ','.join(str(row['ProcessId']) for row in spec['ownedProcesses'])
    ports = ','.join(str(port) for port in spec['protectedPorts'] + spec['ownedPorts'])
    return f"""$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$all=@(Get-CimInstance Win32_Process)
$owned=[Collections.Generic.HashSet[uint32]]::new()
@({roots}) | ForEach-Object {{ [void]$owned.Add([uint32]$_) }}
do {{
  $changed=$false
  foreach($row in $all) {{
    if($owned.Contains([uint32]$row.ParentProcessId) -and $owned.Add([uint32]$row.ProcessId)) {{ $changed=$true }}
  }}
}} while($changed)
$rows=@($all | Where-Object {{ $_.Name -eq 'adb.exe' -or $_.Name -eq 'emulator.exe' -or $_.Name -like 'qemu-system-*' -or $owned.Contains([uint32]$_.ProcessId) }} |
  Select-Object Name,ProcessId,ParentProcessId,CreationDate,ExecutablePath,CommandLine)
$listeners=@(Get-NetTCPConnection -State Listen -ErrorAction Stop | Where-Object {{ $_.LocalPort -in @({ports}) }} |
  Select-Object LocalAddress,LocalPort,OwningProcess)
[pscustomobject]@{{processes=$rows;listeners=$listeners}} | ConvertTo-Json -Depth 4
"""


def run_snapshot(powershell, spec, phase):
    return subprocess.run([str(powershell), '-NoProfile', '-NonInteractive', '-Command', powershell_command(spec, phase)],
                          stdin=subprocess.DEVNULL, capture_output=True, timeout=15,
                          creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))


def baseline_files(folder, current):
    paths = [folder / name for name in ('expected.json', 'host-snapshot.json', 'result.json')]
    contents = {p: p.read_bytes() for p in paths}
    before = validate_spec(identity.decode_json(contents[paths[0]]), 'before')
    result = identity.decode_json(contents[paths[2]])
    if not isinstance(result, dict):
        raise ValueError('Complete original before result required')
    replay = audit(before, identity.decode_json(contents[paths[1]]), 'before')
    if (not replay['passed'] or any(result.get(key) != value for key, value in replay.items())
            or result.get('expectedSpecSha256') != sha(contents[paths[0]])
            or result.get('snapshotSha256') != sha(contents[paths[1]])):
        raise ValueError('Successful original before-run evidence required')
    if any(current[key] != before[key] for key in FIELDS - {'ownedProcesses'}):
        raise ValueError('Before/after AVD and port scope must be identical')
    return contents


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=('before', 'after'))
    parser.add_argument('--powershell', type=Path, required=True)
    parser.add_argument('--expected', type=Path, required=True)
    parser.add_argument('--baseline', type=Path, help='Original successful before output; required for after')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if not identity.is_windows():
            raise ValueError('Windows CIM host required')
        powershell = args.powershell.resolve(strict=True)
        if not powershell.is_file() or powershell.name.casefold() not in ('pwsh.exe', 'pwsh'):
            raise ValueError('PowerShell 7 executable required')
        if (args.phase == 'after') != (args.baseline is not None):
            raise ValueError('After requires original baseline; before forbids one')
        original = args.expected.read_bytes()
        spec = validate_spec(identity.decode_json(original), args.phase)
        bound = baseline_files(args.baseline, spec) if args.baseline else {}
        args.output.mkdir(parents=True, exist_ok=False)
    except (OSError, ValueError, UnicodeError):
        print('Invalid fresh-host inputs, baseline or existing output', file=sys.stderr)
        return 2
    result = dict(passed=False, baseline=BASELINE, phase=args.phase, readOnly=True)
    stdout, stderr = b'', b''
    try:
        completed = run_snapshot(powershell, spec, args.phase)
        stdout, stderr = completed.stdout, completed.stderr
        if completed.returncode:
            raise ValueError('Host snapshot command failed')
        if args.expected.read_bytes() != original or any(p.read_bytes() != raw for p, raw in bound.items()):
            raise ValueError('Before-run evidence or expected identities changed during collection')
        result = audit(spec, identity.decode_json(stdout), args.phase)
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b'', error.stderr or b''
        result['error'] = 'Host snapshot exceeded its 15-second budget'
    except (OSError, ValueError, UnicodeError) as error:
        result['error'] = str(error)
    result.update(expectedSpecSha256=sha(original), snapshotSha256=sha(stdout))
    if bound:
        result['baselineFilesSha256'] = {p.name: sha(raw) for p, raw in bound.items()}
    try:
        (args.output / 'expected.json').write_bytes(original)
        (args.output / 'host-snapshot.json').write_bytes(stdout)
        (args.output / 'host-snapshot.stderr').write_bytes(stderr)
        (args.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    except OSError:
        print('Could not preserve fresh-host evidence', file=sys.stderr)
        return 1
    print(json.dumps(result))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
