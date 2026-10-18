"""Synthetic rejection tests; these do not prove native runtime behavior."""
import copy
import unittest
import test_android_recovery_native_evidence as recovery
import test_android_recovery_native_taps as taps

runner = recovery.runner

def evidence():
    template = recovery.evidence()['recoveryRecords'][0]
    suffixes = ['disabled', 'disabled-closed', 'host-portrait', 'host-cancel-portrait', 'host-landscape',
                'host-cancel-landscape', 'host-close-landscape', 'host-closed', 'sync-portrait',
                'sync-close-portrait', 'sync-landscape', 'sync-close-landscape', 'sync-closed',
                'scan-portrait', 'scan-manual-portrait', 'scan-landscape', 'scan-manual-landscape',
                'scan-close-landscape', 'scan-closed', 'returned-home']
    rows = []
    for theme in ('light', 'dark'):
        for suffix in suffixes:
            kind = suffix.split('-')[0]
            export = 1 if kind == 'host' and not suffix.endswith('-closed') else 0
            if suffix == 'returned-home':
                row = dict(unlocked=True, syncDisabled=True, exportFileCount=0)
            elif suffix.endswith('-closed'):
                row = dict(closed=True, path='/sync', exportFileCount=0, syncDisabled=kind in ('disabled', 'host'))
            else:
                row = copy.deepcopy(template)
                landscape = 'landscape' in suffix
                width, height = (844, 390) if landscape else (390, 844)
                row.update(path='/sync', kind=kind, width=width, height=height, qrCount=1 if kind in ('host', 'sync') else 0,
                           manualOpen=kind == 'host', manualAction=kind == 'scan', hostPinPresent=kind == 'host',
                           headings=[] if kind == 'disabled' else [{'host': 'Link New Device', 'sync': 'Sync QR Code', 'scan': 'Scan QR Code'}[kind]],
                           disabledMessage=kind == 'disabled', syncDisabled=kind in ('disabled', 'host'), exportFileCount=export)
                row['card'].update(height=height-56, bottom=height-40)
                native = row['nativeViewport']
                native['windowBounds'].update(right=1600 if landscape else 720, bottom=720 if landscape else 1600)
                native.update(webViewWidthPx=1600 if landscape else 720, screenHeightPx=720 if landscape else 1600)
                row['action'] = None
                row.update(cardClientWidth=350, cardScrollWidth=350)
                if suffix in ('disabled', 'host-close-landscape', 'sync-close-landscape', 'scan-close-landscape'):
                    row['action'] = dict(caption='Close', aria=True, hittable=True)
                elif '-cancel-' in suffix:
                    row['action'] = dict(caption='Cancel', aria=False, hittable=True)
                elif suffix == 'sync-close-portrait':
                    row['action'] = dict(caption='Close', aria=False, hittable=True)
                elif '-manual-' in suffix:
                    row['action'] = dict(caption='Use discovery or manual input', aria=False, hittable=True)
            row.update(stage=f'qr-{theme}-{suffix}', theme=theme)
            rows.append(row)
    result = dict(qrCardsRequested=True, recoveryCardsRequested=False, scenario='baseline', qrRecords=rows,
                  runtime=dict(api=35, webViewPackage='com.android.webview', webViewVersion='124'))
    result['qrEnableReadiness'] = [dict(theme=t, phase='qr-enable-sync', explicitEnablePending=True, path='/sync', syncEnabled=True, showReady=True, realIpc=True, mocked=False, nearbyPermissionGranted=True) for t in ('light', 'dark')]
    native = taps.evidence()
    result.update(recoveryTapPlans=native['recoveryTapPlans'], recoveryPointerEvents=native['recoveryPointerEvents'])
    for p, e in zip(result['recoveryTapPlans'], result['recoveryPointerEvents']):
        p.update(caption='Close', label=False, activeAfter='', cardPresentAfter=False)
        p['geometry'].update(associated=False, id=None)
        e.update(buttonLabel='Close', labelFor=None)
    return result

class QrEvidenceTests(unittest.TestCase):
    def test_complete_actual_callers_and_single_native_close_proof(self):
        self.assertEqual(len(runner.verify_qr_evidence(evidence())), 40)
        self.assertEqual(runner.verify_qr_taps(evidence()), 8)

    def test_explicit_complete_unmixed_baseline_and_runtime_are_required(self):
        for key, value in [('qrCardsRequested', False), ('recoveryCardsRequested', True), ('scenario', 'overlays'), ('runtime', {})]:
            data = evidence(); data[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)
        for mutation in ('missing', 'duplicate', 'swapped'):
            data = evidence()
            if mutation == 'missing': data['qrRecords'].pop()
            elif mutation == 'duplicate': data['qrRecords'].append(copy.deepcopy(data['qrRecords'][-1]))
            else: data['qrRecords'][0], data['qrRecords'][1] = data['qrRecords'][1], data['qrRecords'][0]
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)

    def test_claims_cannot_replace_real_dom_native_and_theme_states(self):
        for key, value in [('realIpc', False), ('mocked', True), ('bounded', 1), ('documentOverflow', True),
                           ('screenshotStateConfirmed', False), ('path', '/fixture'), ('platform', 'windows'), ('theme', 'dark')]:
            data = evidence(); data['qrRecords'][0][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)
        data = evidence(); data['qrRecords'][0]['nativeViewport']['imeVisible'] = True
        with self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)

    def test_clipping_nonfinite_missing_native_bounds_and_rotation_are_rejected(self):
        for key, value in [('x', -1), ('bottom', 3000), ('right', 400), ('width', float('nan')), ('height', 0)]:
            data = evidence(); data['qrRecords'][0]['card'][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)
        for mutation in ('rotation', 'no-scroll', 'missing-native', 'native-bar'):
            data = evidence(); row = data['qrRecords'][4]
            if mutation == 'rotation': row['nativeViewport']['windowBounds']['bottom'] = 2000
            elif mutation == 'no-scroll': row['scrollHeight'] = row['clientHeight']
            elif mutation == 'missing-native': row['nativeViewport'].pop('windowBounds')
            else: row['nativeViewport']['systemBars']['bottom'] = 500
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)

    def test_actual_host_qr_pin_manual_content_and_sync_error_are_required(self):
        for index, key, value in [(0, 'disabledMessage', False), (0, 'qrCount', 1), (2, 'qrCount', 0),
                                  (2, 'manualOpen', False), (2, 'hostPinPresent', False), (2, 'headings', ['Sync QR Code']),
                                  (8, 'qrCount', 0), (13, 'manualAction', False), (13, 'syncDisabled', True)]:
            data = evidence(); data['qrRecords'][index][key] = value
            with self.subTest(index=index, key=key), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)

    def test_every_terminal_and_manual_fallback_action_must_be_hittable(self):
        for index in (0, 3, 5, 6, 9, 11, 14, 16, 17):
            for key, value in [('hittable', False), ('caption', 'Other action'), ('aria', None)]:
                data = evidence(); data['qrRecords'][index]['action'][key] = value
                with self.subTest(index=index, key=key), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)

    def test_export_lifetime_real_close_and_home_state_cannot_be_skipped(self):
        for index, key, value in [(2, 'exportFileCount', 0), (7, 'exportFileCount', 1), (7, 'closed', False),
                                  (12, 'syncDisabled', True), (18, 'closed', False), (19, 'unlocked', False),
                                  (19, 'syncDisabled', False), (19, 'exportFileCount', 1)]:
            data = evidence(); data['qrRecords'][index][key] = value
            with self.subTest(index=index, key=key), self.assertRaises(RuntimeError): runner.verify_qr_evidence(data)

    def test_native_close_rejects_retry_wrong_source_unknown_tool_and_stale_coordinates(self):
        for key, value in [('injectedTapCount', 2), ('injectedSource', 8194), ('injectedToolType', 0),
                           ('frameCommittedAndGeometryStable', False), ('screenY', 500), ('cardPresentAfter', True),
                           ('beforeScreenshot', '../other.png'), ('preparationFrames', 4)]:
            data = evidence(); data['recoveryTapPlans'][0][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): runner.verify_qr_taps(data)
        for key, value in [('trusted', False), ('pointerType', ''), ('buttonLabel', 'Cancel'), ('clientY', 500), ('theme', 'dark')]:
            data = evidence(); data['recoveryPointerEvents'][0][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): runner.verify_qr_taps(data)

    def test_all_eight_touches_and_native_dimensions_are_independently_required(self):
        for mutation in ('missing', 'extra', 'swapped', 'nan', 'hidden', 'unrestored'):
            data = evidence()
            if mutation == 'missing': data['recoveryTapPlans'].pop()
            elif mutation == 'extra': data['recoveryPointerEvents'].append(copy.deepcopy(data['recoveryPointerEvents'][-1]))
            elif mutation == 'swapped': data['recoveryTapPlans'][0], data['recoveryTapPlans'][1] = data['recoveryTapPlans'][1], data['recoveryTapPlans'][0]
            elif mutation == 'nan': data['recoveryTapPlans'][0]['geometry']['width'] = float('nan')
            elif mutation == 'hidden': data['recoveryTapPlans'][0]['nativeViewport']['imeVisible'] = True
            else: data['recoveryTapPlans'][0]['expectedRestoredHeight'] += 100
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError): runner.verify_qr_taps(data)

    def test_qr_pass_never_bypasses_original_failure_or_permission_data_restore(self):
        good = dict(nativePassed=True, restored=True, blurRestored=True, qrCardsRequested=True,
                    qrCardsPassed=True, cameraPermissionRestored=True, nearbyPermissionRestored=True, qrTapCount=8)
        self.assertTrue(runner.verification_passed(good, False))
        for key in ('nativePassed', 'restored', 'blurRestored', 'qrCardsPassed', 'cameraPermissionRestored'):
            with self.subTest(key=key): self.assertFalse(runner.verification_passed(dict(good, **{key: False}), False))
        for count in (None, 0, 7, 9): self.assertFalse(runner.verification_passed(dict(good, qrTapCount=count), False))
        self.assertFalse(runner.verification_passed(dict(good, error='failed original baseline'), False))
        self.assertFalse(runner.verification_passed(dict(good, cameraRestoreError='not restored'), False))

if __name__ == '__main__': unittest.main()
