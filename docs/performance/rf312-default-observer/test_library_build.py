import copy
import hashlib
import json
import os
from pathlib import Path
import struct
import tempfile
import tarfile
import io
import unittest
from unittest.mock import patch

import build_payload
import frontend_build
import library_build as library


class Payload(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.workspace=self.root/'workspace';self.frontend=self.root/'frontend'
        self.frontend.mkdir();(self.frontend/'index.html').write_text('<script type="module" src="/assets/app.js"></script>')
        (self.frontend/'assets').mkdir();(self.frontend/'assets/app.js').write_bytes(b'production frontend')
        docs=self.workspace/'tauri/src-tauri/resources/docs';docs.mkdir(parents=True)
        (docs/'guide.md').write_bytes(b'guide');market=self.workspace/'SoloSoul_plugin_market';market.mkdir()
        (market/'source').write_bytes(b'plugin source')
        mobile=self.workspace/'tauri/src-tauri/resources-mobile';(mobile/'SoloSoul_plugin_market').mkdir(parents=True)
        (mobile/'SoloSoul_plugin_market/registry.json').write_bytes(b'{}')
        rows=[]
        for name,path in [('SoloSoul_plugin_market/registry.json',mobile/'SoloSoul_plugin_market/registry.json'),('docs/guide.md',docs/'guide.md')]:
            rows.append(dict(path=name,size=path.stat().st_size,sha256=frontend_build.digest(path)))
        text=''.join(r['path']+'\0'+str(r['size'])+'\0'+r['sha256']+'\n' for r in rows)
        (mobile/'bundled-resource-manifest.json').write_text(json.dumps(dict(schema=1,files=rows,version=hashlib.sha256(text.encode()).hexdigest())))
        self.plan=dict(schemaVersion=1,workspace='unavailable-original-checkout',sourceHead='a'*40,pluginHead='b'*40,
            sourceFiles=frontend_build.files(self.workspace,['tauri/src-tauri/resources/docs/guide.md']),
            pluginFiles=frontend_build.files(market),nodeModules=dict(root='unavailable-original-dependencies',files={'package':dict(bytes=1,sha256='0'*64)}),
            tools={},builderSources=frontend_build.builder_sources(),dependencyLock={})
        self.plan_path=self.root/'frontend-plan.json';self.plan_path.write_text(json.dumps(self.plan))
        self.result=dict(passed=True,exitCode=0,frontendBuiltFromCopiedSources=True,originalInputsUnchanged=True,productInstalled=False,
            deviceCommandsExecuted=0,targetInputsDispatched=0,planSha256=frontend_build.digest(self.plan_path),
            builderSources=frontend_build.builder_sources(),sourceHead='a'*40,pluginHead='b'*40,
            sourceWorkspace=str(self.workspace),frontendRoot=str(self.frontend),sourceFiles=1,pluginFiles=1,dependencyFiles=1,
            generatedTrackedFiles={},outputs=frontend_build.check_outputs(self.workspace,self.frontend))
        self.receipt=self.root/'frontend-result.json';self.write()

    def write(self):self.receipt.write_text(json.dumps(self.result))
    def load(self):return build_payload.load(self.receipt,frontend_build.digest(self.receipt))

    def test_complete_materialized_payload_needs_no_original_checkout_or_dependencies(self):
        actual=self.load()
        self.assertEqual(actual['sourceHead'],'a'*40)
        self.assertEqual(len(actual['sources']),1)
        self.assertEqual(len(actual['outputs']['resources']),2)
        build_payload.recheck(actual)

    def test_receipt_or_plan_hash_substitution_refuses(self):
        with self.assertRaisesRegex(ValueError,'receipt changed'):build_payload.load(self.receipt,'0'*64)
        self.plan_path.write_text('{}')
        with self.assertRaisesRegex(ValueError,'plan changed'):self.load()

    def test_real_source_and_complete_plugin_tree_cannot_change(self):
        (self.workspace/'tauri/src-tauri/resources/docs/guide.md').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'product source changed'):self.load()
        (self.workspace/'tauri/src-tauri/resources/docs/guide.md').write_bytes(b'guide')
        (self.workspace/'SoloSoul_plugin_market/extra').write_bytes(b'unknown')
        with self.assertRaisesRegex(ValueError,'complete plugin source'):self.load()

    def test_unexpected_frontend_or_resource_output_refuses(self):
        (self.frontend/'extra.js').write_bytes(b'not recorded')
        with self.assertRaisesRegex(ValueError,'frontend or resources'):self.load()

    def test_derived_binding_cannot_override_arbitrary_tracked_source(self):
        self.result['generatedTrackedFiles']={'tauri/src-tauri/resources/docs/guide.md':frontend_build.entry(self.workspace/'tauri/src-tauri/resources/docs/guide.md')}
        self.write()
        with self.assertRaisesRegex(ValueError,'original and derived'):self.load()

    def test_boolean_exit_code_and_boolean_source_count_refuse(self):
        for key,value in [('exitCode',False),('sourceFiles',True),('deviceCommandsExecuted',False)]:
            before=copy.deepcopy(self.result);self.result[key]=value;self.write()
            with self.subTest(key=key),self.assertRaises(ValueError):self.load()
            self.result=before

    def test_moved_receipt_layout_and_wrong_builder_version_refuse(self):
        self.result['frontendRoot']=str(self.workspace);self.write()
        with self.assertRaisesRegex(ValueError,'materialized paths'):self.load()
        self.result['frontendRoot']=str(self.frontend);self.result['builderSources']['frontend_build.py']='0'*64;self.write()
        with self.assertRaisesRegex(ValueError,'builder and source'):self.load()


class Compiler(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.workspace=self.root/'workspace';self.target=self.root/'target'
        self.source=self.workspace/'src-tauri/src/lib.rs';self.source.parent.mkdir(parents=True);self.source.write_bytes(b'product source')
        self.path=self.target/library.TARGET/'release/libsolo_soul.so';self.path.parent.mkdir(parents=True)
        self.row=dict(reason='compiler-artifact',features=['custom-protocol','default'],fresh=False,
            profile=dict(opt_level='3',debug_assertions=False,test=False),
            target=dict(name='solo_soul',src_path=str(self.source),kind=['staticlib','cdylib','rlib']),filenames=[str(self.path)])

    def raw(self,rows=None):
        return b'\n'.join(json.dumps(r).encode() for r in (rows or [self.row,dict(reason='build-finished',success=True)]))

    def test_actual_fresh_default_release_artifact_is_required(self):
        path,row=library.compiler_artifact(self.raw(),self.workspace,self.target)
        self.assertEqual(path,self.path);self.assertEqual(row['features'],['custom-protocol','default'])

    def test_optional_features_debug_profile_or_cached_product_refuse(self):
        for key,value in [('features',['custom-protocol','default','android-native-perf']),('fresh',True),
                          ('profile',dict(opt_level='3',debug_assertions=True,test=False)),
                          ('profile',dict(opt_level='0',debug_assertions=False,test=False))]:
            before=copy.deepcopy(self.row);self.row[key]=value
            with self.subTest(key=key),self.assertRaisesRegex(ValueError,'default-only Release'):
                library.compiler_artifact(self.raw(),self.workspace,self.target)
            self.row=before

    def test_missing_completion_duplicate_product_or_other_target_refuse(self):
        for rows in ([self.row], [self.row,self.row,dict(reason='build-finished',success=True)],
                     [dict(self.row,filenames=[str(self.root/'old-library.so')]),dict(reason='build-finished',success=True)]):
            with self.subTest(rows=rows),self.assertRaises(ValueError):library.compiler_artifact(self.raw(rows),self.workspace,self.target)

    def test_shared_elf_must_embed_current_entry_and_have_no_observer_marker(self):
        front=self.root/'frontend';(front/'assets').mkdir(parents=True)
        (front/'assets/app.js').write_bytes(b'javascript')
        (front/'index.html').write_text('<script type="module" src="/assets/app.js"></script>')
        valid=bytearray(64);valid[:6]=b'\x7fELF\x02\x01';struct.pack_into('<HH',valid,16,3,62)
        self.path.write_bytes(valid+b'/assets/app.js')
        self.assertEqual(library.verify_library(self.path,front)['entrypoints'],['/assets/app.js'])
        for raw in (valid,valid+b'/assets/app.js__SOLOSOUL_ANDROID_NATIVE_PERF__',b'not ELF'):
            self.path.write_bytes(raw)
            with self.subTest(raw=raw),self.assertRaises(ValueError):library.verify_library(self.path,front)

    def test_environment_clears_wrappers_injected_features_and_codegen_extensions(self):
        p=dict(output=self.root/'new output',workspace=self.workspace,payload=dict(frontend=str(self.root/'frontend')),
            toolchain=self.root/'toolchain',rustup=self.root/'rustup',cargoHome=self.root/'cargo-home',sdk=self.root/'sdk',
            ndk=self.root/'ndk',tools={n:self.root/n for n in ('cargo','rustc','linker','ar')})
        original=dict(PATH='inherited',RUSTC_WRAPPER='forbidden-wrapper',CARGO_FEATURE_NATIVE_PERF='1',
            WRY_IPC_CLASS_INIT='injected',TAURI_CONFIG='injected',CARGO_ENCODED_RUSTFLAGS='injected',
            CARGO_PROFILE_RELEASE_OPT_LEVEL='0',SOLOSOUL_REGISTRY_PUBKEY='injected')
        command,env,override,bridge=library.command_and_environment(p,original)
        self.assertEqual(command[1:9],['build','--locked','--offline','-p','solo_soul','--lib','--release','--target'])
        self.assertNotIn('--features',command)
        for name in ('RUSTC_WRAPPER','CARGO_FEATURE_NATIVE_PERF','WRY_IPC_CLASS_INIT','CARGO_ENCODED_RUSTFLAGS','CARGO_PROFILE_RELEASE_OPT_LEVEL','SOLOSOUL_REGISTRY_PUBKEY'):
            self.assertNotIn(name,env)
        self.assertEqual(set(override),{'build'});self.assertEqual(set(override['build']),{'frontendDist'})
        self.assertEqual(env['CARGO_TARGET_DIR'],str(p['output']/'cargo-target'))
        self.assertEqual(env['ORT_CACHE_DIR'],str(p['output']/'ort-cache'))
        self.assertEqual(env['WRY_ANDROID_KOTLIN_FILES_OUT_DIR'],str(bridge))
        self.assertEqual(original['RUSTC_WRAPPER'],'forbidden-wrapper')


class GradleMetadata(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();(self.root/'app').mkdir()
        self.expected={n:str(self.root/n) for n in ('tauri-android','tauri-plugin-fs')}
        for path in self.expected.values():Path(path).mkdir()
        self.settings='// THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n'
        for name,path in self.expected.items():
            self.settings+="include ':"+name+"'\nproject(':"+name+"').projectDir = new File("+json.dumps(path)+")\n"
        self.build='// THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\nval implementation by configurations\ndependencies {\n  implementation("androidx.lifecycle:lifecycle-process:2.10.0")'
        self.build+=''.join('\n  implementation(project(":'+name+'"))' for name in self.expected)+'\n}'
        self.write()

    def write(self):
        (self.root/'tauri.settings.gradle').write_text(self.settings,encoding='utf-8',newline='\n')
        (self.root/'app/tauri.build.gradle.kts').write_text(self.build,encoding='utf-8',newline='\n')

    def test_exact_locked_paths_and_implementation_set_pass(self):
        library.verify_gradle_metadata(self.root,self.expected)

    def test_unbound_directory_or_duplicate_include_refuses(self):
        self.settings+="include ':tauri-android'\n";self.write()
        with self.assertRaises(ValueError):library.verify_gradle_metadata(self.root,self.expected)
        self.setUp()
        self.expected['tauri-android']=str(self.root/'tauri-plugin-fs')
        with self.assertRaises(ValueError):library.verify_gradle_metadata(self.root,self.expected)

    def test_extra_dsl_statement_or_dependency_refuses(self):
        self.build+='\nprintln("unexpected statement")';self.write()
        with self.assertRaises(ValueError):library.verify_gradle_metadata(self.root,self.expected)
        self.build=self.build.replace('\nprintln("unexpected statement")','').replace('tauri-plugin-fs','unknown');self.write()
        with self.assertRaises(ValueError):library.verify_gradle_metadata(self.root,self.expected)


class RegistryArchive(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.workspace=self.root/'workspace';self.workspace.mkdir()
        self.cargo=self.root/'cargo'
        self.source=self.cargo/'registry/src/fixture/tauri-1.0.0';self.source.mkdir(parents=True)
        self.archive=self.cargo/'registry/cache/fixture/tauri-1.0.0.crate';self.archive.parent.mkdir(parents=True)

    def write(self,members):
        with tarfile.open(self.archive,'w:gz') as archive:
            for name,data in members:
                info=tarfile.TarInfo(name);info.size=len(data);archive.addfile(info,io.BytesIO(data))
        checksum=frontend_build.digest(self.archive)
        (self.workspace/'Cargo.lock').write_text('version = 4\n[[package]]\nname = "tauri"\nversion = "1.0.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "'+checksum+'"\n')

    def test_cached_crate_hash_and_member_digest_are_bound_without_auxiliary_metadata(self):
        self.write([('tauri-1.0.0/build.rs',b'official template')])
        root,rows,archive,row=library.registry_source(self.workspace,self.cargo,'tauri')
        self.assertEqual(root,self.source);self.assertEqual(archive,self.archive)
        self.assertEqual(rows['build.rs'],hashlib.sha256(b'official template').hexdigest())
        self.assertEqual(row['sha256'],frontend_build.digest(self.archive))
        self.assertFalse((self.source/'.cargo-checksum.json').exists())

    def test_archive_checksum_mismatch_and_traversal_or_duplicate_members_refuse(self):
        self.write([('tauri-1.0.0/build.rs',b'official template')])
        with self.archive.open('ab') as stream:stream.write(b'changed archive')
        with self.assertRaisesRegex(ValueError,'checksum differs'):
            library.registry_source(self.workspace,self.cargo,'tauri')
        for members in ([('tauri-1.0.0/../outside',b'bad')],
                        [('tauri-1.0.0/build.rs',b'one'),('tauri-1.0.0/build.rs',b'two')]):
            self.write(members)
            with self.subTest(members=members),self.assertRaises(ValueError):
                library.registry_source(self.workspace,self.cargo,'tauri')


class Configuration(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.workspace=self.root/'workspace'
        self.frontend=self.root/'frontend';self.frontend.mkdir()
        self.rustup=self.root/'rustup';self.toolchain=self.rustup/'toolchains/public-toolchain'
        self.sdk=self.root/'sdk';self.cargo=self.root/'cargo';self.cargo.mkdir()
        native=self.sdk/'ndk/27.0.12077973/toolchains/llvm/prebuilt/windows-x86_64/bin'
        self.tools=dict(cargo=self.toolchain/'bin/cargo.exe',rustc=self.toolchain/'bin/rustc.exe',
            rustcDriver=self.toolchain/'bin/rustc_driver_fixture.dll',
            linker=native/'x86_64-linux-android28-clang.cmd',clang=native/'clang.exe',ar=native/'llvm-ar.exe',
            ndkProperties=self.sdk/'ndk/27.0.12077973/source.properties')
        for name,path in self.tools.items():
            path.parent.mkdir(parents=True,exist_ok=True)
            path.write_bytes(b'Pkg.Revision = 27.0.12077973\n' if name=='ndkProperties' else name.encode())
        self.standard=self.toolchain/'lib/rustlib'/library.TARGET/'lib';self.standard.mkdir(parents=True)
        (self.standard/'libcore.rlib').write_bytes(b'fixture core')
        (self.standard/'libstd.rlib').write_bytes(b'fixture standard')
        manifest=self.workspace/'tauri/src-tauri/Cargo.toml';manifest.parent.mkdir(parents=True)
        manifest.write_text('[features]\ndefault = ["custom-protocol"]\ncustom-protocol = ["tauri/custom-protocol"]\n')
        self.config=dict(schemaVersion=1,frontendResult='bound-receipt',frontendResultSha256='0'*64,
            sdkRoot=str(self.sdk),ndkVersion='27.0.12077973',toolchainRoot=str(self.toolchain),
            rustupHome=str(self.rustup),cargoHome=str(self.cargo),
            toolSha256={name:frontend_build.digest(path) for name,path in self.tools.items()},
            targetStdFiles=frontend_build.files(self.standard))
        self.output=self.root/'new output'
        self.payload=patch.object(build_payload,'load',return_value=dict(workspace=str(self.workspace),frontend=str(self.frontend)))
        self.payload.start();self.addCleanup(self.payload.stop)
        self.codegen=patch.object(library,'official_codegen',return_value={'official':'fixture'})
        self.generated=self.codegen.start();self.addCleanup(self.codegen.stop)

    def test_complete_standard_and_declared_tools_reach_codegen_without_creating_output(self):
        prepared=library.configure(self.config,self.output)
        self.assertEqual(prepared['standard'],self.standard)
        self.assertEqual(prepared['tools'],self.tools)
        self.generated.assert_called_once_with(self.workspace/'tauri',self.cargo)
        self.assertFalse(self.output.exists())

    def test_missing_extra_and_modified_standard_entries_refuse_before_codegen(self):
        rows=self.config['targetStdFiles']
        missing=dict(rows);missing.pop('libcore.rlib')
        extra=dict(rows,unexpected=dict(bytes=1,sha256='0'*64))
        changed=copy.deepcopy(rows);changed['libcore.rlib']['sha256']='0'*64
        for value in (missing,extra,changed):
            with self.subTest(value=value),self.assertRaisesRegex(ValueError,'Complete Android Rust standard library differs'):
                library.configure(dict(self.config,targetStdFiles=value),self.output)
        self.generated.assert_not_called();self.assertFalse(self.output.exists())

    def test_changed_declared_tool_refuses_before_codegen(self):
        (self.tools['clang']).write_bytes(b'changed compiler')
        with self.assertRaisesRegex(ValueError,'Declared compiler tool changed'):
            library.configure(self.config,self.output)
        self.generated.assert_not_called();self.assertFalse(self.output.exists())


if __name__=='__main__':unittest.main()
