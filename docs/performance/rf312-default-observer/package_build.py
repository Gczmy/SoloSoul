"""Package a passed source-bound default library in a fresh isolated Android tree."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import zipfile

from frontend_build import copy_files, entry, files, new_output, relative
import package_environment
import package_payload
from prepare_avd import ordinary, command_path
from runtime_inputs import decode, digest, hash_value

S=Path(__file__).resolve().parent
FIELDS={'schemaVersion','libraryResult','libraryResultSha256','sdkRoot','jdkRoot','gradleRoot','gradleHome',
    'toolSha256','gradleFiles','gradleHomeConfiguration','dependencyReference','testSigner'}


def sources():return {name:digest(S/name) for name in ('package_build.py','package_payload.py','package_environment.py')}


def configure(config,output):
    output=new_output(output)
    if (not isinstance(config,dict) or set(config)!=FIELDS or type(config['schemaVersion']) is not int or config['schemaVersion']!=1):
        raise ValueError('Complete source-bound default package configuration required')
    bound=package_payload.load_library(config['libraryResult'],config['libraryResultSha256']);payload=bound['payload']
    for root in (Path(payload['workspace']),Path(payload['frontend']),Path(bound['receipt']).parent):
        if output.is_relative_to(root):raise ValueError('Package output cannot be inside frozen library or frontend inputs')
    project=Path(payload['workspace'])/'tauri/src-tauri/gen/android'
    prepared=package_environment.tool_inputs(config,project)
    reference=config['dependencyReference'];signer=config['testSigner']
    if (not isinstance(reference,dict) or not isinstance(signer,dict)
            or set(reference)!={'apk','appAssets','certificateSha256'}
            or not isinstance(reference['appAssets'],dict) or not reference['appAssets']
            or not isinstance(reference['apk'],dict) or set(reference['apk'])!={'path','bytes','sha256'}
            or set(signer)!={'keyFile','sha256','certificateSha256','purpose'}
            or signer['purpose']!='internal-validation-only'
            or hash_value(signer['certificateSha256'])!=hash_value(reference['certificateSha256'])):
        raise ValueError('Explicit matching internal test signer and dependency reference required')
    previous=ordinary(reference['apk']['path']);key=ordinary(signer['keyFile'])
    if entry(previous)!={k:reference['apk'][k] for k in ('bytes','sha256')} or entry(key)['sha256']!=hash_value(signer['sha256']):
        raise ValueError('Frozen dependency APK or internal test key changed')
    for name,value in reference['appAssets'].items():relative(name);hash_value(value)
    for path in (previous,key,project,output):command_path(path)
    return dict(output=output,bound=bound,tools=prepared,reference=reference,signer=signer,
        version=package_environment.version_properties(payload),sourceSha256=sources())


def install(source,target,expected):
    if entry(source)!=expected:raise ValueError('Source-generated package input changed')
    if target.exists() or target.is_symlink():
        if entry(target)!=expected:raise ValueError('Conflicting tracked and generated project file')
        return
    target.parent.mkdir(parents=True,exist_ok=True)
    with ordinary(source).open('rb') as a,target.open('xb') as b:shutil.copyfileobj(a,b)
    if entry(source)!=expected or entry(target)!=expected:raise ValueError('Copied package input differs')


def stage_version_file(path,key,source_files,version):
    if key in source_files:
        if entry(path)!=source_files[key]:raise ValueError('Bound Android version file changed')
        values={}
        for line in path.read_text(encoding='utf-8-sig').splitlines():
            line=line.strip()
            if not line or line.startswith(('#','!','//')):continue
            match=re.fullmatch(r'(tauri\.android\.version(?:Name|Code))\s*=\s*(\S+)',line)
            if match is None or match[1] in values:raise ValueError('Plain unique tracked Android version properties required')
            values[match[1]]=match[2]
        if values!={'tauri.android.versionName':version['versionName'],'tauri.android.versionCode':str(version['versionCode'])}:
            raise ValueError('Tracked Android version properties differ from current source configuration')
        return dict(trackedSourcePreserved=True,**entry(path))
    if path.exists() or path.is_symlink():raise ValueError('Unbound Android version properties refused')
    with path.open('xb') as target:target.write(version['properties'].encode('ascii'))
    return dict(trackedSourcePreserved=False,**entry(path))


def stage_inputs(p):
    out=p['output'];bound=p['bound'];payload=bound['payload'];source=Path(payload['workspace'])
    workspace=out/'workspace';workspace.mkdir()
    copy_files(source,workspace,payload['sources'])
    copy_files(source/'SoloSoul_plugin_market',workspace/'SoloSoul_plugin_market',payload['plugins'])
    mobile='tauri/src-tauri/resources-mobile/'
    for name,row in payload['outputs']['mobile'].items():
        install(source/mobile/name,workspace/mobile/name,row)
    android='tauri/src-tauri/gen/android/';generated=source/android
    for name,row in bound['generatedMetadata'].items():install(generated/name,workspace/android/name,row)
    bridge='app/src/main/java/com/solosoul/app/generated/'
    for name,row in bound['generatedBridge'].items():install(generated/bridge/name,workspace/android/bridge/name,row)
    project=workspace/android
    assets=package_payload.source_assets(payload);asset_root=out/'assets';asset_root.mkdir()
    for name,row in assets.items():install(row['path'],asset_root/name,{k:row[k] for k in ('bytes','sha256')})
    native=out/'jni/x86_64/libsolo_soul.so'
    install(bound['library']['path'],native,{k:bound['library'][k] for k in ('bytes','sha256')})
    module_root=out/'modules';module_root.mkdir();modules={}
    for label in sorted(package_payload.MODULES):
        modules[label]=package_payload.stage_module(source/'tauri',Path(bound['cargoHome']),label,module_root/label,payload['sources'])
    properties=project/'app/tauri.properties'
    property_key=android+'app/tauri.properties'
    version_file=stage_version_file(properties,property_key,payload['sources'],p['version'])
    bindings=dict(project=str(project),modules={n:str(module_root/n) for n in modules},buildRoot=str(out/'gradle-build'),
        assets=str(asset_root),jni=str(out/'jni'),jdk=str(p['tools']['jdk']),profileProof=str(out/'profile-proof.json'),
        runtimeProof=str(out/'runtime-proof.json'))
    binding=out/'bindings.json';binding.write_text(json.dumps(bindings,indent=2)+'\n',encoding='utf-8')
    init=out/'package.init.gradle';init.write_text(package_environment.init_script(binding),encoding='utf-8')
    return dict(workspace=str(workspace),project=str(project),modules=modules,bindings=bindings,
        assets={n:{k:r[k] for k in ('bytes','sha256')} for n,r in assets.items()},
        library={k:bound['library'][k] for k in ('bytes','sha256')},
        versionProperties=version_file,
        generatedProjectFiles={str(path.relative_to(workspace)).replace('\\','/'):entry(path)
            for path in [*([properties] if not version_file['trackedSourcePreserved'] else []),
                         *[workspace/android/n for n in bound['generatedMetadata']],
                         *[workspace/android/bridge/n for n in bound['generatedBridge']],
                         *[workspace/mobile/n for n in payload['outputs']['mobile']]]},
        bindingSha256=digest(binding),initSha256=digest(init))


def verify_badging(text,version=None):
    required=("package: name='com.solosoul.app'","native-code: 'x86_64'","minSdkVersion:'28'",
              "targetSdkVersion:'36'","launchable-activity: name='com.solosoul.app.MainActivity'")
    native=[line for line in text.splitlines() if line.startswith('native-code:')]
    if (not all(value in text for value in required) or 'application-debuggable' in text
            or native!=["native-code: 'x86_64'"]):
        raise ValueError('Non-debug current default Android package identity required')
    if version and ("versionCode='"+str(version['versionCode'])+"'" not in text or "versionName='"+version['versionName']+"'" not in text):
        raise ValueError('Current source-derived Android version required')


def package(config_file,output):
    raw=ordinary(config_file).read_bytes();config=decode(raw);p=configure(config,output);out=p['output']
    out.mkdir(exist_ok=False)
    for name in ('temp','java-user','android-user'):(out/name).mkdir()
    (out/'config.json').write_bytes(raw);environment=package_environment.environment(p['tools'],out)
    result=dict(passed=False,actualDefaultApkBuilt=False,defaultArtifactGenerationAccepted=False,
        productInstalled=False,deviceCommandsExecuted=0,targetInputsDispatched=0,
        nativeRuntimeAccepted=False,fullRF312Accepted=False)
    commands=[]
    def call(label,command,expected=0):
        with (out/(label+'.stdout')).open('xb') as stdout,(out/(label+'.stderr')).open('xb') as stderr:
            child=subprocess.Popen(command,cwd=out,env=environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
                creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
            (out/(label+'.live.json')).write_text(json.dumps(dict(pid=child.pid,command=command),indent=2),encoding='utf-8')
            print(label+' started',flush=True);code=child.wait()
        commands.append(dict(label=label,exitCode=code,stdoutSha256=digest(out/(label+'.stdout')),stderrSha256=digest(out/(label+'.stderr'))))
        if code!=expected:raise ValueError(label+' failed')
        return (out/(label+'.stdout')).read_text(encoding='utf-8')
    tools=p['tools']['tools'];sign_tool=[str(tools['java']),'-jar',str(tools['apksigner'])]
    try:
        signature=call('reference-signature',sign_tool+['verify','--verbose','--print-certs',p['reference']['apk']['path']])
        if re.findall(r'certificate SHA-256 digest: (\S+)',signature)!=[p['reference']['certificateSha256']]:
            raise ValueError('Declared frozen dependency signer differs')
        verify_badging(call('reference-badging',[str(tools['aapt2']),'dump','badging',p['reference']['apk']['path']]))
        staged=stage_inputs(p)
        proof=dict(boundLibrary=p['bound'],toolConfig=config,sourceSha256=p['sourceSha256'],staged=staged,version=p['version'])
        (out/'input-proof.json').write_text(json.dumps(proof,indent=2)+'\n',encoding='utf-8')
        cmd=ordinary(Path(os.environ['SystemRoot'])/'System32/cmd.exe')
        args=['-p',staged['project'],':app:assembleX86_64Release','-x',':app:rustBuildX86_64Release',
            '--init-script',str(out/'package.init.gradle'),'--offline','--no-daemon','--max-workers','2',
            '--no-build-cache','--no-configuration-cache','--project-cache-dir',str(out/'gradle-project-cache'),
            '-Dorg.gradle.java.home='+str(p['tools']['jdk']),'-Pkotlin.compiler.execution.strategy=in-process']
        started=time.time();call('gradle-package',package_environment.gradle_command(cmd,p['tools']['gradle']/'bin/gradle.bat',args))
        candidates=list((out/'gradle-build').glob('**/outputs/apk/x86_64/release/*.apk'))
        if len(candidates)!=1 or candidates[0].stat().st_mtime<started-2:raise ValueError('One fresh Gradle default APK required')
        candidate=candidates[0];metadata=decode(ordinary(candidate.parent/'output-metadata.json').read_bytes())
        elements=metadata['elements']
        if (metadata['applicationId']!='com.solosoul.app' or metadata['variantName']!='x86_64Release' or len(elements)!=1
                or elements[0]['outputFile']!=candidate.name or elements[0]['versionName']!=p['version']['versionName']
                or elements[0]['versionCode']!=p['version']['versionCode']):
            raise ValueError('Fresh package metadata differs from current source')
        runtime=decode(ordinary(out/'runtime-proof.json').read_bytes())
        expected={':':str(ordinary(staged['project'],directory=True)),':app':str(ordinary(Path(staged['project'])/'app',directory=True))}
        expected.update({':'+name:str(ordinary(out/'modules'/name,directory=True)) for name in package_payload.MODULES})
        if ({k:ordinary(v,directory=True) for k,v in runtime['projects'].items()}!={k:Path(v) for k,v in expected.items()}
                or ordinary(runtime['javaHome'],directory=True)!=p['tools']['jdk']):raise ValueError('Actual private Gradle graph or JDK differs')
        profile_record=decode(ordinary(out/'profile-proof.json').read_bytes())
        if profile_record['task']!=':app:compileX86_64ReleaseArtProfile' or set(profile_record['files'])!={'baseline.prof','baseline.profm'}:
            raise ValueError('Actual current profile task output pair required')
        profiles={}
        for name,row in profile_record['files'].items():
            path=ordinary(row['path'])
            if not path.is_relative_to(out/'gradle-build') or entry(path)!={k:row[k] for k in ('bytes','sha256')}:
                raise ValueError('Current generated profile changed')
            profiles['dexopt/'+name]={k:row[k] for k in ('bytes','sha256')}
        unsigned=out/'app-x86_64-release-unsigned.apk';install(candidate,unsigned,entry(candidate))
        byte_proof=package_payload.verify_apk_bytes(unsigned,staged['assets'],staged['library'],p['reference'],profiles)
        verify_badging(call('unsigned-badging',[str(tools['aapt2']),'dump','badging',str(unsigned)]),p['version'])
        call('unsigned-signature-refusal',sign_tool+['verify','--verbose',str(unsigned)],expected=1)
        signed=out/'app-x86_64-release-internal-validation.apk'
        call('internal-test-sign',sign_tool+['sign','--ks',p['signer']['keyFile'],'--ks-key-alias','androiddebugkey',
            '--ks-pass','pass:android','--key-pass','pass:android','--v1-signing-enabled','false',
            '--v2-signing-enabled','true','--v3-signing-enabled','true','--v4-signing-enabled','false','--out',str(signed),str(unsigned)])
        signature=call('signed-signature',sign_tool+['verify','--verbose','--print-certs',str(signed)])
        if re.findall(r'certificate SHA-256 digest: (\S+)',signature)!=[p['signer']['certificateSha256']]:raise ValueError('Internal signer differs')
        verify_badging(call('signed-badging',[str(tools['aapt2']),'dump','badging',str(signed)]),p['version'])
        if package_payload.verify_apk_bytes(signed,staged['assets'],staged['library'],p['reference'],profiles)!=byte_proof:
            raise ValueError('Signing changed source or dependency assets')
        with zipfile.ZipFile(unsigned) as a,zipfile.ZipFile(signed) as b:
            if package_payload.zip_files(a)!=package_payload.zip_files(b) or any(a.read(n)!=b.read(n) for n in package_payload.zip_files(a)):
                raise ValueError('Signing changed an APK entry')
        if (sources()!=p['sourceSha256'] or ordinary(config_file).read_bytes()!=raw
                or package_payload.load_library(config['libraryResult'],config['libraryResultSha256'])!=p['bound']):
            raise ValueError('Frozen package inputs changed')
        package_environment.tool_inputs(config,Path(staged['project']))
        if (digest(out/'bindings.json')!=staged['bindingSha256'] or digest(out/'package.init.gradle')!=staged['initSha256']
                or files(Path(staged['workspace']),p['bound']['payload']['sources'])!=p['bound']['payload']['sources']
                or files(Path(staged['workspace'])/'SoloSoul_plugin_market')!=p['bound']['payload']['plugins']):
            raise ValueError('Private source or build binding changed')
        for name,row in staged['generatedProjectFiles'].items():
            if entry(Path(staged['workspace'])/name)!=row:raise ValueError('Generated project metadata changed')
        for name,record in staged['modules'].items():
            if files(out/'modules'/name)!=record['files'] or entry(record['archive']['path'])!={k:record['archive'][k] for k in ('bytes','sha256')}:
                raise ValueError('Locked private module source changed')
        for row in runtime['externalArtifacts'].values():
            package_environment.verify_external_artifact(row)
        if entry(p['signer']['keyFile'])['sha256']!=p['signer']['sha256'] or entry(p['reference']['apk']['path'])!={k:p['reference']['apk'][k] for k in ('bytes','sha256')}:
            raise ValueError('Internal test signer or old reference changed')
        result.update(passed=True,actualDefaultApkBuilt=True,defaultArtifactGenerationAccepted=True,
            apk=dict(path=str(signed),**entry(signed)),libraryReceiptSha256=config['libraryResultSha256'],
            inputProofSha256=digest(out/'input-proof.json'),sourceSha256=p['sourceSha256'],
            certificateSha256=p['signer']['certificateSha256'],version=p['version'],byteProof=byte_proof,
            nonDebuggable=True,allSigningEntriesPreserved=True,resolvedExternalArtifacts=len(runtime['externalArtifacts']),
            privateProjectGraphVerified=True,sharedCargoModuleSourcesWritten=False,
            oldReferenceAndLibraryInputsPreserved=True)
    except Exception as error:result.update(errorType=type(error).__name__,error=str(error))
    result['commands']=commands
    (out/'package-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    return result


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--config',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args(argv)
    try:
        if os.name!='nt':raise ValueError('This default package builder requires Windows')
        result=package(args.config,args.output)
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        print(type(error).__name__+': '+str(error),file=sys.stderr);return 2
    print(json.dumps({k:result[k] for k in ('passed','actualDefaultApkBuilt','defaultArtifactGenerationAccepted','fullRF312Accepted')}))
    return 0 if result['passed'] else 1


if __name__=='__main__':sys.exit(main())
