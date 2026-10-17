"""Build a metadata-only target instrumentation APK for non-default diagnostics.

The separate default UI observer and its rejection rules remain unchanged.
This builder neither starts instrumentation nor accepts native measurements.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

import rf312_ipc_package as package
from rf312_ipc_library import ordinary, command_path, source_path
import build_observer as default_observer

HERE = Path(__file__).resolve().parent
SOURCE = HERE/'rf312-ipc-collector'
APP = 'com.solosoul.app'
PACKAGE = 'org.solosoul.rf312ipc'
CLASS = PACKAGE+'.Collector'
ANDROID = 'http://schemas.android.com/apk/res/android'
PAYLOAD = ('Collector.java', 'AndroidManifest.xml', 'snapshot.js')
FIELDS = {'schemaVersion', 'runtimeInputs', 'runtimeInputsSha256', 'diagnosticLibraryReceipt',
          'diagnosticLibraryReceiptSha256', 'defaultPackageConfig', 'defaultPackageConfigSha256',
          'nodeExecutable', 'toolSha256'}
digest = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()


def sources():
    return {**{'package/'+name: value for name, value in package.source_hashes().items()},
            **{'collector/'+name: digest(SOURCE/name) for name in [*PAYLOAD, 'query.node.test.mjs']},
            'rf312_ipc_collector.py': digest(Path(__file__)),
            'test_rf312_ipc_collector.py': digest(HERE/'test_rf312_ipc_collector.py')}


def validate_config(config):
    if (not isinstance(config, dict) or set(config) != FIELDS or type(config['schemaVersion']) is not int
            or config['schemaVersion'] != 1 or not isinstance(config['toolSha256'], dict)
            or set(config['toolSha256']) != {'d8', 'zipalign', 'node'}):
        raise ValueError('Complete explicit collector configuration required')
    for name in ['runtimeInputs', 'diagnosticLibraryReceipt', 'defaultPackageConfig', 'nodeExecutable']:
        if not isinstance(config[name], str) or not config[name] or not Path(config[name]).is_absolute():
            raise ValueError('Explicit absolute collector input paths required')
    for name in ['runtimeInputs', 'diagnosticLibraryReceipt', 'defaultPackageConfig']:
        package.producer.hash_value(config[name+'Sha256'])
    for value in config['toolSha256'].values():
        package.producer.hash_value(value)


def source_identity(raw):
    root = ET.fromstring(raw)
    if root.tag != 'manifest' or root.attrib != {'package': PACKAGE}:
        raise ValueError('Exact diagnostic collector package required')
    children = list(root)
    if sorted(node.tag for node in children) != ['application', 'instrumentation', 'uses-sdk'] or any(list(node) for node in children):
        raise ValueError('Only three direct metadata collector manifest children allowed')
    expected = {
        'uses-sdk': {'minSdkVersion': '28', 'targetSdkVersion': '35'},
        'application': {'debuggable': 'false', 'allowBackup': 'false', 'hasCode': 'true'},
        'instrumentation': {'name': CLASS, 'targetPackage': APP, 'targetProcesses': APP,
                            'functionalTest': 'true', 'handleProfiling': 'false'}}
    for node in children:
        if node.attrib != {'{'+ANDROID+'}'+name: value for name, value in expected[node.tag].items()}:
            raise ValueError('Explicit non-debug collector attributes and target process required')
    return dict(package=PACKAGE, instrumentationName=CLASS, instrumentationTarget=APP, targetProcesses=APP)


def compiled_identity(text):
    stack, nodes = [], []
    for line in text.splitlines():
        match = re.fullmatch(r'( *)E: ([A-Za-z][A-Za-z0-9_.-]*) \(line=\d+\)', line)
        if match:
            depth = len(match[1])
            while stack and stack[-1]['depth'] >= depth:
                stack.pop()
            node = dict(tag=match[2], depth=depth, parent=stack[-1]['tag'] if stack else None, attributes={})
            nodes.append(node); stack.append(node); continue
        match = re.fullmatch(r'( *)A: ([^=]+)=(.*)', line)
        if match:
            if not stack or len(match[1]) <= stack[-1]['depth']:
                raise ValueError('Compiled attribute has no owning element')
            name = match[2].split('(', 1)[0]
            if name in stack[-1]['attributes']:
                raise ValueError('Duplicate compiled collector attribute')
            stack[-1]['attributes'][name] = match[3]; continue
        if line.strip() and not re.fullmatch(r' *N: \S+ \(line=\d+\)', line):
            raise ValueError('Unknown compiled collector manifest entry')
    roots = [node for node in nodes if node['parent'] is None]
    if (len(roots) != 1 or roots[0]['tag'] != 'manifest' or len(nodes) != 4
            or sorted(node['tag'] for node in nodes if node['parent'] == 'manifest') != ['application', 'instrumentation', 'uses-sdk']):
        raise ValueError('Exact direct collector manifest shape required')

    def string(node, key):
        value = node['attributes'].get(key)
        match = re.fullmatch(r'"([^"\\]*)"(?: \(Raw: "\1"\))?', value or '')
        if not match:
            raise ValueError('Unambiguous compiled collector string required')
        return match[1]

    def flags(node, expected):
        if set(node['attributes']) != {ANDROID+':'+key for key in expected}:
            raise ValueError('Unexpected compiled collector attributes')
        for key, value in expected.items():
            allowed = ('true', '(type 0x12)0xffffffff') if value else ('false', '(type 0x12)0x0')
            if node['attributes'][ANDROID+':'+key] not in allowed:
                raise ValueError('Explicit compiled collector booleans required')

    root = roots[0]
    if string(root, 'package') != PACKAGE or root['attributes'].get(ANDROID+':compileSdkVersion') != '36':
        raise ValueError('Exact compiled package and compile SDK required')
    if root['attributes'].get(ANDROID+':versionCode') != '1' or string(root, ANDROID+':versionName') != '0.0.1':
        raise ValueError('Exact diagnostic collector version required')
    allowed_root = {'package', 'platformBuildVersionCode', 'platformBuildVersionName',
                    ANDROID+':versionCode', ANDROID+':versionName', ANDROID+':compileSdkVersion', ANDROID+':compileSdkVersionCodename'}
    if set(root['attributes']) != allowed_root:
        raise ValueError('Unexpected root identity or shared UID attributes')
    if (root['attributes']['platformBuildVersionCode'] != '36'
            or root['attributes']['platformBuildVersionName'] != '16'
            or string(root, ANDROID+':compileSdkVersionCodename') != '16'):
        raise ValueError('Exact platform build version required')
    uses = next(node for node in nodes if node['tag'] == 'uses-sdk')
    if uses['attributes'] != {ANDROID+':minSdkVersion': '28', ANDROID+':targetSdkVersion': '35'}:
        raise ValueError('Exact collector minimum and target SDK required')
    app = next(node for node in nodes if node['tag'] == 'application')
    flags(app, dict(hasCode=True, debuggable=False, allowBackup=False))
    instrument = next(node for node in nodes if node['tag'] == 'instrumentation')
    for name, value in dict(name=CLASS, targetPackage=APP, targetProcesses=APP).items():
        if string(instrument, ANDROID+':'+name) != value:
            raise ValueError('Exact target instrumentation identity required')
    remaining = dict(instrument, attributes={key: value for key, value in instrument['attributes'].items()
                                          if key not in {ANDROID+':'+name for name in ['name', 'targetPackage', 'targetProcesses']}})
    flags(remaining, dict(functionalTest=True, handleProfiling=False))
    try:
        default_observer.compiled_manifest_identity(text)
    except ValueError as error:
        if str(error) != 'Product package cannot be an observer target':
            raise
    else:
        raise ValueError('Default observer reader accepted target instrumentation')
    return dict(package=PACKAGE, instrumentationName=CLASS, instrumentationTarget=APP, targetProcesses=APP)


def fixed_member(archive, name, raw):
    info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
    info.compress_type = zipfile.ZIP_DEFLATED
    archive.writestr(info, raw)


def build(config_file, output):
    config_file = ordinary(config_file); raw_config = config_file.read_bytes()
    config = package.producer.decode(raw_config); validate_config(config)
    bound = {name: package.producer.bound_json(config[name], config[name+'Sha256'])
             for name in ['runtimeInputs', 'diagnosticLibraryReceipt', 'defaultPackageConfig']}
    runtime = package.producer.load_inputs(config['runtimeInputs'])
    if runtime != bound['runtimeInputs']:
        raise ValueError('Original complete default input binding required')
    library = bound['diagnosticLibraryReceipt']
    if (library.get('passed') is not True or library.get('actualCargoReleaseBuilt') is not True
            or library.get('defaultFeaturesOnly') is not False
            or library.get('sameSourceAsAcceptedDefault') is not True or library.get('producerSources') != package.producer.sources()
            or library.get('originalDefaultInputsAndArtifactsPreserved') is not True
            or library.get('existingDefaultReaderRejected') is not True
            or library.get('sourceHead') != runtime['compiledSourceHead']):
        raise ValueError('Current accepted same-source diagnostic library required')
    native = ordinary(library['path'])
    verification = package.producer.verify_library(native, ordinary(library['frontendRoot'], directory=True),
                                                   runtime['externalSourceFiles']['frontend']['files'])
    if verification['sha256'] != library['sha256']:
        raise ValueError('Formal diagnostic library bytes changed')
    source_name = 'tauri/src-tauri/src/android_native_perf/observer.js'
    observer = ordinary(source_path(runtime['host']['workspaceRoot'], source_name))
    if digest(observer) != runtime['runtimeSourceFiles'].get(source_name):
        raise ValueError('Original early transport observer source required')
    baseline = bound['defaultPackageConfig']; signer = baseline['testSigner']; key = ordinary(signer['keyFile'])
    if (signer['purpose'] != 'internal-validation-only' or digest(key) != signer['sha256']
            or runtime['release']['productionSigningUsed'] is not False
            or signer['certificateSha256'] != runtime['release']['certificateSha256']):
        raise ValueError('Existing internal test signing identity required')
    sdk = ordinary(baseline['sdkRoot'], directory=True); jdk = ordinary(baseline['jdkRoot'], directory=True)
    tools_root = sdk/'build-tools/35.0.0'
    tools = dict(java=jdk/'bin/java.exe', javac=jdk/'bin/javac.exe', jvm=jdk/'bin/server/jvm.dll',
        androidJar=sdk/'platforms/android-36/android.jar', aapt2=tools_root/'aapt2.exe',
        apksigner=tools_root/'lib/apksigner.jar', d8=tools_root/'lib/d8.jar', zipalign=tools_root/'zipalign.exe',
        node=ordinary(config['nodeExecutable']))
    hashes = {}
    for name, path in tools.items():
        ordinary(path); command_path(path); hashes[name] = digest(path)
        expected = config['toolSha256'][name] if name in config['toolSha256'] else baseline['toolSha256'][name]
        if hashes[name] != package.producer.hash_value(expected):
            raise ValueError('Declared collector tool changed')
    identity = source_identity((SOURCE/'AndroidManifest.xml').read_bytes())
    before = sources(); payload_sources = {name: digest(SOURCE/name) for name in PAYLOAD}
    out = package.producer.new_output(output)
    for root in [SOURCE, HERE, sdk, jdk, Path(runtime['host']['workspaceRoot']), Path(library['path']).parent]:
        if out.is_relative_to(root):
            raise ValueError('Collector output must be outside immutable inputs')
    out.mkdir(exist_ok=False)
    for name in ['temp', 'classes', 'dex', 'assets', 'java-user']:
        (out/name).mkdir()
    (out/'config.json').write_bytes(raw_config)
    (out/'assets/snapshot.js').write_bytes((SOURCE/'snapshot.js').read_bytes())
    env = dict(os.environ, TEMP=str(out/'temp'), TMP=str(out/'temp'), TMPDIR=str(out/'temp'),
        JAVA_OPTS='', _JAVA_OPTIONS='', JDK_JAVA_OPTIONS='', RF312_BASE_OBSERVER_SOURCE=str(observer),
        JAVA_TOOL_OPTIONS='-Duser.home="'+(out/'java-user').as_posix()+'" -Djava.io.tmpdir="'+(out/'temp').as_posix()+'"')
    steps = []

    def run(label, command):
        if (sources() != before or any(digest(path) != hashes[name] for name, path in tools.items())
                or digest(key) != signer['sha256'] or digest(observer) != runtime['runtimeSourceFiles'][source_name]):
            raise ValueError('Collector producer, key, observer or tool changed')
        with (out/(label+'.stdout')).open('xb') as stdout, (out/(label+'.stderr')).open('xb') as stderr:
            result = subprocess.run([str(value) for value in command], cwd=out, env=env, stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, timeout=120, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        steps.append(dict(step=label, exitCode=result.returncode, stdoutSha256=digest(out/(label+'.stdout')), stderrSha256=digest(out/(label+'.stderr'))))
        if result.returncode != 0:
            raise ValueError('Collector command failed: '+label)
        return (out/(label+'.stdout')).read_text(encoding='utf-8')

    result = dict(passed=False, productInstalled=False, actualTargetInstrumentationStarted=False,
                  targetInstrumentationCapable=True, defaultObserverCompatible=False,
                  nativeRuntimeAccepted=False, actualIpcAccepted=False, fullRF312Accepted=False)
    try:
        query = run('query-tests', [tools['node'], '--test', '--test-reporter=tap', SOURCE/'query.node.test.mjs'])
        counters = {name: int(value) for name, value in re.findall(r'^# (tests|pass|fail|cancelled|skipped|todo) (\d+)\s*$', query, re.M)}
        if counters != {'tests': 4, 'pass': 4, 'fail': 0, 'cancelled': 0, 'skipped': 0, 'todo': 0}:
            raise ValueError('Complete four-query regression run required')
        run('javac', [tools['javac'], '--release', '11', '-encoding', 'UTF-8', '-cp', tools['androidJar'], '-d', out/'classes', SOURCE/'Collector.java'])
        classes = sorted((out/'classes').rglob('*.class'))
        if [path.relative_to(out/'classes').as_posix() for path in classes] != ['org/solosoul/rf312ipc/Collector.class']:
            raise ValueError('Only the declared collector class may compile')
        jar = out/'collector-classes.jar'
        with zipfile.ZipFile(jar, 'x') as archive:
            for path in classes:
                fixed_member(archive, path.relative_to(out/'classes').as_posix(), path.read_bytes())
        run('d8', [tools['java'], '-cp', tools['d8'], 'com.android.tools.r8.D8', '--lib', tools['androidJar'], '--min-api', '28', '--output', out/'dex', jar])
        if sorted(path.name for path in (out/'dex').iterdir()) != ['classes.dex']:
            raise ValueError('One complete collector DEX required')
        unsigned = out/'collector-unsigned.apk'
        run('aapt2', [tools['aapt2'], 'link', '-o', unsigned, '--manifest', SOURCE/'AndroidManifest.xml', '-I', tools['androidJar'], '-A', out/'assets',
            '--min-sdk-version', '28', '--target-sdk-version', '35', '--version-code', '1', '--version-name', '0.0.1'])
        with zipfile.ZipFile(unsigned, 'a') as archive:
            fixed_member(archive, 'classes.dex', (out/'dex/classes.dex').read_bytes())
        aligned = out/'collector-aligned.apk'
        run('align', [tools['zipalign'], '4', unsigned, aligned])
        apk = out/'ipc-metadata-collector.apk'; sign = [tools['java'], '-jar', tools['apksigner']]
        run('sign', sign+['sign', '--ks', key, '--ks-key-alias', 'androiddebugkey', '--ks-pass', 'pass:android', '--key-pass', 'pass:android',
            '--v1-signing-enabled', 'false', '--v2-signing-enabled', 'true', '--v3-signing-enabled', 'true', '--v4-signing-enabled', 'false', '--out', apk, aligned])
        package.check_certificate(run('verify', sign+['verify', '--verbose', '--print-certs', apk]), signer['certificateSha256'])
        run('signed-alignment', [tools['zipalign'], '-c', '4', apk])
        compiled = compiled_identity(run('manifest', [tools['aapt2'], 'dump', 'xmltree', apk, '--file', 'AndroidManifest.xml']))
        if compiled != identity:
            raise ValueError('Compiled and source collector identity differ')
        with zipfile.ZipFile(aligned) as original, zipfile.ZipFile(apk) as signed:
            expected_names = {'AndroidManifest.xml', 'resources.arsc', 'assets/snapshot.js', 'classes.dex'}
            if set(signed.namelist()) != expected_names or len(signed.namelist()) != 4 or original.namelist() != signed.namelist():
                raise ValueError('Only four complete collector payload entries allowed')
            payload = {name: hashlib.sha256(signed.read(name)).hexdigest() for name in signed.namelist()}
            if any(original.read(name) != signed.read(name) for name in signed.namelist()):
                raise ValueError('Signing changed collector payload')
            if payload['classes.dex'] != digest(out/'dex/classes.dex') or payload['assets/snapshot.js'] != payload_sources['snapshot.js']:
                raise ValueError('Compiled DEX or metadata script changed')
        if (sources() != before or config_file.read_bytes() != raw_config or package.producer.load_inputs(config['runtimeInputs']) != runtime
                or any(digest(path) != hashes[name] for name, path in tools.items()) or digest(key) != signer['sha256']
                or digest(native) != library['sha256']):
            raise ValueError('Frozen collector inputs changed')
        for name in ['runtimeInputs', 'diagnosticLibraryReceipt', 'defaultPackageConfig']:
            package.producer.bound_json(config[name], config[name+'Sha256'])
        result.update(passed=True, apk=str(apk), sha256=digest(apk), bytes=apk.stat().st_size, **identity,
            sourceSha256=before, payloadSourceSha256=payload_sources, payloadSha256=payload, toolSha256=hashes,
            compileSdk=36, minSdk=28, targetSdk=35, applicationDebuggable=False, certificateSha256=signer['certificateSha256'],
            sourceHead=library['sourceHead'], runtimeInputsSha256=config['runtimeInputsSha256'],
            diagnosticLibraryReceiptSha256=config['diagnosticLibraryReceiptSha256'],
            defaultPackageConfigSha256=config['defaultPackageConfigSha256'],
            productionSigningUsed=False, defaultObserverReaderRejected=True, containsNativeLibraries=False,
            queryTests=4, deterministicInsertedZipDates=True)
    except Exception as error:
        import traceback
        (out/'failure.traceback').write_text(traceback.format_exc(), encoding='utf-8')
        result.update(errorType=type(error).__name__, error=str(error))
    result['steps'] = steps
    with (out/'package.json').open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(result, stream, indent=2); stream.write('\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True); parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if os.name != 'nt':
        raise ValueError('This explicit SDK/JDK collector builder requires Windows')
    result = build(args.config, args.output)
    print(json.dumps({key: value for key, value in result.items() if key in ['passed', 'errorType', 'error', 'nativeRuntimeAccepted', 'actualIpcAccepted']}))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
