"""Explicit default-state adapter inputs; original default package readers stay strict."""
import hashlib
from pathlib import Path
import sys

HERE=Path(__file__).resolve().parent
HELPER=HERE/'rf312-default-observer'
sys.path.insert(0,str(HELPER))
sys.path.insert(0,str(HERE/'rf312-ipc-runtime'))
import runtime_units
from runtime_inputs import decode,hash_value,load_inputs
from prepare_avd import ordinary

PUBLIC=('rf312_instrumentation_state.py','test_rf312_instrumentation_state.py',
        'rf312_state_gate.py','test_rf312_state_gate.py',
        'rf312_default_memory.py','test_rf312_default_memory.py',
        'rf312_default_journey.py','test_rf312_default_journey.py',
        'rf312_default_state_driver.py','test_rf312_default_state_driver.py',
        'rf312_default_binding.py','test_rf312_default_binding.py',
        'rf312_journey_memory.py','test_rf312_journey_memory.py',
        'rf312-ipc-runtime/diagnostic_exchange.py','rf312-ipc-runtime/exchange_control.sh',
        'rf312-ipc-runtime/test_diagnostic_exchange.py',
        'rf312_memory_batches.py','test_rf312_memory_batches.py',
        'rf312_ime_readiness.py','test_rf312_ime_readiness.py',
        'rf312_owned_ui.py','test_rf312_owned_ui.py',
        'rf312-owned-ui/OwnedUi.java','rf312-owned-ui/file_binding.py',
        'rf312-owned-ui/protocol_tests.py')
FIELDS={'schemaVersion','runtimeInputs','runtimeInputsSha256','sourceSha256'}
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()


def sources():
    return {**{'default/'+name:value for name,value in runtime_units.sources().items()},
            **{name:sha(HERE/name) for name in PUBLIC}}


def shape(config):
    if not isinstance(config,dict):raise ValueError('Explicit default adapter inputs required')
    version=config.get('schemaVersion')
    fields=FIELDS if version==1 else FIELDS|{'ownedUiPackage','ownedUiPackageSha256'}
    if (set(config)!=fields or type(version) is not int or version not in (1,2)
            or not isinstance(config['runtimeInputs'],str)
            or not Path(config['runtimeInputs']).is_absolute() or not isinstance(config['sourceSha256'],dict)
            or set(config['sourceSha256'])!=set(sources())):
        raise ValueError('Complete explicit default runtime and adapter source binding required')
    hash_value(config['runtimeInputsSha256'])
    for value in config['sourceSha256'].values():hash_value(value)
    if version==2:
        if not isinstance(config['ownedUiPackage'],str) or not Path(config['ownedUiPackage']).is_absolute():
            raise ValueError('Explicit absolute UI package receipt required')
        hash_value(config['ownedUiPackageSha256'])


def load(path):
    path=ordinary(path);raw=path.read_bytes();config=decode(raw);shape(config)
    runtime_path=ordinary(config['runtimeInputs'])
    if sha(runtime_path)!=config['runtimeInputsSha256'] or config['sourceSha256']!=sources():
        raise ValueError('Default runtime or adapter source changed')
    runtime=load_inputs(runtime_path)
    owned=None
    if config['schemaVersion']==2:
        from rf312_owned_ui import package
        owned=package(config['ownedUiPackage'],config['ownedUiPackageSha256'])
    if (path.read_bytes()!=raw or sha(runtime_path)!=config['runtimeInputsSha256']
            or config['sourceSha256']!=sources()):
        raise ValueError('Default inputs changed while validating original guards')
    result=dict(config=config,runtime=runtime)
    if owned is not None:result['ownedUi']=owned
    return result
