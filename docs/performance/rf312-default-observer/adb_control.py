"""Record dedicated ADB calls, including failed calls and partial timeout output."""
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import time


class AdbRecorder:
    """Transport receipts only; the caller retains all device and target guards."""

    def __init__(self, executable, serial, output):
        self.executable = str(executable)
        if not re.fullmatch(r'emulator-[0-9]+', serial):
            raise ValueError('An explicit emulator serial is required')
        self.serial = serial
        self.output = Path(output)
        if not self.output.is_dir():
            raise ValueError('An existing caller-owned output directory is required')
        self.commands = []

    def _record(self, row):
        self.commands.append(row)
        temporary = self.output / 'commands.json.tmp'
        temporary.write_text(json.dumps(self.commands, indent=2) + '\n', encoding='utf-8')
        temporary.replace(self.output / 'commands.json')

    def run(self, *command, **kwargs):
        timeout = kwargs.pop('timeout', 240)
        if (isinstance(timeout, bool) or not isinstance(timeout, (int, float))
                or not math.isfinite(timeout) or not 0 < timeout <= 240):
            raise ValueError('A finite positive ADB budget up to 240 seconds is required')
        argv = [self.executable, '-H', '127.0.0.1', '-P', '5057', '-s', self.serial, *command]
        started = time.monotonic()
        row = {'command': list(command), 'timeoutSeconds': timeout}
        try:
            result = subprocess.run(argv, check=False, timeout=timeout, **kwargs)
        except subprocess.TimeoutExpired as error:
            row.update(exitCode=None, timedOut=True, elapsedSeconds=time.monotonic() - started)
            ordinal = len(self.commands) + 1
            try:
                for name, value in [('stdout', error.stdout), ('stderr', error.stderr)]:
                    if isinstance(value, bytes):
                        filename = f'timed-out-command-{ordinal:03d}.{name}.log'
                        (self.output / filename).write_bytes(value)
                        row[name + 'Sha256'] = hashlib.sha256(value).hexdigest()
                        row[name + 'PartialFile'] = filename
                self._record(row)
            except OSError as receipt_error:
                # Disk or receipt failure must not change the transport exception.
                raise error from receipt_error
            raise
        row.update(exitCode=result.returncode, elapsedSeconds=time.monotonic() - started)
        for name in ('stdout', 'stderr'):
            value = getattr(result, name)
            if isinstance(value, bytes):
                row[name + 'Sha256'] = hashlib.sha256(value).hexdigest()
        self._record(row)
        if result.returncode and isinstance(result.stderr, bytes):
            (self.output / f'failed-command-{len(self.commands):03d}.stderr.log').write_bytes(result.stderr)
        result.check_returncode()
        return result
