"""Read-only serial PSS/RSS trajectory under the original UI and phase deadlines."""
from concurrent.futures import Future
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import queue
import re
import subprocess
import threading
import time

import rf312_instrumentation_state as state
import rf312_memory_batches as batches

APP = 'com.solosoul.app'
PROVIDER = 'com.android.webview'
OBSERVER = 'org.solosoul.rf312observer'
INTERVAL = 2.0
MAX_POINTS = 192


def save(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(json.dumps(value, indent=2, allow_nan=False) + '\n')


def census(raw):
    if not isinstance(raw, bytes) or not 0 < len(raw) <= 4 * 1024 * 1024:
        raise ValueError('Bounded complete native UID census required')
    text = raw.decode('utf-8').replace('\r\n', '\n')
    if '\r' in text or '\0' in text:
        raise ValueError('Unambiguous native process text required')
    lines = text.splitlines()
    if not lines or lines[0].split() != ['UID', 'PID', 'PPID', 'NAME'] or len(lines) > 65537:
        raise ValueError('Exact unfiltered ps UID/PID/PPID/NAME columns required')
    result = {}
    for line in lines[1:]:
        fields = line.split(None, 3)
        if (len(fields) != 4 or any(re.fullmatch('[0-9]+', value) is None for value in fields[:3])
                or not fields[3] or len(fields[3].encode()) > 2048 or any(ord(c) < 32 for c in fields[3])):
            raise ValueError('Complete numeric native process identity required')
        uid, pid, parent = map(int, fields[:3])
        if not 0 <= uid <= 2**31 - 1 or not 0 < pid <= 2**31 - 1 or not 0 <= parent <= 2**31 - 1 or pid in result:
            raise ValueError('Distinct bounded native PIDs required')
        result[pid] = dict(pid=pid, uid=uid, parentPid=parent, name=fields[3])
    if not result:
        raise ValueError('Empty UID census cannot prove absence')
    return result


def verify_census(rows, snapshot, uid):
    processes = snapshot['processes']
    expected = {row['pid']: row for row in processes}
    selected_uids = {uid} | {row['uid'] for row in processes}
    actual = {pid: row for pid, row in rows.items() if row['uid'] in selected_uids}
    if set(actual) != set(expected) or any(actual[pid]['uid'] != row['uid'] for pid, row in expected.items()):
        raise RuntimeError('UID census contains missing or unregistered owned processes')
    if any(row['parentPid'] in expected and pid not in expected for pid, row in rows.items()):
        raise RuntimeError('An owned descendant is outside the strict memory scope')


class Stream:
    def __init__(self, sdk, collector, output, *, uid, deadline, phases, clock=time.monotonic):
        remaining = deadline - clock()
        if (type(uid) is not int or not 10000 <= uid < 20000 or type(deadline) not in (int, float)
                or not math.isfinite(deadline) or not 0 < remaining <= 240
                or not isinstance(phases, tuple) or len(phases) != 12 or len(set(phases)) != 12):
            raise ValueError('Original app UID, twelve phases and remaining UI deadline required')
        self.sdk, self.collector, self.uid, self.deadline = sdk, collector, uid, deadline
        self.output, self.clock, self.phases = Path(output), clock, phases
        if not self.output.is_absolute():
            raise ValueError('Absolute fresh output required')
        self.output.mkdir(exist_ok=False)
        (self.output / 'points').mkdir()
        self.requests = queue.Queue(maxsize=len(phases))
        self.stop = threading.Event()
        self.lock, self.boundary_lock = threading.Lock(), threading.Lock()
        self.thread = None
        self.points, self.boundaries, self.acknowledged = [], [], []
        self.requested, self.pending, self.acknowledgements = [], {}, []
        self.expected_pid = self.observed_root = self.observer_pid = None
        self.failure = None
        self.collection_ended = None
        self.next_due = self.origin = None

    def _capture(self, kind, deadline, *, phase=None, due=None):
        if len(self.points) >= MAX_POINTS:
            raise RuntimeError('Declared memory observation cap exhausted')
        ordinal = len(self.points) + 1
        folder = self.output / 'points' / f'{ordinal:03d}'
        folder.mkdir(exist_ok=False)
        began = self.clock()
        deadline = min(deadline, self.deadline, began + 20)
        row = dict(ordinal=ordinal, kind=kind, phase=phase, plannedDueAt=due,
                   hostStartedAt=began, deadline=deadline, accepted=False, commands=[])
        native_states = []
        transport = batches.Adapter(self.sdk, folder / 'native-sdk', self.uid)

        def read(*command):
            remaining = deadline - self.clock()
            if remaining <= 0:
                raise TimeoutError('Original memory/UI deadline exhausted')
            prefix = folder / f"{len(row['commands']) + 1:03d}"
            record = dict(command=list(command), timeoutSeconds=min(10, remaining))
            row['commands'].append(record)
            try:
                value = transport.run(*command, timeout=record['timeoutSeconds'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                code, out, err = value.returncode, value.stdout, value.stderr
                record.update(nativeEvidence=value.nativeEvidence, logicalReplyAvailable=True)
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
                code, out, err = getattr(error, 'returncode', None), error.stdout, error.stderr
                record.update(exitCode=code, timedOut=isinstance(error, subprocess.TimeoutExpired))
                evidence = getattr(error, 'nativeEvidence', None)
                if evidence is not None:
                    record['nativeEvidence'] = evidence
                    record['logicalReplyAvailable'] = (not isinstance(error, subprocess.TimeoutExpired)
                                                       and (evidence['kind'] == 'direct' or getattr(error, 'logicalReplyAvailable', False)))
                    if evidence['kind'] == 'pair' and not getattr(error, 'logicalReplyAvailable', False):
                        record['logicalReplyAvailable'] = False
                        raise
                for name, raw in [('stdout', out), ('stderr', err)]:
                    if raw is not None:
                        prefix.with_suffix('.' + name).write_bytes(raw)
                        record[name + 'Sha256'] = hashlib.sha256(raw).hexdigest()
                raise
            except BaseException as error:
                record.update(errorType=type(error).__name__, logicalReplyAvailable=False)
                if hasattr(error, 'nativeEvidence'):record['nativeEvidence'] = error.nativeEvidence
                raise
            else:
                record.update(exitCode=code, timedOut=False)
                for name, raw in [('stdout', out), ('stderr', err)]:
                    if not isinstance(raw, bytes):
                        raise ValueError('Captured raw native bytes required')
                    prefix.with_suffix('.' + name).write_bytes(raw)
                    record[name + 'Sha256'] = hashlib.sha256(raw).hexdigest()
                if type(code) is not int or code != 0 or err:
                    raise RuntimeError('Native command failure/stderr cannot become a memory value')
                if self.clock() >= deadline:
                    raise TimeoutError('Memory result arrived outside its original deadline')
                if command == ('shell', 'dumpsys', 'activity', 'processes'):
                    proof = state.parse(out)
                    state.expected(proof, {OBSERVER + '/.Observer': OBSERVER},
                                   process_ids=None if self.observer_pid is None else {OBSERVER: self.observer_pid})
                    self.observer_pid = proof[0]['processId']
                    native_states.append(dict(commandOrdinal=len(row['commands']), nativeRows=proof,
                                              stdoutSha256=record['stdoutSha256']))
                return out
            finally:
                (folder / 'commands.json').write_text(json.dumps(row['commands'], indent=2) + '\n', encoding='utf-8')

        try:
            if kind == 'prelaunch':
                package = read('shell', 'pm', 'list', 'packages', '--uid', str(self.uid))
                if package.splitlines() != [f'package:{APP} uid:{self.uid}'.encode()]:
                    raise RuntimeError('App UID must belong to exactly the declared package')
                row['uniqueAppUidVerified'] = True
            before = census(read('shell', 'ps', '-A', '-o', 'UID,PID,PPID,NAME'))
            snapshot = self.collector(lambda *args: read(*args).decode('utf-8').replace('\r\n', '\n'),
                                      APP, self.uid, PROVIDER)
            after = census(read('shell', 'ps', '-A', '-o', 'UID,PID,PPID,NAME'))
            verify_census(before, snapshot, self.uid)
            verify_census(after, snapshot, self.uid)
            if len(native_states) != 2:
                raise RuntimeError('Both actual supported native-state checks required')
            row.update(snapshot=snapshot, stateProofs=native_states, uidCensusScopeVerified=True)
            roots = [value for value in snapshot['processes'] if value['role'] == 'app' and value['name'] == APP]
            if kind == 'prelaunch':
                if roots or snapshot['state'] != 'not-running' or snapshot['pssBytes'] is not None:
                    raise RuntimeError('Prelaunch must prove absent app without a zero-memory value')
            else:
                if len(roots) != 1 or snapshot['state'] != 'complete' or snapshot['fullProcessScopeAccepted'] is not True:
                    raise RuntimeError('Incomplete main/renderer memory point retained as refused')
                root = {key: roots[0][key] for key in ('pid', 'uid', 'startTicks')}
                with self.lock:
                    if ((self.expected_pid is not None and root['pid'] != self.expected_pid)
                            or (self.observed_root is not None and root != self.observed_root)):
                        raise RuntimeError('Original app process lifetime changed')
                    self.observed_root = root
            if self.clock() > deadline:
                raise TimeoutError('Validated memory point exceeded the original deadline')
            transport.require_empty()
            row['accepted'] = True
        except BaseException as error:
            row.update(errorType=type(error).__name__, error=str(error))
        finally:
            row['nativeTransport'] = transport.close()
            row.update(hostFinishedAt=self.clock(), inputRetried=False)
            if row['accepted'] and row['hostFinishedAt'] > deadline:
                row.update(accepted=False,errorType='TimeoutError',error='Transport receipts exceeded original memory/UI deadline')
            save(folder / 'result.json', row)
            self.points.append(row)
        return row

    def start(self):
        if self.thread is not None or self.origin is not None:
            raise RuntimeError('One explicit memory stream start required')
        self.origin = self.clock()
        point = self._capture('prelaunch', min(self.deadline, self.origin + 20))
        if not point['accepted']:
            raise RuntimeError('Prelaunch memory/census admission refused: ' + point.get('error', 'unknown'))
        self.next_due = self.origin + INTERVAL
        save(self.output / 'intent.json', dict(intervalSeconds=INTERVAL, maxPoints=MAX_POINTS,
             originalUiDeadline=self.deadline, perPointMaximumSeconds=20, perCommandMaximumSeconds=10,
             phaseNames=list(self.phases), targetInstrumentationInjected=False, inputEventsSent=0,
             missedSlotsRetained=True, failedPointsRetained=True, periodicFailureRetried=False))
        self.thread = threading.Thread(target=self._run, name='rf312-read-only-memory', daemon=False)
        self.thread.start()

    def bind(self, pid):
        if type(pid) is not int or pid <= 0 or self.thread is None:
            raise ValueError('Started stream and original positive app PID required')
        with self.lock:
            if self.expected_pid is not None or (self.observed_root is not None and self.observed_root['pid'] != pid):
                raise RuntimeError('Original main process binding differs or was repeated')
            self.expected_pid = pid

    def check(self):
        if self.failure is not None:
            raise RuntimeError('Read-only memory worker failed') from self.failure
        if self.thread is not None and not self.thread.is_alive() and self.boundaries != list(self.phases):
            raise RuntimeError('Memory worker stopped before complete boundary capture')

    def submit_boundary(self, phase, deadline, *, ui_waits_for_capture=False):
        self.check()
        if not self.boundary_lock.acquire(blocking=False):
            raise RuntimeError('Concurrent boundary requests refused')
        try:
            if (type(ui_waits_for_capture) is not bool or self.thread is None or self.expected_pid is None or len(self.requested) >= len(self.phases)
                    or phase != self.phases[len(self.requested)] or type(deadline) not in (int, float)
                    or not math.isfinite(deadline) or not 0 < deadline - self.clock() <= 45):
                raise ValueError('Next original phase and remaining phase deadline required')
            answer = Future()
            deadline = min(deadline, self.deadline)
            marker = dict(phase=phase, ordinal=len(self.requested) + 1, hostRequestedAt=self.clock(),
                          captureDeadline=deadline, uiWaitsForCapture=ui_waits_for_capture)
            save(self.output / f"boundary-{marker['ordinal']:02d}.request.json", marker)
            self.requests.put_nowait((phase, deadline, answer))
            self.requested.append(marker)
            self.pending[phase] = (deadline, answer, marker)
            return marker
        finally:
            self.boundary_lock.release()

    def receive_boundary(self, phase, *, require_timely_ack=True):
        self.check()
        if (type(require_timely_ack) is not bool or phase not in self.pending or len(self.acknowledged) >= len(self.phases)
                or phase != self.phases[len(self.acknowledged)]):
            raise ValueError('Each requested boundary must be acknowledged once in original order')
        deadline, answer, marker = self.pending[phase]
        if self.clock() > self.deadline:
            raise TimeoutError('Original UI aggregate deadline reached before boundary acknowledgement')
        try:
            point = answer.result(timeout=max(.001, min(deadline, self.deadline) - self.clock()))
        except TimeoutError as error:
            raise TimeoutError('Memory boundary exceeded original deadline: ' + phase) from error
        if (not point['accepted'] or point['phase'] != phase or point['hostStartedAt'] < marker['hostRequestedAt']
                or point['hostFinishedAt'] > deadline or self.clock() > self.deadline
                or (require_timely_ack and self.clock() > deadline)):
            raise RuntimeError('Memory boundary refused: ' + point.get('error', 'late capture/acknowledgement'))
        path = self.output / 'points' / f"{point['ordinal']:03d}" / 'result.json'
        receipt = dict(pointOrdinal=point['ordinal'], receiptSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                       pssBytes=point['snapshot']['pssBytes'], hostStartedAt=point['hostStartedAt'], hostFinishedAt=point['hostFinishedAt'])
        acknowledgement = dict(phase=phase, hostAcknowledgedAt=self.clock(), captureDeadlineVerified=True,
                               synchronousAckRequired=require_timely_ack, memoryReceipt=receipt)
        save(self.output / f"boundary-{marker['ordinal']:02d}.ack.json", acknowledgement)
        self.acknowledgements.append(acknowledgement)
        self.acknowledged.append(phase)
        return receipt

    def boundary(self, phase, deadline):
        self.submit_boundary(phase, deadline, ui_waits_for_capture=True)
        return self.receive_boundary(phase)

    def complete_boundaries(self):
        if [row['phase'] for row in self.requested] != list(self.phases) or self.acknowledged:
            raise RuntimeError('All twelve original phase requests required before final acknowledgement')
        # Captures keep their original deadlines; only acknowledgement is deferred.
        return {phase: self.receive_boundary(phase, require_timely_ack=False) for phase in self.phases}

    def _run(self):
        active = None
        try:
            while not self.stop.is_set():
                now = self.clock()
                if now >= self.deadline:
                    raise TimeoutError('Original aggregate UI deadline reached by memory worker')
                self._retain_missed(now)
                try:
                    active = self.requests.get_nowait()
                except queue.Empty:
                    try:
                        active = self.requests.get(timeout=min(max(0, self.next_due - now), self.deadline - now))
                    except queue.Empty:
                        due = self.next_due
                        self.next_due += INTERVAL
                        self._capture('periodic', self.deadline, due=due)
                        continue
                if active is None:
                    break
                phase, deadline, answer = active
                point = self._capture('boundary', deadline, phase=phase)
                if point['accepted']:
                    self.boundaries.append(phase)
                if not point['accepted'] or phase == self.phases[-1]:
                    self.collection_ended = self.clock()
                    self._retain_missed(self.collection_ended, final=True)
                answer.set_result(point)
                active = None
                if not point['accepted'] or phase == self.phases[-1]:
                    break
        except BaseException as error:
            self.failure = error
            if active is not None and not active[2].done():
                active[2].set_exception(error)

    def _retain_missed(self, now, *, final=False):
        while self.next_due + (0 if final else INTERVAL) <= now:
            if len(self.points) >= MAX_POINTS:
                raise RuntimeError('Declared memory observation cap exhausted')
            point = dict(ordinal=len(self.points) + 1, kind='missed-periodic', accepted=False,
                         plannedDueAt=self.next_due, reason='collector-busy-at-declared-slot', commands=[], inputRetried=False)
            self.next_due += INTERVAL
            folder = self.output / 'points' / f"{point['ordinal']:03d}"
            folder.mkdir(exist_ok=False)
            save(folder / 'result.json', point)
            self.points.append(point)

    def close(self):
        self.stop.set()
        try:
            self.requests.put_nowait(None)
        except queue.Full:
            pass
        if self.thread is not None:
            self.thread.join(timeout=21)
        alive = self.thread is not None and self.thread.is_alive()
        accepted = (not alive and self.failure is None and self.expected_pid is not None
                    and self.boundaries == self.acknowledged == list(self.phases) and self.points[0]['accepted']
                    and [row['phase'] for row in self.requested] == list(self.phases)
                    and self.collection_ended is not None and self.next_due > self.collection_ended)
        values = [point for point in self.points if point['accepted'] and point['kind'] != 'prelaunch']
        maximum = max(values, key=lambda point: point['snapshot']['pssBytes']) if values else None
        report = dict(accepted=accepted, scope='serial-sampled-app-uid-and-bound-renderer-uids-during-complete-ui-journey',
            intervalSeconds=INTERVAL, maxPoints=MAX_POINTS, points=len(self.points), boundaryPhases=self.boundaries,
            acknowledgedBoundaryPhases=self.acknowledged,
            requestedBoundaryPhases=[row['phase'] for row in self.requested],
            boundaryAcknowledgements=self.acknowledgements,
            hostTraceStartedAt=self.origin, hostTraceFinishedAt=self.collection_ended,
            scheduledPeriodicSlotsAccounted=self.collection_ended is not None and self.next_due > self.collection_ended,
            acceptedMemoryPoints=len(values), refusedPoints=sum(not point['accepted'] for point in self.points),
            missedPeriodicSlots=sum(point['kind'] == 'missed-periodic' for point in self.points),
            sampledMaximumSerialPssBytes=None if maximum is None else maximum['snapshot']['pssBytes'],
            maximumPointOrdinal=None if maximum is None else maximum['ordinal'],
            workerStopped=not alive, workerErrorType=None if self.failure is None else type(self.failure).__name__,
            mainProcessIdentity=self.observed_root, observerProcessId=self.observer_pid,
            targetInstrumentationInjected=False, javaScriptInjected=False, inputEventsSent=0,
            inputRetried=False, completeBoundaryCoverage=self.boundaries == list(self.phases),
            exactContinuousPeakAccepted=False, applicationOnlyLatencyAccepted=False, performanceDistributionAccepted=False,
            includesSamplingAndVerificationCost=True, fullRF312Accepted=False,
            limitations=['Serial values are not atomic or an exact continuous peak',
                         'Every failed point and missed slot remains an explicit gap',
                         'UID census scope excludes shared WebView/zygote/system services',
                         'Boundary captures run after phase markers while UI may continue',
                         'Workload completion includes control, periodic sampling and final acknowledgement cost'])
        save(self.output / 'result.json', report)
        if not accepted:
            raise RuntimeError('Complete bounded memory stream not accepted')
        return report


@contextmanager
def session(enabled, inputs, output, *, uid, deadline, phases):
    if type(enabled) is not bool:
        raise ValueError('Explicit memory collection boolean required')
    if not enabled:
        yield None
        return
    from adb_control import AdbRecorder
    path = Path(inputs['host']['workspaceRoot']) / 'tauri/scripts/android-process-memory.py'
    if hashlib.sha256(path.read_bytes()).hexdigest() != inputs['canonicalMemoryReaderSha256']:
        raise ValueError('Original strict memory source changed')
    spec = importlib.util.spec_from_file_location('journey_original_memory', path)
    memory = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(memory)
    folder = Path(output) / 'journey-memory'
    sdk_folder = Path(output) / 'journey-memory-sdk'
    sdk_folder.mkdir(exist_ok=False)
    recorder = AdbRecorder(inputs['host']['adbExecutable'], inputs['host']['serial'], sdk_folder)
    stream = Stream(recorder.run, memory.collect_snapshot, folder, uid=uid, deadline=deadline, phases=phases)
    original = None
    try:
        yield stream
    except BaseException as error:
        original = error
        raise
    finally:
        try:
            stream.close()
        except BaseException:
            if original is None:
                raise
