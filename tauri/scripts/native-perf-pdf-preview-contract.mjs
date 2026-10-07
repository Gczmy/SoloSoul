/** RF-312：仅固定公开 PDF 首屏的严格原生结果，耗时包含观测开销。 */
import { normalizeWindowsPath, validateObserver } from './native-perf-run.mjs';
import { PDF_ASSET_SHA, PDF_MASK_SHA, PDF_VIEWPORT, PDF_ROI } from './native-perf-pdf-pixels.mjs';
export const PDF_PREVIEW_UNMEASURED = Object.freeze([
  'PDF-other-pages',
  'PDF-general-documents',
  'OCR',
  'system-sleep',
  'other-platforms',
]);
const exact = (v, keys) =>
  v &&
  typeof v === 'object' &&
  !Array.isArray(v) &&
  Object.keys(v).length === keys.length &&
  keys.every((k) => Object.hasOwn(v, k));
const same = (a, b) => exact(a, Object.keys(b)) && Object.keys(b).every((k) => a[k] === b[k]);
const clock = (v) => Number.isFinite(v) && v >= 0;
const id = (v) => typeof v === 'string' && /^[A-Za-z0-9_.:-]{1,128}$/.test(v);
const trust = (v) =>
  exact(v, ['pointer', 'text', 'untrusted']) &&
  Object.values(v).every((n) => Number.isSafeInteger(n) && n >= 0 && n <= 128) &&
  v.untrusted === 0 &&
  v.text === 1;
function dom(v, phase) {
  if (
    !exact(v, [
      'schemaVersion',
      'scope',
      'phase',
      'outcome',
      'mainVerified',
      'embedded',
      'frameElementCount',
      'atMs',
      'target',
      'inputTrust',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'windows-native-sdk-pdf-dom' ||
    v.phase !== phase ||
    v.outcome !== 'ready' ||
    v.mainVerified !== true ||
    v.embedded !== (phase === 'opened') ||
    !Number.isInteger(v.frameElementCount) ||
    v.frameElementCount < 0 ||
    v.frameElementCount > 4 ||
    !clock(v.atMs) ||
    !trust(v.inputTrust)
  )
    throw new Error('PDF main DOM or input proof rejected');
  if (
    phase === 'closed'
      ? v.target !== null || v.frameElementCount !== 0
      : !exact(v.target, ['x', 'y', 'actionable']) ||
        v.target.actionable !== true ||
        !clock(v.target.x) ||
        !clock(v.target.y) ||
        v.target.x >= 1028 ||
        v.target.y >= 749
  )
    throw new Error('PDF visible close control or closed state rejected');
}
function tree(v, main) {
  if (
    !exact(v, ['mainVerified', 'frames']) ||
    v.mainVerified !== true ||
    !Array.isArray(v.frames) ||
    v.frames.length > 6
  )
    throw new Error('PDF frame proof rejected');
  const depths = new Map([[main, 0]]),
    seen = new Set([main]);
  for (const f of v.frames) {
    if (
      !exact(f, ['id', 'parentId', 'loaderId', 'urlClass', 'originClass']) ||
      ![f.id, f.parentId, f.loaderId].every(id) ||
      seen.has(f.id) ||
      !depths.has(f.parentId) ||
      depths.get(f.parentId) >= 4 ||
      !['application', 'owned-pdf', 'blank', 'component-extension', 'other'].includes(f.urlClass) ||
      !['application', 'owned-pdf', 'component-extension', 'other'].includes(f.originClass)
    )
      throw new Error('PDF frame identity or bounds rejected');
    seen.add(f.id);
    depths.set(f.id, depths.get(f.parentId) + 1);
  }
}
export function checkPdfFirstPage(v, owned, bound, fixture) {
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
      'pdfPreview',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'windows-native-sdk-pdf-first-page' ||
    v.success !== true ||
    v.reason !== null ||
    v.failedStep !== null ||
    v.ipcAll !== null ||
    v.runId !== owned.runId ||
    normalizeWindowsPath(v.root) !== normalizeWindowsPath(owned.root) ||
    v.pid !== bound.pid ||
    v.browserPid !== bound.browserPid ||
    v.port !== bound.port ||
    v.objectCount !== fixture.objectCount ||
    v.timeOriginMs !== bound.timeOriginMs ||
    !same(v.binding, bound.binding) ||
    v.inputMethod !== 'SDK-CDP-Input' ||
    !clock(v.elapsedMs) ||
    v.elapsedMs > 310000 ||
    JSON.stringify(v.unmeasured) !== JSON.stringify(PDF_PREVIEW_UNMEASURED) ||
    !Array.isArray(v.calls) ||
    v.calls.length > 128 ||
    v.calls.some(
      (m) =>
        ![
          'Runtime.evaluate',
          'Page.getFrameTree',
          'Page.captureScreenshot',
          'Input.dispatchMouseEvent',
          'Input.insertText',
        ].includes(m),
    ) ||
    v.calls.filter((m) => m === 'Input.insertText').length !== 1
  )
    throw new Error('PDF first-page identity or exclusive mode rejected');
  const names = ['startup', 'password-unlock', 'workspace', 'attachment-list'];
  if (
    !Array.isArray(v.phases) ||
    v.phases.length !== 4 ||
    v.phases.some(
      (p, i) =>
        !exact(p, ['name', 'success', 'durationMs', 'ipc', 'inputTrust', 'fromAtMs', 'toAtMs']) ||
        p.name !== names[i] ||
        p.success !== true ||
        !clock(p.durationMs) ||
        !clock(p.fromAtMs) ||
        !clock(p.toAtMs) ||
        p.toAtMs < p.fromAtMs ||
        !exact(p.inputTrust, ['pointer', 'text', 'untrusted']) ||
        p.inputTrust.untrusted !== 0 ||
        Object.values(p.inputTrust).some((n) => !Number.isSafeInteger(n) || n < 0 || n > 128) ||
        !exact(p.ipc, ['attempts', 'commands', 'reason']) ||
        p.ipc.reason !== null ||
        !Number.isSafeInteger(p.ipc.attempts) ||
        p.ipc.attempts < 0 ||
        !p.ipc.commands ||
        typeof p.ipc.commands !== 'object' ||
        Array.isArray(p.ipc.commands) ||
        Object.entries(p.ipc.commands).some(
          ([k, n]) => !/^[-A-Za-z0-9_:|]{1,128}$/.test(k) || !Number.isSafeInteger(n) || n <= 0,
        ) ||
        Object.values(p.ipc.commands).reduce((a, b) => a + b, 0) !== p.ipc.attempts,
    )
  )
    throw new Error('PDF prerequisite phases rejected');
  const last = v.lastProbe;
  if (
    !exact(last, [
      'schemaVersion',
      'scope',
      'runId',
      'timeOriginMs',
      'step',
      'href',
      'origin',
      'outcome',
      'rootPresent',
      'frameCount',
      'atMs',
      'target',
      'inputTrust',
      'observer',
    ]) ||
    last.schemaVersion !== 1 ||
    last.scope !== 'windows-native-sdk-ui-probe' ||
    last.runId !== owned.runId ||
    last.timeOriginMs !== bound.timeOriginMs ||
    last.step !== 'attachments' ||
    last.href !== 'http://tauri.localhost/settings/attachments' ||
    last.origin !== 'http://tauri.localhost' ||
    last.outcome !== 'ready' ||
    last.rootPresent !== true ||
    last.frameCount !== 0 ||
    last.target !== null ||
    !clock(last.atMs) ||
    !trust(last.inputTrust) ||
    !exact(last.observer, [
      'schemaVersion',
      'scope',
      'runId',
      'timeOriginMs',
      'installedAtMs',
      'valid',
      'invalidReasons',
      'maxEvents',
      'observedCount',
      'total',
      'commands',
    ]) ||
    last.observer.maxEvents !== 16384 ||
    !Array.isArray(last.observer.invalidReasons) ||
    last.observer.invalidReasons.length !== 0 ||
    !Array.isArray(last.observer.commands) ||
    last.observer.commands.some((c) => !exact(c, ['command', 'atMs'])) ||
    validateObserver(last.observer, owned.runId).valid !== true ||
    last.observer.timeOriginMs !== bound.timeOriginMs
  )
    throw new Error('PDF prerequisite document rejected');
  const d = v.pdfPreview;
  if (
    !exact(d, [
      'schemaVersion',
      'scope',
      'assetSha256',
      'resourceBinding',
      'renderVerified',
      'viewport',
      'roi',
      'referenceMaskSha256',
      'threshold',
      'maxCaptures',
      'intervalMs',
      'requiredConsecutiveMatches',
      'openingStartedAtMs',
      'samples',
      'performanceMetrics',
      'closedDom',
      'mainVerifiedAfterClose',
      'frameCreatedEvents',
    ]) ||
    d.schemaVersion !== 2 ||
    d.scope !== 'windows-native-sdk-public-pdf-first-page' ||
    d.assetSha256 !== PDF_ASSET_SHA ||
    d.renderVerified !== true ||
    !same(d.viewport, PDF_VIEWPORT) ||
    !same(d.roi, PDF_ROI) ||
    d.referenceMaskSha256 !== PDF_MASK_SHA ||
    !same(d.threshold, { alphaMin: 240, rgbMax: 64 }) ||
    d.maxCaptures !== 16 ||
    d.intervalMs !== 250 ||
    d.requiredConsecutiveMatches !== 2 ||
    !clock(d.openingStartedAtMs) ||
    d.mainVerifiedAfterClose !== true ||
    !Number.isInteger(d.frameCreatedEvents) ||
    d.frameCreatedEvents < 0 ||
    d.frameCreatedEvents > 4 ||
    !Array.isArray(d.samples) ||
    d.samples.length < 2 ||
    d.samples.length > 16 ||
    v.calls.filter((m) => m === 'Page.captureScreenshot').length !== d.samples.length
  )
    throw new Error('PDF first-page fixed contract rejected');
  const marker = owned.fixture?.marker,
    descriptor = marker?.attachments?.[1],
    files = owned.fixture?.files;
  const matched = Array.isArray(files)
    ? files.filter((f) => f.relativePath === descriptor?.relativePath)
    : [];
  if (
    marker?.schemaVersion !== 2 ||
    marker.scope !== 'synthetic-native-media-vault-fixture' ||
    marker.publicAssets?.[1]?.fileName !== 'text_only.pdf' ||
    marker.publicAssets[1].sha256 !== PDF_ASSET_SHA ||
    typeof descriptor?.id !== 'string' ||
    !/^att_[a-f0-9-]{36}$/.test(descriptor.id) ||
    descriptor.relativePath !== `attachments/obj_perf_00000000/${descriptor.id}/text_only.pdf` ||
    matched.length !== 1 ||
    !exact(d.resourceBinding, ['kind', 'bytes', 'sha256']) ||
    d.resourceBinding.kind !== 'prepared-vault-ciphertext' ||
    !Number.isSafeInteger(d.resourceBinding.bytes) ||
    d.resourceBinding.bytes < 3036 ||
    d.resourceBinding.bytes > 8192 ||
    !/^[a-f0-9]{64}$/.test(d.resourceBinding.sha256) ||
    d.resourceBinding.sha256 !== matched[0].sha256
  )
    throw new Error('PDF prepared ciphertext binding rejected');
  const tail = [
    'Runtime.evaluate',
    'Page.getFrameTree',
    'Input.dispatchMouseEvent',
    'Input.dispatchMouseEvent',
    ...d.samples.flatMap(() => [
      'Runtime.evaluate',
      'Page.getFrameTree',
      'Page.captureScreenshot',
      'Page.getFrameTree',
      'Runtime.evaluate',
    ]),
    'Runtime.evaluate',
    'Input.dispatchMouseEvent',
    'Input.dispatchMouseEvent',
    'Runtime.evaluate',
    'Page.getFrameTree',
    // SDK 行程完成后读取认证诊断，并再次验证同一主 frame / loader。
    'Runtime.evaluate',
    'Page.getFrameTree',
  ];
  if (JSON.stringify(v.calls.slice(-tail.length)) !== JSON.stringify(tail))
    throw new Error('PDF screenshot protocol guards or order rejected');
  let previous = d.openingStartedAtMs,
    stableIndex = -1;
  for (const [i, s] of d.samples.entries()) {
    if (
      !exact(s, [
        'index',
        'fileName',
        'bytes',
        'sha256',
        'captureStartedAtMs',
        'captureFinishedAtMs',
        'decodeFinishedAtMs',
        'verifiedAtMs',
        'width',
        'height',
        'darkPixels',
        'maskSha256',
        'matches',
        'frameStable',
        'frameBefore',
        'frameAfter',
        'domBefore',
        'domAfter',
      ]) ||
      s.index !== i ||
      s.fileName !== `native-perf-sdk-pdf-first-page-${String(i + 1).padStart(2, '0')}.png` ||
      !Number.isSafeInteger(s.bytes) ||
      s.bytes < 33 ||
      s.bytes > 256 * 1024 ||
      !/^[a-f0-9]{64}$/.test(s.sha256) ||
      s.width !== 1028 ||
      s.height !== 749 ||
      !Number.isInteger(s.darkPixels) ||
      s.darkPixels < 0 ||
      s.darkPixels > 430 * 36 ||
      !/^[a-f0-9]{64}$/.test(s.maskSha256) ||
      typeof s.matches !== 'boolean' ||
      s.matches !== (s.darkPixels === 530 && s.maskSha256 === PDF_MASK_SHA) ||
      ['captureStartedAtMs', 'captureFinishedAtMs', 'decodeFinishedAtMs', 'verifiedAtMs'].some(
        (k) => !clock(s[k]),
      ) ||
      s.captureStartedAtMs < previous + (i ? 250 : 0) ||
      s.captureFinishedAtMs < s.captureStartedAtMs ||
      s.decodeFinishedAtMs < s.captureFinishedAtMs ||
      s.verifiedAtMs < s.decodeFinishedAtMs ||
      typeof s.frameStable !== 'boolean' ||
      s.frameStable !== (JSON.stringify(s.frameBefore) === JSON.stringify(s.frameAfter))
    )
      throw new Error('PDF pixel observation rejected');
    tree(s.frameBefore, bound.binding.mainFrameId);
    tree(s.frameAfter, bound.binding.mainFrameId);
    dom(s.domBefore, 'opened');
    dom(s.domAfter, 'opened');
    if (
      i > 0 &&
      s.matches &&
      s.frameStable &&
      d.samples[i - 1].matches &&
      d.samples[i - 1].frameStable &&
      stableIndex < 0
    )
      stableIndex = i;
    previous = s.verifiedAtMs;
  }
  if (
    stableIndex !== d.samples.length - 1 ||
    previous > v.elapsedMs ||
    previous - d.openingStartedAtMs > 25000
  )
    throw new Error('PDF repeated visible observation rejected');
  dom(d.closedDom, 'closed');
  const m = d.performanceMetrics,
    first = d.samples.find((s) => s.matches && s.frameStable),
    sum = (end, start) => d.samples.reduce((n, s) => n + s[end] - s[start], 0),
    near = (a, b) => clock(a) && Math.abs(a - b) <= 0.000001;
  if (
    !exact(m, [
      'firstMatchObservedMs',
      'stableReadyObservedMs',
      'captureOverheadMs',
      'decodeOverheadMs',
      'pollWaitMs',
      'captures',
    ]) ||
    !near(m.firstMatchObservedMs, first.verifiedAtMs - d.openingStartedAtMs) ||
    !near(m.stableReadyObservedMs, previous - d.openingStartedAtMs) ||
    !near(m.captureOverheadMs, sum('captureFinishedAtMs', 'captureStartedAtMs')) ||
    !near(m.decodeOverheadMs, sum('decodeFinishedAtMs', 'captureFinishedAtMs')) ||
    !clock(m.pollWaitMs) ||
    m.pollWaitMs < 250 * (d.samples.length - 1) ||
    m.pollWaitMs > m.stableReadyObservedMs ||
    m.captures !== d.samples.length
  )
    throw new Error('PDF native clocks or sampling overhead rejected');
  return v;
}
export function summarizePdfFirstPage(samples, requested, integrity = true) {
  const complete =
    integrity &&
    samples.length === requested &&
    samples.every(
      (s) =>
        s.success &&
        s.cleanupIntegrity?.complete &&
        s.nativeProof?.pdfPreview?.renderVerified === true,
    );
  return [
    'firstMatchObservedMs',
    'stableReadyObservedMs',
    'captureOverheadMs',
    'decodeOverheadMs',
    'pollWaitMs',
  ].map((name) => {
    const values = complete
      ? samples.map((s) => s.nativeProof.pdfPreview.performanceMetrics[name]).sort((a, b) => a - b)
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
