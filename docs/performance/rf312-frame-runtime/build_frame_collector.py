"""Build only the explicit frame actor from exact default APK/JDK/SDK inputs.

No SDK daemon, emulator, app installation or native measurement is started.
"""
import argparse,hashlib,json,os,re,subprocess,sys,zipfile
from pathlib import Path
import xml.etree.ElementTree as ET
import frame_binding
from frame_manifest import compiled_identity,PACKAGE,CLASS,APP,ANDROID

ROOT=Path(__file__).resolve().parent
producer=frame_binding.producer
sha=frame_binding.sha
FIELDS={'schemaVersion','runtimeInputs','runtimeInputsSha256','defaultPackageConfig',
        'defaultPackageConfigSha256','nodeExecutable','toolSha256','sourceSha256'}
TOOLS={'java','javac','jvm','androidJar','aapt2','apksigner','d8','zipalign','node'}
LOCAL=('FrameCollector.java','AndroidManifest.xml','readiness.node.test.mjs','frame_manifest.py','build_frame_collector.py','FrameCommitSequence.java','FrameCommitSequenceTest.java','WindowFrameTrace.java')

def source_hashes():
    return {**{'runtime/'+name:value for name,value in frame_binding.sources().items()},
            **{name:sha(ROOT/name) for name in LOCAL}}

def save(path,value):
    with path.open('x',encoding='utf-8',newline='\n') as stream:
        json.dump(value,stream,indent=2,allow_nan=False);stream.write('\n')

def configuration(config):
    if (type(config) is not dict or set(config)!=FIELDS or type(config['schemaVersion']) is not int
            or config['schemaVersion']!=1 or type(config['toolSha256']) is not dict or set(config['toolSha256'])!=TOOLS):
        raise ValueError('Exact frame builder configuration required')
    for name in ['runtimeInputs','defaultPackageConfig','nodeExecutable']:
        if type(config[name]) is not str or not Path(config[name]).is_absolute():
            raise ValueError('Explicit absolute input paths required')
        producer.command_path(producer.ordinary(config[name]))
    for value in [config['runtimeInputsSha256'],config['defaultPackageConfigSha256'],*config['toolSha256'].values()]:
        producer.package.producer.hash_value(value)
    if config['sourceSha256']!=source_hashes():raise ValueError('Complete frame builder source binding required')

def main(argv=None):
    frame_binding.require_assertions()
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args(argv)
    config_file=producer.ordinary(args.config);raw=config_file.read_bytes()
    config=producer.package.producer.decode(raw);configuration(config)
    runtime=producer.package.producer.load_inputs(config['runtimeInputs'])
    if sha(config['runtimeInputs'])!=config['runtimeInputsSha256']:raise ValueError('Original default runtime input bytes changed')
    baseline=producer.package.producer.bound_json(config['defaultPackageConfig'],config['defaultPackageConfigSha256'])
    sdk=producer.ordinary(baseline['sdkRoot'],directory=True);jdk=producer.ordinary(baseline['jdkRoot'],directory=True)
    build=sdk/'build-tools/35.0.0'
    tools=dict(java=jdk/'bin/java.exe',javac=jdk/'bin/javac.exe',jvm=jdk/'bin/server/jvm.dll',
        androidJar=sdk/'platforms/android-36/android.jar',aapt2=build/'aapt2.exe',apksigner=build/'lib/apksigner.jar',
        d8=build/'lib/d8.jar',zipalign=build/'zipalign.exe',node=Path(config['nodeExecutable']))
    for name,path in tools.items():
        producer.ordinary(path);producer.command_path(path)
        if sha(path)!=config['toolSha256'][name]:raise ValueError('Exact frame builder tool identity required')
        if name in ['java','javac','jvm','androidJar','aapt2','apksigner'] and sha(path)!=baseline['toolSha256'][name]:
            raise ValueError('Original baseline JDK and SDK tool identity required')
    if tools['node'].name!='node.exe':raise ValueError('Explicit Windows Node executable required')
    signer=baseline['testSigner'];key=producer.ordinary(signer['keyFile'])
    if (signer['purpose']!='internal-validation-only' or sha(key)!=signer['sha256']
            or signer['certificateSha256']!=runtime['release']['certificateSha256']
            or runtime['release']['productionSigningUsed'] is not False):
        raise ValueError('Original internal validation signer required')
    source=producer.ordinary(ROOT/'FrameCollector.java');manifest=producer.ordinary(ROOT/'AndroidManifest.xml')
    tests=producer.ordinary(ROOT/'readiness.node.test.mjs')
    sequence=producer.ordinary(ROOT/'FrameCommitSequence.java');sequence_test=producer.ordinary(ROOT/'FrameCommitSequenceTest.java')
    window_source=producer.ordinary(ROOT/'WindowFrameTrace.java')
    tree=ET.fromstring(manifest.read_bytes())
    expected={'uses-sdk':{'minSdkVersion':'29','targetSdkVersion':'35'},
        'application':{'debuggable':'false','allowBackup':'false','hasCode':'true'},
        'instrumentation':{'name':CLASS,'targetPackage':APP,'targetProcesses':APP,'functionalTest':'true','handleProfiling':'false'}}
    if tree.tag!='manifest' or tree.attrib!={'package':PACKAGE} or sorted(n.tag for n in tree)!=sorted(expected):
        raise ValueError('Exact source frame manifest required')
    for node in tree:
        if list(node) or node.attrib!={'{'+ANDROID+'}'+name:value for name,value in expected[node.tag].items()}:
            raise ValueError('Frame manifest privileges or identity changed')
    out=args.output.absolute();producer.ordinary(out.parent,directory=True)
    if out.exists():raise ValueError('New frame output directory required')
    out.mkdir(exist_ok=False)
    for name in ['temp','java-user','classes','test-classes','dex']:(out/name).mkdir()
    inputs={str(p):sha(p) for p in [config_file,Path(config['runtimeInputs']),Path(config['defaultPackageConfig']),key]}
    sources={str(p):sha(p) for p in [source,manifest,tests,ROOT/'frame_manifest.py',Path(__file__),sequence,sequence_test,window_source]}
    env=dict(os.environ,TEMP=str(out/'temp'),TMP=str(out/'temp'),TMPDIR=str(out/'temp'),
        JAVA_OPTS='',_JAVA_OPTIONS='',JDK_JAVA_OPTIONS='',
        JAVA_TOOL_OPTIONS='-Duser.home="'+(out/'java-user').as_posix()+'" -Djava.io.tmpdir="'+(out/'temp').as_posix()+'"')
    steps=[]
    record=dict(passed=False,scope='isolated-hardware-frame-diagnostic-APK-generation',productInstalled=False,
        sdkDaemonsStarted=0,nativeJourneysStarted=0,nativeFramesAccepted=False,applicationOnlyLatencyAccepted=False,
        physicalDisplayPresentationProven=False,productionSigningUsed=False,fullRF312Accepted=False)
    def frozen():
        if config_file.read_bytes()!=raw:raise ValueError('Frame configuration changed')
        if any(sha(p)!=v for p,v in {**inputs,**sources}.items()):raise ValueError('Frame inputs or producer sources changed')
        if any(sha(path)!=config['toolSha256'][name] for name,path in tools.items()):raise ValueError('Frame tool changed')
        if source_hashes()!=config['sourceSha256']:raise ValueError('Frame builder or inherited runtime source changed')
    def run(label,command,*,timeout=120,environment=env):
        frozen();command=list(map(str,command));save(out/(label+'.command.json'),command)
        result=subprocess.run(command,cwd=out,env=environment,stdin=subprocess.DEVNULL,capture_output=True,
            timeout=timeout,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        (out/(label+'.stdout')).write_bytes(result.stdout);(out/(label+'.stderr')).write_bytes(result.stderr)
        steps.append(dict(step=label,exitCode=result.returncode,stdoutSha256=sha(out/(label+'.stdout')),stderrSha256=sha(out/(label+'.stderr'))))
        if result.returncode:raise RuntimeError('Frame builder step rejected: '+label)
        frozen();return result.stdout
    try:
        producer.package.check_certificate(run('baseline-signature',[tools['java'],'-jar',tools['apksigner'],
            'verify','--verbose','--print-certs',runtime['release']['apk']['path']]).decode(),signer['certificateSha256'])
        run('javac',[tools['javac'],'--release','11','-encoding','UTF-8','-cp',tools['androidJar'],'-d',out/'classes',source,sequence,window_source])
        classes={p.relative_to(out/'classes').as_posix():dict(bytes=p.stat().st_size,sha256=sha(p)) for p in sorted((out/'classes').rglob('*.class'))}
        if set(classes)!={'org/solosoul/rf312frames/FrameCollector.class','org/solosoul/rf312frames/FrameCollector$Fence.class',
                'org/solosoul/rf312frames/FrameCommitSequence.class','org/solosoul/rf312frames/FrameCommitSequence$Ticket.class',
                'org/solosoul/rf312frames/WindowFrameTrace.class','org/solosoul/rf312frames/WindowFrameTrace$TraceThread.class'}:
            raise ValueError('Exact compiled frame actor classes required')
        run('sequence-test-compile',[tools['javac'],'--release','11','-encoding','UTF-8',
            '-cp',out/'classes','-d',out/'test-classes',sequence_test])
        actual=run('sequence-tests',[tools['java'],'-cp',str(out/'classes')+os.pathsep+str(out/'test-classes'),
            'org.solosoul.rf312frames.FrameCommitSequenceTest'])
        if actual.strip()!=b'FrameCommitSequence: 11 tests passed':raise ValueError('All frame order regressions required')
        expression=re.search(r'private static final String STATE_SCRIPT = (.*?);\n\n',source.read_text(encoding='utf-8'),re.S).group(1)
        strings=re.findall(r'"(?:[^"\\]|\\.)*"',expression)
        if re.fullmatch(r'[\s+]*',re.sub(r'"(?:[^"\\]|\\.)*"','',expression)) is None:
            raise ValueError('Literal compiled frame readiness script required')
        script=out/'login-state.js'
        with script.open('x',encoding='utf-8',newline='\n') as stream:stream.write(''.join(json.loads(s) for s in strings)+'\n')
        stdout=run('readiness-tests',[tools['node'],'--test','--test-reporter=tap',tests],timeout=30,
            environment=dict(env,RF312_FRAME_STATE_SCRIPT=str(script)))
        counters={k.decode():int(v) for k,v in re.findall(rb'^# (tests|pass|fail|cancelled|skipped|todo) ([0-9]+)\s*$',stdout,re.M)}
        if (out/'readiness-tests.stderr').read_bytes() or counters!={'tests':9,'pass':9,'fail':0,'cancelled':0,'skipped':0,'todo':0}:
            raise ValueError('All compiled readiness tests must pass without skips')
        jar=out/'frame-classes.jar'
        with zipfile.ZipFile(jar,'x') as archive:
            for relative in classes:
                info=zipfile.ZipInfo(relative,(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
                archive.writestr(info,(out/'classes'/relative).read_bytes())
        run('d8',[tools['java'],'-cp',tools['d8'],'com.android.tools.r8.D8','--lib',tools['androidJar'],
            '--min-api','29','--output',out/'dex',jar])
        if [p.name for p in (out/'dex').iterdir()]!=['classes.dex']:raise ValueError('One frame actor DEX required')
        unsigned=out/'frame-unsigned.apk'
        run('aapt2',[tools['aapt2'],'link','-o',unsigned,'--manifest',manifest,'-I',tools['androidJar'],
            '--min-sdk-version','29','--target-sdk-version','35','--version-code','1','--version-name','0.0.1'])
        with zipfile.ZipFile(unsigned,'a') as archive:
            names=archive.namelist()
            if len(names)!=len(set(names)) or set(names)-{'AndroidManifest.xml','resources.arsc'} or 'AndroidManifest.xml' not in names:
                raise ValueError('Compiled frame resource payload rejected')
            info=zipfile.ZipInfo('classes.dex',(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
            archive.writestr(info,(out/'dex/classes.dex').read_bytes())
        aligned=out/'frame-aligned.apk';run('align',[tools['zipalign'],'4',unsigned,aligned])
        apk=out/'native-frame-collector.apk'
        run('sign',[tools['java'],'-jar',tools['apksigner'],'sign','--ks',key,'--ks-key-alias','androiddebugkey',
            '--ks-pass','pass:android','--key-pass','pass:android','--v1-signing-enabled','false','--v2-signing-enabled','true',
            '--v3-signing-enabled','true','--v4-signing-enabled','false','--out',apk,aligned])
        producer.package.check_certificate(run('verify',[tools['java'],'-jar',tools['apksigner'],
            'verify','--verbose','--print-certs',apk]).decode(),signer['certificateSha256'])
        run('signed-alignment',[tools['zipalign'],'-c','4',apk])
        identity=compiled_identity(run('manifest',[tools['aapt2'],'dump','xmltree',apk,'--file','AndroidManifest.xml']).decode())
        with zipfile.ZipFile(aligned) as original,zipfile.ZipFile(apk) as signed:
            if (original.namelist()!=signed.namelist() or len(signed.namelist())!=3
                    or set(signed.namelist())!={'AndroidManifest.xml','resources.arsc','classes.dex'}
                    or any(original.read(n)!=signed.read(n) for n in signed.namelist())):
                raise ValueError('Signing changed or added frame payload')
            payload={n:hashlib.sha256(signed.read(n)).hexdigest() for n in signed.namelist()}
        if payload['classes.dex']!=sha(out/'dex/classes.dex'):raise ValueError('Final frame DEX changed')
        frozen()
        record.update(passed=True,apk=str(apk),bytes=apk.stat().st_size,sha256=sha(apk),payloadSha256=payload,
            certificateSha256=signer['certificateSha256'],sourceSha256=sources,toolSha256=config['toolSha256'],
            compiledClasses=classes,readinessScriptSha256=sha(script),readinessTests=9,frameSequenceTests=11,
            compiledManifestIdentity=identity,minSdk=29,targetSdk=35,compileSdk=36,originalDefaultObserverReaderRejected=True,
            defaultTargetApkSha256=runtime['release']['apk']['sha256'],sourceAndToolsPreserved=True)
        record['builderConfigSha256']=sha(config_file)
        record['builderSourceBindings']=config['sourceSha256']
    except BaseException as error:
        import traceback
        (out/'failure.traceback').write_text(traceback.format_exc(),encoding='utf-8')
        record.update(errorType=type(error).__name__,error=str(error))
    record['steps']=steps;save(out/'package.json',record)
    print(json.dumps({k:record.get(k) for k in ['passed','sha256','bytes','readinessTests','nativeFramesAccepted','fullRF312Accepted']}))
    return 0 if record['passed'] else 1

if __name__=='__main__':sys.exit(main())
