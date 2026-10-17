import copy
from pathlib import Path
import tempfile
import unittest
import zipfile

import build_observer as builder
import runtime_inputs


SOURCE = (Path(__file__).parent / 'AndroidManifest.xml').read_bytes()
TREE = '''N: android=http://schemas.android.com/apk/res/android (line=1)
  E: manifest (line=1)
    A: package="org.solosoul.rf312observer" (Raw: "org.solosoul.rf312observer")
      E: application (line=3)
        A: http://schemas.android.com/apk/res/android:label(0x01010001)="Observer" (Raw: "Observer")
      E: instrumentation (line=4)
        A: http://schemas.android.com/apk/res/android:name(0x01010003)=".Observer" (Raw: ".Observer")
        A: http://schemas.android.com/apk/res/android:targetPackage(0x01010021)="org.solosoul.rf312observer" (Raw: "org.solosoul.rf312observer")
'''


class Manifests(unittest.TestCase):
    def test_actual_source_and_compiled_tree_have_identical_self_identity(self):
        actual = builder.compiled_manifest_identity(TREE)
        self.assertEqual(actual, builder.source_manifest_identity(SOURCE))
        self.assertEqual(actual['instrumentationTarget'], builder.PACKAGE)
        self.assertEqual(actual['instrumentationName'], builder.PACKAGE + '.Observer')

    def test_root_package_or_label_cannot_substitute_for_target_package(self):
        # The old substring guard accepted this: the root still names our package.
        mutated = TREE.replace('targetPackage(0x01010021)="org.solosoul.rf312observer" (Raw: "org.solosoul.rf312observer")',
                               'targetPackage(0x01010021)="org.other.app" (Raw: "org.other.app")')
        self.assertIn('android:targetPackage', mutated)
        self.assertIn('"org.solosoul.rf312observer"', mutated)
        self.assertNotIn('"com.solosoul.app"', mutated)
        with self.assertRaises(ValueError):builder.compiled_manifest_identity(mutated)

    def test_product_target_remains_refused(self):
        mutated = TREE.replace('targetPackage(0x01010021)="org.solosoul.rf312observer"',
                               'targetPackage(0x01010021)="com.solosoul.app"')
        with self.assertRaisesRegex(ValueError, 'Product package'):builder.compiled_manifest_identity(mutated)
        with self.assertRaises(ValueError):
            builder.source_manifest_identity(SOURCE.replace(b'targetPackage="org.solosoul.rf312observer"',
                                                           b'targetPackage="com.solosoul.app"'))

    def test_wrong_package_or_instrumentation_class_refused(self):
        for tree in [TREE.replace('A: package="org.solosoul.rf312observer" (Raw: "org.solosoul.rf312observer")',
                                 'A: package="org.other.observer" (Raw: "org.other.observer")'),
                     TREE.replace('.Observer', '.Other')]:
            with self.subTest(tree=tree), self.assertRaises(ValueError):builder.compiled_manifest_identity(tree)

    def test_missing_or_duplicate_instrumentation_refused(self):
        head, instrumentation = TREE.split('      E: instrumentation', 1)
        for tree in [head, TREE + '      E: instrumentation' + instrumentation]:
            with self.subTest(tree=tree), self.assertRaises(ValueError):builder.compiled_manifest_identity(tree)
        duplicate = SOURCE.replace(b'</manifest>', SOURCE[SOURCE.index(b'  <instrumentation'):SOURCE.index(b'</manifest>')] + b'</manifest>')
        with self.assertRaises(ValueError):builder.source_manifest_identity(duplicate)

    def test_target_attribute_on_another_element_is_not_accepted(self):
        target = TREE.splitlines()[-1]
        mutated = TREE.replace(target + '\n', '').replace('      E: instrumentation', target + '\n      E: instrumentation')
        with self.assertRaises(ValueError):builder.compiled_manifest_identity(mutated)

    def test_nested_instrumentation_refused_in_both_formats(self):
        tree = TREE.replace('      E: instrumentation', '          E: instrumentation').replace(
            '        A: http://schemas.android.com/apk/res/android:name',
            '            A: http://schemas.android.com/apk/res/android:name').replace(
            '        A: http://schemas.android.com/apk/res/android:targetPackage',
            '            A: http://schemas.android.com/apk/res/android:targetPackage')
        with self.assertRaisesRegex(ValueError, 'direct manifest child'):builder.compiled_manifest_identity(tree)
        xml = SOURCE.replace(b'android:allowBackup="false"/>', b'android:allowBackup="false">').replace(
            b'</manifest>', b'</application></manifest>')
        with self.assertRaisesRegex(ValueError, 'direct manifest child'):builder.source_manifest_identity(xml)

    def test_duplicate_attribute_or_resource_id_alias_refused(self):
        target = TREE.splitlines()[-1]
        for extra in [target, target.replace('(0x01010021)', '(0x01010022)')]:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                builder.compiled_manifest_identity(TREE + extra + '\n')

    def test_mismatched_raw_string_and_unquoted_value_refused(self):
        for target in ['"org.solosoul.rf312observer" (Raw: "org.other.app")', 'org.solosoul.rf312observer']:
            tree = TREE.replace('targetPackage(0x01010021)="org.solosoul.rf312observer" (Raw: "org.solosoul.rf312observer")',
                                'targetPackage(0x01010021)=' + target)
            with self.subTest(target=target), self.assertRaises(ValueError):builder.compiled_manifest_identity(tree)

    def test_wrong_namespace_and_unknown_tree_syntax_refused(self):
        for tree in [TREE.replace('http://schemas.android.com/apk/res/android:targetPackage', 'other:targetPackage'),
                     TREE + 'unrecognized tree text\n', TREE.replace('E: manifest', 'E: other')]:
            with self.subTest(tree=tree), self.assertRaises(ValueError):builder.compiled_manifest_identity(tree)

    def test_multiple_roots_or_attributes_without_elements_refused(self):
        for tree in [TREE + TREE, ' A: package="org.solosoul.rf312observer"\n' + TREE]:
            with self.subTest(tree=tree), self.assertRaises(ValueError):builder.compiled_manifest_identity(tree)


class Receipt(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.apk = self.root/'observer.apk';self.apk.write_bytes(b'actual test observer bytes')
        self.dex = self.root/'classes.dex';self.dex.write_bytes(b'actual test dex bytes')
        self.source = {n:builder.digest(Path(__file__).parent/n) for n in ('Observer.java','AndroidManifest.xml')}
        self.identity = builder.compiled_manifest_identity(TREE)

    def receipt(self):
        return builder.package_receipt(self.apk, self.dex, self.source, self.identity, 'a'*64, 36, [])

    def test_receipt_binds_canonical_and_legacy_hash_size_and_verified_target(self):
        row = self.receipt()
        self.assertEqual(row['sha256'], builder.digest(self.apk))
        self.assertEqual(row['apkSha256'], row['sha256'])
        self.assertEqual(row['bytes'], self.apk.stat().st_size)
        self.assertEqual(row['sourceSha256'], self.source)
        self.assertEqual(row['dexSha256'], builder.digest(self.dex))
        self.assertFalse(row['targetInstrumentationInjected'])
        self.assertFalse(row['productInstalled'])
        self.assertFalse(row['nativeRuntimeAccepted'])

    def test_unverified_identity_cannot_create_passing_receipt(self):
        self.identity['instrumentationTarget'] = 'com.solosoul.app'
        with self.assertRaisesRegex(ValueError, 'Verified compiled'):
            self.receipt()

    def test_generated_receipt_is_accepted_by_unchanged_artifact_consumer(self):
        # Complete synthetic artifact bytes exercise the actual public consumer.
        library = self.root/'library.so';library.write_bytes(b'ordinary default native library')
        original = self.root/'original.apk';original.write_bytes(b'unchanged restore artifact')
        release = self.root/'release.apk'
        with zipfile.ZipFile(release, 'x') as archive:
            archive.writestr('lib/x86_64/libsolo_soul.so', library.read_bytes())
        row = lambda path:dict(path=str(path), sha256=builder.digest(path), bytes=path.stat().st_size)
        fixtures = {}
        for count in ('100','5000'):
            folder = self.root/count;folder.mkdir();file = folder/'fixture';file.write_bytes(count.encode())
            fixtures[count] = dict(root=str(folder),files={'fixture':builder.digest(file)})
        inputs = dict(compiledSourceHead='b'*40,
            release=dict(acceptedPackagePreparation=True,defaultFeaturesOnly=True,applicationDebuggable=False,
                observerMarkersPresent=False,sourceHead='b'*40,apk=row(release),
                library=dict(row(library),actualCargoReleaseBuilt=True,defaultFeaturesOnly=True,earlyObserverMarkersPresent=False)),
            originalDebug=row(original),originalTest=row(original),runtimeTest=row(original),observer=self.receipt(),
            fixedFixtures=fixtures,publicOcrImage=row(self.dex))
        runtime_inputs.verify_artifacts(inputs)
        for key in ('sha256','instrumentationTarget'):
            mutated = copy.deepcopy(inputs);mutated['observer'].pop(key)
            with self.subTest(key=key), self.assertRaises((ValueError,KeyError)):
                runtime_inputs.verify_artifacts(mutated)
        self.apk.write_bytes(b'tampered observer')
        with self.assertRaisesRegex(ValueError, 'separate observer'):
            runtime_inputs.verify_artifacts(inputs)


if __name__ == '__main__':unittest.main()
