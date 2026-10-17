"""Explicit diagnostic receipts coexist with the unchanged strict default inputs."""
import hashlib
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parent
PUBLIC=ROOT.parent
sys.path[:0]=[str(PUBLIC),str(PUBLIC/'rf312-default-observer')]
import rf312_ipc_collector as producer

FIELDS={'schemaVersion','runtimeInputs','runtimeInputsSha256','diagnosticPackageReceipt',
        'diagnosticPackageReceiptSha256','collectorPackageReceipt','collectorPackageReceiptSha256','avd',
        'adbExecutable','adbSha256','sourceSha256'}
LOCAL=('ipc_receipt.py','native_collector.py','diagnostic_ui_session.py','diagnostic_binding.py',
       'diagnostic_driver.py','diagnostic_journey.py','test_ipc_receipt.py','test_native_collector.py',
       'test_diagnostic_binding.py','test_diagnostic_journey.py','test_diagnostic_driver.py','diagnostic_owner.py')
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()


def sources():
    return {**{'collector/'+name:value for name,value in producer.sources().items()},
            **{name:sha(producer.HERE/name) for name in ['rf312_instrumentation_state.py','test_rf312_instrumentation_state.py','rf312_state_gate.py','test_rf312_state_gate.py','rf312_launch_identity.py','test_rf312_launch_identity.py']},
            **{name:sha(ROOT/name) for name in LOCAL}}


def config_shape(config):
    if (not isinstance(config,dict) or set(config)!=FIELDS or type(config['schemaVersion']) is not int
            or config['schemaVersion']!=1 or not isinstance(config['avd'],str)
            or re.fullmatch(r'SoloSoul_[A-Za-z0-9_-]+',config['avd']) is None):
        raise ValueError('Complete separate diagnostic configuration required')
    for name in ['runtimeInputs','diagnosticPackageReceipt','collectorPackageReceipt']:
        if not isinstance(config[name],str) or not Path(config[name]).is_absolute():
            raise ValueError('Absolute diagnostic receipt paths required')
        producer.package.producer.hash_value(config[name+'Sha256'])
    if not isinstance(config['adbExecutable'],str) or not Path(config['adbExecutable']).is_absolute():
        raise ValueError('Explicit absolute diagnostic SDK client required')
    producer.package.producer.hash_value(config['adbSha256'])
    if not isinstance(config['sourceSha256'],dict) or set(config['sourceSha256'])!=set(sources()):
        raise ValueError('Complete explicit diagnostic runtime source binding required')
    for value in config['sourceSha256'].values():producer.package.producer.hash_value(value)


def load(path):
    path=producer.ordinary(path)
    raw=path.read_bytes();config=producer.package.producer.decode(raw);config_shape(config)
    if config['sourceSha256']!=sources():raise ValueError('Diagnostic runtime source changed')
    runtime=producer.package.producer.bound_json(config['runtimeInputs'],config['runtimeInputsSha256'])
    package=producer.package.producer.bound_json(config['diagnosticPackageReceipt'],config['diagnosticPackageReceiptSha256'])
    collector=producer.package.producer.bound_json(config['collectorPackageReceipt'],config['collectorPackageReceiptSha256'])
    value=dict(config=config,runtime=runtime,package=package,collector=collector,
        host=dict(adbExecutable=config['adbExecutable'],adbSha256=config['adbSha256'],
                  avd=config['avd'],serial=runtime['host']['serial']))
    verify(value)
    if path.read_bytes()!=raw or config['sourceSha256']!=sources():
        raise ValueError('Diagnostic configuration or sources changed while validating')
    return value


def verify(value):
    config=value['config'];config_shape(config)
    if config['sourceSha256']!=sources():raise ValueError('Diagnostic runtime source changed')
    runtime=value['runtime'];package=value['package'];collector=value['collector']
    adb=producer.ordinary(config['adbExecutable']);producer.command_path(adb)
    if adb.name!='adb.exe' or adb.parent.name!='platform-tools' or sha(adb)!=config['adbSha256'] or config['adbSha256']!=runtime['host']['adbSha256']:
        raise ValueError('Explicit same-build diagnostic SDK client required')
    if producer.package.producer.load_inputs(config['runtimeInputs'])!=runtime:
        raise ValueError('Original complete strict default binding required')
    for name in ['runtimeInputs','diagnosticPackageReceipt','collectorPackageReceipt']:
        bound=producer.package.producer.bound_json(config[name],config[name+'Sha256'])
        if bound!=value[{'runtimeInputs':'runtime','diagnosticPackageReceipt':'package','collectorPackageReceipt':'collector'}[name]]:
            raise ValueError('Original diagnostic receipt changed')
    if (package.get('passed') is not True or package.get('actualPackageBuilt') is not True
            or package.get('defaultFeaturesOnly') is not False or package.get('allOtherPayloadsByteIdentical') is not True
            or package.get('onlyChangedPayload')!=producer.package.CORE
            or package.get('sourceSha256')!=producer.package.source_hashes()
            or package.get('baselineApkSha256')!=runtime['release']['apk']['sha256']
            or package.get('sourceHead')!=runtime['compiledSourceHead']):
        raise ValueError('Accepted explicit same-source diagnostic APK required')
    if (collector.get('passed') is not True or collector.get('sourceSha256')!=producer.sources()
            or collector.get('instrumentationTarget')!=producer.APP or collector.get('targetProcesses')!=producer.APP
            or collector.get('instrumentationName')!=producer.CLASS or collector.get('package')!=producer.PACKAGE
            or collector.get('applicationDebuggable') is not False or collector.get('defaultObserverCompatible') is not False
            or collector.get('defaultObserverReaderRejected') is not True or collector.get('targetInstrumentationCapable') is not True
            or collector.get('sourceHead')!=runtime['compiledSourceHead']
            or collector.get('runtimeInputsSha256')!=config['runtimeInputsSha256']):
        raise ValueError('Accepted explicit target metadata collector required')
    for receipt in [package,collector]:
        if (sha(producer.ordinary(receipt['apk']))!=receipt['sha256']
                or receipt['certificateSha256']!=runtime['release']['certificateSha256']
                or receipt.get('productionSigningUsed') is not False):
            raise ValueError('Diagnostic APK or existing internal signing identity changed')
    producer.package.verify_payload(Path(runtime['release']['apk']['path']),Path(package['apk']),package['payloadSha256'])
    import zipfile
    with zipfile.ZipFile(collector['apk']) as archive:
        names=archive.namelist()
        if len(names)!=4 or set(names)!={'AndroidManifest.xml','resources.arsc','assets/snapshot.js','classes.dex'}:
            raise ValueError('Exact metadata collector payload set required')
        if {name:hashlib.sha256(archive.read(name)).hexdigest() for name in names}!=collector['payloadSha256']:
            raise ValueError('Collector payload changed')
