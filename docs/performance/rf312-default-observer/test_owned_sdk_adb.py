import copy
import unittest
from pathlib import PureWindowsPath
from owned_sdk_adb import is_owned_sdk_client


class OwnedSdk(unittest.TestCase):
    def setUp(self):
        self.sdk=PureWindowsPath('X:/SDK with spaces/sdk');self.avd='SoloSoul_unit_only'
        def row(name,pid,parent,offset,path,command):
            return dict(Name=name,ProcessId=pid,ParentProcessId=parent,CreationDate=f'2020-01-01T00:00:{offset:02d}+00:00',
                        ExecutablePath=str(path),CommandLine=command)
        self.root=row('emulator.exe',10,5,1,self.sdk/'emulator/emulator.exe','"'+str(self.sdk/'emulator/emulator.exe')+'" -avd '+self.avd)
        qemu=row('qemu-system-x86_64.exe',11,10,2,self.sdk/'emulator/qemu/windows-x86_64/qemu-system-x86_64.exe','qemu -avd '+self.avd)
        self.client=row('adb.exe',12,11,3,self.sdk/'platform-tools/adb.exe','"'+str(self.sdk/'platform-tools/adb.exe')+'" -s emulator-5578 shell settings put system screen_off_timeout 2147483647')
        self.rows=[self.root,qemu,self.client]

    def accepted(self):return is_owned_sdk_client(self.client,self.rows,self.root,self.sdk,self.avd,'emulator-5578')

    def test_only_original_three_bootstrap_commands_are_admitted(self):
        for tail in ('settings put system screen_off_timeout 2147483647','settings put system accelerometer_rotation 1',
                     'am broadcast -a com.android.emulator.multidisplay.START -n com.android.emulator.multidisplay/.MultiDisplayServiceReceiver "--user 0"'):
            self.client['CommandLine']='"'+str(self.sdk/'platform-tools/adb.exe')+'" -s emulator-5578 shell '+tail
            self.assertTrue(self.accepted())

    def test_logcat_and_input_are_never_owned_even_with_correct_ancestry(self):
        for tail in ('logcat -G 2M','input tap 100 100','am start -W -n com.solosoul.app/.MainActivity'):
            self.client['CommandLine']='"'+str(self.sdk/'platform-tools/adb.exe')+'" -s emulator-5578 shell '+tail
            self.assertFalse(self.accepted())

    def test_missing_executable_root_birth_parent_or_ancestor_refuses(self):
        initial=copy.deepcopy((self.client,self.rows,self.root))
        for kind in ('executable','birth','parent','ancestor'):
            self.client,self.rows,self.root=copy.deepcopy(initial)
            if kind=='executable':self.client['ExecutablePath']=None
            if kind=='birth':self.client['CreationDate']='2019-01-01T00:00:00+00:00'
            if kind=='parent':self.client['ParentProcessId']=999
            if kind=='ancestor':self.rows=[r for r in self.rows if r['ProcessId']!=10]
            self.assertFalse(self.accepted())

    def test_reused_root_cycle_other_vm_and_other_sdk_refuse(self):
        initial=copy.deepcopy((self.client,self.rows,self.root))
        for kind in ('reused','cycle','vm','sdk'):
            self.client,self.rows,self.root=copy.deepcopy(initial)
            if kind=='reused':self.rows[0]['CreationDate']='2021-01-01T00:00:00+00:00'
            if kind=='cycle':self.rows[1]['ParentProcessId']=11
            if kind=='vm':self.rows[1]['CommandLine']='qemu -avd SoloSoul_other'
            if kind=='sdk':self.client['ExecutablePath']='X:/other/adb.exe'
            self.assertFalse(self.accepted())

    def test_duplicate_pid_and_other_serial_cannot_grant_ownership(self):
        self.assertFalse(is_owned_sdk_client(self.client,self.rows,self.root,self.sdk,self.avd,'emulator-5580'))
        self.rows.append(copy.deepcopy(self.rows[1]));self.assertFalse(self.accepted())

    def test_system_idle_row_is_not_an_sdk_ancestor(self):
        self.rows.append(dict(Name='System Idle Process',ProcessId=0,ParentProcessId=0,CreationDate=None,ExecutablePath=None,CommandLine=None))
        self.assertTrue(self.accepted())


if __name__=='__main__':unittest.main()
