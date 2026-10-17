import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import frontend_build as build
import dependency_lock


class Files(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.source=self.root/'source';self.source.mkdir()
        (self.source/'one').write_bytes(b'original one')

    def test_snapshot_includes_hidden_and_nested_files(self):
        (self.source/'.hidden').write_bytes(b'visible input')
        (self.source/'nested').mkdir();(self.source/'nested/two').write_bytes(b'two')
        actual=build.files(self.source)
        self.assertEqual(set(actual),{'one','.hidden','nested/two'})
        self.assertEqual(actual['one'],dict(bytes=12,sha256=build.digest(self.source/'one')))

    def test_relative_names_refuse_traversal_aliases_and_ordinary_false_values(self):
        for name in ('../outside','/outside','x/../y','x//y','x/./y','C:outside','x\\y','',False):
            with self.subTest(name=name),self.assertRaises(ValueError):build.relative(name)
        row=build.entry(self.source/'one')
        with self.assertRaises(ValueError):build.validate_files({'one':row,'ONE':row})

    def test_size_must_be_nonnegative_integer_and_hash_must_be_complete(self):
        for row in (dict(bytes=True,sha256='a'*64),dict(bytes=-1,sha256='a'*64),dict(bytes=2,sha256='z'*64),{}):
            with self.subTest(row=row),self.assertRaises(ValueError):build.validate_files({'one':row})

    def test_copy_preserves_exact_bytes_and_cannot_overwrite_existing_output(self):
        rows=build.files(self.source);destination=self.root/'destination'
        build.copy_files(self.source,destination,rows)
        self.assertEqual(build.files(destination),rows)
        with self.assertRaises(FileExistsError):build.copy_files(self.source,destination,rows)
        self.assertEqual(build.files(destination),rows)

    def test_changed_source_is_refused_before_copy(self):
        rows=build.files(self.source);(self.source/'one').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'changed during copy'):
            build.copy_files(self.source,self.root/'destination',rows)
        self.assertFalse((self.root/'destination').exists())

    def test_new_output_rejects_existing_directory(self):
        with self.assertRaisesRegex(ValueError,'New output'):build.new_output(self.source)

    def test_hashing_retains_parent_reparse_refusal(self):
        original=Path.lstat
        def metadata(path,*args,**kwargs):
            actual=original(path,*args,**kwargs)
            if path==self.source:
                return type('ReparseMetadata',(),{'st_mode':actual.st_mode,'st_file_attributes':0x400})()
            return actual
        with mock.patch.object(Path,'lstat',metadata),self.assertRaisesRegex(ValueError,'reparse parents'):
            build.entry(self.source/'one')

    def test_file_identity_is_checked_again_after_content_hash(self):
        original=build.hashlib.file_digest
        def changed(stream,*args):
            value=original(stream,*args)
            (self.source/'one').write_bytes(b'changed after hash')
            return value
        with mock.patch.object(build.hashlib,'file_digest',side_effect=changed),self.assertRaisesRegex(ValueError,'changed while hashing'):
            build.entry(self.source/'one')


class ResourceOutputs(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.workspace=self.root/'workspace'
        self.frontend=self.root/'frontend';self.frontend.mkdir()
        (self.frontend/'index.html').write_text('<script type="module" src="/one.js"></script>')
        (self.frontend/'one.js').write_text('export const value = 1;')
        self.mobile=self.workspace/'tauri/src-tauri/resources-mobile'
        self.docs=self.workspace/'tauri/src-tauri/resources/docs'
        self.market=self.mobile/'SoloSoul_plugin_market'
        self.docs.mkdir(parents=True);self.market.mkdir(parents=True)
        (self.docs/'guide.md').write_bytes(b'guide')
        (self.market/'registry.json').write_bytes(b'{}')
        rows=[]
        for name,path in [('SoloSoul_plugin_market/registry.json',self.market/'registry.json'),('docs/guide.md',self.docs/'guide.md')]:
            rows.append(dict(path=name,size=path.stat().st_size,sha256=build.digest(path)))
        content=''.join(r['path']+'\0'+str(r['size'])+'\0'+r['sha256']+'\n' for r in rows)
        self.manifest=dict(schema=1,version=hashlib.sha256(content.encode()).hexdigest(),files=rows)
        self.write()

    def write(self):
        (self.mobile/'bundled-resource-manifest.json').write_text(json.dumps(self.manifest),encoding='utf-8')

    def test_real_output_shape_binds_all_frontend_and_resource_bytes(self):
        result=build.check_outputs(self.workspace,self.frontend)
        self.assertEqual(len(result['frontend']),2)
        self.assertEqual(set(result['resources']),{'SoloSoul_plugin_market/registry.json','docs/guide.md'})
        self.assertEqual(result['resourceVersion'],self.manifest['version'])

    def test_missing_or_added_resource_cannot_be_hidden_by_manifest(self):
        (self.docs/'other.md').write_bytes(b'not declared')
        with self.assertRaisesRegex(ValueError,'Complete generated resource'):build.check_outputs(self.workspace,self.frontend)
        (self.docs/'other.md').unlink();self.manifest['files'].pop();self.write()
        with self.assertRaises(ValueError):build.check_outputs(self.workspace,self.frontend)

    def test_resource_mutation_and_wrong_version_refuse(self):
        (self.docs/'guide.md').write_bytes(b'wrong')
        with self.assertRaisesRegex(ValueError,'resource differs'):build.check_outputs(self.workspace,self.frontend)
        (self.docs/'guide.md').write_bytes(b'guide');self.manifest['version']='0'*64;self.write()
        with self.assertRaisesRegex(ValueError,'version'):build.check_outputs(self.workspace,self.frontend)

    def test_unknown_root_duplicate_row_and_boolean_size_refuse(self):
        original=copy.deepcopy(self.manifest)
        for mutation in ('root','duplicate','size'):
            self.manifest=copy.deepcopy(original)
            if mutation=='root':self.manifest['files'][0]['path']='outside/registry.json'
            elif mutation=='duplicate':self.manifest['files'].append(self.manifest['files'][0])
            else:self.manifest['files'][0]['size']=True
            self.write()
            with self.subTest(mutation=mutation),self.assertRaises(ValueError):build.check_outputs(self.workspace,self.frontend)

    def test_unbuilt_frontend_cannot_pass_with_resource_only_success(self):
        (self.frontend/'index.html').write_text('not a built application')
        with self.assertRaisesRegex(ValueError,'production frontend entry'):build.check_outputs(self.workspace,self.frontend)


class Plans(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve()
        for name in ('tauri/source','SoloSoul_plugin_market/source','dependencies/module/package.json','node','python','npm'):
            path=self.root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
        package=dict(name='fixture',version='1.0.0',dependencies={'module':'1.0.0'})
        record=dict(version='1.0.0',resolved='https://registry.invalid/fixture.tgz',integrity='sha512-fixture')
        (self.root/'tauri/package.json').write_text(json.dumps(package))
        (self.root/'tauri/package-lock.json').write_text(json.dumps(dict(name='fixture',version='1.0.0',lockfileVersion=3,
            packages={'':package,'node_modules/module':record})))
        (self.root/'dependencies/.package-lock.json').write_text(json.dumps(dict(lockfileVersion=3,packages={'node_modules/module':record})))
        (self.root/'dependencies/module/package.json').write_text('{"version":"1.0.0"}')
        self.source_names=['tauri/source','tauri/package.json','tauri/package-lock.json']
        self.plan=dict(schemaVersion=1,workspace=str(self.root),sourceHead='a'*40,pluginHead='b'*40,
            sourceFiles=build.files(self.root,self.source_names),pluginFiles=build.files(self.root/'SoloSoul_plugin_market'),
            nodeModules=dict(root=str(self.root/'dependencies'),files=build.files(self.root/'dependencies')),
            tools={n:dict(path=str(self.root/p),**build.entry(self.root/p)) for n,p in [('node','node'),('python','python'),('npmCli','npm')]},
            builderSources=build.builder_sources(),dependencyLock=dependency_lock.verify(self.root/'tauri',self.root/'dependencies'))

    def validate(self):
        def names(root,*_):return ['source'] if Path(root).name=='SoloSoul_plugin_market' else self.source_names
        with mock.patch.object(build,'repository_state',return_value=('a'*40,'b'*40)),mock.patch.object(build,'names',side_effect=names):
            build.validate(self.plan)

    def test_complete_plan_is_accepted_without_running_any_build_tool(self):
        with mock.patch.object(build.subprocess,'Popen',side_effect=AssertionError('No tools')):self.validate()

    def test_source_entry_cannot_be_removed_to_bypass_verification(self):
        self.plan['sourceFiles'].pop('tauri/source')
        with self.assertRaisesRegex(ValueError,'Complete original tracked'):self.validate()

    def test_changed_or_added_dependency_is_refused(self):
        (self.root/'dependencies/extra').write_bytes(b'extra')
        with self.assertRaisesRegex(ValueError,'dependencies changed'):self.validate()

    def test_changed_product_or_tool_or_builder_is_refused(self):
        for field in ('source','tool','builder'):
            with self.subTest(field=field):
                plan=copy.deepcopy(self.plan)
                if field=='source':self.plan['sourceFiles']['tauri/source']['sha256']='0'*64
                elif field=='tool':self.plan['tools']['node']['sha256']='0'*64
                else:self.plan['builderSources']['frontend_build.py']='0'*64
                with self.assertRaises(ValueError):self.validate()
                self.plan=plan


if __name__=='__main__':unittest.main()
