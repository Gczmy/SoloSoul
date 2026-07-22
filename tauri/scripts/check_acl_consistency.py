#!/usr/bin/env python3
"""检查 Tauri 自定义命令与 ACL 白名单的一致性。

- 从 src-tauri/src/lib.rs 的 generate_handler! 块提取全部命令名；
- 从 src-tauri/permissions/solo-soul/default.toml 提取 allow-all-custom-commands 列表；
- handler 有而白名单无 → 报错并 exit 1（防止新命令漏登记导致运行时
  "Command xxx not allowed by ACL"）；
- 白名单有而 handler 无 → 仅警告（可能是已删除命令的遗留项），不阻断。

用法：python3 scripts/check_acl_consistency.py（在 tauri/ 目录下运行）
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIB_RS = ROOT / "src-tauri" / "src" / "lib.rs"
ACL_TOML = ROOT / "src-tauri" / "permissions" / "solo-soul" / "default.toml"


def extract_handler_commands() -> set[str]:
    text = LIB_RS.read_text(encoding="utf-8")
    m = re.search(r"generate_handler!\s*\[(.*?)\]", text, re.S)
    if not m:
        print("ERROR: 未在 lib.rs 中找到 generate_handler! 块", file=sys.stderr)
        sys.exit(2)
    block = m.group(1)
    # 命令以 `path::to::command_name` 形式列出，取 `::name` 后紧跟逗号/闭括号的最后一段，
    # 避免误捕获模块段（如 commands::auth::login 中的 auth）
    return set(re.findall(r"::(\w+)\s*[,\]]", block))


def extract_acl_commands() -> set[str]:
    text = ACL_TOML.read_text(encoding="utf-8")
    return set(re.findall(r'"(\w+)"', text))


def main() -> int:
    handler_cmds = extract_handler_commands()
    acl_cmds = extract_acl_commands()

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
            "allow-all-custom-commands 列表。",
            file=sys.stderr,
        )
        return 1

    print(f"OK: {len(handler_cmds)} 个命令均已登记到 ACL 白名单。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
