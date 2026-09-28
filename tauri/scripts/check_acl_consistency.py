#!/usr/bin/env python3
"""检查 Tauri 命令注册、ACL 白名单与增量 IPC 契约清单。

handler 缺 ACL 为错误，ACL 遗留项仍仅警告。契约清单必须将实际注册命令
完整划分为已迁移与未迁移两组；这不意味着所有命令或事件已经类型化。

用法：python3 scripts/check_acl_consistency.py（在 tauri/ 目录下运行）
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIB_RS = ROOT / "src-tauri" / "src" / "lib.rs"
ACL_TOML = ROOT / "src-tauri" / "permissions" / "solo-soul" / "default.toml"
CONTRACT_MANIFEST = ROOT / "src" / "lib" / "generated" / "ipcContractManifest.json"


def extract_handler_commands(lib_rs: Path = LIB_RS) -> set[str]:
    text = lib_rs.read_text(encoding="utf-8")
    # 聚合所有 generate_handler! 簇，兼容一层嵌套的 #[cfg(...)] 属性。
    blocks = re.findall(r"generate_handler!\s*\[((?:[^\[\]]|\[[^\[\]]*\])*)\]", text, re.S)
    if not blocks:
        raise ValueError("未在 lib.rs 中找到 generate_handler! 块")
    cmds: set[str] = set()
    for block in blocks:
        cmds.update(re.findall(r"::(\w+)\s*[,\]]", block))
    return cmds


def extract_acl_commands(acl_toml: Path = ACL_TOML) -> set[str]:
    text = acl_toml.read_text(encoding="utf-8")
    return set(re.findall(r'"(\w+)"', text))


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"契约清单包含重复 JSON 键：{key}")
        result[key] = value
    return result


def _names(manifest: dict[str, object], key: str) -> set[str]:
    values = manifest.get(key)
    if not isinstance(values, list) or any(
        not isinstance(value, str) or not value.strip() for value in values
    ):
        raise ValueError(f"契约清单 {key} 必须为非空字符串组成的数组")
    names = set(values)
    if len(names) != len(values):
        raise ValueError(f"契约清单 {key} 包含重复项")
    return names


def validate_contract_manifest(
    manifest_path: Path, handler_cmds: set[str], acl_cmds: set[str]
) -> tuple[int, int, int]:
    manifest = json.loads(
        manifest_path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object
    )
    if not isinstance(manifest, dict):
        raise ValueError("契约清单顶层必须为对象")
    if type(manifest.get("schemaVersion")) is not int or manifest["schemaVersion"] != 1:
        raise ValueError("契约清单 schemaVersion 必须为 1")
    generator = manifest.get("generator")
    if (
        not isinstance(generator, dict)
        or generator.get("name") != "solosoul-ipc-contract-gen"
        or not isinstance(generator.get("version"), str)
        or not generator["version"].strip()
    ):
        raise ValueError("契约清单 generator 必须声明 solosoul-ipc-contract-gen 及版本")

    migrated = _names(manifest, "commands")
    unmigrated = _names(manifest, "unmigratedCommands")
    events = _names(manifest, "events")
    sources = _names(manifest, "sources")
    if not sources:
        raise ValueError("契约清单 sources 不得为空")

    overlap = sorted(migrated & unmigrated)
    if overlap:
        raise ValueError(f"契约已迁移/未迁移命令重叠：{overlap}")
    covered = migrated | unmigrated
    missing = sorted(handler_cmds - covered)
    extra = sorted(covered - handler_cmds)
    if missing or extra:
        raise ValueError(
            f"契约命令分区与 handler 不一致；清单缺少：{missing}；清单多出：{extra}"
        )
    missing_acl = sorted(migrated - acl_cmds)
    if missing_acl:
        raise ValueError(f"契约已迁移命令缺少 ACL：{missing_acl}")
    return len(migrated), len(unmigrated), len(events)


def main(
    lib_rs: Path = LIB_RS,
    acl_toml: Path = ACL_TOML,
    manifest_path: Path = CONTRACT_MANIFEST,
) -> int:
    try:
        handler_cmds = extract_handler_commands(lib_rs)
        acl_cmds = extract_acl_commands(acl_toml)
    except (OSError, UnicodeError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2

    missing = sorted(handler_cmds - acl_cmds)
    leftover = sorted(acl_cmds - handler_cmds)

    if leftover:
        print(f"WARN: 白名单中存在但 handler 中未注册（遗留项？）: {leftover}")

    if missing:
        print("ERROR: 以下命令未登记到 ACL 白名单（default.toml）：", file=sys.stderr)
        for cmd in missing:
            print(f"  - {cmd}", file=sys.stderr)
        print(
            "请将上述命令加入 src-tauri/permissions/solo-soul/default.toml 的 "
            "commands.allow 列表。",
            file=sys.stderr,
        )
        return 1

    try:
        migrated, unmigrated, events = validate_contract_manifest(
            manifest_path, handler_cmds, acl_cmds
        )
    except (OSError, UnicodeError, ValueError) as error:
        print(f"ERROR: IPC 契约清单校验失败：{error}", file=sys.stderr)
        return 1

    print(f"OK: {len(handler_cmds)} 个命令均已登记到 ACL 白名单。")
    print(
        f"OK: 增量 IPC 契约已迁移 {migrated} 个命令，未迁移 {unmigrated} 个命令，"
        f"登记事件 {events} 个；不表示全量命令/事件已迁移。"
    )
    return 0


if __name__ == "__main__":
    # Windows 重定向输出可能使用 cp1252，显式支持中文诊断。
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    sys.exit(main())
