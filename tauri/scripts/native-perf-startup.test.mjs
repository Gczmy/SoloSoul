import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import vm from 'node:vm';
import {
  checkStartup,
  checkStartupPreferences,
  checkPair,
  checkTicket,
  summarizePairs,
  startupBinaryPreflight,
  unmeasured,
} from './native-perf-startup.mjs';
const owner = 'a'.repeat(32),
  warmId = 'b'.repeat(32);
const owned = { root: 'C:\\owned\\sample', runId: owner, fixture: { objectCount: 100 } };
function fixture(generation = 1) {
  const runId = generation === 1 ? owner : warmId;
  const binding = {
    source: 'http://tauri.localhost/login',
    mainFrameId: 'FRAME' + generation,
    loaderId: 'LOADER' + generation,
    timeOriginMs: null,
    navigationEvents: 0,
    frameCreatedEvents: 0,
  };
  const clock = 1000 * generation,
    trust = { pointer: 0, text: 0, untrusted: 0 };
  const observer = {
    schemaVersion: 1,
    scope: 'windows-native-tauri-invoke-observer',
    runId,
    valid: true,
    total: 1,
    observedCount: 1,
    invalidReasons: [],
    installedAtMs: 0,
    timeOriginMs: clock,
    maxEvents: 16384,
    commands: [{ command: 'accounts_list', atMs: 1 }],
  };
  const ipc = { attempts: 1, commands: { accounts_list: 1 }, reason: null };
  const bound = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-ui-bound',
    runId,
    root: owned.root,
    pid: 100 + generation,
    port: 9222 + generation,
    browserPid: 200 + generation,
    binding,
    timeOriginMs: clock,
  };
  const proof = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-startup',
    runId,
    root: owned.root,
    pid: bound.pid,
    port: bound.port,
    objectCount: 100,
    browserPid: bound.browserPid,
    binding: structuredClone(binding),
    timeOriginMs: clock,
    inputMethod: 'SDK-CDP-read-only',
    success: true,
    reason: null,
    failedStep: null,
    calls: ['Runtime.evaluate', 'Page.getFrameTree'],
    phases: [
      {
        name: 'startup',
        success: true,
        durationMs: 500,
        ipc: structuredClone(ipc),
        inputTrust: structuredClone(trust),
        fromAtMs: 0,
        toAtMs: 10,
      },
    ],
    lastProbe: {
      schemaVersion: 1,
      scope: 'windows-native-sdk-ui-probe',
      runId,
      step: 'startup',
      outcome: 'ready',
      href: 'http://tauri.localhost/login',
      origin: 'http://tauri.localhost',
      timeOriginMs: clock,
      atMs: 10,
      rootPresent: true,
      frameCount: 0,
      target: null,
      inputTrust: trust,
      observer,
    },
    ipcAll: ipc,
    elapsedMs: 25,
    unmeasured: unmeasured(generation),
    startup: {
      generation,
      ownerRunId: owner,
      evidenceRoot: path.win32.join(owned.root, `startup-0${generation}`),
    },
  };
  return { bound, proof };
}
test('startup accepts readonly pairs with native duration distinct from capture elapsed time', () => {
  const first = fixture(),
    warm = fixture(2);
  assert.equal(checkStartup(first.proof, owned, first.bound, 1, owner).phases[0].durationMs, 500);
  assert.equal(checkStartup(warm.proof, owned, warm.bound, 2, warmId).elapsedMs, 25);
  checkPair(first.proof, warm.proof);
});
test('startup rejects input, failure, old journey format and unsupported fields', () => {
  for (const mutate of [
    (p) => p.calls.push('Input.insertText'),
    (p) => p.calls.reverse(),
    (p) => (p.success = false),
    (p) => (p.reason = 'probe-timeout'),
    (p) => (p.scope = 'windows-native-sdk-ui-journey'),
    (p) => (p.inputMethod = 'SDK-CDP-Input'),
    (p) => (p.startup.ownerRunId = warmId),
    (p) => (p.startup.evidenceRoot = 'C:\\other'),
    (p) => (p.private = 'sentinel'),
    (p) => p.phases.push(p.phases[0]),
    (p) => p.lastProbe.inputTrust.pointer++,
    (p) => (p.lastProbe.target = { private: 'sentinel' }),
    (p) => (p.lastProbe.frameCount = 1),
    (p) => (p.lastProbe.href = 'http://tauri.localhost/'),
    (p) => (p.phases[0].durationMs = NaN),
    (p) => (p.objectCount = 5000),
    (p) => (p.lastProbe.atMs = -1),
  ]) {
    const { proof, bound } = fixture();
    mutate(proof);
    assert.throws(() => checkStartup(proof, owned, bound, 1, owner));
  }
});
test('startup rejects malformed observer events and inconsistent IPC', () => {
  for (const mutate of [
    (p) => (p.lastProbe.observer.valid = false),
    (p) => p.lastProbe.observer.invalidReasons.push('overflow'),
    (p) => (p.lastProbe.observer.runId = warmId),
    (p) => (p.lastProbe.observer.commands[0].private = 'sentinel'),
    (p) => (p.lastProbe.observer.commands[0].atMs = 20),
    (p) => (p.lastProbe.observer.total = 2),
    (p) => (p.ipcAll.attempts = 2),
    (p) => (p.phases[0].ipc.commands.accounts_list = 2),
    (p) => (p.lastProbe.observer.installedAtMs = 11),
  ]) {
    const { proof, bound } = fixture();
    mutate(proof);
    assert.throws(() => checkStartup(proof, owned, bound, 1, owner));
  }
});
test('warm pair rejects reused processes, frame, clock or directories', () => {
  for (const mutate of [
    (p) => (p.runId = owner),
    (p) => (p.pid = 101),
    (p) => (p.browserPid = 201),
    (p) => (p.port = 9223),
    (p) => (p.timeOriginMs = 1000),
    (p) => (p.binding.mainFrameId = 'FRAME1'),
    (p) => (p.binding.loaderId = 'LOADER1'),
    (p) => (p.startup.ownerRunId = warmId),
    (p) => (p.root = 'C:\\other'),
  ]) {
    const first = fixture(),
      warm = fixture(2);
    mutate(warm.proof);
    assert.throws(() => checkPair(first.proof, warm.proof));
  }
});
function ticket() {
  const first = fixture().proof;
  const hashes = {
    ownedSha256: 'c'.repeat(64),
    consumedSha256: 'd'.repeat(64),
    proofSha256: 'e'.repeat(64),
    stoppedSha256: 'f'.repeat(64),
    exeSha256: '0'.repeat(64),
    uiPreferencesSha256: '1'.repeat(64),
  };
  return {
    first,
    hashes,
    value: {
      schemaVersion: 1,
      scope: 'windows-native-sdk-startup-restart-ticket',
      root: owned.root,
      ownerRunId: owner,
      runId: warmId,
      evidenceRoot: path.win32.join(owned.root, 'startup-02'),
      ...hashes,
      updateSourceCandidates: { manifest: ['https://example.com/latest.json'], release: [] },
      priorPid: first.pid,
      priorBrowserPid: first.browserPid,
      priorPort: first.port,
      exitCheck: {
        verified: true,
        queryPid: 50,
        checkedIdentities: 4,
        checkedAtUnixMs: Date.now(),
      },
    },
  };
}
test('warm ticket binds all original hashes and fresh exit receipt', () => {
  const t = ticket();
  assert.equal(checkTicket(t.value, owned, t.first, t.hashes, 4), t.value);
  for (const mutate of [
    (t) => (t.value.runId = owner),
    (t) => (t.value.ownerRunId = warmId),
    (t) => (t.value.proofSha256 = '1'.repeat(64)),
    (t) => (t.value.exeSha256 = '2'.repeat(64)),
    (t) => (t.value.uiPreferencesSha256 = '2'.repeat(64)),
    (t) => (t.value.updateSourceCandidates.manifest = 'invalid'),
    (t) => t.value.priorPid++,
    (t) => t.value.priorPort++,
    (t) => (t.value.exitCheck.checkedAtUnixMs -= 300001),
    (t) => (t.value.exitCheck.queryPid = 0),
    (t) => (t.value.exitCheck.verified = false),
    (t) => t.value.exitCheck.checkedIdentities--,
    (t) => (t.value.private = 'sentinel'),
  ]) {
    const t = ticket();
    mutate(t);
    assert.throws(() => checkTicket(t.value, owned, t.first, t.hashes, 4));
  }
});
test('a failed pair or missing sample rejects whole-group metrics', () => {
  const pair = {
    success: true,
    first: { phases: fixture().proof.phases },
    warm: { phases: fixture(2).proof.phases },
  };
  assert.equal(summarizePairs([pair, pair, pair], 3).metrics.reusedProfile.medianMs, 500);
  for (const [pairs, n, intact] of [
    [[pair, pair], 3, true],
    [[pair, { ...pair, success: false }, pair], 3, true],
    [[pair, { ...pair, cleanupIncomplete: true }, pair], 3, true],
    [[pair, pair, pair], 3, false],
  ])
    assert.equal(summarizePairs(pairs, n, intact).metrics, null);
});
test('offline gate rejects older executable before any launch', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'rf312-startup-unit-'));
  try {
    const exe = path.join(root, 'test.exe');
    await writeFile(exe, 'windows-native-sdk-ui-journey-requested');
    await assert.rejects(startupBinaryPreflight(exe));
    await writeFile(
      exe,
      'windows-native-sdk-startup-requested windows-native-sdk-startup-restart-ticket',
    );
    assert.equal((await startupBinaryPreflight(exe)).matched, true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test('actual startup probe waits for enabled submit and two frames without input or credential access', async () => {
  let clock = 0,
    frames = 0;
  const element = {
    disabled: true,
    getBoundingClientRect: () => ({ x: 10, y: 10, width: 100, height: 40 }),
  };
  Object.defineProperty(element, 'value', {
    get() {
      throw new Error('credential read');
    },
    set() {
      throw new Error('credential write');
    },
  });
  element.click = () => {
    throw new Error('DOM click');
  };
  const observer = fixture().proof.lastProbe.observer;
  const window = {
    __SOLOSOUL_NATIVE_PERF_RUN_ID__: owner,
    __SOLOSOUL_NATIVE_PERF__: { snapshot: () => observer },
    addEventListener: () => {},
  };
  window.top = window;
  const card = { textContent: 'Performance Fixture' };
  const context = {
    window,
    document: {
      getElementById: (id) => (id === 'root' ? { hasChildNodes: () => true } : null),
      querySelectorAll: (selector) =>
        ['[data-login-method-region="password"] input', '[data-login-password-submit]'].includes(
          selector,
        )
          ? [element]
          : selector === '[data-login-card]'
            ? [card]
            : [],
      querySelector: () => card,
    },
    location: {
      pathname: '/login',
      origin: 'http://tauri.localhost',
      href: 'http://tauri.localhost/login',
    },
    performance: { now: () => clock++, timeOrigin: 1000, getEntriesByName: () => [{}] },
    getComputedStyle: () => ({ visibility: 'visible', display: 'block' }),
    requestAnimationFrame: (callback) => {
      frames++;
      if (frames === 3) element.disabled = false;
      queueMicrotask(callback);
    },
  };
  const source = await readFile(
    new URL('../src-tauri/src/native_perf/sdk_journey.js', import.meta.url),
    'utf8',
  );
  const result = await vm.runInNewContext(
    source.replace(
      '__REQUEST__',
      JSON.stringify({ step: 'startup', runId: owner, objectCount: 100, readOnlyStartup: true }),
    ),
    context,
  );
  assert.equal(result.outcome, 'ready');
  assert.ok(frames >= 5);
  assert.equal(result.target, null);
  assert.equal(
    JSON.stringify(result.inputTrust),
    JSON.stringify({ pointer: 0, text: 0, untrusted: 0 }),
  );
});

test('startup preserves fixed preferences and validates only native producer cache sources', () => {
  const fixed = {
    theme: 'light',
    accentColor: 'ocean',
    customAccentHex: '',
    reduceMotion: false,
    androidGlass: 'local',
    language: 'en-US',
    hasSeenOnboarding: true,
    notificationPermissionRequested: true,
  };
  const candidates = {
    manifest: ['https://github.com/Gczmy/SoloSoul/releases/latest/download/latest.json'],
    release: ['https://api.github.com/repos/Gczmy/SoloSoul/releases/latest'],
  };
  const valid = {
    ...fixed,
    updateSources: {
      manifest: { url: candidates.manifest[0], probedAt: Math.floor(Date.now() / 1000) },
      release: null,
      lastChannel: 'manifest',
    },
  };
  assert.doesNotThrow(() => checkStartupPreferences(fixed, candidates));
  assert.doesNotThrow(() => checkStartupPreferences(valid, candidates));
  for (const mutate of [
    (v) => (v.theme = 'dark'),
    (v) => (v.private = 'sentinel'),
    (v) => (v.updateSources.manifest.url = 'https://unapproved.invalid/latest.json'),
    (v) => (v.updateSources.manifest.probedAt += 3600),
    (v) => (v.updateSources.manifest.private = 'sentinel'),
    (v) => {
      delete v.updateSources.release;
      v.updateSources.private = null;
    },
    (v) => (v.updateSources.lastChannel = 'release'),
  ]) {
    const bad = structuredClone(valid);
    mutate(bad);
    assert.throws(() => checkStartupPreferences(bad, candidates));
  }
});
