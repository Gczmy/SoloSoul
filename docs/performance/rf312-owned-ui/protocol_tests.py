from pathlib import Path
import argparse,hashlib,io,json,os,subprocess,sys,tempfile,unittest
R=Path(__file__).resolve().parent
P=R.parent
sys.path[:0]=[str(P/'rf312-default-observer'),str(P/'rf312-ipc-runtime')]
import diagnostic_exchange as exchange
from persistent_control import read_reply
parser=argparse.ArgumentParser(description='Host JVM protocol checks with explicit Android test stubs; never packages stubs into the native artifact')
parser.add_argument('--jdk',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args();J=args.jdk/'bin';ROOT=args.output.absolute();ROOT.mkdir(exist_ok=False)
classes=ROOT/'classes';classes.mkdir();stubs=ROOT/'stubs';stubs.mkdir()
sources={
 'android/os/Process.java':'package android.os; public final class Process {public static int myUid(){return 10001;}}',
 'android/os/SystemClock.java':'''package android.os; public final class SystemClock {
 static long calls=0;
 public static long elapsedRealtimeNanos(){return Boolean.getBoolean("rf312.test.fastclock")?calls++*50000000000L:System.nanoTime();}
 public static void sleep(long ms){try{Thread.sleep(ms);}catch(InterruptedException e){throw new RuntimeException(e);}}
 }''',
 'android/system/StructStat.java':'package android.system; public class StructStat {public int st_uid=10001,st_mode=0700;}',
 'android/system/Os.java':'package android.system; public final class Os {public static StructStat stat(String p){return new StructStat();}public static int umask(int m){return 0;}}'}
for name,text in sources.items():
 file=stubs/name;file.parent.mkdir(parents=True,exist_ok=True);file.write_text(text,encoding='utf-8')
env=dict(os.environ,TEMP=str(ROOT),TMP=str(ROOT))
for name in ['JAVA_TOOL_OPTIONS','_JAVA_OPTIONS','JDK_JAVA_OPTIONS']:env.pop(name,None)
cmd=[str(J/('javac.exe'if os.name=='nt'else'javac')),'--release','11','-d',str(classes),str(R/'OwnedUi.java'),*[str(stubs/name)for name in sources]]
c=subprocess.run(cmd,capture_output=True,timeout=30,env=env,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
(ROOT/'javac.stdout').write_bytes(c.stdout);(ROOT/'javac.stderr').write_bytes(c.stderr);c.check_returncode()

class ProtocolTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory(dir=ROOT);self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
  self.token='a'*32;self.base=self.root/'files'/self.token;(self.base/'owned-ui').mkdir(parents=True)
 def command(self,*,fast=False,uid='10001'):
  args=[str(J/('java.exe'if os.name=='nt'else'java')),'-Duser.home='+str(self.root),'-Djava.io.tmpdir='+str(self.root)]
  if fast:args.append('-Drf312.test.fastclock=true')
  return args+['-cp',str(classes),'org.solosoul.rf312transport.OwnedUi',self.token,'files/'+self.token+'/owned-ui',uid]
 def actor(self,*,fast=False):
  proc=subprocess.Popen(self.command(fast=fast),cwd=self.root,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
  control=exchange.Control(proc,self.root/'rpc');self.addCleanup(control.abort);return control
 def raw(self,wire,**kwargs):
  return subprocess.run(self.command(**kwargs),cwd=self.root,env=env,input=wire,capture_output=True,timeout=5,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
 def test_ping_and_normal_close(self):
  c=self.actor();self.assertEqual(c.call('ping',timeout=3).stdout,b'rf312-control-ready\n');c.close();self.assertEqual(c.process.returncode,0)
 def test_stable_bytes_include_nul_crlf_and_png(self):
  raw=b'\x89PNG\r\n\x1a\npublic\0\r\n';(self.base/'frame-001.png').write_bytes(raw)
  c=self.actor();self.assertEqual(c.call('get-stable','frame-001.png',timeout=3).stdout,raw);c.close()
 def test_stale_request_refused_without_overwrite(self):self.stale('request-001.json',64)
 def test_stale_response_refused_without_publication(self):self.stale('response-001.json',64)
 def test_stale_temporary_refused_without_publication(self):self.stale('request-001.tmp',64)
 def test_error_marker_refuses_without_publication(self):self.stale('error',70)
 def stale(self,name,code):
  file=self.base/name;file.write_bytes(b'preserve');c=self.actor()
  with self.assertRaises(subprocess.CalledProcessError)as error:c.call('exchange','request-001.json',b'{}',timeout=3)
  self.assertEqual(error.exception.returncode,code);self.assertEqual(error.exception.stdout,error.exception.stderr,b'')
  self.assertEqual(file.read_bytes(),b'preserve')
  if name!='request-001.json':self.assertFalse((self.base/'request-001.json').exists())
  c.close()
 def test_ordered_collision_keeps_request_precedence(self):
  (self.base/'error').write_bytes(b'error');self.stale('request-001.json',64)
 def test_invalid_path_is_rejected_as_data(self):
  value=self.raw(b'RF312B 1 exchange ../request-001.json e30=\nRF312B 2 close - -\n');self.assertEqual(value.returncode,0)
  stream=io.BytesIO(value.stdout);self.assertEqual(read_reply(stream,1,bytearray()),(64,b'',b''));self.assertEqual(read_reply(stream,2,bytearray()),(0,b'',b''))
  self.assertFalse((self.root/'files/request-001.json').exists())
 def test_non_ui_opcode_closes_without_execution(self):
  value=self.raw(b'RF312B 1 focus - -\n');self.assertEqual(value.returncode,64);self.assertEqual(value.stdout,b'')
 def test_sequence_replay_closes_after_first_frame(self):
  value=self.raw(b'RF312B 1 ping - -\nRF312B 1 ping - -\n');self.assertEqual(value.returncode,64)
  stream=io.BytesIO(value.stdout);self.assertEqual(read_reply(stream,1,bytearray()),(0,b'rf312-control-ready\n',b''));self.assertEqual(stream.read(),b'')
 def test_oversized_read_refuses_before_data(self):
  (self.base/'frame-001.png').write_bytes(b'x'*(4194304+1));c=self.actor()
  with self.assertRaises(subprocess.CalledProcessError)as error:c.call('get-stable','frame-001.png',timeout=3)
  self.assertEqual(error.exception.returncode,65);self.assertEqual(error.exception.stdout,b'');c.close()
 def test_native_deadline_branch_with_test_only_clock_stub(self):
  c=self.actor(fast=True)
  with self.assertRaises(subprocess.CalledProcessError)as error:c.call('exchange','request-001.json',b'{"public":true}',timeout=3)
  self.assertEqual(error.exception.returncode,71);self.assertEqual(error.exception.stdout,error.exception.stderr,b'')
  self.assertEqual((self.base/'request-001.json').read_bytes(),b'{"public":true}');c.close()
 def test_partial_or_non_ascii_input_never_accepted(self):
  for wire in [b'RF312B 1 ping - -',b'RF312B 1 ping - \0\n']:
   with self.subTest(wire=wire):
    value=self.raw(wire);self.assertEqual(value.returncode,64);self.assertEqual(value.stdout,b'')
 def test_wrong_uid_refused_before_reply(self):
  value=self.raw(b'RF312B 1 ping - -\n',uid='10002');self.assertEqual(value.returncode,64);self.assertEqual(value.stdout,b'')

if __name__=='__main__':
 with(ROOT/'tests.log').open('x',encoding='utf-8')as log:result=unittest.TextTestRunner(stream=log,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(ProtocolTests))
 receipt=dict(passed=result.wasSuccessful(),tests=result.testsRun,failures=len(result.failures),errors=len(result.errors),skipped=len(result.skipped),scope='host-JVM with explicit Android test stubs',nativeAndroidUidOrClockProven=False,stubClassesIncludedInNativeArtifact=False,workerSourceSha256=hashlib.sha256((R/'OwnedUi.java').read_bytes()).hexdigest())
 (ROOT/'result.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8');print(json.dumps(receipt));sys.exit(0 if result.wasSuccessful()else 1)
