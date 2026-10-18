import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import owner_inputs as owner
import prepare_avd as avd


@unittest.skipUnless(os.name=='nt','Windows SDK/host contract')
class OwnerInputs(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup);self.root=Path(temporary.name).resolve()
        self.sdk=self.root/'SDK space';self.jdk=self.root/'JDK space';self.workspace=self.root/'workspace'
        tool_paths=dict(avdManager=self.sdk/'cmdline-tools/latest/bin/avdmanager.bat',java=self.jdk/'bin/java.exe',
            classpath=self.sdk/'cmdline-tools/latest/lib/avdmanager-classpath.jar',imageProperties=self.sdk/'system-images/android-31/default/x86_64/source.properties',
            emulatorPackage=self.sdk/'emulator/package.xml',emulatorExecutable=self.sdk/'emulator/emulator.exe')
        for name,path in tool_paths.items():path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
        tool_paths['imageProperties'].write_text('AndroidVersion.ApiLevel=31\nSystemImage.Abi=x86_64\nSystemImage.TagId=default\n',encoding='utf-8')
        for path in (self.sdk/'build-tools/35.0.0/aapt2.exe',self.sdk/'build-tools/35.0.0/lib/apksigner.jar'):
            path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'not executed')
        self.prepared=self.root/'prepared';self.prepared.mkdir();self.avd_dir=self.prepared/'avd/SoloSoul_unit.avd';self.avd_dir.mkdir(parents=True)
        (self.avd_dir/'config.ini').write_bytes(avd.PROFILE.read_bytes());(self.avd_dir/'userdata.img').write_bytes(b'fresh SDK image')
        self.avd_dir.with_suffix('.ini').write_text('path='+str(self.avd_dir),encoding='utf-8')
        config=dict(schemaVersion=1,sdkRoot=str(self.sdk),jdkRoot=str(self.jdk),avdName='SoloSoul_unit',
            toolSha256={name:avd.digest(path) for name,path in tool_paths.items()})
        self.dump(self.prepared/'config.json',config)
        self.receipt=dict(passed=True,freshSdkUserdata=True,createdNewDedicatedAvd=True,oldUserdataCopied=False,
            emulatorStarted=False,exitCode=0,preparationSourceSha256=avd.digest(avd.S/'prepare_avd.py'),configSha256=avd.PROFILE_SHA256,
            toolSha256=config['toolSha256'],name='SoloSoul_unit',avdDirectory=str(self.avd_dir))
        self.dump(self.prepared/'avd-preparation.json',self.receipt)
        self.runtime=dict(host=dict(workspaceRoot=str(self.workspace),sdkRoot=str(self.sdk),avd='SoloSoul_unit',serial='emulator-5578'))
        host={}
        for name in owner.HOST_FILES:
            path=self.workspace/'tauri/scripts'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'canonical test reader')
            host[name]=avd.digest(path)
        self.dump(self.root/'runtime.json',self.runtime);self.dump(self.root/'units.json',dict(passed=True))
        shell=self.root/'pwsh.exe';shell.write_bytes(b'not executed PowerShell')
        self.config=dict(schemaVersion=1,runtimeInputs=str(self.root/'runtime.json'),avdPreparation=str(self.prepared/'avd-preparation.json'),
            powershell=str(shell),unitEvidence=str(self.root/'units.json'),hostSourceSha256=host,packageReference={})
        for key in ('runtimeInputs','avdPreparation','powershell','unitEvidence'):self.config[key+'Sha256']=avd.digest(Path(self.config[key]))

    def dump(self,path,value):path.write_text(json.dumps(value),encoding='utf-8')

    def validate(self):
        with mock.patch.object(owner,'load_inputs',return_value=self.runtime),mock.patch.object(owner,'validate_units') as units:
            actual=owner.validate_config(self.config)
        return actual,units

    def test_all_declared_inputs_and_prepared_device_bind_before_any_sdk(self):
        with mock.patch('subprocess.run',side_effect=AssertionError('Validation cannot execute SDK')):
            actual,units=self.validate()
        self.assertEqual(actual[0],self.runtime);self.assertEqual(actual[2],self.sdk);units.assert_called_once()

    def test_unknown_config_missing_host_source_and_changed_receipt_refuse(self):
        initial=copy.deepcopy(self.config)
        for change in (dict(typo=True),dict(hostSourceSha256={}),dict(avdPreparationSha256='0'*64)):
            self.config=dict(initial,**change)
            with self.subTest(change=change),self.assertRaises(ValueError):self.validate()

    def test_host_or_restore_reader_mutation_is_not_a_new_accepted_baseline(self):
        path=self.workspace/'tauri/scripts/android-host-audit.py';path.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'Canonical'):self.validate()

    def test_stale_unit_evidence_and_changed_avd_name_refuse(self):
        self.dump(self.root/'units.json',dict(passed=False))
        with self.assertRaises(ValueError):self.validate()
        self.config['unitEvidenceSha256']=avd.digest(self.root/'units.json')
        self.runtime['host']['avd']='SoloSoul_other'
        with self.assertRaisesRegex(ValueError,'fresh unbooted'):self.validate()

    def test_already_booted_avd_or_modified_hardware_profile_refuses(self):
        marker=self.avd_dir/'hardware-qemu.ini';marker.write_bytes(b'booted')
        with self.assertRaisesRegex(ValueError,'Already booted'):self.validate()
        marker.unlink();(self.avd_dir/'config.ini').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'hardware configuration'):self.validate()

    def test_sdk_identity_cannot_silently_select_another_root(self):
        self.runtime['host']['sdkRoot']=str(self.root/'different SDK')
        with self.assertRaisesRegex(ValueError,'exact SDK'):self.validate()

    def test_complete_fresh_inventory_rejects_late_file_directory_or_byte_changes(self):
        plan=dict(avdConfig=str(self.avd_dir/'config.ini'),freshAvdInventory=owner.avd_inventory(self.avd_dir),
            avdRegistry=str(self.avd_dir.with_suffix('.ini')),avdRegistrySha256=avd.digest(self.avd_dir.with_suffix('.ini')))
        owner.verify_fresh_avd(plan)
        (self.avd_dir/'late.lock').mkdir()
        with self.assertRaises(ValueError):owner.verify_fresh_avd(plan)
        (self.avd_dir/'late.lock').rmdir();(self.avd_dir/'userdata.img').write_bytes(b'changed')
        with self.assertRaises(ValueError):owner.verify_fresh_avd(plan)

    def test_declared_sdk_binary_change_after_preparation_is_not_accepted(self):
        package_file=self.root/'package-result.json';self.dump(package_file,dict(passed=True))
        config_file=self.prepared/'config.json';avd_config=owner.read(config_file)
        _,_,tool_paths,_,_=avd.inputs(avd_config,self.prepared/'not-created')
        plan=dict(schemaVersion=1,kind='owned-default-twelve-step-runtime',helperRoot=str(owner.S),ownerSourceFiles=owner.sources(),
            config=self.config,runtime=self.runtime,avdConfig=str(self.avd_dir/'config.ini'),
            packageToolSha256={},packageTools={},avdPreparationConfig=str(config_file),avdPreparationConfigSha256=avd.digest(config_file),
            avdTools={name:str(path) for name,path in tool_paths.items()},avdToolSha256=avd_config['toolSha256'],
            packageVerificationResult=str(package_file),packageVerificationResultSha256=avd.digest(package_file),packageVerification=dict(passed=True))
        with mock.patch.object(owner,'verify_sources'):
            owner.verify_plan(plan)
            tool_paths['emulatorExecutable'].write_bytes(b'changed after preparation')
            with self.assertRaisesRegex(ValueError,'Frozen declared AVD tool'):owner.verify_plan(plan)


if __name__=='__main__':unittest.main()
