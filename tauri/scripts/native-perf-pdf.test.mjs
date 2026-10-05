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

function targetInputs(withSession = true) {
  const i = inputs();
  const target = (id, type, urlClass) => ({
    id,
    type,
    urlClass,
    attached: true,
    parentId: null,
    openerId: null,
    parentFrameId: null,
    browserContextId: null,
  });
  const main = target('APP', 'page', 'application'),
    pdf = target('DOC', 'iframe', 'owned-pdf');
  const state = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-pdf-viewer-candidate',
    documentMatches: true,
    viewerPresent: true,
    loadSucceededMethodPresent: true,
    loadSucceeded: true,
    documentDimensionsPresent: true,
    pageCount: 2,
    paintedFrames: 2,
    timeOriginMs: 150,
  };
  i.proof.pdfDiagnostic.schemaVersion = 2;
  i.proof.pdfDiagnostic.targetDiagnostic = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-pdf-related-targets',
    mainTarget: main,
    snapshots: Array.from({ length: 4 }, (_, snapshotIndex) => ({
      snapshotIndex,
      mainVerified: true,
      targets: withSession ? [main, pdf] : [main],
    })),
    relatedSessions: withSession
      ? [{ sessionId: 'SESSION', parentSessionId: '', target: pdf, active: false, enabled: true }]
      : [],
    candidates: withSession
      ? [
          {
            snapshotIndex: 0,
            sessionId: 'SESSION',
            targetId: 'DOC',
            frame: { id: 'F', loaderId: 'L', urlClass: 'owned-pdf' },
            state,
          },
        ]
      : [],
    sessionCalls: withSession
      ? [
          'Runtime.enable',
          'Target.getTargetInfo',
          'Page.getFrameTree',
          'Runtime.evaluate',
          'Page.getFrameTree',
        ].map((method) => ({ sessionId: 'SESSION', method }))
      : [],
    eventCount: withSession ? 2 : 0,
    autoAttachCleaned: true,
    watcherCleaned: true,
  };
  i.proof.calls.push(
    'Target.getTargetInfo',
    'Target.setAutoAttach',
    ...Array.from({ length: 4 }, () => ['Target.getTargetInfo', 'Target.getTargets']).flat(),
    ...i.proof.pdfDiagnostic.targetDiagnostic.sessionCalls.map((c) => c.method),
    'Target.setAutoAttach',
  );
  return i;
}
test('PDF v2 accepts bounded direct sessions or empty topology without claiming rendering', () => {
  for (const withSession of [true, false]) {
    const i = targetInputs(withSession);
    assert.equal(checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture), i.proof);
    assert.equal(i.proof.pdfDiagnostic.renderVerified, false);
  }
});
test('PDF v2 refuses foreign, repeated, active or unlisted session identities', () => {
  for (const update of [
    (d) => (d.relatedSessions[0].parentSessionId = 'FOREIGN'),
    (d) => d.relatedSessions.push(structuredClone(d.relatedSessions[0])),
    (d) => (d.relatedSessions[0].active = true),
    (d) => (d.relatedSessions[0].target.id = 'APP'),
    (d) => (d.candidates[0].sessionId = 'FOREIGN'),
    (d) => (d.candidates[0].targetId = 'FOREIGN'),
    (d) => d.snapshots[0].targets.pop(),
    (d) => (d.snapshots[0].targets[1].urlClass = 'other'),
    (d) => d.candidates.push(structuredClone(d.candidates[0])),
    (d) => (d.candidates[0].frame.loaderId = 'BAD SPACE'),
  ]) {
    const i = targetInputs();
    update(i.proof.pdfDiagnostic.targetDiagnostic);
    assert.throws(() => checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture));
  }
});
test('PDF v2 refuses target payload leakage, unsafe methods and cleanup failures', () => {
  for (const update of [
    (d) => (d.mainTarget.url = 'PRIVATE'),
    (d) => (d.relatedSessions[0].target.title = 'PRIVATE'),
    (d) => (d.sessionCalls[0].method = 'Input.dispatchMouseEvent'),
    (d) => (d.sessionCalls[0].method = 'Target.createTarget'),
    (d) => (d.sessionCalls[0].sessionId = 'FOREIGN'),
    (d) => (d.autoAttachCleaned = false),
    (d) => (d.watcherCleaned = false),
    (d) => (d.candidates[0].state.documentMatches = false),
    (d) => (d.candidates[0].state.loadSucceededMethodPresent = false),
    (d) => (d.eventCount = 65),
    (d) => (d.snapshots[0].mainVerified = false),
  ]) {
    const i = targetInputs();
    update(i.proof.pdfDiagnostic.targetDiagnostic);
    assert.throws(() => checkPdfDiagnostic(i.proof, i.owned, i.bound, i.fixture));
  }
});
