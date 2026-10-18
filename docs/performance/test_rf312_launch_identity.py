import copy
import os
import unittest
from rf312_launch_identity import wait_for_roots


class RootReadinessTests(unittest.TestCase):
    def setUp(self):
        self.now=0;self.live=True;self.events=[];self.budgets=[]
        self.specs=[dict(Name=name,ProcessId=identifier,ParentProcessId=os.getpid(),ExecutablePath=path,
                         CommandLine='explicit '+name,BornNotBefore='2026-01-01T00:00:00+00:00')
            for name,identifier,path in [('adb.exe',10,'X:/sdk/platform-tools/adb.exe'),('emulator.exe',11,'X:/sdk/emulator/emulator.exe')]]
        self.rows=[{**{k:v for k,v in spec.items() if k!='BornNotBefore'},'CreationDate':'2026-01-01T00:00:01+00:00'} for spec in self.specs]

    def snapshot(self,*,timeout):self.budgets.append(timeout);return copy.deepcopy(self.rows)
    def sleep(self,value):self.now+=value
    def wait(self,snapshot=None,deadline=10,record=None):
        return wait_for_roots(snapshot or self.snapshot,self.specs,lambda:self.live,deadline,
            record or (lambda n,row:self.events.append(row)),clock=lambda:self.now,sleep=self.sleep)

    def test_complete_actual_rows_are_returned_without_inferred_values(self):
        self.assertEqual(self.wait(),self.rows)
        self.assertFalse(self.events[0]['missingValuesInferred']);self.assertFalse(self.events[0]['ownershipGranted'])

    def test_missing_executable_waits_for_actual_observation_with_same_deadline(self):
        def sampled(**kwargs):
            rows=self.snapshot(**kwargs)
            if self.now<.2:rows[1]['ExecutablePath']=None
            return rows
        self.assertEqual(self.wait(sampled),self.rows)
        self.assertEqual(self.budgets,[10,9.9,9.8])
        self.assertIsNone(self.events[0]['observedRoots'][1]['ExecutablePath'])
        self.assertTrue(self.events[-1]['identityRowsComplete'])

    def test_persistent_missing_metadata_exhausts_original_budget_without_acceptance(self):
        self.rows[1]['ExecutablePath']=None
        with self.assertRaises(TimeoutError):self.wait(deadline=.25)
        self.assertEqual(self.now,.25)
        self.assertFalse(any(row['identityRowsComplete'] for row in self.events))

    def test_missing_row_is_not_inferred_dead_but_can_later_be_observed(self):
        def sampled(**kwargs):
            rows=self.snapshot(**kwargs);return rows[:1] if self.now==0 else rows
        self.assertEqual(self.wait(sampled),self.rows)
        self.assertEqual(len(self.events[0]['observedRoots']),1)

    def test_dead_process_handle_rejects_before_snapshot(self):
        self.live=False
        with self.assertRaises(RuntimeError):self.wait()
        self.assertEqual(self.budgets,[])

    def test_wrong_executable_command_parent_or_older_birth_rejected(self):
        for update in [dict(ExecutablePath='X:/other/emulator.exe'),dict(CommandLine='wrong'),
                       dict(ParentProcessId=os.getpid()+1),dict(CreationDate='2025-01-01T00:00:00+00:00')]:
            def sampled(**kwargs):
                rows=self.snapshot(**kwargs);rows[1].update(update);return rows
            with self.subTest(update=update),self.assertRaises(RuntimeError):self.wait(sampled)

    def test_known_birth_changes_or_fields_disappear_while_waiting_are_rejected(self):
        for update in [dict(CreationDate='2026-01-01T00:00:02+00:00'),dict(CreationDate=None)]:
            def sampled(**kwargs):
                rows=self.snapshot(**kwargs);rows[1]['ExecutablePath']=None
                if self.now>0:rows[1].update(update)
                return rows
            with self.subTest(update=update),self.assertRaises(RuntimeError):self.wait(sampled)
            self.now=0

    def test_duplicate_root_is_rejected(self):
        with self.assertRaises(RuntimeError):self.wait(lambda **kwargs:self.rows+[self.rows[0]])

    def test_snapshot_and_recording_cannot_extend_boot_budget(self):
        def slow(**kwargs):self.now=10;return self.rows
        with self.assertRaises(TimeoutError):self.wait(slow)
        self.assertEqual(self.events,[]);self.now=0
        def slow_record(*args):self.now=10
        with self.assertRaises(TimeoutError):self.wait(record=slow_record)

    def test_invalid_deadlines_and_specifications_precede_os_reads(self):
        for value in [0,241,True,float('nan'),float('inf')]:
            with self.subTest(value=value),self.assertRaises(ValueError):self.wait(deadline=value)
        self.specs[1]['ParentProcessId']+=1
        with self.assertRaises(ValueError):self.wait()
        self.assertEqual(self.budgets,[])

    def test_snapshot_failure_preserves_original_exception(self):
        expected=OSError('CIM refused')
        def failed(**kwargs):raise expected
        with self.assertRaises(OSError) as caught:self.wait(failed)
        self.assertIs(caught.exception,expected)

    def test_untyped_sdk_name_is_rejected_before_os_reads(self):
        self.specs[1]['Name']=[]
        with self.assertRaises(ValueError):self.wait()
        self.assertEqual(self.budgets,[])


if __name__=='__main__':unittest.main()
