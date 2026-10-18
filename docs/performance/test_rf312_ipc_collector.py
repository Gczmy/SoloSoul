import io
from pathlib import Path
import tempfile
import unittest
import zipfile

import rf312_ipc_collector as collector


# SDK xmltree syntax: this fixture describes only synthetic validation identities.
COMPILED = '''N: android=http://schemas.android.com/apk/res/android (line=1)
  E: manifest (line=1)
    A: http://schemas.android.com/apk/res/android:versionCode(0x0101021b)=1
    A: http://schemas.android.com/apk/res/android:versionName(0x0101021c)="0.0.1" (Raw: "0.0.1")
    A: http://schemas.android.com/apk/res/android:compileSdkVersion(0x01010572)=36
    A: http://schemas.android.com/apk/res/android:compileSdkVersionCodename(0x01010573)="16" (Raw: "16")
    A: package="org.solosoul.rf312ipc" (Raw: "org.solosoul.rf312ipc")
    A: platformBuildVersionCode=36
    A: platformBuildVersionName=16
      E: uses-sdk (line=2)
        A: http://schemas.android.com/apk/res/android:minSdkVersion(0x0101020c)=28
        A: http://schemas.android.com/apk/res/android:targetSdkVersion(0x01010270)=35
      E: application (line=3)
        A: http://schemas.android.com/apk/res/android:hasCode(0x0101000c)=true
        A: http://schemas.android.com/apk/res/android:debuggable(0x0101000f)=false
        A: http://schemas.android.com/apk/res/android:allowBackup(0x01010280)=false
      E: instrumentation (line=4)
        A: http://schemas.android.com/apk/res/android:name(0x01010003)="org.solosoul.rf312ipc.Collector" (Raw: "org.solosoul.rf312ipc.Collector")
        A: http://schemas.android.com/apk/res/android:targetPackage(0x01010021)="com.solosoul.app" (Raw: "com.solosoul.app")
        A: http://schemas.android.com/apk/res/android:handleProfiling(0x01010022)=false
        A: http://schemas.android.com/apk/res/android:functionalTest(0x01010023)=true
        A: http://schemas.android.com/apk/res/android:targetProcesses(0x01010541)="com.solosoul.app" (Raw: "com.solosoul.app")
'''


class CollectorIdentityTests(unittest.TestCase):
    def setUp(self):
        self.raw = (collector.SOURCE/'AndroidManifest.xml').read_text(encoding='utf-8')

    def test_real_source_and_compiled_identity_agree(self):
        identity = collector.source_identity(self.raw)
        self.assertEqual(collector.compiled_identity(COMPILED), identity)
        self.assertEqual(identity['instrumentationTarget'], 'com.solosoul.app')

    def test_source_refuses_package_and_shared_uid_changes(self):
        for text in [self.raw.replace('package="org.solosoul.rf312ipc"', 'package="other"'),
                     self.raw.replace('<manifest ', '<manifest android:sharedUserId="android.uid.system" ')]:
            with self.subTest(text=text), self.assertRaises(ValueError):collector.source_identity(text)

    def test_source_refuses_wrong_target_process_or_entry(self):
        for name, value in [('targetPackage', 'other'), ('targetProcesses', '*'), ('name', 'other.Entry')]:
            old = collector.APP if name.startswith('target') else collector.CLASS
            text = self.raw.replace('android:'+name+'="'+old+'"', 'android:'+name+'="'+value+'"')
            with self.subTest(name=name), self.assertRaises(ValueError):collector.source_identity(text)

    def test_source_refuses_debuggable_permissions_and_nested_components(self):
        for text in [self.raw.replace('debuggable="false"', 'debuggable="true"'),
                     self.raw.replace('</manifest>', '<uses-permission android:name="android.permission.INTERNET"/></manifest>'),
                     self.raw.replace('hasCode="true" />', 'hasCode="true"><activity android:name="other"/></application>'),
                     self.raw.replace('targetSdkVersion="35"', 'targetSdkVersion="36"')]:
            self.assertNotEqual(text, self.raw)
            with self.subTest(text=text), self.assertRaises(ValueError):collector.source_identity(text)

    def test_compiled_refuses_platform_and_shared_uid_changes(self):
        for text in [COMPILED.replace('platformBuildVersionCode=36', 'platformBuildVersionCode=35'),
                     COMPILED.replace('platformBuildVersionName=16', 'platformBuildVersionName=15'),
                     COMPILED.replace('="16" (Raw: "16")', '="15" (Raw: "15")'),
                     COMPILED.replace('compileSdkVersion(0x01010572)=36', 'compileSdkVersion(0x01010572)=35'),
                     COMPILED.replace('    A: package=', '    A: sharedUserId="system"\n    A: package=')]:
            with self.subTest(text=text), self.assertRaises(ValueError):collector.compiled_identity(text)

    def test_compiled_refuses_wrong_targets_or_missing_process(self):
        for key in ['name', 'targetPackage', 'targetProcesses']:
            lines = COMPILED.splitlines(keepends=True)
            index = next(i for i, line in enumerate(lines) if '/android:'+key+'(' in line)
            value = collector.CLASS if key == 'name' else collector.APP
            changed = list(lines);changed[index] = changed[index].replace(value, 'wrong')
            for text in [''.join(changed), ''.join(lines[:index]+lines[index+1:])]:
                with self.subTest(key=key,text=text), self.assertRaises(ValueError):collector.compiled_identity(text)

    def test_compiled_accepts_explicit_typed_booleans(self):
        typed = COMPILED.replace('=true', '=(type 0x12)0xffffffff').replace('=false', '=(type 0x12)0x0')
        self.assertEqual(collector.compiled_identity(typed), collector.compiled_identity(COMPILED))

    def test_compiled_refuses_ambiguous_or_wrong_booleans(self):
        for old, new in [('debuggable(0x0101000f)=false', 'debuggable(0x0101000f)=true'),
                         ('handleProfiling(0x01010022)=false', 'handleProfiling(0x01010022)=true'),
                         ('hasCode(0x0101000c)=true', 'hasCode(0x0101000c)=1'),
                         ('functionalTest(0x01010023)=true', 'functionalTest(0x01010023)=(type 0x12)0x1')]:
            with self.subTest(new=new), self.assertRaises(ValueError):collector.compiled_identity(COMPILED.replace(old,new))

    def test_compiled_refuses_extra_or_nested_components(self):
        for text in [COMPILED+'      E: uses-permission (line=5)\n',
                     COMPILED.replace('      E: instrumentation', '          E: instrumentation'),
                     COMPILED.replace('      E: uses-sdk', '  E: uses-sdk')]:
            with self.subTest(text=text), self.assertRaises(ValueError):collector.compiled_identity(text)

    def test_compiled_refuses_duplicate_and_ownerless_attributes(self):
        attribute = '    A: package="org.solosoul.rf312ipc" (Raw: "org.solosoul.rf312ipc")\n'
        for text in [COMPILED.replace(attribute,attribute+attribute), attribute+COMPILED,
                     COMPILED.replace(attribute,attribute.replace('    A:', '  A:')),
                     COMPILED.replace(attribute,attribute+attribute.replace('package=', 'package(0x12345678)='))]:
            with self.subTest(text=text), self.assertRaises(ValueError):collector.compiled_identity(text)

    def test_compiled_refuses_unknown_lines_raw_mismatch_and_sdk_changes(self):
        for text in [COMPILED+'unknown\n', COMPILED.replace('(Raw: "0.0.1")', '(Raw: "wrong")'),
                     COMPILED.replace('minSdkVersion(0x0101020c)=28','minSdkVersion(0x0101020c)=27'),
                     COMPILED.replace('targetSdkVersion(0x01010270)=35','targetSdkVersion(0x01010270)=36')]:
            with self.subTest(text=text), self.assertRaises(ValueError):collector.compiled_identity(text)

    def test_existing_default_reader_still_refuses_product_instrumentation(self):
        with self.assertRaisesRegex(ValueError, '^Product package cannot be an observer target$'):
            collector.default_observer.compiled_manifest_identity(COMPILED)


class CollectorConfigurationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def config(self):
        return dict(schemaVersion=1, runtimeInputs=str(self.root/'runtime.json'), runtimeInputsSha256='a'*64,
            diagnosticLibraryReceipt=str(self.root/'library.json'), diagnosticLibraryReceiptSha256='b'*64,
            defaultPackageConfig=str(self.root/'package.json'), defaultPackageConfigSha256='c'*64,
            nodeExecutable=str(self.root/'node.exe'), toolSha256=dict(d8='d'*64, zipalign='e'*64, node='f'*64))

    def test_complete_explicit_config_accepted(self):
        collector.validate_config(self.config())

    def test_config_refuses_missing_extra_and_boolean_version(self):
        for update in [dict(schemaVersion=True),dict(schemaVersion=2),dict(target='other')]:
            config=self.config();config.update(update)
            with self.subTest(update=update),self.assertRaises(ValueError):collector.validate_config(config)
        config=self.config();del config['nodeExecutable']
        with self.assertRaises(ValueError):collector.validate_config(config)

    def test_config_refuses_relative_paths_and_non_hashes(self):
        for key,value in [('runtimeInputs','relative.json'),('nodeExecutable',True),
                          ('runtimeInputsSha256',False),('diagnosticLibraryReceiptSha256','a'*63)]:
            config=self.config();config[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):collector.validate_config(config)

    def test_config_requires_three_exact_tool_hashes(self):
        for tools in [{},True,dict(d8='d'*64,zipalign='e'*64,node='f'*64,java='a'*64),
                      dict(d8='wrong',zipalign='e'*64,node='f'*64)]:
            config=self.config();config['toolSha256']=tools
            with self.subTest(tools=tools),self.assertRaises(ValueError):collector.validate_config(config)

    def test_zip_metadata_and_payload_are_deterministic(self):
        blobs=[]
        for _ in range(2):
            stream=io.BytesIO()
            with zipfile.ZipFile(stream,'w') as archive:
                collector.fixed_member(archive,'classes.dex',b'synthetic dex')
            blobs.append(stream.getvalue())
        self.assertEqual(blobs[0],blobs[1])
        with zipfile.ZipFile(io.BytesIO(blobs[0])) as archive:
            self.assertEqual(archive.namelist(),['classes.dex'])
            self.assertEqual(archive.read('classes.dex'),b'synthetic dex')
            self.assertEqual(archive.getinfo('classes.dex').date_time,(1980,1,1,0,0,0))


if __name__ == '__main__':unittest.main()
