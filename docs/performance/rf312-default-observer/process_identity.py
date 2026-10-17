"""Exact PID reader from the canonical Android restore driver."""
import re
import subprocess


def read_process_ids(read, package):
    if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)+', package):
        raise ValueError('Invalid Android package for PID lookup')
    try:
        actual = read('shell', 'pidof', package)
    except subprocess.CalledProcessError as error:
        # Toybox uses exit 1 with empty captured output only for no matches.
        # Device/shell errors must not become evidence that a process exited.
        if error.returncode == 1 and error.stdout in (b'', '') and error.stderr in (b'', ''):
            return ''
        raise
    if not isinstance(actual, str) or not re.fullmatch(r'[1-9]\d*(?:\s+[1-9]\d*)*', actual.strip()):
        raise RuntimeError('Unrecognized Android PID output')
    return actual.strip()
