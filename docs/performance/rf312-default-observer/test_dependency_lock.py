import copy
import json
from pathlib import Path
import tempfile
import unittest

import dependency_lock as lock


class InstalledLock(unittest.TestCase):
    def setUp(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.root=Path(temporary.name).resolve();self.project=self.root/'project';self.project.mkdir()
        self.dependencies=self.root/'node_modules';self.dependencies.mkdir()
        self.package=dict(name='fixture',version='1.0.0',dependencies={'one':'^1.0.0'})
        self.record=dict(version='1.0.1',resolved='https://registry.invalid/one.tgz',integrity='sha512-fixture')
        self.product=dict(name='fixture',version='1.0.0',lockfileVersion=3,
            packages={'':copy.deepcopy(self.package),'node_modules/one':copy.deepcopy(self.record)})
        self.installed=dict(lockfileVersion=3,packages={'node_modules/one':copy.deepcopy(self.record)})
        (self.dependencies/'one').mkdir();(self.dependencies/'one/package.json').write_text('{"version":"1.0.1"}')
        self.write()

    def write(self):
        for path,value in [(self.project/'package.json',self.package),(self.project/'package-lock.json',self.product),
                           (self.dependencies/'.package-lock.json',self.installed)]:
            path.write_text(json.dumps(value),encoding='utf-8')

    def test_exact_records_and_actual_versions_pass_without_claiming_tarball_verification(self):
        result=lock.verify(self.project,self.dependencies)
        self.assertEqual(result['installedPackages'],1)
        self.assertTrue(result['actualPackageVersionsMatch'])
        self.assertFalse(result['registryTarballIntegrityRecomputed'])

    def test_record_version_source_or_integrity_substitution_refuses(self):
        for key in ('version','resolved','integrity'):
            with self.subTest(key=key):
                self.installed['packages']['node_modules/one'][key]='changed';self.write()
                with self.assertRaisesRegex(ValueError,'record differs'):lock.verify(self.project,self.dependencies)
                self.installed['packages']['node_modules/one']=copy.deepcopy(self.record)

    def test_actual_package_version_must_match_installed_record(self):
        (self.dependencies/'one/package.json').write_text('{"version":"1.0.0"}')
        with self.assertRaisesRegex(ValueError,'Actual installed'):lock.verify(self.project,self.dependencies)

    def test_direct_required_package_and_unknown_or_linked_package_refuse(self):
        self.installed['packages']['node_modules/other']=copy.deepcopy(self.record);self.write()
        with self.assertRaisesRegex(ValueError,'Unknown or linked'):lock.verify(self.project,self.dependencies)
        self.installed['packages'].pop('node_modules/other');self.installed['packages']['node_modules/one']['link']=True;self.write()
        with self.assertRaisesRegex(ValueError,'Unknown or linked'):lock.verify(self.project,self.dependencies)
        self.installed['packages'].pop('node_modules/one');self.installed['packages']['node_modules/other']=copy.deepcopy(self.record);self.write()
        with self.assertRaisesRegex(ValueError,'Required direct'):lock.verify(self.project,self.dependencies)

    def test_root_identity_declared_dependencies_and_lock_version_refuse(self):
        original=copy.deepcopy(self.product)
        for changed in ('identity','dependencies','version'):
            self.product=copy.deepcopy(original)
            if changed=='identity':self.product['version']='2.0.0'
            elif changed=='dependencies':self.product['packages']['']['dependencies']={}
            else:self.product['lockfileVersion']=True
            self.write()
            with self.subTest(changed=changed),self.assertRaises(ValueError):lock.verify(self.project,self.dependencies)

    def test_package_paths_cannot_escape_dependency_root(self):
        for name in ('node_modules/../outside','node_modules//one','outside/one','node_modules/C:one','node_modules/x\\y'):
            with self.subTest(name=name),self.assertRaises(ValueError):lock.installed_path(self.dependencies,name)


if __name__=='__main__':unittest.main()
