"""Actual Windows file-sharing and rejection tests; no application is launched."""
import ctypes
from ctypes import wintypes as w
from contextlib import contextmanager
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("owned_json", Path(__file__).with_name("native-perf-owned-json.py"))
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)


@unittest.skipUnless(os.name == "nt", "Requires actual Windows handles")
class OwnedJsonTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="ss-rf1128-json-")
        self.root = Path(self.directory.name)
        self.file = self.root / "published.json"
        self.file.write_bytes(b'{"publicSynthetic":true}\n')
        self.api = reader._api()

    def tearDown(self):
        self.directory.cleanup()

    @contextmanager
    def hold(self, access=0x80000000 | 0x10000, share=7):
        handle = self.api.CreateFileW(str(self.file), access, share, None, 3, 0, None)
        if handle == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            yield handle
        finally:
            self.assertTrue(self.api.CloseHandle(handle))

    def test_actual_delete_holder_rejects_default_read_but_shared_reader_preserves_bytes(self):
        with self.hold():
            with self.assertRaises(PermissionError) as failure:
                self.file.read_bytes()
            self.assertEqual(failure.exception.errno, 13)
            self.assertEqual(reader.read_owned_json(self.file), {"publicSynthetic": True})
        self.assertEqual(self.file.read_bytes(), b'{"publicSynthetic":true}\n')

    def test_exclusive_lock_remains_real_win32_error_without_retry(self):
        with self.hold(access=0x80000000, share=0):
            with self.assertRaises(OSError) as failure:
                reader.read_owned_json(self.file)
            self.assertEqual(failure.exception.winerror, 32)
        self.assertEqual(reader.read_owned_json(self.file), {"publicSynthetic": True})

    def test_closed_handles_allow_immediate_replacement_and_removal(self):
        for content in [b'{"v":1}', b'{"v":2}']:
            self.file.write_bytes(content)
            self.assertEqual(reader.read_owned_json(self.file), json.loads(content))
            replacement = self.root / "next.json"
            replacement.write_bytes(content)
            os.replace(replacement, self.file)
        self.file.unlink()

    def test_missing_path_is_not_created(self):
        missing = self.root / "missing.json"
        with self.assertRaises(OSError):
            reader.read_owned_json(missing)
        self.assertFalse(missing.exists())

    def test_invalid_paths_limits_and_directory_are_rejected(self):
        for path in [Path("relative.json"), Path(self.root.anchor), self.root]:
            with self.assertRaises((OSError, ValueError)):
                reader.read_owned_json(path)
        for maximum in [0, -1, True, 1.5, 4194305]:
            with self.assertRaises(ValueError):
                reader.read_owned_json(self.file, maximum)

    def test_exact_size_limit_bom_utf8_and_invalid_json(self):
        raw = b'\xef\xbb\xbf{"value":"\xe4\xb8\xad"}'
        self.file.write_bytes(raw)
        self.assertEqual(reader.read_owned_json(self.file, len(raw)), {"value": "中"})
        with self.assertRaises(ValueError):
            reader.read_owned_json(self.file, len(raw)-1)
        for raw in [b'{', b'{"v":"\xff"}']:
            self.file.write_bytes(raw)
            with self.assertRaises((UnicodeError, ValueError)):
                reader.read_owned_json(self.file)
        self.file.unlink()  # Parse failures must close the handles too.

    def test_final_file_symlink_is_rejected_before_content_read(self):
        alias = self.root / "alias.json"
        try:
            os.symlink(self.file, alias)
        except OSError as error:
            if error.winerror == 1314:
                self.skipTest("Actual file symlink fixture requires missing SeCreateSymbolicLinkPrivilege")
            raise
        try:
            self.symlink_reparse_tag = alias.lstat().st_reparse_tag
            self.assertEqual(self.symlink_reparse_tag, 0xA000000C)
            self.assertTrue(alias.is_symlink())
            read_file = self.api.ReadFile
            with patch.object(self.api, "ReadFile", wraps=read_file) as content_read:
                with patch.object(reader, "_api", return_value=self.api):
                    with self.assertRaises(ValueError):
                        reader.read_owned_json(alias)
                content_read.assert_not_called()
                self.symlink_content_read_calls = content_read.call_count
            self.assertEqual(self.file.read_bytes(), b'{"publicSynthetic":true}\n')
            self.symlink_target_preserved = True
        finally:
            alias.unlink()

    def test_ancestor_junction_is_rejected_before_content_read(self):
        target = self.root / "actual"
        target.mkdir()
        (target / "marker.json").write_bytes(b'{"foreign":true}')
        alias = self.root / "redirected"
        # Only a checked absolute fixture path is sent to the fixed mklink builtin.
        result = subprocess.run(["cmd", "/c", "mklink", "/J", str(alias), str(target)], capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        try:
            with self.assertRaises(ValueError):
                reader.read_owned_json(alias / "marker.json")
        finally:
            os.rmdir(alias)

    def test_replaced_name_is_rejected_even_with_identical_bytes(self):
        read_file = self.api.ReadFile
        swapped = False
        def replace_during_read(*args):
            nonlocal swapped
            result = read_file(*args)
            if not swapped:
                other = self.root / "replacement.json"
                other.write_bytes(self.file.read_bytes())
                retired = self.root / "retired.json"
                os.rename(self.file, retired)
                os.rename(other, self.file)
                swapped = True
            return result
        with patch.object(self.api, "ReadFile", side_effect=replace_during_read), patch.object(reader, "_api", return_value=self.api):
            with self.assertRaises(ValueError):
                reader.read_owned_json(self.file)
        self.assertTrue(swapped)

    def test_growth_during_read_is_rejected(self):
        read_file = self.api.ReadFile
        changed = False
        def grow_during_read(*args):
            nonlocal changed
            result = read_file(*args)
            if not changed:
                with self.file.open("ab") as file:
                    file.write(b' ' * 128)
                changed = True
            return result
        with patch.object(self.api, "ReadFile", side_effect=grow_during_read), patch.object(reader, "_api", return_value=self.api):
            with self.assertRaises(ValueError):
                reader.read_owned_json(self.file, 64)
        self.assertTrue(changed)

    def test_empty_dacl_remains_actual_access_denied_and_is_restored(self):
        security = ctypes.WinDLL("advapi32", use_last_error=True)
        security.GetFileSecurityW.argtypes = [w.LPCWSTR, w.DWORD, w.LPVOID, w.DWORD, ctypes.POINTER(w.DWORD)]
        security.GetFileSecurityW.restype = w.BOOL
        security.SetFileSecurityW.argtypes = [w.LPCWSTR, w.DWORD, w.LPVOID]
        security.SetFileSecurityW.restype = w.BOOL
        security.InitializeSecurityDescriptor.argtypes = [w.LPVOID, w.DWORD]
        security.InitializeSecurityDescriptor.restype = w.BOOL
        security.InitializeAcl.argtypes = [w.LPVOID, w.DWORD, w.DWORD]
        security.InitializeAcl.restype = w.BOOL
        security.SetSecurityDescriptorDacl.argtypes = [w.LPVOID, w.BOOL, w.LPVOID, w.BOOL]
        security.SetSecurityDescriptorDacl.restype = w.BOOL
        needed = w.DWORD()
        security.GetFileSecurityW(str(self.file), 4, None, 0, ctypes.byref(needed))
        original = ctypes.create_string_buffer(needed.value)
        self.assertTrue(security.GetFileSecurityW(str(self.file), 4, original, len(original), ctypes.byref(needed)))
        descriptor = ctypes.create_string_buffer(64)
        acl = ctypes.create_string_buffer(8)
        self.assertTrue(security.InitializeSecurityDescriptor(descriptor, 1))
        self.assertTrue(security.InitializeAcl(acl, len(acl), 2))
        self.assertTrue(security.SetSecurityDescriptorDacl(descriptor, True, acl, False))
        self.assertTrue(security.SetFileSecurityW(str(self.file), 4, descriptor))
        try:
            with self.assertRaises(OSError) as failure:
                reader.read_owned_json(self.file)
            self.assertEqual(failure.exception.winerror, 5)
        finally:
            self.assertTrue(security.SetFileSecurityW(str(self.file), 4, original))
        restored = ctypes.create_string_buffer(len(original))
        self.assertTrue(security.GetFileSecurityW(str(self.file), 4, restored, len(restored), ctypes.byref(needed)))
        self.assertEqual(restored.raw, original.raw)
        self.assertEqual(reader.read_owned_json(self.file), {"publicSynthetic": True})


if __name__ == "__main__":
    unittest.main()
