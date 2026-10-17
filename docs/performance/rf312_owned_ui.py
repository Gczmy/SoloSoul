"""Explicit, source-bound Observer-UID UI transport; original auxiliary controls remain."""
import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import zipfile

HERE=Path(__file__).resolve().parent
sys.path[:0]=[str(HERE/'rf312-default-observer'),str(HERE/'rf312-ipc-runtime')]
import persistent_adapter
from runtime_inputs import decode, digest, hash_value
from prepare_avd import ordinary

SOURCE=HERE/'rf312-owned-ui'
CLASS='org.solosoul.rf312transport.OwnedUi'
FIELDS={'schemaVersion','passed','artifact','artifactSha256','bytes','className','compileSdk',
        'minApi','sourceSha256','tools','steps','observerApkChanged','productInstalled','nativeRuntimeAccepted'}


def sources():
    return {name:digest(HERE/name) for name in
            ('rf312_owned_ui.py','rf312-owned-ui/OwnedUi.java','rf312-owned-ui/file_binding.py')}


def package(path, expected_sha256):
    """Read an explicitly frozen build receipt before any SDK or input operation."""
    path=ordinary(path); raw=path.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=hash_value(expected_sha256):
        raise ValueError('Owned UI package receipt changed')
    value=decode(raw)
    if (not isinstance(value,dict) or set(value)!=FIELDS or type(value['schemaVersion'])is not int
            or value['schemaVersion']!=1 or value['passed']is not True or value['className']!=CLASS
            or value['compileSdk']!=36 or value['minApi']!=28 or value['sourceSha256']!=sources()
            or any(value[key]is not False for key in ('observerApkChanged','productInstalled','nativeRuntimeAccepted'))):
        raise ValueError('Exact source-bound Android UI helper package required')
    artifact=ordinary(value['artifact'])
    if (type(value['bytes'])is not int or not 0<value['bytes']<=1024*1024
            or artifact.stat().st_size!=value['bytes'] or digest(artifact)!=hash_value(value['artifactSha256'])):
        raise ValueError('Owned UI artifact bytes changed')
    with zipfile.ZipFile(artifact) as archive:
        if (archive.namelist()!=['classes.dex'] or archive.getinfo('classes.dex').file_size>1024*1024
                or not archive.read('classes.dex').startswith(b'dex\n')):
            raise ValueError('Only the bounded compiled Android helper is allowed')
    if not isinstance(value['tools'],dict) or set(value['tools'])!={'java','javac','android','d8'}:
        raise ValueError('All actual compiler tools must remain bound')
    for row in value['tools'].values():
        if not isinstance(row,dict) or set(row)!={'path','sha256'} or digest(ordinary(row['path']))!=hash_value(row['sha256']):
            raise ValueError('Frozen Android compiler tool changed')
    if (not isinstance(value['steps'],list) or len(value['steps'])!=2
            or [row.get('step') for row in value['steps']]!=['javac','d8']
            or any(type(row.get('exitCode'))is not int or row['exitCode']!=0 for row in value['steps'])):
        raise ValueError('Successful actual Java and Android compilation required')
    if path.read_bytes()!=raw or value['sourceSha256']!=sources() or digest(artifact)!=value['artifactSha256']:
        raise ValueError('UI package changed while validating')
    return value


def file_binding():
    spec=importlib.util.spec_from_file_location('rf312_owned_file_binding',SOURCE/'file_binding.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module.binding


@contextmanager
def binding(observer, original_adb, output, *, executable, serial,
            expected_executable_sha256, script, shell_crlf):
    """Two owned channels with fixed routing, no SDK fallback or input replay."""
    output=Path(output); auxiliary=output/'auxiliary-control';auxiliary.mkdir(exist_ok=False)
    arguments=dict(executable=executable,serial=serial,
                   expected_executable_sha256=expected_executable_sha256,shell_crlf=shell_crlf)
    with persistent_adapter.binding(observer,original_adb,auxiliary,
            script=HERE/'rf312-default-observer/persistent_control.sh',**arguments) as (auxiliary_run,_):
        with file_binding()(observer,original_adb,output,script=script,**arguments) as (ui_run,_):
            def run(*command,**kwargs):
                channel=ui_run if command and command[0]in ('rf312-exchange','rf312-get-stable') else auxiliary_run
                return channel(*command,**kwargs)
            def read(*command):
                return run(*command,stdout=subprocess.PIPE,stderr=subprocess.PIPE).stdout.decode().strip()
            observer.adb=run
            yield run,read


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--jdk',type=Path,required=True)
    parser.add_argument('--android-jar',type=Path,required=True)
    parser.add_argument('--d8',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args(argv)
    suffix='.exe' if os.name=='nt' else ''
    tools={'java':ordinary(args.jdk/'bin'/('java'+suffix)),
           'javac':ordinary(args.jdk/'bin'/('javac'+suffix)),
           'android':ordinary(args.android_jar),'d8':ordinary(args.d8)}
    output=args.output.absolute()
    if any(c in str(output)for c in ('"','\r','\n')) or output.exists() or output.is_symlink():
        parser.error('A new output directory without quote or newline characters is required')
    output.mkdir(exist_ok=False);classes=output/'classes';classes.mkdir();dex=output/'dex';dex.mkdir()
    frozen=sources(); tool_rows={k:dict(path=str(v),sha256=digest(v))for k,v in tools.items()}
    env=dict(os.environ,TEMP=str(output),TMP=str(output))
    for name in ('JAVA_TOOL_OPTIONS','_JAVA_OPTIONS','JDK_JAVA_OPTIONS'):env.pop(name,None)
    env['JAVA_TOOL_OPTIONS']='-Duser.home="'+output.as_posix()+'" -Djava.io.tmpdir="'+output.as_posix()+'"'
    steps=[]
    def run(label,command):
        if sources()!=frozen or any(digest(tools[k])!=v['sha256']for k,v in tool_rows.items()):
            raise ValueError('Build inputs changed')
        started=time.monotonic()
        result=subprocess.run(list(map(str,command)),cwd=output,env=env,stdin=subprocess.DEVNULL,
            capture_output=True,timeout=120,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        (output/(label+'.stdout')).write_bytes(result.stdout);(output/(label+'.stderr')).write_bytes(result.stderr)
        steps.append(dict(step=label,exitCode=result.returncode,elapsedSeconds=time.monotonic()-started));result.check_returncode()
    run('javac',[tools['javac'],'--release','11','-cp',tools['android'],'-d',classes,SOURCE/'OwnedUi.java'])
    expected={CLASS.replace('.','/')+suffix+'.class'for suffix in ('','$Reply','$Refusal')}
    actual={p.relative_to(classes).as_posix()for p in classes.rglob('*.class')}
    if actual!=expected:raise ValueError('Only the three real helper classes may be packaged')
    jar=output/'classes.jar'
    with zipfile.ZipFile(jar,'x',zipfile.ZIP_DEFLATED)as archive:
        for name in sorted(actual):archive.write(classes/name,name)
    run('d8',[tools['java'],'-cp',tools['d8'],'com.android.tools.r8.D8','--lib',tools['android'],
              '--min-api','28','--output',dex,jar])
    artifact=output/'owned-ui.jar'
    with zipfile.ZipFile(artifact,'x',zipfile.ZIP_DEFLATED)as archive:archive.write(dex/'classes.dex','classes.dex')
    value=dict(schemaVersion=1,passed=True,artifact=str(artifact),artifactSha256=digest(artifact),bytes=artifact.stat().st_size,
        className=CLASS,compileSdk=36,minApi=28,sourceSha256=frozen,tools=tool_rows,steps=steps,
        observerApkChanged=False,productInstalled=False,nativeRuntimeAccepted=False)
    receipt=output/'package.json';receipt.write_text(json.dumps(value,indent=2)+'\n',encoding='utf-8')
    package(receipt,digest(receipt))
    print(json.dumps(dict(passed=True,package=str(receipt),sha256=digest(receipt),nativeRuntimeAccepted=False)))
    return 0


if __name__=='__main__':raise SystemExit(main())
