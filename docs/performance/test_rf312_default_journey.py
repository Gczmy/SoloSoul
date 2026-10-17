import ast
from contextlib import contextmanager
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import Mock,patch
import rf312_default_journey as journey


class DefaultJourneyGateTests(unittest.TestCase):
    def focused(self,*,pid='123',focus='com.solosoul.app'):
        tree=ast.parse(Path(journey.__file__).read_text(encoding='utf-8'))
        observe=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='observe')
        function=next(n for n in observe.body if isinstance(n,ast.FunctionDef) and n.name=='focused')
        gate=Mock();reader=SimpleNamespace(read_process_ids=lambda read,app:pid)
        namespace=dict(APP=journey.APP,PICKER=journey.PICKER,re=journey.re,
            read=lambda *args:'mCurrentFocus=Window{abc u0 '+focus+'/.MainActivity}',process_id='123',
            reader=reader,gate=gate,helper=SimpleNamespace(PKG='org.solosoul.rf312observer'),
            observer=SimpleNamespace(deadline=40),journey=SimpleNamespace(current_phase_deadline=30))
        exec(compile(ast.Module(body=[function],type_ignores=[]),'<actual-default-focus-guard>','exec'),namespace)
        return namespace['focused'],gate

    def test_actual_focus_guard_requires_only_own_observer_with_original_deadline(self):
        focused,gate=self.focused();focused()
        gate.check.assert_called_once_with({'org.solosoul.rf312observer/org.solosoul.rf312observer.Observer':'org.solosoul.rf312observer'},deadline=30)

    def test_wrong_process_owner_or_requested_package_never_reaches_state_acceptance(self):
        for arguments,package in [({'pid':'999'},journey.APP),({'focus':'other.app'},journey.APP),({},'other.app')]:
            focused,gate=self.focused(**arguments)
            with self.subTest(arguments=arguments),self.assertRaises(RuntimeError):focused(package)
            gate.check.assert_not_called()

    def test_phase_deadline_is_inherited_and_never_reset(self):
        value=journey.Journey.__new__(journey.Journey);value.current_phase_deadline=None
        with patch.object(journey.original.time,'monotonic',return_value=0):self.assertEqual(value.remaining(45),45)
        self.assertEqual(value.current_phase_deadline,45)
        with patch.object(journey.original.time,'monotonic',return_value=46),self.assertRaises(TimeoutError):value.remaining(45)
        self.assertEqual(value.current_phase_deadline,45)


class DefaultCompositeChannelTests(unittest.TestCase):
    def run_channel(self,enabled,*,fail=False):
        import diagnostic_exchange as exchange
        original=SimpleNamespace(adb=object(),output=Path('owned-output'),process=object(),
            remote='files/'+'a'*32+'/',deadline=123.5,ordinal=7)
        events=[]
        @contextmanager
        def session(*args,**kwargs):
            events.append('observer-start')
            try:yield original
            finally:events.append(('observer-cleanup',original.ordinal))
        @contextmanager
        def binding(client,*args,**kwargs):
            events.append(('binding',client,kwargs['script']))
            try:yield 'channel','read-channel'
            finally:events.append('binding-cleanup')
        helper=SimpleNamespace(session=session)
        inputs=dict(observer='original-package',host=dict(adbExecutable='original-sdk',serial='original-endpoint',adbSha256='a'*64))
        with patch.object(exchange,'binding',side_effect=binding),patch.object(journey,'bind_transport',side_effect=binding):
            try:
                with journey.ui_channel(enabled,helper,'sdk','read',Path('out'),inputs) as (client,channel,read):
                    self.assertEqual((channel,read),('channel','read-channel'))
                    self.assertIs(client.process,original.process);self.assertIs(client.output,original.output)
                    self.assertEqual((client.remote,client.deadline,client.ordinal),(original.remote,123.5,7))
                    self.assertIs(client.adb,original.adb)
                    self.assertEqual(client is original,not enabled)
                    client.ordinal=11
                    if fail:raise TimeoutError('original deadline')
            except TimeoutError:
                self.assertTrue(fail)
            else:self.assertFalse(fail)
        self.assertEqual(events[-2:],['binding-cleanup',('observer-cleanup',11)])
        expected='exchange_control.sh' if enabled else 'persistent_control.sh'
        self.assertEqual(events[1][2].name,expected)
        return events[1][1],original

    def test_default_disabled_channel_preserves_original_session_and_transport(self):
        self.run_channel(False)

    def test_memory_channel_reuses_one_observer_and_exact_original_deadline(self):
        self.run_channel(True)

    def test_failed_composite_request_keeps_original_cleanup_and_request_count(self):
        self.run_channel(True,fail=True)


class DefaultImeReadinessIntegrationTests(unittest.TestCase):
    def test_only_public_text_selection_uses_resized_viewport_readiness(self):
        value=journey.Journey.__new__(journey.Journey);candidates=lambda:[];selected=object()
        with patch.object(journey,'unique_input',return_value=selected)as settled,patch.object(journey.original.Journey,'unique')as original:
            self.assertIs(value.unique(candidates,30,intent='type-public',allow_scroll=True),selected)
        settled.assert_called_once_with(value,candidates,30,allow_scroll=True);original.assert_not_called()

    def test_other_native_actions_keep_original_readiness_and_deadline(self):
        value=journey.Journey.__new__(journey.Journey);candidates=lambda:[];selected=object()
        with patch.object(journey,'unique_input')as settled,patch.object(journey.original.Journey,'unique',return_value=selected)as original:
            self.assertIs(value.unique(candidates,30,intent='tap',allow_scroll=True),selected)
        original.assert_called_once_with(candidates,30,intent='tap',allow_scroll=True);settled.assert_not_called()
