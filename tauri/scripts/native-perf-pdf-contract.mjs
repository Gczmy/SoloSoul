/** RF-312：PDF能力诊断严格校验；永不输出PDF性能分布。 */
import { normalizeWindowsPath } from './native-perf-run.mjs';
export const PDF_UNMEASURED = Object.freeze([
  'PDF-performance',
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
const id = (v) => typeof v === 'string' && /^[A-Za-z0-9_.:-]{1,128}$/.test(v);
const trust = (v) =>
  exact(v, ['pointer', 'text', 'untrusted']) &&
  Object.values(v).every((n) => Number.isSafeInteger(n) && n >= 0 && n <= 128) &&
  v.untrusted === 0;
export function checkPdfDiagnostic(v, owned, bound, fixture) {
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
      'diagnosticOnly',
      'pdfDiagnostic',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'windows-native-sdk-pdf-diagnostic' ||
    v.diagnosticOnly !== true ||
    v.success !== true ||
    v.reason !== null ||
    v.failedStep !== null ||
    v.ipcAll !== null ||
    v.runId !== owned.runId ||
    normalizeWindowsPath(v.root) !== normalizeWindowsPath(owned.root) ||
    v.pid !== bound.pid ||
    v.port !== bound.port ||
    v.browserPid !== bound.browserPid ||
    v.objectCount !== fixture.objectCount ||
    v.timeOriginMs !== bound.timeOriginMs ||
    !exact(v.binding, Object.keys(bound.binding)) ||
    Object.keys(bound.binding).some((k) => v.binding[k] !== bound.binding[k]) ||
    v.inputMethod !== 'SDK-CDP-Input' ||
    !Number.isFinite(v.elapsedMs) ||
    v.elapsedMs < 0 ||
    v.elapsedMs > 310000 ||
    JSON.stringify(v.unmeasured) !== JSON.stringify(PDF_UNMEASURED) ||
    !Array.isArray(v.calls) ||
    v.calls.length > 128 ||
    v.calls.some(
      (m) =>
        ![
          'Runtime.evaluate',
          'Runtime.enable',
          'Runtime.disable',
          'Page.getFrameTree',
          'Page.captureScreenshot',
          'Input.dispatchMouseEvent',
          'Input.insertText',
        ].includes(m),
    ) ||
    v.calls.filter((m) => m === 'Input.insertText').length !== 1 ||
    v.calls.filter((m) => m === 'Page.captureScreenshot').length !== 1 ||
    !v.calls.includes('Runtime.enable') ||
    !v.calls.includes('Runtime.disable') ||
    !Array.isArray(v.phases) ||
    JSON.stringify(v.phases.map((p) => p.name)) !==
      JSON.stringify(['startup', 'password-unlock', 'workspace', 'attachment-list']) ||
    v.phases.some((p) => p.success !== true || !trust(p.inputTrust))
  )
    throw new Error('PDF diagnostic identity or exclusive scope rejected');
  const d = v.pdfDiagnostic;
  if (
    !exact(d, [
      'schemaVersion',
      'scope',
      'assetSha256',
      'renderVerified',
      'performanceMetrics',
      'frameSnapshots',
      'contexts',
      'readinessCandidates',
      'openedDom',
      'closedDom',
      'screenshot',
      'mainVerifiedAfterClose',
      'frameCreatedEvents',
      'contextWatchCleaned',
    ]) ||
    d.schemaVersion !== 1 ||
    d.scope !== 'windows-native-sdk-public-pdf-capability' ||
    d.assetSha256 !== 'ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe' ||
    d.renderVerified !== false ||
    d.performanceMetrics !== null ||
    d.mainVerifiedAfterClose !== true ||
    d.contextWatchCleaned !== true ||
    !Number.isInteger(d.frameCreatedEvents) ||
    d.frameCreatedEvents < 0 ||
    d.frameCreatedEvents > 4 ||
    !Array.isArray(d.frameSnapshots) ||
    d.frameSnapshots.length !== 4 ||
    !Array.isArray(d.contexts) ||
    d.contexts.length > 64 ||
    !Array.isArray(d.readinessCandidates) ||
    d.readinessCandidates.length > 16
  )
    throw new Error('PDF capability proof rejected or masquerades as performance');
  for (const snapshot of d.frameSnapshots) {
    if (
      !exact(snapshot, ['mainVerified', 'frames']) ||
      snapshot.mainVerified !== true ||
      !Array.isArray(snapshot.frames) ||
      snapshot.frames.length > 6
    )
      throw new Error('PDF main-frame proof rejected');
    const seen = new Set([bound.binding.mainFrameId]);
    for (const f of snapshot.frames) {
      if (
        !exact(f, ['id', 'parentId', 'loaderId', 'urlClass', 'originClass']) ||
        !id(f.id) ||
        !id(f.parentId) ||
        !id(f.loaderId) ||
        seen.has(f.id) ||
        !seen.has(f.parentId) ||
        !['application', 'owned-pdf', 'component-extension', 'blank', 'other'].includes(
          f.urlClass,
        ) ||
        !['application', 'owned-pdf', 'component-extension', 'other'].includes(f.originClass)
      )
        throw new Error('PDF child-frame topology rejected');
      seen.add(f.id);
    }
  }
  const contextValid = (c) =>
    exact(c, ['id', 'uniqueId', 'frameId', 'isDefault', 'originClass']) &&
    Number.isSafeInteger(c.id) &&
    c.id > 0 &&
    c.id <= 2147483647 &&
    id(c.uniqueId) &&
    id(c.frameId) &&
    typeof c.isDefault === 'boolean' &&
    ['application', 'owned-pdf', 'component-extension', 'other'].includes(c.originClass);
  if (d.contexts.some((c) => !contextValid(c))) throw new Error('PDF context proof rejected');
  const observed = Array.from({ length: 4 }, () => new Set());
  for (const candidate of d.readinessCandidates) {
    const s = candidate.state,
      c = candidate.context;
    if (
      !exact(candidate, ['snapshotIndex', 'context', 'state']) ||
      !Number.isInteger(candidate.snapshotIndex) ||
      candidate.snapshotIndex < 0 ||
      candidate.snapshotIndex > 3 ||
      !contextValid(c) ||
      c.isDefault !== true ||
      !d.contexts.some((x) => JSON.stringify(x) === JSON.stringify(c)) ||
      !d.frameSnapshots[candidate.snapshotIndex].frames.some(
        (f) =>
          f.id === c.frameId &&
          f.urlClass === c.originClass &&
          ['owned-pdf', 'component-extension'].includes(f.urlClass),
      ) ||
      !exact(s, [
        'schemaVersion',
        'scope',
        'documentMatches',
        'viewerPresent',
        'loadSucceededMethodPresent',
        'loadSucceeded',
        'documentDimensionsPresent',
        'pageCount',
        'paintedFrames',
        'timeOriginMs',
      ]) ||
      s.schemaVersion !== 1 ||
      s.scope !== 'windows-native-sdk-pdf-viewer-candidate' ||
      s.documentMatches !== true ||
      ['viewerPresent', 'loadSucceededMethodPresent', 'documentDimensionsPresent'].some(
        (k) => typeof s[k] !== 'boolean',
      ) ||
      !(s.loadSucceeded === null || typeof s.loadSucceeded === 'boolean') ||
      !(
        s.pageCount === null ||
        (Number.isInteger(s.pageCount) && s.pageCount >= 0 && s.pageCount <= 100)
      ) ||
      s.documentDimensionsPresent !== (s.pageCount !== null) ||
      ![0, 2].includes(s.paintedFrames) ||
      (s.paintedFrames === 2 && s.loadSucceeded !== true) ||
      !Number.isFinite(s.timeOriginMs) ||
      s.timeOriginMs <= 0
    )
      throw new Error('PDF renderer candidate proof rejected');
    const seen = observed[candidate.snapshotIndex];
    if (seen.has(c.uniqueId) || seen.size >= 4)
      throw new Error('PDF repeated context or snapshot budget rejected');
    seen.add(c.uniqueId);
  }
  for (const phase of ['opened', 'closed']) {
    const dom = d[phase + 'Dom'];
    if (
      !exact(dom, [
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
      dom.schemaVersion !== 1 ||
      dom.scope !== 'windows-native-sdk-pdf-dom' ||
      dom.phase !== phase ||
      dom.outcome !== 'ready' ||
      dom.mainVerified !== true ||
      dom.embedded !== (phase === 'opened') ||
      !Number.isSafeInteger(dom.frameElementCount) ||
      dom.frameElementCount < 0 ||
      dom.frameElementCount > 4 ||
      (phase === 'closed' && dom.frameElementCount !== 0) ||
      !Number.isFinite(dom.atMs) ||
      dom.atMs < 0 ||
      !trust(dom.inputTrust) ||
      dom.inputTrust.text !== 1 ||
      (phase === 'closed' && dom.target !== null)
    )
      throw new Error('PDF main-document structure or input rejected');
    if (
      phase === 'opened' &&
      (!exact(dom.target, ['x', 'y', 'actionable']) ||
        dom.target.actionable !== true ||
        ['x', 'y'].some(
          (k) => !Number.isFinite(dom.target[k]) || dom.target[k] < 0 || dom.target[k] > 16384,
        ))
    )
      throw new Error('PDF close target rejected');
  }
  const screenshot = d.screenshot;
  if (
    !exact(screenshot, ['fileName', 'bytes', 'sha256', 'scope']) ||
    screenshot.fileName !== 'native-perf-sdk-pdf-screenshot.png' ||
    !Number.isSafeInteger(screenshot.bytes) ||
    screenshot.bytes < 24 ||
    screenshot.bytes > 262144 ||
    typeof screenshot.sha256 !== 'string' ||
    !/^[a-f0-9]{64}$/.test(screenshot.sha256) ||
    screenshot.scope !== 'owned-public-fixture-WebView-pixels; excludes-DWM'
  )
    throw new Error('PDF screenshot proof rejected');
  return v;
}
