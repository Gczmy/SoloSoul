import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, mkdir, readFile, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import {
  validateMediaManifest,
  readMediaManifest,
  PUBLIC_MEDIA_ASSETS,
  checkPreparedMedia,
  verifyPreparedMediaFiles,
} from './native-perf-media-contract.mjs';
import {
  MEDIA_PHASES,
  checkJourney,
  journeyBinaryPreflight,
  summarizeMediaJourneys,
  unmeasuredFor,
} from './native-perf-sdk-journey.mjs';
function marker() {
  const attachments = PUBLIC_MEDIA_ASSETS.map((asset, i) => ({
    id: `att_00000000-0000-0000-0000-${String(i).padStart(12, '0')}`,
    relativePath: `attachments/obj_perf_00000000/att_00000000-0000-0000-0000-${String(i).padStart(12, '0')}/${asset.fileName}`,
  }));
  return {
    schemaVersion: 2,
    scope: 'synthetic-native-media-vault-fixture',
    generator: 'solosoul-core/examples/perf_baseline --media-fixture-output',
    mediaObjectId: 'obj_perf_00000000',
    includesAttachments: true,
    includesOcrFixture: true,
    baseFixture: {
      schemaVersion: 1,
      scope: 'synthetic-native-vault-fixture',
      generator: 'solosoul-core/examples/perf_baseline',
      fixture: 'deterministic-20th-object-property-match',
      determinism:
        'object-content-only; salt, encryption nonce and account/profile timestamps vary',
      accountId: 'acc_rf312_100',
      accountName: 'Performance Fixture',
      objectCount: 100,
      searchQuery: 'needle',
      expectedSearchMatches: 5,
      buildProfile: 'release',
      kdf: { memoryKiB: 65536, iterations: 3, parallelism: 4 },
      includesProfile: true,
      includesUiPreferences: true,
      includesAttachments: false,
      includesOcrFixture: false,
    },
    publicAssets: structuredClone(PUBLIC_MEDIA_ASSETS),
    attachments,
    closedFiles: [
      'accounts.json',
      'ui_preferences.json',
      'acc_rf312_100/config.json',
      'acc_rf312_100/vault.db',
      ...attachments.map((a) => a.relativePath),
    ]
      .sort()
      .map((relativePath) => ({ relativePath, sha256: 'a'.repeat(64) })),
  };
}
test('media contract accepts only fixed assets, production base and closed canonical paths', () => {
  assert.equal(validateMediaManifest(marker()).closedFiles.length, 8);
  for (const mutate of [
    (m) => (m.schemaVersion = 1),
    (m) => (m.scope = 'synthetic-native-vault-fixture'),
    (m) => (m.private = 'sentinel'),
    (m) => (m.baseFixture.kdf.memoryKiB = 8192),
    (m) => (m.baseFixture.buildProfile = 'debug'),
    (m) => (m.baseFixture.private = 'sentinel'),
    (m) => (m.publicAssets[0].sha256 = 'b'.repeat(64)),
    (m) => m.publicAssets[0].bytes++,
    (m) => (m.attachments[0].relativePath = '../vault.db'),
    (m) => (m.attachments[1] = structuredClone(m.attachments[0])),
    (m) => m.closedFiles.reverse(),
    (m) => (m.closedFiles[0].private = 'sentinel'),
    (m) => (m.closedFiles[0].sha256 = 'x'),
  ]) {
    const m = marker();
    mutate(m);
    assert.throws(() => validateMediaManifest(m));
  }
});
test('media file proof checks actual bytes and bounded marker without writing source', async () => {
  const dir = await mkdtemp(path.join(os.tmpdir(), 'rf312-media-node-'));
  try {
    const m = marker();
    for (const f of m.closedFiles) {
      const full = path.join(dir, f.relativePath);
      await mkdir(path.dirname(full), { recursive: true });
      await writeFile(full, 'public-test-bytes');
      f.sha256 = createHash('sha256').update('public-test-bytes').digest('hex');
    }
    const file = path.join(dir, 'rf312-media-fixture.json');
    const raw = JSON.stringify(m);
    await writeFile(file, raw);
    assert.equal((await readMediaManifest(dir)).schemaVersion, 2);
    assert.equal(await readFile(file, 'utf8'), raw);
    const proof = {
      vault: dir,
      fixture: {
        marker: m,
        searchMatches: 5,
        files: [
          ...structuredClone(m.closedFiles),
          {
            relativePath: 'rf312-media-fixture.json',
            sha256: createHash('sha256').update(raw).digest('hex'),
          },
        ].sort((a, b) =>
          a.relativePath < b.relativePath ? -1 : a.relativePath > b.relativePath ? 1 : 0,
        ),
      },
    };
    await verifyPreparedMediaFiles(proof, m);
    proof.fixture.files.find((f) => f.relativePath === 'rf312-media-fixture.json').sha256 =
      'f'.repeat(64);
    await assert.rejects(verifyPreparedMediaFiles(proof, m));

    await writeFile(path.join(dir, m.closedFiles[0].relativePath), 'changed');
    await assert.rejects(readMediaManifest(dir));
    await writeFile(file, ' '.repeat(128 * 1024 + 1));
    await assert.rejects(readMediaManifest(dir));
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
test('prepared media binds source identity and exactly nine closed-file proofs', () => {
  const source = marker(),
    own = structuredClone(source);
  const files = [
    ...structuredClone(own.closedFiles),
    { relativePath: 'rf312-media-fixture.json', sha256: 'b'.repeat(64) },
  ].sort((a, b) => a.relativePath.localeCompare(b.relativePath));
  const proof = { fixture: { marker: own, searchMatches: 5, files } };
  assert.equal(checkPreparedMedia(proof, source), proof);
  for (const mutate of [
    (p) => (p.fixture.searchMatches = 4),
    (p) => p.fixture.marker.attachments.reverse(),
    (p) => p.fixture.files.pop(),
    (p) => (p.fixture.files[0].sha256 = 'b'.repeat(64)),
    (p) => (p.fixture.files[0].private = 'sentinel'),
  ]) {
    const p = structuredClone(proof);
    mutate(p);
    assert.throws(() => checkPreparedMedia(p, source));
  }
});
const runId = 'a'.repeat(32),
  owned = { runId, root: 'C:\\owned\\sample' };
const state = (image) => ({
  kind: image ? 'image' : 'text',
  decoded: image,
  textMatches: !image,
  width: image ? 500 : 0,
  height: image ? 200 : 0,
  visible: true,
  paintedFrames: 2,
});
function journey() {
  const binding = {
    source: 'http://tauri.localhost/login',
    mainFrameId: 'FRAME',
    loaderId: 'LOADER',
    timeOriginMs: null,
    navigationEvents: 0,
    frameCreatedEvents: 0,
  };
  const bound = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-ui-bound',
    runId,
    root: owned.root,
    pid: 123,
    port: 9222,
    browserPid: 456,
    binding,
    timeOriginMs: 10,
  };
  const observer = {
    schemaVersion: 1,
    scope: 'windows-native-tauri-invoke-observer',
    runId,
    valid: true,
    total: 2,
    observedCount: 2,
    invalidReasons: [],
    installedAtMs: 0,
    timeOriginMs: 10,
    maxEvents: 16384,
    commands: [
      { command: 'fs_read_file_as_data_url', atMs: 4 },
      { command: 'fs_read_file_as_text', atMs: 5 },
    ],
  };
  const phases = MEDIA_PHASES.map((name, i) => {
    const image = name === 'attachment-image-preview',
      text = name === 'attachment-text-preview';
    return {
      name,
      success: true,
      durationMs: 10,
      ipc: {
        attempts: image || text ? 1 : 0,
        commands: image ? { fs_read_file_as_data_url: 1 } : text ? { fs_read_file_as_text: 1 } : {},
        reason: null,
      },
      inputTrust: { pointer: i, text: 0, untrusted: 0 },
      fromAtMs: i,
      toAtMs: i + 1,
      ...(image || text ? { mediaEndState: state(image) } : {}),
    };
  });
  return {
    bound,
    proof: {
      schemaVersion: 1,
      scope: 'windows-native-sdk-media-journey',
      runId,
      root: owned.root,
      pid: 123,
      port: 9222,
      objectCount: 100,
      browserPid: 456,
      binding: structuredClone(binding),
      timeOriginMs: 10,
      inputMethod: 'SDK-CDP-Input',
      success: true,
      reason: null,
      failedStep: null,
      calls: [
        'Page.getFrameTree',
        'Runtime.evaluate',
        'Input.dispatchMouseEvent',
        ...Array(3).fill('Input.insertText'),
      ],
      phases,
      lastProbe: {
        schemaVersion: 1,
        scope: 'windows-native-sdk-ui-probe',
        runId,
        step: 'home',
        outcome: 'ready',
        href: 'http://tauri.localhost/',
        origin: 'http://tauri.localhost',
        timeOriginMs: 10,
        atMs: 10,
        rootPresent: true,
        frameCount: 0,
        target: null,
        inputTrust: { pointer: 15, text: 3, untrusted: 0 },
        observer,
      },
      ipcAll: {
        attempts: 2,
        commands: { fs_read_file_as_data_url: 1, fs_read_file_as_text: 1 },
        reason: null,
      },
      elapsedMs: 100,
      unmeasured: unmeasuredFor(true),
    },
  };
}
test('nine-phase media proof requires decoded/painted public end state and real read attempts', () => {
  const { bound, proof } = journey();
  assert.equal(checkJourney(proof, owned, bound, { objectCount: 100 }, true).phases.length, 9);
  assert.throws(() => checkJourney(proof, owned, bound, { objectCount: 100 }));
  for (const mutate of [
    (p) => (p.scope = 'windows-native-sdk-ui-journey'),
    (p) => (p.phases[4].mediaEndState.decoded = false),
    (p) => (p.phases[4].mediaEndState.width = 499),
    (p) => (p.phases[4].mediaEndState.paintedFrames = 1),
    (p) => (p.phases[5].mediaEndState.textMatches = false),
    (p) => (p.phases[5].mediaEndState.private = 'sentinel'),
    (p) => delete p.phases[4].mediaEndState,
    (p) => {
      p.phases[4].ipc = { attempts: 0, commands: {}, reason: null };
    },
    (p) => (p.phases[3].mediaEndState = state(true)),
    (p) => (p.lastProbe.observer.commands[0].payload = 'sentinel'),
  ]) {
    const p = structuredClone(proof);
    mutate(p);
    assert.throws(() => checkJourney(p, owned, bound, { objectCount: 100 }, true));
  }
});
test('media summaries never accept successful subsets, bad phases or incomplete cleanup', () => {
  const sample = {
    success: true,
    cleanupIntegrity: { complete: true },
    phases: journey().proof.phases,
  };
  const full = Array.from({ length: 3 }, () => structuredClone(sample));
  assert.equal(summarizeMediaJourneys(full, 3)[4].medianMs, 10);
  for (const samples of [
    full.slice(0, 2),
    [...full.slice(0, 2), { ...sample, success: false }],
    [...full.slice(0, 2), { ...sample, cleanupIntegrity: { complete: false } }],
  ])
    assert.ok(
      summarizeMediaJourneys(samples, 3).every((p) => p.medianMs === null && p.p95Ms === null),
    );
  full[2].phases.pop();
  assert.ok(summarizeMediaJourneys(full, 3).every((p) => p.medianMs === null));
  assert.ok(
    summarizeMediaJourneys(
      Array.from({ length: 3 }, () => sample),
      3,
      false,
    ).every((p) => p.medianMs === null),
  );
});
test('media EXE marker gate rejects old input-only builds before launch', async () => {
  const dir = await mkdtemp(path.join(os.tmpdir(), 'rf312-media-exe-'));
  try {
    const file = path.join(dir, 'candidate');
    await writeFile(file, 'windows-native-sdk-ui-journey-requested');
    await assert.rejects(journeyBinaryPreflight(file, true));
    await writeFile(
      file,
      '--native-perf-media-prepare\0windows-native-sdk-media-journey-requested',
    );
    assert.equal((await journeyBinaryPreflight(file, true)).present, true);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
async function probe(
  step,
  {
    width = 500,
    text = 'RF-312 public synthetic attachment preview.\nHello SoloSoul 1234567890.\n',
    mediaJourney = true,
    decodeFails = false,
  } = {},
) {
  let clock = 1,
    frames = 0,
    decodes = 0;
  const node = {
    getBoundingClientRect: () => ({ width: 500, height: 200 }),
    click: () => {
      throw Error('DOM click');
    },
  };
  const image = {
    ...node,
    complete: true,
    naturalWidth: width,
    naturalHeight: 200,
    decode: async () => {
      decodes++;
      if (decodeFails) throw Error('decode');
    },
  };
  const pre = { ...node, textContent: text };
  const overlay = {
    ...node,
    querySelectorAll: (s) => (s === 'pre' ? [pre] : s === 'img[alt="ocr_test.png"]' ? [image] : []),
  };
  const window = {
    __SOLOSOUL_NATIVE_PERF_RUN_ID__: runId,
    __SOLOSOUL_NATIVE_PERF__: { snapshot: () => journey().proof.lastProbe.observer },
    addEventListener: () => {},
  };
  window.top = window;
  const document = {
    querySelectorAll: (s) => (s === '[data-testid="attachment-preview-overlay"]' ? [overlay] : []),
    getElementById: (s) => (s === 'root' ? { hasChildNodes: () => true } : null),
    hasFocus: () => true,
    visibilityState: 'visible',
    body: {},
  };
  const source = await readFile(
    new URL('../src-tauri/src/native_perf/sdk_journey.js', import.meta.url),
    'utf8',
  );
  const result = await vm.runInNewContext(
    source.replace('__REQUEST__', JSON.stringify({ step, runId, objectCount: 100, mediaJourney })),
    {
      window,
      document,
      location: {
        pathname: '/settings/attachments',
        href: 'http://tauri.localhost/settings/attachments',
        origin: 'http://tauri.localhost',
      },
      performance: { now: () => (clock += 10000), timeOrigin: 10 },
      getComputedStyle: () => ({ visibility: 'visible', display: 'block' }),
      requestAnimationFrame: (callback) => {
        frames++;
        queueMicrotask(callback);
      },
      innerWidth: 1200,
      innerHeight: 800,
    },
  );
  return { result, frames, decodes };
}
test('actual media probe waits for image decode and two frames, or exact public text', async () => {
  for (const step of ['imagePreviewReady', 'textPreviewReady']) {
    const { result, frames, decodes } = await probe(step);
    assert.equal(result.outcome, 'ready');
    assert.equal(frames, 2);
    assert.equal(decodes, step === 'imagePreviewReady' ? 1 : 0);
    assert.deepEqual(JSON.parse(JSON.stringify(result.media)), state(step === 'imagePreviewReady'));
    assert.equal(JSON.stringify(result).includes('Hello SoloSoul'), false);
  }
});
test('actual media probe rejects wrong dimensions, text, decode failure and implicit modes', async () => {
  for (const [step, options] of [
    ['imagePreviewReady', { width: 499 }],
    ['imagePreviewReady', { decodeFails: true }],
    ['textPreviewReady', { text: 'wrong' }],
  ]) {
    const { result } = await probe(step, options);
    assert.equal(result.outcome, 'timeout');
    assert.equal(Object.hasOwn(result, 'media'), false);
  }
  await assert.rejects(probe('imagePreviewReady', { mediaJourney: false }));
});

async function scrollProbe({
  step = 'attachmentsCard',
  mediaJourney = true,
  y = 770.03125,
  disabled = false,
  scrollable = true,
  wheelCovered = false,
  actionable = false,
} = {}) {
  let clock = 1;
  const forbidden = () => {
    throw Error('DOM mutation is forbidden');
  };
  const card = {
    disabled,
    getBoundingClientRect: () => ({ x: 339, y: y - 50, width: 200, height: 100 }),
    querySelectorAll: () => [{ textContent: step === 'searchCard' ? 'Search' : 'Attachments' }],
    contains: (node) => node === card,
    click: forbidden,
    scrollIntoView: forbidden,
  };
  const content = {
    scrollHeight: scrollable ? 2000 : 600,
    clientHeight: 600,
    getBoundingClientRect: () => ({ left: 200, top: 80, bottom: 680, width: 800, height: 600 }),
    contains: (node) => node === card || node === content,
    scrollTo: forbidden,
    scrollBy: forbidden,
  };
  for (const node of [card, content])
    for (const key of ['scrollTop', 'scrollLeft', 'value'])
      Object.defineProperty(node, key, { get: () => 0, set: forbidden });
  const window = {
    __SOLOSOUL_NATIVE_PERF_RUN_ID__: runId,
    __SOLOSOUL_NATIVE_PERF__: { snapshot: () => journey().proof.lastProbe.observer },
    addEventListener: () => {},
  };
  window.top = window;
  const document = {
    querySelectorAll: (selector) =>
      selector === '[data-shell-content] [data-ui-card][role="button"]' ? [card] : [],
    querySelector: (selector) => (selector === '[data-shell-content]' ? content : null),
    getElementById: (id) => (id === 'root' ? { hasChildNodes: () => true } : null),
    elementFromPoint: (x, py) =>
      x === 600 && py === 380 ? (wheelCovered ? {} : content) : actionable ? card : null,
  };
  const source = await readFile(
    new URL('../src-tauri/src/native_perf/sdk_journey.js', import.meta.url),
    'utf8',
  );
  return vm.runInNewContext(
    source.replace('__REQUEST__', JSON.stringify({ step, runId, objectCount: 100, mediaJourney })),
    {
      window,
      document,
      location: {
        pathname: '/',
        href: 'http://tauri.localhost/',
        origin: 'http://tauri.localhost',
      },
      performance: { now: () => clock++, timeOrigin: 10 },
      getComputedStyle: () => ({ visibility: 'visible', display: 'block' }),
      requestAnimationFrame: (callback) => queueMicrotask(callback),
      innerWidth: 1200,
      innerHeight: 800,
    },
  );
}
test('actual media probe requests bounded wheel only outside verified content clip without DOM scrolling', async () => {
  const result = await scrollProbe();
  assert.equal(result.outcome, 'ready');
  assert.equal(result.target.actionable, false);
  assert.deepEqual(JSON.parse(JSON.stringify(result.target.scroll)), {
    x: 600,
    y: 380,
    deltaY: 390.03125,
    viewportWidth: 1200,
    viewportHeight: 800,
    clipTop: 80,
    clipBottom: 680,
  });
  assert.equal((await scrollProbe({ y: -10 })).target.scroll.deltaY, -390);
  assert.equal((await scrollProbe({ y: 1500 })).target.scroll.deltaY, 600);
  for (const options of [
    { y: 300 },
    { disabled: true },
    { scrollable: false },
    { wheelCovered: true },
    { step: 'searchCard', mediaJourney: false },
    { y: 300, actionable: true },
  ]) {
    const probe = await scrollProbe(options);
    assert.equal(Object.hasOwn(probe.target, 'scroll'), false, JSON.stringify(options));
  }
});
