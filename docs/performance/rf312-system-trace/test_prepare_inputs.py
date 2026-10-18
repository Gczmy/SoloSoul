from pathlib import Path
import hashlib
import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import prepare_inputs as m


class PrepareInputs(unittest.TestCase):
    def test_complete_payload_and_byte_bindings_in_space_path(self):
        with tempfile.TemporaryDirectory() as root:
            folder = Path(root) / "prepared input with spaces"
            receipt = m.prepare(folder)
            self.assertFalse(receipt["sdkStarted"])
            self.assertFalse(receipt["deviceAdmitted"])
            self.assertFalse(receipt["fullRF312Accepted"])
            for name, expected in receipt["frozenInputs"].items():
                self.assertEqual(hashlib.sha256((folder / name).read_bytes()).hexdigest(), expected)
            spec = importlib.util.spec_from_file_location("prepared_transport", folder / "transport.py")
            transport = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(transport)
            nonce = json.loads((folder / "file-plan.json").read_bytes())["nonce"]
            self.assertEqual((folder / "trace-config.pbtxt").read_bytes(),
                             transport.writer_config((folder / "base-config.pbtxt").read_bytes(),
                                                     transport.remote_paths(nonce)[2]))
            self.assertEqual((folder / "roundtrip.bin").read_bytes(), transport.ROUNDTRIP)

    def test_existing_directory_keeps_all_original_bytes(self):
        with tempfile.TemporaryDirectory() as root:
            folder = Path(root) / "existing"
            folder.mkdir()
            (folder / "keep").write_bytes(b"original")
            with self.assertRaises(ValueError):
                m.prepare(folder)
            self.assertEqual(list(folder.iterdir()), [folder / "keep"])
            self.assertEqual((folder / "keep").read_bytes(), b"original")

    def test_existing_file_is_preserved(self):
        with tempfile.TemporaryDirectory() as root:
            folder = Path(root) / "existing"
            folder.write_bytes(b"original")
            with self.assertRaises(ValueError):
                m.prepare(folder)
            self.assertEqual(folder.read_bytes(), b"original")

    def test_missing_parent_does_not_create_partial_tree(self):
        with tempfile.TemporaryDirectory() as root:
            parent = Path(root) / "absent"
            with self.assertRaises(ValueError):
                m.prepare(parent / "new")
            self.assertFalse(parent.exists())

    def test_source_binding_tamper_prevents_destination_creation(self):
        with tempfile.TemporaryDirectory() as root:
            source = Path(root) / "source"
            source.mkdir()
            for name in m.PAYLOAD:
                shutil.copyfile(m.HERE / name, source / name)
            (source / "transport.py").write_bytes(b"changed")
            destination = Path(root) / "new"
            with patch.object(m, "HERE", source), self.assertRaises(ValueError):
                m.prepare(destination)
            self.assertFalse(destination.exists())

    def test_cannot_prepare_under_source(self):
        with tempfile.TemporaryDirectory() as root:
            source = Path(root)
            with patch.object(m, "HERE", source), self.assertRaises(ValueError):
                m.prepare(source / "new")
            self.assertFalse((source / "new").exists())

    def test_fresh_calls_have_distinct_remote_file_identity(self):
        with tempfile.TemporaryDirectory() as root:
            folders = [Path(root) / name for name in ("one", "two")]
            for folder in folders:
                m.prepare(folder)
            self.assertNotEqual((folders[0] / "file-plan.json").read_bytes(),
                                (folders[1] / "file-plan.json").read_bytes())
            self.assertNotEqual((folders[0] / "trace-config.pbtxt").read_bytes(),
                                (folders[1] / "trace-config.pbtxt").read_bytes())

    def test_optimized_cli_rejects_existing_destination(self):
        with tempfile.TemporaryDirectory() as root:
            result = subprocess.run([sys.executable, "-B", "-O", str(m.HERE / "prepare_inputs.py"), root],
                                    capture_output=True, timeout=5)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, b"")
            self.assertIn(b"Destination must not exist", result.stderr)


if __name__ == "__main__":
    unittest.main()
