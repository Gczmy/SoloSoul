import ast
from pathlib import Path
from types import SimpleNamespace
import builtins,symtable,tempfile,unittest
from unittest.mock import Mock,patch
import rf312_default_state_driver as driver


class DefaultDriverSourceTests(unittest.TestCase):
    def test_all_runtime_paths_have_their_required_global_bindings(self):
        import rf312_default_journey as journey
        import rf312_default_memory as memory
        import rf312_state_gate as gate
        import rf312_default_binding as binding
        import rf312_journey_memory as trajectory
        def required(scope):
            names={symbol.get_name() for symbol in scope.get_symbols() if symbol.is_global() and symbol.is_referenced()}
            for child in scope.get_children():names.update(required(child))
            return names
        for module,name in [(driver,'main'),(journey,'observe'),(journey,'ui_channel'),(memory,'collect'),(gate,'Gate'),
                            (binding,'load'),(binding,'sources'),(binding,'shape'),
                            (trajectory,'Stream'),(trajectory,'session'),(trajectory,'census'),(trajectory,'verify_census')]:
            table=symtable.symtable(Path(module.__file__).read_text(encoding='utf-8'),module.__file__,'exec')
            entry=next(child for child in table.get_children() if child.get_name()==name)
            with self.subTest(module=module.__name__,entry=name):
                self.assertEqual(required(entry)-set(vars(module))-set(vars(builtins)),set())
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.inputs=self.root/'inputs';self.inputs.write_bytes(b'original')
        self.binding={'strict':'original default runtime and adapter'}

    def frozen(self):
        tree=ast.parse(Path(driver.__file__).read_text(encoding='utf-8'));main=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='main')
        function=next(n for n in main.body if isinstance(n,ast.FunctionDef) and n.name=='frozen')
        runtime=dict(host=dict(workspaceRoot=str(self.root)),runtimeSourceFiles={},
            release=dict(apk=dict(path=str(self.root/'apk'),sha256='a'*64)),originalDebug=dict(path=str(self.root/'debug'),sha256='a'*64),
            originalTest=dict(path=str(self.root/'test'),sha256='a'*64),runtimeTest=dict(path=str(self.root/'probe'),sha256='a'*64))
        source=Mock();binding=SimpleNamespace(load=Mock(return_value=self.binding))
        namespace=dict(Path=Path,args=SimpleNamespace(inputs=self.inputs,state_inputs=self.root/'binding'),input_bytes=b'original',
            frozen_inputs=runtime,state_inputs=self.binding,verify_sources=source,state_binding=binding,digest=lambda file:'a'*64)
        exec(compile(ast.Module(body=[function],type_ignores=[]),'<actual-default-source-guard>','exec'),namespace)
        return namespace['frozen'],source,binding

    def test_original_sources_and_new_explicit_binding_are_both_revalidated(self):
        frozen,source,binding=self.frozen();frozen();source.assert_called_once();binding.load.assert_called_once()

    def test_changed_runtime_bytes_refused_before_validation(self):
        frozen,source,binding=self.frozen();self.inputs.write_bytes(b'changed')
        with self.assertRaisesRegex(RuntimeError,'Declared runtime inputs changed'):frozen()
        source.assert_not_called();binding.load.assert_not_called()

    def test_changed_adapter_binding_cannot_continue_native_inputs(self):
        frozen,source,binding=self.frozen();binding.load.return_value={'strict':'changed'}
        with self.assertRaises(AssertionError):frozen()
        source.assert_called_once()


class IndependentCompositeCliTests(unittest.TestCase):
    def main_tree(self):
        return next(n for n in ast.parse(Path(driver.__file__).read_text(encoding='utf-8')).body if isinstance(n,ast.FunctionDef)and n.name=='main')

    def test_actual_cli_defaults_and_both_flags_remain_independent(self):
        tree=self.main_tree();index=next(i for i,n in enumerate(tree.body)if isinstance(n,ast.Assign)and any(isinstance(t,ast.Name)and t.id=='args'for t in n.targets))
        tree.body=tree.body[:index+1]+[ast.Return(ast.Name('args',ast.Load()))]
        module=ast.fix_missing_locations(ast.Module(body=[tree],type_ignores=[]))
        scope=dict(argparse=driver.argparse,Path=Path,__doc__=driver.__doc__)
        exec(compile(module,'<actual-default-cli>','exec'),scope)
        base=['--adb','owned-adb','--serial','emulator-5578','--avd','SoloSoul_Public','--apk','release','--test-apk','probe','--output','unused','--mode','supported','--restore-apk','debug','--object-count','100','--fixture','fixture','--inputs','inputs','--state-inputs','binding']
        for sample,composite in [(False,False),(False,True),(True,False),(True,True)]:
            flags=(['--journey-memory']if sample else[])+(['--composite-ui']if composite else[])
            with self.subTest(sample=sample,composite=composite),patch.object(driver.os.sys,'argv',['driver',*base,*flags]):args=scope['main']()
            self.assertIs(args.journey_memory,sample);self.assertIs(args.composite_ui,composite)

    def test_actual_native_call_forwards_both_selected_flags(self):
        statement=next(n for n in ast.walk(self.main_tree())if isinstance(n,ast.Assign)and any(isinstance(t,ast.Name)and t.id=='evidence'for t in n.targets))
        options=next(n for n in ast.walk(self.main_tree())if isinstance(n,ast.Assign)and any(isinstance(t,ast.Name)and t.id=='channel_options'for t in n.targets))
        for sample,composite in [(False,False),(False,True),(True,False),(True,True)]:
            observe=Mock();adb=object();read=object();output=Path('unused');runtime={}
            scope=dict(external=SimpleNamespace(observe=observe),adb=adb,read=read,output=output,
                args=SimpleNamespace(object_count=100,journey_memory=sample,composite_ui=composite),
                original_packages={'main':{'uid':10105}},report={'publicOcrInput':{'fileName':'public-image'}},frozen_inputs=runtime)
            for owned in (None,{'artifact':'explicit-package.jar'}):
                scope['state_inputs']={}if owned is None else {'ownedUi':owned};observe.reset_mock()
                with self.subTest(sample=sample,composite=composite,owned=owned):exec(compile(ast.Module(body=[options,statement],type_ignores=[]),'<actual-observe-call>','exec'),scope)
                expected={}if owned is None else dict(owned_ui=owned)
                observe.assert_called_once_with(adb,read,output,100,10105,'public-image',runtime,memory_sampling=sample,composite_ui=composite,**expected)
