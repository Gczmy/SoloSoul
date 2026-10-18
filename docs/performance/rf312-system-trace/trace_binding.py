"""Exact local adapter binding; original public readers stay intact."""
from pathlib import Path
import hashlib
import json

ROOT=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
FIELDS={'schemaVersion','adapterInputs','adapterInputsSha256','runtimeInputs','runtimeInputsSha256',
        'sources','originalSources','inputCases','powershell','powershellSha256'}

def verify(plan):
    import trace_session
    legacy = set(plan)==FIELDS and type(plan.get('schemaVersion')) is int and plan['schemaVersion']==1
    startup = (set(plan)==FIELDS|{'traceProfile'} and type(plan.get('schemaVersion')) is int
               and plan['schemaVersion']==2 and plan.get('traceProfile')=='startup-am')
    if (not (legacy or startup)
            or set(plan['inputCases'])!={'100','5000'}):
        raise ValueError('Exact full-journey trace binding required')
    required={'trace_binding.py','trace_journey.py','trace_session.py','transport.py',
              'timeout_streams.py','trace_driver.py','trace_native_owner.py',
              'service_state.py','prepare_journey.py'}
    required_paths={str(ROOT/name) for name in required}
    original_paths={str(ROOT.parent/name) for name in (
        'rf312_default_state_driver.py','rf312_default_journey.py','rf312-native-queue/owner.py')}
    if (not required_paths <= set(plan['sources'])
            or set(plan['originalSources']) != original_paths
            or set(plan['sources']) & set(plan['originalSources'])):
        raise ValueError('Exact executed local modules and disjoint original bindings required')
    for path,digest in {**plan['sources'],**plan['originalSources']}.items():
        source=Path(path)
        if not source.is_absolute() or source.is_symlink() or sha(source)!=digest:
            raise ValueError('Frozen trace adapter or original source changed')
    for name in ['adapterInputs','runtimeInputs','powershell']:
        if sha(plan[name])!=plan[name+'Sha256']:
            raise ValueError('Declared trace input changed')
    roots=[Path(case['root']).resolve() for case in plan['inputCases'].values()]
    if len(set(roots))!=2:raise ValueError('Separate small/large input identities required')
    for case in plan['inputCases'].values():
        folder=Path(case['root'])
        if (not folder.is_dir() or folder.is_symlink()
                or set(case['files'])!={'file-plan.json','base-config.pbtxt','trace-config.pbtxt','roundtrip.bin'}):
            raise ValueError('Complete session case required')
        if {p.name for p in folder.iterdir()}!=set(case['files']):
            raise ValueError('Unexpected session input files')
        for name,digest in case['files'].items():
            if (folder/name).is_symlink() or sha(folder/name)!=digest:
                raise ValueError('Frozen session input changed')
        file_plan = json.loads((folder/'file-plan.json').read_bytes())
        if set(file_plan) != {'nonce'}:
            raise ValueError('Exact session nonce required')
        remote = trace_session.transport.remote_paths(file_plan['nonce'])[2]
        expected = trace_session.config((folder/'base-config.pbtxt').read_bytes(), remote,
                                        profile=plan.get('traceProfile','default'))
        if ((folder/'trace-config.pbtxt').read_bytes()!=expected
                or (folder/'roundtrip.bin').read_bytes()!=trace_session.transport.ROUNDTRIP):
            raise ValueError('Session profile differs from exact declared configuration')
    return plan

def load(path,*,runtime=None,adapter=None):
    def unique(pairs):
        out={}
        for key,value in pairs:
            if key in out:raise ValueError('Duplicate trace binding field')
            out[key]=value
        return out
    plan=json.loads(Path(path).read_bytes(),object_pairs_hook=unique)
    verify(plan)
    if runtime is not None and Path(plan['runtimeInputs']).resolve()!=Path(runtime).resolve():
        raise ValueError('Runtime differs from trace binding')
    if adapter is not None and Path(plan['adapterInputs']).resolve()!=Path(adapter).resolve():
        raise ValueError('State adapter differs from trace binding')
    return plan
