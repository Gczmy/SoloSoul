"""首页原生驱动的恢复与拒绝条件；不连接设备。"""
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("home_runner", Path(__file__).with_name("android-home-native-regression.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class HomeRunnerTests(unittest.TestCase):
    def test_accounts_before_contrast_fix_are_not_complete_visual_acceptance(self):
        old = json.loads((Path(__file__).parents[2] / "docs/verification/fe2-accounts-2026-10-07/native-supported/device-files/report.json").read_text())
        with self.assertRaisesRegex(RuntimeError, 'account theme/persistence'):
            runner.verify_evidence(old, 'accounts')

    def test_accounts_scenario_rejects_baseline_and_unverified_account_preferences(self):
        baseline = json.loads((Path(__file__).parents[2] / "docs/verification/fe2-overlay-toast-2026-10-07/native-supported/device-files/report.json").read_text())
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(baseline, 'accounts')
        accounts = json.loads(json.dumps(baseline))
        accounts['scenario'] = 'accounts'
        accounts['records'].extend({'stage': stage} for stage in
                                  ['account-a-custom-dark', 'account-b-custom-light', 'account-a-after-switch',
                                   'account-b-after-switch', 'account-a-after-second-switch', 'account-preferences-isolated'])
        with self.assertRaisesRegex(RuntimeError, 'account theme/persistence'):
            runner.verify_evidence(accounts, 'accounts')

    def test_keyboard_acceptance_requires_actual_reminder_draft_and_restored_height(self):
        accepted = json.loads((Path(__file__).parents[2] / "docs/verification/fe2-keyboard-2026-10-07/native-supported/device-files/report.json").read_text())
        runner.verify_evidence(accepted, 'keyboard')
        # 使用实际通过报告构造拒绝条件，不能用自造的几何数据证明设备通过。
        changes = [('editor-keyboard-dark', 'backupReminder', False),
                   ('editor-keyboard-save-dark', 'unobscured', False),
                   ('editor-keyboard-after-back', 'draftBeforeBack', 'lost draft'),
                   ('editor-keyboard-after-back', 'historyBeforeBack', -1),
                   ('editor-keyboard-after-back', 'webViewHeightBeforeImePx', 1)]
        for stage, key, value in changes:
            invalid = json.loads(json.dumps(accepted))
            next(record for record in invalid['records'] if record['stage'] == stage)[key] = value
            with self.subTest(stage=stage, key=key), self.assertRaises(RuntimeError):
                runner.verify_evidence(invalid, 'keyboard')

    def test_keyboard_scenario_rejects_report_without_actual_ime_evidence(self):
        baseline = json.loads((Path(__file__).parents[2] / "docs/verification/fe2-overlay-toast-2026-10-07/native-supported/device-files/report.json").read_text())
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(baseline, 'keyboard')
        keyboard = json.loads(json.dumps(baseline))
        keyboard['scenario'] = 'keyboard'
        keyboard['records'][6:6] = [{'stage':stage, 'path':'/editor', 'hittable':True,
                                   'nativeViewport':{'imeVisible':False,'imeHeightPx':0}}
                                  for stage in ('editor-keyboard-dark','editor-keyboard-save-dark','editor-keyboard-after-back')]
        with self.assertRaisesRegex(RuntimeError, 'native keyboard'):
            runner.verify_evidence(keyboard, 'keyboard')

    def test_overlay_run_rejects_baseline_or_nonforeground_notification_evidence(self):
        baseline = json.loads((Path(__file__).parents[2] / "docs/verification/fe2-overlay-toast-2026-10-07/native-supported/device-files/report.json").read_text())
        runner.verify_evidence(baseline, "baseline")
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(baseline, "overlays")
        # 仅在已有生产报告前提下构造拒绝用例，不伪造设备通过结果。
        overlays = json.loads(json.dumps(baseline))
        overlays["scenario"] = "overlays"
        attachment = {**baseline["records"][5], "stage": "object-attachments-dark", "toastBackdrop": "4000"}
        history = {**attachment, "stage": "object-history-dark"}
        overlays["records"][8:8] = [attachment, history, {"stage": "nested-overlays-after-system-back"}]
        with self.assertRaisesRegex(RuntimeError, "foreground notification"):
            runner.verify_evidence(overlays, "overlays")

    def test_cleanup_failure_cannot_be_accepted_with_partial_success_flags(self):
        completed = {"nativePassed": True, "restored": True, "blurRestored": True,
                     "notificationPermissionRestored": True}
        self.assertTrue(runner.verification_passed(completed, True))
        for key in ("error", "restoreError", "blurRestoreError", "notificationRestoreError"):
            self.assertFalse(runner.verification_passed({**completed, key: "cleanup failed"}, True))
        self.assertFalse(runner.verification_passed({**completed, "notificationPermissionRestored": False}, True))

    def test_notification_restore_tracks_grant_and_user_flags(self):
        text = ' android.permission.POST_NOTIFICATIONS: granted=false, flags=[ USER_SET|USER_SENSITIVE_WHEN_DENIED ]'
        self.assertEqual(runner.notification_permission(text), {
            "granted": False, "flags": ["USER_SENSITIVE_WHEN_DENIED", "USER_SET"]})
        self.assertTrue(runner.notification_permission(text.replace('granted=false', 'granted=true'))['granted'])
        with self.assertRaises(RuntimeError):
            runner.notification_permission('requested permissions: android.permission.POST_NOTIFICATIONS')

    def archive(self, path, entries):
        with tarfile.open(path, "w") as archive:
            for name, content, mode in entries:
                info = tarfile.TarInfo(name)
                info.mode = mode
                data = content.encode()
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))

    def test_existing_account_and_unsafe_backup_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "data.tar"
            for name, text in (("accounts.json", '[{"id":"acc_existing"}]'),
                               ("acc_orphan/config.json", "{}"),
                               ("../outside", "{}")):
                self.archive(path, [(name, text, 0o600)])
                with self.assertRaises(RuntimeError):
                    runner.inventory(path, require_empty=True)

    def test_restoration_detects_changed_bytes_and_permissions(self):
        with tempfile.TemporaryDirectory() as directory:
            before = Path(directory) / "before.tar"
            after = Path(directory) / "after.tar"
            self.archive(before, [("ui_preferences.json", '{"theme":"dark"}', 0o600)])
            expected = runner.inventory(before)
            for text, mode in (( '{"theme":"light"}', 0o600), ('{"theme":"dark"}', 0o644)):
                self.archive(after, [("ui_preferences.json", text, mode)])
                self.assertNotEqual(expected, runner.inventory(after))
            self.archive(after, [("ui_preferences.json", '{"theme":"dark"}', 0o600)])
            self.assertEqual(expected, runner.inventory(after))
            self.assertNotIn("dark", json.dumps(expected))

    def test_native_skip_crash_zero_tests_and_wrong_method_are_failures(self):
        text = (f"INSTRUMENTATION_STATUS: test={runner.METHOD}\n"
                "INSTRUMENTATION_STATUS_CODE: 1\nINSTRUMENTATION_STATUS_CODE: 0\n"
                "OK (1 test)\nINSTRUMENTATION_CODE: -1\n")
        runner.verify_native(text)
        for invalid in (text.replace("CODE: 0", "CODE: -3"), text.replace("CODE: 0", "CODE: -2"),
                        text.replace("1 test", "0 tests"), text.replace(runner.METHOD, "other"),
                        text + "Process crashed", text.replace("INSTRUMENTATION_CODE: -1", "")):
            with self.assertRaises(RuntimeError):
                runner.verify_native(invalid)


if __name__ == "__main__":
    unittest.main()
