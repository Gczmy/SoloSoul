import base64
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

import diagnostic_exchange as exchange
import test_observer_protocol as protocol
import test_persistent_control as legacy


class FakeClient(exchange.Client):
    def __init__(self, output, response):
        super().__init__(None, output, SimpleNamespace(poll=lambda: None), 'files/'+'a'*32+'/')
        self.composite_bound = True
        self.response = response
        self.commands = []
        self.image = b'\x89PNG\r\n\x1a\npublic\x00\r\n'

    def call(self, *args, deadline, **kwargs):
        self.commands.append((args, kwargs))
        raw = self.image if args[0] == 'rf312-get-stable' else json.dumps(self.response).encode()
        return subprocess.CompletedProcess(args, 0, raw, b'')


class CompositeProtocol(protocol.Protocol):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def client(self, response):
        return FakeClient(self.directory, response)

    def test_one_exchange_submits_once_and_keeps_original_response_bytes(self):
        client = self.client(protocol.result())
        self.assertTrue(client.snapshot(5)['success'])
        self.assertEqual(len(client.commands), 1)
        self.assertEqual(client.commands[0][0], ('rf312-exchange', client.remote+'request-001.json'))
        self.assertEqual(json.loads(client.commands[0][1]['input']), {'action': 'snapshot', 'screenshot': False})
        self.assertEqual(json.loads((self.directory/'response-001.json').read_bytes()), protocol.result())

    def test_screenshot_uses_one_stable_read_without_changing_any_bytes(self):
        client = self.client(protocol.result(screenshot='frame-001.png'))
        client.snapshot(5, screenshot=True)
        self.assertEqual([command[0][0] for command in client.commands], ['rf312-exchange', 'rf312-get-stable'])
        self.assertEqual((self.directory/'frame-001.png').read_bytes(), client.image)

    def test_unbound_measurement_cannot_fall_back_to_multiple_sdk_requests(self):
        client = self.client(protocol.result());client.composite_bound = False
        with self.assertRaisesRegex(RuntimeError, 'binding required'):
            client.snapshot(5)
        self.assertEqual(client.commands, [])
        self.assertEqual(client.ordinal, 0)

    def test_failed_exchange_is_not_sent_again(self):
        client = self.client(protocol.result())
        client.call = Mock(side_effect=subprocess.TimeoutExpired('owned-exchange', .1, output=b'partial'))
        with self.assertRaises(subprocess.TimeoutExpired):
            client.snapshot(5)
        self.assertEqual(client.call.call_count, 1)
        self.assertEqual(client.ordinal, 1)
        self.assertFalse((self.directory/'response-001.json').exists())


class CompositeWire(unittest.TestCase):
    def test_legacy_reader_still_rejects_new_operations(self):
        with self.assertRaises(ValueError):
            legacy.request_line(1, 'exchange', 'request-001.json', b'{}')
        self.assertEqual(exchange.request_line(1, 'exchange', 'request-001.json', b'{}'),
                         b'RF312B 1 exchange request-001.json e30=\n')
        self.assertEqual(exchange.request_line(2, 'get-stable', 'frame-001.png'),
                         b'RF312B 2 get-stable frame-001.png -\n')

    def test_unsafe_or_replayed_namespace_and_extra_payload_are_rejected(self):
        values = [(True, 'exchange', 'request-001.json', b'{}'),
                  (16385, 'exchange', 'request-001.json', b'{}'),
                  (1, 'exchange', 'request-000.json', b'{}'),
                  (1, 'exchange', 'request-513.json', b'{}'),
                  (1, 'exchange', '../request-001.json', b'{}'),
                  (1, 'exchange', 'request-٠٠١.json', b'{}'),
                  (1, 'exchange', 'request-001.json', b''),
                  (1, 'exchange', 'request-001.json', b'x'*16385),
                  (1, 'get-stable', 'ready;id', b''),
                  (1, 'get-stable', 'ready', b'unexpected')]
        for value in values:
            with self.subTest(value=value[:3]), self.assertRaises(ValueError):
                exchange.request_line(*value)

    def test_original_fixed_commands_and_wire_bytes_remain_unchanged(self):
        for args in [(1, 'ping'), (2, 'focus'), (3, 'exists', 'error'),
                     (4, 'put', 'request-004.json', b'public\r\n\x00')]:
            self.assertEqual(exchange.request_line(*args), legacy.request_line(*args))

    def adapter(self, **change):
        control = Mock()
        control.call.return_value = subprocess.CompletedProcess([], 0, b'raw\r\n\x00', b'')
        value = exchange.Adapter(control, 'files/'+'a'*32+'/', ['owned-adb'], time.monotonic()+20, shell_crlf=True)
        return value, control

    def test_adapter_keeps_binary_bytes_and_bounds_original_deadline(self):
        adapter, control = self.adapter()
        result = adapter.run('rf312-get-stable', adapter.remote+'frame-001.png', timeout=5,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.assertEqual(result.stdout, b'raw\r\n\x00')
        self.assertLessEqual(control.call.call_args.kwargs['timeout'], 5)
        self.assertEqual(control.call.call_args.args, ('get-stable', 'frame-001.png'))

    def test_zero_exit_stderr_and_wrong_namespace_never_become_success(self):
        adapter, control = self.adapter()
        control.call.return_value = subprocess.CompletedProcess([], 0, b'raw', b'unknown error')
        with self.assertRaisesRegex(RuntimeError, 'stderr'):
            adapter.run('rf312-get-stable', adapter.remote+'ready', stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.assertEqual(control.call.call_count, 1)
        control.reset_mock()
        with self.assertRaises(ValueError):
            adapter.run('rf312-get-stable', 'files/'+'b'*32+'/ready', stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        control.call.assert_not_called()


class CompositePipe(legacy.Pipe):
    def actor(self, source):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        process = subprocess.Popen([sys.executable, '-B', '-u', '-c', source],
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        control = exchange.Control(process, Path(temporary.name)/'receipts')
        self.addCleanup(control.abort)
        return control

    def test_new_exchange_uses_canonical_reply_parser_and_raw_receipts(self):
        source = """import base64,sys
for line in sys.stdin.buffer:
 p=line.split();seq=p[1];op=p[2];out=base64.b64decode(p[4]) if op==b'exchange' else b''
 raw=b'RF312B '+seq+b' 0 '+str(len(out)).encode()+b' 0\\nO '+base64.b64encode(out)+b'\\nE \\n'
 sys.stdout.buffer.write(raw);sys.stdout.buffer.flush()
 if op==b'close':break
"""
        control = self.actor(source)
        payload = b'{"public":"\x00\r\n"}'
        self.assertEqual(control.call('exchange', 'request-001.json', payload, timeout=3).stdout, payload)
        self.assertEqual((control.output/'00001.request').read_bytes(),
                         exchange.request_line(1, 'exchange', 'request-001.json', payload))
        control.close()


if __name__ == '__main__':
    unittest.main()
