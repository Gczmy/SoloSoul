#!/usr/bin/env python3
"""运行独立 macOS FE2 客户端，额外阻止进程访问正式默认数据路径。"""

import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile


HERE = Path(__file__).resolve().parent
BINARY = HERE.parents[1] / "target/debug/bundle/macos/SoloSoulFE2Mac.app/Contents/MacOS/solo_soul"
PROFILE = HERE / "formal-data-deny.sb"


def sandbox_command(user_directory):
    formal = {
        "FORMAL_VAULT": user_directory / ".solosoul",
        "FORMAL_DATA": user_directory / "Library/Application Support/com.solosoul.app",
        "FORMAL_CACHE": user_directory / "Library/Caches/com.solosoul.app",
        "FORMAL_WEBKIT": user_directory / "Library/WebKit/com.solosoul.app",
        "FORMAL_SAVED_STATE": user_directory / "Library/Saved Application State/com.solosoul.app.savedState",
        "FORMAL_PREFERENCES": user_directory / "Library/Preferences/com.solosoul.app.plist",
    }
    command = ["/usr/bin/sandbox-exec", "-f", str(PROFILE)]
    for name, path in formal.items():
        command.extend(["-D", f"{name}={path.resolve()}"])
    return command


def check_sandbox():
    # 实际策略验收仅操作自己新建的合成文件，不能探测真实账户内容。
    with tempfile.TemporaryDirectory(prefix="solosoul-fe2-macos-sandbox-") as temporary:
        user_directory = Path(temporary).resolve()
        forbidden = user_directory / ".solosoul"
        forbidden.mkdir()
        fixture = forbidden / "synthetic.txt"
        fixture.write_text("synthetic", encoding="utf-8")
        command = sandbox_command(user_directory)
        read = subprocess.run(command + ["/bin/cat", str(fixture)], capture_output=True)
        write = subprocess.run(command + ["/usr/bin/touch", str(forbidden / "must-not-exist")], capture_output=True)
        if read.returncode == 0 or write.returncode == 0 or (forbidden / "must-not-exist").exists():
            raise RuntimeError("sandbox 未拒绝合成正式目录访问，不启动客户端")
        # 策略必须仍允许操作自己的验收输出，不能将所有进程启动失败误记为保护通过。
        allowed = user_directory / "own-output"
        positive = subprocess.run(command + ["/usr/bin/touch", str(allowed)], capture_output=True)
        if positive.returncode != 0 or not allowed.exists():
            raise RuntimeError("sandbox 正向控制失败，不能确认策略可用")
        print("PASS: actual sandbox denied synthetic formal read/write and allowed own output")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--check-sandbox", action="store_true")
    parser.add_argument("--bind", action="store_true", help="绑定测试 bundle，随后可由电脑控制工具标准启动")
    parser.add_argument("--root", type=Path)
    arguments = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("仅 macOS 可执行")
    if arguments.check_sandbox:
        check_sandbox()
        return
    if not BINARY.is_file():
        parser.error("请先按 README 构建 SoloSoulFE2Mac.app")
    if arguments.prepare:
        if arguments.root:
            parser.error("prepare 只创建新根，不接受已有 root")
        result = subprocess.run([str(BINARY), "--fe2-macos-prepare"], check=False)
        raise SystemExit(result.returncode)
    if not arguments.root:
        parser.error("必须指定 --root；不能回退到正式客户端")
    environment = os.environ.copy()
    environment["SOLOSOUL_FE2_MACOS_ROOT"] = str(arguments.root)
    if arguments.bind:
        subprocess.run([str(BINARY), "--fe2-macos-preflight"], env=environment, check=True)
        pointer = BINARY.parent.parent / "Resources/fe2-macos-root.txt"
        # 不覆盖另一轮绑定；更换根需要先关闭应用并显式移除自己的测试路径文件。
        descriptor = os.open(pointer, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "w", encoding="utf-8") as file:
            file.write(str(arguments.root) + "\n")
        print("PASS: separate test bundle bound to validated synthetic root")
        return
    check_sandbox()
    os.execve("/usr/bin/sandbox-exec", sandbox_command(Path.home()) + [str(BINARY)], environment)


if __name__ == "__main__":
    main()
