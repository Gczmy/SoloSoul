import contextlib
import copy
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

import prepare_avd as avd


@unittest.skipUnless(os.name == 'nt', 'Windows SDK batch entry')
class PrepareAvd(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.sdk = self.root/'SDK relocated space';self.jdk = self.root/'JDK space'
        self.tools = dict(avdManager=self.sdk/'cmdline-tools/latest/bin/avdmanager.bat',
            java=self.jdk/'bin/java.exe', classpath=self.sdk/'cmdline-tools/latest/lib/avdmanager-classpath.jar',
            imageProperties=self.sdk/'system-images/android-31/default/x86_64/source.properties',
            emulatorPackage=self.sdk/'emulator/package.xml', emulatorExecutable=self.sdk/'emulator/emulator.exe')
        for name,path in self.tools.items():
            path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
        self.tools['imageProperties'].write_text('AndroidVersion.ApiLevel=31\nSystemImage.Abi=x86_64\nSystemImage.TagId=default\n',encoding='utf-8')
        self.config = dict(schemaVersion=1,sdkRoot=str(self.sdk),jdkRoot=str(self.jdk),avdName='SoloSoul_public_test',
            toolSha256={name:avd.digest(path) for name,path in self.tools.items()})
        self.config_file = self.root/'config.json';self.write_config()
        self.output = self.root/'result space'
        self.calls = []

    def write_config(self):self.config_file.write_text(json.dumps(self.config),encoding='utf-8')

    def runner(self,command,**kwargs):
        self.calls.append((command,kwargs))
        destination=self.output/'avd'/(self.config['avdName']+'.avd');destination.mkdir()
        (destination/'config.ini').write_bytes(b'sdk generated config\r\n')
        return subprocess.CompletedProcess(command,0,b'created',b'')

    def run_prepare(self,runner=None):return avd.prepare(self.config_file,self.output,runner=runner or self.runner)

    def test_success_creates_only_new_avd_and_preserves_raw_sdk_profile(self):
        before=copy.deepcopy(dict(os.environ));record=self.run_prepare()
        self.assertTrue(record['passed']);self.assertFalse(record['emulatorStarted'])
        self.assertEqual(record['adbCommandsExecuted'],0);self.assertFalse(record['nativeRuntimeAccepted'])
        self.assertEqual(record['configSha256'],avd.PROFILE_SHA256)
        self.assertEqual((self.output/'avdmanager-original-config.ini').read_bytes(),b'sdk generated config\r\n')
        self.assertEqual((self.output/'config.json').read_bytes(),self.config_file.read_bytes())
        self.assertEqual(dict(os.environ),before);self.assertEqual(len(self.calls),1)

    def test_original_manager_budget_input_and_hidden_creation_are_preserved(self):
        self.run_prepare();command,options=self.calls[0]
        self.assertIn(' /d /s /c ""',command)
        self.assertIn('"'+str(self.tools['avdManager'])+'"',command)
        self.assertIn('--package "system-images;android-31;default;x86_64" --device pixel_2',command)
        self.assertIn('--path "'+str(self.output/'avd/SoloSoul_public_test.avd')+'""',command)
        self.assertEqual(options['timeout'],120);self.assertEqual(options['input'],b'no\n')
        self.assertEqual(options['creationflags'],subprocess.CREATE_NO_WINDOW)

    def test_java_and_android_state_are_scoped_to_new_output(self):
        with mock.patch.dict(os.environ,{'JAVA_OPTS':'unsafe inherited option','JDK_JAVA_OPTIONS':'inherited',
                                        'SKIP_JDK_VERSION_CHECK':'1'}):self.run_prepare()
        env=self.calls[0][1]['env']
        for key in ('TEMP','TMP'):self.assertEqual(env[key],str(self.output/'temp'))
        for key in ('JAVA_OPTS','AVDMANAGER_OPTS','_JAVA_OPTIONS','JDK_JAVA_OPTIONS'):self.assertEqual(env[key],'')
        self.assertNotIn('SKIP_JDK_VERSION_CHECK',env)
        self.assertEqual(env['JAVA_TOOL_OPTIONS'],f'-Duser.home="{self.output/"android-user"}" -Djava.io.tmpdir="{self.output/"temp"}"')
        self.assertEqual(env['ANDROID_AVD_HOME'],str(self.output/'avd'))
        self.assertEqual(env['JAVA_HOME'],str(self.jdk))

    def test_existing_output_and_its_sentinel_are_never_overwritten(self):
        self.output.mkdir();sentinel=self.output/'keep';sentinel.write_bytes(b'keep')
        with self.assertRaisesRegex(ValueError,'New output'):self.run_prepare()
        self.assertEqual(sentinel.read_bytes(),b'keep');self.assertEqual(self.calls,[])

    def test_missing_output_parent_does_not_create_a_tree_or_run_sdk(self):
        self.output=self.root/'missing'/'result'
        with self.assertRaises(OSError):self.run_prepare()
        self.assertFalse(self.output.parent.exists());self.assertEqual(self.calls,[])

    def test_incomplete_config_unknown_fields_and_personal_avd_names_refuse(self):
        original=copy.deepcopy(self.config)
        for change in [dict(schemaVersion=True),dict(typo=True),dict(avdName='personal'),dict(avdName='SoloSoul_x&whoami'),dict(toolSha256={})]:
            self.config=dict(original,**change);self.write_config()
            with self.subTest(change=change),self.assertRaises(ValueError):self.run_prepare()
            self.assertFalse(self.output.exists())
        self.assertEqual(self.calls,[])

    def test_duplicate_json_and_nonfinite_values_refuse_before_sdk(self):
        for raw in ['{"schemaVersion":1,"schemaVersion":1}','{"schemaVersion":NaN}']:
            self.config_file.write_text(raw,encoding='utf-8')
            with self.subTest(raw=raw),self.assertRaises(ValueError):self.run_prepare()
        self.assertEqual(self.calls,[]);self.assertFalse(self.output.exists())

    def test_changed_tool_or_wrong_system_image_refuses_before_sdk(self):
        for name,path in self.tools.items():
            old=path.read_bytes();path.write_bytes(old+b'changed')
            with self.subTest(name=name),self.assertRaisesRegex(ValueError,'tool changed'):self.run_prepare()
            path.write_bytes(old)
        self.tools['imageProperties'].write_text('AndroidVersion.ApiLevel=35\nSystemImage.Abi=x86_64\nSystemImage.TagId=default\n',encoding='utf-8')
        self.config['toolSha256']['imageProperties']=avd.digest(self.tools['imageProperties']);self.write_config()
        with self.assertRaisesRegex(ValueError,'API31'):self.run_prepare()
        self.assertEqual(self.calls,[]);self.assertFalse(self.output.exists())

    def test_frozen_profile_change_refuses_before_sdk(self):
        bad=self.root/'profile.ini';bad.write_bytes(b'other hardware')
        with mock.patch.object(avd,'PROFILE',bad),self.assertRaisesRegex(ValueError,'hardware profile'):self.run_prepare()
        self.assertEqual(self.calls,[]);self.assertFalse(self.output.exists())

    def test_missing_emulator_package_refuses_before_sdk_creates_an_avd(self):
        for key in ('emulatorPackage','emulatorExecutable'):
            path=self.tools[key];original=path.read_bytes();path.unlink()
            with self.subTest(key=key),self.assertRaises(FileNotFoundError):self.run_prepare()
            path.write_bytes(original)
            self.assertEqual(self.calls,[]);self.assertFalse(self.output.exists())

    def test_shell_expansion_in_paths_is_refused_while_spaces_are_kept(self):
        self.assertEqual(avd.command_path('D:/SDK space'),'D:/SDK space')
        for character in ('%','!','^','&','|','<','>','"','\n','\r','\x00'):
            with self.subTest(character=repr(character)),self.assertRaises(ValueError):
                avd.build_command('cmd.exe','D:/sdk/avdmanager.bat','SoloSoul_x','D:/result'+character+'injected')

    def test_local_sdk_code_page_cannot_silently_replace_path_characters(self):
        path=str(self.root/'result 中文 🙂')
        try:path.encode('mbcs')
        except UnicodeError:
            self.output=Path(path)
            with self.assertRaisesRegex(ValueError,'code page'):self.run_prepare()
            self.assertEqual(self.calls,[]);self.assertFalse(self.output.exists())
        else:self.assertEqual(avd.command_path(path),path)

    def test_sdk_nonzero_exit_preserves_both_streams_and_failure_receipt(self):
        def failed(command,**kwargs):
            self.calls.append((command,kwargs));return subprocess.CompletedProcess(command,1,b'partial stdout',b'actual stderr')
        record=self.run_prepare(failed)
        self.assertFalse(record['passed']);self.assertEqual(record['exitCode'],1)
        self.assertEqual(record['errorType'],'CalledProcessError')
        self.assertEqual((self.output/'avd-create.stdout.log').read_bytes(),b'partial stdout')
        self.assertEqual((self.output/'avd-create.stderr.log').read_bytes(),b'actual stderr')
        self.assertFalse((self.output/'avdmanager-original-config.ini').exists())

    def test_sdk_timeout_keeps_partial_output_and_cannot_become_success(self):
        def timed_out(command,**kwargs):raise subprocess.TimeoutExpired(command,120,output=b'partial',stderr=b'error')
        record=self.run_prepare(timed_out)
        self.assertFalse(record['passed']);self.assertEqual(record['errorType'],'TimeoutExpired')
        self.assertEqual((self.output/'avd-create.stdout.log').read_bytes(),b'partial')
        self.assertEqual((self.output/'avd-create.stderr.log').read_bytes(),b'error')

    def test_changed_config_during_sdk_execution_is_not_given_passing_profile(self):
        def changed(command,**kwargs):
            result=self.runner(command,**kwargs);self.config_file.write_bytes(b'changed');return result
        record=self.run_prepare(changed)
        self.assertFalse(record['passed']);self.assertEqual(record['errorType'],'ValueError')
        self.assertIn('input changed',record['error'])
        self.assertEqual((self.output/'avd/SoloSoul_public_test.avd/config.ini').read_bytes(),b'sdk generated config\r\n')

    def test_changed_sdk_during_execution_is_preserved_and_refused(self):
        def changed(command,**kwargs):
            result=self.runner(command,**kwargs);self.tools['classpath'].write_bytes(b'changed');return result
        record=self.run_prepare(changed)
        self.assertFalse(record['passed']);self.assertIn('input changed',record['error'])
        self.assertTrue((self.output/'avdmanager-original-config.ini').is_file())

    def test_missing_generated_config_is_not_a_successful_preparation(self):
        record=self.run_prepare(lambda command,**kwargs:subprocess.CompletedProcess(command,0,b'no config',b''))
        self.assertFalse(record['passed']);self.assertEqual(record['errorType'],'FileNotFoundError')

    def test_cli_existing_output_never_calls_the_sdk(self):
        self.output.mkdir()
        with mock.patch('subprocess.run') as sdk,contextlib.redirect_stderr(io.StringIO()):
            code=avd.main(['--config',str(self.config_file),'--output',str(self.output)])
        self.assertEqual(code,2);sdk.assert_not_called()


if __name__ == '__main__':unittest.main()
