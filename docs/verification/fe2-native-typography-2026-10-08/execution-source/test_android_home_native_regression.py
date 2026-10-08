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
    def test_font_scale_mutation_requires_exact_restoration(self):
        for original in ('null', '1', '1.0', '1.30', '2.0'):
            self.assertEqual(runner.font_scale(original + '\n'), original)
        for invalid in ('', 'NaN', 'inf', '-1', '0', '1; reboot', 'font_scale=1.0'):
            with self.subTest(invalid=invalid), self.assertRaises(RuntimeError):
                runner.font_scale(invalid)
        completed = {'nativePassed': True, 'restored': True, 'blurRestored': True,
                     'notificationPermissionRestored': True, 'requestedFontScale': 1.3}
        self.assertFalse(runner.verification_passed(completed, True))
        self.assertTrue(runner.verification_passed({**completed, 'fontScaleRestored': True}, True))
        self.assertFalse(runner.verification_passed({**completed, 'fontScaleRestored': True,
                                                   'fontScaleRestoreError': 'wrong value'}, True))

    def test_actual_rendering_work_rejects_false_idle_or_missing_positive_frame(self):
        path = Path(__file__).parents[2] / 'docs/verification/fe2-rendering-work-2026-10-08/native-api34-supported/device-files/report.json'
        accepted = json.loads(path.read_text())
        runner.verify_evidence(accepted, 'rendering')
        def continuing(row):
            draw = dict(next(r for r in accepted['records'] if r['stage'] == 'rendering-return')['draws'][0])
            draw['phase'] = row['phase']
            row.update(drawCalls=1, draws=[draw])
        for stage, change in (
            ('rendering-idle', lambda row: row.update(hookInstalled=False)),
            ('rendering-idle', lambda row: row.update(elapsedMs=10)),
            ('rendering-idle', continuing),
            ('rendering-away', lambda row: row.update(artworkMounted=True)),
            ('rendering-return', lambda row: row.update(drawCalls=0, draws=[])),
            ('rendering-return', lambda row: row['draws'][0].update(submitMs=float('nan'))),
            ('rendering-return-ready', lambda row: row.update(copyInkPixels=0)),
            ('rendering-return-idle', continuing),
        ):
            invalid = json.loads(json.dumps(accepted))
            change(next(row for row in invalid['records'] if row['stage'] == stage))
            with self.subTest(stage=stage), self.assertRaises(RuntimeError):
                runner.verify_evidence(invalid, 'rendering')

    def test_original_safe_area_reports_do_not_prove_notification_safe_area(self):
        root = Path(__file__).parents[2] / 'docs/verification/fe2-api31-insets-2026-10-08'
        for mode in ('supported', 'fallback'):
            original = json.loads((root / f'native-edge-{mode}/device-files/report.json').read_text())
            runner.verify_evidence(original, 'insets')
            with self.subTest(mode=mode), self.assertRaisesRegex(RuntimeError, 'notification safe-area'):
                runner.verify_evidence(original, 'insets', require_safe_notifications=True)

    def test_actual_notification_safe_area_rejects_missing_or_occluded_actions(self):
        path = Path(__file__).parents[2] / 'docs/verification/fe2-api31-insets-2026-10-08/native-notification-initial-supported/device-files/report.json'
        accepted = json.loads(path.read_text())
        runner.verify_evidence(accepted, 'insets', require_safe_notifications=True)
        for change in (
            lambda row: row.update(backupReminder=False),
            lambda row: row.update(targets=[target for target in row['targets'] if not target.get('notification')]),
            lambda row: next(target for target in row['targets'] if target.get('notification')).update(hittable=False),
            lambda row: next(target for target in row['targets'] if target.get('notification'))['rect'].update(right=row['width']),
        ):
            invalid = json.loads(json.dumps(accepted))
            change(next(row for row in invalid['records'] if row['stage'] == 'safe-home-landscape'))
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                runner.verify_evidence(invalid, 'insets', require_safe_notifications=True)

    def test_real_old_preview_reports_do_not_prove_system_bar_contrast(self):
        root = Path(__file__).parents[2] / 'docs/verification/fe2-api31-2026-10-08'
        for mode in ('supported', 'fallback'):
            original = json.loads((root / f'native-api31-{mode}/device-files/report.json').read_text())
            # 原始内容验收结论保留；浅色预览白色系统图标是后来目测发现的实际缺项。
            runner.verify_evidence(original, 'previews')
            with self.subTest(mode=mode), self.assertRaisesRegex(RuntimeError, 'system bar theme'):
                runner.verify_evidence(original, 'previews', require_preview_bars=True)

    def test_actual_preview_matrix_rejects_missing_content_notification_and_return(self):
        root = Path(__file__).parents[2] / 'docs/verification/fe2-preview-2026-10-08'
        for mode in ('supported', 'fallback'):
            accepted = json.loads((root / f'native-{mode}/device-files/report.json').read_text())
            runner.verify_evidence(accepted, 'previews')
            changes = [
                ('preview-fixtures-encrypted', lambda row: row.update(encrypted=False)),
                ('nonempty-attachments-dark', lambda row: row.update(settled=False)),
                ('file-preview-text-dark', lambda row: row.update(textInkPixels=0)),
                ('file-preview-text-light', lambda row: row.update(preText='missing')),
                ('file-preview-image-light', lambda row: row.update(imageDecoded=False)),
                ('photo-album-light', lambda row: row.update(imageNaturalWidth=1)),
                ('photo-viewer-dark', lambda row: row.update(quadrantPixels=[[0, 0, 0]] * 4)),
                ('photo-viewer-light', lambda row: row.update(imageNaturalHeight=1)),
                ('photo-viewer-light', lambda row: row.update(zoomText='')),
                ('photo-viewer-light', lambda row: row.update(counter='0 / 1')),
                ('photo-viewer-dark', lambda row: row.update(notificationPresentAtCapture=False)),
                ('photo-viewer-light', lambda row: row.update(savedNotification=False)),
                ('photo-viewer-light', lambda row: row.update(toastInPanel=False)),
                ('photo-album-after-back-light', lambda row: row.update(toastInFlow=False)),
                ('attachments-after-back-dark', lambda row: row['controls'][0].update(overlapped=True)),
                ('nonempty-attachments-light', lambda row: row['controls'][0].update(hittable=False)),
                ('file-preview-image-dark', lambda row: row['controls'][0]['rect'].update(bottom=9999)),
                ('preview-returned-detail-light', lambda row: row.update(detail=False)),
                ('preview-returned-detail-dark', lambda row: row.update(search='?section=all')),
                ('preview-returned-detail-light', lambda row: row['systemBars'].update(statusBarLight=False)),
            ]
            for stage, change in changes:
                invalid = json.loads(json.dumps(accepted))
                change(next(row for row in invalid['records'] if row['stage'] == stage))
                with self.subTest(mode=mode, stage=stage), self.assertRaises(RuntimeError):
                    runner.verify_evidence(invalid, 'previews')
            with self.subTest(mode=mode):
                native = root / f'native-{mode}'
                runner.verify_native((native / 'native.log').read_text())
                report = json.loads((native / 'report.json').read_text())
                self.assertTrue(runner.verification_passed(report, True))
                for key in ('nativePassed', 'restored', 'blurRestored', 'notificationPermissionRestored'):
                    self.assertFalse(runner.verification_passed({**report, key: False}, True))

    def test_old_home_and_failed_preview_do_not_prove_nonempty_preview_matrix(self):
        root = Path(__file__).parents[2] / 'docs/verification'
        old = json.loads((root / 'fe2-motion-2026-10-08/native-supported/device-files/report.json').read_text())
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(old, 'previews')
        old['scenario'] = 'previews'
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(old, 'previews')
        failed = root / 'fe2-preview-2026-10-08/attempt-2-supported'
        runner_report = json.loads((failed / 'report.json').read_text())
        self.assertFalse(runner.verification_passed(runner_report, True))
        with self.assertRaises(RuntimeError):
            runner.verify_native((failed / 'native.log').read_text())
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(json.loads((failed / 'device-files/report.json').read_text()), 'previews')

    def test_actual_motion_matrix_rejects_wrong_preference_delivery_and_duration(self):
        root = Path(__file__).parents[2] / 'docs/verification/fe2-motion-2026-10-08/native-supported'
        accepted = json.loads((root / 'cold-device-files/report.json').read_text())
        runner.verify_evidence(accepted, 'cold', True)
        changes = [
            ('cold-login', lambda row: row.update(reduceMotion='false')),
            ('account-a-cold-system-light', lambda row: row.update(reduceMotion='false')),
            ('account-a-cold-system-dark', lambda row: row['motionEffect'].update(navigationTransition='0.32s')),
            ('account-b-cold-system-dark', lambda row: row['savedPreferences'].update(reduceMotion=True)),
            ('account-b-cold-system-dark', lambda row: row.update(reduceMotion='true')),
        ]
        for stage, change in changes:
            invalid = json.loads(json.dumps(accepted))
            change(next(row for row in invalid['records'] if row['stage'] == stage))
            with self.subTest(stage=stage), self.assertRaises(RuntimeError):
                runner.verify_evidence(invalid, 'cold', True)
        invalid = json.loads(json.dumps(accepted))
        invalid.pop('motionPreferenceChecks')
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(invalid, 'cold', True)

    def test_previous_cold_report_does_not_prove_motion_or_partial_acceptance(self):
        path = Path(__file__).parents[2] / 'docs/verification/fe2-cold-2026-10-08/final-supported/cold-device-files/report.json'
        old = json.loads(path.read_text())
        runner.verify_evidence(old, 'cold')
        with self.assertRaisesRegex(RuntimeError, 'reduced-motion'):
            runner.verify_evidence(old, 'cold', True)
        report = json.loads((path.parent.parent / 'report.json').read_text())
        self.assertTrue(runner.verification_passed(report, True))
        report['motionPreferenceChecks'] = True
        self.assertFalse(runner.verification_passed(report, True))

    def test_cold_previous_process_can_be_already_exited_but_not_another_process(self):
        self.assertTrue(runner.validate_previous_process('', 8044))
        self.assertFalse(runner.validate_previous_process('8044\n', 8044))
        for value in ('9999', '8044 9999', 'unknown'):
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                runner.validate_previous_process(value, 8044)

    def test_cold_requires_second_phase_and_cannot_accept_activity_recreation(self):
        old = json.loads((Path(__file__).parents[2] / 'docs/verification/fe2-lifecycle-2026-10-08/native-supported/device-files/report.json').read_text())
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(old, 'cold')
        old['scenario'] = 'cold'
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(old, 'cold')
        complete = {'scenario': 'cold', 'nativePassed': True, 'restored': True,
                    'blurRestored': True, 'nightModeRestored': True, 'notificationPermissionRestored': True}
        self.assertFalse(runner.verification_passed(complete, True))
        self.assertFalse(runner.verification_passed({**complete, 'coldPassed': True}, True))
        self.assertTrue(runner.verification_passed({**complete, 'coldPassed': True, 'oldProcessStopped': True}, True))
        text = (f'INSTRUMENTATION_STATUS: test={runner.COLD_METHOD}\n'
                'INSTRUMENTATION_STATUS_CODE: 1\nINSTRUMENTATION_STATUS_CODE: 0\n'
                'OK (1 test)\nINSTRUMENTATION_CODE: -1\n')
        runner.verify_native(text, runner.COLD_METHOD)
        with self.assertRaises(RuntimeError):
            runner.verify_native(text)

    def test_actual_lifecycle_rejects_missing_recreation_and_wrong_native_mode(self):
        accepted = json.loads((Path(__file__).parents[2] / 'docs/verification/fe2-lifecycle-2026-10-08/native-supported/device-files/report.json').read_text())
        runner.verify_evidence(accepted, 'lifecycle')
        for stage, mutate in [
            ('account-a-after-activity-recreate', lambda row: row.update(activityRecreated=False)),
            ('account-b-after-activity-recreate', lambda row: row.update(webViewRecreated=False)),
            ('account-a-system-light', lambda row: row.update(systemNightMode=True)),
            ('account-b-system-dark', lambda row: row['systemBars'].update(nightMode=False)),
            ('account-a-after-activity-recreate', lambda row: row.update(accountId='foreign-account')),
            ('account-b-after-activity-recreate', lambda row: row.update(copyInkPixels=0)),
        ]:
            invalid = json.loads(json.dumps(accepted))
            mutate(next(row for row in invalid['records'] if row['stage'] == stage))
            with self.subTest(stage=stage), self.assertRaises(RuntimeError):
                runner.verify_evidence(invalid, 'lifecycle')

    def test_lifecycle_requires_new_native_stages_and_night_mode_restoration(self):
        old = json.loads((Path(__file__).parents[2] / 'docs/verification/fe2-accounts-2026-10-07/contrast-native-supported/device-files/report.json').read_text())
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(old, 'lifecycle')
        old['scenario'] = 'lifecycle'
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(old, 'lifecycle')
        complete = {'scenario': 'lifecycle', 'nativePassed': True, 'restored': True,
                    'blurRestored': True, 'notificationPermissionRestored': True}
        self.assertFalse(runner.verification_passed(complete, True))
        self.assertTrue(runner.verification_passed({**complete, 'nightModeRestored': True}, True))
        self.assertFalse(runner.verification_passed({**complete, 'nightModeRestored': True,
                                                   'nightModeRestoreError': 'failed'}, True))

    def test_night_mode_requires_recognized_actual_system_reply(self):
        for mode in ('auto', 'no', 'yes', 'custom'):
            self.assertEqual(runner.night_mode('Night mode: ' + mode + '\n'), mode)
        for reply in ('', 'yes', 'Night mode: unsupported', 'Permission denied'):
            with self.subTest(reply=reply), self.assertRaises(RuntimeError):
                runner.night_mode(reply)

    def test_actual_safe_area_report_rejects_overlap_missing_ime_and_lost_history(self):
        path = Path(__file__).parents[2] / 'docs/verification/fe2-safe-area-2026-10-07/final-native-supported/device-files/report.json'
        accepted = json.loads(path.read_text())
        runner.verify_evidence(accepted, 'insets')
        for stage, change in [
            ('safe-home-portrait', lambda row: row['targets'][0]['rect'].update(y=0)),
            ('safe-home-landscape', lambda row: row.update(width=row['height'])),
            ('safe-home-after-rotation', lambda row: row['targets'][0].update(hittable=False)),
            ('safe-login-keyboard', lambda row: row['targets'][-1]['rect'].update(bottom=9999)),
            ('safe-login-keyboard', lambda row: row['nativeViewport'].update(imeVisible=False)),
            ('safe-login-after-keyboard-back', lambda row: row.update(historyIndex=-1)),
            ('safe-login-after-document-reload', lambda row: row['nativeViewport'].update(webViewHeightPx=1)),
        ]:
            invalid = json.loads(json.dumps(accepted))
            change(next(row for row in invalid['records'] if row['stage'] == stage))
            with self.subTest(stage=stage), self.assertRaises(RuntimeError):
                runner.verify_evidence(invalid, 'insets')

    def test_safe_area_requires_complete_native_rotation_keyboard_and_reload(self):
        root = Path(__file__).parents[2] / 'docs/verification'
        for path in [root / 'fe2-overlay-toast-2026-10-07/native-supported/device-files/report.json',
                     root / 'fe2-safe-area-2026-10-07/before-native/device-files/report.json']:
            with self.subTest(path=path), self.assertRaises(RuntimeError):
                runner.verify_evidence(json.loads(path.read_text()), 'insets')

    def test_actual_account_acceptance_rejects_unreadable_or_missing_action_pixels(self):
        accepted = json.loads((Path(__file__).parents[2] / "docs/verification/fe2-accounts-2026-10-07/contrast-native-supported/device-files/report.json").read_text())
        runner.verify_evidence(accepted, 'accounts')
        for key, value in [('contrast', 1.0), ('inkPixels', 0), ('visible', False), ('hittable', False)]:
            invalid = json.loads(json.dumps(accepted))
            next(row for row in invalid['records'] if row['stage'] == 'account-a-after-switch')['action'][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                runner.verify_evidence(invalid, 'accounts')
        invalid = json.loads(json.dumps(accepted))
        next(row for row in invalid['records'] if row['stage'] == 'account-b-after-switch')['savedPreferences']['customAccentHex'] = '#112233'
        with self.assertRaises(RuntimeError):
            runner.verify_evidence(invalid, 'accounts')

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

    def test_real_system_bar_pass_does_not_prove_preview_header_visibility(self):
        root = Path(__file__).parents[2] / 'docs/verification/fe2-api31-2026-10-08'
        for mode in ('supported', 'fallback'):
            report = json.loads((root / f'native-api31-bars-{mode}/device-files/report.json').read_text())
            runner.verify_evidence(report, 'previews', require_preview_bars=True)
            with self.assertRaisesRegex(RuntimeError, 'legible native preview header'):
                runner.verify_evidence(report, 'previews', require_preview_bars=True, require_preview_ink=True)

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
