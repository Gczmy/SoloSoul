"""Source preservation and non-default artifact admission checks."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest

import rf312_ipc_library as diagnostic


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class DiagnosticLibraryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.workspace = self.root/'workspace'
        self.target = self.root/'target'
        self.library = self.target/diagnostic.default_library.TARGET/'release/libsolo_soul.so'
        self.row = dict(reason='compiler-artifact',target=dict(name='solo_soul',kind=['cdylib'],
                        src_path=str(self.workspace/'src-tauri/src/lib.rs')),
                        features=['default','custom-protocol','android-native-perf'],fresh=False,
                        profile=dict(opt_level='3',debuginfo=0,debug_assertions=False,overflow_checks=False,test=False),
                        filenames=[str(self.library)])

    def cargo(self, rows=None, finished=True):
        values = (rows if rows is not None else [self.row]) + [dict(reason='build-finished',success=finished)]
        return ('\n'.join(json.dumps(row) for row in values)+'\n').encode()

    def frontend(self):
        root = self.root/'frontend';root.mkdir()
        values = {'index.html':b'<script src="/assets/current.js"></script>',
                  'assets/current.js':b'public current frontend'}
        for name,raw in values.items():
            path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
        return root,{name:digest(raw) for name,raw in values.items()}

    def elf(self, extra=b'', machine=62):
        raw=bytearray(64);raw[:6]=b'\x7fELF\x02\x01';struct.pack_into('<HH',raw,16,3,machine)
        self.library.parent.mkdir(parents=True,exist_ok=True)
        self.library.write_bytes(bytes(raw)+extra)
        return self.library

    def test_accept_only_fresh_exact_feature_release_artifact(self):
        path,row=diagnostic.compiler_artifact(self.cargo(),self.workspace,self.target)
        self.assertEqual(path,self.library)
        self.assertEqual(row['features'],self.row['features'])

    def test_refuse_default_extra_missing_and_duplicate_features(self):
        for features in [['default','custom-protocol'],['default','android-native-perf'],
                         self.row['features']+['devtools'],self.row['features']+['android-native-perf']]:
            with self.subTest(features=features):
                row=copy.deepcopy(self.row);row['features']=features
                with self.assertRaises(ValueError):
                    diagnostic.compiler_artifact(self.cargo([row]),self.workspace,self.target)

    def test_refuse_cached_debug_and_test_artifacts(self):
        edits=[lambda r:r.update(fresh=True),lambda r:r['profile'].update(opt_level='0'),
               lambda r:r['profile'].update(debug_assertions=True),lambda r:r['profile'].update(debuginfo=2),
               lambda r:r['profile'].update(test=True)]
        for edit in edits:
            with self.subTest(edit=edit):
                row=copy.deepcopy(self.row);edit(row)
                with self.assertRaises(ValueError):
                    diagnostic.compiler_artifact(self.cargo([row]),self.workspace,self.target)

    def test_refuse_failed_duplicate_foreign_and_wrong_output(self):
        cases=[self.cargo(finished=False),self.cargo([self.row,self.row])]
        row=copy.deepcopy(self.row);row['target']['src_path']=str(self.root/'foreign/lib.rs');cases.append(self.cargo([row]))
        row=copy.deepcopy(self.row);row['filenames']=[str(self.root/'old/libsolo_soul.so')];cases.append(self.cargo([row]))
        for raw in cases:
            with self.subTest(raw=raw):
                with self.assertRaises(ValueError):diagnostic.compiler_artifact(raw,self.workspace,self.target)

    def test_diagnostic_elf_keeps_default_reader_rejection(self):
        frontend,rows=self.frontend()
        extra=b' '.join([*diagnostic.MARKERS,*[name.encode() for name in rows]])
        path=self.elf(extra)
        result=diagnostic.verify_library(path,frontend,rows)
        self.assertFalse(result['defaultFeaturesOnly'])
        self.assertTrue(result['existingDefaultReaderRejected'])
        self.assertEqual(result['baselineFrontendKeysVerified'],2)
        with self.assertRaisesRegex(ValueError,'Optional native observer marker'):
            diagnostic.default_library.verify_library(path,frontend)

    def test_refuse_incomplete_marker_wrong_elf_and_missing_frontend_key(self):
        frontend,rows=self.frontend()
        variants=[(b'index.html assets/current.js',62),
                  (diagnostic.MARKERS[0]+b' index.html assets/current.js',62),
                  (b' '.join(diagnostic.MARKERS)+b' index.html assets/current.js',183),
                  (b' '.join(diagnostic.MARKERS)+b' index.html',62)]
        for extra,machine in variants:
            with self.subTest(machine=machine,extra=extra):
                with self.assertRaises(ValueError):diagnostic.verify_library(self.elf(extra,machine),frontend,rows)

    def test_refuse_changed_frontend_even_with_correct_keys(self):
        frontend,rows=self.frontend()
        (frontend/'assets/current.js').write_bytes(b'changed')
        path=self.elf(b' '.join([*diagnostic.MARKERS,b'index.html assets/current.js']))
        with self.assertRaises(ValueError):diagnostic.verify_library(path,frontend,rows)

    def test_copy_exact_sources_skip_only_declared_generated_file(self):
        source=self.root/'source';source.mkdir()
        (source/'keep.txt').write_bytes(b'unchanged')
        (source/'generated.kt').write_bytes(b'official')
        rows={'keep.txt':digest(b'unchanged'),'generated.kt':digest(b'official')}
        target=self.root/'copy'
        copied=diagnostic.copy_bound(source,rows,target,{'generated.kt'})
        self.assertEqual(copied,{'keep.txt':rows['keep.txt']})
        self.assertEqual((target/'keep.txt').read_bytes(),b'unchanged')
        self.assertFalse((target/'generated.kt').exists())
        self.assertEqual((source/'generated.kt').read_bytes(),b'official')
        with self.assertRaises(ValueError):diagnostic.copy_bound(source,rows,target)

    def test_copy_checks_skipped_file_and_refuses_path_escape(self):
        source=self.root/'source';source.mkdir();(source/'a').write_bytes(b'public')
        with self.assertRaises(ValueError):
            diagnostic.copy_bound(source,{'a':digest(b'wrong')},self.root/'bad',{'a'})
        with self.assertRaises(ValueError):
            diagnostic.copy_bound(source,{'../outside':digest(b'public')},self.root/'escape')

    def test_generated_set_requires_all_baseline_hashes(self):
        library=dict(generatedAndroidBridge={'Ipc.kt':{'sha256':'a'*64}},
                     generatedAndroidMetadata={'tauri.settings.gradle':{'sha256':'b'*64}})
        rows={diagnostic.BRIDGE+'Ipc.kt':'a'*64,diagnostic.GENERATED+'tauri.settings.gradle':'b'*64}
        self.assertEqual(diagnostic.generated_set(library,{'runtimeSourceFiles':rows}),rows)
        rows[diagnostic.BRIDGE+'Ipc.kt']='c'*64
        with self.assertRaises(ValueError):diagnostic.generated_set(library,{'runtimeSourceFiles':rows})

    def test_bound_json_refuses_changed_bytes_and_duplicate_fields(self):
        path=self.root/'receipt.json';path.write_bytes(b'{"public":1}')
        self.assertEqual(diagnostic.bound_json(path,digest(path.read_bytes())),{'public':1})
        with self.assertRaises(ValueError):diagnostic.bound_json(path,'a'*64)
        path.write_bytes(b'{"public":1,"public":2}')
        with self.assertRaises(ValueError):diagnostic.bound_json(path,digest(path.read_bytes()))

    def test_configuration_cannot_add_flags_or_boolean_schema(self):
        for config in [{},{'schemaVersion':True},{'schemaVersion':1,'features':['devtools']}]:
            with self.subTest(config=config):
                with self.assertRaises(ValueError):diagnostic.configure(config,self.root/'output')


if __name__ == '__main__':
    unittest.main()
