"""Capture and build the production frontend in a fresh, explicit local workspace."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import subprocess
import sys
import time

from prepare_avd import ordinary, command_path
from runtime_inputs import decode, digest, hash_value
from dependency_lock import verify as verify_lock

S = Path(__file__).resolve().parent
FIELDS = {'schemaVersion','workspace','sourceHead','pluginHead','sourceFiles','pluginFiles',
          'nodeModules','tools','builderSources','dependencyLock'}
GENERATED_TRACKED = {
    'tauri/src-tauri/bundles/nsis/assets/welcome-sidebar.bmp',
    'tauri/src-tauri/bundles/nsis/assets/header-install.bmp',
    'tauri/src-tauri/bundles/nsis/assets/header-uninstall.bmp',
    'tauri/src-tauri/resources/docs/guides/search-index.json',
}


def relative(name):
    if not isinstance(name,str) or not name or '\\' in name or ':' in name or '\x00' in name:
        raise ValueError('Ordinary relative input name required')
    path = PurePosixPath(name)
    if path.is_absolute() or any(p in ('','.', '..') for p in name.split('/')):
        raise ValueError('Ordinary relative input name required')
    return path


def entry(path):
    path=Path(path).absolute()
    if '..' in path.parts:path=ordinary(path)
    initial=path.lstat()
    for item,metadata in [(path,initial)]+[(p,p.lstat()) for p in path.parents]:
        if stat.S_ISLNK(metadata.st_mode) or getattr(metadata,'st_file_attributes',0)&0x400:
            raise ValueError('Ordinary paths without reparse parents required')
        if not (stat.S_ISREG(metadata.st_mode) if item==path else stat.S_ISDIR(metadata.st_mode)):
            raise ValueError('Ordinary file with directory parents required')
    identity=lambda row:(row.st_dev,row.st_ino,row.st_size,row.st_mtime_ns,row.st_mode,getattr(row,'st_file_attributes',0))
    with path.open('rb') as stream:
        before=path.lstat();handle=os.fstat(stream.fileno())
        # Opening can change metadata before hashing. Bind the opened file first;
        # keep every attribute bit in the complete path and handle snapshots.
        # Windows filename-derived mode bits can differ from handle mode bits.
        shared=lambda row:identity(row)[:4]+identity(row)[5:]
        if (identity(initial)[:-1]!=identity(before)[:-1] or not stat.S_ISREG(handle.st_mode)
                or getattr(before,'st_file_attributes',0)&0x400
                or shared(before)!=shared(handle)):
            raise ValueError('Input identity changed while opening')
        value=hashlib.file_digest(stream,'sha256').hexdigest()
        after=path.lstat();handle_after=os.fstat(stream.fileno())
        if identity(before)!=identity(after) or identity(handle)!=identity(handle_after):
            raise ValueError('Input changed while hashing')
    return dict(bytes=after.st_size,sha256=value)


def files(root, names=None):
    root = ordinary(root,directory=True)
    if names is None:
        names=[]
        for folder, dirs, children in os.walk(root,followlinks=False):
            for name in dirs:ordinary(Path(folder)/name,directory=True)
            names.extend((Path(folder)/name).relative_to(root).as_posix() for name in children)
    result={};seen=set()
    for name in sorted(names):
        path=relative(name)
        if name.casefold() in seen:
            raise ValueError('Aliased source names refused')
        seen.add(name.casefold())
        result[name]=entry(root/path)
    if not result:raise ValueError('Nonempty complete input files required')
    return result


def validate_files(rows):
    if not isinstance(rows,dict) or not rows:raise ValueError('Nonempty complete file snapshot required')
    seen=set()
    for name,row in rows.items():
        relative(name)
        if name.casefold() in seen:raise ValueError('Aliased source names refused')
        seen.add(name.casefold())
        if not isinstance(row,dict) or set(row)!={'bytes','sha256'} or type(row['bytes']) is not int or row['bytes']<0:
            raise ValueError('Exact file size and digest required')
        hash_value(row['sha256'])


def new_output(path):
    path=Path(path).absolute()
    if path.exists() or path.is_symlink():raise ValueError('New output directory required')
    parent=ordinary(path.parent,directory=True)
    result=parent/path.name
    command_path(result)
    return result


def git(root,*args):
    result=subprocess.run(['git','-c','core.quotepath=false','-C',str(root),*args],
        env=dict(os.environ,GIT_OPTIONAL_LOCKS='0'),stdin=subprocess.DEVNULL,capture_output=True,
        timeout=30,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
    result.check_returncode()
    return result.stdout.decode('utf-8').strip()


def names(root,*args):
    return git(root,'ls-files','-z','--',*args).split('\0')[:-1]


def repository_state(root):
    if git(root,'status','--porcelain=v1','--untracked-files=all','--ignore-submodules=all','--','tauri'):
        raise ValueError('Clean product workspace required')
    market=ordinary(root/'SoloSoul_plugin_market',directory=True)
    if git(market,'status','--porcelain=v1','--untracked-files=all'):
        raise ValueError('Clean plugin market required')
    head,plugin=git(root,'rev-parse','HEAD'),git(market,'rev-parse','HEAD')
    if git(root,'ls-tree','HEAD','SoloSoul_plugin_market') != '160000 commit '+plugin+'\tSoloSoul_plugin_market':
        raise ValueError('Plugin source must match the committed submodule')
    return head,plugin


def builder_sources():
    return {n:digest(S/n) for n in ('frontend_build.py','dependency_lock.py','runtime_inputs.py','prepare_avd.py')}


def capture(workspace,node,python,npm_cli,dependencies,output):
    source_before=builder_sources()
    workspace=ordinary(workspace,directory=True);dependencies=ordinary(dependencies,directory=True)
    output=new_output(output)
    tools={n:dict(path=str(ordinary(p)),**entry(p)) for n,p in (
        ('node',node),('python',python),('npmCli',npm_cli))}
    for row in tools.values():command_path(row['path'])
    head,plugin=repository_state(workspace)
    product_names=names(workspace,'tauri')
    if not product_names or any(not n.startswith('tauri/') for n in product_names):
        raise ValueError('Complete tracked product source set required')
    print('Capturing complete tracked product and plugin sources',flush=True)
    product=files(workspace,product_names)
    plugins=files(workspace/'SoloSoul_plugin_market',names(workspace/'SoloSoul_plugin_market'))
    lock=verify_lock(workspace/'tauri',dependencies)
    print('Capturing complete installed Node dependency tree',flush=True)
    dependency_files=files(dependencies)
    plan=dict(schemaVersion=1,workspace=str(workspace),sourceHead=head,pluginHead=plugin,
        sourceFiles=product,pluginFiles=plugins,nodeModules=dict(root=str(dependencies),files=dependency_files),
        tools=tools,builderSources=source_before,dependencyLock=lock)
    print('Independently rechecking the captured input snapshot',flush=True)
    validate(plan)
    output.mkdir(exist_ok=False)
    (output/'frontend-plan.json').write_text(json.dumps(plan,indent=2)+'\n',encoding='utf-8')
    return dict(passed=True,sourceFiles=len(plan['sourceFiles']),pluginFiles=len(plan['pluginFiles']),
                dependencyFiles=len(plan['nodeModules']['files']),buildStarted=False)


def validate(plan):
    if (not isinstance(plan,dict) or set(plan)!=FIELDS or type(plan['schemaVersion']) is not int
            or plan['schemaVersion']!=1 or set(plan['tools'])!={'node','python','npmCli'}
            or set(plan['nodeModules'])!={'root','files'}):
        raise ValueError('Complete frontend preparation plan required')
    root=ordinary(plan['workspace'],directory=True)
    if repository_state(root)!=(plan['sourceHead'],plan['pluginHead']):raise ValueError('Captured source revision changed')
    if plan['builderSources']!=builder_sources():raise ValueError('Captured frontend builder source changed')
    for rows in (plan['sourceFiles'],plan['pluginFiles'],plan['nodeModules']['files']):validate_files(rows)
    if (set(plan['sourceFiles'])!=set(names(root,'tauri'))
            or set(plan['pluginFiles'])!=set(names(root/'SoloSoul_plugin_market'))):
        raise ValueError('Complete original tracked inputs required')
    if files(root,plan['sourceFiles'])!=plan['sourceFiles']:raise ValueError('Captured product source changed')
    if files(root/'SoloSoul_plugin_market',plan['pluginFiles'])!=plan['pluginFiles']:
        raise ValueError('Captured plugin source changed')
    if files(plan['nodeModules']['root'])!=plan['nodeModules']['files']:
        raise ValueError('Complete captured dependencies changed')
    for row in plan['tools'].values():
        if set(row)!={'path','bytes','sha256'} or entry(row['path'])!={k:row[k] for k in ('bytes','sha256')}:
            raise ValueError('Captured frontend executable changed')
        command_path(row['path'])
    if verify_lock(root/'tauri',plan['nodeModules']['root'])!=plan['dependencyLock']:
        raise ValueError('Captured dependency lock binding changed')


def copy_files(source,destination,rows):
    validate_files(rows)
    source=ordinary(source,directory=True)
    for name,row in rows.items():
        origin=source/relative(name)
        if entry(origin)!=row:raise ValueError('Source changed during copy')
        target=destination/relative(name);target.parent.mkdir(parents=True,exist_ok=True)
        with ordinary(origin).open('rb') as a,target.open('xb') as b:shutil.copyfileobj(a,b)
        if entry(origin)!=row or entry(target)!=row:raise ValueError('Copied source differs')


def check_outputs(workspace,frontend):
    frontend_files=files(frontend)
    html=(frontend/'index.html').read_text(encoding='utf-8')
    if '<script type="module"' not in html or not any(n.endswith('.js') for n in frontend_files):
        raise ValueError('Built production frontend entry required')
    mobile=workspace/'tauri/src-tauri/resources-mobile'
    manifest=decode((mobile/'bundled-resource-manifest.json').read_bytes())
    if set(manifest)!={'schema','version','files'} or type(manifest['schema']) is not int or manifest['schema']!=1:
        raise ValueError('Generated bundled resource manifest required')
    resources={};content=[]
    for row in manifest['files']:
        if set(row)!={'path','size','sha256'} or row['path'] in resources:
            raise ValueError('Unique complete resource rows required')
        name=relative(row['path'])
        if name.parts[0]=='docs':path=workspace/'tauri/src-tauri/resources'/name
        elif name.parts[0]=='SoloSoul_plugin_market':path=mobile/name
        else:raise ValueError('Unknown bundled resource root')
        actual=entry(path)
        if type(row['size']) is not int or actual!={'bytes':row['size'],'sha256':hash_value(row['sha256'])}:
            raise ValueError('Generated bundled resource differs')
        resources[row['path']]=actual
        content.append(row['path']+'\0'+str(row['size'])+'\0'+row['sha256']+'\n')
    expected={'docs/'+n:row for n,row in files(workspace/'tauri/src-tauri/resources/docs').items()}
    expected.update({'SoloSoul_plugin_market/'+n:row for n,row in files(mobile/'SoloSoul_plugin_market').items()})
    if resources!=expected or hashlib.sha256(''.join(content).encode()).hexdigest()!=manifest['version']:
        raise ValueError('Complete generated resource set and version required')
    if [n for n in resources] != sorted(resources,key=lambda n:n.encode('utf-16-be')):
        raise ValueError('Canonical generated resource order required')
    return dict(frontend=frontend_files,mobile=files(mobile),resources=resources,resourceVersion=manifest['version'])


def build(plan_file,output):
    raw=Path(plan_file).read_bytes();plan=decode(raw);validate(plan);output=new_output(output)
    output.mkdir(exist_ok=False)
    result=dict(passed=False,productInstalled=False,deviceCommandsExecuted=0,targetInputsDispatched=0,
        fullDefaultArtifactGenerationAccepted=False,fullRF312Accepted=False)
    (output/'frontend-plan.json').write_bytes(raw)
    try:
        workspace=output/'workspace';workspace.mkdir()
        copy_files(plan['workspace'],workspace,plan['sourceFiles'])
        copy_files(Path(plan['workspace'])/'SoloSoul_plugin_market',workspace/'SoloSoul_plugin_market',plan['pluginFiles'])
        copy_files(plan['nodeModules']['root'],workspace/'tauri/node_modules',plan['nodeModules']['files'])
        temp=output/'temp';temp.mkdir();bin_dir=output/'bin';bin_dir.mkdir()
        python=plan['tools']['python']['path'];node=plan['tools']['node']['path']
        (bin_dir/'python3.cmd').write_bytes(('@echo off\r\n"'+python+'" %*\r\n').encode('mbcs'))
        user_config=output/'npm-user.ini';user_config.touch()
        global_config=output/'npm-global.ini';global_config.touch()
        env={k:v for k,v in os.environ.items() if not k.lower().startswith('npm_config_')
             and not k.startswith(('VITE_','TAURI_','SOLOSOUL_')) and k not in ('NODE_OPTIONS','NODE_ENV')}
        env.update(TEMP=str(temp),TMP=str(temp),TMPDIR=str(temp),PYTHONDONTWRITEBYTECODE='1',
            npm_config_cache=str(output/'npm-cache'),npm_config_offline='true',npm_config_audit='false',
            npm_config_fund='false',npm_config_update_notifier='false',npm_config_userconfig=str(user_config),
            npm_config_globalconfig=str(global_config),
            PATH=os.pathsep.join([str(bin_dir),str(Path(node).parent),str(Path(python).parent),env.get('PATH','')]))
        frontend=output/'frontend'
        command=[node,plan['tools']['npmCli']['path'],'run','build','--','--outDir',str(frontend)]
        started=time.monotonic()
        with (output/'build.stdout').open('xb') as stdout,(output/'build.stderr').open('xb') as stderr:
            child=subprocess.Popen(command,cwd=workspace/'tauri',env=env,stdin=subprocess.DEVNULL,
                stdout=stdout,stderr=stderr,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
            (output/'live.json').write_text(json.dumps(dict(pid=child.pid,command=command),indent=2),encoding='utf-8')
            print('Production frontend build started in the copied workspace',flush=True)
            code=child.wait()
        result.update(exitCode=code,elapsedSeconds=time.monotonic()-started,command=command)
        if code!=0:raise ValueError('Production frontend build failed')
        generated=check_outputs(workspace,frontend)
        changed={name:entry(workspace/name) for name,row in plan['sourceFiles'].items() if entry(workspace/name)!=row}
        if set(changed)-GENERATED_TRACKED:raise ValueError('Unexpected copied product source mutation')
        if files(workspace/'SoloSoul_plugin_market',plan['pluginFiles'])!=plan['pluginFiles']:
            raise ValueError('Copied plugin source changed during build')
        validate(plan)
        if Path(plan_file).read_bytes()!=raw:raise ValueError('Frontend plan changed during build')
        result.update(passed=True,frontendBuiltFromCopiedSources=True,originalInputsUnchanged=True,
            sourceHead=plan['sourceHead'],pluginHead=plan['pluginHead'],
            sourceFiles=len(plan['sourceFiles']),pluginFiles=len(plan['pluginFiles']),
            dependencyFiles=len(plan['nodeModules']['files']),planSha256=hashlib.sha256(raw).hexdigest(),
            builderSources=builder_sources(),dependencyLock=plan['dependencyLock'],generatedTrackedFiles=changed,outputs=generated,
            sourceWorkspace=str(workspace),frontendRoot=str(frontend))
    except Exception as error:
        result.update(errorType=type(error).__name__,error=str(error))
    (output/'frontend-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    return result


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__);sub=parser.add_subparsers(dest='mode',required=True)
    prepare=sub.add_parser('capture')
    for name in ('workspace','node','python','npm-cli','dependencies','output'):prepare.add_argument('--'+name,required=True,type=Path)
    run=sub.add_parser('build');run.add_argument('--plan',required=True,type=Path);run.add_argument('--output',required=True,type=Path)
    args=parser.parse_args(argv)
    try:
        if os.name!='nt':raise ValueError('This production build entry requires Windows')
        if args.mode=='capture':
            result=capture(args.workspace,args.node,args.python,args.npm_cli,args.dependencies,args.output)
        else:result=build(args.plan,args.output)
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        print(type(error).__name__+': '+str(error),file=sys.stderr);return 2
    print(json.dumps({k:v for k,v in result.items() if k!='outputs'}))
    return 0 if result['passed'] else 1


if __name__=='__main__':sys.exit(main())
