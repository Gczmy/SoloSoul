"""Run a two-size native startup frame queue in a fresh, explicitly prepared SDK environment.

Only public fixtures are used. Original package, private data and settings are
restored; this target-instrumented run is not a default performance sample.
"""
import argparse
import datetime
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys
import time
import traceback
import uuid
import xml.etree.ElementTree as ET

ROOT=Path(__file__).resolve().parent
import frame_binding as diagnostic_binding
PUBLIC=diagnostic_binding.PUBLIC
WORK=diagnostic_binding.WORK
HELPER=diagnostic_binding.HELPER
sys.path[:0]=[str(HELPER),str(PUBLIC)]
from runtime_inputs import load_inputs,decode,digest
from prepare_avd import inputs as avd_inputs,PROFILE_SHA256
from owner_inputs import avd_inventory
from runtime_owner import fresh_host_audit
from bootstrap_quiet import wait_for_quiet
from rf312_launch_identity import wait_for_roots
from host_release import ReleaseState,wait_for_release,windows_capture
import frame_receipt

APP='com.solosoul.app';COLLECTOR='org.solosoul.rf312frames'
COMPONENT=COLLECTOR+'/'+COLLECTOR+'.FrameCollector'
load=lambda path:decode(Path(path).read_bytes())


def save(path,value):
    with Path(path).open('x',encoding='utf-8',newline='\n') as stream:
        json.dump(value,stream,indent=2,allow_nan=False);stream.write('\n')


def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    result=importlib.util.module_from_spec(spec);spec.loader.exec_module(result)
    return result


def package_identity(payload,info):
    uid=re.findall(r'^\s*userId=(\d+)\s*$',info,re.M)
    flags=re.findall(r'^\s*pkgFlags=\[([^\]]*)\]',info,re.M)
    if len(uid)!=1 or len(flags)!=1 or int(uid[0])<10000:
        raise ValueError('Unambiguous native application UID and package flags required')
    return dict(sha256=hashlib.sha256(payload).hexdigest(),uid=int(uid[0]),debuggable='DEBUGGABLE' in flags[0].split())


def main():
    diagnostic_binding.require_assertions()
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--kind',choices=['native-frames'],required=True)
    parser.add_argument('--preparation',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--diagnostic-inputs',type=Path,required=True)
    parser.add_argument('--powershell',type=Path,required=True)
    parser.add_argument('--powershell-sha256',required=True)
    args=parser.parse_args()
    diagnostic=diagnostic_binding.load(args.diagnostic_inputs)
    runtime_file=Path(diagnostic['config']['runtimeInputs'])
    runtime=load_inputs(runtime_file)
    collector_file=Path(diagnostic['config']['collectorPackageReceipt']);collector=load(collector_file)
    assert collector['passed'] and collector['compiledManifestIdentity']['instrumentationTarget']==APP
    assert digest(collector['apk'])==collector['sha256']
    assert diagnostic['runtime']==runtime and diagnostic['collector']==collector
    apk=Path(runtime['release']['apk']['path']);apk_hash=runtime['release']['apk']['sha256']
    assert diagnostic['package']['defaultFeaturesOnly'] is True
    assert diagnostic['package']['sha256']==apk_hash
    assert digest(apk)==apk_hash
    preparation_file=args.preparation.absolute()
    preparation=load(preparation_file);avd_config_file=preparation_file.parent/'config.json'
    avd_config=load(avd_config_file)
    sdk,jdk,tools,_,_=avd_inputs(avd_config,ROOT/'admission-path-not-created')
    assert preparation['passed'] and preparation['emulatorStarted'] is False and preparation['freshSdkUserdata']
    avd_dir=Path(preparation['avdDirectory']);config=avd_dir/'config.ini'
    assert digest(config)==PROFILE_SHA256
    assert not any(p.name.endswith('.lock') or p.name in ('hardware-qemu.ini','userdata-qemu.img','userdata-qemu.img.qcow2') for p in avd_dir.iterdir())
    fresh_inventory=avd_inventory(avd_dir)
    name=preparation['name'];serial='emulator-5578';adb=sdk/'platform-tools/adb.exe'
    assert digest(adb)==runtime['host']['adbSha256']
    pwsh=diagnostic_binding.producer.ordinary(args.powershell)
    diagnostic_binding.producer.command_path(pwsh)
    diagnostic_binding.producer.package.producer.hash_value(args.powershell_sha256)
    if pwsh.name!='pwsh.exe' or digest(pwsh)!=args.powershell_sha256:raise ValueError('Exact explicit PowerShell identity required')
    host_root=Path(runtime['host']['workspaceRoot'])
    host=module('rf312_ipc_host_identity',host_root/'tauri/scripts/android-host-audit.py')
    fresh=module('rf312_ipc_fresh_identity',host_root/'tauri/scripts/android-fresh-host-audit.py')
    restore=module('rf312_ipc_empty_restore',host_root/'tauri/scripts/android-home-native-regression.py')
    owner=dict(runtime=dict(host=dict(workspaceRoot=str(host_root),avd=name)),config=dict(powershell=str(pwsh)))
    frozen={str(p):digest(p) for p in [Path(__file__),runtime_file,collector_file,preparation_file,avd_config_file,
             config,adb,pwsh,host_root/'tauri/scripts/android-host-audit.py',host_root/'tauri/scripts/android-fresh-host-audit.py',
             host_root/'tauri/scripts/android-home-native-regression.py',ROOT/'frame_receipt.py',
             Path(collector['apk']),apk]}
    frozen.update({str(HELPER/p):digest(HELPER/p) for p in ['runtime_owner.py','owner_inputs.py','bootstrap_quiet.py','host_release.py']})
    frozen.update({str(ROOT/name):digest(ROOT/name) for name in diagnostic_binding.LOCAL})
    frozen[str(args.diagnostic_inputs.absolute())]=digest(args.diagnostic_inputs)
    source_binding=diagnostic_binding.sources()
    for rel in ['rf312_launch_identity.py','test_rf312_launch_identity.py','rf312_instrumentation_state.py','test_rf312_instrumentation_state.py']:
        frozen[str(WORK/'docs/performance'/rel)]=digest(WORK/'docs/performance'/rel)
    source_head=subprocess.check_output(['git','-C',str(WORK),'rev-parse','HEAD']).decode().strip()
    assert name==diagnostic['config']['avd']
    assert adb==Path(diagnostic['host']['adbExecutable']) and digest(adb)==diagnostic['host']['adbSha256']
    out=args.output.absolute();assert not out.exists();out.mkdir(exist_ok=False)
    env=dict(os.environ,ANDROID_HOME=str(sdk),ANDROID_SDK_ROOT=str(sdk),ANDROID_AVD_HOME=str(preparation_file.parent/'avd'),
        ANDROID_USER_HOME=str(preparation_file.parent/'android-user'),ANDROID_EMULATOR_HOME=str(preparation_file.parent/'android-user'),
        ANDROID_ADB_SERVER_PORT='5057',ADB_SERVER_SOCKET='tcp:127.0.0.1:5057',TEMP=str(out),TMP=str(out),PYTHONDONTWRITEBYTECODE='1',PYTHONPATH=os.pathsep.join([str(ROOT),str(HELPER),str(diagnostic_binding.LEGACY),str(PUBLIC)]))
    record=dict(passed=False,scope='explicit-default-apk-two-size-native-startup-frame-commits',kind=args.kind,
        targetInstrumentationInjected=True,metadataJavaScriptEvaluation=True,physicalPublicUiDispatched=False,
        actualIpcAccepted=False,fullJourneyAccepted=False,performanceSampleAccepted=False,fullRF312Accepted=False,
        frozenInputs=frozen,defaultReadersChanged=False,commands=[])
    save(out/'intent.json',record);save(out/'fresh-avd-inventory.json',fresh_inventory)
    server=emulator=instrument=None;roots={};identified=False;installed=False;backup=None;before=None
    original=None;settings_before=None;remote_root=None;sequence=0;exit_deadline=None;restored=False

    def processes(timeout=15):
        command='[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); Get-CimInstance Win32_Process | Select-Object Name,ProcessId,ParentProcessId,CreationDate,ExecutablePath,CommandLine | ConvertTo-Json -Depth 3'
        value=host.decode_json(subprocess.check_output([str(pwsh),'-NoProfile','-NonInteractive','-Command',command],timeout=timeout,creationflags=subprocess.CREATE_NO_WINDOW))
        return value if isinstance(value,list) else [value]

    def call(label,argv,*,device=True,required=True,timeout=30,**kwargs):
        nonlocal sequence
        sequence+=1
        command=[str(adb),'-H','127.0.0.1','-P','5057']+(['-s',serial] if device else [])+list(argv)
        result=subprocess.run(command,cwd=out,env=env,stdin=subprocess.DEVNULL,capture_output=True,timeout=timeout,creationflags=subprocess.CREATE_NO_WINDOW)
        prefix=out/f'{sequence:04d}-{label}'
        prefix.with_suffix('.stdout').write_bytes(result.stdout);prefix.with_suffix('.stderr').write_bytes(result.stderr)
        record['commands'].append(dict(label=label,exitCode=result.returncode))
        if required:result.check_returncode()
        return result

    def read(*argv):return call('read',argv).stdout.decode().strip()

    def restore_adb(*argv,**kwargs):return call('restore',argv,timeout=120)

    def archive(path):path.write_bytes(call('archive-private',['exec-out','run-as',APP,'tar','-cf','-','.'],timeout=120).stdout)

    def settings():
        result={}
        for namespace,key in [('secure','accessibility_enabled'),('secure','enabled_accessibility_services'),
                ('global','disable_window_blurs'),('secure','high_text_contrast_enabled'),('system','font_scale'),
                ('global','animator_duration_scale'),('global','transition_animation_scale'),('global','window_animation_scale')]:
            result[namespace+'.'+key]=read('shell','settings','get',namespace,key)
        result['night']=read('shell','cmd','uimode','night');return result

    def installed_identity(package):
        path=read('shell','pm','path',package)
        assert re.fullmatch(r'package:/data/app/(?:[A-Za-z0-9_~+=.-]+/){1,2}base\.apk',path)
        payload=call('installed-apk',['exec-out','cat',path.removeprefix('package:')],timeout=120).stdout
        info=read('shell','dumpsys','package',package)
        return package_identity(payload,info)

    def remote_receipt(name,deadline):
        assert re.fullmatch(r'(?:ready|phase-001)\.json',name)
        while time.monotonic()<deadline:
            marker=call('metadata-marker',['exec-out','cat',remote_root+'/'+name+'.sha256'],required=False,timeout=5)
            if marker.returncode==0 and re.fullmatch(b'[a-f0-9]{64}\n?',marker.stdout):
                raw=call('metadata-read',['exec-out','cat',remote_root+'/'+name],timeout=5).stdout
                assert hashlib.sha256(raw).hexdigest()==marker.stdout.decode().strip()
                (out/name).write_bytes(raw);return raw
            if instrument.poll() is not None:raise RuntimeError('Collector ended before its expected metadata receipt')
            time.sleep(.1)
        raise TimeoutError('Bounded collector receipt read expired')

    try:
        for port in [5057,5578,5579]:
            with socket.socket() as sock:sock.bind(('127.0.0.1',port))
        fresh_host_audit(owner,'before',out,[])
        server_cmd=[str(adb),'-L','tcp:localhost:5057','server','nodaemon']
        server_floor=datetime.datetime.now(datetime.timezone.utc).isoformat()
        server_log=(out/'adb-server.log').open('xb')
        server=subprocess.Popen(server_cmd,cwd=out,
            env=dict(env,ADB_SERVER_SOCKET='tcp:localhost:5057'),stdin=subprocess.DEVNULL,stdout=server_log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
        time.sleep(1);assert server.poll() is None
        assert call('empty-devices',['devices'],device=False).stdout.decode().strip()=='List of devices attached'
        assert avd_inventory(avd_dir)==fresh_inventory
        command=[str(tools['emulatorExecutable']),'-avd',name,'-port','5578','-accel','on','-gpu','swiftshader',
                 '-memory','2048','-cores','2','-no-window','-no-snapshot','-no-audio','-no-boot-anim']
        emulator_floor=datetime.datetime.now(datetime.timezone.utc).isoformat()
        deadline=time.monotonic()+240
        log=(out/'emulator.log').open('xb')
        emulator=subprocess.Popen(command,cwd=out,env=env,stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
        expected=[dict(Name=kind,ProcessId=process.pid,ParentProcessId=os.getpid(),ExecutablePath=cmd[0],
                       CommandLine=subprocess.list2cmdline(cmd),BornNotBefore=floor)
            for kind,process,cmd,floor in [('adb.exe',server,server_cmd,server_floor),('emulator.exe',emulator,command,emulator_floor)]]
        save(out/'expected-launch-identities.json',expected)
        save(out/'live.json',dict(server=server.pid,emulator=emulator.pid,originalBootDeadlineMonotonic=deadline))
        initial=wait_for_roots(processes,expected,lambda:server.poll() is None and emulator.poll() is None,deadline,
                              lambda number,row:save(out/('identity-'+str(number)+'.json'),row))
        roots.update({row['ProcessId']:row for row in initial});save(out/'host-roots.json',initial)
        print('Fresh forensic AVD boot started with complete actual SDK roots; original boot deadline retained',flush=True)
        while time.monotonic()<deadline:
            response=call('boot',['shell','getprop','sys.boot_completed'],required=False,timeout=min(10,max(.001,deadline-time.monotonic())))
            if response.returncode==0 and response.stdout.strip()==b'1':break
            time.sleep(min(3,max(0,deadline-time.monotonic())))
        else:raise TimeoutError('Original 240-second boot budget exhausted')
        assert read('emu','avd','name').splitlines()[0]==name;identified=True
        assert read('shell','getprop','ro.build.version.sdk')=='31' and read('shell','getprop','ro.product.cpu.abi')=='x86_64'
        assert read('shell','cat','/sys/devices/system/cpu/online')=='0-1'
        assert digest(config)==PROFILE_SHA256
        rows=wait_for_quiet(processes,next(v for v in initial if v['ProcessId']==server.pid),
             next(v for v in initial if v['ProcessId']==emulator.pid),deadline,
             lambda number,row:save(out/('quiet-'+str(number)+'.json'),row))
        for row in rows:
            if row['Name'].startswith('qemu-system-') and row['ParentProcessId']==emulator.pid:
                assert Path(row['ExecutablePath']).is_relative_to(sdk/'emulator') and name in row['CommandLine']
                roots[row['ProcessId']]=row
        assert len(roots)==3
        screenshot=call('preflight-screenshot',['exec-out','screencap','-p']).stdout
        assert screenshot.startswith(b'\x89PNG\r\n\x1a\n');(out/'preflight.png').write_bytes(screenshot)
        call('preflight-window',['shell','dumpsys','window']);call('preflight-anr',['shell','dumpsys','activity','lastanr'])
        call('preflight-dropbox',['shell','dumpsys','dropbox'],timeout=45)
        save(out/'preflight-ready.json',dict(pngSha256=digest(out/'preflight.png'),nativeTestsStarted=0))
        print('Functional preflight captured; waiting for local screenshot and health review',flush=True)
        deadline=time.monotonic()+600
        while not (out/'continue-native.json').is_file():
            if (out/'reject-native.json').exists():raise RuntimeError('Functional boot review rejected')
            assert time.monotonic()<deadline;time.sleep(1)
        review=load(out/'continue-native.json')
        assert review['pngSha256']==digest(out/'preflight.png') and review['screenInspected'] and review['healthReviewed']
        for package in [APP,APP+'.test','org.solosoul.rf312observer',COLLECTOR,APP+'.releaseprobe']:
            result=call('absent-package',['shell','pm','path',package],required=False)
            assert result.returncode in (0,1) and not result.stdout.strip() and not result.stderr.strip()
        settings_before=settings()
        debug=runtime['originalDebug'];call('install-original-debug',['install','-t',debug['path']],timeout=120);installed=True
        original=installed_identity(APP);assert original['sha256']==debug['sha256'] and original['debuggable']
        call('prewarm',['shell','am','start','-W','-n',APP+'/.MainActivity'],timeout=45)
        call('prewarm-stop',['shell','am','force-stop',APP])
        root_mode=read('shell','run-as',APP,'stat','-c','%a','.')
        backup=out/'private-data-before.tar';archive(backup);before=restore.inventory(backup,require_empty=True)
        record['backup']=dict(path=str(backup),sha256=digest(backup),entries=len(before))
        call('install-original-test',['install','-t',runtime['originalTest']['path']],timeout=120)
        assert installed_identity(APP+'.test')['sha256']==runtime['originalTest']['sha256']
        record['outcomes']=[]
        for count in [100,5000]:
            child_out=out/('journey-'+str(count))
            command=[sys.executable,'-B',str(ROOT/'frame_driver.py'),
                '--adb',str(adb),'--serial',serial,'--avd',name,'--apk',str(apk),
                '--restore-apk',runtime['originalDebug']['copy'],'--test-apk',runtime['runtimeTest']['path'],
                '--fixture',runtime['fixedFixtures'][str(count)]['root'],'--inputs',str(runtime_file),
                '--diagnostic-inputs',str(args.diagnostic_inputs),'--object-count',str(count),'--mode','supported','--output',str(child_out)]
            started=time.monotonic()
            with (out/('journey-'+str(count)+'.stdout')).open('xb') as stdout,(out/('journey-'+str(count)+'.stderr')).open('xb') as stderr:
                child=subprocess.Popen(command,cwd=WORK,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
                save(out/('journey-'+str(count)+'.live.json'),dict(pid=child.pid,command=command))
                code=child.wait()
            row=dict(count=count,exitCode=code,passed=False,elapsedSeconds=time.monotonic()-started)
            report=load(child_out/'report.json')
            if code!=0 or report.get('passed') is not True:
                raise RuntimeError('Diagnostic child rejected: '+str(report.get('errorType'))+': '+str(report.get('error')))
            before_child=restore.inventory(child_out/'private-data-before.tar',require_empty=True)
            after_child=restore.inventory(child_out/'private-data-restored.tar',require_empty=True)
            assert before_child==after_child and report.get('restored') is True
            assert report.get('originalApksRestored') is True and report.get('blurRestored') is True
            assert report.get('ownedProbeRemoved') is True and report.get('ownedPublicOcrInputRemoved') is True
            assert report.get('frozenInputsUnchanged') is True and not report.get('restoreError')
            assert code==0 and report['passed'] is True and report['nativeFramesAccepted'] is True
            assert report['fullRF312Accepted'] is False and report['defaultPerformanceSampleAccepted'] is False
            evidence=load(child_out/'default-external/evidence.json')
            contract=load(child_out/'target-instrumentation/metadata-contract.json')
            assert evidence['objectCount']==count and evidence['stageNames']==['startup']
            assert evidence['fullUiJourneyAccepted'] is False and evidence['targetInstrumentationInjected'] is True
            assert evidence['supportedNativeInstrumentationStateVerified'] is True
            assert evidence['nativeInstrumentationStateChecks']>3
            assert evidence['metadataJavaScriptEvaluation'] is True and contract['frameCommitContractAccepted'] is True
            assert contract['committedFrames']==2 and contract['nativeRuntimeAccepted'] is False
            assert contract['physicalDisplayPresentationProven'] is False and contract['applicationOnlyLatencyAccepted'] is False
            for folder in ['target-instrumentation','independent-observer','persistent-control']:
                assert load(child_out/folder/'cleanup.json')['accepted'] is True
            assert settings()==settings_before
            assert installed_identity(APP)==original
            assert installed_identity(APP+'.test')['sha256']==runtime['originalTest']['sha256']
            row.update(passed=True,privateEntries=len(before_child),committedFrames=2,
                       startupUiAndNativeFrameMetadataAccepted=True,originalStateRestored=True)
            record['outcomes'].append(row)
            save(out/('journey-'+str(count)+'-audit.json'),row)
            print(json.dumps(dict(objectCount=count,passed=True,committedFrames=2)),flush=True)
        call('terminal-full-dropbox',['shell','dumpsys','dropbox'],timeout=45)
        record['functionalStepPassed']=len(record['outcomes'])==2
        record['fullJourneyAccepted']=False

    except Exception as error:
        record.update(errorType=type(error).__name__,error=str(error))
        try:
            if (out/'ready.json').is_file():
                observed=load(out/'ready.json')['process']['processId']
                assert type(observed) is int and observed>0
                call('failure-app-logcat',['shell','logcat','-d','--pid='+str(observed)],required=False,timeout=10)
        except Exception as diagnostic_error:record['failureLogcatErrorType']=type(diagnostic_error).__name__
        (out/'failure.traceback').write_text(traceback.format_exc(),encoding='utf-8')
        print(type(error).__name__+': '+str(error),flush=True)
    finally:
        cleanup={}
        if not identified and emulator is not None:
            try:
                identified=read('emu','avd','name').splitlines()[0]==name
                cleanup['consoleIdentityRecoveredAfterBootFailure']=identified
            except Exception as error:cleanup['consoleIdentityRecoveryError']=repr(error)
        if identified:
            try:
                assert read('emu','avd','name').splitlines()[0]==name
                cleanup_rows=processes()
                observed_launcher=[row for row in cleanup_rows if row['ProcessId']==emulator.pid]
                assert len(observed_launcher)==1 and host.same_identity(observed_launcher[0],roots[emulator.pid])
                owned_vms=[row for row in cleanup_rows if row['Name']=='qemu-system-x86_64-headless.exe'
                           and row['ParentProcessId']==emulator.pid]
                assert len(owned_vms)==1
                owned_vm=owned_vms[0]
                assert Path(owned_vm['ExecutablePath']).is_relative_to(sdk/'emulator/qemu')
                assert re.search(r'(?:^|\s)-avd\s+"?'+re.escape(name)+r'"?(?:\s|$)',owned_vm['CommandLine'])
                assert host.birth(owned_vm['CreationDate'])>=host.birth(observed_launcher[0]['CreationDate'])
                if owned_vm['ProcessId'] in roots:assert host.same_identity(owned_vm,roots[owned_vm['ProcessId']])
                roots[owned_vm['ProcessId']]=owned_vm
                save(out/'cleanup-host-roots.json',list(roots.values()))
                if installed:call('stop-target',['shell','am','force-stop',APP])
                if instrument is not None:
                    instrument.wait(timeout=15)
                if before is not None:
                    call('restore-debug-apk',['install','-r',runtime['originalDebug']['path']],timeout=120)
                    call('restore-stop',['shell','am','force-stop',APP])
                    assert installed_identity(APP)==original
                    restore.restore_private_data(restore_adb,read,archive,backup,before,root_mode,out,record)
                    restored=True
                if installed and (before is None or restored):
                    existing=call('remaining-collector',['shell','pm','path',COLLECTOR],required=False)
                    assert existing.returncode in (0,1) and not existing.stderr.strip()
                    if existing.stdout.strip():
                        assert installed_identity(COLLECTOR)['sha256']==collector['sha256']
                        assert call('remove-collector',['uninstall',COLLECTOR]).stdout.strip()==b'Success'
                    assert installed_identity(APP+'.test')['sha256']==runtime['originalTest']['sha256']
                    assert call('remove-original-test',['uninstall',APP+'.test']).stdout.strip()==b'Success'
                    assert call('remove-main',['uninstall',APP]).stdout.strip()==b'Success'
                    cleanup['ownedPackagesRemoved']=True
                if settings_before is not None:
                    cleanup['settingsExactlyUnchanged']=settings()==settings_before
                if installed:call('terminal-health',['shell','dumpsys','activity','lastanr'])
            except Exception as error:cleanup['guestCleanupError']=repr(error)
            try:
                assert read('emu','avd','name').splitlines()[0]==name
                call('emulator-stop',['emu','kill']);exit_deadline=time.monotonic()+45
                cleanup['normalShutdownRequested']=True
            except Exception as error:cleanup['guestShutdownError']=repr(error)
        if emulator is not None:
            try:emulator.wait(timeout=max(.001,(exit_deadline or (time.monotonic()+45))-time.monotonic()));cleanup['launcherExited']=True
            except subprocess.TimeoutExpired:cleanup['launcherExited']=False
        if server is not None and server.poll() is None:
            try:
                call('server-stop',['kill-server'],device=False);server.wait(timeout=15);cleanup['serverExited']=True
            except Exception as error:cleanup['serverCleanupError']=repr(error)
        try:
            if exit_deadline is None:raise RuntimeError('Original shutdown deadline required')
            audit_script=host_root/'tauri/scripts/android-fresh-host-audit.py'
            ordinal=0
            while time.monotonic()<exit_deadline:
                ordinal+=1;remaining=exit_deadline-time.monotonic()
                label='host-after-'+str(ordinal).zfill(3)
                spec=dict(schemaVersion=1,baseline='no-existing-adb-or-emulator',ownedProcesses=list(roots.values()),
                          protectedPorts=[5037,5049],ownedPorts=[5057,5578,5579],avd=name)
                expected=out/(label+'.expected.json');save(expected,spec)
                audit=subprocess.run([sys.executable,'-B',str(audit_script),'after','--powershell',str(pwsh),
                    '--expected',str(expected),'--output',str(out/label),'--baseline',str(out/'fresh-host-before')],
                    stdin=subprocess.DEVNULL,capture_output=True,timeout=min(15,remaining),creationflags=subprocess.CREATE_NO_WINDOW)
                (out/(label+'.stdout')).write_bytes(audit.stdout);(out/(label+'.stderr')).write_bytes(audit.stderr)
                if audit.returncode==0 and load(out/label/'result.json')['passed'] is True:
                    if time.monotonic()>exit_deadline:raise TimeoutError('After audit outside original shutdown deadline')
                    cleanup['freshHostAfterPassed']=True;cleanup['hostAfterReceipt']=label+'/result.json';break
                time.sleep(min(.5,max(0,exit_deadline-time.monotonic())))
            if not cleanup.get('freshHostAfterPassed'):raise TimeoutError('Strict host zero audit did not pass within original shutdown deadline')
        except Exception as error:cleanup['hostCleanupError']=repr(error)
        record['cleanup']=cleanup
        try:
            assert all(digest(path)==expected for path,expected in frozen.items())
            assert load_inputs(runtime_file)==runtime
            assert diagnostic_binding.sources()==source_binding
            diagnostic_binding.verify(diagnostic)
            assert subprocess.check_output(['git','-C',str(WORK),'rev-parse','HEAD']).decode().strip()==source_head
            record['allFrozenInputsPreserved']=True
        except Exception as error:record['frozenRecheckError']=repr(error)
        record['passed']=bool(record.get('functionalStepPassed') and restored and record.get('allFrozenInputsPreserved')
            and all(cleanup.get(key) is True for key in ['ownedPackagesRemoved','settingsExactlyUnchanged','normalShutdownRequested',
                                                       'launcherExited','serverExited','freshHostAfterPassed']))
        record['nativeFramesAccepted']=record['passed']
        record['actualIpcAccepted']=False
        record['performanceSampleAccepted']=False
        record['fullRF312Accepted']=False
        save(out/'result.json',record)
        print(json.dumps({key:record.get(key) for key in ['passed','kind','nativeFramesAccepted','actualIpcAccepted','fullRF312Accepted','cleanup']}),flush=True)
    return 0 if record['passed'] else 1


if __name__=='__main__':sys.exit(main())
