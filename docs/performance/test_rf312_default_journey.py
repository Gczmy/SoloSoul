import ast
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
