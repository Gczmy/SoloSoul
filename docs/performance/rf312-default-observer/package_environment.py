"""Explicit tool configuration and isolated Gradle execution for default packages."""
import hashlib
import json
import os
from pathlib import Path
import re

from frontend_build import entry, files, validate_files
from prepare_avd import ordinary, command_path
from runtime_inputs import decode, digest, hash_value

TOOLS={'java','javac','jvm','aapt2','apksigner','androidJar'}


def gradle_version(project):
    rows=(project/'gradle/wrapper/gradle-wrapper.properties').read_text(encoding='utf-8').splitlines()
    values=[n.split('=',1)[1] for n in rows if n.startswith('distributionUrl=')]
    match=re.search(r'/gradle-([0-9]+(?:\.[0-9]+){1,2})-(?:bin|all)\.zip$',values[0]) if len(values)==1 else None
    if match is None:raise ValueError('One declared Gradle distribution version required')
    return match[1]


def tool_inputs(config,project):
    sdk=ordinary(config['sdkRoot'],directory=True);jdk=ordinary(config['jdkRoot'],directory=True)
    gradle=ordinary(config['gradleRoot'],directory=True);home=ordinary(config['gradleHome'],directory=True)
    if gradle.name!='gradle-'+gradle_version(project):raise ValueError('Gradle runtime must match the tracked wrapper version')
    if set(config['toolSha256'])!=TOOLS:raise ValueError('Complete explicit packaging tools required')
    tools=dict(java=jdk/'bin/java.exe',javac=jdk/'bin/javac.exe',jvm=jdk/'bin/server/jvm.dll',
        aapt2=sdk/'build-tools/35.0.0/aapt2.exe',apksigner=sdk/'build-tools/35.0.0/lib/apksigner.jar',
        androidJar=sdk/'platforms/android-36/android.jar')
    for name,path in tools.items():
        ordinary(path);command_path(path)
        if entry(path)['sha256']!=hash_value(config['toolSha256'][name]):raise ValueError('Declared packaging tool changed')
    validate_files(config['gradleFiles'])
    if files(gradle)!=config['gradleFiles']:raise ValueError('Complete declared Gradle distribution differs')
    command_path(ordinary(gradle/'bin/gradle.bat'))
    configuration={}
    for name in ('gradle.properties','init.gradle','init.gradle.kts'):
        path=home/name
        if path.exists() or path.is_symlink():configuration[name]=entry(path)
    init=home/'init.d'
    if init.exists() or init.is_symlink():
        ordinary(init,directory=True)
        for folder,dirs,names in os.walk(init,followlinks=False):
            for name in dirs:ordinary(Path(folder)/name,directory=True)
            for name in names:
                path=Path(folder)/name;configuration[path.relative_to(home).as_posix()]=entry(path)
    if configuration!=config['gradleHomeConfiguration']:raise ValueError('Declared Gradle home configuration differs')
    for root in (sdk,jdk,gradle,home):command_path(root)
    return dict(sdk=sdk,jdk=jdk,gradle=gradle,home=home,tools=tools,homeConfiguration=configuration)


def version_properties(payload):
    workspace=Path(payload['workspace']);root=workspace/'tauri'
    def product(name):
        key='tauri/src-tauri/'+name;path=root/'src-tauri'/name
        if entry(path)!=payload['sources'][key]:raise ValueError('Version source changed')
        return decode(path.read_bytes())
    app=product('tauri.conf.json');platform=root/'src-tauri/tauri.android.conf.json'
    if platform.exists():
        override=product('tauri.android.conf.json')
        def merge(a,b):
            result=dict(a)
            for k,v in b.items():
                if v is None:result.pop(k,None)
                elif isinstance(v,dict) and isinstance(result.get(k),dict):result[k]=merge(result[k],v)
                else:result[k]=v
            return result
        app=merge(app,override)
    if app['identifier']!='com.solosoul.app':raise ValueError('Current default Android package identity required')
    match=re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)',app['version'])
    if match is None:raise ValueError('Plain configured semantic version required')
    android=app.get('bundle',{}).get('android',{})
    if android.get('autoIncrementVersionCode',False) is not False:raise ValueError('Explicit reproducible version code required')
    plan=decode((Path(payload['receipt']).parent/'frontend-plan.json').read_bytes())
    for name in ('config.schema.json','package.json'):
        relative='@tauri-apps/cli/'+name;path=root/'node_modules'/relative
        if entry(path)!=plan['nodeModules']['files'][relative]:raise ValueError('Locked CLI version metadata changed')
    schema=decode((root/'node_modules/@tauri-apps/cli/config.schema.json').read_bytes())
    rule=schema['definitions']['AndroidConfig']['properties']['versionCode']['description']
    if 'version.major * 1000000 + version.minor * 1000 + version.patch' not in rule:
        raise ValueError('Recorded CLI version-code rule required')
    major,minor,patch=map(int,match.groups())
    code=android.get('versionCode',major*1000000+minor*1000+patch)
    if type(code) is not int or not 1<=code<=2100000000:raise ValueError('Valid explicit Android version code required')
    text='tauri.android.versionCode='+str(code)+'\ntauri.android.versionName='+app['version']+'\n'
    return dict(versionName=app['version'],versionCode=code,githubRunNumber=0,properties=text,
        cliVersion=decode((root/'node_modules/@tauri-apps/cli/package.json').read_bytes())['version'])


def gradle_command(cmd,script,args):
    for value in (cmd,script,*args):command_path(value)
    return '"'+str(cmd)+'" /d /s /c ""'+str(script)+'" '+ ' '.join('"'+str(n)+'"' for n in args)+'"'


def verify_external_artifact(row):
    if not isinstance(row,dict) or set(row)!={'path','bytes','sha256'} or not isinstance(row['path'],str):
        raise ValueError('Complete resolved Gradle dependency required')
    from frontend_build import validate_files
    expected={k:row[k] for k in ('bytes','sha256')};validate_files({'dependency':expected})
    path=Path(row['path'])
    if not path.is_absolute() or '..' in path.parts:raise ValueError('Absolute local dependency path required')
    if os.name=='nt':
        if re.fullmatch(r'[A-Za-z]:',path.drive) is None:raise ValueError('Plain local drive dependency path required')
        # Extended syntax avoids legacy MAX_PATH without changing global policy.
        # entry still binds file identity and rejects reparse parents and mutation.
        if len(str(path))>=260:path=Path(chr(92)*2+'?'+chr(92)+str(path))
    if entry(path)!=expected:raise ValueError('Resolved Gradle dependency changed')


def environment(prepared,output,original=None):
    original=dict(os.environ if original is None else original)
    blocked={'JAVA_OPTS','GRADLE_OPTS','_JAVA_OPTIONS','JDK_JAVA_OPTIONS','JAVA_TOOL_OPTIONS','GRADLE_INIT_SCRIPT'}
    env={k:v for k,v in original.items() if k not in blocked and not k.startswith(('SOLOSOUL_','TAURI_','WRY_'))}
    env.update(JAVA_HOME=str(prepared['jdk']),GRADLE_HOME=str(prepared['gradle']),GRADLE_USER_HOME=str(prepared['home']),
        ANDROID_HOME=str(prepared['sdk']),ANDROID_SDK_ROOT=str(prepared['sdk']),ANDROID_USER_HOME=str(output/'android-user'),
        TEMP=str(output/'temp'),TMP=str(output/'temp'),TMPDIR=str(output/'temp'),GITHUB_RUN_NUMBER='0',
        JAVA_OPTS='',GRADLE_OPTS='',_JAVA_OPTIONS='',JDK_JAVA_OPTIONS='',
        JAVA_TOOL_OPTIONS=f'-Duser.home="{output / "java-user"}" -Djava.io.tmpdir="{output / "temp"}"',
        PATH=str(prepared['jdk']/'bin')+os.pathsep+original.get('PATH',''))
    return env


def init_script(binding):
    literal="'"+str(binding).replace('\\','\\\\').replace("'","\\'")+"'"
    return '''import groovy.json.JsonOutput
import groovy.json.JsonSlurper
import java.security.MessageDigest
def input = new JsonSlurper().parse(new File(BINDING))
def sha = { File file ->
  def digest = MessageDigest.getInstance('SHA-256')
  file.withInputStream { stream -> byte[] bytes = new byte[65536]; int n
    while ((n = stream.read(bytes)) > 0) digest.update(bytes, 0, n)
  }
  digest.digest().encodeHex().toString()
}
gradle.settingsEvaluated { settings ->
  if (settings.rootDir.canonicalPath == new File(input.project).canonicalPath) {
    input.modules.each { name, path -> settings.project(':' + name).projectDir = new File(path) }
  }
}
gradle.beforeProject { project ->
  def scope = MessageDigest.getInstance('SHA-256').digest(project.projectDir.canonicalPath.getBytes('UTF-8')).encodeHex().toString()
  project.layout.buildDirectory.set(new File(input.buildRoot, scope))
  if (project.path == ':app' && project.rootDir.canonicalPath == new File(input.project).canonicalPath) {
    project.pluginManager.withPlugin('com.android.application') {
      def android = project.extensions.getByName('android')
      android.sourceSets.getByName('main').assets.setSrcDirs([new File(input.assets)])
      android.sourceSets.getByName('main').jniLibs.setSrcDirs([new File(input.jni)])
      project.extensions.getByName('androidComponents').finalizeDsl { dsl ->
        dsl.buildTypes.getByName('release').signingConfig = null
      }
      project.afterEvaluate {
        project.tasks.matching { it.name == 'compileX86_64ReleaseArtProfile' }.configureEach { task ->
          task.doLast {
            def profiles = [:]
            task.outputs.files.files.each { output ->
              def leaves = output.isDirectory() ? project.fileTree(output).files : [output]
              leaves.findAll { it.isFile() && it.name in ['baseline.prof', 'baseline.profm'] }.each { file ->
                if (!file.canonicalPath.startsWith(new File(input.buildRoot).canonicalPath + File.separator)) throw new GradleException('Foreign generated profile')
                if (profiles.containsKey(file.name) && profiles[file.name].path != file.canonicalPath) throw new GradleException('Duplicate generated profile')
                profiles[file.name] = [path:file.canonicalPath, bytes:file.length(), sha256:sha(file)]
              }
            }
            if (profiles.keySet() != (['baseline.prof','baseline.profm'] as Set)) throw new GradleException('Complete generated profile pair required')
            new File(input.profileProof).text = JsonOutput.prettyPrint(JsonOutput.toJson([task:task.path,files:profiles]))
          }
        }
      }
    }
  }
}
gradle.projectsEvaluated {
  if (gradle.rootProject.projectDir.canonicalPath == new File(input.project).canonicalPath) {
    def expected = [':':new File(input.project).canonicalPath, ':app':new File(input.project,'app').canonicalPath]
    input.modules.each { name,path -> expected[':' + name] = new File(path).canonicalPath }
    def actual = gradle.rootProject.allprojects.collectEntries { [(it.path):it.projectDir.canonicalPath] }
    if (actual != expected) throw new GradleException('Declared private project graph differs')
    if (new File(System.getProperty('java.home')).canonicalPath != new File(input.jdk).canonicalPath) throw new GradleException('Declared JDK differs')
    def artifacts = [:]
    def record = { String role, configuration ->
      def external = configuration.incoming.artifactView { view ->
        view.componentFilter { id -> id instanceof org.gradle.api.artifacts.component.ModuleComponentIdentifier }
      }
      external.artifacts.artifacts.each { artifact ->
        def component = artifact.id.componentIdentifier
        def file = artifact.file
        def key = role + '/' + component.displayName + '/' + artifact.id.displayName
        def value = [path:file.canonicalPath,bytes:file.length(),sha256:sha(file)]
        if (artifacts.containsKey(key) && artifacts[key] != value) throw new GradleException('Conflicting resolved artifact')
        artifacts[key] = value
      }
    }
    record(':app/runtime', gradle.rootProject.project(':app').configurations.getByName('x86_64ReleaseRuntimeClasspath'))
    gradle.rootProject.allprojects.each { project -> record(project.path + '/buildscript', project.buildscript.configurations.getByName('classpath')) }
    new File(input.runtimeProof).text = JsonOutput.prettyPrint(JsonOutput.toJson([javaVersion:System.getProperty('java.version'),javaHome:System.getProperty('java.home'),projects:actual,externalArtifacts:artifacts]))
  }
}
'''.replace('BINDING',literal)
