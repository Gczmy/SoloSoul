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
import xml.etree.ElementTree as ET
import zipfile


PACKAGE = 'org.solosoul.rf312observer'
ANDROID = 'http://schemas.android.com/apk/res/android'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest_identity(package, instrumentation):
    if (package != PACKAGE or len(instrumentation) != 1
            or instrumentation[0] != dict(name='.Observer', targetPackage=PACKAGE)):
        raise ValueError('Observer must instrument only its own package with .Observer')
    return dict(package=package, instrumentationName=PACKAGE + '.Observer',
                instrumentationTarget=instrumentation[0]['targetPackage'])


def source_manifest_identity(raw):
    root = ET.fromstring(raw)
    if root.tag != 'manifest':
        raise ValueError('One observer manifest root required')
    instruments = list(root.iter('instrumentation'))
    if any(node not in list(root) for node in instruments):
        raise ValueError('Instrumentation must be a direct manifest child')
    return manifest_identity(root.get('package'), [
        {name: node.get('{' + ANDROID + '}' + name) for name in ('name', 'targetPackage')}
        for node in instruments])


def compiled_manifest_identity(text):
    # aapt2 xmltree is indented: bind each attribute to its actual element.
    # A package string in a label or the root cannot prove the targetPackage.
    stack, roots, instruments = [], [], []
    for line in text.splitlines():
        element = re.fullmatch(r'( *)E: ([A-Za-z][A-Za-z0-9_.-]*) \(line=[0-9]+\)', line)
        if element:
            depth, tag = len(element[1]), element[2]
            while stack and stack[-1]['depth'] >= depth:
                stack.pop()
            node = dict(depth=depth, tag=tag, attributes={})
            if not stack:
                roots.append(node)
            if tag == 'instrumentation':
                if len(stack) != 1 or stack[0]['tag'] != 'manifest':
                    raise ValueError('Compiled instrumentation must be a direct manifest child')
                instruments.append(node)
            stack.append(node)
            continue
        attribute = re.fullmatch(r'( *)A: ([^=]+)=(.*)', line)
        if attribute:
            if not stack or len(attribute[1]) <= stack[-1]['depth']:
                raise ValueError('Compiled manifest attribute has no element')
            name = attribute[2]
            if name in stack[-1]['attributes']:
                raise ValueError('Duplicate compiled manifest attribute')
            stack[-1]['attributes'][name] = attribute[3]
            continue
        if line.strip() and not re.fullmatch(r' *N: \S+ \(line=[0-9]+\)', line):
            raise ValueError('Unknown compiled manifest tree entry')
    if len(roots) != 1 or roots[0]['tag'] != 'manifest':
        raise ValueError('One compiled observer manifest root required')

    def string(node, name):
        values = [value for key, value in node['attributes'].items()
                  if key == name or key.startswith(name + '(')]
        if len(values) != 1:
            raise ValueError('One exact compiled manifest attribute required')
        match = re.fullmatch(r'"([^"\\]*)"(?: \(Raw: "\1"\))?', values[0])
        if not match:
            raise ValueError('Unambiguous compiled manifest string required')
        return match[1]

    if '"com.solosoul.app"' in text:
        raise ValueError('Product package cannot be an observer target')
    return manifest_identity(string(roots[0], 'package'), [
        {name: string(node, ANDROID + ':' + name) for name in ('name', 'targetPackage')}
        for node in instruments])


def package_receipt(apk, dex, sources, identity, certificate, compile_sdk, steps):
    if identity != manifest_identity(PACKAGE, [dict(name='.Observer', targetPackage=PACKAGE)]):
        raise ValueError('Verified compiled observer identity required')
    apk_sha = digest(apk)
    return dict(passed=True, sourceSha256=sources, apk=str(apk), sha256=apk_sha,
                apkSha256=apk_sha, bytes=apk.stat().st_size, **identity,
                dexSha256=digest(dex), certificateSha256=certificate,
                compileSdk=compile_sdk, minSdk=28, targetSdk=35,
                targetInstrumentationInjected=False, productInstalled=False,
                nativeRuntimeAccepted=False, steps=steps)


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
    source_identity = source_manifest_identity((source / 'AndroidManifest.xml').read_bytes())
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
    identity = compiled_manifest_identity(manifest)
    if identity != source_identity:
        raise RuntimeError('Compiled observer identity differs from bound source')
    with zipfile.ZipFile(aligned) as before, zipfile.ZipFile(apk) as after:
        if any(before.read(name) != after.read(name) for name in before.namelist()):
            raise RuntimeError('APK payload changed during signing')
    receipt = package_receipt(apk, dex / 'classes.dex', sources, identity,
                              certificates[0], args.compile_sdk, steps)
    (output / 'package.json').write_text(json.dumps(receipt, indent=2), encoding='utf-8')
    print(json.dumps({'passed': True, 'productInstalled': False, 'nativeRuntimeAccepted': False}))


if __name__ == '__main__':
    main()
