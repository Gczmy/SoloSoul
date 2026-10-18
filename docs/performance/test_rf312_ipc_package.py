import hashlib
from pathlib import Path
import tempfile
import unittest
import warnings
import zipfile

import rf312_ipc_package as package


class DiagnosticPackageTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.base=self.root/'base.apk'
        with zipfile.ZipFile(self.base,'x') as archive:
            archive.writestr('AndroidManifest.xml',b'unchanged manifest')
            archive.writestr('classes.dex',b'unchanged classes')
            archive.writestr(package.CORE,b'default core')
        self.default=hashlib.sha256(b'default core').hexdigest()

    def test_changes_exactly_one_core_and_preserves_other_payloads(self):
        output=self.root/'diagnostic.apk'
        before,after=package.unsigned_copy(self.base,b'diagnostic core',self.default,output)
        self.assertEqual([name for name in before if before[name]!=after[name]],[package.CORE])
        self.assertEqual(package.verify_payload(self.base,output,after),after)

    def test_refuses_wrong_default_or_identical_new_core(self):
        for native,expected in [(b'new','a'*64),(b'default core',self.default)]:
            with self.subTest(native=native):
                with self.assertRaises(ValueError):package.unsigned_copy(self.base,native,expected,self.root/'bad.apk')

    def test_refuses_existing_output_without_overwrite(self):
        output=self.root/'exists.apk';output.write_bytes(b'preserve')
        with self.assertRaises(ValueError):package.unsigned_copy(self.base,b'new',self.default,output)
        self.assertEqual(output.read_bytes(),b'preserve')

    def test_refuses_duplicate_and_escaping_entries(self):
        for name in [package.CORE,'../escape','/absolute','windows\\escape']:
            with self.subTest(name=name):
                bad=self.root/'bad-source.apk'
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore',UserWarning)
                    with zipfile.ZipFile(bad,'w') as archive:
                        archive.writestr(package.CORE,b'default core');archive.writestr(name.replace(chr(92),'/'),b'bad')
                if chr(92) in name:
                    raw=bad.read_bytes();canonical=name.replace(chr(92),'/').encode()
                    self.assertEqual(raw.count(canonical),2)
                    bad.write_bytes(raw.replace(canonical,name.encode()))
                with self.assertRaises(ValueError):package.unsigned_copy(bad,b'new',self.default,self.root/'output.apk')

    def test_refuses_old_v1_signature_and_missing_core(self):
        for name in ['META-INF/CERT.RSA','no-core']:
            bad=self.root/'signed.apk'
            with zipfile.ZipFile(bad,'w') as archive:
                archive.writestr(name,b'bad')
                if name!='no-core':archive.writestr(package.CORE,b'default core')
            with self.assertRaises(ValueError):package.unsigned_copy(bad,b'new',self.default,self.root/'output.apk')

    def test_detects_additional_payload_change_after_signing(self):
        output=self.root/'unsigned.apk';_,expected=package.unsigned_copy(self.base,b'new',self.default,output)
        bad=self.root/'mutated.apk'
        with zipfile.ZipFile(output) as source,zipfile.ZipFile(bad,'x') as target:
            for info in source.infolist():target.writestr(info,b'changed' if info.filename=='classes.dex' else source.read(info.filename))
        with self.assertRaises(ValueError):package.verify_payload(self.base,bad,expected)


    def config(self):
        return dict(schemaVersion=1,runtimeInputs=str(self.root/'runtime.json'),runtimeInputsSha256='a'*64,
            diagnosticLibraryReceipt=str(self.root/'library.json'),diagnosticLibraryReceiptSha256='b'*64,
            defaultPackageConfig=str(self.root/'config.json'),defaultPackageConfigSha256='c'*64,
            toolSha256=dict(zipalign='d'*64))

    def test_complete_explicit_configuration_is_accepted(self):
        package.validate_config_shape(self.config())

    def test_boolean_schema_missing_and_extra_fields_are_rejected(self):
        for update in [dict(schemaVersion=True),dict(features=['devtools']),dict(schemaVersion=2)]:
            value=self.config();value.update(update)
            with self.subTest(update=update),self.assertRaises(ValueError):package.validate_config_shape(value)
        value=self.config();del value['runtimeInputsSha256']
        with self.assertRaises(ValueError):package.validate_config_shape(value)

    def test_receipt_paths_and_digests_are_not_implicit_or_coerced(self):
        for key,value in [('runtimeInputs','relative.json'),('diagnosticLibraryReceipt',True),
                          ('defaultPackageConfigSha256','wrong'),('runtimeInputsSha256',False)]:
            config=self.config();config[key]=value
            with self.subTest(key=key,value=value),self.assertRaises(ValueError):package.validate_config_shape(config)

    def test_alignment_tool_requires_one_exact_declared_hash(self):
        for tools in [{},{'zipalign':'d'*64,'java':'e'*64},{'zipalign':'wrong'},True]:
            config=self.config();config['toolSha256']=tools
            with self.subTest(tools=tools),self.assertRaises(ValueError):package.validate_config_shape(config)

    def test_certificate_must_be_single_and_exact(self):
        line='Signer #1 certificate SHA-256 digest: '+'a'*64+'\n'
        package.check_certificate(line,'a'*64)
        for text in ['',line+line,line.replace('a'*64,'b'*64)]:
            with self.subTest(text=text),self.assertRaises(ValueError):package.check_certificate(text,'a'*64)

    def test_case_distinct_apk_resources_remain_distinct(self):
        with zipfile.ZipFile(self.base,'a') as archive:
            archive.writestr('res/2F.xml',b'upper');archive.writestr('res/2f.xml',b'lower')
        _,after=package.unsigned_copy(self.base,b'new',self.default,self.root/'case.apk')
        self.assertNotEqual(after['res/2F.xml'],after['res/2f.xml'])

    def test_noncanonical_directory_and_separator_names_are_rejected(self):
        for name in ['dir//','nested//file','./file','C:drive','../folder/']:
            bad=self.root/'malformed.apk'
            with zipfile.ZipFile(bad,'w') as archive:
                archive.writestr(package.CORE,b'default core');archive.writestr(name,b'bad')
            with self.subTest(name=name),self.assertRaises(ValueError):
                package.unsigned_copy(bad,b'new',self.default,self.root/'not-written.apk')


if __name__=='__main__':unittest.main()
