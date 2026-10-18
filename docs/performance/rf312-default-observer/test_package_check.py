import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
import package_check as package

sha=lambda raw:hashlib.sha256(raw).hexdigest()


class PackageBytes(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup);self.root=Path(temporary.name)
        self.core=b'original default native core';self.public=b'known public resource'
        self.manifest=dict(schema=1,version='a'*64,files=[dict(path='models/public.txt',size=len(self.public),sha256=sha(self.public))])
        self.raw=json.dumps(self.manifest).encode();self.rows={'assets/bundled-resource-manifest.json':self.raw,
            'assets/models/public.txt':self.public,'assets/dependency.bin':b'dependency','assets/profile.bin':b'profile',
            'lib/x86_64/libsolo_soul.so':self.core}
        self.current=self.root/'release.apk';self.old=self.root/'previous.apk';self.debug=self.root/'debug.apk'
        for path in (self.current,self.old,self.debug):self.write(path,self.rows)
        self.reference=dict(previousRelease=dict(path=str(self.old),sha256=package.digest(self.old),bytes=self.old.stat().st_size),
            appAssets={'bundled-resource-manifest.json':sha(self.raw),'models/public.txt':sha(self.public)},generatedProfiles={'profile.bin':sha(b'profile')})
        self.inputs=dict(release=dict(apk=dict(path=str(self.current)),library=dict(sha256=sha(self.core)),
            assetFilesVerified=2,generatedProfileFilesVerified=1,totalAssetFilesVerified=4,dependencyAssetFilesVerified=1),
            originalDebug=dict(path=str(self.debug)))

    def write(self,path,rows):
        with zipfile.ZipFile(path,'w') as archive:
            for name,value in rows.items():archive.writestr(name,value)

    def verify(self):return package.verify_bytes(self.inputs,self.reference)

    def test_all_original_groups_and_debug_resources_are_required(self):
        self.assertEqual(self.verify(),self.manifest)
        self.assertEqual(len(self.reference['appAssets']),2)

    def test_previous_reference_cannot_be_changed_or_replaced(self):
        self.old.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'Previous verified'):self.verify()

    def test_unknown_asset_missing_profile_and_changed_dependency_refuse(self):
        for kind in ('extra','profile','dependency'):
            rows=dict(self.rows)
            if kind=='extra':rows['assets/unknown.bin']=b'unknown'
            if kind=='profile':rows.pop('assets/profile.bin')
            if kind=='dependency':rows['assets/dependency.bin']=b'changed'
            self.write(self.current,rows)
            with self.subTest(kind=kind),self.assertRaises(ValueError):self.verify()

    def test_wrong_native_abi_and_changed_native_library_refuse(self):
        for kind in ('abi','core'):
            rows=dict(self.rows)
            if kind=='abi':rows['lib/arm64-v8a/libsolo_soul.so']=self.core
            else:rows['lib/x86_64/libsolo_soul.so']=b'changed'
            self.write(self.current,rows)
            with self.subTest(kind=kind),self.assertRaises(ValueError):self.verify()

    def test_restore_manifest_or_resource_difference_cannot_be_prepared(self):
        for name in ('assets/bundled-resource-manifest.json','assets/models/public.txt'):
            rows=dict(self.rows);rows[name]=b'changed';self.write(self.debug,rows)
            with self.subTest(name=name),self.assertRaises(ValueError):self.verify()

    def test_dropping_a_source_group_or_aliasing_it_into_profiles_refuses(self):
        self.reference['appAssets'].pop('models/public.txt')
        with self.assertRaises(ValueError):self.verify()

    def test_optional_observer_native_core_cannot_be_labeled_default(self):
        core=b'android-native-ipc-transport-observer';rows=dict(self.rows);rows['lib/x86_64/libsolo_soul.so']=core
        for path in (self.current,self.old):self.write(path,rows)
        self.reference['previousRelease'].update(sha256=package.digest(self.old),bytes=self.old.stat().st_size)
        self.inputs['release']['library']['sha256']=sha(core)
        with self.assertRaisesRegex(ValueError,'Optional'):self.verify()


if __name__=='__main__':unittest.main()
