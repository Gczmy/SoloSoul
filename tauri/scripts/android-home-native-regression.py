#!/usr/bin/env python3
"""FE2 首页原生验收：专用模拟器、无既有账户、完整私有目录备份/恢复。

测试会通过生产 UI 创建公开临时账户。备份仅存本机结果目录，不上传。
恢复前必须 force-stop；恢复后核对全部文件内容、链接与权限。
"""
import argparse
import hashlib
import json
import math
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
COLD_METHOD = "coldAccountPreferencesSurviveProcessRestart"
NOTIFICATION = "android.permission.POST_NOTIFICATIONS"


def night_mode(text):
    match = re.fullmatch(r"Night mode: (auto|no|yes|custom)", text.strip())
    if not match:
        raise RuntimeError("Unrecognized system night mode; refusing mutation")
    return match[1]


def font_scale(text):
    value = text.strip()
    if value == 'null':
        return value
    if not re.fullmatch(r'(?:0|[1-9]\d*)(?:\.\d+)?', value) or not 0 < float(value) <= 10:
        raise RuntimeError('Unrecognized system font scale; refusing mutation')
    return value


def validate_previous_process(actual, expected):
    # instrumentation 完成后 Android 可能已结束目标进程。空结果是已退出，
    # 不能把 pidof 的退出 1 当成故障，也不能停止不属于第一阶段的其他进程。
    if actual.strip() not in ('', str(expected)):
        raise RuntimeError('Native process identity differs from first phase')
    return not actual.strip()


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


def verify_native(text, method=METHOD):
    text = text.replace("\r\n", "\n")
    statuses = [int(n) for n in re.findall(r"^INSTRUMENTATION_STATUS_CODE:\s*(-?\d+)\s*$", text, re.M)]
    if (statuses != [1, 0] or f"INSTRUMENTATION_STATUS: test={method}\n" not in text
            or not re.search(r"^INSTRUMENTATION_CODE:\s*-1\s*$", text, re.M)
            or "OK (1 test)" not in text
            or re.search(r"INSTRUMENTATION_FAILED|Process crashed|FAILURES!!!", text)):
        raise RuntimeError("Missing exact successful one-test native report; skips are failures")


def verification_passed(report, needs_notification):
    # 文件比较通过后，根权限核对或收尾仍可能失败，不能仅看部分成功标记。
    return bool(report.get("nativePassed") and report.get("restored") and report.get("blurRestored")
                and (not needs_notification or report.get("notificationPermissionRestored"))
                and (report.get("scenario") not in ("lifecycle", "cold") or report.get("nightModeRestored"))
                and (report.get("scenario") != "cold" or (report.get("coldPassed") and report.get("oldProcessStopped")))
                and (not report.get("motionPreferenceChecks") or report.get("motionPreferencePassed"))
                and (report.get('requestedFontScale') is None or report.get('fontScaleRestored'))
                and not any(report.get(key) for key in
                            ("error", "restoreError", "blurRestoreError", "notificationRestoreError", "nightModeRestoreError", "fontScaleRestoreError")))


def typography_clipped(sample):
    glyph = sample.get('glyph', {})
    clippers = sample.get('clippers')
    if (not isinstance(clippers, list) or not all(isinstance(glyph.get(key), (int, float))
            and math.isfinite(glyph[key]) for key in ('x', 'y', 'right', 'bottom', 'width', 'height'))
            or glyph['width'] <= 0 or glyph['height'] <= 0):
        raise RuntimeError('Missing actual typography clipping geometry')
    clipped = False
    for clip in clippers:
        box = clip.get('box', {})
        if (not all(isinstance(clip.get(axis), bool) for axis in ('clipX', 'clipY'))
                or not (clip['clipX'] or clip['clipY'])
                or not all(isinstance(box.get(key), (int, float)) and math.isfinite(box[key])
                           for key in ('x', 'y', 'right', 'bottom'))):
            raise RuntimeError('Missing actual typography clipping geometry')
        clipped |= ((clip['clipX'] and (glyph['x'] < box['x'] - 1 or glyph['right'] > box['right'] + 1))
                    or (clip['clipY'] and (glyph['y'] < box['y'] - 1 or glyph['bottom'] > box['bottom'] + 1)))
    return clipped


def verify_font_scale_pair(normal, enlarged):
    scale = enlarged.get('requestedFontScale')
    if normal.get('requestedFontScale') != 1.0 or scale not in (1.3, 2.0) or normal.get('runtime') != enlarged.get('runtime'):
        raise RuntimeError('Missing matched actual native font-scale pair')
    verify_evidence(normal, 'baseline', requested_font_scale=1.0)
    verify_evidence(enlarged, 'baseline', requested_font_scale=scale)
    originals = {row['stage']: row for row in normal['records'] if 'typography' in row}
    comparisons = []
    for row in enlarged['records']:
        if 'typography' not in row:
            continue
        previous = originals[row['stage']]['typography']
        for old, new in zip(previous['samples'], row['typography']['samples']):
            ratio = new['glyph']['height'] / old['glyph']['height']
            if old['text'] != new['text'] or ratio < scale * 0.9:
                raise RuntimeError('System font scale did not actually enlarge matched production text')
            comparisons.append({'stage': row['stage'], 'selector': new['selector'],
                                'normal_glyph_height': old['glyph']['height'],
                                'enlarged_glyph_height': new['glyph']['height'], 'height_ratio': ratio})
    if len(comparisons) != 12:
        raise RuntimeError('Missing all six production home typography comparisons')
    return comparisons


def verify_evidence(evidence, scenario, motion=False, *, require_preview_bars=False, require_preview_ink=False,
                    require_safe_notifications=False, requested_font_scale=None, require_typography_header=False,
                    require_keyboard_cancel=False):
    if motion and (scenario not in ("accounts", "cold") or evidence.get("motionPreferenceChecks") is not True):
        raise RuntimeError("Missing native reduced-motion preference checks")
    stages = [record["stage"] for record in evidence["records"]]
    expected = ["local-light", "enhanced-light", "enhanced-dark", "enhanced-dark-after-unlock", "encrypted-preferences",
                "object-editor-dark"]
    if scenario == "keyboard":
        expected.extend(["editor-keyboard-dark", "editor-keyboard-save-dark"])
        if require_keyboard_cancel:
            expected.append("editor-keyboard-cancel-dark")
        expected.append("editor-keyboard-after-back")
    expected.extend(["object-list-dark", "object-detail-dark"])
    if scenario == "overlays":
        expected.extend(["object-attachments-dark", "object-history-dark", "nested-overlays-after-system-back"])
    if scenario == "previews":
        expected.extend(["preview-fixtures-encrypted"] + [f"{stage}-dark" for stage in (
            "nonempty-attachments", "file-preview-text", "file-preview-image", "photo-album", "photo-viewer",
            "photo-album-after-back", "attachments-after-back", "preview-returned-detail")])
    expected.extend(["object-actions-dark", "object-list-after-system-back", "populated-enhanced-dark", "populated-enhanced-light"])
    if scenario == "previews":
        expected.extend([f"{stage}-light" for stage in (
            "nonempty-attachments", "file-preview-text", "file-preview-image", "photo-album", "photo-viewer",
            "photo-album-after-back", "attachments-after-back", "preview-returned-detail")])
    if scenario in ("accounts", "lifecycle"):
        expected.extend(['account-a-custom-dark', 'account-b-custom-light', 'account-a-after-switch',
                         'account-b-after-switch', 'account-a-after-second-switch'])
        if scenario == 'lifecycle':
            expected.extend(['account-a-system-light', 'account-a-after-activity-recreate',
                             'account-b-system-dark', 'account-b-after-activity-recreate'])
        expected.append('account-preferences-isolated')
    if scenario == "insets":
        expected.extend(['safe-home-portrait', 'safe-home-landscape', 'safe-home-after-rotation',
                         'safe-login-portrait', 'safe-login-keyboard', 'safe-login-after-keyboard-back', 'safe-login-after-document-reload'])
    if scenario == "rendering":
        expected.extend(['rendering-idle', 'rendering-away', 'rendering-return',
                         'rendering-return-ready', 'rendering-return-idle'])
    if scenario == 'cold':
        expected = ['cold-login', 'account-a-cold-system-light', 'account-a-cold-system-dark',
                    'account-b-cold-system-dark', 'account-preferences-isolated']
    if evidence.get('fontMenuBeforeDetail') is True:
        if scenario != 'baseline' or requested_font_scale is None:
            raise RuntimeError('Native font-menu order requires explicit font-scale verification')
        expected.remove('object-actions-dark')
        expected.insert(expected.index('object-detail-dark'), 'object-actions-dark')
    if scenario not in ("baseline", "overlays", "keyboard", "accounts", "insets", "lifecycle", "cold", "previews", "rendering") or evidence.get("scenario", "baseline") != scenario or stages != expected:
        raise RuntimeError(f"Incomplete production-home evidence ({scenario}): {stages}")
    if requested_font_scale is not None:
        if evidence.get('requestedFontScale') != requested_font_scale:
            raise RuntimeError('Missing requested native font-scale evidence')
        for row in evidence['records']:
            if row['stage'] not in ('local-light', 'enhanced-light', 'enhanced-dark', 'enhanced-dark-after-unlock',
                                    'populated-enhanced-dark', 'populated-enhanced-light'):
                continue
            typography = row.get('typography', {})
            samples = typography.get('samples', [])
            native_scale = typography.get('nativeFontScale')
            if (not isinstance(native_scale, (int, float)) or not math.isfinite(native_scale)
                    or abs(native_scale - requested_font_scale) > 0.001
                    or type(typography.get('textZoom')) is not int or typography['textZoom'] <= 0
                    or len(samples) != 2
                    or [sample.get('selector') for sample in samples] !=
                       ['.android-overview-copy > p:first-child', '.android-overview-copy strong']
                    or any(sample.get('found') is not True or sample.get('clipped') is not False
                           or sample.get('glyph', {}).get('height', 0) <= 0
                           or typography_clipped(sample)
                           or not sample.get('text') for sample in samples)):
                raise RuntimeError('Incomplete actual native typography evidence')
            if require_typography_header:
                header = typography.get('header', {})
                controls = header.get('controls', [])
                if (header.get('headingRight', math.inf) > header.get('actionsLeft', -math.inf) + 1
                        or [control.get('label') for control in controls] != ['Lock Vault', 'Account management', 'More actions']
                        or any(control.get('visible') is not True or control.get('hittable') is not True
                               or control.get('width', 0) < 48 or control.get('height', 0) < 48
                               or control.get('x', -1) < 0 or control.get('right', math.inf) > typography.get('viewportWidth', 0)
                               or control.get('svg', {}).get('width', 0) <= 0
                               or control.get('svg', {}).get('height', 0) <= 0
                               or control.get('inkPixels', 0) < 6 for control in controls)):
                    raise RuntimeError('Missing actual native enlarged-text header visibility')
        if require_typography_header:
            # 最终大字号矩阵同时覆盖真实提醒下的菜单；只有顶栏证据不能证明菜单可用。
            menu = next(row for row in evidence['records'] if row['stage'] == 'object-actions-dark')
            controls = menu.get('controls', [])
            if (menu.get('path') != '/workspace' or 'section=travel' not in menu.get('search', '')
                    or menu.get('visible') is not True or menu.get('settled') is not True
                    or menu.get('overflow') is not False or menu.get('toastCount') != 1
                    or any(menu.get(key) is not True for key in ('backupReminder', 'toastInFlow',
                                                               'toastInPanel', 'toastVisible', 'toastHittable'))
                    or [control.get('label') for control in controls] != ['Edit', 'History', 'Attachments', 'Delete']
                    or any(any(control.get(key) is not True for key in ('found', 'visible', 'hittable'))
                           or control.get('overlapped') is not False
                           or control.get('rect', {}).get('x', -1) < 0
                           or control.get('rect', {}).get('y', -1) < 0
                           or control.get('rect', {}).get('right', math.inf) > menu.get('viewportWidth', 0) + 1
                           or control.get('rect', {}).get('bottom', math.inf) > menu.get('viewportHeight', 0) + 1
                           for control in controls)):
                raise RuntimeError('Missing actual native enlarged-text menu/reminder visibility')
    if scenario == 'rendering':
        samples = {row['stage']: row for row in evidence['records']}
        for phase in ('idle', 'away', 'return', 'return-idle'):
            row = samples[f'rendering-{phase}']
            mounted = phase != 'away'
            draws = row.get('draws', [])
            if (row.get('hookInstalled') is not True or row.get('documentHidden') is not False
                    or row.get('phase') != phase or row.get('elapsedMs', 0) < 1000
                    or row.get('artworkMounted') is not mounted or row.get('path') != ('/' if mounted else '/tools')
                    or row.get('drawCalls') != len(draws)
                    or (mounted and (row.get('ready') is not True or row.get('canvasWidth', 0) <= 0 or row.get('canvasHeight', 0) <= 0))
                    or (phase == 'return' and not draws) or (phase != 'return' and draws)
                    or any(draw.get('phase') != phase or draw.get('connected') is not True
                           or not isinstance(draw.get('submitMs'), (int, float)) or not math.isfinite(draw['submitMs'])
                           or draw['submitMs'] < 0 or draw.get('width', 0) <= 0 or draw.get('height', 0) <= 0 for draw in draws)):
                raise RuntimeError(f'Incomplete or continuing WebGL submission evidence: {phase}')
        frame = samples['rendering-return-ready']
        if frame.get('ready') is not True or frame.get('copyInkPixels', 0) < 20 or frame.get('count') != '1':
            raise RuntimeError('Missing actual returned-home composed content frame')
    if scenario == "previews":
        fixtures = next(row for row in evidence['records'] if row['stage'] == 'preview-fixtures-encrypted')
        if fixtures.get('savedCount') != 2 or fixtures.get('encrypted') is not True:
            raise RuntimeError('Missing real encrypted preview fixtures')
        for row in evidence['records']:
            stage = row['stage']
            if stage.startswith(('nonempty-attachments-', 'file-preview-', 'photo-album-', 'photo-viewer-', 'attachments-after-back-')):
                dark = stage.endswith('-dark')
                if row.get('theme') != ('dark' if dark else 'light') or not row.get('visible') or not row.get('settled') or row.get('overflow'):
                    raise RuntimeError(f'Invalid native preview surface: {stage}')
                kind = row.get('kind')
                expected_kind = ('text' if stage.startswith('file-preview-text-') else 'image' if stage.startswith('file-preview-image-')
                                 else 'viewer' if stage.startswith('photo-viewer-') else 'album' if stage.startswith('photo-album-') else 'attachments')
                if kind != expected_kind:
                    raise RuntimeError(f'Wrong native preview kind: {stage}')
                if require_preview_bars:
                    bars = row.get('systemBars', {})
                    if (bars.get('statusBarLight') is not (not dark)
                            or bars.get('navigationBarLight') is not (not dark)):
                        raise RuntimeError(f'Wrong native preview system bar theme: {stage}')
                if require_preview_ink and kind != 'attachments':
                    foregrounds = row.get('headerForegrounds', [])
                    if len(foregrounds)<3 or any(item.get('contrast',0)<4.5 or item.get('inkPixels',0)<6 for item in foregrounds):
                        raise RuntimeError(f'Missing legible native preview header: {stage}')
                if kind in ('image','album','viewer') and row.get('imageDecoded') is not True:
                    raise RuntimeError(f'Missing decoded native preview: {stage}')
                if kind in ('image','viewer') and (row.get('imageNaturalWidth') != 320 or row.get('imageNaturalHeight') != 240):
                    raise RuntimeError('Wrong full preview dimensions')
                if kind == 'album' and (row.get('imageNaturalHeight',0)<=0 or abs(row.get('imageNaturalWidth',0)/row['imageNaturalHeight']-4/3)>.02):
                    raise RuntimeError('Wrong decoded thumbnail aspect')
                if kind == 'text' and ('FE2 public preview text' not in row.get('preText','') or row.get('textInkPixels',0)<20):
                    raise RuntimeError('Missing actual decrypted preview text')
                if kind in ('image','viewer'):
                    expected_pixels = ((228,61,48),(36,179,94),(40,107,224),(233,181,46))
                    pixels = row.get('quadrantPixels', [])
                    if len(pixels) != 4 or any(len(actual) != 3 or any(abs(a-b)>12 for a,b in zip(actual,target)) for actual,target in zip(pixels,expected_pixels)):
                        raise RuntimeError(f'Missing composed native image pixels: {stage}')
                if kind in ('image','viewer') and '%' not in row.get('zoomText',''):
                    raise RuntimeError('Missing native zoom toolbar')
                if kind == 'viewer' and row.get('counter','').strip() != '1 / 1':
                    raise RuntimeError('Missing actual album count')
                notification = 'saved' if kind == 'viewer' else 'backup' if dark and 'after-back' not in stage else 'none'
                if row.get('notificationKind') != notification or (notification != 'none' and (row.get('toastCount') != 1 or not row.get('notificationPresentAtCapture') or not row.get('savedNotification' if notification == 'saved' else 'backupReminder') or not all(row.get(k) for k in ('toastInPanel','toastInFlow','toastVisible','toastHittable')))):
                    raise RuntimeError(f'Missing actual foreground preview notification: {stage}')
                if row.get('toastCount', 0) > 0 and (row['toastCount'] != 1 or not all(row.get(k) for k in ('toastInPanel','toastInFlow','toastVisible','toastHittable'))):
                    raise RuntimeError(f'Invalid optional foreground preview notification: {stage}')
                controls = row.get('controls', [])
                if len(controls)<2 or any(not c.get('hittable') or c.get('overlapped') for c in controls):
                    raise RuntimeError(f'Missing native preview control hits: {stage}')
                native = row.get('nativeViewport', {})
                width = row.get('viewportWidth', 0)
                if width <= 0 or native.get('webViewWidthPx', 0) <= 0 or native.get('screenHeightPx', 0) <= 0:
                    raise RuntimeError(f'Missing actual native preview bounds: {stage}')
                scale = native['webViewWidthPx'] / width
                bars = native.get('systemBars', {})
                for control in controls:
                    rect = control.get('rect', {})
                    if (rect.get('x',-1)<0 or rect.get('right',float('inf'))>width+1
                            or native.get('webViewScreenY',0)+rect.get('y',-1)*scale<bars.get('top',0)-1
                            or native.get('webViewScreenY',0)+rect.get('bottom',float('inf'))*scale>native['screenHeightPx']-bars.get('bottom',0)+1):
                        raise RuntimeError(f'Preview control outside native safe bounds: {stage}')

            if stage.startswith('preview-returned-detail-'):
                if row.get('path') != '/workspace' or 'section=travel' not in row.get('search','') or not row.get('detail') or row.get('panel') or row.get('systemBars',{}).get('statusBarLight') is not stage.endswith('-light'):
                    raise RuntimeError(f'Native preview return/theme not restored: {stage}')
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
        if require_keyboard_cancel:
            cancel = samples['editor-keyboard-cancel-dark']
            native = cancel.get('nativeViewport', {})
            rect = cancel.get('rect', {})
            if (native.get('imeVisible') is not True or native.get('imeHeightPx', 0) <= 0
                    or cancel.get('path') != '/editor' or not cancel.get('hittable')
                    or not cancel.get('unobscured') or cancel.get('overlapped')
                    or rect.get('width', 0) < 48 or rect.get('height', 0) < 48
                    or cancel.get('controlBottomOnScreenPx', float('inf')) > cancel.get('keyboardTopOnScreenPx', 0) + 1):
                raise RuntimeError('Incomplete native keyboard/cancel evidence')
        if (after.get('draft') != before.get('draft', '') + ' XYZ'
                or after.get('draftBeforeBack') != after.get('draft')
                or before.get('historyIndex') != after.get('historyIndex')
                or after.get('historyBeforeBack') != after.get('historyIndex')
                or after.get('webViewHeightBeforeImePx') != after.get('nativeViewport', {}).get('webViewHeightPx')):
            raise RuntimeError('Keyboard dismissal changed draft or navigation history')
    if scenario == 'insets':
        samples = {record['stage']: record for record in evidence['records'] if record['stage'].startswith('safe-')}
        if require_safe_notifications:
            landscape = samples['safe-home-landscape']
            if (landscape.get('backupReminder') is not True
                    or sum(target.get('notification') is True for target in landscape.get('targets', [])) < 2):
                raise RuntimeError('Missing actual landscape notification safe-area evidence')
        for stage, record in samples.items():
            native = record.get('nativeViewport', {})
            bars, bounds = native.get('systemBars', {}), native.get('windowBounds', {})
            targets = record.get('targets', [])
            login = stage.startswith('safe-login-')
            ime = stage == 'safe-login-keyboard'
            if (not all(edge in bars and edge in bounds for edge in ('top', 'right', 'bottom', 'left'))
                    or len(targets) < (2 if login else 8) or native.get('imeVisible') is not ime
                    or (ime and native.get('imeHeightPx', 0) <= 0)
                    or record.get('path') != ('/login' if login else '/')
                    or not record.get('login' if login else 'home') or record.get('overflow')
                    or record.get('width', 0) <= 0 or native.get('webViewWidthPx', 0) <= 0):
                raise RuntimeError(f'Incomplete physical safe-area evidence: {stage}')
            scale = native['webViewWidthPx'] / record['width']
            limits = {'left': bounds['left'] + bars['left'], 'right': bounds['right'] - bars['right'],
                      'top': bounds['top'] + bars['top'],
                      'bottom': bounds['bottom'] - max(bars['bottom'], native.get('imeHeightPx', 0) if ime else 0)}
            for control in targets:
                rect = control.get('rect', {})
                if (not control.get('hittable') or not all(edge in rect for edge in ('x', 'y', 'right', 'bottom'))
                        or native['webViewScreenX'] + rect['x'] * scale < limits['left'] - 1
                        or native['webViewScreenX'] + rect['right'] * scale > limits['right'] + 1
                        or native['webViewScreenY'] + rect['y'] * scale < limits['top'] - 1
                        or native['webViewScreenY'] + rect['bottom'] * scale > limits['bottom'] + 1):
                    raise RuntimeError(f'Control intersects physical system bars/cutout/IME: {stage}')
        if (samples['safe-home-portrait']['width'] >= samples['safe-home-portrait']['height']
                or samples['safe-home-landscape']['width'] <= samples['safe-home-landscape']['height']
                or samples['safe-home-after-rotation']['width'] >= samples['safe-home-after-rotation']['height']
                or samples['safe-home-portrait']['nativeViewport']['systemBars']['top'] <= 0
                or samples['safe-login-portrait']['historyIndex'] != samples['safe-login-after-keyboard-back']['historyIndex']
                or samples['safe-login-portrait']['nativeViewport']['webViewHeightPx'] != samples['safe-login-after-document-reload']['nativeViewport']['webViewHeightPx']):
            raise RuntimeError('Missing actual rotation, keyboard-first Back or document-reload safe-area evidence')
    if scenario == 'cold':
        login = evidence['records'][0]
        pid = evidence.get('processId', 0)
        if (not isinstance(pid, int) or pid <= 0 or login.get('processId') != pid
                or login.get('previousProcessId', 0) <= 0 or login['previousProcessId'] == pid
                or login.get('path') != '/login' or login.get('home') or not login.get('locked')
                or not login.get('passwordRequired') or login.get('accountCount') != 2
                or login.get('theme') != 'dark' or login.get('glass') != 'enhanced'
                or login.get('background') != '#1a211d' or login.get('foreground') != '#d6ddd8'
                or login.get('accent') != '#112233' or (motion and login.get('reduceMotion') != 'true')):
            raise RuntimeError('Missing actual new-process locked startup')
    if scenario in ('accounts', 'lifecycle', 'cold'):
        account_ids = {}
        for record in evidence['records']:
            stage = record['stage']
            if not stage.startswith(('account-a-', 'account-b-')):
                continue
            dark = stage.startswith('account-a-')
            name = 'FE2 public visual test' if dark else 'FE2 public second account'
            surface = '#242d28' if dark else '#ffffff'
            prefs = {'theme': 'dark' if dark else 'light', 'accentColor': 'custom',
                     'customAccentHex': '#112233' if dark else '#ffee00',
                     'androidGlass': 'enhanced' if dark else 'local',
                     'defaultDarkTheme' if dark else 'defaultLightTheme': 'forest-night' if dark else 'clean-slate'}
            if motion:
                prefs['reduceMotion'] = dark
                if dark:
                    durations = record.get('motionEffect', {}).get('navigationTransition', '').split(',')
                    try:
                        reduced = all(float(v.strip()[:-2]) / 1000 <= 0.001 if v.strip().endswith('ms')
                                      else float(v.strip()[:-1]) <= 0.001 for v in durations)
                    except ValueError:
                        reduced = False
                    if not reduced:
                        raise RuntimeError('Missing actual reduced-motion CSS effect')
                if record.get('reduceMotion') != str(dark).lower():
                    raise RuntimeError('Missing actual account reduced-motion delivery')
            bars = record.get('systemBars', {})
            action = record.get('action') or {}
            if (record.get('accountName') != name or name not in record.get('name', '')
                    or not record.get('accountId') or record.get('savedPreferences') != prefs
                    or record.get('theme') != prefs['theme'] or record.get('glass') != prefs['androidGlass']
                    or record.get('accent') != prefs['customAccentHex'] or record.get('surface') != surface
                    or record.get('background') != ('#1a211d' if dark else '#f6f8fa')
                    or record.get('foreground') != ('#d6ddd8' if dark else '#1f2328')
                    or record.get('primaryInk') != ('#ffffff' if dark else '#000000')
                    or record.get('liquidBase') != surface or record.get('count') != ('1' if dark else '0')
                    or bars.get('statusBarLight') is not (not dark) or bars.get('navigationBarLight') is not (not dark)
                    or not record.get('home') or not record.get('copyVisible') or record.get('overflow')
                    or record.get('navCount') != 4 or record.get('copyInkPixels', 0) < 20
                    or action.get('contrast', 0) < 4.5 or action.get('inkPixels', 0) < 20
                    or not action.get('visible') or not action.get('hittable')
                    or (dark and not (record.get('ready') and record.get('canvasVisible')))
                    or (not dark and record.get('artRect') is not None)):
                raise RuntimeError(f'Incomplete native account theme/persistence evidence: {stage}')
            if name in account_ids and account_ids[name] != record['accountId']:
                raise RuntimeError('Account identity changed across UI switches')
            account_ids[name] = record['accountId']
            if scenario == 'cold':
                system_dark = stage != 'account-a-cold-system-light'
                if record.get('systemNightMode') is not system_dark or bars.get('nightMode') is not system_dark:
                    raise RuntimeError(f'Missing actual post-cold system mode: {stage}')
            if scenario == 'lifecycle' and stage in ('account-a-system-light', 'account-a-after-activity-recreate',
                                                    'account-b-system-dark', 'account-b-after-activity-recreate'):
                if (record.get('systemNightMode') is not (not dark) or bars.get('nightMode') is not (not dark)
                        or ('recreate' in stage and (not record.get('activityRecreated') or not record.get('webViewRecreated')))):
                    raise RuntimeError(f'Missing real system mode or Activity/WebView recreation: {stage}')
        summary = evidence['records'][-1]
        if (len(set(account_ids.values())) != 2 or summary.get('accountCount') != 2
                or not all(summary.get(k) for k in ('aRetainsObjectCount', 'bRemainsEmpty', 'savedPreferencesUnchanged'))):
            raise RuntimeError('Missing native account isolation evidence')
    return stages


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("adb", "serial", "avd", "apk", "test-apk", "output"):
        parser.add_argument(f"--{name}", required=True)
    parser.add_argument("--mode", choices=("supported", "fallback"), required=True)
    parser.add_argument("--scenario", choices=("baseline", "overlays", "keyboard", "accounts", "insets", "lifecycle", "cold", "previews", "rendering"), default="baseline")
    parser.add_argument("--motion-preferences", action="store_true", help="Verify true/false reduced motion across account/process changes (cold only)")
    parser.add_argument('--font-scale', type=float, choices=(1.0, 1.3, 2.0), help='Actual Android system font scale; baseline only, restored afterward')
    args = parser.parse_args()
    if args.motion_preferences and args.scenario != "cold":
        parser.error("Reduced-motion process matrix requires --scenario cold")
    if args.font_scale is not None and args.scenario != 'baseline':
        parser.error('Native font-scale observation requires --scenario baseline')
    if not re.fullmatch(r"emulator-\d+", args.serial) or not re.fullmatch(r"SoloSoul_[A-Za-z0-9_-]+", args.avd):
        parser.error("Requires explicitly named dedicated SoloSoul emulator")
    output = Path(args.output).resolve()
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    report = {"serial": args.serial, "avd": args.avd, "mode": args.mode, "scenario": args.scenario,
              "motionPreferenceChecks": args.motion_preferences, "startedAt": time.time(), "passed": False, "restored": False}
    report['requestedFontScale'] = args.font_scale
    selected = False
    backup_valid = False
    mutation_started = False
    old_blur = None
    old_notification = None
    old_night = None
    old_font_scale = None
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
        if args.font_scale is not None:
            old_font_scale = font_scale(read('shell', 'settings', 'get', 'system', 'font_scale'))
            report['fontScaleBefore'] = old_font_scale
        if args.scenario in ('lifecycle', 'cold'):
            old_night = night_mode(read('shell', 'cmd', 'uimode', 'night'))
            report['nightModeBefore'] = old_night
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
        if args.font_scale is not None:
            adb('shell', 'settings', 'put', 'system', 'font_scale', str(args.font_scale), stdout=subprocess.PIPE)
        adb("shell", "settings", "put", "global", "disable_window_blurs", "1" if args.mode == "fallback" else "0", stdout=subprocess.PIPE)
        mutation_started = True
        adb("install", "-r", str(Path(args.apk).resolve()), stdout=subprocess.PIPE)
        adb("install", "-r", str(Path(args.test_apk).resolve()), stdout=subprocess.PIPE)
        tag = f"fe2-home-{uuid.uuid4().hex}"
        # 输出由 JUnit 最后写入；不关闭最后一个 Activity 以免 Tauri 退出同进程 runner。
        result = adb("shell", "am", "instrument", "-w", "-r", "-e", "waitForActivitiesToComplete", "false",
                     "-e", "privateDataBackedUp", "true", "-e", "evidenceTag", tag,
                     "-e", "scenario", "accounts" if args.scenario == "cold" else args.scenario,
                     "-e", "prepareColdRestart", "true" if args.scenario == "cold" else "false",
                     "-e", "motionPreferenceChecks", str(args.motion_preferences).lower(),
                     *(("-e", "fontScale", str(args.font_scale)) if args.font_scale is not None else ()),
                     "-e", "class", f"com.solosoul.app.AndroidHomeInstrumentedTest#{METHOD}",
                     "com.solosoul.app.test/androidx.test.runner.AndroidJUnitRunner",
                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        text = result.stdout.decode()
        (output / "native.log").write_text(text)
        adb("pull", f"/sdcard/Android/data/{APP}/files/{tag}", str(output / "device-files"), stdout=subprocess.PIPE)
        verify_native(text)
        evidence = json.loads((output / "device-files/report.json").read_text())
        stages = verify_evidence(evidence, "accounts" if args.scenario == "cold" else args.scenario,
                                 args.motion_preferences, require_preview_bars=args.scenario == "previews",
                                 require_preview_ink=args.scenario == "previews",
                                 require_safe_notifications=args.scenario == "insets", requested_font_scale=args.font_scale,
                                 require_typography_header=args.font_scale is not None,
                                 require_keyboard_cancel=args.scenario == 'keyboard')
        report["stages"] = stages
        if args.scenario == 'cold':
            if not evidence['records'][-1].get('startupUiPrefsConfirmed'):
                raise RuntimeError('First phase did not confirm durable startup appearance')
            previous_pid = evidence['processId']
            if not isinstance(previous_pid, int) or previous_pid <= 0:
                raise RuntimeError('Missing first-phase native process identity')
            actual = read('shell', 'sh', '-c', f'pidof {APP} || true')
            report['oldProcessAlreadyExited'] = validate_previous_process(actual, previous_pid)
            ids = sorted({row['accountId'] for row in evidence['records'] if row['stage'].startswith(('account-a-', 'account-b-'))})
            if len(ids) != 2 or any(not re.fullmatch(r'acc_[a-f0-9]{16}', value) for value in ids):
                raise RuntimeError('Missing exactly two prepared synthetic accounts')
            adb('shell', 'cmd', 'uimode', 'night', 'no', stdout=subprocess.PIPE)
            adb('shell', 'am', 'force-stop', APP, stdout=subprocess.PIPE)
            if read('shell', 'sh', '-c', f'pidof {APP} || true'):
                raise RuntimeError('Old app process still alive; no cold-start acceptance')
            report['oldProcessStopped'] = True
            cold_tag = tag + '-cold'
            result = adb('shell', 'am', 'instrument', '-w', '-r', '-e', 'waitForActivitiesToComplete', 'false',
                         '-e', 'privateDataBackedUp', 'true', '-e', 'evidenceTag', cold_tag,
                         '-e', 'initialEvidenceTag', tag, '-e', 'scenario', 'cold',
                         '-e', 'motionPreferenceChecks', str(args.motion_preferences).lower(),
                         '-e', 'previousProcessId', str(previous_pid), '-e', 'expectedAccountIds', ','.join(ids),
                         '-e', 'class', f'com.solosoul.app.AndroidHomeInstrumentedTest#{COLD_METHOD}',
                         'com.solosoul.app.test/androidx.test.runner.AndroidJUnitRunner',
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            text = result.stdout.decode()
            (output / 'cold-native.log').write_text(text)
            adb('pull', f'/sdcard/Android/data/{APP}/files/{cold_tag}', str(output / 'cold-device-files'), stdout=subprocess.PIPE)
            verify_native(text, COLD_METHOD)
            cold = json.loads((output / 'cold-device-files/report.json').read_text())
            report['coldStages'] = verify_evidence(cold, 'cold', args.motion_preferences)
            if (cold['records'][0]['previousProcessId'] != previous_pid
                    or {row['accountId'] for row in cold['records'] if row['stage'].startswith(('account-a-', 'account-b-'))} != set(ids)):
                raise RuntimeError('Cold phase process or account identity differs from actual prepared phase')
            report['coldProcessAlreadyExited'] = validate_previous_process(
                read('shell', 'sh', '-c', f'pidof {APP} || true'), cold['processId'])
            report['previousProcessId'] = previous_pid
            report['coldProcessId'] = cold['processId']
            report['coldPassed'] = True
        if args.motion_preferences:
            report['motionPreferencePassed'] = True
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
            if old_night is not None:
                try:
                    adb('shell', 'cmd', 'uimode', 'night', old_night, stdout=subprocess.PIPE)
                    if night_mode(read('shell', 'cmd', 'uimode', 'night')) != old_night:
                        raise RuntimeError('System night mode differs from original state')
                    report['nightModeRestored'] = True
                except Exception as error:
                    report['nightModeRestoreError'] = str(error)
            if old_font_scale is not None:
                try:
                    command = ('delete',) if old_font_scale == 'null' else ('put',)
                    adb('shell', 'settings', *command, 'system', 'font_scale',
                        *(() if old_font_scale == 'null' else (old_font_scale,)), stdout=subprocess.PIPE)
                    if font_scale(read('shell', 'settings', 'get', 'system', 'font_scale')) != old_font_scale:
                        raise RuntimeError('System font scale differs from original state')
                    report['fontScaleRestored'] = True
                except Exception as error:
                    report['fontScaleRestoreError'] = str(error)
        report["passed"] = verification_passed(report, old_notification is not None)
        report["finishedAt"] = time.time()
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        cases = [(METHOD, bool(report.get('stages')) if args.scenario == 'cold' else report['passed'])]
        if args.scenario == 'cold':
            cases.append((COLD_METHOD, report['passed']))
        suite = ET.Element("testsuite", name="android-production-home", tests=str(len(cases)),
                           failures=str(sum(not passed for _, passed in cases)), skipped="0")
        for method, passed in cases:
            case = ET.SubElement(suite, "testcase", classname="AndroidHomeInstrumentedTest", name=method)
            if not passed:
                ET.SubElement(case, "failure", message=report.get("restoreError", report.get("error", "Incomplete evidence")))
        ET.ElementTree(suite).write(output / "junit.xml", encoding="utf-8", xml_declaration=True)
        print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
