"""Explicit default APK driver with supported system-state evidence and unchanged restoration."""
import argparse,hashlib,importlib.util,io,json,math,os,re,shlex,subprocess,tarfile,time,traceback,uuid
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parent.parent))
import xml.etree.ElementTree as ET
import rf312_default_binding as state_binding
from adb_control import AdbRecorder
from runtime_inputs import load_inputs,verify_sources
from default_driver import (APP,METHOD,COLD_METHOD,NOTIFICATION,activity_font_scale,capture_font_scale,
    digest,inventory,night_mode,notification_permission,restore_font_scale,verification_passed)
shared_restore=None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("adb", "serial", "avd", "apk", "test-apk", "output"):
        parser.add_argument(f"--{name}", required=True)
    parser.add_argument("--mode", choices=("supported", "fallback"), required=True)
    parser.add_argument("--picker-pointer-mode",choices=("finger","unknown"),default="finger")
    parser.add_argument("--scenario", choices=("baseline", "overlays", "keyboard", "accounts", "insets", "lifecycle", "cold", "previews", "rendering"), default="baseline")
    parser.add_argument("--motion-preferences", action="store_true", help="Verify true/false reduced motion across account/process changes (cold only)")
    parser.add_argument('--font-scale', type=float, choices=(1.0, 1.3, 2.0), help='Actual Android system font scale; baseline only, restored afterward')
    parser.add_argument('--restore-apk', required=True)
    parser.add_argument('--object-count',type=int,choices=(100,5000),required=True)
    parser.add_argument('--fixture',required=True)
    parser.add_argument("--inputs", required=True, type=Path)
    parser.add_argument("--state-inputs", required=True, type=Path)
    parser.add_argument('--journey-memory',action='store_true',help='Explicit read-only periodic and twelve-boundary memory trajectory; original UI budgets remain')
    parser.add_argument('--composite-ui',action='store_true',help='Explicit composite UI control without requiring memory sampling; enables same-channel comparisons')
    parser.add_argument('--trace-plan', required=True, type=Path)
    args = parser.parse_args()
    import trace_binding
    trace_inputs=trace_binding.load(args.trace_plan,runtime=args.inputs,adapter=args.state_inputs)
    if args.scenario != 'baseline' or args.motion_preferences or args.font_scale is not None:
        parser.error('Fixed-Vault driver accepts baseline only; original restoration guards stay unchanged')
    global shared_restore
    input_bytes=args.inputs.read_bytes()
    state_inputs=state_binding.load(args.state_inputs)
    if 'ownedUi' in state_inputs and not (args.journey_memory or args.composite_ui):
        parser.error('Version 2 UI package inputs require the explicit composite channel')
    if Path(state_inputs['config']['runtimeInputs']).resolve()!=args.inputs.resolve():parser.error('Exact bound runtime inputs required')
    frozen_inputs=load_inputs(args.inputs)
    assert state_inputs['runtime']==frozen_inputs
    if Path(args.adb).resolve()!=Path(frozen_inputs['host']['adbExecutable']).resolve() or args.serial!=frozen_inputs['host']['serial'] or args.avd!=frozen_inputs['host']['avd']:
        parser.error('Exact declared SDK, serial and AVD required')
    _shared_path=Path(frozen_inputs['host']['workspaceRoot'])/'tauri/scripts/android-home-native-regression.py'
    _shared_spec=importlib.util.spec_from_file_location('current_owned_restore',_shared_path)
    shared_restore=importlib.util.module_from_spec(_shared_spec)
    _shared_spec.loader.exec_module(shared_restore)
    if (Path(args.apk).resolve() != Path(frozen_inputs['release']['apk']['path']).resolve()
            or Path(args.restore_apk).resolve() != Path(frozen_inputs['originalDebug']['copy']).resolve()
            or Path(args.test_apk).resolve() != Path(frozen_inputs['runtimeTest']['path']).resolve()):
        parser.error('Release validation requires the exact frozen Release/restore/test APKs')
    fixture_spec=frozen_inputs['fixedFixtures'][str(args.object_count)]
    if Path(args.fixture).resolve()!=Path(fixture_spec['root']).resolve():
        parser.error('Only the exact frozen public fixture is allowed')
    if args.motion_preferences and args.scenario != "cold":
        parser.error("Reduced-motion process matrix requires --scenario cold")
    if args.font_scale is not None and args.scenario != 'baseline':
        parser.error('Native font-scale observation requires --scenario baseline')
    if not re.fullmatch(r"emulator-\d+", args.serial) or not re.fullmatch(r"SoloSoul_[A-Za-z0-9_-]+", args.avd):
        parser.error("Requires explicitly named dedicated SoloSoul emulator")
    output = Path(args.output).resolve()
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    report = {"serial": args.serial, "avd": args.avd, "mode": args.mode, "scenario": args.scenario,
              "motionPreferenceChecks": args.motion_preferences, "startedAt": time.time(), "passed": False, "restored": False}
    report['requestedFontScale'] = args.font_scale
    report['journeyMemorySamplingEnabled']=args.journey_memory
    report['compositeUiRequested']=args.composite_ui
    report['compositeUiEnabled']=args.journey_memory or args.composite_ui
    report['ownedUiTransportEnabled']='ownedUi' in state_inputs
    if 'ownedUi' in state_inputs:
        report['ownedUiPackageSha256']=state_inputs['config']['ownedUiPackageSha256']
    report['runtimeInputsSha256']=hashlib.sha256(input_bytes).hexdigest()
    report['runtimeSourcesSha256']=frozen_inputs['fullJourneyFiles']
    selected = False
    backup_valid = False
    mutation_started = False
    old_blur = None
    old_notification = None
    old_night = None
    old_font_scale = None
    old_font_effective = None
    backup = output / "private-data-before.tar"
    before = None
    root_mode = None
    original_packages = None
    recorder = AdbRecorder(args.adb, args.serial, output)
    commands = recorder.commands
    public_image = None
    public_image_staged = False

    def adb(*command, **kwargs):
        return recorder.run(*command, **kwargs)

    def read(*command):
        return adb(*command, stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode().strip()

    def read_font_raw(remaining):
        return adb('shell', 'settings', 'get', 'system', 'font_scale', timeout=remaining,
                   stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode()

    def read_font_effective(remaining):
        return activity_font_scale(adb('shell', 'dumpsys', 'activity', 'processes', timeout=remaining,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode())

    def write_font(action, value, remaining):
        adb('shell', 'settings', action, 'system', 'font_scale',
            *(() if value is None else (value,)), timeout=remaining,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def archive(path):
        with open(path, "xb") as stream:
            os.chmod(path, 0o600)
            adb("exec-out", "run-as", APP, "tar", "-cf", "-", ".", stdout=stream, stderr=subprocess.PIPE)

    def frozen():
        root = Path(frozen_inputs['host']['workspaceRoot'])
        if args.inputs.read_bytes()!=input_bytes:raise RuntimeError('Declared runtime inputs changed')
        verify_sources(frozen_inputs)
        trace_binding.verify(trace_inputs)
        assert state_binding.load(args.state_inputs)==state_inputs
        if any(digest(root/name) != value for name,value in frozen_inputs['runtimeSourceFiles'].items()):
            raise RuntimeError('Frozen Release runtime source changed')
        for row in (frozen_inputs['release']['apk'], frozen_inputs['originalDebug'], frozen_inputs['originalTest'], frozen_inputs['runtimeTest']):
            if digest(Path(row.get('path', row.get('copy')))) != row['sha256']:
                raise RuntimeError('Frozen APK changed')

    def installed(package, label):
        paths = read('shell', 'pm', 'path', package).splitlines()
        if (len(paths) != 1 or not re.fullmatch(r'package:/data/app/(?:[A-Za-z0-9_~+=.-]+/){1,2}base\.apk', paths[0])
                or any(part in ('.', '..') for part in paths[0].split('/'))):
            raise RuntimeError('Expected a single installed base APK')
        payload = adb('exec-out', 'cat', paths[0].removeprefix('package:'),
                      stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout
        dump = read('shell', 'dumpsys', 'package', package)
        (output/(label+'-package.log')).write_text(dump, encoding='utf-8')
        uid = re.findall(r'^\s*userId=(\d+)\s*$', dump, re.M)
        flags = re.findall(r'^\s*pkgFlags=\[([^\]]*)\]', dump, re.M)
        if len(uid) != 1 or len(flags) != 1:
            raise RuntimeError('Missing unambiguous package UID/debug flags')
        return {'sha256': hashlib.sha256(payload).hexdigest(), 'bytes': len(payload),
                'uid': int(uid[0]), 'debuggable': 'DEBUGGABLE' in flags[0].split()}

    try:
        frozen()
        devices = read("devices", "-l").splitlines()
        if not any(line.split()[:2] == [args.serial, "device"] for line in devices):
            raise RuntimeError("Dedicated emulator missing/offline/unauthorized")
        if read("emu", "avd", "name").splitlines()[0] != args.avd:
            raise RuntimeError("AVD identity mismatch")
        selected = True
        if read('shell','pm','list','packages','com.solosoul.app.releaseprobe'):
            raise RuntimeError('Dedicated Release probe package already exists; refusing replacement')
        original_packages = {'main': installed(APP, 'original-main'),
                             'test': installed(APP+'.test', 'original-test')}
        report['originalInstalledPackages'] = original_packages
        if (original_packages['main']['sha256'] != frozen_inputs['originalDebug']['sha256']
                or original_packages['test']['sha256'] != frozen_inputs['originalTest']['sha256']
                or not original_packages['main']['debuggable']):
            raise RuntimeError('Installed original packages differ; refusing Release replacement')
        if args.font_scale is not None:
            old_font_scale, old_font_effective = capture_font_scale(read_font_raw, read_font_effective)
            report['fontScaleBefore'] = old_font_scale
            report['fontScaleBeforeEffective'] = old_font_effective
        if args.scenario in ('lifecycle', 'cold'):
            old_night = night_mode(read('shell', 'cmd', 'uimode', 'night'))
            report['nightModeBefore'] = old_night
        report["api"] = int(read("shell", "getprop", "ro.build.version.sdk"))
        report["abi"] = read("shell", "getprop", "ro.product.cpu.abi")
        if report["api"] < 31:
            raise RuntimeError("Requires API >=31")
        if report["api"] >= 33:
            old_notification = notification_permission(read("shell", "dumpsys", "package", APP))
            report["notificationPermissionBefore"] = old_notification
        report["artifacts"] = [{"path": str(Path(path).resolve()), "sha256": digest(path)}
                               for path in (args.apk, args.test_apk)]
        adb("shell", "am", "force-stop", APP, stdout=subprocess.PIPE)
        # 先备份已安装专用应用的数据；install -r 保留数据且不得先执行 clear。
        root_mode = read("shell", "run-as", APP, "stat", "-c", "%a", ".")
        archive(backup)
        before = inventory(backup, require_empty=True)
        backup_valid = True
        fixture=Path(fixture_spec['root'])
        actual_files={p.relative_to(fixture).as_posix():digest(p) for p in fixture.rglob('*') if p.is_file()}
        if actual_files!=fixture_spec['files']:
            raise RuntimeError('Public fixed fixture source changed')
        fixture_tar=output/'public-fixed-fixture.tar'
        with tarfile.open(fixture_tar,'w') as packed:
            for relative,expected in sorted(actual_files.items()):
                path=fixture/relative
                if path.is_symlink() or path.stat().st_file_attributes&0x400:
                    raise RuntimeError('Public fixture must be ordinary')
                data=path.read_bytes()
                if hashlib.sha256(data).hexdigest()!=expected:raise RuntimeError('Fixture bytes changed')
                member=tarfile.TarInfo(relative);member.size=len(data);member.mode=0o600
                packed.addfile(member,io.BytesIO(data))
        remote_fixture=f'/data/local/tmp/rf312-fixed-fixture-{uuid.uuid4().hex}.tar'
        mutation_started=True
        try:
            adb('push',str(fixture_tar),remote_fixture,stdout=subprocess.PIPE)
            adb('shell','chmod','644',remote_fixture,stdout=subprocess.PIPE)
            adb('shell','run-as',APP,'tar','-xf',remote_fixture,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            device_hashes=read('shell','run-as',APP,'sha256sum',*sorted(actual_files))
            observed={}
            for line in device_hashes.splitlines():
                match=re.fullmatch(r'([a-f0-9]{64})  (.+)',line)
                if not match:raise RuntimeError('Unrecognized transferred public file checksum')
                observed[match[2]]=match[1]
            if observed!=actual_files:raise RuntimeError('Transferred public fixture bytes differ')
            report['fixedFixture']={'objectCount':args.object_count,'all8TransferredFileHashesVerified':True,'source':fixture_spec}
        finally:
            adb('shell','rm','-f','--',remote_fixture,stdout=subprocess.PIPE)
        if {p.relative_to(fixture).as_posix():digest(p) for p in fixture.rglob('*') if p.is_file()}!=actual_files:
            raise RuntimeError('Source fixed fixture was modified')
        report["backup"] = {"path": str(backup), "sha256": digest(backup), "entries": len(before)}
        old_blur = read("shell", "settings", "get", "global", "disable_window_blurs")
        if args.font_scale is not None:
            adb('shell', 'settings', 'put', 'system', 'font_scale', str(args.font_scale), stdout=subprocess.PIPE)
        adb("shell", "settings", "put", "global", "disable_window_blurs", "1" if args.mode == "fallback" else "0", stdout=subprocess.PIPE)
        mutation_started = True
        adb("install", "-r", str(Path(args.apk).resolve()), stdout=subprocess.PIPE)
        if read('shell','pm','list','packages','com.solosoul.app.releaseprobe'):
            raise RuntimeError('Default external control must not install a target instrumentation APK')
        report['targetInstrumentationInjected'] = False
        current = installed(APP, 'release-main')
        report['releaseInstalledPackage'] = current
        if (current['sha256'] != frozen_inputs['release']['apk']['sha256'] or current['debuggable']
                or current['uid'] != original_packages['main']['uid']):
            raise RuntimeError('Exact non-debuggable Release package/UID verification failed')
        frozen()
        tag = f"rf312-media-{uuid.uuid4().hex}"
        image_name='rf312-ocr-'+tag.removeprefix('rf312-media-')+'.png'
        public_image='/sdcard/Download/'+image_name
        image=frozen_inputs['publicOcrImage']
        if digest(Path(image['path']))!=image['sha256']:raise RuntimeError('Public OCR PNG changed')
        adb('shell','sh','-c',f'"test ! -e {public_image}"',stdout=subprocess.PIPE)
        adb('push',image['path'],public_image,stdout=subprocess.PIPE)
        public_image_staged=True
        if read('shell','sha256sum',public_image).split()!=[image['sha256'],public_image]:raise RuntimeError('Staged OCR image bytes mismatch')
        adb('shell','am','broadcast','-a','android.intent.action.MEDIA_SCANNER_SCAN_FILE','-d','file://'+public_image,stdout=subprocess.PIPE)
        report['publicOcrInput']={'path':public_image,'fileName':image_name,'sha256':image['sha256'],'publicFixtureOnly':True}
        canonical_image=read('shell','readlink','-f',public_image)
        image_where=shlex.quote("_data='"+canonical_image+"'")
        media_state=read('shell','content','query','--uri','content://media/external/file','--where',image_where,'--projection','_id:_display_name:mime_type:date_modified:_size')
        (output/'owned-public-image-media-before.log').write_text(media_state+'\n',encoding='utf-8')


        external_path = Path(__file__).parent/'trace_journey.py'
        if digest(external_path) != trace_inputs['sources'][str(external_path.resolve())]:
            raise RuntimeError('Explicit supported-state journey changed')
        external_spec = importlib.util.spec_from_file_location('default_external', external_path)
        external = importlib.util.module_from_spec(external_spec)
        external_spec.loader.exec_module(external)
        channel_options={}if 'ownedUi'not in state_inputs else dict(owned_ui=state_inputs['ownedUi'])
        evidence = external.observe(adb, read, output, args.object_count, original_packages['main']['uid'], report['publicOcrInput']['fileName'], frozen_inputs,memory_sampling=args.journey_memory,composite_ui=args.composite_ui,trace_inputs=trace_inputs,**channel_options)
        assert evidence['compositeUiRequested'] is args.composite_ui
        assert evidence['compositeUiEnabled'] is (args.journey_memory or args.composite_ui)
        assert evidence['journeyMemorySamplingEnabled'] is args.journey_memory
        report['defaultExternalEvidence'] = evidence
        report['stages'] = evidence['stageNames']
        report['nativePassed'] = bool(evidence['success'] is True and evidence['supportedNativeStateVerified']
            and evidence['postJourneyDirectAbsenceVerified'])
        report['supportedNativeStateVerified']=evidence['supportedNativeStateVerified']

    except Exception as error:
        report["error"] = str(error)
        report["errorType"] = type(error).__name__
        (output/'driver-exception.traceback.log').write_text(traceback.format_exc(), encoding='utf-8')
        try:
            for label, command in [('window', ['shell','dumpsys','window','windows']),
                    ('power', ['shell','dumpsys','power']),
                    ('accessibility', ['shell','dumpsys','accessibility'])]:
                diagnostic = adb(*command,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                (output/('failure-'+label+'.stdout.log')).write_bytes(diagnostic.stdout)
                (output/('failure-'+label+'.stderr.log')).write_bytes(diagnostic.stderr)
            start_file = output/'default-external/observed-process-start.json'
            if start_file.exists():
                observed = json.loads(start_file.read_text(encoding='utf-8'))
                diagnostic = adb('shell','logcat','-d','--pid='+str(observed['pid']),
                    stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                (output/'failure-app-logcat.stdout.log').write_bytes(diagnostic.stdout)
                (output/'failure-app-logcat.stderr.log').write_bytes(diagnostic.stderr)
        except Exception as diagnostic_error:
            report['postFailureDiagnosticErrorType'] = type(diagnostic_error).__name__
    finally:
        if selected:
            try:
                adb("shell", "am", "force-stop", APP, stdout=subprocess.PIPE)
                if backup_valid and mutation_started:
                    # Release run-as is unavailable. Restore the original APK first,
                    # without clearing data; only then restore the full original TAR.
                    adb('install', '-r', str(Path(args.restore_apk).resolve()), stdout=subprocess.PIPE)
                    adb('shell', 'am', 'force-stop', APP, stdout=subprocess.PIPE)
                    restored_packages = {'main': installed(APP, 'restored-main'),
                                         'test': installed(APP+'.test', 'restored-test')}
                    report['restoredInstalledPackages'] = restored_packages
                    if restored_packages != original_packages:
                        raise RuntimeError('Original APK bytes, UID or debug flags were not restored')
                    report['originalApksRestored'] = True
                    probe_list = read('shell','pm','list','packages','com.solosoul.app.releaseprobe')
                    if probe_list:
                        if probe_list != 'package:com.solosoul.app.releaseprobe':
                            raise RuntimeError('Unexpected probe package identity; refusing uninstall')
                        probe = installed('com.solosoul.app.releaseprobe', 'probe-before-uninstall')
                        if probe['sha256'] != frozen_inputs['runtimeTest']['sha256']:
                            raise RuntimeError('Probe package bytes changed; refusing uninstall')
                        adb('uninstall','com.solosoul.app.releaseprobe',stdout=subprocess.PIPE)
                    if read('shell','pm','list','packages','com.solosoul.app.releaseprobe'):
                        raise RuntimeError('Dedicated probe package was not removed')
                    report['ownedProbeRemoved'] = True
                    shared_restore.restore_private_data(adb,read,archive,backup,before,root_mode,output,report)
            except Exception as error:
                report["restoreError"] = str(error)
            # 私有目录恢复异常也必须尝试恢复系统模糊设置，两项清理互不依赖。
            if old_blur is not None:
                try:
                    command = ("delete",) if old_blur == "null" else ("put",)
                    adb("shell", "settings", *command, "global", "disable_window_blurs",
                        *(() if old_blur == "null" else (old_blur,)), stdout=subprocess.PIPE)
                    report["blurRestored"] = True
                except Exception as error:
                    report["blurRestoreError"] = str(error)
            if old_notification is not None:
                try:
                    current = notification_permission(read("shell", "dumpsys", "package", APP))
                    if current["granted"] != old_notification["granted"]:
                        adb("shell", "pm", "grant" if old_notification["granted"] else "revoke",
                            APP, NOTIFICATION, stdout=subprocess.PIPE)
                    # 通知对话框可能设置 USER_SET / USER_FIXED；只恢复本应用这一权限。
                    adb("shell", "pm", "clear-permission-flags", APP, NOTIFICATION,
                        "user-set", "user-fixed", stdout=subprocess.PIPE)
                    flags = [flag.lower().replace("_", "-") for flag in old_notification["flags"]
                             if flag in ("USER_SET", "USER_FIXED")]
                    if flags:
                        adb("shell", "pm", "set-permission-flags", APP, NOTIFICATION,
                            *flags, stdout=subprocess.PIPE)
                    actual = notification_permission(read("shell", "dumpsys", "package", APP))
                    if actual != old_notification:
                        raise RuntimeError("Notification permission/flags differ from original state")
                    report["notificationPermissionRestored"] = True
                except Exception as error:
                    report["notificationRestoreError"] = str(error)
            if old_night is not None:
                try:
                    adb('shell', 'cmd', 'uimode', 'night', old_night, stdout=subprocess.PIPE)
                    if night_mode(read('shell', 'cmd', 'uimode', 'night')) != old_night:
                        raise RuntimeError('System night mode differs from original state')
                    report['nightModeRestored'] = True
                except Exception as error:
                    report['nightModeRestoreError'] = str(error)
            if old_font_scale is not None:
                try:
                    report['fontScaleRestoreTrace'] = {}
                    restore_font_scale(old_font_scale, old_font_effective, write_font,
                                       read_font_raw, read_font_effective,
                                       trace=report['fontScaleRestoreTrace'])
                    report['fontScaleRestored'] = True
                except Exception as error:
                    report['fontScaleRestoreError'] = str(error)
        if public_image_staged:
            try:
                if not re.fullmatch(r'/sdcard/Download/rf312-ocr-[a-f0-9]{32}\.png',public_image):raise RuntimeError('Owned image path rejected')
                expected=frozen_inputs['publicOcrImage']['sha256']
                if read('shell','sha256sum',public_image).split()!=[expected,public_image]:raise RuntimeError('Owned public image changed; retain for inspection')
                canonical=read('shell','readlink','-f',public_image)
                if not canonical.endswith('/Download/'+Path(public_image).name):raise RuntimeError('Owned media path alias changed')
                where=shlex.quote("_data='"+canonical+"'")
                adb('shell','content','delete','--uri','content://media/external/file','--where',where,stdout=subprocess.PIPE)
                state=read('shell','sh','-c',f'"if test -e {public_image}; then echo present; else echo absent; fi"')
                if state=='present':
                    if read('shell','sha256sum',public_image).split()!=[expected,public_image]:raise RuntimeError('Remaining owned image changed')
                    adb('shell','rm',public_image,stdout=subprocess.PIPE)
                elif state!='absent':raise RuntimeError('Owned file existence could not be established')
                adb('shell','sh','-c',f'"test ! -e {public_image}"',stdout=subprocess.PIPE)
                remaining=read('shell','content','query','--uri','content://media/external/file','--where',where,'--projection','_id')
                if remaining not in ('No result found.',''):raise RuntimeError('Owned media index still exists: '+remaining)
                report['ownedPublicOcrInputRemoved']=True
            except Exception as error:
                report['publicOcrCleanupError']=str(error)
        try:
            frozen()
            report['frozenInputsUnchanged'] = True
        except Exception as error:
            report['sourceRestoreError'] = str(error)
        report['passed'] = bool(verification_passed(report, old_notification is not None)
                                and report.get('originalApksRestored')
                                and report.get('ownedProbeRemoved')
                                and report.get('frozenInputsUnchanged')
                                and not report.get('sourceRestoreError')
                                and report.get('ownedPublicOcrInputRemoved'))
        report['defaultStateAdapterAccepted']=report['passed'] and report.get('supportedNativeStateVerified') is True
        report['targetInstrumentationInjected']=False
        report['javaScriptInjected']=False
        report['performanceDistributionAccepted']=False
        report['fullRF312Accepted']=False
        report["finishedAt"] = time.time()
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        cases = [(METHOD, bool(report.get('stages')) if args.scenario == 'cold' else report['passed'])]
        if args.scenario == 'cold':
            cases.append((COLD_METHOD, report['passed']))
        suite = ET.Element("testsuite", name="android-production-home", tests=str(len(cases)),
                           failures=str(sum(not passed for _, passed in cases)), skipped="0")
        for method, passed in cases:
            case = ET.SubElement(suite, "testcase", classname="DefaultExternalPublicUnlockObserverSmoke", name=method)
            if not passed:
                ET.SubElement(case, "failure", message=report.get("restoreError", report.get("error", "Incomplete evidence")))
        ET.ElementTree(suite).write(output / "junit.xml", encoding="utf-8", xml_declaration=True)
        print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1

if __name__=='__main__':raise SystemExit(main())
