from contextlib import contextmanager
from pathlib import Path
import base64,hashlib,importlib.util,json,subprocess,tempfile,time,unittest,zipfile
from types import SimpleNamespace
from unittest.mock import Mock,patch
import rf312_default_binding as state_binding
import rf312_owned_ui as transport
R=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('public_owned_binding_unit',R/'rf312-owned-ui/file_binding.py')
owned=importlib.util.module_from_spec(spec);spec.loader.exec_module(owned)

class BindingTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
        self.exe=self.root/'owned-sdk';self.exe.write_bytes(b'fixed sdk')
        self.script=self.root/'control.sh';self.script.write_bytes(b'fixed private helper')
        self.hash=hashlib.sha256(self.script.read_bytes()).hexdigest()
        self.uid=b'10123';self.bad_hash=False;self.residual=False;self.calls=[]
        self.previous=Mock();self.observer=owned.exchange.Client(self.previous,self.root,SimpleNamespace(poll=lambda:None),'files/'+'a'*32+'/')
        self.process=SimpleNamespace(pid=999999,returncode=None)
        self.control=SimpleNamespace(sequence=0,broken=False,stderr=b'',calls=[])
        def call(operation,*args,**kwargs):
            self.control.sequence+=1;self.control.calls.append((operation,args,kwargs))
            return subprocess.CompletedProcess([],0,b'rf312-control-ready\n'if operation=='ping'else b'ready',b'')
        def close():self.process.returncode=1 if self.control.broken else 0
        self.control.call=call;self.control.close=close

    def sdk(self,*args,**kwargs):
        self.calls.append(args)
        if args[:3]==('shell','run-as',owned.exchange.PKG):tail=args[3:]
        elif args[:6]==('shell','-T','-e','none','run-as',owned.exchange.PKG):tail=args[6:]
        elif args[:3]==('exec-out','run-as',owned.exchange.PKG):tail=args[3:]
        else:raise AssertionError(args)
        self.assertEqual(kwargs['stdout'],subprocess.PIPE);self.assertEqual(kwargs['stderr'],subprocess.PIPE)
        out=b''
        if tail==('id','-u'):out=self.uid+b'\n'
        elif tail[:3]==('stat','-c','%a'):out=b'444\n'if tail[-1].endswith('/control.jar')else b'700\n'
        elif tail[:3]==('stat','-c','%u'):out=self.uid+b'\n'
        elif tail[0]=='sha256sum':out=(('0'*64 if self.bad_hash else self.hash)+' '+tail[1]+'\n').encode()
        elif tail[:2]==('ls','-A'):out=b'foreign\ncontrol.jar\n'if self.residual else b'control.jar\n'
        elif tail[:2]==('sh','-c'):
            if 'base64 -d >'in tail[2]:self.assertEqual(base64.b64decode(kwargs['input'],validate=True),self.script.read_bytes())
        elif tail[0]=='cat':out=self.script.read_bytes()
        elif tail[0]not in('rm','rmdir','chmod'):raise AssertionError(args)
        return subprocess.CompletedProcess(args,0,out,b'')

    def context(self):
        return owned.binding(self.observer,self.sdk,self.root,executable=self.exe,serial='emulator-5578',
            expected_executable_sha256=hashlib.sha256(self.exe.read_bytes()).hexdigest(),script=self.script,shell_crlf=True)

    def patches(self):
        return patch.object(owned.subprocess,'Popen',return_value=self.process),patch.object(owned.exchange,'Control',return_value=self.control)

    def report(self):return json.loads((self.root/'persistent-control/cleanup.json').read_bytes())

    def test_owned_scope_and_callbacks_restore_after_normal_close(self):
        a,b=self.patches()
        with a as process,b:
            with self.context()as(channel,_):
                self.assertEqual(self.observer.adb,channel);self.assertTrue(self.observer.composite_bound)
            command=process.call_args.args[0]
        self.assertIn('run-as',command);self.assertIn('org.solosoul.rf312transport.OwnedUi',command[-1]);self.assertIn('10123',command[-1])
        self.assertIs(self.observer.adb,self.previous);self.assertFalse(self.observer.composite_bound)
        self.assertTrue(self.report()['accepted']);self.assertTrue(self.report()['ownedRemoteScratchRemoved'])
        self.assertEqual([c[0]for c in self.control.calls],['ping','get-stable'])

    def test_non_application_uid_refused_before_any_process_or_mutation(self):
        self.uid=b'2000';a,b=self.patches()
        with a as process,b,self.assertRaisesRegex(RuntimeError,'Observer UID'):
            with self.context():pass
        process.assert_not_called();self.assertEqual(len(self.calls),1)
        self.assertFalse(self.report()['accepted']);self.assertIs(self.observer.adb,self.previous)

    def test_wrong_remote_script_identity_retains_files_and_original_error(self):
        self.bad_hash=True;a,b=self.patches()
        with a as process,b,self.assertRaises(AssertionError):
            with self.context():pass
        process.assert_not_called();self.assertFalse(any(c[3]=='rm'for c in self.calls))
        self.assertFalse(self.report()['accepted']);self.assertTrue(self.report()['errors'])

    def test_broken_measurement_never_falls_back_or_hides_original_exception(self):
        a,b=self.patches()
        with a,b,self.assertRaisesRegex(RuntimeError,'original-native-failure'):
            with self.context():
                self.control.broken=True
                raise RuntimeError('original-native-failure')
        self.assertFalse(self.report()['accepted']);self.assertFalse(self.report()['normalControlClose'])
        self.assertIs(self.observer.adb,self.previous);self.previous.assert_not_called()

    def test_unexpected_residual_refuses_deletion_and_acceptance(self):
        a,b=self.patches()
        with a,b,self.assertRaisesRegex(RuntimeError,'cleanup not accepted'):
            with self.context():self.residual=True
        self.assertFalse(self.report()['accepted']);self.assertTrue(self.report()['normalControlClose'])
        self.assertFalse(any(c[3]=='rm'for c in self.calls));self.assertIs(self.observer.adb,self.previous)

    def test_original_observer_expiry_refuses_before_readiness(self):
        self.observer.deadline=time.monotonic()-1;a,b=self.patches()
        with a,b,self.assertRaises(TimeoutError):
            with self.context():pass
        self.assertEqual([c[0]for c in self.control.calls],['ping'])
        self.assertFalse(self.report()['accepted']);self.assertIs(self.observer.adb,self.previous)

    def test_unowned_namespace_refused_before_sdk_or_process(self):
        self.observer.remote='files/unowned/';a,b=self.patches()
        with a as process,b,self.assertRaises(ValueError):
            with self.context():pass
        self.assertEqual(self.calls,[]);process.assert_not_called()


class RoutingTests(unittest.TestCase):
    def context(self, *, fail=None):
        events=[];observer=SimpleNamespace(adb='sdk',composite_bound=False)
        @contextmanager
        def auxiliary(client,*args,**kwargs):
            self.assertEqual(kwargs['script'].name,'persistent_control.sh')
            previous=client.adb
            def run(*command,**kwargs):
                events.append(('auxiliary',command));return subprocess.CompletedProcess([],0,b'aux\r\n',b'')
            client.adb=run
            try:yield run,None
            finally:client.adb=previous;events.append('auxiliary-closed')
        @contextmanager
        def files(client,*args,**kwargs):
            self.assertEqual(kwargs['script'].name,'owned-ui.jar')
            previous=client.adb
            def run(*command,**kwargs):
                events.append(('ui',command))
                if fail is not None:raise fail
                return subprocess.CompletedProcess([],0,b'ui\0\r\n',b'')
            client.adb=run;client.composite_bound=True
            try:yield run,None
            finally:client.adb=previous;client.composite_bound=False;events.append('ui-closed')
        return observer,events,auxiliary,files

    def test_fixed_routes_use_two_channels_and_restore_callbacks_in_order(self):
        observer,events,auxiliary,files=self.context()
        with tempfile.TemporaryDirectory()as directory,patch.object(transport.persistent_adapter,'binding',side_effect=auxiliary),patch.object(transport,'file_binding',return_value=files):
            with transport.binding(observer,Mock(),directory,executable='sdk',serial='emulator-5578',expected_executable_sha256='a'*64,script=Path('owned-ui.jar'),shell_crlf=True)as(run,read):
                self.assertIs(observer.adb,run)
                self.assertEqual(run('rf312-exchange','request').stdout,b'ui\0\r\n')
                self.assertEqual(run('rf312-get-stable','frame').stdout,b'ui\0\r\n')
                self.assertEqual(read('shell','pidof','com.solosoul.app'),'aux')
        self.assertEqual([event[0]for event in events[:-2]],['ui','ui','auxiliary'])
        self.assertEqual(events[-2:],['ui-closed','auxiliary-closed'])
        self.assertEqual(observer.adb,'sdk');self.assertFalse(observer.composite_bound)

    def test_failed_ui_operation_is_never_replayed_on_auxiliary_or_sdk(self):
        failure=subprocess.TimeoutExpired('fixed-original-command',5)
        observer,events,auxiliary,files=self.context(fail=failure);sdk=Mock()
        with tempfile.TemporaryDirectory()as directory,patch.object(transport.persistent_adapter,'binding',side_effect=auxiliary),patch.object(transport,'file_binding',return_value=files):
            with self.assertRaises(subprocess.TimeoutExpired)as caught:
                with transport.binding(observer,sdk,directory,executable='sdk',serial='emulator-5578',expected_executable_sha256='a'*64,script=Path('owned-ui.jar'),shell_crlf=True)as(run,_):
                    run('rf312-exchange','request')
        self.assertIs(caught.exception,failure);sdk.assert_not_called()
        self.assertEqual(events,[('ui',('rf312-exchange','request')),'ui-closed','auxiliary-closed'])
        self.assertEqual(observer.adb,'sdk');self.assertFalse(observer.composite_bound)


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
        artifact=self.root/'unit-only.jar'
        with zipfile.ZipFile(artifact,'x')as archive:archive.writestr('classes.dex',b'dex\nunit-test-only')
        tool=self.root/'unit-tool';tool.write_bytes(b'unit-only frozen tool')
        self.value=dict(schemaVersion=1,passed=True,artifact=str(artifact),artifactSha256=transport.digest(artifact),bytes=artifact.stat().st_size,
            className=transport.CLASS,compileSdk=36,minApi=28,sourceSha256=transport.sources(),
            tools={name:dict(path=str(tool),sha256=transport.digest(tool))for name in ('java','javac','android','d8')},
            steps=[dict(step=name,exitCode=0)for name in ('javac','d8')],observerApkChanged=False,productInstalled=False,nativeRuntimeAccepted=False)
        self.receipt=self.root/'package.json'
    def write(self):
        self.receipt.write_text(json.dumps(self.value),encoding='utf-8')
        return transport.digest(self.receipt)
    def test_complete_explicit_package_loads_without_running_any_native_command(self):
        digest=self.write()
        with patch.object(transport.subprocess,'run')as command:
            self.assertEqual(transport.package(self.receipt,digest),self.value)
        command.assert_not_called()
    def test_artifact_source_tool_and_receipt_changes_are_refused(self):
        digest=self.write()
        for target in [self.receipt,Path(self.value['artifact']),Path(self.value['tools']['java']['path'])]:
            before=target.read_bytes();target.write_bytes(before+b'changed')
            try:
                with self.subTest(target=target.name),self.assertRaises(ValueError):transport.package(self.receipt,digest)
            finally:target.write_bytes(before)
        self.value['sourceSha256']['rf312-owned-ui/OwnedUi.java']='a'*64
        with self.assertRaises(ValueError):transport.package(self.receipt,self.write())
    def test_extra_archive_entry_failed_compile_or_missing_tool_cannot_be_accepted(self):
        original=json.loads(json.dumps(self.value))
        for update in [dict(passed=False),dict(steps=[dict(step='javac',exitCode=0),dict(step='d8',exitCode=1)]),dict(tools={}),dict(schemaVersion=True)]:
            self.value={**original,**update}
            with self.subTest(update=update),self.assertRaises(ValueError):transport.package(self.receipt,self.write())
        self.value=original;artifact=Path(self.value['artifact'])
        with zipfile.ZipFile(artifact,'a')as archive:archive.writestr('android/os/SystemClock.class',b'unit-stub')
        self.value.update(artifactSha256=transport.digest(artifact),bytes=artifact.stat().st_size)
        with self.assertRaises(ValueError):transport.package(self.receipt,self.write())


class VersionTwoBindingTests(unittest.TestCase):
    def test_version_two_requires_both_exact_package_fields_and_preserves_v1(self):
        with patch.object(state_binding,'sources',return_value={'source.py':'a'*64}):
            base=dict(schemaVersion=1,runtimeInputs=str(Path(__file__).resolve()),runtimeInputsSha256='a'*64,sourceSha256={'source.py':'a'*64})
            state_binding.shape(base)
            version={**base,'schemaVersion':2,'ownedUiPackage':str(Path(__file__).resolve()),'ownedUiPackageSha256':'b'*64}
            state_binding.shape(version)
            for key in ('ownedUiPackage','ownedUiPackageSha256'):
                incomplete=dict(version);del incomplete[key]
                with self.assertRaises(ValueError):state_binding.shape(incomplete)
            with self.assertRaises(ValueError):state_binding.shape({**base,'ownedUiPackage':version['ownedUiPackage'],'ownedUiPackageSha256':'b'*64})

    def test_bad_package_cannot_bypass_original_runtime_reader(self):
        with tempfile.TemporaryDirectory()as directory:
            root=Path(directory);runtime=root/'runtime.json';runtime.write_bytes(b'{}')
            config=dict(schemaVersion=2,runtimeInputs=str(runtime),runtimeInputsSha256=transport.digest(runtime),sourceSha256={'source.py':'a'*64},ownedUiPackage=str(root/'package.json'),ownedUiPackageSha256='b'*64)
            path=root/'inputs.json';path.write_text(json.dumps(config),encoding='utf-8')
            with patch.object(state_binding,'sources',return_value=config['sourceSha256']),patch.object(state_binding,'load_inputs',return_value={'strict':'original'})as reader,patch.object(transport,'package',side_effect=ValueError('package changed')):
                with self.assertRaisesRegex(ValueError,'package changed'):state_binding.load(path)
            reader.assert_called_once_with(runtime)


class JourneyIntegrationTests(unittest.TestCase):
    def test_owned_package_cannot_implicitly_enable_a_different_ui_channel(self):
        import rf312_default_journey as journey
        helper=Mock()
        with self.assertRaises(ValueError):
            with journey.ui_channel(False,helper,Mock(),Mock(),Path('unused'),{},owned_ui={'artifact':'unused'}):pass
        helper.session.assert_not_called()

    def test_explicit_package_selects_java_binder_and_preserves_original_session_cleanup(self):
        import rf312_default_journey as journey
        observer=SimpleNamespace(adb=object(),output=Path('owned-output'),process=object(),
            remote='files/'+'a'*32+'/',deadline=123,ordinal=7)
        events=[]
        @contextmanager
        def session(*args,**kwargs):
            try:yield observer
            finally:events.append(('observer-closed',observer.ordinal))
        @contextmanager
        def binder(client,*args,**kwargs):
            self.assertIs(client.process,observer.process);self.assertEqual(client.deadline,123)
            self.assertEqual(kwargs['script'],Path('bound-package.jar'))
            try:yield 'java-route','java-read'
            finally:events.append('dual-route-closed')
        inputs=dict(observer='original-package',host=dict(adbExecutable='sdk',serial='emulator-5578',adbSha256='a'*64))
        with patch.object(transport,'binding',side_effect=binder)as selected,patch.object(journey,'bind_transport')as old, self.assertRaisesRegex(RuntimeError,'original failure'):
            with journey.ui_channel(True,SimpleNamespace(session=session),Mock(),Mock(),Path('unused'),inputs,owned_ui={'artifact':'bound-package.jar'})as(client,run,read):
                self.assertEqual((run,read),('java-route','java-read'));client.ordinal=11
                raise RuntimeError('original failure')
        selected.assert_called_once();old.assert_not_called()
        self.assertEqual(events,['dual-route-closed',('observer-closed',11)])
