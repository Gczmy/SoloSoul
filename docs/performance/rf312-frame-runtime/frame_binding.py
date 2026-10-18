"""Separate, explicit native frame probe binding; default readers stay unchanged."""
import hashlib
import os
from pathlib import Path
import re
import sys
import zipfile

ROOT=Path(__file__).resolve().parent
PUBLIC=Path(os.environ.get('RF312_FRAME_SHARED_PUBLIC',str(ROOT.parent))).resolve()
WORK=PUBLIC.parents[1]
HELPER=PUBLIC/'rf312-default-observer'
LEGACY=PUBLIC/'rf312-ipc-runtime'
sys.path[:0]=[str(HELPER),str(LEGACY),str(PUBLIC)]
import diagnostic_binding as inherited
from runtime_inputs import load_inputs
producer=inherited.producer
require_assertions=inherited.require_assertions
sha=inherited.sha
LOCAL=('frame_binding.py','frame_owner.py','frame_driver.py','frame_ui_session.py',
       'frame_session.py','frame_journey.py','frame_receipt.py','test_frame_receipt.py','test_frame_runtime.py','window_trace.py','test_window_trace.py')
FIELDS={'schemaVersion','scope','sharedPublic','runtimeInputs','runtimeInputsSha256',
        'collectorPackageReceipt','collectorPackageReceiptSha256','avd','sourceSha256'}


def sources():
    return {**{'inherited/'+name:value for name,value in inherited.sources().items()},
            **{name:sha(ROOT/name) for name in LOCAL}}


def shape(config):
    if (type(config) is not dict or set(config)!=FIELDS or type(config['schemaVersion']) is not int
            or config['schemaVersion']!=1 or config['scope']!='explicit-default-apk-native-frame-commits'
            or not isinstance(config['avd'],str) or re.fullmatch(r'SoloSoul_[A-Za-z0-9_-]+',config['avd']) is None):
        raise ValueError('Exact separate native frame configuration required')
    for name in ['runtimeInputs','collectorPackageReceipt','sharedPublic']:
        if type(config[name]) is not str or not Path(config[name]).is_absolute():
            raise ValueError('Explicit absolute frame input paths required')
    if Path(config['sharedPublic']).resolve()!=PUBLIC:raise ValueError('Declared shared source root differs')
    for name in ['runtimeInputsSha256','collectorPackageReceiptSha256']:
        producer.package.producer.hash_value(config[name])
    if config['sourceSha256']!=sources():raise ValueError('Complete inherited and frame source binding required')


def verify(value):
    require_assertions()
    config=value['config'];shape(config)
    runtime=load_inputs(config['runtimeInputs'])
    if runtime!=value['runtime']:raise ValueError('Original complete default runtime binding changed')
    if sha(config['runtimeInputs'])!=config['runtimeInputsSha256']:raise ValueError('Runtime input bytes changed')
    collector=producer.package.producer.bound_json(config['collectorPackageReceipt'],config['collectorPackageReceiptSha256'])
    if collector!=value['collector']:raise ValueError('Frame package receipt changed')
    if (collector.get('passed') is not True or collector.get('productionSigningUsed') is not False
            or collector.get('sourceAndToolsPreserved') is not True or collector.get('originalDefaultObserverReaderRejected') is not True
            or collector.get('defaultTargetApkSha256')!=runtime['release']['apk']['sha256']
            or collector.get('certificateSha256')!=runtime['release']['certificateSha256']
            or collector.get('compiledManifestIdentity')!={'package':'org.solosoul.rf312frames',
               'instrumentationName':'org.solosoul.rf312frames.FrameCollector','instrumentationTarget':'com.solosoul.app',
               'targetProcesses':'com.solosoul.app'} or collector.get('minSdk')!=29 or collector.get('targetSdk')!=35):
        raise ValueError('Exact independently built native frame actor required')
    if sha(producer.ordinary(collector['apk']))!=collector['sha256']:raise ValueError('Frame actor APK changed')
    if any(sha(producer.ordinary(path))!=digest for path,digest in collector['sourceSha256'].items()):
        raise ValueError('Frame actor producer sources changed')
    with zipfile.ZipFile(collector['apk']) as archive:
        names=archive.namelist()
        if len(names)!=3 or set(names)!={'AndroidManifest.xml','resources.arsc','classes.dex'}:
            raise ValueError('Exact frame actor payload required')
        if {name:hashlib.sha256(archive.read(name)).hexdigest() for name in names}!=collector['payloadSha256']:
            raise ValueError('Frame actor payload changed')
    expected={'apk':runtime['release']['apk']['path'],'sha256':runtime['release']['apk']['sha256'],
              'defaultFeaturesOnly':True,'sourceHead':runtime['compiledSourceHead']}
    if value['package']!=expected:raise ValueError('Frame probe must target unchanged default APK')


def load(path):
    require_assertions()
    path=producer.ordinary(path);raw=path.read_bytes()
    config=producer.package.producer.decode(raw);shape(config)
    runtime=load_inputs(config['runtimeInputs'])
    collector=producer.package.producer.bound_json(config['collectorPackageReceipt'],config['collectorPackageReceiptSha256'])
    value=dict(config=config,runtime=runtime,collector=collector,
        package=dict(apk=runtime['release']['apk']['path'],sha256=runtime['release']['apk']['sha256'],
                     defaultFeaturesOnly=True,sourceHead=runtime['compiledSourceHead']),
        host=dict(adbExecutable=runtime['host']['adbExecutable'],adbSha256=runtime['host']['adbSha256'],
                  avd=config['avd'],serial=runtime['host']['serial']))
    verify(value)
    if path.read_bytes()!=raw or config['sourceSha256']!=sources():raise ValueError('Frame inputs changed while validating')
    return value
