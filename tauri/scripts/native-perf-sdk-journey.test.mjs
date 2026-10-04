import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdtemp, writeFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import {
  PHASES,
  checkBound,
  checkJourney,
  journeyBinaryPreflight,
  summarizeJourneys,
} from './native-perf-sdk-journey.mjs';
const runId = 'a'.repeat(32),
  owned = { root: 'C:\\owned\\sample', runId };
function fixture() {
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
    total: 0,
    observedCount: 0,
    invalidReasons: [],
    installedAtMs: 0,
    timeOriginMs: 10,
    maxEvents: 16384,
    commands: [],
  };
  const proof = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-ui-journey',
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
      'Input.insertText',
      'Input.insertText',
      'Input.insertText',
    ],
    phases: PHASES.map((name, index) => ({
      name,
      success: true,
      durationMs: 10,
      ipc: { attempts: 0, commands: {}, reason: null },
      inputTrust: { pointer: index, text: 0, untrusted: 0 },
      fromAtMs: index,
      toAtMs: index + 1,
    })),
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
      inputTrust: { pointer: 10, text: 3, untrusted: 0 },
      observer,
    },
    ipcAll: { attempts: 0, commands: {}, reason: null },
    elapsedMs: 60,
    unmeasured: [
      'OCR',
      'attachment-preview',
      'system-sleep',
      'same-profile-warm-start',
      'other-platforms',
    ],
  };
  return { bound, proof };
}
test('SDK journey accepts six measured phases only with same owned document and trusted input', () => {
  const { bound, proof } = fixture();
  assert.equal(checkBound(bound, owned, 123, 9222), bound);
  assert.equal(checkJourney(proof, owned, bound, { objectCount: 100 }).phases.length, 6);
});
test('SDK binding fails closed before public input authorization', () => {
  const { bound } = fixture();
  for (const [key, value] of [
    ['runId', 'b'.repeat(32)],
    ['pid', 99],
    ['root', 'C:\\other'],
    ['browserPid', 0],
    ['timeOriginMs', 0],
  ])
    assert.throws(() => checkBound({ ...bound, [key]: value }, owned, 123, 9222));
  assert.throws(() => checkBound({ ...bound, private: 'sentinel' }, owned, 123, 9222));
  for (const [key, value] of [
    ['private', 'sentinel'],
    ['navigationEvents', 1],
    ['mainFrameId', ''],
    ['timeOriginMs', 10],
  ])
    assert.throws(() =>
      checkBound({ ...bound, binding: { ...bound.binding, [key]: value } }, owned, 123, 9222),
    );
});
test('SDK journey rejects navigation, missing phases, wrong methods, failed inputs and invalid IPC', () => {
  for (const mutate of [
    (p) => p.timeOriginMs++,
    (p) => (p.lastProbe.schemaVersion = 0),
    (p) => (p.lastProbe.origin = 'http://other'),
    (p) => (p.lastProbe.atMs = NaN),
    (p) => (p.binding.loaderId = 'OTHER'),
    (p) => p.phases.pop(),
    (p) => p.calls.push('Runtime.callFunctionOn'),
    (p) => p.lastProbe.inputTrust.untrusted++,
    (p) => (p.lastProbe.inputTrust.text = 2),
    (p) => (p.lastProbe.observer.valid = false),
    (p) => (p.phases[0].ipc.attempts = 1),
    (p) => (p.phases[2].durationMs = NaN),
    (p) => (p.success = false),
  ]) {
    const { bound, proof } = fixture();
    mutate(proof);
    assert.throws(() => checkJourney(proof, owned, bound, { objectCount: 100 }));
  }
});
test('SDK proof validators reject private extra fields', () => {
  for (const mutate of [
    (p) => (p.private = 'sentinel'),
    (p) => (p.binding.private = 'sentinel'),
    (p) => (p.ipcAll.private = 'sentinel'),
    (p) => (p.lastProbe.private = 'sentinel'),
    (p) => (p.lastProbe.observer.private = 'sentinel'),
    (p) => (p.phases[0].private = 'sentinel'),
  ]) {
    const { bound, proof } = fixture();
    mutate(proof);
    assert.throws(() => checkJourney(proof, owned, bound, { objectCount: 100 }));
  }
});
test('SDK journey rejects an old EXE before any GUI startup', async () => {
  const dir = await mkdtemp(path.join(os.tmpdir(), 'rf312-sdk-marker-'));
  try {
    const exe = path.join(dir, 'candidate');
    await writeFile(exe, 'old-build');
    await assert.rejects(journeyBinaryPreflight(exe));
    await writeFile(exe, 'windows-native-sdk-ui-journey-requested');
    assert.equal((await journeyBinaryPreflight(exe)).present, true);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
test('actual SDK probe observes startup without reading password or sending actions', async () => {
  let time = 1;
  const element = {
    disabled: false,
    getBoundingClientRect: () => ({ x: 0, y: 0, width: 100, height: 40 }),
  };
  Object.defineProperty(element, 'value', {
    get() {
      throw new Error('password read');
    },
    set() {
      throw new Error('password write');
    },
  });
  element.click = () => {
    throw new Error('DOM click');
  };
  const screen = { textContent: 'Performance Fixture' },
    document = {
      getElementById: (key) => (key === 'root' ? { hasChildNodes: () => true } : null),
      querySelectorAll: (selector) =>
        selector === '[data-login-method-region="password"] input'
          ? [element]
          : selector === '[data-login-card]'
            ? [screen]
            : [],
      querySelector: () => screen,
    };
  const observer = fixture().proof.lastProbe.observer,
    window = {
      __SOLOSOUL_NATIVE_PERF_RUN_ID__: runId,
      __SOLOSOUL_NATIVE_PERF__: { snapshot: () => observer },
      addEventListener: () => {},
    };
  window.top = window;
  const context = {
    window,
    document,
    location: {
      pathname: '/login',
      origin: 'http://tauri.localhost',
      href: 'http://tauri.localhost/login',
    },
    performance: { now: () => time++, timeOrigin: 10, getEntriesByName: () => [{}] },
    getComputedStyle: () => ({ visibility: 'visible', display: 'block' }),
    requestAnimationFrame: (callback) => queueMicrotask(callback),
    innerWidth: 1200,
    innerHeight: 800,
  };
  const source = await readFile(
      new URL('../src-tauri/src/native_perf/sdk_journey.js', import.meta.url),
      'utf8',
    ),
    result = await vm.runInNewContext(
      source.replace('__REQUEST__', JSON.stringify({ step: 'startup', runId, objectCount: 100 })),
      context,
    );
  assert.equal(result.outcome, 'ready');
  assert.equal(result.rootPresent, true);
  assert.equal(result.target, null);
  assert.equal(result.inputTrust.text, 0);
  assert.equal(JSON.stringify(result).includes('password'), false);
});

test('rejected, interrupted or source-invalid samples never enter SDK performance summaries', () => {
  const phase = { name: 'startup', success: true, durationMs: 123 };
  const rejected = { success: false, phases: [phase] };
  assert.equal(summarizeJourneys([rejected], 3)[0].medianMs, null);
  assert.equal(summarizeJourneys([{ success: true, phases: [phase] }], 3, false)[0].medianMs, null);
  assert.equal(
    summarizeJourneys([{ success: true, phases: [phase] }, rejected], 3)[0].successfulSamples,
    1,
  );
});
