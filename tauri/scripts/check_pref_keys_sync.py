#!/usr/bin/env python3
"""检查 P026 偏好 key 白名单与前端 AppSettings 键的一致性（R005-①）。

- 从 src-tauri/src/commands/settings.rs 提取 `ALLOWED_PREF_KEYS` 白名单；
- 从 src/stores/settingsStore.ts 提取 `AppSettings` 接口的全部属性名；
- 任一侧有另一侧没有的 key → 报错并 exit 1（防止未来新增/删除 key 时
  白名单与前端脱钩：前端 updateSetting 发送未知 key 会被后端拒绝，
  后端白名单残留 key 则失去校验意义）。

用法：python3 scripts/check_pref_keys_sync.py（在 tauri/ 目录下运行）
"""

import re
import sys
from pathlib import Path

# Windows 控制台默认 cp1252，无法打印中文 → 强制 UTF-8 输出
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

ROOT = Path(__file__).resolve().parent.parent
SETTINGS_RS = ROOT / "src-tauri" / "src" / "commands" / "settings.rs"
SETTINGS_STORE_TS = ROOT / "src" / "stores" / "settingsStore.ts"


def strip_line_comments(text: str) -> str:
    """移除 // 行注释（保留块注释内的内容不参与键提取即可）。"""
    return re.sub(r"//[^\n]*", "", text)


def extract_rust_whitelist() -> set[str]:
    text = strip_line_comments(SETTINGS_RS.read_text(encoding="utf-8"))
    m = re.search(r"const ALLOWED_PREF_KEYS:\s*&\[&str\]\s*=\s*&\[(.*?)\];", text, re.S)
    if not m:
        print("ERROR: 未在 settings.rs 中找到 ALLOWED_PREF_KEYS 常量", file=sys.stderr)
        sys.exit(2)
    return set(re.findall(r'"(\w+)"', m.group(1)))


def extract_ts_app_settings_keys() -> set[str]:
    text = SETTINGS_STORE_TS.read_text(encoding="utf-8")
    m = re.search(r"export interface AppSettings\s*\{(.*?)\n\}", text, re.S)
    if not m:
        print("ERROR: 未在 settingsStore.ts 中找到 AppSettings 接口", file=sys.stderr)
        sys.exit(2)
    keys = set()
    for line in m.group(1).splitlines():
        # 属性行形如 `  keyName: Type;`（缩进 + 冒号）；用 `\s+` 而非固定两空格，
        # 避免格式化缩进变化时检查静默失效。注释行（`/**` / ` *`）以 / * 开头不匹配。
        hit = re.match(r"^\s+([A-Za-z][A-Za-z0-9]*):", line)
        if hit:
            keys.add(hit.group(1))
    return keys


def main() -> int:
    rust_keys = extract_rust_whitelist()
    ts_keys = extract_ts_app_settings_keys()

    only_rust = sorted(rust_keys - ts_keys)
    only_ts = sorted(ts_keys - rust_keys)

    if not only_rust and not only_ts:
        print(f"OK: 白名单与前端 AppSettings 完全一致（{len(rust_keys)} 个 key）")
        return 0

    for k in only_rust:
        print(f"ERROR: 白名单独有、前端 AppSettings 缺失的 key: {k}")
    for k in only_ts:
        print(f"ERROR: 前端 AppSettings 独有、白名单缺失的 key: {k}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
