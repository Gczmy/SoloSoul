from pathlib import Path
import importlib.util,json,subprocess,sys,tempfile,unittest
spec=importlib.util.spec_from_file_location('trace_probe',Path(__file__).with_name('service_state.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
def vi(n):
    b=bytearray()
    while n>=128:b.append((n&127)|128);n>>=7
    b.append(n);return bytes(b)
def field(n,v):return vi(n*8+2)+vi(len(v))+v if isinstance(v,bytes) else vi(n*8)+vi(v)
def state(*,total=0,started=0,missing=False,foreign=False):
    producer=field(1,1)+field(2,b'traced_probes')+field(3,0)
    names=sorted(m.REQUIRED)
    if missing:names.pop()
    ds=b''.join(field(2,field(1,field(1,n.encode()))+field(2,2 if foreign else 1)) for n in names)
    return field(1,producer)+ds+field(3,total)+field(4,started)
class Codec(unittest.TestCase):
    def test_valid_raw_state(self):self.assertEqual(m.service_state(state())['currentlyStartedSessions'],0)
    def test_started_is_not_cumulative(self):self.assertEqual(m.service_state(state(total=2,started=1))['currentlyStartedSessions'],1)
    def test_unknown_field_preserved_core(self):self.assertEqual(m.service_state(state()+field(100,b'future'))['producerCount'],1)
    def test_missing_source_rejected(self):
        with self.assertRaises(ValueError):m.service_state(state(missing=True))
    def test_unbound_source_rejected(self):
        with self.assertRaises(ValueError):m.service_state(state(foreign=True))
    def test_duplicate_count_rejected(self):
        with self.assertRaises(ValueError):m.service_state(state()+field(3,0))
    def test_missing_counts_not_zero(self):
        with self.assertRaises(ValueError):m.service_state(state()[:-4])
    def test_bad_session_order_rejected(self):
        with self.assertRaises(ValueError):m.service_state(state(total=0,started=1))
    def test_truncated_varint_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x80')
    def test_truncated_length_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x0a\x05a')
    def test_noncanonical_varint_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x88\0\0')
    def test_overflow_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x08'+b'\xff'*10)
    def test_group_wire_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x0b')
    def test_zero_field_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\0\0')
    def test_byte_bound_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'x'*262145)
    def test_field_bound_rejected(self):
        with self.assertRaises(ValueError):m.fields(field(1,0)*4097)

    def test_protozero_four_byte_length_accepted(self):
        self.assertEqual(m.fields(b'\x0a\x87\x80\x80\x00abcdefg'),{1:[(2,b'abcdefg')]})
    def test_protozero_zero_length_accepted(self):
        self.assertEqual(m.fields(b'\x0a\x80\x80\x80\x00'),{1:[(2,b'')]})
    def test_three_byte_redundant_length_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x0a\x81\x80\x00x')
    def test_five_byte_redundant_length_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x0a\x81\x80\x80\x80\x00x')
    def test_redundant_scalar_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x08\x80\x80\x80\x00')
    def test_truncated_protozero_length_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x0a\x87\x80\x80\x00abc')
    def test_unterminated_four_byte_length_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x0a\x80\x80\x80\x80')
    def test_ten_byte_overflow_length_rejected(self):
        with self.assertRaises(ValueError):m.fields(b'\x0a'+b'\xff'*9+b'\x02')


class CommandLine(unittest.TestCase):
    def invoke(self, raw, optimized=False):
        with tempfile.TemporaryDirectory() as folder:
            p=Path(folder)/'service state.raw';p.write_bytes(raw)
            return subprocess.run([sys.executable,'-B']+(['-O'] if optimized else [])+[str(Path(__file__).with_name('service_state.py')),str(p)],capture_output=True,timeout=10)
    def test_cli_accepts_complete_state(self):
        result=self.invoke(state());self.assertEqual(result.returncode,0)
        self.assertEqual(json.loads(result.stdout)['currentlyStartedSessions'],0)
    def test_cli_rejects_missing_counts(self):
        result=self.invoke(state()[:-4]);self.assertNotEqual(result.returncode,0)
        self.assertEqual(result.stdout,b'')
    def test_cli_optimized_mode_keeps_rejection(self):
        result=self.invoke(state()[:-4],optimized=True);self.assertNotEqual(result.returncode,0)
        self.assertEqual(result.stdout,b'')
    def test_cli_bounds_input_before_decode(self):
        result=self.invoke(b'x'*262145);self.assertNotEqual(result.returncode,0)
        self.assertIn(b'exceeds bound',result.stderr)

if __name__=='__main__':unittest.main()
