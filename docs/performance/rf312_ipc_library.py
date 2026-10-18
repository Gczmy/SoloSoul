"""Build an explicitly non-default IPC diagnostic from an accepted default source set.

The default library/APK readers remain unchanged. This output is never a default
performance sample and does not prove IPC counts before an actual native run.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import time
import tomllib

HERE = Path(__file__).resolve().parent
HELPER = HERE / 'rf312-default-observer'
sys.path.insert(0, str(HELPER))
import build_payload
import library_build as default_library
from frontend_build import entry, files, new_output, validate_files
from prepare_avd import ordinary, command_path
from runtime_inputs import decode, digest, hash_value, load_inputs, source_path
import runtime_units

FEATURE = 'android-native-perf'
FIELDS = {'schemaVersion', 'runtimeInputs', 'runtimeInputsSha256', 'defaultBuildConfig',
          'defaultBuildConfigSha256', 'defaultLibraryReceipt', 'defaultLibraryReceiptSha256'}
MARKERS = (b'android-native-ipc-transport-observer', b'__SOLOSOUL_ANDROID_NATIVE_PERF__')
BRIDGE = 'tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/generated/'
GENERATED = 'tauri/src-tauri/gen/android/'


def sources():
    return {**{'default/'+name: value for name, value in runtime_units.sources().items()},
            'rf312_ipc_library.py': digest(Path(__file__)),
            'test_rf312_ipc_library.py': digest(HERE/'test_rf312_ipc_library.py')}


def bound_json(path, expected):
    path = ordinary(path)
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != hash_value(expected):
        raise ValueError('Declared baseline receipt changed')
    return decode(raw)


def generated_set(library, runtime):
    rows = {BRIDGE+name: row['sha256'] for name, row in library['generatedAndroidBridge'].items()}
    rows.update({GENERATED+name: row['sha256'] for name, row in library['generatedAndroidMetadata'].items()})
    if not rows or any(runtime['runtimeSourceFiles'].get(name) != value for name, value in rows.items()):
        raise ValueError('Complete baseline Android generated files must be bound')
    return rows


def copy_bound(root, names, destination, skipped=()):
    """Copy exact ordinary sources to a fresh directory; never edit their origin."""
    if destination.exists() or destination.is_symlink():
        raise ValueError('Fresh source destination required')
    destination.mkdir(parents=True, exist_ok=False)
    copied = {}
    for name, expected in names.items():
        source = ordinary(source_path(root, name))
        raw = source.read_bytes()
        if hashlib.sha256(raw).hexdigest() != hash_value(expected):
            raise ValueError('Original bound file changed before copy')
        if name in skipped:
            continue
        target = destination/name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(raw)
        if digest(target) != expected:
            raise ValueError('Copied source bytes differ')
        copied[name] = expected
    return copied


def compiler_artifact(raw, workspace, target):
    rows = [decode(line) for line in raw.splitlines() if line.strip()]
    if not rows or rows[-1] != dict(reason='build-finished', success=True):
        raise ValueError('Successful real Cargo termination required')
    matches = [row for row in rows if row.get('reason') == 'compiler-artifact'
               and row.get('target', {}).get('name') == 'solo_soul'
               and Path(row['target']['src_path']).resolve() == (workspace/'src-tauri/src/lib.rs').resolve()]
    if len(matches) != 1:
        raise ValueError('One real product library artifact required')
    row = matches[0]
    profile = row['profile']
    if (set(row['features']) != {'default', 'custom-protocol', FEATURE} or len(row['features']) != 3 or row.get('fresh') is not False
            or profile['opt_level'] != '3' or profile['debug_assertions'] is not False
            or profile.get('debuginfo') != 0 or profile['test'] is not False or 'cdylib' not in row['target']['kind']):
        raise ValueError('Fresh optimized library with only the explicit diagnostic feature required')
    path = target/default_library.TARGET/'release/libsolo_soul.so'
    if [Path(name).resolve() for name in row['filenames'] if Path(name).name == 'libsolo_soul.so'] != [path.resolve()]:
        raise ValueError('Exact new diagnostic target required')
    return path, row


def verify_library(path, frontend, frontend_files):
    raw = ordinary(path).read_bytes()
    if raw[:6] != b'\x7fELF\x02\x01' or len(raw) < 64 or struct.unpack_from('<HH', raw, 16) != (3, 62):
        raise ValueError('x86_64 shared diagnostic ELF required')
    if not all(marker in raw for marker in MARKERS):
        raise ValueError('Both non-default diagnostic markers required')
    if not frontend_files:
        raise ValueError('Complete unchanged frontend required')
    for name, expected in frontend_files.items():
        if digest(ordinary(source_path(frontend, name))) != expected or name.encode('utf-8') not in raw:
            raise ValueError('Baseline frontend bytes or embedded key differ')
    try:
        default_library.verify_library(path, frontend)
    except ValueError as error:
        if str(error) != 'Optional native observer marker in default library':
            raise
    else:
        raise ValueError('Default reader unexpectedly accepted a diagnostic library')
    return dict(**entry(path), defaultFeaturesOnly=False, diagnosticMarkersPresent=True,
                baselineFrontendKeysVerified=len(frontend_files), existingDefaultReaderRejected=True)


def configure(config, output):
    if not isinstance(config, dict) or set(config) != FIELDS or type(config['schemaVersion']) is not int or config['schemaVersion'] != 1:
        raise ValueError('Complete explicit diagnostic configuration required')
    runtime = bound_json(config['runtimeInputs'], config['runtimeInputsSha256'])
    if load_inputs(config['runtimeInputs']) != runtime:
        raise ValueError('Current strict default runtime binding required')
    baseline = bound_json(config['defaultLibraryReceipt'], config['defaultLibraryReceiptSha256'])
    cfg = bound_json(config['defaultBuildConfig'], config['defaultBuildConfigSha256'])
    if (set(cfg) != default_library.FIELDS or baseline.get('passed') is not True
            or baseline.get('actualCargoReleaseBuilt') is not True or baseline.get('defaultFeaturesOnly') is not True
            or baseline.get('earlyObserverMarkersPresent') is not False
            or baseline['sha256'] != runtime['release']['library']['sha256']
            or baseline['sourceHead'] != runtime['compiledSourceHead']):
        raise ValueError('Accepted same-source default baseline required')
    payload = build_payload.load(cfg['frontendResult'], cfg['frontendResultSha256'])
    if (Path(payload['workspace']) != Path(runtime['host']['workspaceRoot'])
            or set(runtime['externalSourceFiles']) != {'frontend'}
            or Path(runtime['externalSourceFiles']['frontend']['root']) != Path(payload['frontend'])
            or payload['sourceHead'] != baseline['sourceHead']):
        raise ValueError('Exact accepted workspace and frontend required')
    default_library.verify_library(ordinary(baseline['path']), Path(payload['frontend']))
    original_generated = generated_set(baseline, runtime)
    sdk = ordinary(cfg['sdkRoot'], directory=True)
    toolchain = ordinary(cfg['toolchainRoot'], directory=True)
    rustup = ordinary(cfg['rustupHome'], directory=True)
    cargo_home = ordinary(cfg['cargoHome'], directory=True)
    if toolchain != (rustup/'toolchains'/toolchain.name).resolve():
        raise ValueError('Toolchain must belong to the declared rustup home')
    drivers = list((toolchain/'bin').glob('rustc_driver*.dll'))
    if len(drivers) != 1 or set(cfg['toolSha256']) != default_library.TOOLS:
        raise ValueError('Complete explicit compiler identity required')
    ndk = sdk/'ndk'/cfg['ndkVersion']
    native = ndk/'toolchains/llvm/prebuilt/windows-x86_64/bin'
    tools = dict(cargo=toolchain/'bin/cargo.exe', rustc=toolchain/'bin/rustc.exe', rustcDriver=drivers[0],
                 linker=native/'x86_64-linux-android28-clang.cmd', clang=native/'clang.exe', ar=native/'llvm-ar.exe',
                 ndkProperties=ndk/'source.properties')
    for name, path in tools.items():
        ordinary(path); command_path(path)
        if digest(path) != hash_value(cfg['toolSha256'][name]):
            raise ValueError('Baseline compiler changed')
    if 'Pkg.Revision = '+cfg['ndkVersion'] not in tools['ndkProperties'].read_text().splitlines():
        raise ValueError('NDK revision differs')
    standard = toolchain/'lib/rustlib'/default_library.TARGET/'lib'
    validate_files(cfg['targetStdFiles'])
    if files(standard) != cfg['targetStdFiles']:
        raise ValueError('Complete baseline target standard library changed')
    manifest = tomllib.loads((Path(payload['workspace'])/'tauri/src-tauri/Cargo.toml').read_text(encoding='utf-8'))
    if (manifest['features']['default'] != ['custom-protocol']
            or manifest['features']['custom-protocol'] != ['tauri/custom-protocol']
            or manifest['features'].get(FEATURE) != []):
        raise ValueError('Original default features and dependency-free diagnostic feature required')
    output = new_output(output)
    for root in [Path(payload['workspace']), Path(payload['frontend']), ordinary(baseline['path']).parent,
                 sdk, toolchain, rustup, cargo_home]:
        if output.is_relative_to(root):
            raise ValueError('Diagnostic output must be outside all immutable baseline roots')
    for path in (output, sdk, toolchain, rustup, cargo_home):
        command_path(path)
    return dict(runtime=runtime, baseline=baseline, originalPayload=payload, buildConfig=cfg,
                generated=original_generated, sdk=sdk, toolchain=toolchain, rustup=rustup, cargoHome=cargo_home,
                ndk=ndk, tools=tools, standard=standard, output=output,
                codegen=default_library.official_codegen(Path(payload['workspace'])/'tauri', cargo_home))


def build(config_file, output):
    config_file = ordinary(config_file)
    config_raw = config_file.read_bytes()
    config = decode(config_raw)
    p = configure(config, output)
    before = sources()
    out = p['output']
    out.mkdir(exist_ok=False)
    for name in ['temp', 'ort-cache']:
        (out/name).mkdir()
    (out/'config.json').write_bytes(config_raw)
    result = dict(passed=False, actualCargoReleaseBuilt=False, defaultFeaturesOnly=False,
                  optionalFeature=FEATURE, diagnosticPackageBuilt=False, nativeRuntimeAccepted=False,
                  actualIpcAccepted=False, productInstalled=False, deviceCommandsExecuted=0,
                  targetInputsDispatched=0, fullRF312Accepted=False)
    try:
        materialized = out/'materialized'
        workspace_root = materialized/'workspace'
        frontend = materialized/'frontend'
        copied = copy_bound(Path(p['runtime']['host']['workspaceRoot']), p['runtime']['runtimeSourceFiles'],
                            workspace_root, p['generated'])
        external = p['runtime']['externalSourceFiles']['frontend']
        copy_bound(Path(external['root']), external['files'], frontend)
        p.update(workspace=workspace_root/'tauri', payload=dict(frontend=str(frontend)))
        p['cargoConfigs'] = default_library.cargo_configs(p['workspace'], p['cargoHome'])
        command, environment, override, bridge = default_library.command_and_environment(p)
        command += ['--features', FEATURE]
        bridge.mkdir(parents=True, exist_ok=False)
        proof = dict(baselineConfig=config,baselineLibrarySha256=p['baseline']['sha256'],
                     baselineApkSha256=p['runtime']['release']['apk']['sha256'],sourceHead=p['baseline']['sourceHead'],
                     baselineSourceEntries=p['runtime']['originalSourceEntries'],
                     copiedWorkspaceFiles=len(copied),copiedFrontendFiles=len(external['files']),
                     derivedFilesRegeneratedInsteadOfCopied=p['generated'],
                     baselineWorkspaceSources=p['runtime']['runtimeSourceFiles'],baselineFrontendFiles=external['files'],
                     tools=p['buildConfig']['toolSha256'],targetStdFiles=p['buildConfig']['targetStdFiles'],
                     officialCodegen=p['codegen'],cargoConfigs=p['cargoConfigs'],producerSources=before,
                     freshTargetDirectory=True,targetCacheReused=False,command=command,
                     frontendOnlyConfigOverride=override,defaultFeatures=['custom-protocol'],extraFeatures=[FEATURE])
        with (out/'input-proof.json').open('x',encoding='utf-8',newline='\n') as stream:
            json.dump(proof,stream,indent=2);stream.write('\n')
        started = time.monotonic()
        with (out/'cargo.stdout').open('xb') as stdout, (out/'cargo.stderr').open('xb') as stderr:
            child = subprocess.Popen(command,cwd=p['workspace'],env=environment,stdin=subprocess.DEVNULL,
                                     stdout=stdout,stderr=stderr,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
            with (out/'live.json').open('x',encoding='utf-8',newline='\n') as stream:
                json.dump(dict(pid=child.pid,command=command),stream,indent=2)
            print('Fresh same-source non-default IPC diagnostic compilation started',flush=True)
            code = child.wait()
        result.update(exitCode=code,elapsedSeconds=time.monotonic()-started)
        if code != 0:
            raise ValueError('Diagnostic Cargo build failed; original outputs retained')
        path, artifact = compiler_artifact((out/'cargo.stdout').read_bytes(),p['workspace'],out/'cargo-target')
        if artifact['profile'] != p['baseline']['actualCargoProfile']:
            raise ValueError('Diagnostic profile differs from accepted baseline')
        verification = verify_library(path,frontend,external['files'])
        for name, expected in p['runtime']['runtimeSourceFiles'].items():
            if digest(ordinary(source_path(workspace_root,name))) != expected:
                raise ValueError('Copied or regenerated baseline source differs')
        if files(bridge) != p['codegen']['bridge']:
            raise ValueError('Official complete Android bridge differs')
        generated = p['workspace']/'src-tauri/gen/android'
        if entry(generated/'app/proguard-tauri.pro') != p['codegen']['proguard']:
            raise ValueError('Official Android ProGuard output differs')
        default_library.verify_gradle_metadata(generated,p['codegen']['androidLibraryRoots'])
        if (sources() != before or config_file.read_bytes() != config_raw
                or default_library.cargo_configs(p['workspace'],p['cargoHome']) != p['cargoConfigs']
                or files(p['standard']) != p['buildConfig']['targetStdFiles']
                or any(digest(path) != p['buildConfig']['toolSha256'][name] for name,path in p['tools'].items())
                or any(entry(path) != row for path,row in p['codegen']['sourceFiles'].items())):
            raise ValueError('Diagnostic producer, compiler or locked codegen input changed')
        if load_inputs(config['runtimeInputs']) != p['runtime']:
            raise ValueError('Original complete default baseline changed')
        for field in ['defaultBuildConfig','defaultLibraryReceipt']:
            bound_json(config[field],config[field+'Sha256'])
        build_payload.recheck(p['originalPayload'])
        copy = out/'diagnostic-libsolo_soul.so'
        shutil.copyfile(path,copy)
        if entry(copy) != {key:verification[key] for key in ['bytes','sha256']}:
            raise ValueError('Copied diagnostic library differs')
        result.update(passed=True,actualCargoReleaseBuilt=True,**verification,path=str(copy),
                      actualCargoFeatures=artifact['features'],actualCargoProfile=artifact['profile'],
                      sameSourceAsAcceptedDefault=True,baselineSourceEntries=p['runtime']['originalSourceEntries'],
                      originalDefaultInputsAndArtifactsPreserved=True,sourceHead=p['baseline']['sourceHead'],
                      materializedWorkspace=str(workspace_root),frontendRoot=str(frontend),
                      inputProofSha256=digest(out/'input-proof.json'),producerSources=before)
    except Exception as error:
        result.update(errorType=type(error).__name__,error=str(error))
    with (out/'library.json').open('x',encoding='utf-8',newline='\n') as stream:
        json.dump(result,stream,indent=2);stream.write('\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    args = parser.parse_args()
    if os.name != 'nt':
        raise ValueError('This explicit diagnostic compiler requires Windows')
    result = build(args.config,args.output)
    print(json.dumps({key:value for key,value in result.items() if key != 'producerSources'}))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
