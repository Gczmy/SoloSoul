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
        'rf312-ipc-runtime/test_diagnostic_exchange.py')
FIELDS={'schemaVersion','runtimeInputs','runtimeInputsSha256','sourceSha256'}
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()


def sources():
    return {**{'default/'+name:value for name,value in runtime_units.sources().items()},
            **{name:sha(HERE/name) for name in PUBLIC}}


def shape(config):
    if (not isinstance(config,dict) or set(config)!=FIELDS or type(config['schemaVersion']) is not int
            or config['schemaVersion']!=1 or not isinstance(config['runtimeInputs'],str)
            or not Path(config['runtimeInputs']).is_absolute() or not isinstance(config['sourceSha256'],dict)
            or set(config['sourceSha256'])!=set(sources())):
        raise ValueError('Complete explicit default runtime and adapter source binding required')
    hash_value(config['runtimeInputsSha256'])
    for value in config['sourceSha256'].values():hash_value(value)


def load(path):
    path=ordinary(path);raw=path.read_bytes();config=decode(raw);shape(config)
    runtime_path=ordinary(config['runtimeInputs'])
    if sha(runtime_path)!=config['runtimeInputsSha256'] or config['sourceSha256']!=sources():
        raise ValueError('Default runtime or adapter source changed')
    runtime=load_inputs(runtime_path)
    if (path.read_bytes()!=raw or sha(runtime_path)!=config['runtimeInputsSha256']
            or config['sourceSha256']!=sources()):
        raise ValueError('Default inputs changed while validating original guards')
    return dict(config=config,runtime=runtime)
