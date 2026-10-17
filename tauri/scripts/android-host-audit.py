#!/usr/bin/env python3
"""Read-only Windows host cleanup attestation for an isolated Android run."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path, PureWindowsPath
import re
import subprocess
import sys


IDENTITY_FIELDS = {"Name", "ProcessId", "ParentProcessId", "CreationDate", "ExecutablePath", "CommandLine"}
SPEC_FIELDS = {"schemaVersion", "oldAdb", "ownedProcesses", "ownedPorts", "avd"}
TIMESTAMP = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?(?:Z|[+-]\d{2}:\d{2})")
AVD = re.compile(r"SoloSoul_[A-Za-z0-9_.-]{1,96}")
SERVER = re.compile(r"(?:^|\s)-L\s+tcp:(?:localhost|127\.0\.0\.1|\[::1\]):([0-9]+)\s+server\s+nodaemon$", re.I)


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON member")
        result[key] = value
    return result


def _constant(_value):
    raise ValueError("Non-finite JSON value")


def decode_json(raw):
    """Never depend on Windows' active ANSI code page, even without -X utf8."""
    return json.loads(raw.decode("utf-8-sig"), object_pairs_hook=_object, parse_constant=_constant)


def _integer(value, minimum, maximum, label):
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError("Invalid " + label)
    return value


def _text(value, label):
    if not isinstance(value, str) or not value or len(value) > 32768 or "\x00" in value:
        raise ValueError("Invalid " + label)
    value.encode("utf-8")
    return value


def birth(value):
    if not isinstance(value, str) or not TIMESTAMP.fullmatch(value):
        raise ValueError("Expected offset-aware microsecond process birth")
    return datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))


def identity(value):
    if not isinstance(value, dict) or set(value) != IDENTITY_FIELDS:
        raise ValueError("Incomplete process identity")
    _integer(value["ProcessId"], 1, 2**32 - 1, "process ID")
    _integer(value["ParentProcessId"], 0, 2**32 - 1, "parent process ID")
    birth(value["CreationDate"])
    for key in ("Name", "ExecutablePath", "CommandLine"):
        _text(value[key], key)
    if not PureWindowsPath(value["ExecutablePath"]).is_absolute():
        raise ValueError("Expected absolute Windows executable path")
    return value


def validate_spec(value):
    if not isinstance(value, dict) or set(value) != SPEC_FIELDS or type(value["schemaVersion"]) is not int or value["schemaVersion"] != 1:
        raise ValueError("Unsupported expected-run schema")
    old = identity(value["oldAdb"])
    if old["Name"].casefold() != "adb.exe" or PureWindowsPath(old["ExecutablePath"]).name.casefold() != "adb.exe":
        raise ValueError("Protected process must be adb.exe")
    endpoint = SERVER.search(old["CommandLine"])
    if endpoint is None:
        raise ValueError("Protected ADB must be a loopback nodaemon server")
    old_port = _integer(int(endpoint.group(1)), 1, 65535, "protected ADB port")
    owned = value["ownedProcesses"]
    if not isinstance(owned, list) or len(owned) > 128:
        raise ValueError("Invalid owned process list")
    ids = [identity(row)["ProcessId"] for row in owned]
    if len(set(ids)) != len(ids) or old["ProcessId"] in ids:
        raise ValueError("Owned process IDs overlap or include protected ADB")
    ports = value["ownedPorts"]
    if not isinstance(ports, list) or not 1 <= len(ports) <= 16:
        raise ValueError("Expected owned listener ports")
    for port in ports:
        _integer(port, 1, 65535, "owned listener port")
    if len(set(ports)) != len(ports) or old_port in ports:
        raise ValueError("Owned ports overlap or include protected ADB port")
    if not isinstance(value["avd"], str) or not AVD.fullmatch(value["avd"]):
        raise ValueError("Expected dedicated SoloSoul AVD")
    return value


def powershell_command(spec):
    validate_spec(spec)
    # Only validated integer lists and a restricted AVD token enter shell text.
    roots = ",".join(str(row["ProcessId"]) for row in spec["ownedProcesses"])
    ports = ",".join(str(port) for port in spec["ownedPorts"])
    avd = re.escape(spec["avd"])
    old_id = spec["oldAdb"]["ProcessId"]
    return f"""$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$all=@(Get-CimInstance Win32_Process)
$owned=[System.Collections.Generic.HashSet[uint32]]::new()
@({roots}) | ForEach-Object {{ [void]$owned.Add([uint32]$_) }}
do {{
  $changed=$false
  foreach ($row in $all) {{
    if ($owned.Contains([uint32]$row.ParentProcessId) -and $owned.Add([uint32]$row.ProcessId)) {{ $changed=$true }}
  }}
}} while ($changed)
$rows=@($all | Where-Object {{
  $_.ProcessId -eq {old_id} -or $owned.Contains([uint32]$_.ProcessId) -or
  ($_.Name -match '^(emulator|qemu-system-).*' -and $_.CommandLine -match '(?<![A-Za-z0-9_.-]){avd}(?![A-Za-z0-9_.-])')
}} | Select-Object Name,ProcessId,ParentProcessId,CreationDate,ExecutablePath,CommandLine)
$listeners=@(Get-NetTCPConnection -State Listen -ErrorAction Stop | Where-Object {{ $_.LocalPort -in @({ports}) }} |
  Select-Object LocalAddress,LocalPort,OwningProcess)
[pscustomobject]@{{processes=$rows;listeners=$listeners}} | ConvertTo-Json -Depth 4
"""


def validate_snapshot(value):
    if not isinstance(value, dict) or set(value) != {"processes", "listeners"}:
        raise ValueError("Incomplete host snapshot")
    rows, listeners = value["processes"], value["listeners"]
    if not isinstance(rows, list) or not isinstance(listeners, list):
        raise ValueError("Host snapshot arrays required")
    ids = [identity(row)["ProcessId"] for row in rows]
    if len(set(ids)) != len(ids):
        raise ValueError("Duplicate observed process ID")
    for row in listeners:
        if not isinstance(row, dict) or set(row) != {"LocalAddress", "LocalPort", "OwningProcess"}:
            raise ValueError("Incomplete listener identity")
        _text(row["LocalAddress"], "listener address")
        _integer(row["LocalPort"], 1, 65535, "listener port")
        _integer(row["OwningProcess"], 1, 2**32 - 1, "listener owner")
    return value


def same_identity(actual, expected):
    return (all(actual[key] == expected[key] for key in ("Name", "ProcessId", "ParentProcessId", "CommandLine"))
            and PureWindowsPath(actual["ExecutablePath"]) == PureWindowsPath(expected["ExecutablePath"])
            and birth(actual["CreationDate"]) == birth(expected["CreationDate"]))


def audit(spec, snapshot):
    validate_spec(spec)
    validate_snapshot(snapshot)
    reasons = []
    matches = [row for row in snapshot["processes"] if row["ProcessId"] == spec["oldAdb"]["ProcessId"]]
    old_matches = len(matches) == 1 and same_identity(matches[0], spec["oldAdb"])
    if not old_matches:
        reasons.append("protected_old_adb_missing_or_changed")
    owned = {row["ProcessId"]: row for row in spec["ownedProcesses"]}
    others = [row for row in snapshot["processes"] if row["ProcessId"] != spec["oldAdb"]["ProcessId"]]
    if any(row["ProcessId"] in owned and not same_identity(row, owned[row["ProcessId"]]) for row in others):
        # A reused PID cannot attest the old run's descendant cleanup.
        reasons.append("owned_identity_changed_or_pid_reused")
    if others:
        reasons.append("owned_descendants_or_avd_helpers_present")
    if any(row["LocalPort"] not in spec["ownedPorts"] for row in snapshot["listeners"]):
        raise ValueError("Unexpected listener outside declared scope")
    if snapshot["listeners"]:
        reasons.append("owned_ports_still_listening")
    return dict(passed=not reasons, protectedOldAdbIdentityUnchanged=old_matches,
                ownedProcessRows=len(others), ownedListeners=len(snapshot["listeners"]), reasons=reasons,
                snapshotEncoding="utf-8-sig", readOnly=True)


def run_snapshot(powershell, spec):
    return subprocess.run([str(powershell), "-NoProfile", "-NonInteractive", "-Command", powershell_command(spec)],
                          stdin=subprocess.DEVNULL, capture_output=True, timeout=15,
                          creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))


def is_windows():
    return os.name == "nt"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--powershell", type=Path, required=True, help="PowerShell 7 pwsh executable")
    parser.add_argument("--expected", type=Path, required=True, help="Immutable before-run identity specification")
    parser.add_argument("--output", type=Path, required=True, help="New local evidence directory; never overwritten")
    args = parser.parse_args(argv)
    try:
        if not is_windows():
            raise ValueError("Windows CIM host required")
        powershell = args.powershell.resolve(strict=True)
        if not powershell.is_file() or powershell.name.casefold() not in {"pwsh.exe", "pwsh"}:
            raise ValueError("Expected PowerShell 7 executable")
        original = args.expected.read_bytes()
        spec = validate_spec(decode_json(original))
        args.output.mkdir(parents=True, exist_ok=False)
    except (OSError, ValueError, UnicodeError):
        print("Invalid host-audit inputs or output already exists", file=sys.stderr)
        return 2
    result = dict(passed=False, readOnly=True)
    stdout, stderr = b"", b""
    try:
        completed = run_snapshot(powershell, spec)
        stdout, stderr = completed.stdout, completed.stderr
        if completed.returncode != 0:
            raise ValueError("Host snapshot command failed")
        if args.expected.read_bytes() != original:
            raise ValueError("Expected identities changed during collection")
        result = audit(spec, decode_json(stdout))
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b"", error.stderr or b""
        result["error"] = "Host snapshot exceeded its 15-second budget"
    except (OSError, ValueError, UnicodeError) as error:
        result["error"] = str(error)
    result["expectedSpecSha256"] = hashlib.sha256(original).hexdigest()
    result["snapshotSha256"] = hashlib.sha256(stdout).hexdigest()
    try:
        (args.output / "host-snapshot.json").write_bytes(stdout)
        (args.output / "host-snapshot.stderr").write_bytes(stderr)
        encoded = json.dumps(result, indent=2) + "\n"
        (args.output / "result.json").write_text(encoded, encoding="utf-8")
    except OSError:
        print("Could not preserve host-audit evidence", file=sys.stderr)
        return 1
    print(json.dumps(result))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
