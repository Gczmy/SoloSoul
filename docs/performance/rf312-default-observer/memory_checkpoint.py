"""Bounded, read-only memory checkpoint after the original UI observer exits."""
from pathlib import Path
import json
import math
import re
import subprocess
import time

APP = 'com.solosoul.app'
OBSERVER = 'org.solosoul.rf312observer'
PROVIDER = 'com.android.webview'


def collect(collector, adb, output, *, uid, pid, timeout=20, clock=time.monotonic):
    """Use the existing strict reader; never replace a refused/incomplete sample."""
    if (type(uid) is not int or not 10000 <= uid < 20000 or type(pid) is not int or pid <= 0
            or type(timeout) not in (int, float) or not math.isfinite(timeout) or not 0 < timeout <= 20):
        raise ValueError('Explicit app identity and at most twenty seconds required')
    output = Path(output)
    output.mkdir(exist_ok=False)
    began = clock()
    deadline = began + timeout
    commands = []

    def remaining():
        value = deadline - clock()
        if not value > 0:
            raise TimeoutError('Read-only memory checkpoint deadline exhausted')
        return min(10, value)

    def read(*command):
        budget = remaining()
        ordinal = len(commands) + 1
        row = dict(command=list(command), timeoutSeconds=budget)
        commands.append(row)
        try:
            result = adb(*command, timeout=budget, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        except subprocess.TimeoutExpired as error:
            stdout, stderr = error.stdout, error.stderr
            row.update(timedOut=True, exitCode=None,
                       stdoutAvailable=stdout is not None, stderrAvailable=stderr is not None)
            if stdout is not None:
                (output / f'{ordinal:03d}.stdout').write_bytes(stdout)
            if stderr is not None:
                (output / f'{ordinal:03d}.stderr').write_bytes(stderr)
            raise
        except subprocess.CalledProcessError as error:
            row.update(timedOut=False, exitCode=error.returncode)
            (output / f'{ordinal:03d}.stdout').write_bytes(error.stdout or b'')
            (output / f'{ordinal:03d}.stderr').write_bytes(error.stderr or b'')
            raise
        else:
            row.update(timedOut=False, exitCode=result.returncode)
            (output / f'{ordinal:03d}.stdout').write_bytes(result.stdout)
            (output / f'{ordinal:03d}.stderr').write_bytes(result.stderr)
            result.check_returncode()
            remaining()
            return result.stdout.decode('utf-8', errors='strict').replace('\r\n', '\n')
        finally:
            (output / 'commands.json').write_text(json.dumps(commands, indent=2) + '\n', encoding='utf-8')

    def identity():
        if read('shell', 'pidof', APP).strip() != str(pid):
            raise RuntimeError('Current main PID differs from completed UI journey')
        instrumentation = read('shell', 'dumpsys', 'activity', 'instrumentation')
        if APP in instrumentation or OBSERVER in instrumentation:
            raise RuntimeError('Memory checkpoint requires both instrumentation sessions absent')
        if read('shell', 'pm', 'list', 'packages', OBSERVER).strip():
            raise RuntimeError('Original observer cleanup must finish before memory sampling')
        if not re.search(r'mCurrentFocus=.*' + re.escape(APP) + '/', read('shell', 'dumpsys', 'window', 'displays')):
            raise RuntimeError('Completed application must remain focused')

    report = dict(accepted=False, scope='post-journey-unlocked-home-memory-checkpoint',
                  targetInstrumentationInjected=False, uiObserverActive=False,
                  readOnly=True, repeatedPerformanceDistributionAccepted=False,
                  wholeJourneyMemoryDistributionAccepted=False, inputRetried=False)
    try:
        identity()
        snapshot = collector(read, APP, uid, PROVIDER)
        roots = [row for row in snapshot['processes'] if row['role'] == 'app' and row['name'] == APP]
        if (snapshot.get('fullProcessScopeAccepted') is not True or snapshot.get('state') != 'complete'
                or len(roots) != 1 or roots[0]['pid'] != pid or roots[0]['uid'] != uid):
            raise RuntimeError('Strict complete main/renderer snapshot required')
        identity()
        remaining()
        report.update(accepted=True, elapsedMs=(clock() - began) * 1000, snapshot=snapshot)
        return report
    except BaseException as error:
        report.update(errorType=type(error).__name__, error=str(error))
        raise
    finally:
        (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
