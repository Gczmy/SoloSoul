"""Prototype one owned, bounded pipe; unknown execution outcomes never replay."""
import base64
import hashlib
import json
import math
from pathlib import Path
import queue
import re
import subprocess
import threading
import time

LIMIT = 4 * 1024 * 1024
ERROR_LIMIT = 65536


def request_line(sequence, operation, name='-', payload=b''):
    if type(sequence) is not int or not 1 <= sequence <= 16384:
        raise ValueError('Bounded monotonic control sequence required')
    if not isinstance(name, str) or not isinstance(payload, bytes):
        raise ValueError('Explicit protocol types required')
    if operation in ('size', 'read'):
        valid = re.fullmatch(r'(?:ready|error|stopped|response-[0-9]{3}\.json|frame-[0-9]{3}\.png)', name)
    elif operation == 'exists':
        valid = re.fullmatch(r'(?:ready|error|stopped|response-[0-9]{3}\.json)', name)
    elif operation == 'put':
        valid = re.fullmatch(r'request-[0-9]{3}\.json', name)
    elif operation == 'stop':
        valid = name == 'stop'
    elif operation in ('ping', 'focus', 'pid', 'instrumentation', 'app-stop', 'app-start', 'close'):
        valid = name == '-'
    else:
        valid = False
    if not valid or len(payload) > 16384 or operation != 'put' and payload:
        raise ValueError('Only bounded fixed public protocol operations allowed')
    data = base64.b64encode(payload).decode() if operation == 'put' else '-'
    if operation == 'put' and not payload:
        raise ValueError('An atomic request cannot be empty')
    return f'RF312B {sequence} {operation} {name} {data}\n'.encode('ascii')


def read_reply(stream, sequence, partial):
    header = stream.readline(129)
    partial.extend(header)
    match = re.fullmatch(rb'RF312B ([1-9][0-9]{0,4}) (0|[1-9][0-9]{0,2}) (0|[1-9][0-9]{0,6}) (0|[1-9][0-9]{0,4})\r?\n', header)
    if match is None:
        raise RuntimeError('Invalid or truncated control frame header')
    actual, code, out_size, err_size = map(int, match.groups())
    if actual != sequence or code > 255 or out_size > LIMIT or err_size > ERROR_LIMIT:
        raise RuntimeError('Control sequence, exit code or byte bound differs')
    def exact(label, size):
        encoded_size = 4 * ((size + 2) // 3)
        line = stream.readline(encoded_size + 5)
        partial.extend(line)
        # Only the framing newline may be LF or CRLF. No content byte is
        # normalized: the payload is strictly decoded and checked separately.
        if line.endswith(b'\r\n'):
            body = line[:-2]
        elif line.endswith(b'\n'):
            body = line[:-1]
        else:
            raise RuntimeError('Truncated Base64 control frame body')
        if not body.startswith(label + b' ') or len(body) != encoded_size + 2:
            raise RuntimeError('Control payload label or encoded length differs')
        encoded = body[2:]
        try:
            result = base64.b64decode(encoded, validate=True)
        except ValueError as error:
            raise RuntimeError('Invalid Base64 control payload') from error
        if len(result) != size or base64.b64encode(result) != encoded:
            raise RuntimeError('Control decoded length or canonical encoding differs')
        return result
    return code, exact(b'O', out_size), exact(b'E', err_size)


class Control:
    """One Popen handle from the caller, with exclusive requests and raw receipts."""
    def __init__(self, process, output):
        if any(getattr(process, name, None) is None for name in ('stdin', 'stdout', 'stderr')):
            raise ValueError('Three owned binary pipes required')
        self.process = process
        self.output = Path(output)
        self.output.mkdir(exist_ok=False)
        self.sequence = 0
        self.broken = False
        self.exchanges = queue.Queue(maxsize=1)
        self.lock = threading.Lock()
        self.stderr = bytearray()
        self.stderr_overflow = False
        self.stderr_error = None
        self.reader = threading.Thread(target=self._serve, daemon=True)
        self.drainer = threading.Thread(target=self._drain, daemon=True)
        self.reader.start()
        self.drainer.start()

    def _drain(self):
        try:
            while data := self.process.stderr.read1(4096):
                available = ERROR_LIMIT - len(self.stderr)
                self.stderr.extend(data[:max(0, available)])
                if len(data) > available:
                    self.stderr_overflow = True
        except BaseException as error:
            self.stderr_error = error

    def _serve(self):
        while item := self.exchanges.get():
            line, sequence, partial, answer = item
            try:
                if self.process.stdin.write(line) != len(line):
                    raise RuntimeError('Truncated control request write')
                self.process.stdin.flush()
                answer.put((True, read_reply(self.process.stdout, sequence, partial)))
            except BaseException as error:
                answer.put((False, error))
                return

    def abort(self):
        self.broken = True
        if self.process.poll() is None:
            self.process.terminate()
        self.process.wait(timeout=10)
        try:
            self.exchanges.put_nowait(None)
        except queue.Full:
            pass
        for handle in (self.process.stdin, self.process.stdout, self.process.stderr):
            handle.close()
        self.reader.join(timeout=1)
        self.drainer.join(timeout=1)
        (self.output / 'transport.stderr').write_bytes(bytes(self.stderr))

    def call(self, operation, name='-', payload=b'', *, timeout):
        if isinstance(timeout, bool) or not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or not 0 < timeout <= 240:
            raise ValueError('Original finite positive budget up to 240 seconds required')
        if not self.lock.acquire(blocking=False):
            raise RuntimeError('Concurrent control calls refused')
        started = time.monotonic()
        try:
            if self.broken:
                raise RuntimeError('Broken control channel cannot be reused or replayed')
            line = request_line(self.sequence + 1, operation, name, payload)
            self.sequence += 1
            partial = bytearray()
            row = dict(sequence=self.sequence, operation=operation, name=name, timeoutSeconds=timeout,
                       requestSha256=hashlib.sha256(line).hexdigest())
            answer = queue.Queue(maxsize=1)
            failure = None
            try:
                self.exchanges.put_nowait((line, self.sequence, partial, answer))
                remaining = timeout - (time.monotonic() - started)
                if remaining <= 0:
                    raise queue.Empty
                success, reply = answer.get(timeout=remaining)
                if not success:
                    raise reply
                if time.monotonic() - started > timeout:
                    raise queue.Empty
                if self.stderr_overflow or self.stderr_error or self.stderr:
                    raise RuntimeError('Control transport emitted unexpected stderr')
                code, out, err = reply
                row.update(exitCode=code, stdoutSha256=hashlib.sha256(out).hexdigest(),
                           stderrSha256=hashlib.sha256(err).hexdigest())
            except BaseException as error:
                row.update(errorType=type(error).__name__, timedOut=isinstance(error, queue.Empty))
                try:
                    self.abort()
                except BaseException as cleanup_error:
                    row['ownedPipeCleanupErrorType'] = type(cleanup_error).__name__
                if isinstance(error, queue.Empty):
                    failure = subprocess.TimeoutExpired(self.process.args, timeout, output=bytes(partial), stderr=bytes(self.stderr))
                    raise failure from error
                failure = error
                raise
            finally:
                raw = bytes(partial)
                prefix = self.output / f'{self.sequence:05d}'
                try:
                    prefix.with_suffix('.request').write_bytes(line)
                    prefix.with_suffix('.reply').write_bytes(raw)
                    row.update(elapsedSeconds=time.monotonic()-started, replyBytes=len(raw),
                               replySha256=hashlib.sha256(raw).hexdigest())
                    prefix.with_suffix('.json').write_text(json.dumps(row, indent=2)+'\n', encoding='utf-8')
                except OSError as receipt_error:
                    if failure is not None:
                        raise failure from receipt_error
                    self.abort()
                    raise
            result = subprocess.CompletedProcess([operation, name], code, out, err)
            result.check_returncode()
            return result
        finally:
            self.lock.release()

    def close(self, timeout=10):
        if self.broken:
            return
        started = time.monotonic()
        try:
            self.call('close', timeout=timeout)
            self.process.stdin.close()
            remaining = timeout - (time.monotonic() - started)
            if remaining <= 0:
                raise subprocess.TimeoutExpired(self.process.args, timeout)
            self.process.wait(timeout=remaining)
            self.drainer.join(timeout=1)
            if self.process.returncode != 0 or self.stderr or self.stderr_overflow or self.stderr_error:
                raise RuntimeError('Control terminal exit/stderr differs')
        finally:
            self.abort()
