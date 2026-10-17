from pathlib import Path
import importlib.util,json,unittest,xml.etree.ElementTree as ET
S=Path(__file__).parent
spec=importlib.util.spec_from_file_location('existing_plain_tests',S/'test_observer_protocol.py')
t=importlib.util.module_from_spec(spec);spec.loader.exec_module(t)
class Probe(unittest.TestCase):
 setUp=t.Protocol.setUp
 client=t.Protocol.client
 def node(self):
  return ET.Element('node',{'package':t.m.APP,'class':'android.widget.EditText','text':'','content-desc':'','password':'true','bounds':'[1,2][10,20]','enabled':'true','visible-to-user':'true'})
 def response(self,**extra):
  row=t.result(action='inspect-target',intent='tap',matchCount=1,matched=True,inputDispatched=False,
   targetEnabled=True,targetVisible=True,targetFocused=False,targetFocusable=True,targetClickable=True,
   compatibleCount=1,candidateBounds=['[1,2][10,20]'],imeVisible=True,imeWindowCount=1,imeBounds=['[0,1200][1080,1920]'])
  row.update(extra);return row
 def test_exact_probe_is_read_only(self):
  client=self.client(self.response());probe=client.inspect(self.node(),'tap',5)
  self.assertTrue(t.m.Client.target_ready(self.node(),probe,'tap'))
  sent=json.loads(client.commands[0][1]['input']);self.assertEqual(sent['action'],'inspect-target')
  self.assertIs(probe['inputDispatched'],False)
 def test_changed_target_waits_without_dispatch(self):
  client=self.client(self.response(matchCount=0,matched=False))
  self.assertFalse(t.m.Client.target_ready(self.node(),client.inspect(self.node(),'tap',5),'tap'))
 def test_boolean_count_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'inspection receipt'):self.client(self.response(matchCount=True)).inspect(self.node(),'tap',5)
 def test_dispatched_probe_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'inspection receipt'):self.client(self.response(inputDispatched=True)).inspect(self.node(),'tap',5)
 def test_impossible_exact_match_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'inspection receipt'):self.client(self.response(matchCount=0)).inspect(self.node(),'tap',5)
 def test_other_intent_receipt_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'inspection receipt'):self.client(self.response(intent='type-public')).inspect(self.node(),'tap',5)
 def test_non_boolean_flags_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'readiness flags'):self.client(self.response(targetFocused=1)).inspect(self.node(),'tap',5)
 def test_unbounded_candidate_geometry_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'candidate geometry'):self.client(self.response(compatibleCount=5001)).inspect(self.node(),'tap',5)
 def test_unfocused_native_input_not_ready(self):
  probe=self.response(intent='type-public');self.assertFalse(t.m.Client.target_ready(self.node(),probe,'type-public'))
  probe['targetFocused']=True;self.assertTrue(t.m.Client.target_ready(self.node(),probe,'type-public'))
 def test_unknown_intent_rejected_before_request(self):
  client=self.client(self.response())
  with self.assertRaises(ValueError):client.inspect(self.node(),'generic-action',5)
  self.assertEqual(client.ordinal,0)
 def test_cold_ime_not_yet_visible_refuses_typing(self):
  probe=self.response(intent='type-public',targetFocused=True,imeVisible=False,imeWindowCount=0,imeBounds=[])
  self.assertFalse(t.m.Client.target_ready(self.node(),probe,'type-public'))
 def test_ambiguous_ime_window_not_ready(self):
  probe=self.response(intent='type-public',targetFocused=True,imeWindowCount=2)
  self.assertFalse(t.m.Client.target_ready(self.node(),probe,'type-public'))
 def test_boolean_ime_count_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'IME geometry'):self.client(self.response(imeWindowCount=True)).inspect(self.node(),'tap',5)
 def test_inconsistent_ime_visibility_rejected(self):
  with self.assertRaisesRegex(RuntimeError,'IME geometry'):self.client(self.response(imeVisible=False)).inspect(self.node(),'tap',5)
if __name__=='__main__':unittest.main(verbosity=1)
