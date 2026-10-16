#!/usr/bin/env python3
"""Build the independent public-fixture observer; never installs or starts an app."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import zipfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sdk', type=Path, required=True)
    parser.add_argument('--jdk', type=Path, required=True)
    parser.add_argument('--build-tools', default='35.0.0')
    parser.add_argument('--compile-sdk', type=int, default=36)
    parser.add_argument('--output', type=Path, required=True, help='New local output directory')
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9]+(?:\.[0-9]+){1,3}', args.build_tools) or args.compile_sdk < 36:
        parser.error('Expected an installed build-tools version and compile SDK >= 36')
    sdk, jdk = args.sdk.resolve(strict=True), args.jdk.resolve(strict=True)
    source = Path(__file__).resolve().parent
    tools = sdk / 'build-tools' / args.build_tools
    android = sdk / 'platforms' / ('android-' + str(args.compile_sdk)) / 'android.jar'
    suffix = '.exe' if os.name == 'nt' else ''
    executable = lambda root, name: root / (name + suffix)
    required = [android, tools / 'lib/d8.jar', tools / 'lib/apksigner.jar',
                executable(tools, 'aapt2'), executable(tools, 'zipalign')]
    required += [executable(jdk / 'bin', name) for name in ('java', 'javac', 'keytool')]
    if not all(p.is_file() for p in required):
        parser.error('Required local JDK/Android SDK tools are missing')
    output = args.output.absolute()
    if any(character in output.as_posix() for character in ('"', '\r', '\n')):
        parser.error('Output path cannot contain quote or newline characters')
    if output.exists() or output.is_symlink():
        parser.error('Output must be a new directory')
    output.mkdir(parents=True, exist_ok=False)
    output = output.resolve(strict=True)
    sources = {name: digest(source / name) for name in ('Observer.java', 'AndroidManifest.xml')}
    env = dict(os.environ, TEMP=str(output), TMP=str(output),
               JAVA_TOOL_OPTIONS='-Duser.home="' + output.as_posix() + '" -Djava.io.tmpdir="' + output.as_posix() + '"')
    steps = []

    def run(name, command):
        if any(digest(source / n) != value for n, value in sources.items()):
            raise RuntimeError('Observer sources changed during compilation')
        result = subprocess.run([str(item) for item in command], cwd=output, env=env,
                                stdin=subprocess.DEVNULL, capture_output=True, timeout=120)
        (output / (name + '.stdout.log')).write_bytes(result.stdout)
        (output / (name + '.stderr.log')).write_bytes(result.stderr)
        steps.append({'step': name, 'exitCode': result.returncode})
        (output / 'steps.json').write_text(json.dumps(steps, indent=2), encoding='utf-8')
        result.check_returncode()
        if any(digest(source / n) != value for n, value in sources.items()):
            raise RuntimeError('Observer sources changed during compilation')
        return result.stdout.decode('utf-8')

    classes, dex = output / 'classes', output / 'dex'
    classes.mkdir()
    dex.mkdir()
    run('javac', [executable(jdk / 'bin', 'javac'), '--release', '11', '-cp', android,
                  '-d', classes, source / 'Observer.java'])
    jar = output / 'observer-classes.jar'
    with zipfile.ZipFile(jar, 'x', zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(classes.rglob('*.class')):
            archive.write(path, path.relative_to(classes).as_posix())
    java = executable(jdk / 'bin', 'java')
    run('d8', [java, '-cp', tools / 'lib/d8.jar', 'com.android.tools.r8.D8', '--lib', android,
               '--min-api', '28', '--output', dex, jar])
    unsigned = output / 'observer-unsigned.apk'
    run('aapt2', [executable(tools, 'aapt2'), 'link', '-o', unsigned, '--manifest',
                  source / 'AndroidManifest.xml', '-I', android, '--min-sdk-version', '28',
                  '--target-sdk-version', '35', '--version-code', '1', '--version-name', '0.0.1'])
    with zipfile.ZipFile(unsigned, 'a', zipfile.ZIP_DEFLATED) as archive:
        archive.write(dex / 'classes.dex', 'classes.dex')
    aligned = output / 'observer-aligned.apk'
    run('zipalign', [executable(tools, 'zipalign'), '-f', '4', unsigned, aligned])
    key = output / 'observer-debug.jks'
    run('test-key', [executable(jdk / 'bin', 'keytool'), '-genkeypair', '-noprompt', '-alias',
                     'androiddebugkey', '-keyalg', 'RSA', '-keysize', '2048', '-validity', '3650',
                     '-dname', 'CN=SoloSoul Public Fixture Observer,OU=Test', '-keystore', key,
                     '-storepass', 'android', '-keypass', 'android'])
    apk = output / 'external-journey-observer.apk'
    signer = [java, '-jar', tools / 'lib/apksigner.jar']
    run('sign', signer + ['sign', '--ks', key, '--ks-key-alias', 'androiddebugkey', '--ks-pass',
                         'pass:android', '--key-pass', 'pass:android', '--v4-signing-enabled',
                         'false', '--out', apk, aligned])
    certificate = run('verify', signer + ['verify', '--verbose', '--print-certs', apk])
    certificates = re.findall(r'certificate SHA-256 digest: (\S+)', certificate)
    if len(certificates) != 1:
        raise RuntimeError('One internal test signer required')
    manifest = run('manifest', [executable(tools, 'aapt2'), 'dump', 'xmltree', apk,
                                '--file', 'AndroidManifest.xml'])
    if ('android:targetPackage' not in manifest or '"org.solosoul.rf312observer"' not in manifest
            or '"com.solosoul.app"' in manifest):
        raise RuntimeError('Observer must instrument only its own package')
    with zipfile.ZipFile(aligned) as before, zipfile.ZipFile(apk) as after:
        if any(before.read(name) != after.read(name) for name in before.namelist()):
            raise RuntimeError('APK payload changed during signing')
    receipt = dict(passed=True, sourceSha256=sources, apk=str(apk), apkSha256=digest(apk),
                   dexSha256=digest(dex / 'classes.dex'), certificateSha256=certificates[0],
                   compileSdk=args.compile_sdk, minSdk=28, targetSdk=35,
                   targetInstrumentationInjected=False, productInstalled=False,
                   nativeRuntimeAccepted=False, steps=steps)
    (output / 'package.json').write_text(json.dumps(receipt, indent=2), encoding='utf-8')
    print(json.dumps({'passed': True, 'productInstalled': False, 'nativeRuntimeAccepted': False}))


if __name__ == '__main__':
    main()
