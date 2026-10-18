import ast
import copy
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
import diagnostic_binding as binding


class BindingTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        r=Path(self.temp.name)
        self.config=dict(schemaVersion=1,runtimeInputs=str(r/'runtime.json'),runtimeInputsSha256='a'*64,
            diagnosticPackageReceipt=str(r/'app.json'),diagnosticPackageReceiptSha256='b'*64,
            collectorPackageReceipt=str(r/'collector.json'),collectorPackageReceiptSha256='c'*64,avd='SoloSoul_RF312_IPC_V51',
            adbExecutable=str(r/'platform-tools/adb.exe'),adbSha256='d'*64,sourceSha256=binding.sources())

    def test_explicit_diagnostic_config_keeps_default_input_separate(self):
        binding.config_shape(self.config)

    def test_optimized_interpreter_cannot_disable_validation_assertions(self):
        for optimize in [1, 2]:
            with self.subTest(optimize=optimize), patch.object(binding.sys, 'flags', SimpleNamespace(optimize=optimize)):
                with self.assertRaisesRegex(ValueError, 'Python assertions enabled'):
                    binding.require_assertions()

    def test_optimized_load_refuses_before_reading_receipts(self):
        with patch.object(binding.sys, 'flags', SimpleNamespace(optimize=1)), patch.object(binding.producer, 'ordinary') as read:
            with self.assertRaisesRegex(ValueError, 'Python assertions enabled'):
                binding.load(Path(self.temp.name) / 'unread-receipt.json')
            read.assert_not_called()

    def test_optimized_verify_refuses_before_validating_configuration(self):
        with patch.object(binding.sys, 'flags', SimpleNamespace(optimize=2)), patch.object(binding, 'config_shape') as validate:
            with self.assertRaisesRegex(ValueError, 'Python assertions enabled'):
                binding.verify({'config': self.config})
            validate.assert_not_called()

    def test_missing_extra_and_boolean_schema_rejected(self):
        for update in [dict(schemaVersion=True),dict(schemaVersion=2),dict(release={'apk':'relabelled'})]:
            value=copy.deepcopy(self.config);value.update(update)
            with self.subTest(update=update),self.assertRaises(ValueError):binding.config_shape(value)
        value=copy.deepcopy(self.config);del value['runtimeInputsSha256']
        with self.assertRaises(ValueError):binding.config_shape(value)

    def test_implicit_path_wrong_hash_and_generic_avd_rejected(self):
        for key,value in [('runtimeInputs','relative'),('collectorPackageReceipt',True),
                          ('diagnosticPackageReceiptSha256','x'*64),('avd','daily-device')]:
            changed=copy.deepcopy(self.config);changed[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):binding.config_shape(changed)

    def test_complete_runtime_sources_are_required(self):
        for source in [None,{},dict(self.config['sourceSha256'],extra='a'*64)]:
            with self.subTest(source=source),self.assertRaises(ValueError):
                binding.config_shape(dict(self.config,sourceSha256=source))
        changed=copy.deepcopy(self.config);changed['sourceSha256'][next(iter(changed['sourceSha256']))]='bad'
        with self.assertRaises(ValueError):binding.config_shape(changed)

    def test_changed_runtime_source_is_rejected_before_artifact_reads(self):
        import json
        config=copy.deepcopy(self.config)
        first=next(iter(config['sourceSha256']))
        config['sourceSha256'][first]='f'*64 if config['sourceSha256'][first]!='f'*64 else 'e'*64
        path=Path(self.temp.name)/'config.json';path.write_text(json.dumps(config),encoding='utf-8')
        with patch.object(binding.producer.package.producer,'bound_json') as read:
            with self.assertRaisesRegex(ValueError,'runtime source changed'):binding.load(path)
            read.assert_not_called()

    def test_changed_config_bytes_during_validation_are_rejected(self):
        import json
        path=Path(self.temp.name)/'config.json';path.write_text(json.dumps(self.config),encoding='utf-8')
        runtime={'host':{'serial':'emulator-5578'}}
        def changed(value):path.write_bytes(b'changed during validation')
        with patch.object(binding.producer.package.producer,'bound_json',side_effect=[runtime,{},{}]),patch.object(binding,'verify',side_effect=changed):
            with self.assertRaisesRegex(ValueError,'configuration or sources changed while'):binding.load(path)

    def test_source_modified_during_artifact_validation_is_rejected(self):
        import json
        path=Path(self.temp.name)/'config.json';path.write_text(json.dumps(self.config),encoding='utf-8')
        runtime={'host':{'serial':'emulator-5578'}}
        changed=dict(self.config['sourceSha256']);changed[next(iter(changed))]='e'*64
        with patch.object(binding,'sources',side_effect=[self.config['sourceSha256'],self.config['sourceSha256'],changed]),patch.object(binding.producer.package.producer,'bound_json',side_effect=[runtime,{},{}]),patch.object(binding,'verify'):
            with self.assertRaisesRegex(ValueError,'configuration or sources changed while'):binding.load(path)

    def test_exception_capture_cannot_replace_frozen_diagnostic_binding(self):
        # Execute the real nested frozen() with synthetic file/receipt inputs.
        # The SDK diagnostic local deliberately has an incompatible type.
        source=(binding.ROOT/'diagnostic_driver.py').read_text(encoding='utf-8')
        main=next(n for n in ast.parse(source).body if isinstance(n,ast.FunctionDef) and n.name=='main')
        frozen=next(n for n in main.body if isinstance(n,ast.FunctionDef) and n.name=='frozen')
        root=Path(self.temp.name);original=root/'runtime.json';original.write_bytes(b'original runtime')
        config=root/'diagnostic.json';config.write_bytes(b'original diagnostic')
        payload=root/'apk';payload.write_bytes(b'public synthetic artifact')
        row=dict(path=str(payload),sha256=binding.sha(payload))
        runtime=dict(host=dict(workspaceRoot=str(root)),runtimeSourceFiles={},release=dict(apk=row),
                     originalDebug=row,originalTest=row,runtimeTest=row)
        expected=object()
        namespace=dict(Path=Path,args=SimpleNamespace(inputs=original,diagnostic_inputs=config),
            input_bytes=original.read_bytes(),diagnostic_bytes=config.read_bytes(),frozen_inputs=runtime,
            diagnostic_sources={},digest=binding.sha,verify_sources=lambda _:None,binding_record=expected,
            diagnostic=SimpleNamespace(stdout=b'synthetic error capture',stderr=b''),
            diagnostic_binding=SimpleNamespace(sources=lambda:{},verify=lambda actual:self.assertIs(actual,expected)))
        exec(compile(ast.Module(body=[frozen],type_ignores=[]),'<frozen-diagnostic-binding>','exec'),namespace)
        namespace['frozen']()


if __name__=='__main__':unittest.main()
