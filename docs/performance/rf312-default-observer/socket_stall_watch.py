"""Passive host evidence capture; never changes the observed worker's result."""

import hashlib
import json
from pathlib import Path
import subprocess
import time


def windows_capture(powershell, server_pid, worker_pid, *, runner=subprocess.run):
    """Return a bounded, read-only capture callback; no ADB command is issued."""
    if any(type(value) is not int or value <= 0 for value in (server_pid, worker_pid)):
        raise ValueError("Actual positive process identities required")
    command = """$ErrorActionPreference='Stop'
$processes=@(Get-CimInstance Win32_Process | Where-Object Name -eq 'adb.exe' | Select-Object Name,ProcessId,ParentProcessId,CreationDate,ExecutablePath,CommandLine)
$connections=@(Get-NetTCPConnection | Where-Object { $_.LocalPort -eq 5057 -or $_.RemotePort -eq 5057 } | Select-Object LocalAddress,LocalPort,RemoteAddress,RemotePort,@{Name='State';Expression={[string]$_.State}},OwningProcess)
[pscustomobject]@{processes=$processes;connections=$connections;serverPid=SERVER;workerPid=WORKER} | ConvertTo-Json -Depth 5
""".replace("SERVER", str(server_pid)).replace("WORKER", str(worker_pid))

    def capture():
        result = runner(
            [str(powershell), "-NoProfile", "-NonInteractive", "-Command", command],
            stdin=subprocess.DEVNULL, capture_output=True, timeout=10,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
        return result.returncode, result.stdout, result.stderr

    return capture


def wait_for_worker(worker, output, receipts, capture, *, clock=time.monotonic,
                    wall_clock=time.time, sleep=time.sleep):
    """Wait without a new worker deadline and return its real exit code.

    Each receipt is (commands.json, report.json). Finished case reports only
    exclude old receipts from monitoring; only the process handle ends waiting.
    Capture failures remain advisory. Original input/restore/deadline owners
    keep control of the worker, which is never restarted, signalled, or retried.
    """
    output = Path(output)
    summary = dict(enabled=False, captures=0, captureErrors=[], workerResultChanged=False)
    try:
        output.mkdir(exist_ok=False)
        summary["enabled"] = True
    except OSError as error:
        summary["captureErrors"].append(type(error).__name__)
        return worker.wait(), summary
    last_capture = float("-inf")
    try:
        while worker.poll() is None:
            try:
                active = []
                for receipt, report in receipts:
                    receipt, report = Path(receipt), Path(report)
                    if not receipt.is_file():
                        continue
                    if report.is_file():
                        state = json.loads(report.read_text(encoding="utf-8-sig"))
                        if state.get("finishedAt"):
                            continue
                    active.append((receipt.stat().st_mtime, receipt))
                if active:
                    modified, receipt = max(active, key=lambda item: item[0])
                    stale = wall_clock() - modified
                    now = clock()
                    if stale > 1 and now - last_capture >= 5:
                        last_capture = now
                        summary["captures"] += 1
                        ordinal = summary["captures"]
                        metadata = dict(receiptName=receipt.name, receiptStaleSeconds=stale)
                        try:
                            code, stdout, stderr = capture()
                            if (type(code) is not int or not isinstance(stdout, bytes)
                                    or not isinstance(stderr, bytes) or max(len(stdout), len(stderr)) > 2_097_152):
                                raise ValueError("Bounded binary capture result required")
                            for name, value in [("stdout", stdout), ("stderr", stderr)]:
                                (output / f"capture-{ordinal:03d}.{name}").write_bytes(value)
                                metadata[name + "Sha256"] = hashlib.sha256(value).hexdigest()
                            metadata["exitCode"] = code
                            if code:
                                summary["captureErrors"].append(f"capture_exit_{code}")
                        except Exception as error:
                            metadata["errorType"] = type(error).__name__
                            summary["captureErrors"].append(type(error).__name__)
                        (output / f"capture-{ordinal:03d}.json").write_text(
                            json.dumps(metadata, indent=2) + "\n", encoding="utf-8"
                        )
            except Exception as error:
                summary["captureErrors"].append(type(error).__name__)
            sleep(.25)
    except Exception as error:
        summary["captureErrors"].append(type(error).__name__)
    code = worker.wait()
    summary["workerExitCode"] = code
    try:
        (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    except OSError as error:
        summary["captureErrors"].append(type(error).__name__)
    return code, summary
