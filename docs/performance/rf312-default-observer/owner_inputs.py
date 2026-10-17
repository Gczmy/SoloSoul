"""Freeze explicit inputs for an independently owned default-APK runtime."""
import argparse
import json
from pathlib import Path
import re
import sys
from runtime_inputs import decode,digest,hash_value,load_inputs,verify_sources,verify_artifacts
from prepare_avd import ordinary,PROFILE_SHA256,inputs as avd_inputs
from runtime_units import sources,validate as validate_units
import package_check

S=Path(__file__).resolve().parent
FIELDS={'schemaVersion','runtimeInputs','runtimeInputsSha256','avdPreparation','avdPreparationSha256',
        'powershell','powershellSha256','hostSourceSha256','unitEvidence','unitEvidenceSha256','packageReference'}
HOST_FILES={'android-host-audit.py','android-fresh-host-audit.py','android-home-native-regression.py','android-process-memory.py'}


def read(path):return decode(Path(path).read_bytes())


def avd_inventory(path):
    root=ordinary(path,directory=True);files,folders={},[]
    for item in sorted(root.rglob('*')):
        directory=item.is_dir();ordinary(item,directory=directory)
        if directory:folders.append(item.relative_to(root).as_posix())
        else:files[item.relative_to(root).as_posix()]=digest(item)
    return dict(files=files,directories=folders)


def verify_fresh_avd(plan):
    if avd_inventory(Path(plan['avdConfig']).parent)!=plan['freshAvdInventory']:
        raise ValueError('Prepared fresh AVD changed or was booted before owner admission')
    if digest(plan['avdRegistry'])!=plan['avdRegistrySha256']:
        raise ValueError('Prepared AVD registry identity changed')


def validate_config(config):
    if (not isinstance(config,dict) or set(config)!=FIELDS or type(config['schemaVersion']) is not int
            or config['schemaVersion']!=1 or set(config['hostSourceSha256'])!=HOST_FILES):
        raise ValueError('Complete explicit owner configuration required')
    for key in ('runtimeInputs','avdPreparation','powershell','unitEvidence'):
        ordinary(config[key])
        if digest(config[key])!=hash_value(config[key+'Sha256']):raise ValueError('Declared owner input changed')
    if Path(config['powershell']).name.casefold()!='pwsh.exe':raise ValueError('Explicit PowerShell 7 executable required')
    runtime=load_inputs(config['runtimeInputs']);workspace=Path(runtime['host']['workspaceRoot'])
    for name,value in config['hostSourceSha256'].items():
        if digest(ordinary(workspace/'tauri/scripts'/name))!=hash_value(value):raise ValueError('Canonical host or restoration reader changed')
    validate_units(read(config['unitEvidence']))
    preparation_path=Path(config['avdPreparation']);preparation=read(preparation_path)
    avd_config=read(preparation_path.parent/'config.json')
    sdk,jdk,tools,_,_=avd_inputs(avd_config,preparation_path.parent/'owner-admission-not-created')
    if (preparation.get('passed') is not True or not preparation.get('freshSdkUserdata')
            or not preparation.get('createdNewDedicatedAvd') or preparation.get('oldUserdataCopied') is not False
            or preparation.get('emulatorStarted') is not False or preparation.get('exitCode')!=0
            or preparation.get('preparationSourceSha256')!=digest(S/'prepare_avd.py')
            or preparation.get('configSha256')!=PROFILE_SHA256 or preparation['toolSha256']!=avd_config['toolSha256']
            or preparation['name']!=runtime['host']['avd'] or sdk!=Path(runtime['host']['sdkRoot']).resolve()):
        raise ValueError('Successful fresh unbooted preparation for this exact SDK and AVD required')
    avd=ordinary(preparation['avdDirectory'],directory=True)
    if avd.name!=runtime['host']['avd']+'.avd' or digest(ordinary(avd/'config.ini'))!=PROFILE_SHA256:
        raise ValueError('Original prepared AVD hardware configuration required')
    if any(p.name.endswith('.lock') or p.name in ('hardware-qemu.ini','userdata-qemu.img','userdata-qemu.img.qcow2') for p in avd.iterdir()):
        raise ValueError('Already booted or live AVD cannot replace fresh SDK userdata')
    package_tools=dict(aapt2=str(ordinary(sdk/'build-tools/35.0.0/aapt2.exe')),
        apksigner=str(ordinary(sdk/'build-tools/35.0.0/lib/apksigner.jar')),java=str(tools['java']))
    return runtime,preparation,sdk,jdk,package_tools,tools


def verify_plan(plan,*,artifacts=False):
    if plan.get('schemaVersion')!=1 or plan.get('kind')!='owned-default-twelve-step-runtime':raise ValueError('Prepared owner plan required')
    if Path(plan['helperRoot']).resolve()!=S or sources()!=plan['ownerSourceFiles']:raise ValueError('Frozen owner source files changed')
    config=plan['config'];runtime=read(config['runtimeInputs'])
    for key in ('runtimeInputs','avdPreparation','powershell','unitEvidence'):
        if digest(config[key])!=config[key+'Sha256']:raise ValueError('Frozen owner input changed')
    if runtime!=plan['runtime']:raise ValueError('Frozen declared runtime differs')
    verify_sources(runtime)
    if artifacts:verify_artifacts(runtime)
    root=Path(runtime['host']['workspaceRoot'])
    for name,value in config['hostSourceSha256'].items():
        if digest(root/'tauri/scripts'/name)!=value:raise ValueError('Canonical host source changed')
    if digest(plan['avdConfig'])!=PROFILE_SHA256:raise ValueError('Prepared hardware configuration changed')
    for name,value in plan['packageToolSha256'].items():
        if digest(plan['packageTools'][name])!=value:raise ValueError('Frozen package tool changed')
    if digest(plan['avdPreparationConfig'])!=plan['avdPreparationConfigSha256']:
        raise ValueError('Original AVD preparation configuration changed')
    for name,value in plan['avdToolSha256'].items():
        if digest(plan['avdTools'][name])!=value:raise ValueError('Frozen declared AVD tool changed')
    if (digest(plan['packageVerificationResult'])!=plan['packageVerificationResultSha256']
            or read(plan['packageVerificationResult'])!=plan['packageVerification']):
        raise ValueError('Original package verification evidence changed')


def load_plan(path,*,artifacts=False):
    plan=read(path);verify_plan(plan,artifacts=artifacts);return plan


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args(argv)
    try:
        raw=args.config.read_bytes();config=decode(raw)
        runtime,preparation,sdk,jdk,tools,avd_tools=validate_config(config)
        if args.output.exists() or args.output.is_symlink():raise ValueError('New owner preparation output required')
        parent=ordinary(args.output.absolute().parent,directory=True);output=parent/args.output.name
    except (OSError,ValueError,KeyError,TypeError):
        print('Invalid owner inputs, fresh AVD or existing output',file=sys.stderr);return 2
    output.mkdir(exist_ok=False)
    result=dict(passed=False,emulatorStarted=False,adbCommandsExecuted=0,fullPortableOuterRunnerAccepted=False,fullRF312Accepted=False)
    try:
        frozen=sources()
        package=package_check.verify(runtime,config['packageReference'],tools,output/'package-verification')
        if args.config.read_bytes()!=raw or sources()!=frozen:raise ValueError('Owner inputs changed during package verification')
        plan=dict(schemaVersion=1,kind='owned-default-twelve-step-runtime',config=config,runtime=runtime,
            helperRoot=str(S),ownerSourceFiles=frozen,packageTools=tools,packageToolSha256={n:digest(v) for n,v in tools.items()},
            avdPreparationConfig=str(Path(config['avdPreparation']).parent/'config.json'),
            avdPreparationConfigSha256=digest(Path(config['avdPreparation']).parent/'config.json'),
            avdTools={name:str(path) for name,path in avd_tools.items()},avdToolSha256=preparation['toolSha256'],
            avdConfig=str(Path(preparation['avdDirectory'])/'config.ini'),avdRoot=str(Path(preparation['avdDirectory']).parent),
            androidUserRoot=str(Path(config['avdPreparation']).parent/'android-user'),
            freshAvdInventory=avd_inventory(preparation['avdDirectory']),
            avdRegistry=str(Path(preparation['avdDirectory']).with_suffix('.ini')),
            avdRegistrySha256=digest(Path(preparation['avdDirectory']).with_suffix('.ini')),
            resourceManifest=read(output/'package-verification/bundled-resource-manifest.json'),
            packageVerification=package,packageVerificationResult=str(output/'package-verification/result.json'),
            packageVerificationResultSha256=digest(output/'package-verification/result.json'))
        verify_plan(plan,artifacts=True)
        verify_fresh_avd(plan)
        (output/'owner-plan.json').write_text(json.dumps(plan,indent=2)+'\n',encoding='utf-8')
        result.update(passed=True,ownerPlanSha256=digest(output/'owner-plan.json'),allAssetFilesVerified=package['allAssetsVerified'])
    except Exception as error:result.update(errorType=type(error).__name__,error=str(error))
    (output/'config.json').write_bytes(raw)
    (output/'preparation-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result));return 0 if result['passed'] else 1


if __name__=='__main__':sys.exit(main())
