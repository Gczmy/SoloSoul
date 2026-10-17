"""Read-only sampled quiescence before installing into a dedicated emulator."""
import math
import time


def wait_for_quiet(snapshot, server, emulator, deadline, record, *, clock=time.monotonic, sleep=time.sleep):
    """Keep the caller's boot deadline and exact roots; never accept a helper."""
    if (type(deadline) not in (int, float) or not math.isfinite(deadline)
            or not 0 < deadline - clock() <= 240):
        raise ValueError('Existing finite boot deadline required')
    fields = {'Name', 'ProcessId', 'ParentProcessId', 'CreationDate', 'ExecutablePath', 'CommandLine'}
    for expected, name in [(server, 'adb.exe'), (emulator, 'emulator.exe')]:
        if (not isinstance(expected, dict) or expected.get('Name') != name
                or set(expected) != fields or type(expected.get('ParentProcessId')) is not int
                or expected['ParentProcessId'] < 0
                or type(expected.get('ProcessId')) is not int or expected['ProcessId'] <= 0
                or not all(expected.get(k) for k in ('ExecutablePath', 'CommandLine', 'CreationDate'))):
            raise ValueError('Previously verified complete owned root required')
    if server['ProcessId'] == emulator['ProcessId']:
        raise ValueError('Distinct owned roots required')
    since = None
    samples = 0
    ordinal = 0

    def remaining():
        value = deadline - clock()
        if value <= 0:
            raise TimeoutError('Original boot deadline exhausted before sampled quiet window')
        return value

    while True:
        rows = snapshot(timeout=min(15, remaining()))
        remaining()
        if not isinstance(rows, list) or any(not isinstance(row, dict) for row in rows):
            raise ValueError('Complete current host rows required')
        for expected in (server, emulator):
            actual = [row for row in rows if row.get('ProcessId') == expected['ProcessId']]
            if actual != [expected]:
                raise RuntimeError('Owned root missing, replaced or ambiguous during bootstrap')
        helpers = [row for row in rows if row.get('Name') == 'adb.exe'
                   and row.get('ProcessId') != server['ProcessId']]
        observed = clock()
        if helpers:
            since = None
            samples = 0
        else:
            if since is None:
                since = observed
            samples += 1
        ordinal += 1
        elapsed = 0 if since is None else observed - since
        accepted = not helpers and samples >= 2 and elapsed >= 5
        record(ordinal, dict(helpers=helpers, sampledQuietSeconds=elapsed,
                             consecutiveQuietSamples=samples, accepted=accepted,
                             helperOwnershipGranted=False))
        remaining()
        if accepted:
            return rows
        sleep(min(.5, remaining()))
