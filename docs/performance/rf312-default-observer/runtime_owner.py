"""Own a fresh default-APK runtime with explicit verified artifacts and SDK paths."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,re,socket,subprocess,sys,time,traceback
from owned_sdk_adb import is_owned_sdk_client
from owner_inputs import load_plan,verify_plan,verify_fresh_avd,ordinary
from host_release import ReleaseState,wait_for_release,windows_capture

SOURCE=Path(__file__).resolve().parent
load=lambda p:json.loads(Path(p).read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
save=lambda p,v:Path(p).write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8',newline='\n')

def fresh_host_audit(owner,phase,stage,roots):
    spec=dict(schemaVersion=1,baseline='no-existing-adb-or-emulator',ownedProcesses=roots,
        protectedPorts=[5037,5049],ownedPorts=[5057,5578,5579],avd=owner['runtime']['host']['avd'])
    expected=stage/('fresh-expected-'+phase+'.json');save(expected,spec)
    command=[sys.executable,str(Path(owner['runtime']['host']['workspaceRoot'])/'tauri/scripts/android-fresh-host-audit.py'),
        phase,'--powershell',owner['config']['powershell'],'--expected',str(expected),'--output',str(stage/('fresh-host-'+phase))]
    if phase=='after':command+=['--baseline',str(stage/'fresh-host-before')]
    result=subprocess.run(command,capture_output=True,timeout=45,creationflags=subprocess.CREATE_NO_WINDOW)
    (stage/('fresh-host-'+phase+'.stdout')).write_bytes(result.stdout);(stage/('fresh-host-'+phase+'.stderr')).write_bytes(result.stderr)
    result.check_returncode();assert load(stage/('fresh-host-'+phase)/'result.json')['passed']

def run(plan_path,output):
    plan_bytes=Path(plan_path).read_bytes();owner=load_plan(plan_path,artifacts=True);runtime=owner['runtime']
    verify_fresh_avd(owner)
    parent=ordinary(Path(output).absolute().parent,directory=True);stage=parent/Path(output).name;stage.mkdir(exist_ok=False)
    root=Path(runtime['host']['workspaceRoot']);sdk=Path(runtime['host']['sdkRoot'])
    proof=stage/'native-default-full-journey-r1';proof.mkdir(exist_ok=False)
    name,serial=runtime['host']['avd'],runtime['host']['serial']
    assert serial=='emulator-5578','The original dedicated port and observer contract is retained'
    config=Path(owner['avdConfig']);config_hash=sha(config)
    for port in [5057,5578,5579]:
        with socket.socket() as sock:sock.bind(('127.0.0.1',port))
    adb=sdk/'platform-tools/adb.exe';pwsh=owner['config']['powershell']
    spec=importlib.util.spec_from_file_location('runtime_canonical_host_identity',root/'tauri/scripts/android-host-audit.py')
    identity=importlib.util.module_from_spec(spec);spec.loader.exec_module(identity)
    env=dict(os.environ,ANDROID_HOME=str(sdk),ANDROID_SDK_ROOT=str(sdk),ANDROID_AVD_HOME=owner['avdRoot'],
        ANDROID_USER_HOME=owner['androidUserRoot'],ANDROID_EMULATOR_HOME=owner['androidUserRoot'],ANDROID_ADB_SERVER_PORT='5057',
        ADB_SERVER_SOCKET='tcp:127.0.0.1:5057',TEMP=str(stage),TMP=str(stage),PYTHONDONTWRITEBYTECODE='1')
    save(stage/'owner-binding.json',dict(planSha256=sha(plan_path),ownerSources=owner['ownerSourceFiles'],runtimeInputsSha256=owner['config']['runtimeInputsSha256']))
    record = dict(scope='current-default-release-independent-twelve-step-ui-journey', passed=False, oldPrivateServerTouched=False,
                  targetInstrumentationInjected=False, javaScriptInjected=False, commands=[])
    sequence = 0
    server = emulator = None
    identified = installed = mutation_started = False
    all_restored = False
    settings_before = None
    emulator_exit_deadline = None

    def host_processes(timeout=15):
        command = "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); Get-CimInstance Win32_Process | Select-Object Name,ProcessId,ParentProcessId,CreationDate,ExecutablePath,CommandLine | ConvertTo-Json -Depth 3"
        rows = identity.decode_json(subprocess.check_output([pwsh, '-NoProfile', '-NonInteractive', '-Command', command], timeout=timeout, creationflags=subprocess.CREATE_NO_WINDOW))
        return rows if isinstance(rows, list) else [rows]

    def protected_state(rows):
        baseline=load(stage/'fresh-host-baseline.json')
        assert baseline['mode']=='no-existing-adb-or-emulator'
        own_id=server.pid if server is not None else None
        foreign=[p for p in rows if p['Name']=='adb.exe' and p['ProcessId']!=own_id]
        recognized=[];unverified=[]
        for process in foreign:
            initial=proof/'initial-owned-host-roots.json'
            roots=load(initial) if initial.is_file() else []
            root=next((p for p in roots if emulator is not None and p['ProcessId']==emulator.pid),None)
            if root is not None and is_owned_sdk_client(process,rows,root,sdk,name,'emulator-5578'):recognized.append(process)
            else:unverified.append(process)
        if recognized:save(proof/'owned-sdk-bootstrap-clients.json',recognized)
        if unverified:save(proof/'protected-state-rejection.json',dict(unverifiedAdbProcesses=unverified,recognizedSdkClients=recognized))
        assert not unverified,'Fresh empty host gained an unverified ADB process'
        if server is not None and server.poll() is None:
            actual=[p for p in rows if p['ProcessId']==own_id]
            assert len(actual)==1 and Path(actual[0]['ExecutablePath'])==adb
            assert actual[0]['CommandLine'].endswith('-L tcp:localhost:5057 server nodaemon')
            initial=proof/'initial-owned-host-roots.json'
            if initial.is_file():
                expected=next(p for p in load(initial) if p['ProcessId']==own_id)
                assert actual[0]==expected,'Owned ADB identity changed'
        command="@(Get-NetTCPConnection -State Listen -ErrorAction Stop | Where-Object LocalPort -In 5037,5049).Count"
        assert subprocess.check_output([pwsh,'-NoProfile','-Command',command],text=True,encoding='utf-8-sig').strip()=='0'
        return dict(mode='no-existing-adb-or-emulator',foreignAdbAbsent=True,protectedPortsUnused=True)

    fresh_host_audit(owner, 'before', stage, [])
    save(stage/'fresh-host-baseline.json',dict(mode='no-existing-adb-or-emulator'))
    old_identity = protected_state(host_processes())
    save(proof / 'fresh-protected-state-before.json', old_identity)

    def call(label, args, required=True, timeout=30):
        nonlocal sequence
        sequence += 1
        result = subprocess.run([str(adb), '-H', '127.0.0.1', '-P', '5057', *args], cwd=stage, env=env,
            stdin=subprocess.DEVNULL, capture_output=True, timeout=timeout, creationflags=subprocess.CREATE_NO_WINDOW)
        prefix = f'{sequence:03d}-{label}'
        (proof / (prefix+'.stdout')).write_bytes(result.stdout)
        (proof / (prefix+'.stderr')).write_bytes(result.stderr)
        record['commands'].append(dict(label=label, exitCode=result.returncode))
        if required:
            result.check_returncode()
        return result

    def device(label, args, **kwargs):
        return call(label, ['-s', serial, *args], **kwargs)

    def settings():
        result = {}
        for namespace, key in [('secure','accessibility_enabled'),('secure','enabled_accessibility_services'),('global', 'disable_window_blurs'), ('secure', 'high_text_contrast_enabled'),
                ('system', 'font_scale'), ('global', 'animator_duration_scale'), ('global', 'transition_animation_scale'),
                ('global', 'window_animation_scale')]:
            result[namespace+'.'+key] = device('setting-'+key, ['shell', 'settings', 'get', namespace, key]).stdout.decode()
        result['night'] = device('night', ['shell', 'cmd', 'uimode', 'night']).stdout.decode()
        return result

    try:
        server_log = (proof / 'adb-server.log').open('xb')
        server = subprocess.Popen([str(adb), '-L', 'tcp:localhost:5057', 'server', 'nodaemon'], cwd=stage,
            env=dict(env, ADB_SERVER_SOCKET='tcp:localhost:5057', ADB_TRACE='sockets,transport,services'), stdin=subprocess.DEVNULL, stdout=server_log,
            stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW)
        time.sleep(1)
        assert server.poll() is None
        assert call('empty-devices', ['devices']).stdout.decode().strip() == 'List of devices attached'
        emu_log = (proof / 'emulator.log').open('xb')
        command = [str(sdk / 'emulator/emulator.exe'), '-avd', name, '-port', '5578', '-accel', 'on',
            '-gpu', 'swiftshader', '-memory', '2048', '-cores', '2', '-no-window', '-no-snapshot', '-no-audio', '-no-boot-anim']
        emulator = subprocess.Popen(command, cwd=stage, env=env, stdin=subprocess.DEVNULL, stdout=emu_log,
            stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW)
        record.update(serverPid=server.pid, emulatorLauncherPid=emulator.pid, emulatorCommand=command)
        save(proof / 'live.json', record)
        initial_roots=[p for p in host_processes() if p['ProcessId'] in (server.pid,emulator.pid)]
        assert len(initial_roots)==2
        save(proof/'initial-owned-host-roots.json',initial_roots)
        print('Fresh dedicated API31 boot started; Release build/package already terminal', flush=True)
        deadline = time.monotonic()+240
        while time.monotonic() < deadline:
            result = device('boot', ['shell', 'getprop', 'sys.boot_completed'], required=False, timeout=10)
            if result.returncode == 0 and result.stdout.strip() == b'1':
                break
            time.sleep(3)
        else:
            raise TimeoutError('Original 240-second boot budget exhausted')
        assert device('avd', ['emu', 'avd', 'name']).stdout.decode().splitlines()[0].strip() == name
        identified = True
        assert device('actual-online-cpu', ['shell','cat','/sys/devices/system/cpu/online']).stdout.strip() == b'0-1'
        actual_memory = device('actual-memory', ['shell','cat','/proc/meminfo']).stdout.decode().replace('\r\n','\n')
        mem = re.findall(r'^MemTotal:\s*(\d+) kB$', actual_memory, re.M)
        assert len(mem)==1 and 1700*1024 < int(mem[0]) < 2100*1024
        assert device('api', ['shell', 'getprop', 'ro.build.version.sdk']).stdout.strip() == b'31'
        assert device('abi', ['shell', 'getprop', 'ro.product.cpu.abi']).stdout.strip() == b'x86_64'
        assert sha(config) == config_hash, 'AVD configuration changed during initial boot; preserve and review'
        # Read-only boot settling. Unknown identities never become owned: install
        # and input remain prohibited until two snapshots contain no other clients.
        from bootstrap_quiet import wait_for_quiet
        processes = wait_for_quiet(host_processes,
            next(row for row in initial_roots if row['ProcessId']==server.pid),
            next(row for row in initial_roots if row['ProcessId']==emulator.pid),deadline,
            lambda number,row:save(proof/('pre-native-quiet-'+str(number)+'.json'),row))
        actual_old = protected_state(processes)
        assert actual_old == old_identity
        own_server = [p for p in processes if p['ProcessId'] == server.pid]
        own_vm = [p for p in processes if p['Name'].startswith('qemu-system-x86_64') and
                  name in (p['CommandLine'] or '') and str(sdk / 'emulator') in (p['ExecutablePath'] or '')]
        assert len(own_server) == len(own_vm) == 1
        save(proof / 'owned-host-processes.json', [*own_server, *own_vm])
        device('provider', ['shell', 'dumpsys', 'webviewupdate'])
        device('system-anr', ['shell', 'dumpsys', 'dropbox', '--print', 'system_app_anr'], timeout=45)
        device('last-anr', ['shell', 'dumpsys', 'activity', 'lastanr'])
        windows = device('preflight-windows', ['shell', 'dumpsys', 'window']).stdout.decode()
        focus = [line.strip() for line in windows.splitlines() if 'mCurrentFocus=' in line]
        screenshot = device('preflight-screenshot', ['exec-out', 'screencap', '-p']).stdout
        assert screenshot.startswith(b'\x89PNG\r\n\x1a\n')
        (proof / 'preflight.png').write_bytes(screenshot)
        assert not any('Application Not Responding' in line or 'Application Error' in line for line in focus)
        for package in ['com.solosoul.app', 'com.solosoul.app.test', 'com.solosoul.app.releaseprobe', 'org.solosoul.rf312observer']:
            absent = device('absent-'+package, ['shell', 'pm', 'path', package], required=False)
            assert absent.returncode in (0, 1) and not absent.stdout.strip() and not absent.stderr.strip()
        save(proof / 'environment-preflight.json', dict(api=31, abi='x86_64', focus=focus,
            pngSha256=sha(proof / 'preflight.png'), avdConfigUnchanged=True, nativeTestsStarted=0))
        print('API31 preflight captured; waiting for local Launcher and ANR evidence review', flush=True)
        deadline = time.monotonic()+600
        while not (proof / 'continue-native.json').is_file():
            if (proof / 'reject-native.json').is_file():
                raise RuntimeError('Independent native preflight explicitly rejected')
            assert time.monotonic() < deadline, 'Bounded local preflight review expired'
            time.sleep(1)
        review = load(proof / 'continue-native.json')
        assert review['pngSha256'] == sha(proof / 'preflight.png') and review['screenInspected']
        assert review['blockingSystemErrorDialogAbsent'] and review['systemAnrEvidenceReviewed']
        independent=load(proof/'preflight-fresh-independent/result.json')
        assert independent['passed'] and review['independentResultSha256']==sha(proof/'preflight-fresh-independent/result.json')
        assert review['latestPngSha256']==sha(proof/'preflight-fresh-independent/preflight.png')
        verify_plan(owner,artifacts=True)
        assert Path(plan_path).read_bytes()==plan_bytes
        settings_before = settings()
        packages = {row['kind']: row for row in (runtime['originalDebug'],runtime['originalTest'])}
        installed = True
        for kind in ['main', 'test']:
            row = packages[kind]
            assert row['signatureVerified'] and sha(Path(row['path'])) == row['sha256']
            device('install-'+kind, ['install', '-t', row['path']], timeout=120)
            package = 'com.solosoul.app'+('.test' if kind == 'test' else '')
            path = device('installed-path-'+kind, ['shell', 'pm', 'path', package]).stdout.decode().strip()
            assert re.fullmatch(r'package:/data/app/(?:[A-Za-z0-9_~+=.-]+/){1,2}base\.apk', path)
            payload = device('installed-bytes-'+kind, ['exec-out', 'cat', path.removeprefix('package:')], timeout=120).stdout
            assert hashlib.sha256(payload).hexdigest() == row['sha256']
        device('prewarm-main', ['shell', 'am', 'start', '-W', '-n', 'com.solosoul.app/.MainActivity'], timeout=45)
        manifest = owner['resourceManifest']
        marker = manifest['version']+'\n'+''.join(f"{e['path']}\t{e['size']}\t{e['sha256']}\n" for e in sorted(manifest['files'], key=lambda e:e['path']))
        deadline = time.monotonic()+45
        while time.monotonic() < deadline:
            present = device('resource-marker-present', ['shell', 'run-as', 'com.solosoul.app', 'test', '-f', 'app_resources/.bundled-resource-manifest'], required=False)
            if present.returncode == 0:
                break
            assert present.returncode == 1 and not present.stdout and not present.stderr
            time.sleep(1)
        else:
            raise TimeoutError('Actual MainActivity resource preparation exceeded 45 seconds')
        actual_marker = device('resource-marker', ['exec-out', 'run-as', 'com.solosoul.app', 'cat', 'app_resources/.bundled-resource-manifest']).stdout.decode()
        assert actual_marker == marker
        expected = {'app_resources/'+entry['path']:entry['sha256'] for entry in manifest['files']}
        text = device('resource-file-hashes', ['shell', 'run-as', 'com.solosoul.app', 'sha256sum', *sorted(expected)]).stdout.decode()
        actual = {}
        for line in text.splitlines():
            match = re.fullmatch(r'([a-f0-9]{64})  (.+)', line)
            assert match
            actual[match[2]] = match[1]
        assert actual == expected
        device('prewarm-force-stop', ['shell', 'am', 'force-stop', 'com.solosoul.app'])
        payload = device('prewarm-private', ['exec-out', 'run-as', 'com.solosoul.app', 'tar', '-cf', '-', '.'], timeout=120).stdout
        baseline = proof / 'prepared-empty-private.tar'
        baseline.write_bytes(payload)
        spec = importlib.util.spec_from_file_location('restore', root / 'tauri/scripts/android-home-native-regression.py')
        restore = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(restore)
        inventory = restore.inventory(baseline, require_empty=True)
        save(proof / 'prepared-empty-private-inventory.json', inventory)
        save(proof / 'environment-ready.json', dict(api=31, abi='x86_64', preflightReviewed=True,
            actualMainActivityResourcesVerified=True, resourceFiles=len(expected), manifestVersion=manifest['version'],
            preparedPrivateEntries=len(inventory), preparedPrivateTarSha256=sha(baseline), settingsBefore=settings_before,
            initializationExcludedFromMeasuredControls=True, targetInstrumentationInjected=False))
        mutation_started = True
        command = [sys.executable, str(SOURCE / 'runtime_cohort.py'), '--plan', str(plan_path), '--output', str(stage)]
        with (proof / 'cohort.stdout').open('xb') as stdout, (proof / 'cohort.stderr').open('xb') as stderr:
            child = subprocess.Popen(command, cwd=root, env=env, stdin=subprocess.DEVNULL, stdout=stdout,
                stderr=stderr, creationflags=subprocess.CREATE_NO_WINDOW)
            save(proof / 'cohort.live.json', dict(pid=child.pid, command=command))
            print('Actual MainActivity resources verified; current default twelve-step UI journey started', flush=True)
            watch_spec=importlib.util.spec_from_file_location('passive_socket_watch',SOURCE/'socket_stall_watch.py')
            watch=importlib.util.module_from_spec(watch_spec);watch_spec.loader.exec_module(watch)
            capture=watch.windows_capture(pwsh,server.pid,child.pid)
            code,watch_summary=watch.wait_for_worker(child,proof/'socket-stalls',[
                (stage/('prototype-'+str(size))/'commands.json',stage/('prototype-'+str(size))/'report.json')
                for size in [100,5000]],capture)
            record['socketWatch']=watch_summary
        report = load(stage / 'external-terminal-r2.json')
        all_restored = bool(report['outcomes']) and all(row['allOriginalStateRestored'] for row in report['outcomes'])
        record['cohortRestored'] = all_restored
        record['cohortCompleted'] = len(report['outcomes'])
        assert code == 0 and report.get('prototypePassed') and all_restored
        record['passed'] = True
    except Exception as error:
        record['error'] = type(error).__name__+': '+str(error)
        (proof / 'error.txt').write_text(traceback.format_exc(), encoding='utf-8')
        print(record['error'], flush=True)
    finally:
        cleanup = {}
        try:
            roots={p.pid for p in (server,emulator) if p is not None}
            known=[p for p in host_processes() if p['ProcessId'] in roots or
                (p['Name'].startswith('qemu-system-') and emulator is not None and p['ParentProcessId']==emulator.pid
                 and name in (p['CommandLine'] or '') and str(sdk/'emulator') in (p['ExecutablePath'] or ''))]
            save(proof/'cleanup-owned-host-processes.json',known)
        except Exception as error:
            cleanup['ownedProcessCaptureError']=repr(error)
        if not identified and emulator is not None:
            try:
                # Console ownership can be proven even if Android did not finish booting.
                identity = device('cleanup-boot-avd', ['emu', 'avd', 'name'], required=False)
                identified = identity.returncode == 0 and identity.stdout.decode().splitlines()[0].strip() == name
            except Exception as error:
                cleanup['bootIdentityVerificationError'] = repr(error)
        if identified:
            try:
                assert device('cleanup-avd', ['emu', 'avd', 'name']).stdout.decode().splitlines()[0].strip() == name
                health = proof / 'terminal-health';health.mkdir(exist_ok=False)
                (health/'dropbox.stdout').write_bytes(device('terminal-dropbox',['shell','dumpsys','dropbox'],timeout=45).stdout)
                (health/'last-anr.stdout').write_bytes(device('terminal-last-anr',['shell','dumpsys','activity','lastanr'],timeout=45).stdout)
                if settings_before is not None:
                    settings_after = settings()
                    cleanup['systemSettingsExactlyRestored'] = settings_after == settings_before
                    save(proof / 'system-restoration.json', dict(before=settings_before, after=settings_after,
                        exactlyEqual=settings_after == settings_before))
                if installed and (not mutation_started or all_restored):
                    device('cleanup-force-stop', ['shell', 'am', 'force-stop', 'com.solosoul.app'])
                    cleanup['testUninstalled'] = device('uninstall-test', ['uninstall', 'com.solosoul.app.test']).stdout.strip() == b'Success'
                    cleanup['mainUninstalled'] = device('uninstall-main', ['uninstall', 'com.solosoul.app']).stdout.strip() == b'Success'
                elif installed:
                    cleanup['packagesRetainedForStrictRecovery'] = True
                device('emulator-stop', ['emu', 'kill'])
                cleanup['emulatorShutdownRequested'] = True
            except Exception as error:
                cleanup['error'] = repr(error)
        if emulator is not None:
            emulator_exit_deadline = time.monotonic()+45
            try:
                emulator.wait(timeout=45)
                cleanup['emulatorLauncherExited'] = True
            except subprocess.TimeoutExpired:
                cleanup['emulatorLauncherExited'] = False
        if server is not None and server.poll() is None:
            try:
                call('server-stop', ['kill-server'])
                server.wait(timeout=15)
                cleanup['ownedServerExited'] = True
            except Exception as error:
                cleanup['serverError'] = repr(error)
        try:
            cleanup['protectedAdbStateUnchanged'] = protected_state(host_processes()) == old_identity
        except Exception as error:
            cleanup['protectedAdbStateVerificationError'] = repr(error)
        record['cleanup'] = cleanup
        save(proof / 'result.json', record)
        print(json.dumps(dict(passed=record['passed'], cleanup=cleanup)), flush=True)

    try:
        roots={}
        for evidence_name in ('initial-owned-host-roots.json','owned-host-processes.json','cleanup-owned-host-processes.json'):
            path=proof/evidence_name
            if path.is_file():
                for row in load(path):
                    if row['ProcessId'] in roots:assert identity.same_identity(row,roots[row['ProcessId']])
                    roots[row['ProcessId']]=row
        release_ok=True
        if emulator_exit_deadline is not None and time.monotonic()<emulator_exit_deadline:
            fresh_spec=importlib.util.spec_from_file_location('runtime_release_canonical_fresh',root/'tauri/scripts/android-fresh-host-audit.py')
            fresh=importlib.util.module_from_spec(fresh_spec);fresh_spec.loader.exec_module(fresh)
            release_spec=dict(schemaVersion=1,baseline='no-existing-adb-or-emulator',ownedProcesses=list(roots.values()),
                protectedPorts=[5037,5049],ownedPorts=[5057,5578,5579],avd=runtime['host']['avd'])
            state=ReleaseState(fresh,release_spec,sdk,os.environ['SystemRoot'])
            capture=windows_capture(fresh,pwsh,proof/'host-release-captures')
            try:
                released=wait_for_release(state,capture,emulator_exit_deadline,
                    lambda number,value:save(proof/('host-release-'+str(number)+'.json'),value))
                roots={row['ProcessId']:row for row in released}
            except Exception as error:
                release_ok=False;record['hostReleaseError']=repr(error)
        fresh_host_audit(owner,'after',stage,list(roots.values()))
        record['freshHostAfterPassed']=release_ok
    except Exception as error:record['freshHostAfterError']=repr(error)
    verify_plan(owner,artifacts=True)
    assert Path(plan_path).read_bytes()==plan_bytes
    required=['systemSettingsExactlyRestored','mainUninstalled','testUninstalled','emulatorShutdownRequested',
        'emulatorLauncherExited','ownedServerExited','protectedAdbStateUnchanged']
    record['passed']=record['passed'] and record.get('freshHostAfterPassed') is True and all(record['cleanup'].get(k) is True for k in required)
    save(proof/'result.json',record)
    if record['passed']:
        command=[sys.executable,str(SOURCE/'runtime_terminal_audit.py'),'--plan',str(plan_path),'--output',str(stage),'--host-audit','fresh-host-after']
        audit=subprocess.run(command,env=env,capture_output=True,creationflags=subprocess.CREATE_NO_WINDOW)
        (stage/'terminal-audit.stdout').write_bytes(audit.stdout);(stage/'terminal-audit.stderr').write_bytes(audit.stderr)
        record['terminalAuditPassed']=audit.returncode==0 and load(stage/'independent-terminal-audit.json')['fullUiQueueAccepted']
        record['passed']=record['terminalAuditPassed']
        if record['passed']:
            replay=subprocess.run([sys.executable,str(SOURCE/'runtime_replay.py'),'--plan',str(plan_path),'--output',str(stage)],
                env=env,capture_output=True,creationflags=subprocess.CREATE_NO_WINDOW)
            (stage/'replay-audit.stdout').write_bytes(replay.stdout);(stage/'replay-audit.stderr').write_bytes(replay.stderr)
            record['replayAuditPassed']=replay.returncode==0 and load(stage/'independent-runtime-memory-audit.json')['twoSizeCheckpointAccepted']
            record['passed']=record['replayAuditPassed']
    verify_plan(owner,artifacts=True)
    assert Path(plan_path).read_bytes()==plan_bytes
    record.update(ownerPlanSha256=sha(plan_path),fullDefaultJourneyAccepted=False,performanceDistributionAccepted=False)
    save(stage/'owner-result.json',record)
    print(json.dumps(dict(passed=record['passed'],fullDefaultJourneyAccepted=False)),flush=True)
    return 0 if record['passed'] else 1

def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--plan',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args(argv);return run(args.plan,args.output)

if __name__=='__main__':sys.exit(main())
