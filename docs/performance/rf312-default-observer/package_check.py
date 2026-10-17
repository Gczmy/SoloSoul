"""Check supplied default APKs with explicit local tools and preserved reference bytes."""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import zipfile

from runtime_inputs import digest, hash_value


def members(archive, prefix):
    names = archive.namelist()
    if len(names) != len(set(names)):raise ValueError('Duplicate APK entries')
    return {name for name in names if name.startswith(prefix) and not name.endswith('/')}


def assets(archive, expected):
    for name, value in expected.items():
        path = PurePosixPath(name)
        if not name or path.is_absolute() or '..' in path.parts or '\\' in name or ':' in name:
            raise ValueError('Relative asset name required')
        if hashlib.sha256(archive.read('assets/'+name)).hexdigest() != hash_value(value):
            raise ValueError('Original asset bytes changed')


def resource_manifest(release, debug):
    name = 'assets/bundled-resource-manifest.json'
    raw = release.read(name)
    if debug.read(name) != raw:raise ValueError('Restore APK resource manifest differs')
    manifest = json.loads(raw)
    if set(manifest) != {'schema','version','files'} or type(manifest['schema']) is not int or manifest['schema'] != 1:
        raise ValueError('Original bundled resource manifest required')
    hash_value(manifest['version'])
    files = {}
    for row in manifest['files']:
        if set(row) != {'path','size','sha256'} or type(row['size']) is not int or row['size'] < 0:
            raise ValueError('Complete bundled resource entry required')
        if row['path'] in files:raise ValueError('Duplicate bundled resource')
        assets(release,{row['path']:row['sha256']});assets(debug,{row['path']:row['sha256']})
        if len(release.read('assets/'+row['path'])) != row['size']:raise ValueError('Bundled resource size differs')
        files[row['path']] = row['sha256']
    if not files:raise ValueError('Nonempty bundled resources required')
    return manifest


def verify_bytes(inputs, reference):
    if set(reference) != {'previousRelease','appAssets','generatedProfiles'}:
        raise ValueError('Complete previous default APK and asset reference required')
    previous = reference['previousRelease']
    if set(previous) != {'path','sha256','bytes'} or digest(previous['path']) != hash_value(previous['sha256']):
        raise ValueError('Previous verified default APK changed')
    if Path(previous['path']).stat().st_size != previous['bytes']:raise ValueError('Previous default APK size differs')
    current = inputs['release']
    source, profiles = reference['appAssets'], reference['generatedProfiles']
    if (len(source) != current['assetFilesVerified'] or len(profiles) != current['generatedProfileFilesVerified']
            or set(source) & set(profiles)):
        raise ValueError('Complete disjoint original asset groups required')
    with zipfile.ZipFile(current['apk']['path']) as a, zipfile.ZipFile(previous['path']) as b, \
            zipfile.ZipFile(inputs['originalDebug']['path']) as debug:
        actual, old = members(a,'assets/'), members(b,'assets/')
        if actual != old or len(actual) != current['totalAssetFilesVerified']:raise ValueError('Unknown or missing APK asset')
        assets(a, source);assets(a, profiles)
        dependency = actual - {'assets/'+n for n in source} - {'assets/'+n for n in profiles}
        if len(dependency) != current['dependencyAssetFilesVerified'] or any(a.read(n) != b.read(n) for n in dependency):
            raise ValueError('Dependency asset set or bytes changed')
        libraries, old_libraries = members(a,'lib/'), members(b,'lib/')
        if (not libraries or libraries != old_libraries or any(not n.startswith('lib/x86_64/') for n in libraries)
                or any(a.read(n) != b.read(n) for n in libraries)):
            raise ValueError('Original default native library set or bytes changed')
        core = a.read('lib/x86_64/libsolo_soul.so')
        if hashlib.sha256(core).hexdigest() != inputs['release']['library']['sha256']:
            raise ValueError('Original default native core changed')
        if b'android-native-ipc-transport-observer' in core or b'__SOLOSOUL_ANDROID_NATIVE_PERF__' in core:
            raise ValueError('Optional instrumentation cannot be measured as default')
        return resource_manifest(a,debug)


def verify(inputs, reference, tools, output):
    output = Path(output);output.mkdir(exist_ok=False)
    manifest = verify_bytes(inputs,reference)
    frozen = {name:digest(Path(value)) for name,value in tools.items()}
    environment = dict(os.environ,TEMP=str(output),TMP=str(output),JAVA_OPTS='',_JAVA_OPTIONS='',JDK_JAVA_OPTIONS='',
        JAVA_TOOL_OPTIONS=f'-Djava.io.tmpdir="{output}"')

    def call(name, command):
        if any(digest(Path(tools[n])) != value for n,value in frozen.items()):raise ValueError('Package tools changed')
        result = subprocess.run([str(v) for v in command],env=environment,stdin=subprocess.DEVNULL,
            capture_output=True,timeout=120,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        (output/(name+'.stdout')).write_bytes(result.stdout);(output/(name+'.stderr')).write_bytes(result.stderr)
        result.check_returncode()
        if any(digest(Path(tools[n])) != value for n,value in frozen.items()):raise ValueError('Package tools changed')
        return result.stdout.decode('utf-8')

    text = call('release-badging',[tools['aapt2'],'dump','badging',inputs['release']['apk']['path']])
    if not all(value in text for value in ("package: name='com.solosoul.app'", "native-code: 'x86_64'",
            "minSdkVersion:'28'", "targetSdkVersion:'36'", "launchable-activity: name='com.solosoul.app.MainActivity'")):
        raise ValueError('Default Release package identity differs')
    if 'application-debuggable' in text:raise ValueError('Non-debug default APK required')
    rows = [('release',dict(inputs['release']['apk'],certificateSha256=inputs['release']['certificateSha256']))]
    rows += [(n,inputs[n]) for n in ('originalDebug','originalTest','runtimeTest')]
    rows += [('observer',dict(path=inputs['observer']['apk'],certificateSha256=inputs['observer']['certificateSha256']))]
    for role,row in rows:
        text = call(role+'-signature',[tools['java'],'-jar',tools['apksigner'],'verify','--verbose','--print-certs',row['path']])
        if re.findall(r'certificate SHA-256 digest: (\S+)',text) != [row['certificateSha256']]:
            raise ValueError('Declared internal test signer differs')
    (output/'bundled-resource-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    result = dict(passed=True,allAssetsVerified=inputs['release']['totalAssetFilesVerified'],
        appAssetsVerified=len(reference['appAssets']),dependencyAssetsVerified=inputs['release']['dependencyAssetFilesVerified'],
        generatedProfilesVerified=len(reference['generatedProfiles']),allNativeLibrariesByteIdentical=True,
        defaultRustLibraryByteIdentical=True,nonDebuggableDefaultRelease=True,certificateMatched=True,verifiedSigners=len(rows),
        originalDebugResourceManifestAndFilesMatched=True,bundledResourceFiles=len(manifest['files']),
        resourceManifestSha256=digest(output/'bundled-resource-manifest.json'),toolSha256=frozen,
        fullDefaultJourneyAccepted=False,apkSha256=inputs['release']['apk']['sha256'])
    (output/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    return result
