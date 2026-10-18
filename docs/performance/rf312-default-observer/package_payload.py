"""Source-bound inputs and byte checks for a fresh default Android package."""
import hashlib
import json
from pathlib import Path
import re
import tarfile
import zipfile

import build_payload
from frontend_build import entry, files, relative, validate_files
import library_build
from prepare_avd import ordinary
from runtime_inputs import decode, digest, hash_value

CORE='lib/x86_64/libsolo_soul.so'
PROFILE_NAMES={'dexopt/baseline.prof','dexopt/baseline.profm'}
MODULES={'tauri-android':'tauri','tauri-plugin-biometric':'tauri-plugin-biometric',
    'tauri-plugin-dialog':'tauri-plugin-dialog','tauri-plugin-fs':'tauri-plugin-fs',
    'tauri-plugin-notification':'tauri-plugin-notification','tauri-plugin-shell':'tauri-plugin-shell'}


def load_library(receipt, expected_sha256):
    receipt=ordinary(receipt)
    if digest(receipt)!=hash_value(expected_sha256):raise ValueError('Declared library receipt changed')
    raw=receipt.read_bytes();result=decode(raw);out=receipt.parent
    flags=('passed','actualCargoReleaseBuilt','defaultFeaturesOnly','frozenInputsHeld')
    if (any(result.get(n) is not True for n in flags) or type(result.get('exitCode')) is not int
            or result['exitCode']!=0 or result.get('productInstalled') is not False
            or result.get('nativeRuntimeAccepted') is not False or result.get('fullRF312Accepted') is not False
            or any(type(result.get(n)) is not int or result[n]!=0 for n in ('deviceCommandsExecuted','targetInputsDispatched'))):
        raise ValueError('Passing default-library generation receipt required')
    proof_path=ordinary(out/'input-proof.json')
    if digest(proof_path)!=hash_value(result['inputProofSha256']):raise ValueError('Library input proof changed')
    proof=decode(proof_path.read_bytes());config=decode(ordinary(out/'config.json').read_bytes())
    if result['compilerSourceSha256']!=library_build.sources() or proof['compilerSourceSha256']!=library_build.sources():
        raise ValueError('Recorded library compiler source required')
    payload=build_payload.load(config['frontendResult'],config['frontendResultSha256'])
    if (payload!=proof['payload'] or result['frontendReceiptSha256']!=config['frontendResultSha256']
            or result['sourceHead']!=payload['sourceHead']
            or ordinary(result['materializedWorkspace'],directory=True)!=Path(payload['workspace'])
            or ordinary(result['frontendRoot'],directory=True)!=Path(payload['frontend'])):
        raise ValueError('Exact source and frontend library binding required')
    library=ordinary(out/'release-libsolo_soul.so')
    if ordinary(result['path'])!=library or entry(library)!={k:result[k] for k in ('bytes','sha256')}:
        raise ValueError('Copied generated default library differs')
    artifact_path,artifact=library_build.compiler_artifact((out/'cargo.stdout').read_bytes(),
        Path(payload['workspace'])/'tauri',out/'cargo-target')
    if entry(artifact_path)!=entry(library) or artifact['features']!=result['actualCargoFeatures'] or artifact['profile']!=result['actualCargoProfile']:
        raise ValueError('Actual compiler artifact differs from default library')
    verified=library_build.verify_library(library,Path(payload['frontend']))
    body=library.read_bytes()
    if verified['entrypoints']!=result['entrypoints'] or any(name.encode() not in body for name in payload['outputs']['frontend']):
        raise ValueError('Complete production frontend keys required')
    generated=Path(payload['workspace'])/'tauri/src-tauri/gen/android'
    bridge=generated/'app/src/main/java/com/solosoul/app/generated'
    if files(bridge)!=result['generatedAndroidBridge'] or files(bridge)!=proof['officialCodegen']['bridge']:
        raise ValueError('Same-build official bridge required')
    for name,row in result['generatedAndroidMetadata'].items():
        relative(name)
        if entry(generated/name)!=row:raise ValueError('Same-build Android metadata changed')
    library_build.verify_gradle_metadata(generated,proof['officialCodegen']['androidLibraryRoots'])
    if receipt.read_bytes()!=raw:raise ValueError('Library receipt changed during validation')
    return dict(receipt=str(receipt),receiptSha256=expected_sha256,payload=payload,
        library=dict(path=str(library),bytes=result['bytes'],sha256=result['sha256']),
        officialCodegen=proof['officialCodegen'],generatedBridge=result['generatedAndroidBridge'],
        generatedMetadata=result['generatedAndroidMetadata'],cargoHome=config['cargoHome'])


def source_assets(payload):
    root=Path(payload['workspace'])/'tauri/src-tauri';rows={}
    prefix='tauri/src-tauri/gen/android/app/src/main/assets/'
    for key,expected in payload['sources'].items():
        if not key.startswith(prefix):continue
        name=key[len(prefix):];relative(name)
        path=Path(payload['workspace'])/key
        if entry(path)!=expected:raise ValueError('Tracked Android asset changed')
        rows[name]=dict(path=str(path),**expected)
    for name,expected in payload['outputs']['resources'].items():
        relative(name)
        if name.startswith('docs/'):path=root/'resources'/name
        elif name.startswith('SoloSoul_plugin_market/'):path=root/'resources-mobile'/name
        else:raise ValueError('Known current bundled resource root required')
        if entry(path)!=expected:raise ValueError('Current source asset changed')
        rows[name]=dict(path=str(path),**expected)
    name='bundled-resource-manifest.json';path=root/'resources-mobile'/name
    expected=payload['outputs']['mobile'][name]
    if entry(path)!=expected:raise ValueError('Current bundled manifest differs')
    rows[name]=dict(path=str(path),**expected)
    validate_files({n:{k:r[k] for k in ('bytes','sha256')} for n,r in rows.items()})
    return rows


def stage_module(workspace,cargo_home,label,target,product_sources):
    if label not in MODULES:raise ValueError('Known locked Android module required')
    if target.exists() or target.is_symlink():raise ValueError('New private module directory required')
    ordinary(target.parent,directory=True)
    _,checksums,archive,archive_row=library_build.registry_source(workspace,cargo_home,MODULES[label])
    prefix='mobile/android/' if label=='tauri-android' else 'android/'
    expected={n[len(prefix):]:value for n,value in checksums.items() if n.startswith(prefix)}
    data={}
    with tarfile.open(archive,'r:gz') as reader:
        crate_prefix=MODULES[label]+'-'
        for member in reader.getmembers():
            if not member.isfile():continue
            package,name=member.name.split('/',1)
            if not package.startswith(crate_prefix) or not name.startswith(prefix):continue
            name=name[len(prefix):];relative(name)
            with reader.extractfile(member) as stream:body=stream.read()
            if name in data or hashlib.sha256(body).hexdigest()!=expected[name]:raise ValueError('Locked Android archive member differs')
            data[name]=body
    if set(data)!=set(expected) or not data or entry(archive)!=archive_row:
        raise ValueError('Complete unchanged Android archive required')
    original={n:dict(bytes=len(body),sha256=hashlib.sha256(body).hexdigest()) for n,body in data.items()};validate_files(original)
    patches={}
    if label=='tauri-android':
        for name in ('PluginManager.kt','PluginHandle.kt'):
            source=workspace/'src-tauri/gen/android/patches/tauri-android'/name
            key='tauri/src-tauri/gen/android/patches/tauri-android/'+name
            expected_row=product_sources[key]
            if entry(source)!=expected_row:raise ValueError('Canonical product Android patch changed')
            destination='src/main/java/app/tauri/plugin/'+name
            if destination not in data:raise ValueError('Official Android patch target missing')
            body=source.read_bytes()
            if b'SoloSoul PATCH' not in body:raise ValueError('Canonical Android patch marker required')
            patches[destination]=body
    target.mkdir(exist_ok=False)
    for name,body in data.items():
        path=target/relative(name);path.parent.mkdir(parents=True,exist_ok=True)
        with path.open('xb') as stream:stream.write(body)
    if files(target)!=original:raise ValueError('Private official module copy differs')
    for name,body in patches.items():(target/name).write_bytes(body)
    rows={n:dict(bytes=len(body),sha256=hashlib.sha256(body).hexdigest()) for n,body in {**data,**patches}.items()}
    if files(target)!=rows:raise ValueError('Canonical private module patch differs')
    return dict(module=label,archive=dict(path=str(archive),**archive_row),files=rows,
        canonicalProductPatches=sorted(patches),sharedCacheSourceWritten=False)


def zip_files(archive):
    names=archive.namelist()
    if len(names)!=len(set(names)):raise ValueError('Duplicate APK entries')
    for name in names:relative(name[:-1] if name.endswith('/') else name)
    return {name for name in names if not name.endswith('/')}


def row(body):return dict(bytes=len(body),sha256=hashlib.sha256(body).hexdigest())


def verify_apk_bytes(apk,current_assets,library,reference,profiles):
    previous=ordinary(reference['apk']['path'])
    if entry(previous)!={k:reference['apk'][k] for k in ('bytes','sha256')}:
        raise ValueError('Frozen dependency APK changed')
    if set(profiles)!=PROFILE_NAMES:raise ValueError('Complete generated profile pair required')
    validate_files(current_assets);validate_files(profiles);validate_files({'core':library})
    with zipfile.ZipFile(previous) as old,zipfile.ZipFile(ordinary(apk)) as new:
        old_names=zip_files(old);new_names=zip_files(new)
        old_assets={n[7:]:row(old.read(n)) for n in old_names if n.startswith('assets/')}
        for name,sha in reference['appAssets'].items():
            relative(name)
            if old_assets[name]['sha256']!=hash_value(sha):raise ValueError('Frozen old app asset differs')
        if not PROFILE_NAMES.issubset(old_assets):raise ValueError('Original profile pair missing')
        dependency={n:r for n,r in old_assets.items() if n not in reference['appAssets'] and n not in PROFILE_NAMES}
        if set(current_assets)&(set(dependency)|PROFILE_NAMES):raise ValueError('Disjoint app and generated dependency assets required')
        expected={**current_assets,**dependency,**profiles}
        actual={n[7:]:row(new.read(n)) for n in new_names if n.startswith('assets/')}
        if actual!=expected:raise ValueError('Complete source, dependency and profile asset set differs')
        old_native={n:row(old.read(n)) for n in old_names if n.startswith('lib/')}
        if CORE not in old_native or any(not n.startswith('lib/x86_64/') for n in old_native):
            raise ValueError('Frozen x86_64 dependency library set required')
        native={n:row(new.read(n)) for n in new_names if n.startswith('lib/')}
        if native!={**old_native,CORE:library}:raise ValueError('Default core or original dependency library differs')
        body=new.read(CORE)
        if any(value in body for value in (b'android-native-ipc-transport-observer',b'__SOLOSOUL_ANDROID_NATIVE_PERF__')):
            raise ValueError('Optional native observer in default APK')
    return dict(passed=True,appAssets=len(current_assets),dependencyAssets=len(dependency),
        generatedProfiles=len(profiles),allNativeLibraries=len(native),exactCurrentCoreAndOriginalDependencies=True,
        oldAppAssetTreeNotCopiedOrReinterpreted=True,fullRF312Accepted=False)
