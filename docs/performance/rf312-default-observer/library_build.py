"""Build a default Android Release library from a verified materialized frontend."""
import argparse
import hashlib
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tarfile
import time
import tomllib

import build_payload
from frontend_build import entry, files, new_output, relative, validate_files
from prepare_avd import ordinary, command_path
from runtime_inputs import decode, digest, hash_value

S=Path(__file__).resolve().parent
FIELDS={'schemaVersion','frontendResult','frontendResultSha256','sdkRoot','ndkVersion',
        'toolchainRoot','rustupHome','cargoHome','toolSha256','targetStdFiles'}
TOOLS={'cargo','rustc','rustcDriver','linker','clang','ar','ndkProperties'}
LINK_FLAGS='-Clink-arg=-landroid -Clink-arg=-llog -Clink-arg=-lOpenSLES'
TARGET='x86_64-linux-android'


def sources():
    return {n:digest(S/n) for n in ('library_build.py','build_payload.py','frontend_build.py',
        'dependency_lock.py','runtime_inputs.py','prepare_avd.py')}


def cargo_configs(workspace,cargo_home):
    paths=[]
    for root in (workspace,*workspace.parents):
        paths.extend(root/'.cargo'/n for n in ('config','config.toml'))
    paths.extend(cargo_home/n for n in ('config','config.toml'))
    return {str(p):entry(p) for p in paths if p.exists() or p.is_symlink()}


def registry_source(workspace,cargo_home,name):
    packages=tomllib.loads((workspace/'Cargo.lock').read_text(encoding='utf-8'))['package']
    matches=[p for p in packages if p['name']==name]
    if (len(matches)!=1 or matches[0].get('source')!='registry+https://github.com/rust-lang/crates.io-index'):
        raise ValueError('One locked registry Android codegen dependency required')
    package=matches[0];prefix=name+'-'+package['version']
    roots=list((cargo_home/'registry/src').glob('*/'+prefix))
    archives=list((cargo_home/'registry/cache').glob('*/'+prefix+'.crate'))
    if len(roots)!=1 or len(archives)!=1:raise ValueError('One cached archive and local source for locked dependency required')
    root=ordinary(roots[0],directory=True);archive=ordinary(archives[0])
    # Verify the actual .crate bytes against Cargo.lock, rather than requiring
    # optional per-file metadata which newer Cargo caches may not provide.
    if digest(archive)!=hash_value(package['checksum']):raise ValueError('Registry archive checksum differs from Cargo.lock')
    archive_row=entry(archive);rows={}
    with tarfile.open(archive,'r:gz') as reader:
        for member in reader.getmembers():
            if member.isdir():continue
            if not member.isfile() or not member.name.startswith(prefix+'/'):
                raise ValueError('Regular in-package registry member required')
            name=member.name[len(prefix)+1:];relative(name)
            if name in rows:raise ValueError('Duplicate registry archive member')
            with reader.extractfile(member) as stream:rows[name]=hashlib.file_digest(stream,'sha256').hexdigest()
    if not rows or entry(archive)!=archive_row:raise ValueError('Complete unchanged registry archive required')
    return root,rows,archive,archive_row


def official_codegen(workspace,cargo_home):
    bound={}
    def crate(name):
        root,rows,archive,archive_row=registry_source(workspace,cargo_home,name)
        bound[str(archive)]=archive_row
        return root,rows
    def source(root,rows,name):
        path=ordinary(root/name);row=entry(path)
        if row['sha256']!=rows[name]:raise ValueError('Official Android generator source checksum differs')
        bound[str(path)]=row
        return path.read_bytes().decode('utf-8')
    expected={};roots={}
    for name,folder in (('tauri','mobile/android-codegen'),('wry','src/android/kotlin')):
        root,checksums=crate(name);source(root,checksums,'build.rs')
        for path in sorted((root/folder).iterdir()):
            text=source(root,checksums,path.relative_to(root).as_posix())
            text=text.replace('{{package}}','com.solosoul.app').replace('{{library}}','solo_soul')
            if name=='wry':
                text=text.replace('{{package-unescaped}}','com.solosoul.app').replace('{{class-extension}}','').replace('{{class-init}}','')
                text=('# THIS FILE IS AUTO-GENERATED. DO NOT MODIFY!!\n\n' if path.suffix=='.pro'
                      else '/* THIS FILE IS AUTO-GENERATED. DO NOT MODIFY!! */\n\n')+text
            if path.name in expected:raise ValueError('Duplicate official Android generated name')
            data=text.encode('utf-8');expected[path.name]=dict(bytes=len(data),sha256=hashlib.sha256(data).hexdigest())
    root,checksums=crate('tauri');proguard=source(root,checksums,'mobile/proguard-tauri.pro').replace('$PACKAGE','com.solosoul.app').encode('utf-8')
    roots['tauri-android']=str(ordinary(root/'mobile/android',directory=True))
    for name in ('tauri-plugin-biometric','tauri-plugin-dialog','tauri-plugin-fs','tauri-plugin-notification','tauri-plugin-shell'):
        root,_=crate(name);roots[name]=str(ordinary(root/'android',directory=True))
    root,checksums=crate('tauri-build')
    for name in ('src/lib.rs','src/mobile.rs'):source(root,checksums,name)
    return dict(sourceFiles=bound,bridge=expected,proguard=dict(bytes=len(proguard),sha256=hashlib.sha256(proguard).hexdigest()),
                androidLibraryRoots=roots)


def verify_gradle_metadata(generated,expected):
    rows={};included=[]
    for line in (generated/'tauri.settings.gradle').read_text(encoding='utf-8').splitlines():
        if not line or line=='// THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.':continue
        include=re.fullmatch(r"include ':([^']+)'",line)
        project=re.fullmatch(r"project\(':([^']+)'\)\.projectDir = new File\((\".*\")\)",line)
        if include:included.append(include[1])
        elif project:
            if project[1] in rows:raise ValueError('Duplicate generated Gradle library')
            rows[project[1]]=str(ordinary(json.loads(project[2]),directory=True))
        else:raise ValueError('Unexpected generated Gradle settings statement')
    if rows!=expected or set(included)!=set(expected) or len(included)!=len(expected):
        raise ValueError('Generated Gradle dependencies differ from locked Android libraries')
    text=(generated/'app/tauri.build.gradle.kts').read_text(encoding='utf-8')
    names=re.findall(r'  implementation\(project\(":([^\"]+)"\)\)',text)
    remaining=re.sub(r'\n  implementation\(project\(":([^\"]+)"\)\)','',text)
    fixed='// THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\nval implementation by configurations\ndependencies {\n  implementation("androidx.lifecycle:lifecycle-process:2.10.0")\n}'
    if set(names)!=set(expected) or len(names)!=len(expected) or remaining!=fixed:
        raise ValueError('Generated Gradle implementation set or configuration differs')


def configure(config,output):
    output=new_output(output)
    if (not isinstance(config,dict) or set(config)!=FIELDS or type(config['schemaVersion']) is not int
            or config['schemaVersion']!=1 or not isinstance(config['toolSha256'],dict)
            or set(config['toolSha256'])!=TOOLS or not isinstance(config['ndkVersion'],str)
            or not re.fullmatch(r'[0-9]+(?:\.[0-9]+){2,3}',config['ndkVersion'])):
        raise ValueError('Complete explicit default-library configuration required')
    payload=build_payload.load(config['frontendResult'],config['frontendResultSha256'])
    workspace=Path(payload['workspace'])/'tauri'
    for root in (Path(payload['workspace']),Path(payload['frontend'])):
        if output.is_relative_to(root):raise ValueError('Library output cannot be inside bound frontend inputs')
    sdk=ordinary(config['sdkRoot'],directory=True);toolchain=ordinary(config['toolchainRoot'],directory=True)
    rustup=ordinary(config['rustupHome'],directory=True);cargo_home=ordinary(config['cargoHome'],directory=True)
    if toolchain!=(rustup/'toolchains'/toolchain.name).resolve():raise ValueError('Toolchain must belong to the declared rustup home')
    driver=list((toolchain/'bin').glob('rustc_driver*.dll'))
    if len(driver)!=1:raise ValueError('One explicit Rust compiler driver required')
    ndk=sdk/'ndk'/config['ndkVersion'];native=ndk/'toolchains/llvm/prebuilt/windows-x86_64/bin'
    tools=dict(cargo=toolchain/'bin/cargo.exe',rustc=toolchain/'bin/rustc.exe',rustcDriver=driver[0],
        linker=native/'x86_64-linux-android28-clang.cmd',clang=native/'clang.exe',ar=native/'llvm-ar.exe',
        ndkProperties=ndk/'source.properties')
    for name,path in tools.items():
        ordinary(path);command_path(path)
        if digest(path)!=hash_value(config['toolSha256'][name]):raise ValueError('Declared compiler tool changed')
    if ('Pkg.Revision = '+config['ndkVersion']) not in tools['ndkProperties'].read_text().splitlines():
        raise ValueError('Declared NDK revision differs')
    standard=toolchain/'lib/rustlib'/TARGET/'lib'
    validate_files(config['targetStdFiles'])
    if files(standard)!=config['targetStdFiles']:raise ValueError('Complete Android Rust standard library differs')
    manifest=tomllib.loads((workspace/'src-tauri/Cargo.toml').read_text(encoding='utf-8'))
    if manifest['features']['default']!=['custom-protocol'] or manifest['features']['custom-protocol']!=['tauri/custom-protocol']:
        raise ValueError('Original default custom-protocol feature definition required')
    for path in (output,workspace,sdk,toolchain,rustup,cargo_home):command_path(path)
    bridge=workspace/'src-tauri/gen/android/app/src/main/java/com/solosoul/app/generated'
    if bridge.exists() or bridge.is_symlink():raise ValueError('Fresh official Android codegen output required')
    return dict(payload=payload,workspace=workspace,sdk=sdk,ndk=ndk,toolchain=toolchain,
        rustup=rustup,cargoHome=cargo_home,tools=tools,standard=standard,output=output,
        cargoConfigs=cargo_configs(workspace,cargo_home),sourceSha256=sources(),codegen=official_codegen(workspace,cargo_home))


def command_and_environment(prepared,environment=None):
    p=prepared;out=p['output'];workspace=p['workspace']
    frontend=Path(p['payload']['frontend'])
    relative=Path(os.path.relpath(frontend,workspace/'src-tauri')).as_posix()
    override=dict(build=dict(frontendDist=relative))
    if (workspace/'src-tauri'/relative).resolve()!=frontend:raise ValueError('Exact relative frontend override required')
    original=dict(os.environ if environment is None else environment)
    prefixes=('WRY_','TAURI_','SOLOSOUL_','CARGO_PROFILE_','CARGO_TARGET_','CARGO_BUILD_',
              'CARGO_FEATURE_','CFLAGS_','CXXFLAGS_','LDFLAGS_')
    removed={'RUSTFLAGS','RUSTDOCFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','RUSTC_BOOTSTRAP',
             'CARGO_ENCODED_RUSTFLAGS','PDFIUM_LIBRARY_PATH','CFLAGS','CXXFLAGS','LDFLAGS'}
    env={k:v for k,v in original.items() if not k.startswith(prefixes) and k not in removed}
    bridge=workspace/'src-tauri/gen/android/app/src/main/java/com/solosoul/app/generated'
    env.update(TEMP=str(out/'temp'),TMP=str(out/'temp'),TMPDIR=str(out/'temp'),
        CARGO_HOME=str(p['cargoHome']),CARGO_TARGET_DIR=str(out/'cargo-target'),CARGO_BUILD_JOBS='1',
        RUSTUP_HOME=str(p['rustup']),RUSTUP_TOOLCHAIN=p['toolchain'].name,RUSTC=str(p['tools']['rustc']),
        CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=str(p['tools']['linker']),
        CC_x86_64_linux_android=str(p['tools']['linker']),AR_x86_64_linux_android=str(p['tools']['ar']),
        ANDROID_HOME=str(p['sdk']),ANDROID_SDK_ROOT=str(p['sdk']),NDK_HOME=str(p['ndk']),
        ANDROID_NDK_HOME=str(p['ndk']),ORT_CACHE_DIR=str(out/'ort-cache'),RUSTFLAGS=LINK_FLAGS,
        TAURI_CONFIG=json.dumps(override,separators=(',',':')),
        TAURI_ANDROID_PROJECT_PATH=str(workspace/'src-tauri/gen/android'),TAURI_ANDROID_PACKAGE_UNESCAPED='com.solosoul.app',
        WRY_ANDROID_KOTLIN_FILES_OUT_DIR=str(bridge),WRY_ANDROID_PACKAGE='com.solosoul.app',WRY_ANDROID_LIBRARY='solo_soul',
        PYTHONDONTWRITEBYTECODE='1',PATH=str(p['toolchain']/'bin')+os.pathsep+original.get('PATH',''))
    command=[str(p['tools']['cargo']),'build','--locked','--offline','-p','solo_soul','--lib','--release',
             '--target',TARGET,'--message-format=json-render-diagnostics']
    return command,env,override,bridge


def compiler_artifact(raw,workspace,target):
    rows=[decode(line) for line in raw.splitlines() if line.strip()]
    if not rows or rows[-1]!=dict(reason='build-finished',success=True):raise ValueError('Successful Cargo build-finished receipt required')
    matches=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')=='solo_soul'
             and Path(r['target']['src_path']).resolve()==(workspace/'src-tauri/src/lib.rs').resolve()]
    if len(matches)!=1:raise ValueError('One actual product compiler artifact required')
    row=matches[0];profile=row['profile']
    if (set(row['features'])!={'default','custom-protocol'} or row.get('fresh') is not False
            or profile['opt_level']!='3' or profile['debug_assertions'] is not False or profile['test'] is not False
            or 'cdylib' not in row['target']['kind']):
        raise ValueError('Actual fresh default-only Release artifact required')
    path=target/TARGET/'release/libsolo_soul.so'
    if [Path(n).resolve() for n in row['filenames'] if Path(n).name=='libsolo_soul.so']!=[path.resolve()]:
        raise ValueError('Exact new target library required')
    return path,row


class Entrypoints(HTMLParser):
    def __init__(self):super().__init__();self.paths=[]
    def handle_starttag(self,tag,attrs):
        values=dict(attrs)
        value=values.get('src') if tag=='script' else values.get('href') if tag=='link' and values.get('rel') in ('modulepreload','stylesheet') else None
        if value and value.startswith('/assets/'):
            if value in self.paths:raise ValueError('Duplicate frontend entrypoint')
            self.paths.append(value)


def verify_library(path,frontend):
    raw=ordinary(path).read_bytes()
    if raw[:6]!=b'\x7fELF\x02\x01' or len(raw)<64 or struct.unpack_from('<HH',raw,16)!=(3,62):
        raise ValueError('x86_64 little-endian shared ELF required')
    if any(n in raw for n in (b'android-native-ipc-transport-observer',b'__SOLOSOUL_ANDROID_NATIVE_PERF__')):
        raise ValueError('Optional native observer marker in default library')
    parser=Entrypoints();parser.feed((frontend/'index.html').read_text(encoding='utf-8'))
    if not parser.paths:raise ValueError('Actual production frontend entrypoints required')
    for name in parser.paths:
        if not (frontend/name.removeprefix('/')).is_file() or name.encode() not in raw:
            raise ValueError('New frontend entrypoint missing from actual library')
    return dict(**entry(path),entrypoints=parser.paths,earlyObserverMarkersPresent=False)


def build(config_file,output):
    raw=ordinary(config_file).read_bytes();config=decode(raw);p=configure(config,output)
    command,env,override,bridge=command_and_environment(p)
    out=p['output'];out.mkdir(exist_ok=False);(out/'temp').mkdir();(out/'ort-cache').mkdir();bridge.mkdir(parents=True,exist_ok=False)
    (out/'config.json').write_bytes(raw)
    (out/'input-proof.json').write_text(json.dumps(dict(payload=p['payload'],compilerSourceSha256=p['sourceSha256'],
        toolSha256=config['toolSha256'],targetStdFiles=config['targetStdFiles'],cargoConfigs=p['cargoConfigs'],officialCodegen=p['codegen'],
        command=command,frontendOnlyConfigOverride=override,defaultFeatures=['custom-protocol'],extraFeatures=[],
        freshTargetDirectory=True),indent=2)+'\n',encoding='utf-8')
    result=dict(passed=False,actualCargoReleaseBuilt=False,defaultFeaturesOnly=False,productInstalled=False,
        deviceCommandsExecuted=0,targetInputsDispatched=0,nativeRuntimeAccepted=False,fullRF312Accepted=False)
    try:
        before=sources();started=time.monotonic()
        with (out/'cargo.stdout').open('xb') as stdout,(out/'cargo.stderr').open('xb') as stderr:
            child=subprocess.Popen(command,cwd=p['workspace'],env=env,stdin=subprocess.DEVNULL,
                stdout=stdout,stderr=stderr,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
            (out/'live.json').write_text(json.dumps(dict(pid=child.pid,command=command),indent=2),encoding='utf-8')
            print('Fresh default Android Release compilation started',flush=True)
            code=child.wait()
        result.update(exitCode=code,elapsedSeconds=time.monotonic()-started)
        if code!=0:raise ValueError('Default Android Cargo build failed')
        build_payload.recheck(p['payload'])
        if (ordinary(config_file).read_bytes()!=raw or sources()!=before
                or cargo_configs(p['workspace'],p['cargoHome'])!=p['cargoConfigs']
                or any(digest(path)!=config['toolSha256'][n] for n,path in p['tools'].items())
                or any(entry(path)!=row for path,row in p['codegen']['sourceFiles'].items())
                or files(p['standard'])!=config['targetStdFiles']):
            raise ValueError('Frozen compiler configuration or input changed')
        path,artifact=compiler_artifact((out/'cargo.stdout').read_bytes(),p['workspace'],out/'cargo-target')
        verification=verify_library(path,Path(p['payload']['frontend']))
        copy=out/'release-libsolo_soul.so';shutil.copyfile(path,copy)
        if entry(copy)!={k:verification[k] for k in ('bytes','sha256')}:raise ValueError('Copied Release library differs')
        generated=p['workspace']/'src-tauri/gen/android'
        bridge_files=files(bridge)
        if bridge_files!=p['codegen']['bridge']:
            raise ValueError('Complete official Cargo Android codegen required')
        if entry(generated/'app/proguard-tauri.pro')!=p['codegen']['proguard']:
            raise ValueError('Official Cargo Android ProGuard output differs')
        verify_gradle_metadata(generated,p['codegen']['androidLibraryRoots'])
        metadata={n:entry(generated/n) for n in ('app/proguard-tauri.pro','app/tauri.build.gradle.kts','tauri.settings.gradle')}
        result.update(passed=True,path=str(copy),**verification,actualCargoReleaseBuilt=True,defaultFeaturesOnly=True,
            sourceHead=p['payload']['sourceHead'],frozenInputsHeld=True,frontendReceiptSha256=config['frontendResultSha256'],
            inputProofSha256=digest(out/'input-proof.json'),compilerSourceSha256=before,
            actualCargoFeatures=artifact['features'],actualCargoProfile=artifact['profile'],actualProductSource=artifact['target']['src_path'],
            generatedAndroidBridge=bridge_files,generatedAndroidMetadata=metadata,
            materializedWorkspace=p['payload']['workspace'],frontendRoot=p['payload']['frontend'])
    except Exception as error:result.update(errorType=type(error).__name__,error=str(error))
    (out/'library.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    return result


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--config',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path);args=parser.parse_args(argv)
    try:
        if os.name!='nt':raise ValueError('This default library entry requires Windows')
        result=build(args.config,args.output)
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        print(type(error).__name__+': '+str(error),file=sys.stderr);return 2
    print(json.dumps({k:v for k,v in result.items() if k not in ('generatedAndroidBridge','generatedAndroidMetadata')}))
    return 0 if result['passed'] else 1


if __name__=='__main__':sys.exit(main())
