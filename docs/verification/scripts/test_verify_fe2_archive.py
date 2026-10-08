"""使用独立临时仓库验证归档恢复路径；不读写项目正式数据。"""

import copy
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

from verify_fe2_archive import VerificationError, verify


class ArchiveVerificationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="solosoul-fe2-archive-test-")
        self.addCleanup(self.temporary.cleanup)
        self.repo = Path(self.temporary.name) / "repo"
        self.repo.mkdir()
        self.index_path = self.repo / "docs/verification/index.json"
        self.archive_path = Path(self.temporary.name) / "full-evidence.tar.gz"
        self.checkpoint_name = "docs/verification/checkpoint.json"
        self.retained_name = "docs/verification/evidence/key.png"
        self.external_name = "docs/verification/evidence/raw.log"
        self.replacement_name = "docs/verification/evidence/README.md"
        self.source_name = "tauri/src/fixture.ts"
        self.original = {
            self.checkpoint_name: b'{"stage":"historical-complete-evidence"}\n',
            self.retained_name: b"representative synthetic pixels\x00\xff",
            self.external_name: b"full synthetic raw results\n",
            self.replacement_name: b"[raw](raw.log)\n",
        }
        for name, data in self.original.items():
            self.write(name, data)
        self.write(self.source_name, b"export const syntheticFixture = true;\n")
        self.git("init", "--quiet")
        self.git("add", "--all")
        self.git(
            "-c", "user.name=Archive Test", "-c", "user.email=archive@example.invalid",
            "-c", "core.hooksPath=/dev/null", "commit", "--quiet", "-m", "Original synthetic evidence",
        )
        self.commit = self.git("rev-parse", "HEAD").strip()
        self.index = {
            "schema_version": 1,
            "original_commit": self.commit,
            "archive": {"filename": self.archive_path.name, "sha256": "0" * 64, "file_count": len(self.original)},
            "files": [
                {
                    "path": name,
                    "sha256": hashlib.sha256(data).hexdigest(),
                    "bytes": len(data),
                    "retained": name == self.retained_name,
                    **({"checkout_status": "replaced_with_indexed_document"} if name == self.replacement_name else {}),
                }
                for name, data in self.original.items()
            ],
            "checkpoint_path": self.checkpoint_name,
            "replacement_documents": [],
        }
        replacement = b"Raw results are retained in the original Git commit.\n"
        self.write(self.replacement_name, replacement)
        self.index["replacement_documents"].append({
            "path": self.replacement_name,
            "sha256": hashlib.sha256(replacement).hexdigest(),
            "bytes": len(replacement),
        })
        (self.repo / self.external_name).unlink()
        self.write_archive()

    def git(self, *arguments):
        return subprocess.run(
            ["git", "-C", str(self.repo), *arguments],
            check=True, capture_output=True, text=True,
        ).stdout

    def write(self, name, data):
        destination = self.repo / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)

    def write_index_and_checkpoint(self):
        self.index_path.write_text(json.dumps(self.index), encoding="utf-8")
        checkpoint = {
            "external_evidence_archive": {
                "index": str(self.index_path.relative_to(self.repo)),
                "original_commit": self.commit,
                "file_count": 3,
                "retained_file_count": 1,
                "archive_sha256": self.index["archive"]["sha256"],
                "historical_checkpoint_sha256": hashlib.sha256(self.original[self.checkpoint_name]).hexdigest(),
            },
            "source_sha256": {
                self.source_name: hashlib.sha256((self.repo / self.source_name).read_bytes()).hexdigest(),
            },
            "historical": {"evidence_sha256": {
                self.external_name: hashlib.sha256(self.original[self.external_name]).hexdigest(),
            }},
        }
        self.write(self.checkpoint_name, json.dumps(checkpoint).encode("utf-8"))

    def write_archive(self, omit=(), changes=None, extra=()):
        changes = changes or {}
        with tarfile.open(self.archive_path, "w:gz") as archive:
            for name, original in self.original.items():
                if name in omit:
                    continue
                data = changes.get(name, original)
                member = tarfile.TarInfo(name)
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))
            for member, data in extra:
                archive.addfile(member, io.BytesIO(data) if member.isfile() else None)
        self.index["archive"]["sha256"] = hashlib.sha256(self.archive_path.read_bytes()).hexdigest()
        self.write_index_and_checkpoint()

    def check(self, use_tar=True):
        return verify(self.repo, self.index_path, self.archive_path if use_tar else None)

    def assert_rejected(self, text, use_tar=True):
        with self.assertRaisesRegex(VerificationError, text):
            self.check(use_tar)

    def test_full_tar_and_current_checkout_validate_without_extracting(self):
        before = sorted(path.relative_to(self.repo).as_posix() for path in self.repo.rglob("*") if path.is_file())
        result = self.check()
        self.assertTrue(result["passed"])
        self.assertEqual(result["archived_files_checked"], 4)
        self.assertEqual(result["retained_files_checked"], 1)
        self.assertEqual(result["replacement_documents_checked"], 1)
        self.assertEqual(result["current_source_files_checked"], 1)
        self.assertEqual(before, sorted(path.relative_to(self.repo).as_posix() for path in self.repo.rglob("*") if path.is_file()))
        self.assertFalse((self.repo / self.external_name).exists())

    def test_original_git_commit_validates_external_files_after_checkout_replacement(self):
        before = self.git("status", "--porcelain=v1")
        result = self.check(use_tar=False)
        self.assertTrue(result["passed"])
        self.assertEqual(result["mode"], "git_original_commit")
        self.assertEqual(result["archived_files_checked"], 3)
        self.assertEqual(before, self.git("status", "--porcelain=v1"))

    def test_omitted_file_is_detected_even_when_package_hash_matches(self):
        self.write_archive(omit=(self.external_name,))
        self.assert_rejected("tar 遗漏")

    def test_same_length_tampering_is_detected(self):
        self.write_archive(changes={self.external_name: b"x" * len(self.original[self.external_name])})
        self.assert_rejected("tar 文件哈希不符")

    def test_whole_package_hash_is_checked(self):
        with self.archive_path.open("ab") as archive:
            archive.write(b"unexpected trailing bytes")
        self.assert_rejected("整包 SHA-256")

    def test_unsafe_archive_paths_are_rejected_without_materializing_them(self):
        for name in ("../escaped-file", "/absolute-file", "docs/verification/../escaped-file", "./relative-file"):
            with self.subTest(name=name):
                member = tarfile.TarInfo(name)
                self.write_archive(extra=((member, b""),))
                self.assert_rejected("不安全或不规范")
        self.assertFalse((self.repo.parent / "escaped-file").exists())

    def test_symlinks_and_hardlinks_are_rejected(self):
        for kind in (tarfile.SYMTYPE, tarfile.LNKTYPE):
            with self.subTest(kind=kind):
                member = tarfile.TarInfo("docs/verification/evidence/link")
                member.type = kind
                member.linkname = self.retained_name
                self.write_archive(extra=((member, b""),))
                self.assert_rejected("tar 包含链接")

    def test_duplicate_member_is_rejected(self):
        member = tarfile.TarInfo(self.external_name)
        member.size = len(self.original[self.external_name])
        self.write_archive(extra=((member, self.original[self.external_name]),))
        self.assert_rejected("tar 重复成员")

    def test_unknown_member_is_rejected(self):
        member = tarfile.TarInfo("docs/verification/evidence/unindexed.txt")
        self.write_archive(extra=((member, b""),))
        self.assert_rejected("tar 包含未知文件")

    def test_device_member_is_rejected(self):
        member = tarfile.TarInfo("docs/verification/evidence/device")
        member.type = tarfile.CHRTYPE
        self.write_archive(extra=((member, b""),))
        self.assert_rejected("不是普通文件")

    def test_retained_file_changes_are_detected_in_both_modes(self):
        self.write(self.retained_name, b"modified current pixels")
        self.assert_rejected("保留文件已修改", use_tar=True)
        self.assert_rejected("保留文件已修改", use_tar=False)

    def test_replacement_document_changes_are_detected(self):
        self.write(self.replacement_name, b"edited replacement")
        self.assert_rejected("替代文档已修改")

    def test_current_source_changes_are_detected(self):
        self.write(self.source_name, b"export const syntheticFixture = false;\n")
        self.assert_rejected("当前源文件与 checkpoint 不符")

    def test_retained_symlink_is_rejected(self):
        retained = self.repo / self.retained_name
        retained.unlink()
        retained.symlink_to(self.repo / self.replacement_name)
        self.assert_rejected("仓库路径包含符号链接")

    def test_checkpoint_archive_reference_mismatch_is_detected(self):
        checkpoint = json.loads((self.repo / self.checkpoint_name).read_text())
        checkpoint["external_evidence_archive"]["file_count"] = 0
        self.write(self.checkpoint_name, json.dumps(checkpoint).encode())
        self.assert_rejected("file_count 与索引不符")

    def test_git_original_blob_tampering_in_index_is_detected(self):
        entry = next(entry for entry in self.index["files"] if entry["path"] == self.replacement_name)
        entry["sha256"] = "0" * 64
        self.write_index_and_checkpoint()
        self.assert_rejected("原始 Git 文件与索引不符", use_tar=False)

    def test_git_symlink_cannot_be_verified_as_an_ordinary_file(self):
        link_name = "docs/verification/evidence/original-link"
        (self.repo / link_name).symlink_to("key.png")
        self.git("add", "--", link_name)
        self.git(
            "-c", "user.name=Archive Test", "-c", "user.email=archive@example.invalid",
            "-c", "core.hooksPath=/dev/null", "commit", "--quiet", "-m", "Synthetic link",
        )
        self.commit = self.git("rev-parse", "HEAD").strip()
        self.index["original_commit"] = self.commit
        self.index["files"].append({
            "path": link_name, "sha256": hashlib.sha256(b"key.png").hexdigest(),
            "bytes": 7, "retained": False,
        })
        self.index["archive"]["file_count"] += 1
        self.write_index_and_checkpoint()
        checkpoint = json.loads((self.repo / self.checkpoint_name).read_text())
        checkpoint["external_evidence_archive"]["file_count"] = 4
        self.write(self.checkpoint_name, json.dumps(checkpoint).encode())
        self.assert_rejected("原始提交不是普通文件", use_tar=False)

    def test_commit_and_paths_cannot_inject_git_batch_requests(self):
        for value in ("HEAD", "--help", self.commit + "\nHEAD"):
            with self.subTest(commit=value):
                self.index["original_commit"] = value
                self.write_index_and_checkpoint()
                self.assert_rejected("original_commit", use_tar=False)
        self.index["original_commit"] = self.commit
        self.index["files"][0]["path"] = self.external_name + "\nHEAD"
        self.write_index_and_checkpoint()
        self.assert_rejected("路径含控制字符", use_tar=False)

    def test_duplicate_index_file_is_rejected(self):
        self.index["files"].append(copy.deepcopy(self.index["files"][0]))
        self.write_index_and_checkpoint()
        self.assert_rejected("索引重复路径")


if __name__ == "__main__":
    unittest.main()
