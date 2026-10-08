#!/usr/bin/env python3
"""只读验证 FE2 归档索引、保留资料及原始 Git / tar.gz 证据。

不解压归档、不修改 Git、不联网；没有 --archive 时读取索引指定的原始提交。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile


DEFAULT_INDEX = "docs/verification/fe2-evidence-index-2026-10-08.json"
CHUNK_SIZE = 1024 * 1024
HEX_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
HEX_COMMIT = re.compile(r"[0-9a-f]{40}\Z")


class VerificationError(Exception):
    """索引或证据不符合归档约定。"""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise VerificationError(message)


def relative_path(value: object) -> str:
    """只接受规范的仓库相对路径，避免 Git 表达式或 tar 路径歧义。"""
    require(isinstance(value, str) and bool(value), "文件路径必须是非空字符串")
    require(
        not any(ord(character) < 32 or ord(character) == 127 for character in value),
        f"路径含控制字符：{value!r}",
    )
    require("\\" not in value and ":" not in value, f"路径不是规范 POSIX 相对路径：{value}")
    path = PurePosixPath(value)
    require(
        not path.is_absolute()
        and all(part not in (".", "..", ".git") for part in value.split("/"))
        and str(path) == value,
        f"不安全或不规范的相对路径：{value}",
    )
    return value


def regular_repo_file(repo: Path, name: str) -> Path:
    path = repo
    for component in PurePosixPath(name).parts:
        path = path / component
        require(not path.is_symlink(), f"仓库路径包含符号链接：{name}")
    require(path.is_file(), f"缺少保留文件：{name}")
    return path


def sha256_stream(stream, size: int | None = None) -> tuple[str, int]:
    digest = hashlib.sha256()
    count = 0
    while size is None or count < size:
        requested = CHUNK_SIZE if size is None else min(CHUNK_SIZE, size - count)
        chunk = stream.read(requested)
        if not chunk:
            break
        digest.update(chunk)
        count += len(chunk)
    if size is not None:
        require(count == size, f"数据截断：期望 {size} 字节，实际 {count} 字节")
    return digest.hexdigest(), count


def sha256_file(path: Path) -> tuple[str, int]:
    with path.open("rb") as stream:
        return sha256_stream(stream)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"JSON 重复键：{key}")
        result[key] = value
    return result


def read_json(path: Path):
    with path.open(encoding="utf-8") as stream:
        return json.load(stream, object_pairs_hook=unique_object)


def load_index(path: Path) -> tuple[dict, dict[str, dict]]:
    index = read_json(path)
    require(isinstance(index, dict) and type(index.get("schema_version")) is int and index["schema_version"] == 1, "索引 schema_version 必须是 1")
    commit = index.get("original_commit")
    require(isinstance(commit, str) and bool(HEX_COMMIT.fullmatch(commit)), "original_commit 必须是完整的 40 位小写 Git 提交哈希")
    archive = index.get("archive")
    require(isinstance(archive, dict), "索引缺少 archive")
    require(isinstance(archive.get("sha256"), str) and bool(HEX_SHA256.fullmatch(archive["sha256"])), "archive.sha256 非法")
    filename = relative_path(archive.get("filename"))
    require("/" not in filename and filename.endswith(".tar.gz"), "archive.filename 必须是 tar.gz 文件名")
    entries = index.get("files")
    require(isinstance(entries, list) and bool(entries), "索引 files 必须是非空列表")
    files = {}
    for entry in entries:
        require(isinstance(entry, dict), "索引文件条目必须是对象")
        name = relative_path(entry.get("path"))
        require(name not in files, f"索引重复路径：{name}")
        require(isinstance(entry.get("sha256"), str) and bool(HEX_SHA256.fullmatch(entry["sha256"])), f"文件 SHA-256 非法：{name}")
        require(type(entry.get("bytes")) is int and entry["bytes"] >= 0, f"文件大小非法：{name}")
        require(type(entry.get("retained")) is bool, f"retained 必须是布尔值：{name}")
        files[name] = entry
    require(type(archive.get("file_count")) is int and archive["file_count"] == len(files), "archive.file_count 与完整 files 清单数量不同")
    relative_path(index.get("checkpoint_path"))
    replacements = index.get("replacement_documents", [])
    require(isinstance(replacements, list), "replacement_documents 必须是列表")
    replacement_names = set()
    for replacement in replacements:
        require(isinstance(replacement, dict), "替代文档条目必须是对象")
        name = relative_path(replacement.get("path"))
        require(name not in replacement_names, f"替代文档重复路径：{name}")
        replacement_names.add(name)
        require(name in files and not files[name]["retained"], f"替代文档缺少迁出的原件：{name}")
        require(files[name].get("checkout_status") == "replaced_with_indexed_document", f"替代文档原件缺少 checkout_status 标记：{name}")
        require(isinstance(replacement.get("sha256"), str) and bool(HEX_SHA256.fullmatch(replacement["sha256"])), f"替代文档 SHA-256 非法：{name}")
        require(type(replacement.get("bytes")) is int and replacement["bytes"] >= 0, f"替代文档大小非法：{name}")
    return index, files


def verify_retained(repo: Path, files: dict[str, dict]) -> int:
    count = 0
    for name, entry in files.items():
        if entry["retained"]:
            digest, size = sha256_file(regular_repo_file(repo, name))
            require((digest, size) == (entry["sha256"], entry["bytes"]), f"保留文件已修改：{name}")
            count += 1
    return count


def verify_replacements(repo: Path, index: dict) -> int:
    replacements = index.get("replacement_documents", [])
    for entry in replacements:
        name = entry["path"]
        digest, size = sha256_file(regular_repo_file(repo, name))
        require((digest, size) == (entry["sha256"], entry["bytes"]), f"替代文档已修改：{name}")
    return len(replacements)


def verify_git(repo: Path, commit: str, files: dict[str, dict]) -> int:
    environment = {**os.environ, "GIT_OPTIONAL_LOCKS": "0", "GIT_TERMINAL_PROMPT": "0"}
    revision = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "--verify", f"{commit}^{{commit}}"],
        check=False, capture_output=True, text=True, env=environment,
    )
    require(revision.returncode == 0 and revision.stdout.strip() == commit, f"本地 Git 缺少原始提交：{commit}")
    tree = subprocess.run(
        ["git", "-C", str(repo), "ls-tree", "-rz", "--full-tree", commit],
        check=False, capture_output=True, env=environment,
    )
    require(tree.returncode == 0, "Git 原始目录树读取失败")
    external_paths = {name for name, entry in files.items() if not entry["retained"]}
    ordinary_files = set()
    for record in tree.stdout.split(b"\0"):
        if not record:
            continue
        metadata, name_bytes = record.split(b"\t", 1)
        mode, kind, _ = metadata.split()
        try:
            name = name_bytes.decode("utf-8")
        except UnicodeDecodeError:
            continue  # 索引本身是 UTF-8；其他历史文件不在本次核验范围。
        if name in external_paths:
            require(mode in (b"100644", b"100755") and kind == b"blob", f"原始提交不是普通文件：{name}")
            ordinary_files.add(name)
    missing = sorted(external_paths - ordinary_files)
    require(not missing, f"原始提交缺少普通文件：{', '.join(missing[:3])}")
    count = 0
    process = subprocess.Popen(
        ["git", "-C", str(repo), "cat-file", "--batch"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=environment,
    )
    try:
        for name, entry in files.items():
            if entry["retained"]:
                continue
            process.stdin.write(f"{commit}:{name}\n".encode("utf-8"))
            process.stdin.flush()
            fields = process.stdout.readline().rstrip(b"\n").split()
            require(len(fields) == 3 and fields[1] == b"blob" and fields[2].isdigit(), f"原始提交缺少普通文件 blob：{name}")
            size = int(fields[2])
            digest, read_size = sha256_stream(process.stdout, size)
            require(process.stdout.read(1) == b"\n", f"Git blob 分隔符非法：{name}")
            require((digest, read_size) == (entry["sha256"], entry["bytes"]), f"原始 Git 文件与索引不符：{name}")
            count += 1
        process.stdin.close()
        require(process.wait() == 0, "Git cat-file 读取失败")
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait()
        if not process.stdin.closed:
            process.stdin.close()
        process.stdout.close()
    return count


def verify_tar(archive_path: Path, index: dict, files: dict[str, dict]) -> int:
    digest, _ = sha256_file(archive_path)
    require(digest == index["archive"]["sha256"], "tar.gz 整包 SHA-256 与索引不符")
    parents = {
        str(parent)
        for name in files
        for parent in PurePosixPath(name).parents
        if str(parent) != "."
    }
    seen = set()
    seen_members = set()
    with tarfile.open(archive_path, "r|gz") as archive:
        for member in archive:
            name = relative_path(member.name.rstrip("/") if member.isdir() else member.name)
            require(name not in seen_members, f"tar 重复成员：{name}")
            seen_members.add(name)
            require(not member.issym() and not member.islnk(), f"tar 包含链接：{name}")
            if member.isdir():
                require(name in parents, f"tar 包含未知目录：{name}")
                continue
            require(member.isfile(), f"tar 成员不是普通文件：{name}")
            require(name in files, f"tar 包含未知文件：{name}")
            entry = files[name]
            require(member.size == entry["bytes"], f"tar 文件大小不符：{name}")
            with archive.extractfile(member) as stream:
                file_digest, size = sha256_stream(stream, member.size)
            require((file_digest, size) == (entry["sha256"], entry["bytes"]), f"tar 文件哈希不符：{name}")
            seen.add(name)
    missing = sorted(set(files) - seen)
    require(not missing, f"tar 遗漏 {len(missing)} 个文件：{', '.join(missing[:3])}")
    return len(seen)


def verify_checkpoint(repo: Path, index_path: Path, index: dict, files: dict[str, dict]) -> tuple[int, int]:
    checkpoint = read_json(regular_repo_file(repo, index["checkpoint_path"]))
    reference = checkpoint.get("external_evidence_archive")
    require(isinstance(reference, dict), "checkpoint 缺少 external_evidence_archive 引用")
    indexed_path = relative_path(reference.get("index"))
    require((repo / indexed_path).resolve() == index_path.resolve(), "checkpoint 指向的归档索引不符")
    external_count = sum(not entry["retained"] for entry in files.values())
    expected = {
        "original_commit": index["original_commit"],
        "file_count": external_count,
        "retained_file_count": len(files) - external_count,
        "archive_sha256": index["archive"]["sha256"],
    }
    for key, value in expected.items():
        require(reference.get(key) == value, f"checkpoint 的 {key} 与索引不符")
    require(index["checkpoint_path"] in files, "归档索引缺少原始 checkpoint")
    require(reference.get("historical_checkpoint_sha256") == files[index["checkpoint_path"]]["sha256"], "checkpoint 的原始 checkpoint 哈希与索引不符")
    source_hashes = checkpoint.get("source_sha256")
    require(isinstance(source_hashes, dict) and bool(source_hashes), "checkpoint 缺少当前 source_sha256")
    for name, digest in source_hashes.items():
        relative_path(name)
        require(isinstance(digest, str) and bool(HEX_SHA256.fullmatch(digest)), f"checkpoint 源文件哈希非法：{name}")
        current_digest, _ = sha256_file(regular_repo_file(repo, name))
        require(current_digest == digest, f"当前源文件与 checkpoint 不符：{name}")
    checked = 0

    def visit(value):
        nonlocal checked
        if isinstance(value, dict):
            for key, child in value.items():
                if key.endswith("evidence_sha256") and isinstance(child, dict):
                    for name, digest in child.items():
                        if name in files:
                            require(digest == files[name]["sha256"], f"checkpoint 历史证据哈希与索引不符：{name}")
                            checked += 1
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(checkpoint)
    return checked, len(source_hashes)


def verify(repo: Path, index_path: Path, archive_path: Path | None = None) -> dict:
    repo = repo.resolve()
    index, files = load_index(index_path)
    retained = verify_retained(repo, files)
    replacements = verify_replacements(repo, index)
    checkpoint_refs, source_files = verify_checkpoint(repo, index_path, index, files)
    if archive_path is None:
        checked = verify_git(repo, index["original_commit"], files)
        mode = "git_original_commit"
    else:
        checked = verify_tar(archive_path, index, files)
        mode = "tar_gz_stream"
    return {
        "passed": True,
        "mode": mode,
        "original_commit": index["original_commit"],
        "indexed_files": len(files),
        "retained_files_checked": retained,
        "replacement_documents_checked": replacements,
        "archived_files_checked": checked,
        "checkpoint_evidence_references_checked": checkpoint_refs,
        "current_source_files_checked": source_files,
        "archive_sha256": index["archive"]["sha256"],
        "scope": "保留文件、替代文档、完整归档（tar 模式）或迁出文件原始 Git blob（Git 模式）、checkpoint 归档引用及当前源文件哈希；不重新执行产品验收",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    default_repo = Path(__file__).resolve().parents[3]
    parser.add_argument("--repo", type=Path, default=default_repo)
    parser.add_argument("--index", type=Path, help=f"默认：<repo>/{DEFAULT_INDEX}")
    parser.add_argument("--archive", type=Path, help="完整外部 tar.gz 归档；不提供时从原始 Git 提交核验")
    arguments = parser.parse_args(argv)
    index_path = arguments.index or arguments.repo / DEFAULT_INDEX
    try:
        result = verify(arguments.repo, index_path, arguments.archive)
    except (VerificationError, OSError, ValueError, tarfile.TarError, subprocess.SubprocessError) as error:
        print(json.dumps({"passed": False, "error": str(error)}, ensure_ascii=False), file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
