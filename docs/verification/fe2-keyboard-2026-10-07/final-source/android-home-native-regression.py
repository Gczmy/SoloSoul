#!/usr/bin/env python3
"""FE2 首页原生验收：专用模拟器、无既有账户、完整私有目录备份/恢复。

测试会通过生产 UI 创建公开临时账户。备份仅存本机结果目录，不上传。
恢复前必须 force-stop；恢复后核对全部文件内容、链接与权限。
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import time
import uuid
import xml.etree.ElementTree as ET


APP = "com.solosoul.app"
METHOD = "realHomePreservesGlassAndCopyAcrossLockUnlock"
NOTIFICATION = "android.permission.POST_NOTIFICATIONS"


def notification_permission(package_dump):
    match = re.search(r"^\s*android\.permission\.POST_NOTIFICATIONS: granted=(true|false), flags=\[([^\]]*)\]", package_dump, re.M)
    if not match:
        raise RuntimeError("Missing current notification permission state; refusing mutation")
    return {"granted": match[1] == "true", "flags": sorted(flag.strip() for flag in match[2].split("|") if flag.strip())}


def digest(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def inventory(path, require_empty=False):
    """比较内容/权限而非 tar 的时间戳；不将偏好、数据库内容写入报告。"""
    result = {}
    with tarfile.open(path) as archive:
        for member in archive:
            name = member.name.removeprefix("./")
            if name == ".":
                continue
            if name.startswith("/") or ".." in Path(name).parts:
                raise RuntimeError("Backup contains unsafe path")
            if require_empty and any(part.startswith("acc_") for part in Path(name).parts):
                raise RuntimeError("Dedicated app contains existing account; refusing test")
            entry = {"type": member.type.decode("ascii"), "mode": member.mode}
            if member.isfile():
                data = archive.extractfile(member).read()
                entry["sha256"] = hashlib.sha256(data).hexdigest()
                if require_empty and Path(name).name == "accounts.json":
                    if json.loads(data) != []:
                        raise RuntimeError("Dedicated app has nonempty accounts manifest")
            elif member.issym() or member.islnk():
                entry["target"] = member.linkname
            elif not member.isdir():
                raise RuntimeError("Backup contains unsupported special file")
            result[name] = entry
    return result


def verify_native(text):
    text = text.replace("\r\n", "\n")
    statuses = [int(n) for n in re.findall(r"^INSTRUMENTATION_STATUS_CODE:\s*(-?\d+)\s*$", text, re.M)]
    if (statuses != [1, 0] or f"INSTRUMENTATION_STATUS: test={METHOD}\n" not in text
            or not re.search(r"^INSTRUMENTATION_CODE:\s*-1\s*$", text, re.M)
            or "OK (1 test)" not in text
            or re.search(r"INSTRUMENTATION_FAILED|Process crashed|FAILURES!!!", text)):
        raise RuntimeError("Missing exact successful one-test native report; skips are failures")


def verification_passed(report, needs_notification):
    # 文件比较通过后，根权限核对或收尾仍可能失败，不能仅看部分成功标记。
    return bool(report.get("nativePassed") and report.get("restored") and report.get("blurRestored")
                and (not needs_notification or report.get("notificationPermissionRestored"))
                and not any(report.get(key) for key in
                            ("error", "restoreError", "blurRestoreError", "notificationRestoreError")))


def verify_evidence(evidence, scenario):
    stages = [record["stage"] for record in evidence["records"]]
    expected = ["local-light", "enhanced-light", "enhanced-dark", "enhanced-dark-after-unlock", "encrypted-preferences",
                "object-editor-dark"]
    if scenario == "keyboard":
        expected.extend(["editor-keyboard-dark", "editor-keyboard-save-dark", "editor-keyboard-after-back"])
    expected.extend(["object-list-dark", "object-detail-dark"])
    if scenario == "overlays":
        expected.extend(["object-attachments-dark", "object-history-dark", "nested-overlays-after-system-back"])
    expected.extend(["object-actions-dark", "object-list-after-system-back", "populated-enhanced-dark", "populated-enhanced-light"])
    if scenario not in ("baseline", "overlays", "keyboard") or evidence.get("scenario", "baseline") != scenario or stages != expected:
        raise RuntimeError(f"Incomplete production-home evidence ({scenario}): {stages}")
    if scenario == "overlays":
        for record in evidence["records"]:
            if record["stage"] in ("object-attachments-dark", "object-history-dark"):
                if (record.get("toastCount") != 1 or record.get("toastBackdrop") != "5100"
                        or not all(record.get(key) for key in ("backupReminder", "toastInFlow", "toastInPanel", "toastVisible", "toastHittable"))
                        or not record.get("controls")
                        or any(not all(control.get(key) for key in ("found", "visible", "hittable"))
                               or control.get("overlapped") for control in record["controls"])):
                    raise RuntimeError(f"Incomplete native foreground notification evidence: {record['stage']}")
    if scenario == "keyboard":
        samples = {record['stage']: record for record in evidence['records'] if record['stage'].startswith('editor-keyboard-')}
        for stage in ('editor-keyboard-dark', 'editor-keyboard-save-dark', 'editor-keyboard-after-back'):
            record = samples[stage]
            native = record.get('nativeViewport', {})
            visible = stage != 'editor-keyboard-after-back'
            if (native.get('imeVisible') is not visible or (visible and native.get('imeHeightPx', 0) <= 0)
                    or record.get('path') != '/editor' or not record.get('hittable') or not record.get('unobscured') or record.get('overlapped')
                    or record.get('controlBottomOnScreenPx', float('inf')) > record.get('keyboardTopOnScreenPx', 0) + 1
                    or (visible and (record.get('toastCount') != 1 or not record.get('backupReminder')))):
                raise RuntimeError(f"Incomplete native keyboard/control evidence: {stage}")
        before, after = samples['editor-keyboard-dark'], samples['editor-keyboard-after-back']
        if (after.get('draft') != before.get('draft', '') + ' XYZ'
                or after.get('draftBeforeBack') != after.get('draft')
                or before.get('historyIndex') != after.get('historyIndex')
                or after.get('historyBeforeBack') != after.get('historyIndex')
                or after.get('webViewHeightBeforeImePx') != after.get('nativeViewport', {}).get('webViewHeightPx')):
            raise RuntimeError('Keyboard dismissal changed draft or navigation history')
    return stages


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("adb", "serial", "avd", "apk", "test-apk", "output"):
        parser.add_argument(f"--{name}", required=True)
    parser.add_argument("--mode", choices=("supported", "fallback"), required=True)
    parser.add_argument("--scenario", choices=("baseline", "overlays", "keyboard"), default="baseline")
    args = parser.parse_args()
    if not re.fullmatch(r"emulator-\d+", args.serial) or not re.fullmatch(r"SoloSoul_[A-Za-z0-9_-]+", args.avd):
        parser.error("Requires explicitly named dedicated SoloSoul emulator")
    output = Path(args.output).resolve()
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    report = {"serial": args.serial, "avd": args.avd, "mode": args.mode, "scenario": args.scenario,
              "startedAt": time.time(), "passed": False, "restored": False}
    selected = False
    backup_valid = False
    mutation_started = False
    old_blur = None
    old_notification = None
    backup = output / "private-data-before.tar"
    before = None
    root_mode = None

    def adb(*command, **kwargs):
        return subprocess.run([args.adb, "-s", args.serial, *command], check=True,
                              timeout=240, **kwargs)

    def read(*command):
        return adb(*command, stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode().strip()

    def archive(path):
        with open(path, "xb") as stream:
            os.chmod(path, 0o600)
            adb("exec-out", "run-as", APP, "tar", "-cf", "-", ".", stdout=stream, stderr=subprocess.PIPE)

    try:
        devices = read("devices", "-l").splitlines()
        if not any(line.split()[:2] == [args.serial, "device"] for line in devices):
            raise RuntimeError("Dedicated emulator missing/offline/unauthorized")
        if read("emu", "avd", "name").splitlines()[0] != args.avd:
            raise RuntimeError("AVD identity mismatch")
        selected = True
        report["api"] = int(read("shell", "getprop", "ro.build.version.sdk"))
        report["abi"] = read("shell", "getprop", "ro.product.cpu.abi")
        if report["api"] < 31:
            raise RuntimeError("Requires API >=31")
        if report["api"] >= 33:
            old_notification = notification_permission(read("shell", "dumpsys", "package", APP))
            report["notificationPermissionBefore"] = old_notification
        report["artifacts"] = [{"path": str(Path(path).resolve()), "sha256": digest(path)}
                               for path in (args.apk, args.test_apk)]
        adb("shell", "am", "force-stop", APP, stdout=subprocess.PIPE)
        # 先备份已安装专用应用的数据；install -r 保留数据且不得先执行 clear。
        root_mode = read("shell", "run-as", APP, "stat", "-c", "%a", ".")
        archive(backup)
        before = inventory(backup, require_empty=True)
        backup_valid = True
        report["backup"] = {"path": str(backup), "sha256": digest(backup), "entries": len(before)}
        old_blur = read("shell", "settings", "get", "global", "disable_window_blurs")
        adb("shell", "settings", "put", "global", "disable_window_blurs", "1" if args.mode == "fallback" else "0", stdout=subprocess.PIPE)
        mutation_started = True
        adb("install", "-r", str(Path(args.apk).resolve()), stdout=subprocess.PIPE)
        adb("install", "-r", str(Path(args.test_apk).resolve()), stdout=subprocess.PIPE)
        tag = f"fe2-home-{uuid.uuid4().hex}"
        # 输出由 JUnit 最后写入；不关闭最后一个 Activity 以免 Tauri 退出同进程 runner。
        result = adb("shell", "am", "instrument", "-w", "-r", "-e", "waitForActivitiesToComplete", "false",
                     "-e", "privateDataBackedUp", "true", "-e", "evidenceTag", tag,
                     "-e", "scenario", args.scenario,
                     "-e", "class", f"com.solosoul.app.AndroidHomeInstrumentedTest#{METHOD}",
                     "com.solosoul.app.test/androidx.test.runner.AndroidJUnitRunner",
                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        text = result.stdout.decode()
        (output / "native.log").write_text(text)
        adb("pull", f"/sdcard/Android/data/{APP}/files/{tag}", str(output / "device-files"), stdout=subprocess.PIPE)
        verify_native(text)
        evidence = json.loads((output / "device-files/report.json").read_text())
        stages = verify_evidence(evidence, args.scenario)
        report["stages"] = stages
        report["nativePassed"] = True
    except Exception as error:
        report["error"] = str(error)
    finally:
        if selected:
            try:
                adb("shell", "am", "force-stop", APP, stdout=subprocess.PIPE)
                if backup_valid and mutation_started:
                    # 只移除专用应用私有根下的成员，拒绝不符合单一文件名的输出。
                    names = read("shell", "run-as", APP, "ls", "-A").splitlines()
                    if any(not re.fullmatch(r"[A-Za-z0-9_.-]+", name) or name in (".", "..") for name in names):
                        raise RuntimeError("Unsafe private root entry; keep backup for manual recovery")
                    if names:
                        adb("shell", "run-as", APP, "rm", "-rf", "--", *names, stdout=subprocess.PIPE)
                    # adb exec-in 的 stdin 在部分版本会提前结束，toybox tar 对截断
                    # 归档仍可能退出 0；使用完整落盘归档并保持权限，最终逐文件校验。
                    remote_backup = f"/data/local/tmp/fe2-private-backup-{uuid.uuid4().hex}.tar"
                    adb("push", str(backup), remote_backup, stdout=subprocess.PIPE)
                    adb("shell", "chmod", "644", remote_backup, stdout=subprocess.PIPE)
                    adb("shell", "run-as", APP, "tar", "-xpf", remote_backup,
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    adb("shell", "run-as", APP, "chmod", root_mode, ".", stdout=subprocess.PIPE)
                    restored = output / "private-data-restored.tar"
                    archive(restored)
                    after = inventory(restored, require_empty=True)
                    if before != after:
                        changed = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
                        report["restoreChangedPaths"] = changed
                        raise RuntimeError("Private data does not match backup")
                    report["restoreVerification"] = {"entries": len(after), "allFileContentsLinksAndModesMatch": True}
                    if read("shell", "run-as", APP, "stat", "-c", "%a", ".") != root_mode:
                        raise RuntimeError("Private root mode differs from backup")
                    adb("shell", "rm", remote_backup, stdout=subprocess.PIPE)
                    report["restored"] = True
            except Exception as error:
                report["restoreError"] = str(error)
            # 私有目录恢复异常也必须尝试恢复系统模糊设置，两项清理互不依赖。
            if old_blur is not None:
                try:
                    command = ("delete",) if old_blur == "null" else ("put",)
                    adb("shell", "settings", *command, "global", "disable_window_blurs",
                        *(() if old_blur == "null" else (old_blur,)), stdout=subprocess.PIPE)
                    report["blurRestored"] = True
                except Exception as error:
                    report["blurRestoreError"] = str(error)
            if old_notification is not None:
                try:
                    current = notification_permission(read("shell", "dumpsys", "package", APP))
                    if current["granted"] != old_notification["granted"]:
                        adb("shell", "pm", "grant" if old_notification["granted"] else "revoke",
                            APP, NOTIFICATION, stdout=subprocess.PIPE)
                    # 通知对话框可能设置 USER_SET / USER_FIXED；只恢复本应用这一权限。
                    adb("shell", "pm", "clear-permission-flags", APP, NOTIFICATION,
                        "user-set", "user-fixed", stdout=subprocess.PIPE)
                    flags = [flag.lower().replace("_", "-") for flag in old_notification["flags"]
                             if flag in ("USER_SET", "USER_FIXED")]
                    if flags:
                        adb("shell", "pm", "set-permission-flags", APP, NOTIFICATION,
                            *flags, stdout=subprocess.PIPE)
                    actual = notification_permission(read("shell", "dumpsys", "package", APP))
                    if actual != old_notification:
                        raise RuntimeError("Notification permission/flags differ from original state")
                    report["notificationPermissionRestored"] = True
                except Exception as error:
                    report["notificationRestoreError"] = str(error)
        report["passed"] = verification_passed(report, old_notification is not None)
        report["finishedAt"] = time.time()
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        suite = ET.Element("testsuite", name="android-production-home", tests="1",
                           failures="0" if report["passed"] else "1", skipped="0")
        case = ET.SubElement(suite, "testcase", classname="AndroidHomeInstrumentedTest", name=METHOD)
        if not report["passed"]:
            ET.SubElement(case, "failure", message=report.get("restoreError", report.get("error", "Incomplete evidence")))
        ET.ElementTree(suite).write(output / "junit.xml", encoding="utf-8", xml_declaration=True)
        print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
