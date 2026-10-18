import shlex,subprocess,time,unittest
from persistent_control import request_line
from persistent_adapter import Adapter,SnapshotCadence,PKG,APP

REMOTE='files/'+'a'*32+'/'
PREFIX=['owned-adb','-H','127.0.0.1','-P','5057','-s','emulator-5578']
CAPTURE=dict(stdout=subprocess.PIPE,stderr=subprocess.PIPE)

class Backend:
    def __init__(self,code=0,out=b'',err=b'',error=None):self.calls=[];self.code=code;self.out=out;self.err=err;self.error=error
    def call(self,op,name='-',payload=b'',*,timeout):
        request_line(1,op,name,payload)
        self.calls.append((op,name,payload,timeout))
        if self.error is not None:raise self.error
        result=subprocess.CompletedProcess([op,name],self.code,self.out,self.err);result.check_returncode();return result

class ExistingCalls(unittest.TestCase):
    def adapter(self,backend=None,*,deadline=None,crlf=True):
        backend=backend or Backend()
        return Adapter(backend,REMOTE,PREFIX,time.monotonic()+240 if deadline is None else deadline,shell_crlf=crlf),backend

    def test_binary_exec_out_file_bytes_are_never_normalized(self):
        body=b'\x00\r\n\n\r'+bytes(range(256));adapter,backend=self.adapter(Backend(out=body))
        result=adapter.run('exec-out','run-as',PKG,'cat',REMOTE+'frame-001.png',timeout=2,**CAPTURE)
        self.assertEqual(result.stdout,body);self.assertEqual(backend.calls[0][:3],('read','frame-001.png',b''))
        self.assertEqual(result.args,PREFIX+['exec-out','run-as',PKG,'cat',REMOTE+'frame-001.png'])

    def test_verified_shell_serialization_and_original_read_interface(self):
        adapter,backend=self.adapter(Backend(out=b'6274\n'))
        result=adapter.run('shell','run-as',PKG,'stat','-c','%s',REMOTE+'frame-001.png',**CAPTURE)
        self.assertEqual(result.stdout,b'6274\r\n');self.assertEqual(backend.calls[0][0],'size')
        self.assertEqual(adapter.read('shell','run-as',PKG,'stat','-c','%s',REMOTE+'frame-001.png'),'6274')

    def test_real_errors_and_empty_exit_one_remain_distinct(self):
        for err,expected in [(b'',b''),(b'not readable\n',b'not readable\r\n')]:
            adapter,backend=self.adapter(Backend(1,err=err))
            with self.assertRaises(subprocess.CalledProcessError) as caught:
                adapter.run('shell','run-as',PKG,'test','-e',REMOTE+'error',**CAPTURE)
            self.assertEqual((caught.exception.returncode,caught.exception.stdout,caught.exception.stderr),(1,b'',expected))
            self.assertEqual(caught.exception.cmd,PREFIX+['shell','run-as',PKG,'test','-e',REMOTE+'error'])
            self.assertEqual(len(backend.calls),1)

    def test_atomic_existing_request_command_and_bytes_match(self):
        adapter,backend=self.adapter();tmp=REMOTE+'request-012.tmp';final=REMOTE+'request-012.json'
        command='run-as '+PKG+' sh -c '+shlex.quote('umask 077; cat > '+tmp+' && mv '+tmp+' '+final)
        value=b'{"action":"tap","target":{"bounds":"[1,2][3,4]"}}'
        adapter.run('shell',command,input=value,timeout=3.25,**CAPTURE)
        self.assertEqual(backend.calls[0][:3],('put','request-012.json',value));self.assertEqual(backend.calls[0][3],3.25)

    def test_unrecognized_command_namespace_or_atomic_mutation_has_no_fallback(self):
        adapter,backend=self.adapter()
        for command in [('shell','input','tap','1','2'),('shell','run-as',APP,'cat','files/private'),
                        ('exec-out','run-as',PKG,'cat','files/'+'b'*32+'/frame-001.png'),
                        ('shell','run-as',PKG,'touch',REMOTE+'error'),
                        ('shell',"run-as "+PKG+" sh -c 'cat > "+REMOTE+"request-001.tmp; input tap 1 2'")]:
            with self.subTest(command=command),self.assertRaises(ValueError):adapter.run(*command,**CAPTURE)
        self.assertEqual(backend.calls,[])

    def test_all_original_fixed_queries_and_start_commands_map_once(self):
        adapter,backend=self.adapter()
        commands=[(('shell','dumpsys','window','displays'),'focus'),(('shell','pidof',APP),'pid'),
            (('shell','dumpsys','activity','instrumentation'),'instrumentation'),(('shell','am','force-stop',APP),'app-stop'),
            (('shell','am','start','-W','-n',APP+'/.MainActivity'),'app-start')]
        for command,expected in commands:
            adapter.run(*command,**CAPTURE);self.assertEqual(backend.calls[-1][0],expected)
        self.assertEqual(len(backend.calls),len(commands))

    def test_transport_exception_is_preserved_and_never_resent(self):
        for error in [RuntimeError('fixed protocol refused'),subprocess.TimeoutExpired('fixed public input',1)]:
            adapter,backend=self.adapter(Backend(error=error))
            with self.assertRaises(type(error)) as caught:adapter.run('shell','pidof',APP,**CAPTURE)
            self.assertIs(caught.exception,error);self.assertEqual(len(backend.calls),1)

    def test_invalid_options_and_expired_budgets_do_not_dispatch(self):
        adapter,backend=self.adapter()
        for kwargs in [dict(stdout=None,stderr=subprocess.PIPE),dict(**CAPTURE,creationflags=0),
                       dict(**CAPTURE,timeout=True),dict(**CAPTURE,timeout=241)]:
            with self.assertRaises(ValueError):adapter.run('shell','pidof',APP,**kwargs)
        expired,backend=self.adapter(backend,deadline=time.monotonic()-1)
        with self.assertRaises(TimeoutError):expired.run('shell','pidof',APP,**CAPTURE)
        self.assertEqual(backend.calls,[])

class Cadence(unittest.TestCase):
    def clock(self):
        state={'now':0.0,'sleeps':[]}
        def sleep(value):state['sleeps'].append(value);state['now']+=value
        return SnapshotCadence(clock=lambda:state['now'],sleep=sleep),state
    def test_three_reads_still_span_at_least_350_ms(self):
        cadence,state=self.clock();times=[]
        for _ in range(3):cadence.before(2);times.append(state['now'])
        self.assertEqual(times,[0,.175,.350]);self.assertGreaterEqual(times[-1]-times[0],.350)
    def test_short_deadline_never_extends_or_starts_next_read(self):
        cadence,state=self.clock();cadence.before(1)
        with self.assertRaises(TimeoutError):cadence.before(.1)
        self.assertEqual(state['now'],.1);self.assertEqual(cadence.last,0)
    def test_already_expired_deadline_does_not_sleep(self):
        cadence,state=self.clock()
        with self.assertRaises(TimeoutError):cadence.before(0)
        self.assertEqual(state['sleeps'],[])
    def test_premature_wakeup_fails_without_busy_polling(self):
        sleeps=[];cadence=SnapshotCadence(clock=lambda:0,sleep=sleeps.append);cadence.before(1)
        with self.assertRaises(RuntimeError):cadence.before(1)
        self.assertEqual(len(sleeps),32);self.assertTrue(all(value>=.001 for value in sleeps));self.assertEqual(cadence.last,0)
    def test_coarse_clock_waits_until_due_without_rounding_down_the_interval(self):
        import math
        state={'actual':0.0,'sleeps':[]}
        def clock():return math.floor(state['actual']/.015625)*.015625
        def sleep(value):state['sleeps'].append(value);state['actual']+=value
        cadence=SnapshotCadence(clock=clock,sleep=sleep);observed=[]
        for _ in range(3):cadence.before(2);observed.append(cadence.last)
        self.assertGreater(len(state['sleeps']),2)
        self.assertGreaterEqual(observed[1]-observed[0],.175)
        self.assertGreaterEqual(observed[2]-observed[1],.175)
        self.assertGreaterEqual(observed[2]-observed[0],.350)
    def test_progressing_early_wakeup_reuses_original_due_time(self):
        state={'now':0.0,'sleeps':[]}
        def sleep(value):
            state['sleeps'].append(value);state['now']+=value-.001 if len(state['sleeps'])==1 else value
        cadence=SnapshotCadence(clock=lambda:state['now'],sleep=sleep);cadence.before(1);cadence.before(1)
        self.assertEqual(len(state['sleeps']),2);self.assertAlmostEqual(cadence.last,.175)
    def test_repeated_wait_cannot_extend_short_original_deadline(self):
        state={'now':0.0,'sleeps':[]}
        def sleep(value):
            state['sleeps'].append(value);state['now']+=value-.001 if len(state['sleeps'])==1 else value
        cadence=SnapshotCadence(clock=lambda:state['now'],sleep=sleep);cadence.before(1)
        with self.assertRaises(TimeoutError):cadence.before(.175)
        self.assertEqual(cadence.last,0);self.assertAlmostEqual(state['now'],.175)
        self.assertEqual(len(state['sleeps']),2)
    def test_backward_clock_before_or_during_wait_preserves_last_snapshot(self):
        state={'now':1.0};cadence=SnapshotCadence(clock=lambda:state['now'],sleep=lambda _:state.update(now=.9));cadence.before(2)
        with self.assertRaisesRegex(RuntimeError,'backwards'):cadence.before(2)
        self.assertEqual(cadence.last,1)
        state['now']=.8
        with self.assertRaisesRegex(RuntimeError,'backwards'):cadence.before(2)
        self.assertEqual(cadence.last,1)

if __name__=='__main__':unittest.main()
