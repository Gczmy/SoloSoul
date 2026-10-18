import re
import sys
from pathlib import Path
import frame_binding
import build_observer as default_observer
APP='com.solosoul.app'
PACKAGE='org.solosoul.rf312frames'
CLASS=PACKAGE+'.FrameCollector'
ANDROID='http://schemas.android.com/apk/res/android'

def compiled_identity(text):
    stack, nodes = [], []
    for line in text.splitlines():
        match = re.fullmatch(r'( *)E: ([A-Za-z][A-Za-z0-9_.-]*) \(line=\d+\)', line)
        if match:
            depth = len(match[1])
            while stack and stack[-1]['depth'] >= depth:
                stack.pop()
            node = dict(tag=match[2], depth=depth, parent=stack[-1]['tag'] if stack else None, attributes={})
            nodes.append(node); stack.append(node); continue
        match = re.fullmatch(r'( *)A: ([^=]+)=(.*)', line)
        if match:
            if not stack or len(match[1]) <= stack[-1]['depth']:
                raise ValueError('Compiled attribute has no owning element')
            name = match[2].split('(', 1)[0]
            if name in stack[-1]['attributes']:
                raise ValueError('Duplicate compiled collector attribute')
            stack[-1]['attributes'][name] = match[3]; continue
        if line.strip() and not re.fullmatch(r' *N: \S+ \(line=\d+\)', line):
            raise ValueError('Unknown compiled collector manifest entry')
    roots = [node for node in nodes if node['parent'] is None]
    if (len(roots) != 1 or roots[0]['tag'] != 'manifest' or len(nodes) != 4
            or sorted(node['tag'] for node in nodes if node['parent'] == 'manifest') != ['application', 'instrumentation', 'uses-sdk']):
        raise ValueError('Exact direct collector manifest shape required')

    def string(node, key):
        value = node['attributes'].get(key)
        match = re.fullmatch(r'"([^"\\]*)"(?: \(Raw: "\1"\))?', value or '')
        if not match:
            raise ValueError('Unambiguous compiled collector string required')
        return match[1]

    def flags(node, expected):
        if set(node['attributes']) != {ANDROID+':'+key for key in expected}:
            raise ValueError('Unexpected compiled collector attributes')
        for key, value in expected.items():
            allowed = ('true', '(type 0x12)0xffffffff') if value else ('false', '(type 0x12)0x0')
            if node['attributes'][ANDROID+':'+key] not in allowed:
                raise ValueError('Explicit compiled collector booleans required')

    root = roots[0]
    if string(root, 'package') != PACKAGE or root['attributes'].get(ANDROID+':compileSdkVersion') != '36':
        raise ValueError('Exact compiled package and compile SDK required')
    if root['attributes'].get(ANDROID+':versionCode') != '1' or string(root, ANDROID+':versionName') != '0.0.1':
        raise ValueError('Exact diagnostic collector version required')
    allowed_root = {'package', 'platformBuildVersionCode', 'platformBuildVersionName',
                    ANDROID+':versionCode', ANDROID+':versionName', ANDROID+':compileSdkVersion', ANDROID+':compileSdkVersionCodename'}
    if set(root['attributes']) != allowed_root:
        raise ValueError('Unexpected root identity or shared UID attributes')
    if (root['attributes']['platformBuildVersionCode'] != '36'
            or root['attributes']['platformBuildVersionName'] != '16'
            or string(root, ANDROID+':compileSdkVersionCodename') != '16'):
        raise ValueError('Exact platform build version required')
    uses = next(node for node in nodes if node['tag'] == 'uses-sdk')
    if uses['attributes'] != {ANDROID+':minSdkVersion': '29', ANDROID+':targetSdkVersion': '35'}:
        raise ValueError('Exact collector minimum and target SDK required')
    app = next(node for node in nodes if node['tag'] == 'application')
    flags(app, dict(hasCode=True, debuggable=False, allowBackup=False))
    instrument = next(node for node in nodes if node['tag'] == 'instrumentation')
    for name, value in dict(name=CLASS, targetPackage=APP, targetProcesses=APP).items():
        if string(instrument, ANDROID+':'+name) != value:
            raise ValueError('Exact target instrumentation identity required')
    remaining = dict(instrument, attributes={key: value for key, value in instrument['attributes'].items()
                                          if key not in {ANDROID+':'+name for name in ['name', 'targetPackage', 'targetProcesses']}})
    flags(remaining, dict(functionalTest=True, handleProfiling=False))
    try:
        default_observer.compiled_manifest_identity(text)
    except ValueError as error:
        if str(error) != 'Product package cannot be an observer target':
            raise
    else:
        raise ValueError('Default observer reader accepted target instrumentation')
    return dict(package=PACKAGE, instrumentationName=CLASS, instrumentationTarget=APP, targetProcesses=APP)

