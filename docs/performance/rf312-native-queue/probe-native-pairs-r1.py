from pathlib import Path
import argparse,hashlib,json,re,subprocess,sys

W=Path(__file__).resolve().parents[3];P=W/'docs/performance'
sys.path.insert(0,str(P))
import rf312_default_binding as binding
from adb_control import AdbRecorder
from rf312_memory_batches import Adapter,PROCESSES,SERVICES

parser=argparse.ArgumentParser();parser.add_argument('--native',type=Path,required=True);parser.add_argument('--inputs',type=Path,required=True)
from queue_context import require_assertions
require_assertions()
args=parser.parse_args();native=args.native.resolve(strict=True)
runtime=json.loads(args.inputs.read_bytes());host=runtime['host'];source=binding.sources()
assert len(source)==97 and not (native/'continue-native.json').exists() and not (native/'result.json').exists()
assert json.loads((native/'preflight-ready.json').read_bytes())['nativeTestsStarted']==0
assert hashlib.sha256(Path(host['adbExecutable']).read_bytes()).hexdigest()==host['adbSha256']
folder=native/'metadata-pair-probe-r1';folder.mkdir(exist_ok=False)
actual=folder/'actual-sdk';actual.mkdir(exist_ok=False)
sdk=AdbRecorder(host['adbExecutable'],host['serial'],actual)
def direct(label,*command):
    value=sdk.run(*command,timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    (folder/(label+'.stdout')).write_bytes(value.stdout);(folder/(label+'.stderr')).write_bytes(value.stderr)
    assert value.returncode==0 and value.stderr==b''
    return value.stdout
name_raw=direct('avd-name','emu','avd','name');normalized=name_raw.replace(b'\r\r\n',b'\n').replace(b'\r\n',b'\n');assert b'\r' not in normalized and normalized.splitlines()==[host['avd'].encode(),b'OK']
pid=direct('launcher-pid','shell','pidof','com.android.launcher3').strip();assert re.fullmatch(rb'[1-9][0-9]*',pid)
pid=pid.decode();adapter=Adapter(sdk.run,folder/'paired-native',10105)
inventory=adapter.run(*PROCESSES,timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
services=adapter.run(*SERVICES,timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
assert inventory.returncode==services.returncode==0 and inventory.stderr==services.stderr==b''
assert b'ACTIVITY MANAGER RUNNING PROCESSES' in inventory.stdout
assert b'ACTIVITY MANAGER SERVICES' in services.stdout
stat=adapter.run('shell','cat',f'/proc/{pid}/stat',timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
status=adapter.run('shell','cat',f'/proc/{pid}/status',timeout=10,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
assert stat.returncode==status.returncode==0 and stat.stderr==status.stderr==b''
assert stat.stdout.startswith(pid.encode()+b' (')
assert re.search(rb'^Pid:\s*'+pid.encode()+rb'\s*$',status.stdout,re.M)
adapter.require_empty();transport=adapter.close()
assert transport['nativeCalls']==2 and transport['logicalRepliesConsumed']==4 and transport['allRepliesConsumed']
assert source==binding.sources()
result=dict(passed=True,readOnly=True,actualEvidenceDirectory='metadata-pair-probe-r1',consoleIdentityCrcrlfVerified=True,nativeUiTestsStarted=0,applicationMemorySamplesAccepted=0,
    positiveInventoryPair=True,positiveLauncherKernelPair=True,logicalReads=4,physicalPairSdkCalls=2,
    separateStdoutAndStderrVerified=True,memberExitCodesVerified=True,noPayloadTransformation=True,
    ownAvdNameVerified=True,sourceSha256=source,
    actualSdkLedgerSha256=hashlib.sha256((actual/'commands.json').read_bytes()).hexdigest(),
    transportReceiptSha256=hashlib.sha256((folder/'paired-native/result.json').read_bytes()).hexdigest(),
    probeSourceSha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
with (folder/'probe.json').open('x',encoding='utf-8') as out:out.write(json.dumps(result,indent=2)+'\n')
(native/'metadata-pair-probe').mkdir(exist_ok=False)
with (native/'metadata-pair-probe/probe.json').open('x',encoding='utf-8') as out:out.write(json.dumps(result,indent=2)+'\n')
print(json.dumps({key:value for key,value in result.items() if key!='sourceSha256'}))
