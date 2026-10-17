import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import package_environment as environment


class ExplicitTools(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.project=self.root/'project'
        wrapper=self.project/'gradle/wrapper/gradle-wrapper.properties';wrapper.parent.mkdir(parents=True)
        wrapper.write_text('distributionUrl=https\\://example.invalid/gradle-8.14.3-bin.zip\n')
        self.sdk=self.root/'sdk';self.jdk=self.root/'jdk';self.gradle=self.root/'gradle-8.14.3';self.home=self.root/'home';self.home.mkdir()
        self.paths=dict(java=self.jdk/'bin/java.exe',javac=self.jdk/'bin/javac.exe',jvm=self.jdk/'bin/server/jvm.dll',
            aapt2=self.sdk/'build-tools/35.0.0/aapt2.exe',apksigner=self.sdk/'build-tools/35.0.0/lib/apksigner.jar',
            androidJar=self.sdk/'platforms/android-36/android.jar')
        for name,path in self.paths.items():path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
        for name in ('bin/gradle.bat','lib/runtime.jar'):
            path=self.gradle/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
        self.config=dict(sdkRoot=str(self.sdk),jdkRoot=str(self.jdk),gradleRoot=str(self.gradle),gradleHome=str(self.home),
            toolSha256={name:environment.digest(path) for name,path in self.paths.items()},
            gradleFiles=environment.files(self.gradle),gradleHomeConfiguration={})
    def test_complete_declared_runtime_matches_wrapper(self):
        actual=environment.tool_inputs(self.config,self.project)
        self.assertEqual(actual['tools'],self.paths);self.assertEqual(actual['gradle'],self.gradle)
    def test_different_runtime_version_refuses(self):
        wrong=self.root/'gradle-9.0';self.gradle.rename(wrong);self.config['gradleRoot']=str(wrong)
        with self.assertRaisesRegex(ValueError,'wrapper version'):environment.tool_inputs(self.config,self.project)
    def test_changed_packaging_tool_refuses(self):
        self.paths['java'].write_bytes(b'changed Java')
        with self.assertRaisesRegex(ValueError,'packaging tool changed'):environment.tool_inputs(self.config,self.project)
    def test_incomplete_tool_or_gradle_distribution_refuses(self):
        config=copy.deepcopy(self.config);config['toolSha256'].pop('jvm')
        with self.assertRaisesRegex(ValueError,'Complete explicit'):environment.tool_inputs(config,self.project)
        config=copy.deepcopy(self.config);config['gradleFiles'].pop('lib/runtime.jar')
        with self.assertRaisesRegex(ValueError,'distribution differs'):environment.tool_inputs(config,self.project)
    def test_unbound_home_initialization_script_refuses(self):
        (self.home/'init.d').mkdir();(self.home/'init.d/injected.gradle').write_text('println "unexpected"')
        with self.assertRaisesRegex(ValueError,'home configuration differs'):environment.tool_inputs(self.config,self.project)
    def test_shell_expansion_in_batch_arguments_refuses(self):
        for argument in ('path%EXPAND%','path&exit','quoted"path'):
            with self.subTest(argument=argument),self.assertRaisesRegex(ValueError,'Shell expansion'):
                environment.gradle_command(self.root/'cmd.exe',self.gradle/'bin/gradle.bat',[argument])
    def test_inherited_signing_java_and_ci_overrides_are_removed(self):
        prepared=dict(jdk=self.jdk,gradle=self.gradle,home=self.home,sdk=self.sdk)
        actual=environment.environment(prepared,self.root/'out',{'PATH':'original','JAVA_HOME':'foreign',
            'JAVA_TOOL_OPTIONS':'-javaagent:foreign','GRADLE_INIT_SCRIPT':'foreign','SOLOSOUL_KEYSTORE_PATH':'production',
            'TAURI_CONFIG':'foreign','WRY_ANDROID_LIBRARY':'foreign','GITHUB_RUN_NUMBER':'999'})
        self.assertEqual(actual['JAVA_HOME'],str(self.jdk));self.assertEqual(actual['GITHUB_RUN_NUMBER'],'0')
        self.assertFalse(set(actual)&{'SOLOSOUL_KEYSTORE_PATH','GRADLE_INIT_SCRIPT','TAURI_CONFIG','WRY_ANDROID_LIBRARY'})
        self.assertNotIn('foreign',actual['JAVA_TOOL_OPTIONS']);self.assertIn(str(self.root/'out/temp'),actual['JAVA_TOOL_OPTIONS'])


class VersionMetadata(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.workspace=self.root/'workspace'
        self.tauri=self.workspace/'tauri';self.product=self.tauri/'src-tauri';self.product.mkdir(parents=True)
        self.app=dict(version='2.13.2',identifier='com.solosoul.app',bundle=dict(android={}))
        self.platform=dict(bundle=dict(resources={'mobile':'mobile'}))
        cli=self.tauri/'node_modules/@tauri-apps/cli';cli.mkdir(parents=True)
        rule='versionCode = version.major * 1000000 + version.minor * 1000 + version.patch'
        (cli/'config.schema.json').write_text(json.dumps(dict(definitions=dict(AndroidConfig=dict(properties=dict(versionCode=dict(description=rule)))))))
        (cli/'package.json').write_text(json.dumps(dict(version='2.11.2')))
        plan=dict(nodeModules=dict(files={n:environment.entry(self.tauri/'node_modules'/n) for n in
            ('@tauri-apps/cli/config.schema.json','@tauri-apps/cli/package.json')}))
        (self.root/'frontend-plan.json').write_text(json.dumps(plan))
        self.payload=dict(workspace=str(self.workspace),receipt=str(self.root/'frontend-result.json'),sources={})
        self.write()
    def write(self):
        for name,value in (('tauri.conf.json',self.app),('tauri.android.conf.json',self.platform)):
            path=self.product/name;path.write_text(json.dumps(value));self.payload['sources']['tauri/src-tauri/'+name]=environment.entry(path)
    def test_current_version_is_derived_from_bound_cli_rule(self):
        actual=environment.version_properties(self.payload)
        self.assertEqual((actual['versionName'],actual['versionCode'],actual['githubRunNumber']),('2.13.2',2013002,0))
    def test_platform_version_and_explicit_code_are_merged(self):
        self.platform=dict(version='3.2.1',bundle=dict(android=dict(versionCode=99)));self.write()
        actual=environment.version_properties(self.payload)
        self.assertEqual((actual['versionName'],actual['versionCode']),('3.2.1',99))
    def test_auto_increment_and_invalid_or_boolean_codes_refuse(self):
        for value in (dict(autoIncrementVersionCode=True),dict(versionCode=True),dict(versionCode=0),dict(versionCode=2100000001)):
            self.app['bundle']['android']=value;self.write()
            with self.subTest(value=value),self.assertRaises(ValueError):environment.version_properties(self.payload)
    def test_unbound_cli_metadata_refuses(self):
        (self.tauri/'node_modules/@tauri-apps/cli/config.schema.json').write_bytes(b'changed schema')
        with self.assertRaisesRegex(ValueError,'Locked CLI'):environment.version_properties(self.payload)
    def test_foreign_package_identity_and_nonsemantic_version_refuse(self):
        self.app['identifier']='com.other.app';self.write()
        with self.assertRaisesRegex(ValueError,'package identity'):environment.version_properties(self.payload)
        self.app['identifier']='com.solosoul.app';self.app['version']='../package.json';self.write()
        with self.assertRaisesRegex(ValueError,'semantic version'):environment.version_properties(self.payload)


class ExternalArtifacts(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve()
    def test_actual_long_local_dependency_keeps_complete_hash_verification(self):
        path=self.root/('public-'+('x'*195)+'.aar')
        native=Path(chr(92)*2+'?'+chr(92)+str(path)) if os.name=='nt' else path
        native.write_bytes(b'complete public archive');self.addCleanup(native.unlink)
        row=dict(path=str(path),bytes=23,sha256=environment.digest(native))
        if os.name=='nt':self.assertGreaterEqual(len(str(path)),260)
        environment.verify_external_artifact(row)
        native.write_bytes(b'changed public archive')
        with self.assertRaisesRegex(ValueError,'dependency changed'):environment.verify_external_artifact(row)
    def test_relative_traversal_and_device_aliases_refuse_before_reading(self):
        values=['relative.aar',str(self.root/'..'/'outside.aar')]
        if os.name=='nt':values += [chr(92)*2+'server'+chr(92)+'share'+chr(92)+'file.aar',chr(92)*2+'?'+chr(92)+str(self.root/'file.aar')]
        for value in values:
            with self.subTest(value=value),mock.patch.object(environment,'entry') as reader:
                with self.assertRaises(ValueError):environment.verify_external_artifact(dict(path=value,bytes=1,sha256='0'*64))
                reader.assert_not_called()
    def test_incomplete_or_invalid_record_refuses_before_reading(self):
        for row in ({'path':str(self.root/'a.aar')},dict(path=str(self.root/'a.aar'),bytes=True,sha256='0'*64),
                    dict(path=str(self.root/'a.aar'),bytes=1,sha256='bad')):
            with self.subTest(row=row),mock.patch.object(environment,'entry') as reader:
                with self.assertRaises(ValueError):environment.verify_external_artifact(row)
                reader.assert_not_called()


if __name__=='__main__':unittest.main()
