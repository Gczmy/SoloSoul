/** RF-312：固定公开图片、真实原生选择器和首次 OCR 可见结果的严格证明。 */
import { lstat, realpath, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { normalizeWindowsPath, validateObserver } from './native-perf-run.mjs';
export const OCR_ASSET_SHA = '56a9d54d9b70a3ee1b22e0b08b755b6cfc15ee125df4b11d52f8523583dcacaa';
export const OCR_TEXT = 'helloppocrv6solosoulocr1234567890';
export const OCR_UNMEASURED = Object.freeze([
  'OCR-general-images',
  'OCR-PDF',
  'OCR-engine-stage-attribution',
  'system-sleep',
  'other-platforms',
]);
const exact = (v, keys) =>
  v &&
  typeof v === 'object' &&
  !Array.isArray(v) &&
  Object.keys(v).length === keys.length &&
  keys.every((k) => Object.hasOwn(v, k));
const clock = (v) => Number.isFinite(v) && v >= 0;
const handle = (v) => Number.isSafeInteger(v) && v > 0;
const equal = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const trust = (v) =>
  exact(v, ['pointer', 'text', 'untrusted']) &&
  Object.values(v).every((n) => Number.isSafeInteger(n) && n >= 0 && n <= 128) &&
  v.untrusted === 0 &&
  v.text === 1;
function result(v) {
  if (
    !exact(v, [
      'kind',
      'text',
      'normalizedText',
      'visible',
      'paintedFrames',
      'tier',
      'firstScanInvokeCount',
    ]) ||
    v.kind !== 'fixed-public-image' ||
    typeof v.text !== 'string' ||
    v.text.length > 256 ||
    !/^[\x00-\x7f]+$/.test(v.text) ||
    v.text.replace(/[^a-z0-9]/gi, '').toLowerCase() !== OCR_TEXT ||
    v.normalizedText !== OCR_TEXT ||
    v.visible !== true ||
    v.paintedFrames !== 2 ||
    v.tier !== 'small' ||
    v.firstScanInvokeCount !== 1
  )
    throw new Error('Fixed public OCR result rejected');
}
export function checkOcrPicker(v, pid) {
  if (
    !exact(v, [
      'schemaVersion',
      'scope',
      'pid',
      'mainHwnd',
      'dialogHwnd',
      'directOwnerVerified',
      'foregroundVerified',
      'controls',
      'filenameEditHwnd',
      'openButtonHwnd',
      'inputMethod',
      'inputPathVerified',
      'actionStartedAtMs',
      'openedObservedAtMs',
      'selectionSubmittedAtMs',
      'closedObservedAtMs',
      'closedVerified',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'owned-ocr-native-file-picker' ||
    v.pid !== pid ||
    ![v.mainHwnd, v.dialogHwnd, v.filenameEditHwnd, v.openButtonHwnd].every(handle) ||
    v.mainHwnd === v.dialogHwnd ||
    v.directOwnerVerified !== true ||
    v.foregroundVerified !== true ||
    v.inputMethod !== 'WM_SETTEXT+readback+BM_CLICK' ||
    v.inputPathVerified !== true ||
    v.closedVerified !== true ||
    !Array.isArray(v.controls) ||
    !v.controls.length ||
    v.controls.length > 128
  )
    throw new Error('Owned native OCR picker identity rejected');
  const times = [
    'actionStartedAtMs',
    'openedObservedAtMs',
    'selectionSubmittedAtMs',
    'closedObservedAtMs',
  ];
  if (
    times.some((k, i) => !clock(v[k]) || (i && v[k] < v[times[i - 1]])) ||
    v.openedObservedAtMs - v.actionStartedAtMs > 30000 ||
    v.closedObservedAtMs - v.selectionSubmittedAtMs > 6000
  )
    throw new Error('OCR native picker clocks rejected');
  const controls = new Map();
  for (const c of v.controls) {
    if (
      !exact(c, ['hwnd', 'parent', 'class', 'id', 'visible', 'enabled', 'password']) ||
      !handle(c.hwnd) ||
      !handle(c.parent) ||
      c.hwnd === v.dialogHwnd ||
      c.hwnd === v.mainHwnd ||
      controls.has(c.hwnd) ||
      typeof c.class !== 'string' ||
      c.class.length < 1 ||
      c.class.length > 63 ||
      !Number.isInteger(c.id) ||
      c.id < -1 ||
      c.id > 65535 ||
      ['visible', 'enabled', 'password'].some((k) => typeof c[k] !== 'boolean')
    )
      throw new Error('OCR picker control metadata rejected');
    controls.set(c.hwnd, c);
  }
  for (const c of controls.values()) {
    let parent = c.parent;
    const seen = new Set([c.hwnd]);
    let depth = 0;
    while (parent !== v.dialogHwnd) {
      if (seen.has(parent) || ++depth > 8 || !controls.has(parent))
        throw new Error('OCR picker control tree rejected');
      seen.add(parent);
      parent = controls.get(parent).parent;
    }
  }
  const combos = v.controls.filter(
    (c) => c.class === 'ComboBoxEx32' && c.id === 1148 && c.visible && c.enabled,
  );
  const buttons = v.controls.filter(
    (c) =>
      c.parent === v.dialogHwnd && c.class === 'Button' && c.id === 1 && c.visible && c.enabled,
  );
  const edits =
    combos.length === 1
      ? v.controls.filter((c) => {
          if (c.class !== 'Edit' || !c.visible || !c.enabled || c.password) return false;
          let parent = c.parent;
          for (let i = 0; i < 8; i++) {
            if (parent === combos[0].hwnd) return true;
            if (parent === v.dialogHwnd) return false;
            parent = controls.get(parent).parent;
          }
          return false;
        })
      : [];
  if (
    combos.length !== 1 ||
    buttons.length !== 1 ||
    edits.length !== 1 ||
    buttons[0].hwnd !== v.openButtonHwnd ||
    edits[0].hwnd !== v.filenameEditHwnd
  )
    throw new Error('OCR native filename/Open control selection rejected');
  return v;
}
export function checkOcrJourney(v, owned, bound, fixture) {
  if (
    !exact(v, [
      'schemaVersion',
      'scope',
      'runId',
      'root',
      'pid',
      'port',
      'objectCount',
      'browserPid',
      'binding',
      'timeOriginMs',
      'inputMethod',
      'success',
      'reason',
      'failedStep',
      'calls',
      'phases',
      'lastProbe',
      'ipcAll',
      'elapsedMs',
      'unmeasured',
      'ocr',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'windows-native-sdk-first-ocr' ||
    v.success !== true ||
    v.reason !== null ||
    v.failedStep !== null ||
    v.runId !== owned.runId ||
    normalizeWindowsPath(v.root) !== normalizeWindowsPath(owned.root) ||
    v.pid !== bound.pid ||
    v.browserPid !== bound.browserPid ||
    v.port !== bound.port ||
    v.objectCount !== fixture.objectCount ||
    v.timeOriginMs !== bound.timeOriginMs ||
    !equal(v.binding, bound.binding) ||
    v.inputMethod !== 'SDK-CDP-Input+owned-Win32-picker' ||
    !clock(v.elapsedMs) ||
    v.elapsedMs > 310000 ||
    !equal(v.unmeasured, OCR_UNMEASURED) ||
    !Array.isArray(v.calls) ||
    v.calls.length > 128 ||
    v.calls.some(
      (m) =>
        ![
          'Runtime.evaluate',
          'Page.getFrameTree',
          'Input.dispatchMouseEvent',
          'Input.insertText',
        ].includes(m),
    ) ||
    v.calls.filter((m) => m === 'Input.insertText').length !== 1
  )
    throw new Error('OCR exclusive native journey identity rejected');
  const ipc = (x) =>
    exact(x, ['attempts', 'commands', 'reason']) &&
    x.reason === null &&
    Number.isSafeInteger(x.attempts) &&
    x.attempts >= 0 &&
    x.commands &&
    typeof x.commands === 'object' &&
    !Array.isArray(x.commands) &&
    Object.entries(x.commands).every(
      ([k, n]) => /^[-A-Za-z0-9_:|]{1,128}$/.test(k) && Number.isSafeInteger(n) && n > 0,
    ) &&
    Object.values(x.commands).reduce((a, b) => a + b, 0) === x.attempts;
  const names = ['startup', 'password-unlock', 'workspace', 'ocr-page', 'first-ocr'];
  if (
    !Array.isArray(v.phases) ||
    v.phases.length !== names.length ||
    v.phases.some(
      (p, i) =>
        !exact(
          p,
          i === 4
            ? [
                'name',
                'success',
                'durationMs',
                'ipc',
                'inputTrust',
                'fromAtMs',
                'toAtMs',
                'ocrEndState',
              ]
            : ['name', 'success', 'durationMs', 'ipc', 'inputTrust', 'fromAtMs', 'toAtMs'],
        ) ||
        p.name !== names[i] ||
        p.success !== true ||
        !clock(p.durationMs) ||
        !clock(p.fromAtMs) ||
        !clock(p.toAtMs) ||
        p.toAtMs < p.fromAtMs ||
        !ipc(p.ipc) ||
        !exact(p.inputTrust, ['pointer', 'text', 'untrusted']) ||
        p.inputTrust.untrusted !== 0 ||
        Object.values(p.inputTrust).some((n) => !Number.isSafeInteger(n) || n < 0 || n > 128),
    )
  )
    throw new Error('OCR phases rejected');
  const last = v.lastProbe;
  if (
    !exact(last, [
      'schemaVersion',
      'scope',
      'runId',
      'step',
      'outcome',
      'href',
      'origin',
      'timeOriginMs',
      'atMs',
      'rootPresent',
      'frameCount',
      'target',
      'inputTrust',
      'observer',
      'ocr',
    ]) ||
    last.schemaVersion !== 1 ||
    last.scope !== 'windows-native-sdk-ui-probe' ||
    last.runId !== owned.runId ||
    last.step !== 'ocrResultReady' ||
    last.outcome !== 'ready' ||
    last.href !== 'http://tauri.localhost/ocr' ||
    last.origin !== 'http://tauri.localhost' ||
    last.timeOriginMs !== bound.timeOriginMs ||
    !clock(last.atMs) ||
    last.rootPresent !== true ||
    last.frameCount !== 0 ||
    last.target !== null ||
    !trust(last.inputTrust) ||
    last.inputTrust.pointer < 7 ||
    !exact(last.observer, [
      'schemaVersion',
      'scope',
      'runId',
      'valid',
      'total',
      'observedCount',
      'invalidReasons',
      'installedAtMs',
      'timeOriginMs',
      'maxEvents',
      'commands',
    ]) ||
    last.observer.maxEvents !== 16384 ||
    validateObserver(last.observer, owned.runId).valid !== true ||
    last.observer.timeOriginMs !== bound.timeOriginMs ||
    last.observer.commands.some((c) => !exact(c, ['command', 'atMs']) || c.atMs > last.atMs) ||
    !ipc(v.ipcAll) ||
    v.ipcAll.commands.ocr_scan_image !== 1
  )
    throw new Error('OCR result document or complete IPC observer rejected');
  const counts = {};
  for (const event of last.observer.commands)
    counts[event.command] = (counts[event.command] ?? 0) + 1;
  if (
    v.ipcAll.attempts !== last.observer.commands.length ||
    Object.keys(counts).length !== Object.keys(v.ipcAll.commands).length ||
    Object.entries(counts).some(([k, n]) => v.ipcAll.commands[k] !== n)
  )
    throw new Error('OCR IPC totals differ');
  const d = v.ocr;
  if (
    !exact(d, [
      'schemaVersion',
      'scope',
      'resource',
      'firstPerProcess',
      'picker',
      'result',
      'resultObservedAtMs',
      'performanceMetrics',
      'closedPickerVerified',
    ]) ||
    d.schemaVersion !== 1 ||
    d.scope !== 'windows-native-sdk-public-first-ocr' ||
    d.firstPerProcess !== true ||
    d.closedPickerVerified !== true ||
    !clock(d.resultObservedAtMs) ||
    d.resultObservedAtMs > v.elapsedMs ||
    !exact(d.resource, [
      'relativePath',
      'bytes',
      'sha256',
      'width',
      'height',
      'tier',
      'preferencesAbsent',
    ]) ||
    d.resource.relativePath !== 'profile/Documents/ocr_test.png' ||
    d.resource.bytes !== 6274 ||
    d.resource.sha256 !== OCR_ASSET_SHA ||
    d.resource.width !== 500 ||
    d.resource.height !== 200 ||
    d.resource.tier !== 'small' ||
    d.resource.preferencesAbsent !== true
  )
    throw new Error('Fixed OCR resource rejected');
  const assets = owned.fixture?.marker?.publicAssets;
  if (
    !Array.isArray(assets) ||
    assets.filter((a) => a.fileName === 'ocr_test.png' && a.sha256 === OCR_ASSET_SHA).length !== 1
  )
    throw new Error('OCR public source fixture binding rejected');
  checkOcrPicker(d.picker, bound.pid);
  result(d.result);
  result(last.ocr);
  result(v.phases[4].ocrEndState);
  if (
    !equal(d.result, last.ocr) ||
    !equal(d.result, v.phases[4].ocrEndState) ||
    d.resultObservedAtMs < d.picker.closedObservedAtMs
  )
    throw new Error('OCR result clocks or cross-proof data differ');
  const metrics = d.performanceMetrics,
    p = d.picker,
    expected = {
      pickerOpenedObservedMs: p.openedObservedAtMs - p.actionStartedAtMs,
      pickerSelectionClosedMs: p.closedObservedAtMs - p.openedObservedAtMs,
      firstOcrResultObservedMs: d.resultObservedAtMs - p.selectionSubmittedAtMs,
      totalOcrObservedMs: d.resultObservedAtMs - p.actionStartedAtMs,
    };
  if (
    !exact(metrics, Object.keys(expected)) ||
    Object.entries(expected).some(
      ([k, n]) => !clock(metrics[k]) || Math.abs(metrics[k] - n) > 0.000001,
    )
  )
    throw new Error('OCR observed timing boundaries differ');
  return v;
}
export function summarizeOcrJourneys(samples, requested, integrity = true) {
  const complete =
    integrity &&
    samples.length === requested &&
    samples.every(
      (s) =>
        s.success &&
        s.cleanupIntegrity?.complete &&
        s.nativeProof?.ocr?.closedPickerVerified === true,
    );
  return [
    'pickerOpenedObservedMs',
    'pickerSelectionClosedMs',
    'firstOcrResultObservedMs',
    'totalOcrObservedMs',
  ].map((name) => {
    const values = complete
      ? samples.map((s) => s.nativeProof.ocr.performanceMetrics[name]).sort((a, b) => a - b)
      : [];
    const valid = values.length === requested && values.every(clock);
    return {
      name,
      requestedSamples: requested,
      successfulSamples: valid ? values.length : 0,
      medianMs: valid
        ? (values[Math.floor((values.length - 1) / 2)] +
            values[Math.ceil((values.length - 1) / 2)]) /
          2
        : null,
      p95Ms: valid ? values[Math.ceil(values.length * 0.95) - 1] : null,
    };
  });
}

export async function verifyPublicOcrInput(root, preserve = false) {
  const file = path.join(root, 'profile', 'Documents', 'ocr_test.png'),
    stat = await lstat(file);
  if (
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    stat.size !== 6274 ||
    normalizeWindowsPath(await realpath(file)) !== normalizeWindowsPath(file)
  )
    throw new Error('OCR selected public image path differs');
  const bytes = await readFile(file);
  if (bytes.length !== 6274 || createHash('sha256').update(bytes).digest('hex') !== OCR_ASSET_SHA)
    throw new Error('OCR selected public image bytes differ');
  if (preserve)
    await writeFile(path.join(root, 'native-perf-ocr-input.png'), bytes, { flag: 'wx' });
  return { relativePath: 'profile/Documents/ocr_test.png', bytes: 6274, sha256: OCR_ASSET_SHA };
}
