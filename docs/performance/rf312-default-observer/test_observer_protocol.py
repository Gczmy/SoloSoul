from pathlib import Path
import importlib.util,json,tempfile,types,unittest
S=Path(__file__).parent
spec=importlib.util.spec_from_file_location('journey_protocol',S/'observer_session.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

def result(action='snapshot',**kwargs):
    value=dict(action=action,success=True,rootPackage=m.APP,allowedRoot=True,
        nativeStartedAtNs=1,nativeFinishedAtNs=2,xml='<hierarchy/>')
    value.update(kwargs);return value
class FakeClient(m.Client):
    def __init__(self,output,response):
        super().__init__(None,output,types.SimpleNamespace(poll=lambda:None),'files/'+'a'*32+'/')
        self.response=response;self.commands=[]
    def alive(self,deadline):pass
    def exists(self,name,deadline):return True
    def get(self,name,deadline):return json.dumps(self.response).encode()
    def call(self,*args,deadline,**kwargs):
        self.commands.append((args,kwargs));return types.SimpleNamespace(stdout=b'',stderr=b'',returncode=0)
class Protocol(unittest.TestCase):
    def setUp(self):
        root=S/'unit-temp';root.mkdir(exist_ok=True)
        self.temp=tempfile.TemporaryDirectory(prefix='owned-protocol-',dir=root)
        self.directory=Path(self.temp.name).resolve()
        self.assertTrue(self.directory.is_relative_to(root.resolve()))
        self.addCleanup(self.temp.cleanup)
    def client(self,response):return FakeClient(self.directory,response)
    def tap(self):
        return dict(action='tap',expectedPackage=m.APP,target=dict(package=m.APP,
            **{'class':'android.widget.Button','text':'Public','content-desc':'','password':'false','bounds':'[1,2][10,20]'}))
    def test_owned_snapshot(self):
        self.assertTrue(self.client(result()).snapshot(5)['success'])
    def test_unowned_empty_snapshot_is_wait_not_input(self):
        row=self.client(result(rootPackage='unrelated',allowedRoot=False)).snapshot(5)
        self.assertFalse(row['allowedRoot'])
    def test_unrelated_contents_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'Unrelated hierarchy'):
            self.client(result(rootPackage='unrelated',allowedRoot=False,xml='<hierarchy><node/></hierarchy>')).snapshot(5)
    def test_root_flag_mismatch_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'ownership'):
            self.client(result(rootPackage='unrelated')).snapshot(5)
    def test_incorrect_action_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'action mismatch'):
            self.client(result(action='back')).snapshot(5)
    def test_boolean_clock_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'clock'):
            self.client(result(nativeStartedAtNs=True)).snapshot(5)
    def test_backwards_clock_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'clock'):
            self.client(result(nativeFinishedAtNs=0)).snapshot(5)
    def test_wrong_input_owner_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'owner differs'):
            self.client(result(action='tap',rootPackage='com.android.documentsui')).request(self.tap(),5)
    def test_mouse_substitution_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'finger'):
            self.client(result(action='tap',singleFingerPair=True,toolType='mouse',bounds='[1,2][10,20]')).request(self.tap(),5)
    def test_missing_up_receipt_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'finger'):
            self.client(result(action='tap',singleFingerPair=False,toolType='finger',bounds='[1,2][10,20]')).request(self.tap(),5)
    def test_moved_target_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'finger'):
            self.client(result(action='tap',singleFingerPair=True,toolType='finger',bounds='[2,2][11,20]')).request(self.tap(),5)
    def test_native_key_pair_required(self):
        with self.assertRaisesRegex(RuntimeError,'keyboard'):
            self.client(result(action='type-public',publicTextOnly=True,keyEvents=3)).request(dict(action='type-public',expectedPackage=m.APP),5)
    def test_arbitrary_text_refused_before_request(self):
        client=self.client(result())
        with self.assertRaisesRegex(ValueError,'public fixture'):
            client.type_public(None,'not-a-public-fixture',5)
        self.assertEqual(client.ordinal,0)
    def test_longer_budget_refused(self):
        client=self.client(result())
        with self.assertRaisesRegex(ValueError,'45'):
            client.snapshot(46)
        self.assertEqual(client.ordinal,0)
    def test_untrusted_label_only_goes_to_stdin(self):
        value=self.tap();label="public'$(untrusted)";value['target']['text']=label
        client=self.client(result(action='tap',singleFingerPair=True,toolType='finger',bounds='[1,2][10,20]'))
        self.assertTrue(client.request(value,5)['singleFingerPair'])
        self.assertNotIn(label,client.commands[0][0][1])
        self.assertEqual(json.loads(client.commands[0][1]['input'])['target']['text'],label)
    def test_doctype_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'hierarchy receipt'):
            self.client(result(xml='<!DOCTYPE hierarchy><hierarchy/>')).snapshot(5)
if __name__=='__main__':unittest.main(verbosity=2)
