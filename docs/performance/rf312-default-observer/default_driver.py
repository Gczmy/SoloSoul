#!/usr/bin/env python3
"""FE2 首页原生验收：专用模拟器、无既有账户、完整私有目录备份/恢复。

默认APK外部启动观测；使用固定公开账户，不安装目标插桩APK或注入JS。备份仅存本地。
恢复前必须 force-stop；恢复后核对全部文件内容、链接与权限。
"""
import argparse
from adb_control import AdbRecorder
import traceback
import importlib.util
import io
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
import shlex
import xml.etree.ElementTree as ET


from runtime_inputs import load_inputs, verify_sources
shared_restore=None

APP = "com.solosoul.app"
METHOD = "defaultExternalPublicUnlockObserverSmoke"
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


def activity_font_scale(text):
    lines = [line.strip() for line in text.splitlines()
             if line.strip().startswith('mGlobalConfiguration:')]
    match = re.fullmatch(r'mGlobalConfiguration:\s*\{(\S+)\s+.*\}', lines[0]) if len(lines) == 1 else None
    if match is None or match[1] == 'null':
        raise RuntimeError('Unrecognized global font configuration; refusing mutation')
    return float(font_scale(match[1]))


def effective_font_scale(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or not 0 < value <= 10:
        raise RuntimeError('Unrecognized effective font scale; refusing mutation')
    return float(value)


def font_deadline(timeout, clock):
    if isinstance(timeout, bool) or not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or not 0 < timeout <= 30:
        raise RuntimeError('Invalid font restoration budget')
    deadline = clock() + timeout

    def remaining():
        value = deadline - clock()
        if value <= 0:
            raise RuntimeError('System font configuration did not converge within restoration budget')
        return value

    return remaining


def capture_font_scale(read_raw, read_effective, *, timeout=10.0, clock=time.monotonic):
    remaining = font_deadline(timeout, clock)
    original = font_scale(read_raw(remaining()))
    effective = effective_font_scale(read_effective(remaining()))
    if (font_scale(read_raw(remaining())) != original or
            (original != 'null' and not math.isclose(float(original), effective, rel_tol=0, abs_tol=0.001))):
        raise RuntimeError('Original font setting/configuration changed; refusing mutation')
    return original, effective


def restore_font_scale(original, effective, write, read_raw, read_effective, *,
                       timeout=10.0, clock=time.monotonic, pause=time.sleep, trace=None):
    """Settle the effective configuration, then restore exact raw presence/bytes."""
    original = font_scale(original)
    effective = effective_font_scale(effective)
    if original != 'null' and not math.isclose(float(original), effective, rel_tol=0, abs_tol=0.001):
        raise RuntimeError('Original font setting/configuration differs; refusing mutation')
    remaining = font_deadline(timeout, clock)
    trace = {} if trace is None else trace
    trace.update(originalRawValue=original, originalEffectiveValue=effective,
                 budgetSeconds=timeout, configurationObservations=[], finalRawValues=[], verified=False)
    # Reset configuration first. Android may normalize/write back this value.
    write('put', str(effective), remaining())
    while True:
        actual = effective_font_scale(read_effective(remaining()))
        trace['configurationObservations'].append(actual)
        if math.isclose(actual, effective, rel_tol=0, abs_tol=0.001):
            break
        pause(min(0.05, remaining()))
    # The effective scale now stays unchanged, so the final raw write cannot
    # trigger the configuration-reset writeback that lost the absent key.
    write('delete' if original == 'null' else 'put', None if original == 'null' else original, remaining())
    actual_raw = font_scale(read_raw(remaining()))
    trace['finalRawValues'].append(actual_raw)
    if actual_raw != original:
        raise RuntimeError('System font scale differs from original state')
    final_effective = effective_font_scale(read_effective(remaining()))
    trace['finalEffectiveValue'] = final_effective
    if not math.isclose(final_effective, effective, rel_tol=0, abs_tol=0.001):
        raise RuntimeError('Effective system font scale differs from original state')
    actual_raw = font_scale(read_raw(remaining()))
    trace['finalRawValues'].append(actual_raw)
    if actual_raw != original:
        raise RuntimeError('System font scale changed after configuration readback')
    trace['verified'] = True


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
                or login.get('background') != '#1a211d' or login.get('foreground') != '#ffffff'
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
                    or record.get('foreground') != ('#ffffff' if dark else '#111111')
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



def verify_fixed_evidence(evidence,count,run_id,uid,picker_mode):
    names=['activity-startup','password-unlock','first-ocr-user-flow','workspace','search-needle','attachments-open','image-preview','text-preview','pdf-text-native-diagnostic','pdf-scanned-native-diagnostic','application-lock','password-reunlock']
    if (evidence.get('scope')!='android-native-fixed-media-ui-diagnostic' or evidence.get('success') is not True
            or evidence.get('objectCount')!=count or evidence.get('runId')!=run_id or evidence.get('uid')!=uid
            or not isinstance(evidence.get('processId'),int) or evidence['processId']<=0
            or evidence.get('memorySamplerExited') is not True or evidence.get('memoryError')):
        raise RuntimeError('Fixed native process/fixture/measurement outcome rejected')
    if evidence.get('measurementDiagnosticOnly') is not True or evidence.get('pdfRenderingProven') is not False or evidence.get('fullMediaOrPerformanceAccepted') is not False:
        raise RuntimeError('Diagnostic scope must not claim PDF rendering or performance')
    if not evidence.get('imagePreview',{}).get('imageLoaded') or not evidence.get('textPreview',{}).get('knownText'):
        raise RuntimeError('Actual public image/text contents unavailable')
    for name in ['text_only.pdf','scanned.pdf']:
        proof=evidence.get('nativePdf-'+name,{})
        if not (proof.get('knownPublicPlaintextVerified') is True and proof.get('closedToOriginalAttachmentList') is True and proof.get('sameUid') is True and proof.get('focused') is True and proof.get('activity')=='com.solosoul.app.PdfPreviewActivity' and proof.get('pageInfo')==('1 / 2' if name=='text_only.pdf' else '1 / 1') and proof.get('inkPixels',0)>100 and proof.get('windowPixels',{}).get('actualWindowPageWhitePixels',0)>100 and proof.get('windowPixels',{}).get('actualWindowPageDarkPixels',0)>100 and proof.get('bitmapWidth',0)>100 and proof.get('bitmapHeight',0)>100):
            raise RuntimeError('Actual native product PDF observation required: '+name)
    if evidence.get('nativePdf-text_only.pdf',{}).get('secondPage',{}).get('pageInfo')!='2 / 2':raise RuntimeError('Actual native second PDF page required')
    if evidence.get('textAfterCommit',{}).get('nearForegroundPixels',0)<=100 or evidence.get('textAfterCommit',{}).get('knownText') is not True:raise RuntimeError('Actual committed text window pixels required')
    if not all(evidence.get('firstOcr',{}).get(k) is True for k in ['hello','soloSoul','digits','nativeSystemPickerUsed','firstActualOcrTransport']):raise RuntimeError('Actual first public OCR corpus required')
    phases=evidence.get('phases',[])
    if [p.get('name') for p in phases]!=names:raise RuntimeError('Complete twelve-phase UI sequence required')
    action=evidence.get('nativePickerFileAction',{})
    expected_tool=1 if picker_mode=='finger' else 0
    expected_action='NATIVE_FINGER_TAP' if picker_mode=='finger' else 'NATIVE_UNKNOWN_POINTER_TAP'
    events=action.get('events',[])
    if (evidence.get('pickerPointerMode')!=picker_mode or action.get('action')!=expected_action
            or action.get('accepted') is not True or action.get('pointerMode')!=picker_mode
            or len(events)!=2 or [e.get('action') for e in events]!=[0,1]
            or any(e.get('kind')!='file' or e.get('injected') is not True or e.get('source')!=4098
                   or e.get('toolType')!=expected_tool or e.get('pointerId')!=0 or e.get('pointerCount')!=1
                   or e.get('pressure')!=1 or e.get('size')!=1 for e in events)
            or events[0].get('downTime')!=events[1].get('downTime')
            or events[1].get('eventTime',0)-events[0].get('eventTime',0)<80):
        raise RuntimeError('Actual exact-mode native file coordinate DOWN/UP required')
    visible=evidence.get('ocrWindowVisibility',{})
    if (visible.get('knownText') is not True or visible.get('nearForegroundPixels',0)<=100
            or visible.get('style',{}).get('visible') is not True
            or visible.get('screenshot')!='diagnostic-first-ocr-result-visible.png'):
        raise RuntimeError('Actual visible public OCR window pixels required')
    search=next(row for row in phases if row['name']=='search-needle').get('ui',{})
    if (search.get('path')!='/search' or search.get('publicSearch') is not True
            or search.get('searchResults')!=min(count//20,50) or search.get('errors')!=0
            or evidence.get('fixtureSearchMatches')!=count//20
            or evidence.get('displayedSearchResults')!=min(count//20,50)
            or evidence.get('productionSearchResultLimit')!=50):
        raise RuntimeError('Actual known public search results required')
    before=next(row for row in phases if row['name']=='password-unlock').get('observer',{}).get('commands',[])
    first=next(row for row in phases if row['name']=='first-ocr-user-flow').get('observer',{}).get('commands',[])
    if any(event.get('command')=='ocr_scan_image' for event in before) or sum(event.get('command')=='ocr_scan_image' for event in first)!=1:
        raise RuntimeError('Exactly one first public OCR transport required')
    previous=[];origin=None
    for row in phases:
        start,end=row.get('nativeStartedAtNs'),row.get('nativeFinishedAtNs')
        if (row.get('success') is not True or type(start) is not int or type(end) is not int or end<start
                or not math.isclose(row.get('durationMs',-1),(end-start)/1e6,rel_tol=0,abs_tol=0.000001)):
            raise RuntimeError('Native phase clock/duration rejected')
        obs=row.get('observer',{});events=obs.get('commands',[])
        if (obs.get('scope')!='android-native-ipc-transport-observer' or obs.get('valid') is not True
                or obs.get('runId')!=run_id or obs.get('invalidReasons')!=[]
                or obs.get('observedCount')!=len(events) or len(events)>obs.get('maxEvents',0)
                or events[:len(previous)]!=previous):raise RuntimeError('Early observer immutable prefix rejected')
        if origin is None:origin=obs.get('timeOriginMs')
        if obs.get('timeOriginMs')!=origin:raise RuntimeError('Document clock changed')
        at=obs.get('installedAtMs')
        for event in events:
            when=event.get('atMs')
            if (set(event)!={'command','transport','atMs'} or not re.fullmatch(r'[A-Za-z0-9_.:|/-]{1,128}',event.get('command',''))
                    or event.get('transport') not in ['android-java-bridge','android-channel-fetch']
                    or not isinstance(when,(int,float)) or isinstance(when,bool) or not math.isfinite(when)
                    or not at<=when<=obs.get('atMs',-1)):raise RuntimeError('Transport metadata/order rejected')
            at=when
        lo,hi=row.get('ipcStart'),row.get('ipcEnd')
        if (type(lo) is not int or type(hi) is not int or not 0<=lo<=hi==len(events)
                or row.get('transportAttempts')!=hi-lo):raise RuntimeError('Phase transport count rejected')
        previous=events
    if sum(e['command']=='ocr_scan_image' for e in previous)!=1:raise RuntimeError('Exactly one OCR transport in whole journey required')
    if sum(e['command']=='login' for e in previous)!=2:raise RuntimeError('Exactly two real public-password unlocks required')
    samples=evidence.get('memorySamples',[])
    if not samples:raise RuntimeError('Main-process PSS samples missing')
    prior=0
    for sample in samples:
        a,b=sample.get('startedAtNs'),sample.get('finishedAtNs');value=sample.get('pssBytes')
        if type(a) is not int or type(b) is not int or not prior<=a<=b or type(value) is not int or value<=0:
            raise RuntimeError('Nonoverlapping actual main-process PSS sample rejected')
        prior=b

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("adb", "serial", "avd", "apk", "test-apk", "output"):
        parser.add_argument(f"--{name}", required=True)
    parser.add_argument("--mode", choices=("supported", "fallback"), required=True)
    parser.add_argument("--picker-pointer-mode",choices=("finger","unknown"),default="finger")
    parser.add_argument("--scenario", choices=("baseline", "overlays", "keyboard", "accounts", "insets", "lifecycle", "cold", "previews", "rendering"), default="baseline")
    parser.add_argument("--motion-preferences", action="store_true", help="Verify true/false reduced motion across account/process changes (cold only)")
    parser.add_argument('--font-scale', type=float, choices=(1.0, 1.3, 2.0), help='Actual Android system font scale; baseline only, restored afterward')
    parser.add_argument('--restore-apk', required=True)
    parser.add_argument('--object-count',type=int,choices=(100,5000),required=True)
    parser.add_argument('--fixture',required=True)
    parser.add_argument("--inputs", required=True, type=Path)
    args = parser.parse_args()
    if args.scenario != 'baseline' or args.motion_preferences or args.font_scale is not None:
        parser.error('Fixed-Vault driver accepts baseline only; original restoration guards stay unchanged')
    global shared_restore
    input_bytes=args.inputs.read_bytes()
    frozen_inputs=load_inputs(args.inputs)
    if Path(args.adb).resolve()!=Path(frozen_inputs['host']['adbExecutable']).resolve() or args.serial!=frozen_inputs['host']['serial'] or args.avd!=frozen_inputs['host']['avd']:
        parser.error('Exact declared SDK, serial and AVD required')
    _shared_path=Path(frozen_inputs['host']['workspaceRoot'])/'tauri/scripts/android-home-native-regression.py'
    _shared_spec=importlib.util.spec_from_file_location('current_owned_restore',_shared_path)
    shared_restore=importlib.util.module_from_spec(_shared_spec)
    _shared_spec.loader.exec_module(shared_restore)
    if (Path(args.apk).resolve() != Path(frozen_inputs['release']['apk']['path']).resolve()
            or Path(args.restore_apk).resolve() != Path(frozen_inputs['originalDebug']['copy']).resolve()
            or Path(args.test_apk).resolve() != Path(frozen_inputs['runtimeTest']['path']).resolve()):
        parser.error('Release validation requires the exact frozen Release/restore/test APKs')
    fixture_spec=frozen_inputs['fixedFixtures'][str(args.object_count)]
    if Path(args.fixture).resolve()!=Path(fixture_spec['root']).resolve():
        parser.error('Only the exact frozen public fixture is allowed')
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
    report['runtimeInputsSha256']=hashlib.sha256(input_bytes).hexdigest()
    report['runtimeSourcesSha256']=frozen_inputs['fullJourneyFiles']
    selected = False
    backup_valid = False
    mutation_started = False
    old_blur = None
    old_notification = None
    old_night = None
    old_font_scale = None
    old_font_effective = None
    backup = output / "private-data-before.tar"
    before = None
    root_mode = None
    original_packages = None
    recorder = AdbRecorder(args.adb, args.serial, output)
    commands = recorder.commands
    public_image = None
    public_image_staged = False

    def adb(*command, **kwargs):
        return recorder.run(*command, **kwargs)

    def read(*command):
        return adb(*command, stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode().strip()

    def read_font_raw(remaining):
        return adb('shell', 'settings', 'get', 'system', 'font_scale', timeout=remaining,
                   stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode()

    def read_font_effective(remaining):
        return activity_font_scale(adb('shell', 'dumpsys', 'activity', 'processes', timeout=remaining,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode())

    def write_font(action, value, remaining):
        adb('shell', 'settings', action, 'system', 'font_scale',
            *(() if value is None else (value,)), timeout=remaining,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def archive(path):
        with open(path, "xb") as stream:
            os.chmod(path, 0o600)
            adb("exec-out", "run-as", APP, "tar", "-cf", "-", ".", stdout=stream, stderr=subprocess.PIPE)

    def frozen():
        root = Path(frozen_inputs['host']['workspaceRoot'])
        if args.inputs.read_bytes()!=input_bytes:raise RuntimeError('Declared runtime inputs changed')
        verify_sources(frozen_inputs)
        if any(digest(root/name) != value for name,value in frozen_inputs['runtimeSourceFiles'].items()):
            raise RuntimeError('Frozen Release runtime source changed')
        for row in (frozen_inputs['release']['apk'], frozen_inputs['originalDebug'], frozen_inputs['originalTest'], frozen_inputs['runtimeTest']):
            if digest(Path(row.get('path', row.get('copy')))) != row['sha256']:
                raise RuntimeError('Frozen APK changed')

    def installed(package, label):
        paths = read('shell', 'pm', 'path', package).splitlines()
        if (len(paths) != 1 or not re.fullmatch(r'package:/data/app/(?:[A-Za-z0-9_~+=.-]+/){1,2}base\.apk', paths[0])
                or any(part in ('.', '..') for part in paths[0].split('/'))):
            raise RuntimeError('Expected a single installed base APK')
        payload = adb('exec-out', 'cat', paths[0].removeprefix('package:'),
                      stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout
        dump = read('shell', 'dumpsys', 'package', package)
        (output/(label+'-package.log')).write_text(dump, encoding='utf-8')
        uid = re.findall(r'^\s*userId=(\d+)\s*$', dump, re.M)
        flags = re.findall(r'^\s*pkgFlags=\[([^\]]*)\]', dump, re.M)
        if len(uid) != 1 or len(flags) != 1:
            raise RuntimeError('Missing unambiguous package UID/debug flags')
        return {'sha256': hashlib.sha256(payload).hexdigest(), 'bytes': len(payload),
                'uid': int(uid[0]), 'debuggable': 'DEBUGGABLE' in flags[0].split()}

    try:
        frozen()
        devices = read("devices", "-l").splitlines()
        if not any(line.split()[:2] == [args.serial, "device"] for line in devices):
            raise RuntimeError("Dedicated emulator missing/offline/unauthorized")
        if read("emu", "avd", "name").splitlines()[0] != args.avd:
            raise RuntimeError("AVD identity mismatch")
        selected = True
        if read('shell','pm','list','packages','com.solosoul.app.releaseprobe'):
            raise RuntimeError('Dedicated Release probe package already exists; refusing replacement')
        original_packages = {'main': installed(APP, 'original-main'),
                             'test': installed(APP+'.test', 'original-test')}
        report['originalInstalledPackages'] = original_packages
        if (original_packages['main']['sha256'] != frozen_inputs['originalDebug']['sha256']
                or original_packages['test']['sha256'] != frozen_inputs['originalTest']['sha256']
                or not original_packages['main']['debuggable']):
            raise RuntimeError('Installed original packages differ; refusing Release replacement')
        if args.font_scale is not None:
            old_font_scale, old_font_effective = capture_font_scale(read_font_raw, read_font_effective)
            report['fontScaleBefore'] = old_font_scale
            report['fontScaleBeforeEffective'] = old_font_effective
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
        fixture=Path(fixture_spec['root'])
        actual_files={p.relative_to(fixture).as_posix():digest(p) for p in fixture.rglob('*') if p.is_file()}
        if actual_files!=fixture_spec['files']:
            raise RuntimeError('Public fixed fixture source changed')
        fixture_tar=output/'public-fixed-fixture.tar'
        with tarfile.open(fixture_tar,'w') as packed:
            for relative,expected in sorted(actual_files.items()):
                path=fixture/relative
                if path.is_symlink() or path.stat().st_file_attributes&0x400:
                    raise RuntimeError('Public fixture must be ordinary')
                data=path.read_bytes()
                if hashlib.sha256(data).hexdigest()!=expected:raise RuntimeError('Fixture bytes changed')
                member=tarfile.TarInfo(relative);member.size=len(data);member.mode=0o600
                packed.addfile(member,io.BytesIO(data))
        remote_fixture=f'/data/local/tmp/rf312-fixed-fixture-{uuid.uuid4().hex}.tar'
        mutation_started=True
        try:
            adb('push',str(fixture_tar),remote_fixture,stdout=subprocess.PIPE)
            adb('shell','chmod','644',remote_fixture,stdout=subprocess.PIPE)
            adb('shell','run-as',APP,'tar','-xf',remote_fixture,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            device_hashes=read('shell','run-as',APP,'sha256sum',*sorted(actual_files))
            observed={}
            for line in device_hashes.splitlines():
                match=re.fullmatch(r'([a-f0-9]{64})  (.+)',line)
                if not match:raise RuntimeError('Unrecognized transferred public file checksum')
                observed[match[2]]=match[1]
            if observed!=actual_files:raise RuntimeError('Transferred public fixture bytes differ')
            report['fixedFixture']={'objectCount':args.object_count,'all8TransferredFileHashesVerified':True,'source':fixture_spec}
        finally:
            adb('shell','rm','-f','--',remote_fixture,stdout=subprocess.PIPE)
        if {p.relative_to(fixture).as_posix():digest(p) for p in fixture.rglob('*') if p.is_file()}!=actual_files:
            raise RuntimeError('Source fixed fixture was modified')
        report["backup"] = {"path": str(backup), "sha256": digest(backup), "entries": len(before)}
        old_blur = read("shell", "settings", "get", "global", "disable_window_blurs")
        if args.font_scale is not None:
            adb('shell', 'settings', 'put', 'system', 'font_scale', str(args.font_scale), stdout=subprocess.PIPE)
        adb("shell", "settings", "put", "global", "disable_window_blurs", "1" if args.mode == "fallback" else "0", stdout=subprocess.PIPE)
        mutation_started = True
        adb("install", "-r", str(Path(args.apk).resolve()), stdout=subprocess.PIPE)
        if read('shell','pm','list','packages','com.solosoul.app.releaseprobe'):
            raise RuntimeError('Default external control must not install a target instrumentation APK')
        report['targetInstrumentationInjected'] = False
        current = installed(APP, 'release-main')
        report['releaseInstalledPackage'] = current
        if (current['sha256'] != frozen_inputs['release']['apk']['sha256'] or current['debuggable']
                or current['uid'] != original_packages['main']['uid']):
            raise RuntimeError('Exact non-debuggable Release package/UID verification failed')
        frozen()
        tag = f"rf312-media-{uuid.uuid4().hex}"
        image_name='rf312-ocr-'+tag.removeprefix('rf312-media-')+'.png'
        public_image='/sdcard/Download/'+image_name
        image=frozen_inputs['publicOcrImage']
        if digest(Path(image['path']))!=image['sha256']:raise RuntimeError('Public OCR PNG changed')
        adb('shell','sh','-c',f'"test ! -e {public_image}"',stdout=subprocess.PIPE)
        adb('push',image['path'],public_image,stdout=subprocess.PIPE)
        public_image_staged=True
        if read('shell','sha256sum',public_image).split()!=[image['sha256'],public_image]:raise RuntimeError('Staged OCR image bytes mismatch')
        adb('shell','am','broadcast','-a','android.intent.action.MEDIA_SCANNER_SCAN_FILE','-d','file://'+public_image,stdout=subprocess.PIPE)
        report['publicOcrInput']={'path':public_image,'fileName':image_name,'sha256':image['sha256'],'publicFixtureOnly':True}
        canonical_image=read('shell','readlink','-f',public_image)
        image_where=shlex.quote("_data='"+canonical_image+"'")
        media_state=read('shell','content','query','--uri','content://media/external/file','--where',image_where,'--projection','_id:_display_name:mime_type:date_modified:_size')
        (output/'owned-public-image-media-before.log').write_text(media_state+'\n',encoding='utf-8')


        external_path = Path(__file__).parent/'default_journey.py'
        if digest(external_path) != frozen_inputs['externalObserverSha256']:
            raise RuntimeError('Frozen external observer changed')
        external_spec = importlib.util.spec_from_file_location('default_external', external_path)
        external = importlib.util.module_from_spec(external_spec)
        external_spec.loader.exec_module(external)
        evidence = external.observe(adb, read, output, args.object_count, original_packages['main']['uid'], report['publicOcrInput']['fileName'], frozen_inputs)
        report['defaultExternalEvidence'] = evidence
        report['stages'] = evidence['stageNames']
        report['nativePassed'] = evidence['success'] is True

    except Exception as error:
        report["error"] = str(error)
        report["errorType"] = type(error).__name__
        (output/'driver-exception.traceback.log').write_text(traceback.format_exc(), encoding='utf-8')
        try:
            for label, command in [('window', ['shell','dumpsys','window','windows']),
                    ('power', ['shell','dumpsys','power']),
                    ('accessibility', ['shell','dumpsys','accessibility'])]:
                diagnostic = adb(*command,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                (output/('failure-'+label+'.stdout.log')).write_bytes(diagnostic.stdout)
                (output/('failure-'+label+'.stderr.log')).write_bytes(diagnostic.stderr)
            start_file = output/'default-external/observed-process-start.json'
            if start_file.exists():
                observed = json.loads(start_file.read_text(encoding='utf-8'))
                diagnostic = adb('shell','logcat','-d','--pid='+str(observed['pid']),
                    stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                (output/'failure-app-logcat.stdout.log').write_bytes(diagnostic.stdout)
                (output/'failure-app-logcat.stderr.log').write_bytes(diagnostic.stderr)
        except Exception as diagnostic_error:
            report['postFailureDiagnosticErrorType'] = type(diagnostic_error).__name__
    finally:
        if selected:
            try:
                adb("shell", "am", "force-stop", APP, stdout=subprocess.PIPE)
                if backup_valid and mutation_started:
                    # Release run-as is unavailable. Restore the original APK first,
                    # without clearing data; only then restore the full original TAR.
                    adb('install', '-r', str(Path(args.restore_apk).resolve()), stdout=subprocess.PIPE)
                    adb('shell', 'am', 'force-stop', APP, stdout=subprocess.PIPE)
                    restored_packages = {'main': installed(APP, 'restored-main'),
                                         'test': installed(APP+'.test', 'restored-test')}
                    report['restoredInstalledPackages'] = restored_packages
                    if restored_packages != original_packages:
                        raise RuntimeError('Original APK bytes, UID or debug flags were not restored')
                    report['originalApksRestored'] = True
                    probe_list = read('shell','pm','list','packages','com.solosoul.app.releaseprobe')
                    if probe_list:
                        if probe_list != 'package:com.solosoul.app.releaseprobe':
                            raise RuntimeError('Unexpected probe package identity; refusing uninstall')
                        probe = installed('com.solosoul.app.releaseprobe', 'probe-before-uninstall')
                        if probe['sha256'] != frozen_inputs['runtimeTest']['sha256']:
                            raise RuntimeError('Probe package bytes changed; refusing uninstall')
                        adb('uninstall','com.solosoul.app.releaseprobe',stdout=subprocess.PIPE)
                    if read('shell','pm','list','packages','com.solosoul.app.releaseprobe'):
                        raise RuntimeError('Dedicated probe package was not removed')
                    report['ownedProbeRemoved'] = True
                    shared_restore.restore_private_data(adb,read,archive,backup,before,root_mode,output,report)
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
                    report['fontScaleRestoreTrace'] = {}
                    restore_font_scale(old_font_scale, old_font_effective, write_font,
                                       read_font_raw, read_font_effective,
                                       trace=report['fontScaleRestoreTrace'])
                    report['fontScaleRestored'] = True
                except Exception as error:
                    report['fontScaleRestoreError'] = str(error)
        if public_image_staged:
            try:
                if not re.fullmatch(r'/sdcard/Download/rf312-ocr-[a-f0-9]{32}\.png',public_image):raise RuntimeError('Owned image path rejected')
                expected=frozen_inputs['publicOcrImage']['sha256']
                if read('shell','sha256sum',public_image).split()!=[expected,public_image]:raise RuntimeError('Owned public image changed; retain for inspection')
                canonical=read('shell','readlink','-f',public_image)
                if not canonical.endswith('/Download/'+Path(public_image).name):raise RuntimeError('Owned media path alias changed')
                where=shlex.quote("_data='"+canonical+"'")
                adb('shell','content','delete','--uri','content://media/external/file','--where',where,stdout=subprocess.PIPE)
                state=read('shell','sh','-c',f'"if test -e {public_image}; then echo present; else echo absent; fi"')
                if state=='present':
                    if read('shell','sha256sum',public_image).split()!=[expected,public_image]:raise RuntimeError('Remaining owned image changed')
                    adb('shell','rm',public_image,stdout=subprocess.PIPE)
                elif state!='absent':raise RuntimeError('Owned file existence could not be established')
                adb('shell','sh','-c',f'"test ! -e {public_image}"',stdout=subprocess.PIPE)
                remaining=read('shell','content','query','--uri','content://media/external/file','--where',where,'--projection','_id')
                if remaining not in ('No result found.',''):raise RuntimeError('Owned media index still exists: '+remaining)
                report['ownedPublicOcrInputRemoved']=True
            except Exception as error:
                report['publicOcrCleanupError']=str(error)
        try:
            frozen()
            report['frozenInputsUnchanged'] = True
        except Exception as error:
            report['sourceRestoreError'] = str(error)
        report['passed'] = bool(verification_passed(report, old_notification is not None)
                                and report.get('originalApksRestored')
                                and report.get('ownedProbeRemoved')
                                and report.get('frozenInputsUnchanged')
                                and not report.get('sourceRestoreError')
                                and report.get('ownedPublicOcrInputRemoved'))
        report["finishedAt"] = time.time()
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        cases = [(METHOD, bool(report.get('stages')) if args.scenario == 'cold' else report['passed'])]
        if args.scenario == 'cold':
            cases.append((COLD_METHOD, report['passed']))
        suite = ET.Element("testsuite", name="android-production-home", tests=str(len(cases)),
                           failures=str(sum(not passed for _, passed in cases)), skipped="0")
        for method, passed in cases:
            case = ET.SubElement(suite, "testcase", classname="DefaultExternalPublicUnlockObserverSmoke", name=method)
            if not passed:
                ET.SubElement(case, "failure", message=report.get("restoreError", report.get("error", "Incomplete evidence")))
        ET.ElementTree(suite).write(output / "junit.xml", encoding="utf-8", xml_declaration=True)
        print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
