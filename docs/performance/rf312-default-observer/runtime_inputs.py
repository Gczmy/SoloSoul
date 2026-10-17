"""Bind a verified default-build snapshot to explicit local runtime paths."""
import argparse
import ast
import copy
import hashlib
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import sys
import zipfile

S = Path(__file__).resolve().parent
RUNTIME_FILES = ('default_driver.py', 'default_journey.py', 'observer_session.py', 'process_identity.py',
                 'native_journey_contract.py', 'live_pixel_counter.py', 'public_pixel_counter.py',
                 'read-public-counter.ps1', 'adb_control.py', 'body_navigation.py', 'preview_scope.py',
                 'scaled_ocr.py', 'persistent_adapter.py', 'persistent_control.py', 'persistent_control.sh',
                 'scroll_readiness.py', 'memory_checkpoint.py', 'runtime_inputs.py', 'Observer.java', 'AndroidManifest.xml')
CONFIG_FIELDS = {'schemaVersion', 'referenceInputs', 'referenceWorkspace', 'workspaceRoot', 'sdkRoot',
                 'referenceInputsSha256', 'adbSha256', 'serial', 'avd', 'observerPackage',
                 'observerPackageSha256', 'artifactPaths', 'fixtureRoots', 'externalSourceRoots'}
ARTIFACT_ROLES = {'release', 'originalDebug', 'originalTest', 'runtimeTest', 'nativeLibrary', 'observer'}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:raise ValueError('Duplicate input member')
        result[key] = value
    return result


def decode(raw):
    def constant(_value):raise ValueError('Nonfinite input value')
    return json.loads(raw.decode('utf-8-sig'), object_pairs_hook=_object, parse_constant=constant)


def hash_value(value):
    if not isinstance(value, str) or not re.fullmatch('[a-f0-9]{64}', value):
        raise ValueError('Explicit SHA-256 required')
    return value


def relative_source(name, reference_root):
    if not isinstance(name, str) or not name or '\x00' in name:
        raise ValueError('Source name required')
    path = PureWindowsPath(name)
    if '..' in path.parts:raise ValueError('Source traversal refused')
    if path.is_absolute():path = path.relative_to(PureWindowsPath(reference_root))
    if path.drive or path.root or not path.parts:raise ValueError('Workspace-relative source required')
    return path.as_posix()


def source_sets(sources, reference_root, external_roots):
    if not isinstance(sources,dict) or not sources:raise ValueError('Complete original source bindings required')
    if not isinstance(reference_root,str) or not PureWindowsPath(reference_root).is_absolute():
        raise ValueError('Original absolute Windows workspace required')
    if not isinstance(external_roots, dict):raise ValueError('Explicit external source roots required')
    workspace, external = {}, {}
    for group, row in external_roots.items():
        if not isinstance(group,str) or not re.fullmatch('[a-z][a-z0-9-]{0,31}',group) or set(row)!={'referenceRoot','root'}:
            raise ValueError('Named external root with original and current paths required')
        if not PureWindowsPath(row['referenceRoot']).is_absolute():raise ValueError('Original external root required')
        root=Path(row['root']).resolve(strict=True)
        if not root.is_dir():raise ValueError('Existing external source directory required')
        external[group]=dict(root=str(root),files={})
    for name,value in sources.items():
        if not isinstance(name,str) or not name:raise ValueError('Source name required')
        path=PureWindowsPath(name)
        if '..' in path.parts:raise ValueError('Source traversal refused')
        if not path.is_absolute() or path.is_relative_to(PureWindowsPath(reference_root)):
            target=workspace;key=relative_source(name,reference_root)
        else:
            groups=[group for group,row in external_roots.items() if path.is_relative_to(PureWindowsPath(row['referenceRoot']))]
            if len(groups)!=1:raise ValueError('Exactly one explicit external source root required')
            group=groups[0];target=external[group]['files'];key=relative_source(name,external_roots[group]['referenceRoot'])
        if key in target:raise ValueError('Rebinding cannot collapse source entries')
        target[key]=hash_value(value)
    if not workspace or any(not row['files'] for row in external.values()):raise ValueError('Every declared source root must be bound')
    if len(workspace)+sum(len(row['files']) for row in external.values())!=len(sources):raise ValueError('Original source entries must all survive')
    return workspace,external


def source_path(root, name):
    if not isinstance(name, str) or not name:raise ValueError('Bound relative source name required')
    path = PurePosixPath(name)
    if ('\\' in name or path.is_absolute()
            or '..' in path.parts or ':' in name):
        raise ValueError('Bound relative source name required')
    root = Path(root).resolve(strict=True)
    resolved = (root / name).resolve(strict=True)
    if not resolved.is_relative_to(root) or not resolved.is_file():raise ValueError('Source outside declared workspace')
    return resolved


def verify_sources(inputs):
    host = inputs['host']
    if digest(host['adbExecutable']) != hash_value(host['adbSha256']):raise ValueError('Declared SDK bytes changed')
    if set(inputs['fullJourneyFiles']) != set(RUNTIME_FILES):raise ValueError('Complete runtime source set required')
    for name, value in inputs['fullJourneyFiles'].items():
        if digest(source_path(S, name)) != hash_value(value):raise ValueError('Frozen journey source changed')
    for name, value in inputs['runtimeSourceFiles'].items():
        if digest(source_path(host['workspaceRoot'], name)) != hash_value(value):raise ValueError('Frozen product source changed')
    for row in inputs['externalSourceFiles'].values():
        for name,value in row['files'].items():
            if digest(source_path(row['root'],name))!=hash_value(value):raise ValueError('Frozen external source changed')


def verify_artifacts(inputs):
    release = inputs['release']
    if (release.get('acceptedPackagePreparation') is not True or release.get('defaultFeaturesOnly') is not True
            or release.get('applicationDebuggable') is not False or release.get('observerMarkersPresent') is not False
            or release.get('sourceHead') != inputs['compiledSourceHead']):
        raise ValueError('Verified non-debug default Release provenance required')
    library = release['library']
    if library.get('actualCargoReleaseBuilt') is not True or library.get('defaultFeaturesOnly') is not True or library.get('earlyObserverMarkersPresent') is not False:
        raise ValueError('Original default native library provenance required')
    for row in [release['apk'], inputs['originalDebug'], inputs['originalTest'], inputs['runtimeTest'], library]:
        path = Path(row.get('path', row.get('copy')))
        if not path.is_file() or digest(path) != hash_value(row['sha256']) or path.stat().st_size != row['bytes']:
            raise ValueError('Declared APK or native library changed')
    with zipfile.ZipFile(release['apk']['path']) as archive:
        if len(archive.namelist()) != len(set(archive.namelist())):raise ValueError('Duplicate APK entries')
        core = archive.read('lib/x86_64/libsolo_soul.so')
        if hashlib.sha256(core).hexdigest() != library['sha256']:
            raise ValueError('Packaged library differs from verified default build')
        if b'android-native-ipc-transport-observer' in core or b'__SOLOSOUL_ANDROID_NATIVE_PERF__' in core:
            raise ValueError('Optional observer binary cannot be measured as default')
    package = inputs['observer']
    if (package.get('passed') is not True or package.get('instrumentationTarget') != 'org.solosoul.rf312observer'
            or package.get('targetInstrumentationInjected') is not False
            or digest(package['apk']) != hash_value(package['sha256'])):
        raise ValueError('Verified separate observer artifact required')
    if set(package['sourceSha256']) != {'Observer.java', 'AndroidManifest.xml'}:
        raise ValueError('Complete observer source binding required')
    for name, value in package['sourceSha256'].items():
        if digest(S/name) != hash_value(value):raise ValueError('Observer source differs from compiled package')
    for count in ('100', '5000'):
        fixture = inputs['fixedFixtures'][count];root = Path(fixture['root'])
        actual = {}
        for path in root.rglob('*'):
            if path.is_symlink() or getattr(path.lstat(), 'st_file_attributes', 0) & 0x400:
                raise ValueError('Ordinary fixed public fixture required')
            if path.is_file():actual[path.relative_to(root).as_posix()] = digest(path)
        if not actual or actual != fixture['files']:raise ValueError('Complete public fixture bytes required')
    image = inputs['publicOcrImage']
    if digest(image['path']) != hash_value(image['sha256']):raise ValueError('Public image changed')


def validate_shape(inputs):
    if not isinstance(inputs, dict) or inputs.get('portableFormat') != 1 or type(inputs.get('portableFormat')) is not int:
        raise ValueError('Prepared portable input format required')
    host = inputs['host']
    if set(host) != {'workspaceRoot', 'sdkRoot', 'adbExecutable', 'adbSha256', 'serial', 'avd'}:
        raise ValueError('Explicit complete host configuration required')
    if not re.fullmatch('emulator-[0-9]+', host['serial']) or not re.fullmatch('SoloSoul_[A-Za-z0-9_-]+', host['avd']):
        raise ValueError('Dedicated emulator identity required')
    if Path(host['adbExecutable']).resolve() != (Path(host['sdkRoot'])/'platform-tools/adb.exe').resolve():
        raise ValueError('SDK executable must belong to the declared SDK')
    if set(inputs['fixedFixtures']) != {'100', '5000'}:raise ValueError('Both original fixture sizes required')
    if inputs['externalObserverSha256'] != inputs['fullJourneyFiles']['default_journey.py']:
        raise ValueError('Journey entry source binding differs')
    if inputs['originalSourceEntries']!=len(inputs['runtimeSourceFiles'])+sum(len(row['files']) for row in inputs['externalSourceFiles'].values()):
        raise ValueError('No original source entry may be dropped')
    return inputs


def load_inputs(path):
    inputs = validate_shape(decode(Path(path).read_bytes()))
    verify_sources(inputs)
    verify_artifacts(inputs)
    return inputs


def bind_reference(reference, config, observer):
    inputs = copy.deepcopy(reference)
    inputs['portableFormat'] = 1
    inputs['runtimeSourceFiles'],inputs['externalSourceFiles'] = source_sets(reference['runtimeSourceFiles'], config['referenceWorkspace'],config['externalSourceRoots'])
    inputs['originalSourceEntries']=len(reference['runtimeSourceFiles'])
    workspace = Path(config['workspaceRoot']).resolve(strict=True)
    sdk = Path(config['sdkRoot']).resolve(strict=True)
    inputs['host'] = dict(workspaceRoot=str(workspace), sdkRoot=str(sdk),
        adbExecutable=str(sdk/'platform-tools/adb.exe'), adbSha256=hash_value(config['adbSha256']),
        serial=config['serial'], avd=config['avd'])
    for role in ('release', 'originalDebug', 'originalTest', 'runtimeTest', 'nativeLibrary'):
        row = inputs['release']['apk'] if role == 'release' else inputs['release']['library'] if role == 'nativeLibrary' else inputs[role]
        path = str(Path(config['artifactPaths'][role]).resolve(strict=True))
        row['path'] = path
        if 'copy' in row:row['copy'] = path
    for count in ('100', '5000'):
        inputs['fixedFixtures'][count]['root'] = str(Path(config['fixtureRoots'][count]).resolve(strict=True))
    image = relative_source(inputs['publicOcrImage']['path'], config['referenceWorkspace'])
    inputs['publicOcrImage']['path'] = str(source_path(workspace, image))
    inputs['observer'] = copy.deepcopy(observer)
    inputs['observer']['apk'] = str(Path(config['artifactPaths']['observer']).resolve(strict=True))
    inputs['fullJourneyFiles'] = {name: digest(S/name) for name in RUNTIME_FILES}
    inputs['pixelInputFiles'] = {name: digest(S/name) for name in ('read-public-counter.ps1', 'public_pixel_counter.py', 'live_pixel_counter.py')}
    inputs.update(externalObserverSha256=digest(S/'default_journey.py'),
        journeyObserverSessionSha256=digest(S/'observer_session.py'),processReaderSha256=digest(S/'process_identity.py'),
        livePixelCounterSha256=digest(S/'live_pixel_counter.py'))
    original = (workspace/'tauri/scripts/android-home-native-regression.py').read_text(encoding='utf-8')
    extracted = (S/'process_identity.py').read_text(encoding='utf-8')
    def function(text):
        return ast.get_source_segment(text,next(n for n in ast.parse(text).body if isinstance(n,ast.FunctionDef) and n.name=='read_process_ids'))
    if function(original) != function(extracted):raise ValueError('Canonical PID reader changed; update explicitly')
    return validate_shape(inputs)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True, help='New local directory; never overwrites')
    args = parser.parse_args(argv)
    try:
        raw = args.config.read_bytes();config = decode(raw)
        if (not isinstance(config, dict) or set(config) != CONFIG_FIELDS
                or type(config['schemaVersion']) is not int or config['schemaVersion'] != 1
                or set(config['artifactPaths']) != ARTIFACT_ROLES or set(config['fixtureRoots']) != {'100', '5000'}):
            raise ValueError('Complete explicit runtime preparation configuration required')
        reference_path = Path(config['referenceInputs']);reference_bytes = reference_path.read_bytes()
        package_path = Path(config['observerPackage']);package_bytes = package_path.read_bytes()
        if digest(reference_path)!=hash_value(config['referenceInputsSha256']) or digest(package_path)!=hash_value(config['observerPackageSha256']):
            raise ValueError('Declared verified reference or observer package changed')
        inputs = bind_reference(decode(reference_bytes), config, decode(package_bytes))
        if args.output.exists() or args.output.is_symlink():raise ValueError('New output required')
    except (OSError, ValueError, KeyError, TypeError):
        print('Invalid runtime preparation inputs or existing output', file=sys.stderr)
        return 2
    args.output.mkdir(parents=True, exist_ok=False)
    result = dict(passed=False, sdkCommandsExecuted=0, targetInputsDispatched=0,
                  fullPortableRunnerAccepted=False, fullRF312Accepted=False)
    try:
        verify_sources(inputs);verify_artifacts(inputs)
        if args.config.read_bytes()!=raw or reference_path.read_bytes()!=reference_bytes or package_path.read_bytes()!=package_bytes:
            raise ValueError('Preparation inputs changed during verification')
        output = json.dumps(inputs, indent=2)+'\n'
        (args.output/'inputs.json').write_text(output, encoding='utf-8')
        result.update(passed=True, runtimeSourceFiles=inputs['originalSourceEntries'],workspaceSourceFiles=len(inputs['runtimeSourceFiles']),
            externalSourceFiles=sum(len(row['files']) for row in inputs['externalSourceFiles'].values()),
            journeySourceFiles=len(inputs['fullJourneyFiles']), fixtureSizes=[100,5000],
            inputsSha256=digest(args.output/'inputs.json'))
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile) as error:
        result['errorType']=type(error).__name__;result['error']=str(error)
    (args.output/'config.json').write_bytes(raw)
    (args.output/'binding-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result))
    return 0 if result['passed'] else 1


if __name__ == '__main__':sys.exit(main())
