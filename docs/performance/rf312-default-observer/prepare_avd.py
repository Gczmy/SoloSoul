"""Create a fresh API31 public-test AVD from explicit local SDK/JDK inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

from runtime_inputs import decode, digest, hash_value

S = Path(__file__).resolve().parent
PROFILE = S / 'api31-hardware.ini'
PROFILE_SHA256 = 'b6268289e372b91822d745867c06e4ee3c866e295be4202ae65dd3945131e448'
FIELDS = {'schemaVersion', 'sdkRoot', 'jdkRoot', 'avdName', 'toolSha256'}
TOOLS = {'avdManager', 'java', 'classpath', 'imageProperties', 'emulatorPackage', 'emulatorExecutable'}
PACKAGE = 'system-images;android-31;default;x86_64'


def ordinary(path, *, directory=False):
    path = Path(path).absolute()
    for item in (path, *path.parents):
        if item.is_symlink() or getattr(item.lstat(), 'st_file_attributes', 0) & 0x400:
            raise ValueError('Ordinary paths without reparse parents required')
    if not (path.is_dir() if directory else path.is_file()):
        raise ValueError('Existing ordinary input required')
    return path.resolve(strict=True)


def command_path(path):
    value = str(path)
    # cmd.exe runs the unmodified SDK batch entry. Refuse expansion/control
    # syntax rather than allowing a path to become another shell command.
    if any(character in value for character in '\x00\r\n"%!^&|<>'):
        raise ValueError('Shell expansion or control characters in path')
    try:
        if value.encode('mbcs').decode('mbcs') != value:raise ValueError('SDK code page loses path characters')
    except UnicodeError as error:
        raise ValueError('SDK code page cannot represent this path') from error
    return value


def inputs(config, output):
    if os.name != 'nt':raise ValueError('This AVD preparation entry requires Windows')
    if (not isinstance(config, dict) or set(config) != FIELDS
            or type(config['schemaVersion']) is not int or config['schemaVersion'] != 1
            or not isinstance(config['toolSha256'], dict) or set(config['toolSha256']) != TOOLS
            or not isinstance(config['avdName'], str)
            or not re.fullmatch(r'SoloSoul_[A-Za-z0-9_-]{1,80}', config['avdName'])):
        raise ValueError('Complete explicit SDK/JDK and dedicated AVD configuration required')
    sdk = ordinary(config['sdkRoot'], directory=True)
    jdk = ordinary(config['jdkRoot'], directory=True)
    paths = dict(avdManager=sdk/'cmdline-tools/latest/bin/avdmanager.bat',
        java=jdk/'bin/java.exe', classpath=sdk/'cmdline-tools/latest/lib/avdmanager-classpath.jar',
        imageProperties=sdk/'system-images/android-31/default/x86_64/source.properties',
        emulatorPackage=sdk/'emulator/package.xml', emulatorExecutable=sdk/'emulator/emulator.exe')
    for name, path in paths.items():
        ordinary(path)
        if digest(path) != hash_value(config['toolSha256'][name]):raise ValueError('Declared preparation tool changed')
    properties = paths['imageProperties'].read_text(encoding='utf-8').splitlines()
    for item in ('AndroidVersion.ApiLevel=31', 'SystemImage.Abi=x86_64', 'SystemImage.TagId=default'):
        if properties.count(item) != 1:raise ValueError('Original API31 default x86_64 system image required')
    if digest(ordinary(PROFILE)) != PROFILE_SHA256:raise ValueError('Frozen hardware profile changed')
    output = Path(output).absolute()
    if output.exists() or output.is_symlink():raise ValueError('New output directory required')
    parent = ordinary(output.parent, directory=True)
    output = parent/output.name
    command = ordinary(Path(os.environ['SystemRoot'])/'System32/cmd.exe')
    for path in (sdk, jdk, output, command):command_path(path)
    return sdk, jdk, paths, output, command


def build_command(cmd, manager, name, destination):
    for path in (cmd, manager, destination):command_path(path)
    if not re.fullmatch(r'SoloSoul_[A-Za-z0-9_-]{1,80}', name):raise ValueError('Dedicated AVD name required')
    # /s strips this one outer quote pair; every path remains quoted inside it.
    return (f'"{cmd}" /d /s /c ""{manager}" create avd --name {name} '
            f'--package "{PACKAGE}" --device pixel_2 --path "{destination}""')


def prepare(config_file, output, *, runner=None):
    raw = Path(config_file).read_bytes();config = decode(raw)
    sdk, jdk, tools, output, cmd = inputs(config, output)
    source_hash = digest(Path(__file__))
    name = config['avdName'];destination = output/'avd'/(name+'.avd')
    user, temp = output/'android-user', output/'temp'
    command = build_command(cmd, tools['avdManager'], name, destination)
    env = dict(os.environ, JAVA_HOME=str(jdk), ANDROID_HOME=str(sdk), ANDROID_SDK_ROOT=str(sdk),
        ANDROID_AVD_HOME=str(destination.parent), ANDROID_USER_HOME=str(user), ANDROID_EMULATOR_HOME=str(user),
        TEMP=str(temp), TMP=str(temp), JAVA_OPTS='', AVDMANAGER_OPTS='', _JAVA_OPTIONS='', JDK_JAVA_OPTIONS='',
        JAVA_TOOL_OPTIONS=f'-Duser.home="{user}" -Djava.io.tmpdir="{temp}"')
    env.pop('SKIP_JDK_VERSION_CHECK', None)
    output.mkdir(exist_ok=False);destination.parent.mkdir();user.mkdir();temp.mkdir()
    (output/'config.json').write_bytes(raw)
    (output/'command.json').write_text(json.dumps(dict(command=command, timeoutSeconds=120), indent=2)+'\n', encoding='utf-8')
    record = dict(passed=False, api=31, abi='x86_64', name=name, createdNewDedicatedAvd=False,
        oldUserdataCopied=False, oldAvdDirectoriesModified=False, emulatorStarted=False, adbCommandsExecuted=0,
        targetInputsDispatched=0, nativeRuntimeAccepted=False, fullPortableOuterRunnerAccepted=False,
        fullRF312Accepted=False, runtimeOverrideCpu=2, runtimeOverrideMemoryMiB=2048,
        hardwarePresetSha256=PROFILE_SHA256, preparationSourceSha256=source_hash, toolSha256=config['toolSha256'])
    started = time.monotonic()
    try:
        result = (runner or subprocess.run)(command, env=env, cwd=output, input=b'no\n', capture_output=True,
            timeout=120, creationflags=subprocess.CREATE_NO_WINDOW)
        (output/'avd-create.stdout.log').write_bytes(result.stdout)
        (output/'avd-create.stderr.log').write_bytes(result.stderr)
        record['exitCode'] = result.returncode
        result.check_returncode()
        generated = ordinary(destination/'config.ini')
        original = generated.read_bytes()
        (output/'avdmanager-original-config.ini').write_bytes(original)
        if (Path(config_file).read_bytes() != raw or digest(Path(__file__)) != source_hash
                or digest(PROFILE) != PROFILE_SHA256
                or any(digest(path) != config['toolSha256'][key] for key, path in tools.items())):
            raise ValueError('Preparation input changed while the SDK was running')
        generated.write_bytes(PROFILE.read_bytes())
        if digest(generated) != PROFILE_SHA256:raise ValueError('Applied hardware profile differs')
        record.update(passed=True, createdNewDedicatedAvd=True, freshSdkUserdata=True,
            originalGeneratedConfigSha256=hashlib.sha256(original).hexdigest(),
            configSha256=digest(generated), avdDirectory=str(destination),
            systemImageSourcePropertiesSha256=digest(tools['imageProperties']))
    except Exception as error:
        if isinstance(error, subprocess.TimeoutExpired):
            (output/'avd-create.stdout.log').write_bytes(error.stdout or b'')
            (output/'avd-create.stderr.log').write_bytes(error.stderr or b'')
        record.update(errorType=type(error).__name__, error=str(error))
    record['elapsedSeconds'] = time.monotonic()-started
    (output/'avd-preparation.json').write_text(json.dumps(record, indent=2)+'\n', encoding='utf-8')
    return record


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True, help='New directory with an existing ordinary parent')
    args = parser.parse_args(argv)
    try:record = prepare(args.config, args.output)
    except (OSError, ValueError, KeyError, TypeError):
        print('AVD preparation did not complete; existing evidence is preserved', file=sys.stderr)
        return 2
    print(json.dumps({key: record[key] for key in ('passed','emulatorStarted','adbCommandsExecuted','nativeRuntimeAccepted')}))
    return 0 if record['passed'] else 1


if __name__ == '__main__':sys.exit(main())
