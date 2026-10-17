from pathlib import Path
import hashlib,json,tempfile,unittest
from unittest.mock import patch
import rf312_default_binding as binding


class DefaultBindingTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.runtime=self.root/'runtime.json';self.runtime.write_bytes(b'{"synthetic":true}\n')
        self.source={'one.py':'a'*64};self.config=dict(schemaVersion=1,runtimeInputs=str(self.runtime),
            runtimeInputsSha256=hashlib.sha256(self.runtime.read_bytes()).hexdigest(),sourceSha256=self.source)

    def write(self,config):
        file=self.root/'config.json';file.write_text(json.dumps(config),encoding='utf-8');return file

    def test_full_original_default_reader_is_still_called(self):
        with patch.object(binding,'sources',return_value=self.source),patch.object(binding,'load_inputs',return_value={'strict':'original'}) as original:
            value=binding.load(self.write(self.config));self.assertEqual(value['runtime'],{'strict':'original'})
            original.assert_called_once_with(self.runtime)

    def test_missing_extra_boolean_version_relative_path_and_omitted_sources_rejected(self):
        with patch.object(binding,'sources',return_value=self.source):
            for update in [dict(schemaVersion=True),dict(targetOverride='other'),dict(runtimeInputs='relative.json'),dict(sourceSha256={})]:
                config={**self.config,**update}
                with self.subTest(update=update),self.assertRaises(ValueError):binding.shape(config)
            config=dict(self.config);del config['runtimeInputsSha256']
            with self.assertRaises(ValueError):binding.shape(config)

    def test_changed_runtime_or_source_bytes_refused_before_reader(self):
        with patch.object(binding,'sources',return_value=self.source),patch.object(binding,'load_inputs') as original:
            for update in [dict(runtimeInputsSha256='b'*64),dict(sourceSha256={'one.py':'b'*64})]:
                with self.assertRaises(ValueError):binding.load(self.write({**self.config,**update}))
            original.assert_not_called()

    def test_original_default_marker_or_artifact_refusal_propagates(self):
        with patch.object(binding,'sources',return_value=self.source),patch.object(binding,'load_inputs',side_effect=ValueError('original default guard')):
            with self.assertRaisesRegex(ValueError,'original default guard'):binding.load(self.write(self.config))

    def test_runtime_modified_during_original_validation_is_rejected(self):
        def changed(path):self.runtime.write_bytes(b'changed');return {}
        with patch.object(binding,'sources',return_value=self.source),patch.object(binding,'load_inputs',side_effect=changed):
            with self.assertRaisesRegex(ValueError,'changed while'):binding.load(self.write(self.config))
