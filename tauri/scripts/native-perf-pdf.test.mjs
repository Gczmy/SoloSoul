import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
import { checkPdfDiagnostic, PDF_UNMEASURED } from './native-perf-pdf-contract.mjs';
function inputs() {
  const owned = { runId: 'a'.repeat(32), root: 'C:\\owned' };
  const bound = {
    pid: 10,
    port: 9222,
    browserPid: 20,
    timeOriginMs: 100,
    binding: {
      source: 'http://tauri.localhost/login',
      mainFrameId: 'MAIN',
      loaderId: 'LOADER',
      timeOriginMs: null,
      navigationEvents: 0,
      frameCreatedEvents: 0,
    },
  };
  const trust = { pointer: 8, text: 1, untrusted: 0 };
  const dom = (phase) => ({
    schemaVersion: 1,
    scope: 'windows-native-sdk-pdf-dom',
    phase,
    outcome: 'ready',
    mainVerified: true,
    embedded: phase === 'opened',
    frameElementCount: 0,
    atMs: 10,
    target: phase === 'opened' ? { x: 10, y: 20, actionable: true } : null,
    inputTrust: { ...trust },
  });
  const proof = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-pdf-diagnostic',
    runId: owned.runId,
    root: owned.root,
    pid: 10,
    port: 9222,
    objectCount: 100,
    browserPid: 20,
    binding: { ...bound.binding },
    timeOriginMs: 100,
    inputMethod: 'SDK-CDP-Input',
    success: true,
    reason: null,
    failedStep: null,
    calls: [
      'Runtime.evaluate',
      'Page.getFrameTree',
      'Input.dispatchMouseEvent',
      'Input.insertText',
      'Runtime.enable',
      'Runtime.disable',
      'Page.captureScreenshot',
    ],
    phases: ['startup', 'password-unlock', 'workspace', 'attachment-list'].map((name) => ({
      name,
      success: true,
      inputTrust: { ...trust },
    })),
    lastProbe: {},
    ipcAll: null,
    elapsedMs: 10,
    unmeasured: [...PDF_UNMEASURED],
    diagnosticOnly: true,
    pdfDiagnostic: {
      schemaVersion: 1,
      scope: 'windows-native-sdk-public-pdf-capability',
      assetSha256: 'ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe',
      renderVerified: false,
      performanceMetrics: null,
      frameSnapshots: Array.from({ length: 4 }, () => ({
        mainVerified: true,
        frames: [
          {
            id: 'PDF',
            parentId: 'MAIN',
            loaderId: 'PDFLOADER',
            urlClass: 'owned-pdf',
            originClass: 'owned-pdf',
          },
        ],
      })),
      contexts: [],
      readinessCandidates: [],
      openedDom: dom('opened'),
      closedDom: dom('closed'),
      screenshot: {
        fileName: 'native-perf-sdk-pdf-screenshot.png',
        bytes: 100,
        sha256: 'b'.repeat(64),
        scope: 'owned-public-fixture-WebView-pixels; excludes-DWM',
      },
      mainVerifiedAfterClose: true,
      frameCreatedEvents: 1,
      contextWatchCleaned: true,
    },
  };
  return { owned, bound, proof, fixture: { objectCount: 100 } };
}
test('PDF diagnostic accepts capability capture while all performance remains null', () => {
  const i = inputs();
  assert.equal(checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture), i.proof);
  assert.equal(i.proof.pdfDiagnostic.renderVerified, false);
  assert.equal(i.proof.pdfDiagnostic.performanceMetrics, null);
});
test('PDF diagnostic rejects changed identity, widened scope, false rendering claims and cleanup failures', () => {
  for (const update of [
    (p) => p.pid++,
    (p) => (p.binding.loaderId = 'NEW'),
    (p) => (p.ipcAll = {}),
    (p) => (p.diagnosticOnly = false),
    (p) => (p.pdfDiagnostic.renderVerified = true),
    (p) => (p.pdfDiagnostic.performanceMetrics = { medianMs: 1 }),
    (p) => (p.pdfDiagnostic.contextWatchCleaned = false),
    (p) => (p.pdfDiagnostic.frameSnapshots[0].frames[0].parentId = 'OTHER'),
    (p) => (p.pdfDiagnostic.screenshot.fileName = '../secret.png'),
    (p) => (p.pdfDiagnostic.closedDom.inputTrust.untrusted = 1),
    (p) => (p.privateValue = 'SECRET'),
  ]) {
    const i = inputs();
    update(i.proof);
    assert.throws(() => checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture));
  }
});
test('PDF candidate context must bind an observed descendant frame and exact typed state', () => {
  const i = inputs(),
    context = {
      id: 2,
      uniqueId: 'UNIQUE',
      frameId: 'PDF',
      isDefault: true,
      originClass: 'owned-pdf',
    };
  i.proof.pdfDiagnostic.contexts = [context];
  i.proof.pdfDiagnostic.readinessCandidates = [
    {
      snapshotIndex: 0,
      context,
      state: {
        schemaVersion: 1,
        scope: 'windows-native-sdk-pdf-viewer-candidate',
        documentMatches: true,
        viewerPresent: true,
        loadSucceededMethodPresent: true,
        loadSucceeded: true,
        documentDimensionsPresent: true,
        pageCount: 1,
        paintedFrames: 2,
        timeOriginMs: 150,
      },
    },
  ];
  assert.equal(checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture), i.proof);
  const original = structuredClone(i.proof.pdfDiagnostic.readinessCandidates[0]);
  i.proof.pdfDiagnostic.readinessCandidates.push({ ...original, snapshotIndex: 1 });
  assert.equal(checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture), i.proof);
  i.proof.pdfDiagnostic.readinessCandidates.push({ ...original });
  assert.throws(() => checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture));
  i.proof.pdfDiagnostic.readinessCandidates.pop();
  i.proof.pdfDiagnostic.frameSnapshots[1].frames = [];
  assert.throws(() => checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture));
  i.proof.pdfDiagnostic.readinessCandidates.pop();
  i.proof.pdfDiagnostic.readinessCandidates[0].snapshotIndex = 4;
  assert.throws(() => checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture));
  i.proof.pdfDiagnostic.readinessCandidates[0] = {
    ...original,
    context: { ...context, frameId: 'UNKNOWN' },
  };
  assert.throws(() => checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture));
});
const viewerSource = await readFile(
  new URL('../src-tauri/src/native_perf/sdk_pdf_viewer.js', import.meta.url),
  'utf8',
);
async function viewer(
  view,
  url = 'chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html',
) {
  let frames = 0;
  const context = {
    location: { href: url },
    document: { querySelectorAll: () => (view ? [view] : []) },
    performance: { timeOrigin: 150 },
    requestAnimationFrame: (fn) => {
      frames++;
      fn();
    },
    setTimeout: () => 1,
    clearTimeout: () => {},
  };
  const result = await vm.runInNewContext(
    viewerSource.replace(
      '__REQUEST__',
      JSON.stringify({
        expectedUrl: 'chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html',
      }),
    ),
    context,
  );
  return { result: JSON.parse(JSON.stringify(result)), frames };
}
test('actual viewer expression distinguishes missing API and never waits a null loaded promise', async () => {
  const v = { loaded: null, documentDimensions: null };
  Object.defineProperty(v, 'loaded', {
    get() {
      throw new Error('must not use initial loaded promise');
    },
  });
  const { result, frames } = await viewer(v);
  assert.equal(result.viewerPresent, true);
  assert.equal(result.loadSucceededMethodPresent, false);
  assert.equal(result.loadSucceeded, null);
  assert.equal(result.pageCount, null);
  assert.equal(frames, 0);
});
test('actual viewer expression reads load state and two frames without input or business access', async () => {
  const v = {
    getLoadSucceededForTesting: () => true,
    documentDimensions: { pageDimensions: [{}] },
  };
  for (const k of ['click', 'scroll', 'invoke'])
    v[k] = () => {
      throw new Error('forbidden mutation');
    };
  const { result, frames } = await viewer(v);
  assert.equal(result.loadSucceeded, true);
  assert.equal(result.pageCount, 1);
  assert.equal(result.paintedFrames, 2);
  assert.equal(frames, 2);
});
test('actual viewer expression rejects replaced document and does not call its method', async () => {
  const { result } = await viewer(
    {
      getLoadSucceededForTesting: () => {
        throw new Error('must not inspect wrong document');
      },
    },
    'http://private.invalid/',
  );
  assert.equal(result.documentMatches, false);
  assert.equal(result.viewerPresent, false);
  assert.equal(result.loadSucceeded, null);
});
