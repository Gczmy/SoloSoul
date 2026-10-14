import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { deflateSync } from 'node:zlib';
import {
  PDF_ASSET_SHA,
  PDF_MASK_SHA,
  PDF_VIEWPORT,
  PDF_ROI,
  inspectPdfPixels,
  pngCrc,
} from './native-perf-pdf-pixels.mjs';
import {
  checkPdfFirstPage,
  summarizePdfFirstPage,
  PDF_PREVIEW_UNMEASURED,
} from './native-perf-pdf-preview-contract.mjs';
import { journeyBinaryPreflight } from './native-perf-sdk-journey.mjs';
const reference = await readFile(
  new URL('../src-tauri/src/native_perf/fixtures/pdf-first-page.png', import.meta.url),
);
const observed = inspectPdfPixels(reference);
// 原件复制自 RF-312 公开夹具诊断 sample-002；历史参考不替换。
const integratedReference = await readFile(
  new URL('../src-tauri/src/native_perf/fixtures/pdf-first-page-integrated.png', import.meta.url),
);
const integratedViewport = { width: 1200, height: 831 };
const integratedRoi = { x: 266, y: 221, width: 430, height: 36 };
const resourceId = 'att_00000000-0000-0000-0000-000000000001';
const resourcePath = `attachments/obj_perf_00000000/${resourceId}/text_only.pdf`;
const owned = {
  runId: 'a'.repeat(32),
  root: 'C:/public-owned',
  fixture: {
    marker: {
      schemaVersion: 2,
      scope: 'synthetic-native-media-vault-fixture',
      publicAssets: [{}, { fileName: 'text_only.pdf', sha256: PDF_ASSET_SHA }],
      attachments: [{}, { id: resourceId, relativePath: resourcePath }],
    },
    files: [{ relativePath: resourcePath, sha256: 'c'.repeat(64) }],
  },
};
const bound = {
  pid: 10,
  browserPid: 11,
  port: 9222,
  timeOriginMs: 1000,
  binding: {
    source: 'http://tauri.localhost/login',
    mainFrameId: 'MAIN',
    loaderId: 'LOADER',
    timeOriginMs: null,
    navigationEvents: 0,
    frameCreatedEvents: 0,
  },
};
const fixture = { objectCount: 100 };
const trust = { pointer: 20, text: 1, untrusted: 0 };
const dom = (phase) => ({
  schemaVersion: 1,
  scope: 'windows-native-sdk-pdf-dom',
  phase,
  outcome: 'ready',
  mainVerified: true,
  embedded: phase === 'opened',
  frameElementCount: 0,
  atMs: 100,
  target: phase === 'closed' ? null : { x: 500, y: 60, actionable: true },
  inputTrust: { ...trust },
});
function proof(schemaVersion = 2) {
  const image = schemaVersion === 3 ? integratedReference : reference;
  const pixels = inspectPdfPixels(image, schemaVersion);
  const samples = [0, 1].map((i) => ({
    ...pixels,
    frameStable: true,
    index: i,
    fileName: `native-perf-sdk-pdf-first-page-${String(i + 1).padStart(2, '0')}.png`,
    bytes: image.length,
    sha256: 'b'.repeat(64),
    captureStartedAtMs: 21000 + i * 500,
    captureFinishedAtMs: 21010 + i * 500,
    decodeFinishedAtMs: 21012 + i * 500,
    verifiedAtMs: 21020 + i * 500,
    frameBefore: { mainVerified: true, frames: [] },
    frameAfter: { mainVerified: true, frames: [] },
    domBefore: dom('opened'),
    domAfter: dom('opened'),
  }));
  return {
    schemaVersion: 1,
    scope: 'windows-native-sdk-pdf-first-page',
    runId: owned.runId,
    root: owned.root,
    pid: 10,
    browserPid: 11,
    port: 9222,
    objectCount: 100,
    binding: structuredClone(bound.binding),
    timeOriginMs: 1000,
    inputMethod: 'SDK-CDP-Input',
    success: true,
    reason: null,
    failedStep: null,
    calls: [
      'Input.insertText',
      'Runtime.evaluate',
      'Page.getFrameTree',
      'Input.dispatchMouseEvent',
      'Input.dispatchMouseEvent',
      ...[0, 1].flatMap(() => [
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
      // 完成 PDF 关闭后仍需读取认证快照并核对同一主文档。
      'Runtime.evaluate',
      'Page.getFrameTree',
    ],
    phases: ['startup', 'password-unlock', 'workspace', 'attachment-list'].map((name) => ({
      name,
      success: true,
      durationMs: 1,
      ipc: { attempts: 0, commands: {}, reason: null },
      inputTrust: { ...trust },
      fromAtMs: 0,
      toAtMs: 1,
    })),
    lastProbe: {
      schemaVersion: 1,
      scope: 'windows-native-sdk-ui-probe',
      runId: owned.runId,
      timeOriginMs: 1000,
      step: 'attachments',
      href: 'http://tauri.localhost/settings/attachments',
      origin: 'http://tauri.localhost',
      outcome: 'ready',
      rootPresent: true,
      frameCount: 0,
      atMs: 100,
      target: null,
      inputTrust: { ...trust },
      observer: {
        schemaVersion: 1,
        scope: 'windows-native-tauri-invoke-observer',
        runId: owned.runId,
        timeOriginMs: 1000,
        installedAtMs: 0,
        valid: true,
        invalidReasons: [],
        maxEvents: 16384,
        observedCount: 0,
        total: 0,
        commands: [],
      },
    },
    ipcAll: null,
    elapsedMs: 23000,
    unmeasured: [...PDF_PREVIEW_UNMEASURED],
    pdfPreview: {
      schemaVersion,
      scope: 'windows-native-sdk-public-pdf-first-page',
      assetSha256: PDF_ASSET_SHA,
      resourceBinding: { kind: 'prepared-vault-ciphertext', bytes: 3076, sha256: 'c'.repeat(64) },
      renderVerified: true,
      viewport: { ...(schemaVersion === 3 ? integratedViewport : PDF_VIEWPORT) },
      roi: { ...(schemaVersion === 3 ? integratedRoi : PDF_ROI) },
      referenceMaskSha256: PDF_MASK_SHA,
      threshold: { alphaMin: 240, rgbMax: 64 },
      maxCaptures: 16,
      intervalMs: 250,
      requiredConsecutiveMatches: 2,
      openingStartedAtMs: 20000,
      samples,
      performanceMetrics: {
        firstMatchObservedMs: 1020,
        stableReadyObservedMs: 1520,
        captureOverheadMs: 20,
        decodeOverheadMs: 4,
        pollWaitMs: 250,
        captures: 2,
      },
      closedDom: dom('closed'),
      mainVerifiedAfterClose: true,
      frameCreatedEvents: 0,
    },
  };
}
const check = (v) => checkPdfFirstPage(v, owned, bound, fixture);
test('actual public PNG has independent fixed text mask', () =>
  assert.deepEqual(observed, {
    width: 1028,
    height: 749,
    darkPixels: 530,
    maskSha256: PDF_MASK_SHA,
    matches: true,
  }));
function chunk(kind, data) {
  const type = Buffer.from(kind),
    size = Buffer.alloc(4),
    crc = Buffer.alloc(4);
  size.writeUInt32BE(data.length);
  crc.writeUInt32BE(pngCrc(Buffer.concat([type, data])));
  return Buffer.concat([size, type, data, crc]);
}
function blank(partial = false, viewport = PDF_VIEWPORT, roi = PDF_ROI) {
  const { width, height } = viewport;
  const h = Buffer.alloc(13);
  h.writeUInt32BE(width);
  h.writeUInt32BE(height, 4);
  h[8] = 8;
  h[9] = 6;
  const raw = Buffer.alloc((width * 4 + 1) * height, 255);
  for (let y = 0; y < height; y++) raw[y * (width * 4 + 1)] = 0;
  if (partial) {
    const at = (roi.y + 15) * (width * 4 + 1) + 1 + (roi.x + 15) * 4;
    raw[at] = raw[at + 1] = raw[at + 2] = 0;
  }
  return Buffer.concat([
    reference.subarray(0, 8),
    chunk('IHDR', h),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}
test('independent decoder rejects blank, partial text, corrupt CRC and allocation bounds', () => {
  assert.equal(inspectPdfPixels(blank()).matches, false);
  const partial = inspectPdfPixels(blank(true));
  assert.equal(partial.matches, false);
  assert.equal(partial.darkPixels, 1);
  const corrupt = Buffer.from(reference);
  corrupt[100] ^= 1;
  for (const b of [
    corrupt,
    reference.subarray(0, 100),
    Buffer.alloc(300000),
    Buffer.from('not png'),
  ])
    assert.throws(() => inspectPdfPixels(b));
  const wrong = blank();
  wrong.writeUInt32BE(4096, 16);
  wrong.writeUInt32BE(pngCrc(wrong.subarray(12, 29)), 29);
  assert.throws(() => inspectPdfPixels(wrong), /viewport/);
});
test('strict complete first-page native proof accepts repeated observations', () =>
  assert.equal(check(proof()).pdfPreview.renderVerified, true));

test('actual integrated PNG keeps the original text mask with an explicit geometry version', () => {
  assert.equal(
    createHash('sha256').update(reference).digest('hex'),
    '5ffffdfc3c9f4a9db05a3170f5df71d07da13ce35a6528ffb011a435ff54b4c9',
  );
  assert.equal(
    createHash('sha256').update(integratedReference).digest('hex'),
    'f85d1cb4076535fbaec6825675e0f2e26aad2167b15729230919530700283609',
  );
  assert.deepEqual(inspectPdfPixels(integratedReference, 3), {
    ...integratedViewport,
    darkPixels: 530,
    maskSha256: PDF_MASK_SHA,
    matches: true,
  });
  assert.throws(() => inspectPdfPixels(integratedReference, 2));
  assert.throws(() => inspectPdfPixels(reference, 3));
  for (const version of [1, 4, '3', null, {}, '__proto__'])
    assert.throws(() => inspectPdfPixels(reference, version));
});

test('current geometry rejects blank, partial text, corruption, format and unknown viewport', () => {
  assert.equal(inspectPdfPixels(blank(false, integratedViewport, integratedRoi), 3).matches, false);
  const partial = inspectPdfPixels(blank(true, integratedViewport, integratedRoi), 3);
  assert.equal(partial.matches, false);
  assert.equal(partial.darkPixels, 1);
  const crc = Buffer.from(integratedReference);
  crc[100] ^= 1;
  assert.throws(() => inspectPdfPixels(crc, 3));
  for (const [at, value] of [
    [16, 1201],
    [20, 832],
    [24, 16],
    [25, 2],
    [28, 1],
  ]) {
    const wrong = Buffer.from(integratedReference);
    if (at < 24) wrong.writeUInt32BE(value, at);
    else wrong[at] = value;
    wrong.writeUInt32BE(pngCrc(wrong.subarray(12, 29)), 29);
    assert.throws(() => inspectPdfPixels(wrong, 3));
  }
});

test('native proof binds both geometry versions and visible close control bounds', () => {
  for (const version of [2, 3]) {
    const v = proof(version);
    assert.equal(check(v), v);
    const other = proof(version === 2 ? 3 : 2).pdfPreview;
    const edits = [
      (d) => (d.schemaVersion = version === 2 ? 3 : 2),
      (d) => (d.schemaVersion = 4),
      (d) => (d.schemaVersion = String(version)),
      (d) => (d.viewport = other.viewport),
      (d) => (d.roi = other.roi),
      (d) => (d.samples[0].width = other.samples[0].width),
      (d) => (d.samples[0].height = other.samples[0].height),
      (d) => (d.samples[0].domAfter.target.x = d.viewport.width),
      (d) => (d.samples[0].domAfter.target.y = d.viewport.height),
      (d) => (d.samples[1].captureStartedAtMs = d.samples[0].verifiedAtMs + 249),
      (d) => (d.samples[1].domAfter.inputTrust.untrusted = 1),
      (d) => (d.requiredConsecutiveMatches = 1),
      (d) => (d.maxCaptures = 17),
      (d) => (d.intervalMs = 249),
    ];
    for (const edit of edits) {
      const bad = structuredClone(v);
      edit(bad.pdfPreview);
      assert.throws(() => check(bad));
    }
  }
  const current = proof(3);
  current.pdfPreview.samples[0].domAfter.target.x = 1100;
  current.pdfPreview.samples[0].domAfter.target.y = 800;
  assert.equal(check(current), current);
});
test('PDF protocol rejects missing, reordered or extra final authentication guards', () => {
  const edits = [
    (v) => v.calls.splice(-2),
    (v) => v.calls.splice(-1),
    (v) => v.calls.splice(-2, 1),
    (v) => v.calls.splice(-2, 2, 'Page.getFrameTree', 'Runtime.evaluate'),
    (v) => v.calls.push('Runtime.evaluate', 'Page.getFrameTree'),
    (v) => v.calls.splice(-4, 1),
    (v) => v.calls.splice(-2, 0, 'Page.captureScreenshot'),
  ];
  for (const edit of edits) {
    const v = proof();
    edit(v);
    assert.throws(() => check(v), /protocol guards|fixed contract/);
  }
});

test('identity, source, private payload, PNG proof and time tampering fail', () => {
  const edits = [
    (v) => v.pid++,
    (v) => v.browserPid++,
    (v) => (v.binding.loaderId = 'OTHER'),
    (v) => (v.lastProbe.href = 'http://evil.invalid/'),
    (v) => (v.lastProbe.observer.valid = false),
    (v) => (v.lastProbe.observer.extra = 'private'),
    (v) => (v.pdfPreview.assetSha256 = '0'.repeat(64)),
    (v) => (v.pdfPreview.resourceBinding.sha256 = PDF_ASSET_SHA),
    (v) => (v.pdfPreview.resourceBinding.bytes = 3035),
    (v) => (v.pdfPreview.resourceBinding.kind = 'plaintext'),
    (v) => (v.pdfPreview.viewport.width = 1000),
    (v) => v.pdfPreview.roi.x++,
    (v) => (v.pdfPreview.requiredConsecutiveMatches = 1),
    (v) => (v.pdfPreview.renderVerified = false),
    (v) => (v.pdfPreview.mainVerifiedAfterClose = false),
    (v) => (v.pdfPreview.frameCreatedEvents = 5),
    (v) => (v.pdfPreview.samples[0].sha256 = 'bad'),
    (v) => (v.pdfPreview.samples[0].fileName = '../escape.png'),
    (v) => (v.pdfPreview.samples[0].matches = false),
    (v) => (v.pdfPreview.samples[0].maskSha256 = '0'.repeat(64)),
    (v) => (v.pdfPreview.samples[0].privateText = 'private'),
    (v) => (v.pdfPreview.samples[1].captureStartedAtMs = 21100),
    (v) => (v.pdfPreview.samples[1].frameAfter.mainVerified = false),
    (v) => (v.pdfPreview.samples[1].domAfter.inputTrust.untrusted = 1),
    (v) => (v.pdfPreview.closedDom.embedded = true),
    (v) => (v.pdfPreview.performanceMetrics.stableReadyObservedMs = 1),
    (v) => (v.pdfPreview.performanceMetrics.captureOverheadMs = 0),
    (v) => v.calls.push('Target.attachToTarget'),
    (v) => v.calls.pop(),
    (v) => (v.unmeasured = []),
    (v) => v.phases.pop(),
  ];
  for (const edit of edits) {
    const v = proof();
    edit(v);
    assert.throws(() => check(v));
  }
});
test('distribution requires the entire requested group, identities and cleanup', () => {
  const samples = [1, 2, 3].map(() => ({
    success: true,
    cleanupIntegrity: { complete: true },
    nativeProof: proof(),
  }));
  const sum = summarizePdfFirstPage(samples, 3, true);
  assert.equal(sum[1].successfulSamples, 3);
  assert.equal(sum[1].medianMs, 1520);
  assert.equal(sum[1].p95Ms, 1520);
  for (const group of [
    samples.slice(0, 2),
    [{ ...samples[0], success: false }, ...samples.slice(1)],
    [{ ...samples[0], cleanupIntegrity: { complete: false } }, ...samples.slice(1)],
  ])
    assert.ok(
      summarizePdfFirstPage(group, 3, true).every(
        (x) => x.medianMs === null && x.successfulSamples === 0,
      ),
    );
  assert.ok(summarizePdfFirstPage(samples, 3, false).every((x) => x.medianMs === null));
});
test('preview preflight rejects older EXE without exact marker', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'ss-pdf-pixel-preflight-'));
  const file = path.join(dir, 'mock.exe');
  try {
    await writeFile(
      file,
      '--native-perf-media-prepare\0windows-native-sdk-media-journey-requested',
    );
    await assert.rejects(journeyBinaryPreflight(file, true, false, true));
    await writeFile(
      file,
      '--native-perf-media-prepare\0windows-native-sdk-media-journey-requested\0windows-native-sdk-pdf-first-page-requested\0windows-native-sdk-pdf-cipher-binding-requested windows-native-sdk-pdf-frame-stability-requested',
    );
    assert.equal((await journeyBinaryPreflight(file, true, false, true)).present, true);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

// 子 frame 变化的捕获可保留与计入采样开销，但不能冒充首次匹配或稳定就绪。
test('transient child-frame screenshots require two later stable captures and keep their overhead', () => {
  const v = proof(),
    d = v.pdfPreview,
    transient = structuredClone(d.samples[0]);
  transient.frameStable = false;
  transient.frameAfter.frames = [
    {
      id: 'PDF_CHILD',
      parentId: 'MAIN',
      loaderId: 'PDF_LOADER',
      urlClass: 'owned-pdf',
      originClass: 'owned-pdf',
    },
  ];
  for (const key of [
    'captureStartedAtMs',
    'captureFinishedAtMs',
    'decodeFinishedAtMs',
    'verifiedAtMs',
  ])
    transient[key] -= 500;
  d.samples.unshift(transient);
  d.samples.forEach((shot, i) => {
    shot.index = i;
    shot.fileName = `native-perf-sdk-pdf-first-page-${String(i + 1).padStart(2, '0')}.png`;
  });
  v.calls.splice(
    v.calls.indexOf('Page.captureScreenshot') - 2,
    0,
    'Runtime.evaluate',
    'Page.getFrameTree',
    'Page.captureScreenshot',
    'Page.getFrameTree',
    'Runtime.evaluate',
  );
  d.performanceMetrics.captureOverheadMs += 10;
  d.performanceMetrics.decodeOverheadMs += 2;
  d.performanceMetrics.pollWaitMs = 500;
  d.performanceMetrics.captures = 3;
  assert.equal(checkPdfFirstPage(v, owned, bound, fixture), v);
  const dishonest = structuredClone(v);
  dishonest.pdfPreview.samples[0].frameStable = true;
  assert.throws(() => checkPdfFirstPage(dishonest, owned, bound, fixture));
  const early = structuredClone(v);
  early.pdfPreview.performanceMetrics.firstMatchObservedMs -= 500;
  assert.throws(() => checkPdfFirstPage(early, owned, bound, fixture));
  const unstableLast = structuredClone(v);
  unstableLast.pdfPreview.samples[2].frameAfter = transient.frameAfter;
  unstableLast.pdfPreview.samples[2].frameStable = false;
  assert.throws(() => checkPdfFirstPage(unstableLast, owned, bound, fixture));
});
