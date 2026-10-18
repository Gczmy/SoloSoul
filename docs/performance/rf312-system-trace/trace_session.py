"""Bounded whole-journey capture; original UI/owner gates stay outside."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import time

import transport

sha = lambda raw: hashlib.sha256(raw).hexdigest()
BASE_HASH = 'de6be90adb296dcaa2cb168e141fb76d5eb6ffe0ed15340c174bab544d40a518'
MAX_BYTES = 32 * 1024 * 1024


def config(base, output):
    if sha(base) != BASE_HASH:
        raise ValueError('Original base configuration changed')
    # A distinct fixed whole-journey contract, never reclassifies five-second data.
    body = base.replace(b'duration_ms: 5000\n', b'duration_ms: 240000\n')
    return body + ('\nmax_file_size_bytes: 33554432\nwrite_into_file: true\noutput_path: "'
                   + output + '"\n').encode('ascii')


def log_messages(result, expected):
    if (type(result.returncode) is not int or result.returncode != 0
            or not isinstance(result.stdout, bytes) or result.stdout != b''
            or not isinstance(result.stderr, bytes) or len(result.stderr) > 16384
            or not result.stderr.endswith(b'\n')):
        raise ValueError('Unproven bounded-session CLI status')
    messages = []
    for line in result.stderr.splitlines():
        match = re.fullmatch(rb'\[[ 0-9]+\.[0-9]+\] +perfetto_cmd\.cc:[1-9][0-9]* +(.+)', line)
        if not match:
            raise ValueError('Unknown session CLI log')
        messages.append(match[1])
    if messages != expected:
        raise ValueError('Incomplete or contradictory session CLI log')


class Session:
    def __init__(self, call, inputs, output):
        self.raw_call, self.inputs, self.out = call, Path(inputs), Path(output)
        if not self.out.is_dir() or any(self.out.iterdir()):
            raise ValueError('Caller-owned empty output directory required')
        self.plan_raw = (self.inputs/'file-plan.json').read_bytes()
        plan = json.loads(self.plan_raw)
        if set(plan) != {'nonce'}:
            raise ValueError('Exact file plan required')
        self.names = transport.remote_paths(plan['nonce'])
        self.key = 'rf312-' + plan['nonce']
        self.body = config((self.inputs/'base-config.pbtxt').read_bytes(), self.names[2])
        self._verify()
        self.owned = []
        self.started = self.start_attempted = self.stop_attempted = self.output_owned = False
        self.cleanup_recorded = False
        self.stop_completed = False

    def call(self, label, args, *, timeout):
        if timeout not in (5,45):
            raise ValueError('Unexpected SDK deadline')
        result = self.raw_call(label,args,timeout=timeout)
        if (type(result.returncode) is not int or not isinstance(result.stdout,bytes)
                or not isinstance(result.stderr,bytes)):
            raise ValueError('Raw SDK command result required')
        if result.returncode:
            raise subprocess.CalledProcessError(result.returncode,args,result.stdout,result.stderr)
        return result

    def _verify(self):
        if ((self.inputs/'file-plan.json').read_bytes() != self.plan_raw
                or sha((self.inputs/'base-config.pbtxt').read_bytes()) != BASE_HASH
                or (self.inputs/'trace-config.pbtxt').read_bytes() != self.body
                or (self.inputs/'roundtrip.bin').read_bytes() != transport.ROUNDTRIP):
            raise ValueError('Frozen session inputs changed')

    def _state(self, label, count):
        result = self.call(label, ['exec-out','perfetto','--query-raw'], timeout=5)
        if result.returncode != 0 or result.stderr:
            raise ValueError('Invalid session state command')
        state = transport.service_state(result.stdout)
        if state['totalSessions'] != count or state['currentlyStartedSessions'] != count:
            raise ValueError('Unexpected session occupancy')
        return state

    def _key_exists(self):
        try:
            result = self.call('session-key-state', ['shell','-T','perfetto','--is_detached='+self.key], timeout=5)
        except subprocess.CalledProcessError as error:
            if error.returncode == 2 and error.stdout == error.stderr == b'':
                return False
            raise
        if result.returncode != 0 or result.stdout or result.stderr:
            raise ValueError('Unexpected detach-key probe result')
        return True

    def _stop(self):
        if self.stop_attempted:
            raise ValueError('Session stop is one-shot')
        self.stop_attempted = True
        self.stop_before_host_ns = time.monotonic_ns()
        if not self._key_exists():
            raise ValueError('Owned session missing or expired')
        result = self.call('session-stop', ['shell','-T','perfetto','--attach='+self.key,'--stop'], timeout=45)
        log_messages(result, [b'Trace written into the output file'])
        self.after = self._state('session-state-after', 0)
        self.stop_completed = True

    def _cleanup(self):
        if self.cleanup_recorded:
            raise ValueError('Cleanup receipt already reserved')
        errors = []
        if self.start_attempted and not self.stop_attempted:
            try:
                # Only our nonce key may be stopped; never stop all sessions/PIDs.
                if self._key_exists():
                    self._stop()
            except Exception as error:
                errors.append(dict(step='owned-key-stop',type=type(error).__name__,error=str(error)))
        if self.owned:
            try:
                self.call('session-owned-files-remove', ['shell','rm','-f',*self.owned], timeout=5)
            except Exception as error:
                errors.append(dict(step='owned-file-remove',type=type(error).__name__,error=str(error)))
        receipt = dict(passed=not errors and (not self.start_attempted or self.stop_completed)
                       and not (self.start_attempted and not self.output_owned),
                       exactOwnedFiles=list(self.owned), errors=errors,
                       unclaimedOutputMayRemain=self.start_attempted and not self.output_owned,
                       automaticStopRetries=False, ttlMs=240000,
                       sessionStopCompleted=self.stop_completed,
                       foreignSessionStopOrRecursiveDeleteUsed=False)
        with (self.out/'session-cleanup.json').open('x',encoding='utf-8',newline='\n') as stream:
            self.cleanup_recorded = True
            json.dump(receipt,stream,indent=2);stream.write('\n')
        return receipt

    def start(self):
        if self.started or self.start_attempted or self.cleanup_recorded:
            raise ValueError('Capture cannot start twice')
        try:
            self._verify()
            self.before = self._state('session-state-before', 0)
            for role, name, remote in [('fixture','roundtrip.bin',self.names[0]),
                                       ('config','trace-config.pbtxt',self.names[1])]:
                result = self.call('session-'+role+'-reserve', ['shell','-T','set -C; : > '+remote], timeout=5)
                self.owned.append(remote)
                if result.returncode != 0 or result.stdout or result.stderr:
                    raise ValueError('Exclusive file reservation failed')
                self.call('session-'+role+'-push', ['push',str(self.inputs/name),remote], timeout=5)
                result = self.call('session-'+role+'-readback', ['exec-out','cat',remote], timeout=5)
                if result.returncode != 0 or result.stderr or result.stdout != (self.inputs/name).read_bytes():
                    raise ValueError('Raw bounded-session file transfer differs')
            self.start_attempted = True
            self.start_before_host_ns = time.monotonic_ns()
            result = self.call('session-start', ['shell','-T','perfetto','--txt','-c',self.names[1],
                                                '--detach='+self.key], timeout=5)
            log_messages(result, [b'Connected to the Perfetto traced service, TTL: 240s'])
            if not self._key_exists():
                raise ValueError('Detached session identity not proven')
            self.output_owned = True
            self.owned.append(self.names[2])
            self.active_state = self._state('session-state-active', 1)
            self.active_after_host_ns = time.monotonic_ns()
            self.started = True
            return self
        except BaseException:
            self._cleanup()
            raise

    def finish(self):
        if not self.started or self.stop_attempted or self.cleanup_recorded:
            raise ValueError('One active session required')
        try:
            self._verify()
            self._stop()
            result = self.call('session-trace-hash', ['shell','sha256sum',self.names[2]], timeout=5)
            parts = result.stdout.decode('ascii').split()
            if (result.stderr or len(parts) != 2 or parts[1] != self.names[2]
                    or not re.fullmatch('[0-9a-f]{64}',parts[0])):
                raise ValueError('Remote trace identity differs')
            path = self.out/'system.trace'
            if path.exists() or path.is_symlink():
                raise ValueError('Trace destination already used')
            self.call('session-trace-pull', ['pull',self.names[2],str(path)], timeout=45)
            if path.is_symlink() or not path.is_file() or not 1 <= path.stat().st_size <= MAX_BYTES:
                raise ValueError('Invalid bounded-session trace size')
            raw = path.read_bytes()
            if sha(raw) != parts[0]:
                raise ValueError('Bounded-session trace hash differs')
            packets = transport.fields(raw,MAX_BYTES,500000)
            if set(packets) != {1} or not packets[1] or any(w != 2 or not b for w,b in packets[1]):
                raise ValueError('Invalid trace packet envelope')
            receipt = dict(passed=True,scope='bounded-whole-journey-trace-transport-only',
                           traceSha256=sha(raw),traceBytes=len(raw),topLevelPackets=len(packets[1]),
                           ttlMs=240000,ownedSessionKeyProven=True,recordingStoppedExplicitly=True,
                           serviceBefore=self.before,serviceActive=self.active_state,serviceAfter=self.after,
                           hostMonotonicBrackets=dict(startBeforeNs=self.start_before_host_ns,
                             activeAfterNs=self.active_after_host_ns,stopBeforeNs=self.stop_before_host_ns),
                           nativeFramesAccepted=False,applicationPerformanceAccepted=False,
                           wholeJourneyCoverageAccepted=False,fullRF312Accepted=False)
        finally:
            cleanup = self._cleanup()
        if not cleanup['passed']:
            raise ValueError('Bounded-session cleanup failed')
        with (self.out/'session-transport.json').open('x',encoding='utf-8',newline='\n') as stream:
            json.dump(receipt,stream,indent=2);stream.write('\n')
        return receipt
