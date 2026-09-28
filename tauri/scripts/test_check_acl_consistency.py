"""使用临时注册/ACL/契约文件验证真实检查入口，不依赖仓库生成物。"""

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from check_acl_consistency import main, validate_contract_manifest


class ContractAclTests(unittest.TestCase):
    def setUp(self):
        fixture = tempfile.TemporaryDirectory(prefix="solosoul-contract-acl-")
        self.addCleanup(fixture.cleanup)
        self.root = Path(fixture.name)
        self.lib = self.root / "lib.rs"
        self.acl = self.root / "default.toml"
        self.manifest_path = self.root / "ipcContractManifest.json"
        self.lib.write_text(
            "tauri::generate_handler![commands::system::get_app_info,];\n"
            "tauri::generate_handler![#[cfg(desktop)] commands::auth::unlock,];\n",
            encoding="utf-8",
        )
        self.write_acl(["get_app_info", "unlock"])
        self.manifest = {
            "schemaVersion": 1,
            "generator": {"name": "solosoul-ipc-contract-gen", "version": "0.1.0"},
            "commands": ["get_app_info"],
            "events": [],
            "unmigratedCommands": ["unlock"],
            "sources": ["src-tauri/src/lib.rs", "src-tauri/src/commands/system.rs"],
        }
        self.write_manifest()

    def write_acl(self, names):
        self.acl.write_text(
            "[[permission]]\ncommands.allow = " + json.dumps(names) + "\n",
            encoding="utf-8",
        )

    def write_manifest(self):
        self.manifest_path.write_text(json.dumps(self.manifest), encoding="utf-8")

    def check(self):
        output, errors = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            status = main(self.lib, self.acl, self.manifest_path)
        return status, output.getvalue(), errors.getvalue()

    def assert_rejected(self, message):
        status, output, errors = self.check()
        self.assertNotEqual(status, 0)
        self.assertIn(message, errors)
        self.assertNotIn("OK:", output)

    def test_valid_incremental_partition_reports_actual_coverage_without_writing(self):
        before = {path.name: path.read_bytes() for path in self.root.iterdir()}
        status, output, errors = self.check()
        self.assertEqual(status, 0, errors)
        self.assertIn("2 个命令均已登记", output)
        self.assertIn("已迁移 1 个命令，未迁移 1 个命令", output)
        self.assertIn("不表示全量", output)
        self.assertEqual(errors, "")
        self.assertEqual(before, {path.name: path.read_bytes() for path in self.root.iterdir()})

    def test_acl_leftover_remains_warning(self):
        self.write_acl(["get_app_info", "unlock", "removed_legacy_command"])
        status, output, errors = self.check()
        self.assertEqual(status, 0, errors)
        self.assertIn("WARN:", output)
        self.assertIn("removed_legacy_command", output)

    def test_registered_command_missing_acl_fails_even_when_unmigrated(self):
        self.write_acl(["get_app_info"])
        self.assert_rejected("unlock")

    def test_migrated_command_requires_acl(self):
        with self.assertRaisesRegex(ValueError, "已迁移命令缺少 ACL.*get_app_info"):
            validate_contract_manifest(
                self.manifest_path, {"get_app_info", "unlock"}, {"unlock"}
            )

    def test_missing_registered_command_in_manifest_fails(self):
        self.manifest["unmigratedCommands"] = []
        self.write_manifest()
        self.assert_rejected("清单缺少：['unlock']")

    def test_extra_manifest_command_fails_even_if_acl_contains_it(self):
        self.manifest["commands"].append("unregistered")
        self.write_manifest()
        self.write_acl(["get_app_info", "unlock", "unregistered"])
        self.assert_rejected("清单多出：['unregistered']")

    def test_migrated_unmigrated_overlap_fails(self):
        self.manifest["unmigratedCommands"].append("get_app_info")
        self.write_manifest()
        self.assert_rejected("已迁移/未迁移命令重叠")

    def test_duplicate_list_entries_fail(self):
        for key, value in (
            ("commands", "get_app_info"),
            ("unmigratedCommands", "unlock"),
            ("events", "synthetic-progress"),
            ("sources", "src-tauri/src/lib.rs"),
        ):
            with self.subTest(key=key):
                original = self.manifest[key]
                self.manifest[key] = [value, value]
                self.write_manifest()
                self.assert_rejected(f"{key} 包含重复项")
                self.manifest[key] = original

    def test_malformed_or_duplicate_key_json_fails(self):
        for raw, expected in (
            ("{", "IPC 契约清单校验失败"),
            ('{"schemaVersion":1,"schemaVersion":1}', "重复 JSON 键"),
            ("[]", "顶层必须为对象"),
        ):
            with self.subTest(raw=raw):
                self.manifest_path.write_text(raw, encoding="utf-8")
                self.assert_rejected(expected)

    def test_invalid_schema_or_missing_required_fields_fail(self):
        bad_fields = [
            ("schemaVersion", True),
            ("schemaVersion", 2),
            ("generator", {"name": "other", "version": "0.1.0"}),
            ("generator", {"name": "solosoul-ipc-contract-gen", "version": ""}),
            ("commands", "get_app_info"),
            ("unmigratedCommands", [None]),
            ("events", [7]),
            ("sources", []),
            ("sources", [""]),
        ]
        for key, value in bad_fields:
            with self.subTest(key=key, value=value):
                original = self.manifest[key]
                self.manifest[key] = value
                self.write_manifest()
                self.assert_rejected("契约清单")
                self.manifest[key] = original
        for key in list(self.manifest):
            with self.subTest(missing=key):
                original = self.manifest.pop(key)
                self.write_manifest()
                self.assert_rejected("契约清单")
                self.manifest[key] = original

    def test_missing_manifest_fails(self):
        self.manifest_path.unlink()
        self.assert_rejected("IPC 契约清单校验失败")

    def test_missing_handler_block_fails(self):
        self.lib.write_text("fn main() {}\n", encoding="utf-8")
        self.assert_rejected("未在 lib.rs 中找到 generate_handler! 块")


if __name__ == "__main__":
    unittest.main()
