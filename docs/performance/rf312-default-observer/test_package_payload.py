import copy
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

import package_payload as package


class RefusedLibrary(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.receipt=self.root/'library.json'
        self.value=dict(passed=True,actualCargoReleaseBuilt=True,defaultFeaturesOnly=True,frozenInputsHeld=True,
            exitCode=0,productInstalled=False,nativeRuntimeAccepted=False,fullRF312Accepted=False,
            deviceCommandsExecuted=0,targetInputsDispatched=0)
    def write(self):self.receipt.write_text(json.dumps(self.value))
    def test_wrong_declared_receipt_hash_refuses(self):
        self.write()
        with self.assertRaisesRegex(ValueError,'receipt changed'):package.load_library(self.receipt,'0'*64)
    def test_formal_refusal_cannot_be_adopted_as_passing_library(self):
        self.value['passed']=False;self.write()
        with self.assertRaisesRegex(ValueError,'Passing default-library'):package.load_library(self.receipt,package.digest(self.receipt))
    def test_boolean_exit_or_counters_and_native_acceptance_refuse(self):
        for name,value in (('exitCode',False),('deviceCommandsExecuted',False),('nativeRuntimeAccepted',True)):
            original=copy.deepcopy(self.value);self.value[name]=value;self.write()
            with self.subTest(name=name),self.assertRaisesRegex(ValueError,'Passing default-library'):
                package.load_library(self.receipt,package.digest(self.receipt))
            self.value=original


class SourceAssets(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();root=self.root/'tauri/src-tauri'
        self.docs=root/'resources/docs/guide.md';self.docs.parent.mkdir(parents=True);self.docs.write_bytes(b'guide')
        plugin=root/'resources-mobile/SoloSoul_plugin_market/registry.json';plugin.parent.mkdir(parents=True);plugin.write_bytes(b'{}')
        manifest=root/'resources-mobile/bundled-resource-manifest.json';manifest.write_bytes(b'manifest')
        self.payload=dict(workspace=str(self.root),sources={},outputs=dict(resources={'docs/guide.md':package.entry(self.docs),
            'SoloSoul_plugin_market/registry.json':package.entry(plugin)},mobile={'bundled-resource-manifest.json':package.entry(manifest)}))
    def test_current_resources_and_one_manifest_are_complete(self):
        rows=package.source_assets(self.payload)
        self.assertEqual(set(rows),{'docs/guide.md','SoloSoul_plugin_market/registry.json','bundled-resource-manifest.json'})
    def test_changed_current_source_refuses(self):
        self.docs.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'source asset changed'):package.source_assets(self.payload)
    def test_unknown_resource_root_or_traversal_refuses(self):
        for name in ('other/guide.md','docs/../outside'):
            self.payload['outputs']['resources']={name:package.row(b'guide')}
            with self.subTest(name=name),self.assertRaises(ValueError):package.source_assets(self.payload)


    def track_native(self,name,body):
        key='tauri/src-tauri/gen/android/app/src/main/assets/'+name
        path=self.root/key;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
        self.payload['sources'][key]=package.entry(path)
        return path
    def test_extra_tracked_document_is_retained_without_old_package_bytes(self):
        path=self.track_native('docs/PRIVACY_POLICY.md',b'current public policy')
        result=package.source_assets(self.payload)
        self.assertEqual({k:result['docs/PRIVACY_POLICY.md'][k] for k in ('bytes','sha256')},package.entry(path))
        self.assertEqual(Path(result['docs/PRIVACY_POLICY.md']['path']),path)
    def test_generated_current_resource_replaces_verified_native_source_copy(self):
        self.track_native('docs/guide.md',b'old native copy')
        result=package.source_assets(self.payload)
        self.assertEqual({k:result['docs/guide.md'][k] for k in ('bytes','sha256')},self.payload['outputs']['resources']['docs/guide.md'])
    def test_changed_native_copy_is_rejected_even_when_generated_resource_overrides_it(self):
        path=self.track_native('docs/guide.md',b'bound native copy');path.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'Tracked Android asset changed'):package.source_assets(self.payload)
    def test_tracked_native_asset_traversal_is_rejected(self):
        self.payload['sources']['tauri/src-tauri/gen/android/app/src/main/assets/../outside']=package.row(b'foreign')
        with self.assertRaises(ValueError):package.source_assets(self.payload)


class ModuleSources(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.workspace=self.root/'workspace';self.workspace.mkdir()
        self.cargo=self.root/'cargo';self.cached=self.cargo/'registry/src/fixture/tauri-1.0.0';self.cached.mkdir(parents=True)
        self.archive=self.cargo/'registry/cache/fixture/tauri-1.0.0.crate';self.archive.parent.mkdir(parents=True)
        self.sources={}
        for name in ('PluginManager.kt','PluginHandle.kt'):
            path=self.workspace/'src-tauri/gen/android/patches/tauri-android'/name;path.parent.mkdir(parents=True,exist_ok=True)
            path.write_bytes(('// SoloSoul PATCH\n'+name).encode())
            self.sources['tauri/src-tauri/gen/android/patches/tauri-android/'+name]=package.entry(path)
        self.members=[('mobile/android/AndroidManifest.xml',b'official manifest'),('mobile/android/build.gradle.kts',b'official build')]
        self.members += [('mobile/android/src/main/java/app/tauri/plugin/'+n,b'official '+n.encode()) for n in ('PluginManager.kt','PluginHandle.kt')]
        self.target=self.root/'private-module';self.write()
    def write(self):
        with tarfile.open(self.archive,'w:gz') as reader:
            for name,body in self.members:
                info=tarfile.TarInfo('tauri-1.0.0/'+name);info.size=len(body);reader.addfile(info,io.BytesIO(body))
        checksum=package.digest(self.archive)
        (self.workspace/'Cargo.lock').write_text('version = 4\n[[package]]\nname = "tauri"\nversion = "1.0.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "'+checksum+'"\n')
    def stage(self):return package.stage_module(self.workspace,self.cargo,'tauri-android',self.target,self.sources)
    def test_archive_sources_and_canonical_patches_are_private_and_ignore_cache_poison(self):
        poison=self.cached/'mobile/android/AndroidManifest.xml';poison.parent.mkdir(parents=True);poison.write_bytes(b'cache mutation')
        result=self.stage()
        self.assertEqual((self.target/'AndroidManifest.xml').read_bytes(),b'official manifest')
        self.assertEqual(poison.read_bytes(),b'cache mutation')
        self.assertEqual(len(result['canonicalProductPatches']),2)
        self.assertFalse(result['sharedCacheSourceWritten'])
        with self.assertRaisesRegex(ValueError,'New private'):self.stage()
    def test_missing_canonical_patch_refuses_before_output(self):
        self.sources.pop(next(iter(self.sources)))
        with self.assertRaises(KeyError):self.stage()
        self.assertFalse(self.target.exists())
    def test_changed_archive_checksum_refuses_before_output(self):
        with self.archive.open('ab') as stream:stream.write(b'changed')
        with self.assertRaisesRegex(ValueError,'checksum differs'):self.stage()
        self.assertFalse(self.target.exists())
    def test_traversal_and_windows_case_aliases_refuse_before_output(self):
        original=list(self.members)
        for extra in ([('mobile/android/../outside',b'bad')],
                      [('mobile/android/Foo.kt',b'a'),('mobile/android/foo.kt',b'b')]):
            self.members=original+extra;self.write()
            with self.subTest(extra=extra),self.assertRaises(ValueError):self.stage()
            self.assertFalse(self.target.exists())


class ApkContents(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.old=self.root/'old.apk';self.new=self.root/'new.apk'
        self.old_items={'assets/guide.txt':b'guide','assets/obsolete.txt':b'old doc',
            'assets/ocr/model.bin':b'ocr model','assets/dexopt/baseline.prof':b'old profile',
            'assets/dexopt/baseline.profm':b'old metadata',package.CORE:b'old core',
            'lib/x86_64/libmlkit_google_ocr_pipeline.so':b'original OCR library'}
        self.write(self.old,self.old_items)
        self.reference=dict(apk=dict(path=str(self.old),**package.entry(self.old)),
            appAssets={n:hashlib.sha256(self.old_items['assets/'+n]).hexdigest() for n in ('guide.txt','obsolete.txt')})
        self.asset_data={'guide.txt':b'guide','new-current.txt':b'new current source'}
        self.profile_data={'dexopt/baseline.prof':b'new generated profile','dexopt/baseline.profm':b'new generated metadata'}
        self.current={n:package.row(b) for n,b in self.asset_data.items()};self.profiles={n:package.row(b) for n,b in self.profile_data.items()}
        self.library=package.row(b'new current core')
        self.items={n:b for n,b in self.old_items.items() if not n.startswith('assets/')}
        self.items[package.CORE]=b'new current core'
        self.items.update({'assets/'+n:b for n,b in {**self.asset_data,**self.profile_data,'ocr/model.bin':b'ocr model'}.items()})
    def write(self,path,items):
        with zipfile.ZipFile(path,'w') as archive:
            for n,b in items.items():archive.writestr(n,b)
    def verify(self):
        self.write(self.new,self.items)
        return package.verify_apk_bytes(self.new,self.current,self.library,self.reference,self.profiles)
    def test_new_source_and_generated_profiles_keep_whole_dependency_bytes(self):
        result=self.verify()
        self.assertEqual((result['appAssets'],result['dependencyAssets'],result['generatedProfiles'],result['allNativeLibraries']),(2,1,2,2))
        self.assertTrue(result['oldAppAssetTreeNotCopiedOrReinterpreted'])
    def test_missing_or_unknown_source_or_dependency_assets_refuse(self):
        original=dict(self.items)
        for mutation in ('missing','unknown','changed-dependency','stale-old-doc'):
            self.items=dict(original)
            if mutation=='missing':self.items.pop('assets/guide.txt')
            elif mutation=='unknown':self.items['assets/unknown.bin']=b'unknown'
            elif mutation=='changed-dependency':self.items['assets/ocr/model.bin']=b'wrong'
            else:self.items['assets/obsolete.txt']=b'old doc'
            with self.subTest(mutation=mutation),self.assertRaisesRegex(ValueError,'asset set differs'):self.verify()
    def test_core_dependency_library_and_foreign_abi_changes_refuse(self):
        original=dict(self.items)
        for name in (package.CORE,'lib/x86_64/libmlkit_google_ocr_pipeline.so','lib/arm64-v8a/foreign.so'):
            self.items=dict(original);self.items[name]=b'changed'
            with self.subTest(name=name),self.assertRaisesRegex(ValueError,'library differs'):self.verify()
    def test_source_and_dependency_names_cannot_overlap(self):
        self.current['ocr/model.bin']=package.row(b'ocr model')
        with self.assertRaisesRegex(ValueError,'Disjoint'):self.verify()
    def test_generated_profiles_must_match_actual_new_outputs(self):
        self.items['assets/dexopt/baseline.prof']=b'old profile'
        with self.assertRaisesRegex(ValueError,'asset set differs'):self.verify()
    def test_optional_observer_marker_refuses_even_with_matching_declared_core_hash(self):
        self.items[package.CORE]=b'__SOLOSOUL_ANDROID_NATIVE_PERF__';self.library=package.row(self.items[package.CORE])
        with self.assertRaisesRegex(ValueError,'Optional native observer'):self.verify()
    def test_changed_reference_and_duplicate_zip_entries_refuse(self):
        self.write(self.new,self.items)
        with self.assertWarns(UserWarning),zipfile.ZipFile(self.new,'a') as archive:archive.writestr('assets/guide.txt',b'duplicate')
        with self.assertRaisesRegex(ValueError,'Duplicate APK'):
            package.verify_apk_bytes(self.new,self.current,self.library,self.reference,self.profiles)
        with self.old.open('ab') as stream:stream.write(b'changed')
        with self.assertRaisesRegex(ValueError,'Frozen dependency APK'):self.verify()
    def test_unsafe_zip_paths_and_boolean_sizes_refuse(self):
        self.items['res/../outside']=b'bad'
        with self.assertRaises(ValueError):self.verify()
        self.items.pop('res/../outside');self.library=dict(bytes=True,sha256='0'*64)
        with self.assertRaises(ValueError):self.verify()


if __name__=='__main__':unittest.main()
