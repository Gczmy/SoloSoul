"""Generate an explicit non-default IPC APK from an accepted default package.

Only the verified diagnostic native core may change. This producer neither
installs the APK nor accepts native IPC, UI, memory or timing measurements.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

import rf312_ipc_library as producer

HERE = Path(__file__).resolve().parent
CORE = 'lib/x86_64/libsolo_soul.so'
FIELDS = {'schemaVersion', 'runtimeInputs', 'runtimeInputsSha256',
          'diagnosticLibraryReceipt', 'diagnosticLibraryReceiptSha256',
          'defaultPackageConfig', 'defaultPackageConfigSha256', 'toolSha256'}
sha = lambda raw: hashlib.sha256(raw).hexdigest()
digest = lambda path: sha(Path(path).read_bytes())


def source_hashes():
    return {**{'library/'+name: value for name, value in producer.sources().items()},
            'rf312_ipc_package.py': digest(Path(__file__)),
            'test_rf312_ipc_package.py': digest(HERE/'test_rf312_ipc_package.py')}


def validate_config_shape(config):
    if (not isinstance(config, dict) or set(config) != FIELDS
            or type(config['schemaVersion']) is not int or config['schemaVersion'] != 1
            or not isinstance(config['toolSha256'], dict) or set(config['toolSha256']) != {'zipalign'}):
        raise ValueError('Complete explicit non-default package configuration required')
    for name in ['runtimeInputs', 'diagnosticLibraryReceipt', 'defaultPackageConfig']:
        value = config[name]
        if not isinstance(value, str) or not value or not Path(value).is_absolute():
            raise ValueError('Explicit absolute receipt paths required')
        producer.hash_value(config[name+'Sha256'])
    producer.hash_value(config['toolSha256']['zipalign'])


def archive_rows(archive):
    names = archive.namelist()
    if any(info.orig_filename != info.filename for info in archive.infolist()):
        raise ValueError('Original APK names must already be canonical')
    if len(names) != len(set(names)) or CORE not in names:
        raise ValueError('Complete unique baseline APK entries required')
    for name in names:
        path = PurePosixPath(name)
        canonical = path.as_posix()+('/' if name.endswith('/') else '')
        if (not name or '\\' in name or ':' in name or path.is_absolute() or '..' in path.parts
                or canonical != name):
            raise ValueError('Canonical in-package APK names required')
        if re.fullmatch(r'META-INF/[^/]+\.(?:SF|RSA|DSA|EC)', name, re.I):
            raise ValueError('Old v1 signature entries cannot be copied')
    return {name: sha(archive.read(name)) for name in names}


def unsigned_copy(base, native, expected_base_core, output):
    import zipfile
    if output.exists() or output.is_symlink():
        raise ValueError('New diagnostic output required')
    with zipfile.ZipFile(base) as source:
        before = archive_rows(source)
        if before[CORE] != expected_base_core or sha(native) == expected_base_core:
            raise ValueError('Exact default core and a distinct diagnostic core required')
        with zipfile.ZipFile(output, 'x') as target:
            for info in source.infolist():
                target.writestr(copy.copy(info), native if info.filename == CORE else source.read(info.filename))
    with zipfile.ZipFile(output) as result:
        after = archive_rows(result)
    if (set(before) != set(after) or [name for name in before if before[name] != after[name]] != [CORE]
            or after[CORE] != sha(native)):
        raise ValueError('Only the exact diagnostic native payload may change')
    return before, after


def verify_payload(base, signed, expected):
    import zipfile
    with zipfile.ZipFile(base) as source, zipfile.ZipFile(signed) as result:
        before, after = archive_rows(source), archive_rows(result)
    if (after != expected or set(before) != set(after)
            or [name for name in before if before[name] != after[name]] != [CORE]):
        raise ValueError('Signing or alignment changed additional APK payload')
    return after


def check_certificate(text, expected):
    if re.findall(r'certificate SHA-256 digest: (\S+)', text) != [producer.hash_value(expected)]:
        raise ValueError('One exact internal test certificate required')


def package(config_file, output):
    config_file = producer.ordinary(config_file)
    raw_config = config_file.read_bytes()
    declared = producer.decode(raw_config)
    validate_config_shape(declared)
    bound = {name: producer.bound_json(declared[name], declared[name+'Sha256'])
             for name in ['runtimeInputs', 'diagnosticLibraryReceipt', 'defaultPackageConfig']}
    inputs_file = Path(declared['runtimeInputs'])
    inputs = producer.load_inputs(inputs_file)
    if inputs != bound['runtimeInputs']:
        raise ValueError('Accepted unchanged default runtime binding required')
    library = bound['diagnosticLibraryReceipt']
    if (library.get('passed') is not True or library.get('actualCargoReleaseBuilt') is not True
            or library.get('defaultFeaturesOnly') is not False
            or library.get('sameSourceAsAcceptedDefault') is not True
            or library.get('originalDefaultInputsAndArtifactsPreserved') is not True
            or library.get('existingDefaultReaderRejected') is not True
            or library.get('producerSources') != producer.sources()
            or library.get('sourceHead') != inputs['compiledSourceHead']):
        raise ValueError('Formally accepted same-source diagnostic library required')
    native = producer.ordinary(library['path'])
    frontend = producer.ordinary(library['frontendRoot'], directory=True)
    verification = producer.verify_library(native, frontend, inputs['externalSourceFiles']['frontend']['files'])
    if verification['sha256'] != library['sha256']:
        raise ValueError('Formal diagnostic library bytes changed')
    base = producer.ordinary(inputs['release']['apk']['path'])
    base_hash = digest(base)
    config = bound['defaultPackageConfig']
    signer = config['testSigner']
    key = producer.ordinary(signer['keyFile'])
    if (signer['purpose'] != 'internal-validation-only' or digest(key) != producer.hash_value(signer['sha256'])
            or signer['certificateSha256'] != inputs['release']['certificateSha256']
            or inputs['release']['productionSigningUsed'] is not False):
        raise ValueError('Existing internal validation signing identity required')
    sdk = producer.ordinary(config['sdkRoot'], directory=True)
    jdk = producer.ordinary(config['jdkRoot'], directory=True)
    tools = dict(java=jdk/'bin/java.exe', jvm=jdk/'bin/server/jvm.dll',
                 apksigner=sdk/'build-tools/35.0.0/lib/apksigner.jar',
                 zipalign=sdk/'build-tools/35.0.0/zipalign.exe')
    hashes = {}
    for name, path in tools.items():
        producer.ordinary(path); producer.command_path(path)
        expected = declared['toolSha256'][name] if name == 'zipalign' else config['toolSha256'][name]
        hashes[name] = digest(path)
        if hashes[name] != producer.hash_value(expected):
            raise ValueError('Declared signing or alignment tool changed')
    out = producer.new_output(output)
    for root in [Path(inputs['host']['workspaceRoot']), frontend, native.parent, base.parent, sdk, jdk, HERE]:
        if out.is_relative_to(root):
            raise ValueError('Package output must be outside immutable input roots')
    out.mkdir(exist_ok=False)
    for name in ['temp', 'java-user']:
        (out/name).mkdir()
    (out/'config.json').write_bytes(raw_config)
    env = dict(os.environ, TEMP=str(out/'temp'), TMP=str(out/'temp'), TMPDIR=str(out/'temp'),
               JAVA_OPTS='', _JAVA_OPTIONS='', JDK_JAVA_OPTIONS='',
               JAVA_TOOL_OPTIONS='-Duser.home="'+(out/'java-user').as_posix()+'" -Djava.io.tmpdir="'+(out/'temp').as_posix()+'"')
    sources = source_hashes()
    steps = []

    def run(label, command):
        if {name: digest(path) for name, path in tools.items()} != hashes:
            raise ValueError('Signing or alignment tool changed')
        with (out/(label+'.stdout')).open('xb') as stdout, (out/(label+'.stderr')).open('xb') as stderr:
            child = subprocess.run([str(value) for value in command], cwd=out, env=env, stdin=subprocess.DEVNULL,
                                   stdout=stdout, stderr=stderr, timeout=120,
                                   creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        steps.append(dict(step=label, exitCode=child.returncode,
                          stdoutSha256=digest(out/(label+'.stdout')), stderrSha256=digest(out/(label+'.stderr'))))
        if child.returncode != 0:
            raise ValueError('Diagnostic package command failed: '+label)
        return (out/(label+'.stdout')).read_text(encoding='utf-8')

    result = dict(passed=False, defaultFeaturesOnly=False, productInstalled=False, nativeRuntimeAccepted=False,
                  actualIpcAccepted=False, fullRF312Accepted=False, productionSigningUsed=False)
    try:
        sign = [tools['java'], '-jar', tools['apksigner']]
        check_certificate(run('baseline-verify', sign+['verify', '--verbose', '--print-certs', base]), signer['certificateSha256'])
        unsigned = out/'diagnostic-unsigned.apk'
        _, expected = unsigned_copy(base, native.read_bytes(), inputs['release']['library']['sha256'], unsigned)
        aligned = out/'diagnostic-aligned.apk'
        run('align', [tools['zipalign'], '-P', '16', '4', unsigned, aligned])
        run('alignment-check', [tools['zipalign'], '-c', '-P', '16', '4', aligned])
        apk = out/'app-x86_64-release-ipc-diagnostic.apk'
        run('sign', sign+['sign', '--ks', key, '--ks-key-alias', 'androiddebugkey', '--ks-pass', 'pass:android',
            '--key-pass', 'pass:android', '--v1-signing-enabled', 'false', '--v2-signing-enabled', 'true',
            '--v3-signing-enabled', 'true', '--v4-signing-enabled', 'false', '--out', apk, aligned])
        check_certificate(run('verify', sign+['verify', '--verbose', '--print-certs', apk]), signer['certificateSha256'])
        run('signed-alignment-check', [tools['zipalign'], '-c', '-P', '16', '4', apk])
        actual = verify_payload(base, apk, expected)
        if (digest(base) != base_hash or digest(native) != library['sha256'] or digest(key) != signer['sha256']
                or config_file.read_bytes() != raw_config or producer.load_inputs(inputs_file) != inputs
                or source_hashes() != sources):
            raise ValueError('Frozen baseline, diagnostic, signing key or producer changed')
        for name in ['runtimeInputs', 'diagnosticLibraryReceipt', 'defaultPackageConfig']:
            producer.bound_json(declared[name], declared[name+'Sha256'])
        result.update(passed=True, actualPackageBuilt=True, apk=str(apk), sha256=digest(apk), bytes=apk.stat().st_size,
                      baselineApkSha256=base_hash, diagnosticLibrarySha256=library['sha256'], sourceHead=library['sourceHead'],
                      libraryReceiptSha256=declared['diagnosticLibraryReceiptSha256'],
                      onlyChangedPayload=CORE, allOtherPayloadsByteIdentical=True, unchangedEntryCount=len(actual)-1,
                      payloadSha256=actual, sourceSha256=sources, toolSha256=hashes,
                      certificateSha256=signer['certificateSha256'], signedAlignmentVerified=True)
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
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if os.name != 'nt':
        raise ValueError('This explicit package signer requires Windows')
    result = package(args.config, args.output)
    print(json.dumps({key: value for key, value in result.items() if key in
                     ['passed', 'error', 'errorType', 'nativeRuntimeAccepted', 'actualIpcAccepted']}))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
