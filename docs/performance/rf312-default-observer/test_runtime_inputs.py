import copy
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import contextlib

import runtime_inputs as runtime


def encode(value):return json.dumps(value).encode('utf-8')


class SourceNames(unittest.TestCase):
    def test_absolute_and_relative_sources_rebind_without_dropping_hashes(self):
        original = {'tauri/src/main.ts': 'a'*64, 'D:\\old\\repo\\tauri\\mobile\\one.bin': 'b'*64}
        actual,external = runtime.source_sets(original, 'D:\\old\\repo',{})
        self.assertEqual(external,{})
        self.assertEqual(actual, {'tauri/src/main.ts': 'a'*64, 'tauri/mobile/one.bin': 'b'*64})
        self.assertEqual(len(actual), len(original))
        self.assertEqual(len(original), 2)

    def test_external_roots_drive_relative_paths_and_traversal_are_not_rebound(self):
        for value in ['D:\\other\\x', 'C:relative.py', '\\rooted.py', '../outside.py', 'a/../b.py', '']:
            with self.subTest(value=value), self.assertRaises(ValueError):
                runtime.source_sets({value: 'a'*64}, 'D:\\old\\repo',{})

    def test_alias_collisions_cannot_silently_reduce_original_source_set(self):
        with self.assertRaisesRegex(ValueError, 'collapse'):
            runtime.source_sets({'a.py': 'a'*64, 'D:\\old\\repo\\a.py': 'a'*64}, 'D:\\old\\repo',{})

    def test_bad_hashes_duplicate_json_and_nonfinite_constants_refuse(self):
        for value in [True, '', 'G'*64, 'a'*63]:
            with self.subTest(value=value), self.assertRaises(ValueError):runtime.hash_value(value)
        for raw in [b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}', b'\x81']:
            with self.subTest(raw=raw), self.assertRaises((ValueError, UnicodeError)):runtime.decode(raw)


class BindingFixture:
    def setUp(self):
        temp = tempfile.TemporaryDirectory();self.addCleanup(temp.cleanup)
        self.root = Path(temp.name).resolve();self.workspace = self.root/'workspace';self.workspace.mkdir()
        source = self.workspace/'tauri/scripts/android-home-native-regression.py';source.parent.mkdir(parents=True)
        source.write_bytes((runtime.S/'process_identity.py').read_bytes())
        self.image = self.workspace/'image.png';self.image.write_bytes(b'public image bytes')
        self.sdk = self.root/'sdk';(self.sdk/'platform-tools').mkdir(parents=True)
        self.adb = self.sdk/'platform-tools/adb.exe';self.adb.write_bytes(b'not executed test SDK')
        self.artifacts = {}
        for name in runtime.ARTIFACT_ROLES:
            path = self.root/(name+'.bin');path.write_bytes(name.encode());self.artifacts[name] = str(path)
        self.fixtures = {}
        for size in ('100','5000'):
            path = self.root/('fixture-'+size);path.mkdir();self.fixtures[size] = str(path)
        self.reference = dict(runtimeSourceFiles={'D:\\old\\repo\\image.png': runtime.digest(self.image)},
            release=dict(apk=dict(path='old-release', sha256='a'*64, bytes=7), library=dict(path='old-lib')),
            originalDebug=dict(path='old-debug',copy='old-debug'),originalTest=dict(path='old-test',copy='old-test'),
            runtimeTest=dict(path='old-runtime'),fixedFixtures={n:dict(root='old-'+n,files={}) for n in ('100','5000')},
            publicOcrImage=dict(path='D:\\old\\repo\\image.png',sha256=runtime.digest(self.image)))
        self.config = dict(schemaVersion=1,referenceWorkspace='D:\\old\\repo',workspaceRoot=str(self.workspace),
            sdkRoot=str(self.sdk),adbSha256=runtime.digest(self.adb),serial='emulator-5578',avd='SoloSoul_public_test',
            artifactPaths=self.artifacts,fixtureRoots=self.fixtures,externalSourceRoots={})

    def bind(self):return runtime.bind_reference(self.reference,self.config,dict(apk='old-observer'))


class Bindings(BindingFixture, unittest.TestCase):
    def test_rebinding_preserves_reference_and_changes_only_declared_paths(self):
        original = copy.deepcopy(self.reference);inputs = self.bind()
        self.assertEqual(self.reference, original)
        self.assertEqual(inputs['release']['apk']['sha256'], 'a'*64)
        self.assertEqual(inputs['release']['apk']['path'], self.artifacts['release'])
        self.assertEqual(inputs['originalTest']['path'], inputs['originalTest']['copy'])
        self.assertEqual(inputs['publicOcrImage']['path'], str(self.image))
        self.assertEqual(inputs['host']['adbExecutable'], str(self.adb))
        self.assertEqual(set(inputs['fullJourneyFiles']), set(runtime.RUNTIME_FILES))

    def test_source_and_sdk_mutations_refuse_before_any_runtime_input(self):
        inputs = self.bind();runtime.verify_sources(inputs)
        self.image.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'product'):runtime.verify_sources(inputs)
        self.image.write_bytes(b'public image bytes');self.adb.write_bytes(b'changed SDK')
        with self.assertRaisesRegex(ValueError, 'SDK'):runtime.verify_sources(inputs)

    def test_source_names_cannot_escape_runtime_or_workspace_root(self):
        for name in ['/absolute', '../outside', 'C:/absolute', 'a\\b', True]:
            with self.subTest(name=name), self.assertRaises(ValueError):runtime.source_path(self.workspace,name)

    def test_external_frontend_sources_require_explicit_roots_and_keep_all_hashes(self):
        folder=self.root/'frontend';folder.mkdir();(folder/'index.html').write_bytes(b'fixed frontend')
        self.reference['runtimeSourceFiles']['D:\\old\\frontend\\index.html']=runtime.digest(folder/'index.html')
        with self.assertRaisesRegex(ValueError,'explicit external'):self.bind()
        self.config['externalSourceRoots']={'frontend':dict(referenceRoot='D:\\old\\frontend',root=str(folder))}
        inputs=self.bind();runtime.verify_sources(inputs)
        self.assertEqual(inputs['originalSourceEntries'],2)
        self.assertEqual(len(inputs['runtimeSourceFiles']),1)
        self.assertEqual(inputs['externalSourceFiles']['frontend']['files'],{'index.html':runtime.digest(folder/'index.html')})
        (folder/'index.html').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'external'):runtime.verify_sources(inputs)

    def test_missing_external_bindings_cannot_be_renamed_into_a_smaller_set(self):
        inputs=self.bind();inputs['originalSourceEntries']+=1
        with self.assertRaisesRegex(ValueError,'dropped'):runtime.validate_shape(inputs)

    def test_incomplete_runtime_set_and_changed_entry_binding_refuse(self):
        inputs = self.bind();inputs['fullJourneyFiles'].pop('memory_checkpoint.py')
        with self.assertRaisesRegex(ValueError, 'Complete runtime'):runtime.verify_sources(inputs)
        inputs = self.bind();inputs['externalObserverSha256'] = '0'*64
        with self.assertRaisesRegex(ValueError, 'entry'):runtime.validate_shape(inputs)

    def test_relocation_cannot_select_another_sdk_or_nondedicated_device(self):
        for key,value in [('serial','physical-device'),('avd','personal'),('adbExecutable',str(self.root/'other-adb.exe'))]:
            inputs = self.bind();inputs['host'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):runtime.validate_shape(inputs)

    def test_optional_or_debug_build_provenance_is_not_accepted_as_default(self):
        for change in [dict(acceptedPackagePreparation=False),dict(defaultFeaturesOnly=False),
                       dict(applicationDebuggable=True),dict(observerMarkersPresent=True)]:
            inputs = self.bind();inputs['release'].update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):runtime.verify_artifacts(inputs)


class Cli(BindingFixture, unittest.TestCase):
    def setUp(self):
        super().setUp()
        self.reference_file = self.root/'reference.json';self.reference_file.write_bytes(encode(self.reference))
        self.package_file = self.root/'observer.json';self.package_file.write_bytes(encode(dict(apk='old-observer')))
        self.config.update(referenceInputs=str(self.reference_file),referenceInputsSha256=runtime.digest(self.reference_file),
            observerPackage=str(self.package_file),observerPackageSha256=runtime.digest(self.package_file))
        self.config_file = self.root/'config.json';self.config_file.write_bytes(encode(self.config))
        self.output = self.root/'output'

    def invoke(self, *, source_effect=None, artifact_effect=None):
        out,err = io.StringIO(),io.StringIO()
        with contextlib.redirect_stdout(out),contextlib.redirect_stderr(err), \
                mock.patch.object(runtime,'verify_sources',side_effect=source_effect) as sources, \
                mock.patch.object(runtime,'verify_artifacts',side_effect=artifact_effect) as artifacts:
            code = runtime.main(['--config',str(self.config_file),'--output',str(self.output)])
        return code,out.getvalue(),err.getvalue(),sources,artifacts

    def test_preparation_records_exact_reference_binding_and_never_runs_sdk(self):
        with mock.patch('subprocess.run',side_effect=AssertionError('Preparation cannot execute SDK')):
            code,text,_,sources,artifacts = self.invoke()
        self.assertEqual(code,0);self.assertTrue(json.loads(text)['passed'])
        self.assertEqual(sources.call_count,1);self.assertEqual(artifacts.call_count,1)
        self.assertEqual((self.output/'config.json').read_bytes(),self.config_file.read_bytes())
        self.assertEqual(json.loads(text)['inputsSha256'],runtime.digest(self.output/'inputs.json'))

    def test_existing_output_is_not_overwritten_or_verified(self):
        self.output.mkdir();sentinel = self.output/'keep';sentinel.write_bytes(b'keep')
        code,_,_,sources,artifacts = self.invoke()
        self.assertEqual(code,2);sources.assert_not_called();artifacts.assert_not_called()
        self.assertEqual(sentinel.read_bytes(),b'keep')

    def test_reference_receipt_hash_or_unknown_config_field_refuses_before_output(self):
        for value in [dict(referenceInputsSha256='0'*64),dict(observerPackageSha256='0'*64),dict(typo=True)]:
            config = dict(self.config,**value);self.config_file.write_bytes(encode(config))
            code,_,_,sources,_ = self.invoke()
            self.assertEqual(code,2);sources.assert_not_called();self.assertFalse(self.output.exists())

    def test_changed_preparation_input_or_artifact_failure_is_preserved(self):
        def changed(_inputs):self.reference_file.write_bytes(b'changed')
        code,text,_,_,_ = self.invoke(source_effect=changed)
        self.assertEqual(code,1);self.assertFalse(json.loads(text)['passed'])
        self.assertFalse((self.output/'inputs.json').exists())
        self.assertIn('changed during',(self.output/'binding-result.json').read_text())

    def test_artifact_refusal_is_not_overwritten_with_a_success_manifest(self):
        code,text,_,_,_ = self.invoke(artifact_effect=ValueError('original strict artifact refusal'))
        self.assertEqual(code,1);self.assertFalse(json.loads(text)['passed'])
        self.assertFalse((self.output/'inputs.json').exists())
        self.assertIn('original strict artifact refusal',(self.output/'binding-result.json').read_text())


if __name__ == '__main__':unittest.main()
