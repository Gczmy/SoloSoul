"""RF-312: read-only memory of an app UID and its bound isolated renderers.

API31 text format is checked strictly. This does not include shared WebView
services/zygotes, and serial PSS reads are not an atomic or continuous peak.
"""
import argparse
from pathlib import Path
import json
import re
import subprocess
import time


class MemoryEvidenceError(RuntimeError):
    pass


RECORD = r"ProcessRecord\{([a-fA-F0-9]+) ([1-9]\d*):([^/\s}]+)/([^\s}]+)\}"
PACKAGE = r"[a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)+"


def process_records(text):
    result = {}
    for match in re.finditer(r"^\s*\*APP\* UID (\d+) (" + RECORD + r")\s*$", text, re.M):
        uid, record, token, pid, name, label = match.groups()
        pid = int(pid)
        if pid in result:
            raise MemoryEvidenceError('Duplicate process identity')
        result[pid] = dict(pid=pid, uid=int(uid), name=name, label=label, record=record)
    if not result:
        raise MemoryEvidenceError('Unrecognized activity process registry')
    return result


def validate_target(package, uid, provider):
    if not re.fullmatch(PACKAGE, package) or not re.fullmatch(PACKAGE, provider) or not 10000 <= uid < 20000:
        raise MemoryEvidenceError('Only explicit user-zero app/package identities are supported')


def owned_processes(processes, services, package, uid, provider):
    validate_target(package, uid, provider)
    registry = process_records(processes)
    own = {pid: dict(row, role='app') for pid, row in registry.items() if row['uid'] == uid}
    if any(row['name'] != package and not row['name'].startswith(package + ':') for row in own.values()):
        raise MemoryEvidenceError('Unexpected process sharing the app UID')
    if any(row['label'] != 'u0a' + str(uid - 10000) for row in own.values()):
        raise MemoryEvidenceError('App process user label differs from its UID')
    roots = [row for row in own.values() if row['name'] == package]
    if own and len(roots) != 1:
        raise MemoryEvidenceError('Missing or ambiguous main process')
    if not services.startswith('ACTIVITY MANAGER SERVICES (dumpsys activity services)'):
        raise MemoryEvidenceError('Unrecognized service registry')
    root_record = roots[0]['record'] if roots else None
    blocks = re.split(r"(?=^  \* ServiceRecord\{)", services.split('  Connection bindings to services:')[0], flags=re.M)
    for block in blocks:
        header = re.match(r"^  \* ServiceRecord\{[^\s}]+ u0 ([^\s}]+)\}\s*$", block, re.M)
        if not header:
            continue
        component = header.group(1)
        if not component.startswith(package + '/org.chromium.content.app.SandboxedProcessService'):
            continue
        app = re.findall(r"^    app=(" + RECORD + r")\s*$", block, re.M)
        isolated = re.findall(r"^    isolatedProc=(" + RECORD + r")\s*$", block, re.M)
        clients = re.findall(r"^      \* Client AppBindRecord\{[^\s}]+ (" + RECORD + r")\}\s*$", block, re.M)
        if (len(app) != 1 or isolated != app or not clients or
                any(row[0] != root_record for row in clients) or
                re.findall(r"^    recentCallingUid=(\d+)\s*$", block, re.M) != [str(uid)] or
                re.findall(r"^    recentCallingPackage=(\S+)\s*$", block, re.M) != [package]):
            raise MemoryEvidenceError('Renderer service has no unique owned client binding')
        record, _, pid, name, label = app[0]
        pid = int(pid)
        row = registry.get(pid)
        if (not row or row['record'] != record or row['uid'] < 99000 or row['uid'] > 99999 or
                not name.startswith(provider + ':sandboxed_process') or
                not re.fullmatch(r'u0a' + str(uid - 10000) + r'i\d+', label) or pid in own):
            raise MemoryEvidenceError('Renderer process identity is missing or inconsistent')
        own[pid] = dict(row, role='renderer')
    expected_isolated = {pid for pid, row in registry.items()
                         if re.fullmatch(r'u0a' + str(uid - 10000) + r'i\d+', row['label'])}
    if expected_isolated != {pid for pid, row in own.items() if row['role'] == 'renderer'}:
        raise MemoryEvidenceError('App isolated process is not covered by a service binding')
    return sorted(own.values(), key=lambda row: row['pid'])


def kernel_identity(stat, status, expected):
    match = re.fullmatch(r"([1-9]\d*) \((.*)\) (\S+(?: \S+)*)\s*", stat.strip())
    if not match or int(match.group(1)) != expected['pid']:
        raise MemoryEvidenceError('Kernel PID identity is invalid')
    fields = match.group(3).split()
    uids = re.findall(r'^Uid:\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s*$', status, re.M)
    if len(fields) < 20 or not fields[19].isdigit() or int(fields[19]) < 1:
        raise MemoryEvidenceError('Kernel process birth identity is missing')
    if len(uids) != 1 or set(map(int, uids[0])) != {expected['uid']}:
        raise MemoryEvidenceError('Kernel UID differs from the bound process')
    return dict(pid=expected['pid'], uid=expected['uid'], startTicks=int(fields[19]))


def memory_values(text, expected):
    headers = re.findall(r'^\*\* MEMINFO in pid (\d+) \[([^\]]+)\] \*\*\s*$', text, re.M)
    totals = re.findall(r'TOTAL PSS:\s+(\d+)\s+TOTAL RSS:\s+(\d+)\s+TOTAL SWAP \(KB\):\s+(\d+)', text)
    if headers != [(str(expected['pid']), expected['name'])] or len(totals) != 1:
        raise MemoryEvidenceError('Memory summary is incomplete or belongs to another process')
    pss, rss, swap = map(int, totals[0])
    if pss < 1 or rss < pss:
        raise MemoryEvidenceError('Memory totals are invalid')
    return dict(pssBytes=pss * 1024, rssBytes=rss * 1024, swapBytes=swap * 1024)


def collect_snapshot(run, package, uid, provider):
    validate_target(package, uid, provider)
    began = time.monotonic()
    processes = run('shell', 'dumpsys', 'activity', 'processes')
    services = run('shell', 'dumpsys', 'activity', 'services', package)
    before = owned_processes(processes, services, package, uid, provider)
    rows = []
    for expected in before:
        pid = str(expected['pid'])
        started = time.monotonic()
        identity = kernel_identity(run('shell', 'cat', '/proc/' + pid + '/stat'),
                                   run('shell', 'cat', '/proc/' + pid + '/status'), expected)
        # Never invoke the target's IApplicationThread.dumpMemInfo: old WebView
        # sandbox processes can crash in that binder handler. --local keeps the
        # dump in system_server; all numbers still come from the actual kernel.
        values = memory_values(run('shell', 'dumpsys', 'meminfo', '--local', '-s', pid), expected)
        after_identity = kernel_identity(run('shell', 'cat', '/proc/' + pid + '/stat'),
                                         run('shell', 'cat', '/proc/' + pid + '/status'), expected)
        if identity != after_identity:
            raise MemoryEvidenceError('Process identity changed during memory collection')
        rows.append(dict(expected, **identity, **values, collectionMs=(time.monotonic() - started) * 1000))
    after = owned_processes(run('shell', 'dumpsys', 'activity', 'processes'),
                            run('shell', 'dumpsys', 'activity', 'services', package), package, uid, provider)
    if before != after:
        raise MemoryEvidenceError('Owned process set or service binding changed during collection')
    renderers = [row for row in rows if row['role'] == 'renderer']
    return dict(schemaVersion=1, scope='app-uid-and-bound-isolated-renderers',
                state='not-running' if not rows else ('complete' if renderers else 'main-only-before-renderer'),
                fullProcessScopeAccepted=bool(renderers), processes=rows,
                pssBytes=sum(row['pssBytes'] for row in rows) if renderers else None,
                collectionMs=(time.monotonic() - began) * 1000,
                excludes=['shared-webview-services', 'zygotes', 'system-processes'],
                definition='serial PSS reads; includes test runner in app; not atomic, continuous peak or uninstrumented performance')


class AdbRunner:
    def __init__(self, executable, serial, avd, output):
        if not re.fullmatch(r'emulator-\d+', serial) or not re.fullmatch(r'SoloSoul_[A-Za-z0-9_-]+', avd):
            raise MemoryEvidenceError('An explicit dedicated SoloSoul emulator is required')
        self.prefix = [str(Path(executable).resolve(strict=True)), '-H', 'localhost', '-P', '5049', '-s', serial]
        self.output = Path(output)
        self.output.mkdir(parents=False, exist_ok=False)
        self.commands = []
        if self('emu', 'avd', 'name').splitlines()[0] != avd:
            raise MemoryEvidenceError('Dedicated emulator identity differs')

    def __call__(self, *command):
        began = time.monotonic()
        result = subprocess.run(self.prefix + list(command), stdin=subprocess.DEVNULL,
                                capture_output=True, timeout=10, check=False)
        ordinal = len(self.commands) + 1
        (self.output / f'{ordinal:03d}.stdout.txt').write_bytes(result.stdout)
        (self.output / f'{ordinal:03d}.stderr.txt').write_bytes(result.stderr)
        self.commands.append(dict(command=list(command), exitCode=result.returncode,
                                 elapsedMs=(time.monotonic() - began) * 1000))
        (self.output / 'commands.json').write_text(json.dumps(self.commands, indent=2) + '\n', encoding='utf-8')
        if result.returncode:
            raise MemoryEvidenceError('Read-only Android diagnostic command failed')
        return result.stdout.decode('utf-8', errors='strict').replace('\r\n', '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['adb', 'serial', 'avd', 'package', 'provider', 'output']:
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--uid', required=True, type=int)
    args = parser.parse_args()
    run = AdbRunner(args.adb, args.serial, args.avd, args.output)
    try:
        result = collect_snapshot(run, args.package, args.uid, args.provider)
    except Exception as error:
        (run.output / 'result.json').write_text(json.dumps({'accepted': False, 'error': str(error)}) + '\n', encoding='utf-8')
        raise
    (run.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result))
    return 0 if result['fullProcessScopeAccepted'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
