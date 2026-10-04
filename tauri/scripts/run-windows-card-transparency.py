#!/usr/bin/env python3
"""RF-121：真实 Windows 关闭透明效果与恢复 Mica 的 Card 验收。"""
import argparse
import ctypes
import ctypes.wintypes as w
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

if sys.platform != "win32":
    raise SystemExit("Windows Card accessibility regression requires Windows")
import winreg

KEY = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"


class HighContrast(ctypes.Structure):
    _fields_ = [("cbSize", w.UINT), ("dwFlags", w.DWORD), ("scheme", w.LPWSTR)]


USER = ctypes.WinDLL("user32", use_last_error=True)
USER.SystemParametersInfoW.argtypes = [w.UINT, w.UINT, w.LPVOID, w.UINT]
USER.SystemParametersInfoW.restype = w.BOOL
USER.GetSysColor.argtypes = [ctypes.c_int]
USER.GetSysColor.restype = w.DWORD


def write_new(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as file:
        json.dump(value, file, ensure_ascii=False, indent=2)
        file.write("\n")


def digest(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def snapshot():
    contrast = HighContrast()
    contrast.cbSize = ctypes.sizeof(contrast)
    if not USER.SystemParametersInfoW(0x0042, contrast.cbSize, ctypes.byref(contrast), 0):
        raise ctypes.WinError(ctypes.get_last_error())
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, KEY) as key:
        transparency, kind = winreg.QueryValueEx(key, "EnableTransparency")
    if kind != winreg.REG_DWORD or transparency not in (0, 1):
        raise RuntimeError("Actual transparency preference is not a known DWORD")
    # 只复制 scheme，不把系统返回的指针传入后续 SET 或记录地址。
    return {
        "highContrastFlags": contrast.dwFlags,
        "highContrastOn": bool(contrast.dwFlags & 1),
        "defaultScheme": contrast.scheme,
        "transparency": transparency,
        "transparencyKind": kind,
        "systemColors": {str(i): USER.GetSysColor(i) for i in range(31)},
    }


def set_transparency(value, kind):
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, KEY, 0, winreg.KEY_SET_VALUE) as key:
        winreg.SetValueEx(key, "EnableTransparency", 0, kind, value)


def verify_restore(original):
    restored = snapshot()
    if restored != original:
        raise RuntimeError("Actual Windows visual settings did not return to the original snapshot")
    return restored


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", required=True, type=Path)
    parser.add_argument("--fixture", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    for path in (args.exe, args.fixture, args.output):
        if not path.is_absolute() or len(path.drive) != 2 or path.drive[1] != ":" or path == Path(path.anchor):
            parser.error("All paths must be absolute non-root paths")
    if args.output.exists():
        parser.error("Output directory must be new")
    for path in (args.exe, args.fixture):
        if not path.is_file() or path.lstat().st_file_attributes & 0x400:
            parser.error("Inputs must be ordinary files")
    fixture = args.fixture.read_bytes()
    if len(fixture) > 2 * 1024 * 1024 or b"<!-- SoloSoul RF-121 generated fixture -->" not in fixture:
        parser.error("Expected the bounded generated RF-121 Card fixture")
    # 防止误启动正式客户端；此启发式不是签名保证，运行前仍需核对例程冻结 SHA。
    markers = [b"com.solosoul.windows-card-regression", b"card_surface_report", b"windows-card-regression"]
    with args.exe.open("rb") as binary:
        remaining = set(markers)
        overlap = b""
        while chunk := binary.read(1024 * 1024):
            data = overlap + chunk
            remaining = {marker for marker in remaining if marker not in data}
            overlap = data[-256:]
        if remaining:
            parser.error("EXE lacks independent Windows Card regression markers")
    original = snapshot()
    if original["highContrastOn"] or original["transparency"] != 1:
        parser.error("This two-lane checkpoint requires the original ordinary transparent setting")
    args.output.mkdir()
    result = {"scope": "windows-card-real-transparency", "success": False,
              "binarySha256": digest(args.exe), "fixtureSha256": digest(args.fixture),
              "original": original, "lanes": [], "systemRestored": False}
    write_new(args.output / "original-settings.json", original)
    try:
        for mode in ("transparency-off", "mica"):
            lane = {"mode": mode, "exit": None, "restored": False}
            result["lanes"].append(lane)
            try:
                if mode == "transparency-off":
                    set_transparency(0, original["transparencyKind"])
                lane["actualSettings"] = snapshot()
                if mode == "transparency-off" and lane["actualSettings"]["transparency"] != 0:
                    raise RuntimeError("Transparency-off was not actually active")
                output = args.output / mode
                command = [str(args.exe), "--card-fixture", str(args.fixture), "--output", str(output), "--expect", mode]
                lane["command"] = command
                started = time.monotonic()
                with (args.output / (mode + ".log")).open("wb") as log:
                    child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT,
                                             creationflags=subprocess.CREATE_NO_WINDOW)
                    lane["pid"] = child.pid
                    try:
                        lane["exit"] = child.wait(timeout=65)
                    except subprocess.TimeoutExpired:
                        # Popen 持有本次新建进程句柄，仅结束这个超时测试根进程。
                        child.kill()
                        child.wait()
                        raise RuntimeError("Independent native fixture exceeded its bounded timeout")
                lane["seconds"] = time.monotonic() - started
                if lane["exit"] != 0:
                    raise RuntimeError("Independent native fixture failed; original evidence retained")
                native = json.loads((output / "result.json").read_text(encoding="utf-8-sig"))
                if native.get("success") is not True or len(native.get("samples", [])) != 8:
                    raise RuntimeError("Eight successful native samples are required")
                lane["samples"] = native["samples"]
            finally:
                if mode == "transparency-off":
                    set_transparency(original["transparency"], original["transparencyKind"])
                lane["restoredSettings"] = verify_restore(original)
                lane["restored"] = True
        result["success"] = True
    except Exception as error:
        result["errorType"] = type(error).__name__
        result["error"] = str(error)
    finally:
        # 即使恢复验证失败，也发布原始结果，避免异常掩盖设置变化。
        try:
            result["finalSettings"] = snapshot()
            result["systemRestored"] = result["finalSettings"] == original
            if not result["systemRestored"]:
                result["success"] = False
                result["restoreError"] = "Actual settings differ from the original snapshot"
        except Exception as error:
            result["success"] = False
            result["restoreErrorType"] = type(error).__name__
            result["restoreError"] = str(error)
        write_new(args.output / "accessibility-result.json", result)
    print(json.dumps({"success": result["success"], "systemRestored": result["systemRestored"],
                      "lanes": [{k: x.get(k) for k in ("mode", "exit", "restored")} for x in result["lanes"]]}))
    return 0 if result["success"] else 1


if __name__ == "__main__":
    sys.exit(main())
