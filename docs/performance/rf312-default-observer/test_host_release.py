import copy
import importlib.util
import json
from pathlib import Path,PureWindowsPath
import subprocess
import tempfile
import unittest
from host_release import ReleaseState,wait_for_release,windows_capture

ROOT=Path(__file__).resolve().parents[3]
_spec=importlib.util.spec_from_file_location('release_test_canonical_fresh',ROOT/'tauri/scripts/android-fresh-host-audit.py')
fresh=importlib.util.module_from_spec(_spec);_spec.loader.exec_module(fresh)


class HostRelease(unittest.TestCase):
    def setUp(self):
        self.sdk=PureWindowsPath('X:/SDK space/sdk');self.system=PureWindowsPath('Y:/Windows')
        def row(name,pid,parent,second,path,command):
            return dict(Name=name,ProcessId=pid,ParentProcessId=parent,CreationDate=f'2020-01-01T00:00:{second:02d}.000001+00:00',
                        ExecutablePath=str(path),CommandLine=command)
        self.server=row('adb.exe',10,1,1,self.sdk/'platform-tools/adb.exe','"'+str(self.sdk/'platform-tools/adb.exe')+'" -L tcp:localhost:5057 server nodaemon')
        self.launcher=row('emulator.exe',11,1,2,self.sdk/'emulator/emulator.exe','"'+str(self.sdk/'emulator/emulator.exe')+'" -avd SoloSoul_unit -port 5578')
        self.vm=row('qemu-system-x86_64-headless.exe',12,11,3,self.sdk/'emulator/qemu/windows-x86_64/qemu-system-x86_64-headless.exe','qemu -avd SoloSoul_unit')
        binary=str(self.sdk/'emulator/emulator')
        self.cmd=row('cmd.exe',13,12,4,self.system/'System32/cmd.exe','/C ""'+binary+'" -kill 12 -sleep 20 >nul 2>&1"')
        self.helper=row('emulator.exe',14,13,5,self.sdk/'emulator/emulator.exe','"'+binary+'"  -kill 12 -sleep 20 ')
        self.console=row('conhost.exe',15,11,3,self.system/'System32/conhost.exe',chr(92)+'??'+chr(92)+str(self.system/'System32/conhost.exe')+' 0x4')
        self.spec=dict(schemaVersion=1,baseline='no-existing-adb-or-emulator',ownedProcesses=[self.server,self.launcher,self.vm],
            protectedPorts=[5037,5049],ownedPorts=[5057,5578,5579],avd='SoloSoul_unit')
        self.now=0;self.events=[];self.budgets=[]

    def state(self):return ReleaseState(fresh,self.spec,self.sdk,self.system)

    def snapshot(self,rows=None,listeners=None):return dict(processes=rows or [],listeners=listeners or [])

    def test_only_exact_observed_shutdown_chain_is_tracked_until_absent(self):
        state=self.state()
        self.assertFalse(state.observe(self.snapshot([self.helper,self.console,self.cmd])))
        self.assertEqual(set(state.known),{10,11,12,13,14,15})
        self.assertTrue(state.observe(self.snapshot()))
        self.assertEqual(fresh.audit(state.spec(),self.snapshot(),'after')['passed'],True)

    def test_original_roots_must_match_sdk_avd_and_complete_vm_ancestry(self):
        original=copy.deepcopy(self.spec)
        for change in ('avd','sdk','parent'):
            self.spec=copy.deepcopy(original)
            if change=='avd':self.spec['ownedProcesses'][1]['CommandLine']='emulator -avd SoloSoul_other'
            if change=='sdk':self.spec['ownedProcesses'][0]['ExecutablePath']='Z:/other/adb.exe'
            if change=='parent':self.spec['ownedProcesses'][2]['ParentProcessId']=999
            with self.subTest(change=change),self.assertRaises(ValueError):self.state()

    def test_other_sdk_binary_kill_target_or_parent_cannot_be_tracked(self):
        original=copy.deepcopy((self.cmd,self.helper))
        for change in ('sdk','target','parent','command'):
            cmd,helper=copy.deepcopy(original)
            if change=='sdk':helper['ExecutablePath']='Z:/other/emulator.exe'
            if change=='target':helper['CommandLine']=helper['CommandLine'].replace('-kill 12','-kill 999')
            if change=='parent':cmd['ParentProcessId']=999
            if change=='command':cmd['CommandLine']=cmd['CommandLine'].replace('-sleep 20','-sleep 30')
            with self.subTest(change=change),self.assertRaises(RuntimeError):self.state().observe(self.snapshot([helper,cmd]))

    def test_unrecognized_adb_or_arbitrary_descendant_fails_without_waiting(self):
        for name,path in (('adb.exe',self.sdk/'platform-tools/adb.exe'),('notepad.exe',self.system/'notepad.exe')):
            row=dict(self.cmd,Name=name,ExecutablePath=str(path))
            with self.subTest(name=name),self.assertRaises(RuntimeError):self.state().observe(self.snapshot([row]))

    def test_cycle_missing_executable_and_child_older_than_parent_refuse(self):
        for change in ('cycle','executable','birth'):
            row=copy.deepcopy(self.cmd)
            if change=='cycle':row['ParentProcessId']=row['ProcessId']
            if change=='executable':row['ExecutablePath']=None
            if change=='birth':row['CreationDate']='2019-01-01T00:00:00.000001+00:00'
            with self.subTest(change=change),self.assertRaises((ValueError,RuntimeError)):self.state().observe(self.snapshot([row]))

    def test_recorded_root_or_newly_tracked_helper_pid_reuse_refuses(self):
        state=self.state();state.observe(self.snapshot([self.cmd,self.helper]))
        for original in (self.server,self.helper):
            changed=dict(original,CreationDate='2021-01-01T00:00:00.000001+00:00')
            with self.subTest(name=original['Name']),self.assertRaises(RuntimeError):state.observe(self.snapshot([changed]))

    def test_protected_foreign_and_helper_owned_listeners_are_not_absorbed(self):
        for port,pid in ((5037,10),(5057,999),(5057,14)):
            listener=dict(LocalAddress='127.0.0.1',LocalPort=port,OwningProcess=pid)
            with self.subTest(port=port,pid=pid),self.assertRaises(RuntimeError):
                self.state().observe(self.snapshot([self.cmd,self.helper], [listener]))

    def test_remaining_original_budget_and_all_pending_observations_are_preserved(self):
        state=self.state()
        def capture(spec,*,timeout):
            self.budgets.append(timeout)
            return self.snapshot([self.cmd,self.helper]) if self.now<1 else self.snapshot()
        def sleep(delay):self.now+=delay
        actual=wait_for_release(state,capture,2,lambda n,row:self.events.append(row),clock=lambda:self.now,sleep=sleep)
        self.assertEqual(self.budgets,[2,1.5,1]);self.assertEqual(len(actual),5)
        self.assertEqual([row['accepted'] for row in self.events],[False,False,True])
        self.assertTrue(all(row['inputDispatched'] is False and row['processTerminationRequested'] is False for row in self.events))

    def test_persistent_verified_helper_exhausts_original_deadline(self):
        def capture(spec,*,timeout):return self.snapshot([self.cmd,self.helper])
        def sleep(delay):self.now+=delay
        with self.assertRaises(TimeoutError):wait_for_release(self.state(),capture,1,lambda n,row:self.events.append(row),clock=lambda:self.now,sleep=sleep)
        self.assertEqual(self.now,1);self.assertFalse(any(row['accepted'] for row in self.events))

    def test_slow_capture_cannot_extend_remaining_deadline_even_if_empty(self):
        def capture(spec,*,timeout):self.now=2;return self.snapshot()
        with self.assertRaises(TimeoutError):wait_for_release(self.state(),capture,1,lambda n,row:self.events.append(row),clock=lambda:self.now)
        self.assertTrue(self.events[0]['deadlineExpired'])

    def test_invalid_or_expanded_budget_precedes_capture(self):
        for value in (0,-1,46,float('inf'),float('nan'),True):
            with self.subTest(value=value),self.assertRaises(ValueError):
                wait_for_release(self.state(),lambda *a,**k:self.fail('capture forbidden'),value,lambda *a:None,clock=lambda:0)

    def test_capture_failure_is_not_replaced_with_success(self):
        error=OSError('actual CIM refusal')
        def capture(*args,**kwargs):raise error
        with self.assertRaises(OSError) as actual:wait_for_release(self.state(),capture,1,lambda *a:None,clock=lambda:0)
        self.assertIs(actual.exception,error)

    def test_windows_capture_uses_original_query_binary_output_and_supplied_budget(self):
        with tempfile.TemporaryDirectory() as temporary:
            output=Path(temporary)/'new';calls=[]
            def runner(command,**kwargs):
                calls.append((command,kwargs));return subprocess.CompletedProcess(command,0,json.dumps(self.snapshot()).encode(),b'')
            capture=windows_capture(fresh,'declared-pwsh.exe',output,runner=runner)
            self.assertEqual(capture(self.spec,timeout=1.25),self.snapshot())
            self.assertEqual(calls[0][0][-1],fresh.powershell_command(self.spec,'after'));self.assertEqual(calls[0][1]['timeout'],1.25)
            self.assertFalse(calls[0][1].get('text',False));self.assertEqual((output/'001.stderr').read_bytes(),b'')

    def test_windows_capture_keeps_partial_timeout_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            output=Path(temporary)/'new'
            def runner(command,**kwargs):raise subprocess.TimeoutExpired(command,kwargs['timeout'],output=b'partial',stderr=b'error')
            capture=windows_capture(fresh,'declared-pwsh.exe',output,runner=runner)
            with self.assertRaises(subprocess.TimeoutExpired):capture(self.spec,timeout=.5)
            self.assertEqual((output/'001.stdout').read_bytes(),b'partial');self.assertEqual((output/'001.stderr').read_bytes(),b'error')


if __name__=='__main__':unittest.main()
