import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { checkAuthAttribution, describeAuthAttribution } from './native-perf-auth-contract.mjs';
import { authBinaryPreflight, runAuthenticationJourney } from './native-perf-auth-attribution.mjs';
const runId = 'a'.repeat(32),
  owned = { runId },
  bound = {
    runId,
    pid: 123,
    timeOriginMs: 1000,
    binding: { mainFrameId: 'frame', loaderId: 'loader' },
  };
function fixture() {
  return {
    schemaVersion: 1,
    scope: 'windows-native-auth-attribution',
    runId,
    pid: 123,
    binding: {
      source: 'http://tauri.localhost/',
      mainFrameId: 'frame',
      loaderId: 'loader',
      timeOriginMs: null,
      navigationEvents: 0,
      frameCreatedEvents: 0,
    },
    timeOriginMs: 1000,
    frontend: {
      schemaVersion: 1,
      scope: 'windows-native-auth-observer',
      runId,
      timeOriginMs: 1000,
      atMs: 100,
      valid: true,
      invalidReasons: [],
      invokeCount: 2,
      maxReplies: 128,
      maxFlows: 64,
      replies: [
        { invokeSeq: 1, command: 'login', startedAtMs: 11, headersAtMs: 20, status: 'tauri-ok' },
        {
          invokeSeq: 2,
          command: 'vault_list_accounts',
          startedAtMs: 31,
          headersAtMs: 40,
          status: 'tauri-ok',
        },
      ],
      flows: [
        {
          id: 1,
          events: [
            ['started', 0],
            ['login-await-start', 10],
            ['login-await-ok', 25],
            ['accounts-await-start', 30],
            ['accounts-await-ok', 45],
            ['state-set-start', 46],
            ['state-set-done', 47],
            ['finished', 48],
          ].map(([stage, atMs]) => ({ stage, atMs })),
        },
      ],
    },
    backend: {
      schemaVersion: 1,
      scope: 'windows-native-auth-backend',
      runId,
      clockScope: 'process-monotonic',
      atMs: 10000,
      valid: true,
      invalidReasons: [],
      maxAttempts: 64,
      maxStages: 128,
      attempts: [
        {
          id: 1,
          kind: 'login',
          rootVerified: true,
          startedAtMs: 5000,
          endedAtMs: 5010,
          outcome: 'completed',
          stages: [
            { name: 'core-unlock', startedAtMs: 5001, endedAtMs: 5008, outcome: 'completed' },
            { name: 'kdf', startedAtMs: 5002, endedAtMs: 5006, outcome: 'completed' },
          ],
        },
        {
          id: 2,
          kind: 'accounts-refresh',
          rootVerified: true,
          startedAtMs: 5020,
          endedAtMs: 5024,
          outcome: 'completed',
          stages: [
            { name: 'accounts-list', startedAtMs: 5021, endedAtMs: 5023, outcome: 'completed' },
          ],
        },
      ],
    },
  };
}
test('auth attribution binds owned document and separates header arrival, actual await and backend clocks', () => {
  const value = fixture();
  assert.equal(checkAuthAttribution(value, owned, bound), value);
  const report = describeAuthAttribution(value, owned, bound);
  assert.equal(report.performanceMetrics, null);
  assert.equal(report.diagnosticOnly, true);
  assert.deepEqual(report.frontend.flows[0].awaits[0], {
    command: 'login',
    invokeSeq: 1,
    status: 'tauri-ok',
    headersDelayMs: 9,
    promiseWaitMs: 15,
    afterHeadersToAwaitMs: 5,
  });
  assert.equal(report.backend.attempts[0].durationMs, 10);
  assert.equal(report.backend.attempts[0].stages[0].durationMs, 7);
  assert.equal(report.backend.attempts[0].stages[1].durationMs, 4);
  assert.deepEqual(report.serialCommandPairing[0].pairs, [{ invokeSeq: 1, attemptId: 1 }]);
});
test('auth attribution rejects foreign identity, private fields, invalid chronology and impossible flow transitions', () => {
  for (const mutate of [
    (v) => (v.runId = 'b'.repeat(32)),
    (v) => v.pid++,
    (v) => v.timeOriginMs++,
    (v) => (v.binding.loaderId = 'foreign'),
    (v) => (v.frontend.replies[0].payload = 'private'),
    (v) => (v.frontend.replies[0].headersAtMs = 101),
    (v) => (v.frontend.replies[0].invokeSeq = 3),
    (v) => (v.frontend.flows[0].events[2].stage = 'state-set-done'),
    (v) => (v.frontend.flows[0].events[3].atMs = 2),
    (v) => (v.backend.attempts[0].rootVerified = false),
    (v) => (v.backend.attempts[0].stages[0].endedAtMs = 5011),
    (v) => (v.backend.attempts[0].stages[1].startedAtMs = 4999),
    (v) => (v.backend.attempts[1].stages[0].name = 'kdf'),
    (v) => (v.backend.attempts[0].private = 'sentinel'),
  ]) {
    const v = fixture();
    mutate(v);
    assert.throws(() => checkAuthAttribution(v, owned, bound));
  }
});
test('auth attribution keeps genuinely pending await and worker as null duration without inventing completion', () => {
  const v = fixture();
  v.frontend.flows[0].events.length = 2;
  v.frontend.replies.length = 1;
  v.frontend.replies[0].status = 'pending';
  v.frontend.replies[0].headersAtMs = null;
  v.backend.attempts.length = 1;
  v.backend.attempts[0].outcome = 'running';
  v.backend.attempts[0].endedAtMs = null;
  v.backend.attempts[0].stages[0].outcome = 'running';
  v.backend.attempts[0].stages[0].endedAtMs = null;
  const report = describeAuthAttribution(v, owned, bound);
  assert.equal(report.frontend.flows[0].awaits[0].promiseWaitMs, null);
  assert.equal(report.frontend.flows[0].awaits[0].headersDelayMs, null);
  assert.equal(report.backend.attempts[0].durationMs, null);
  assert.equal(report.backend.attempts[0].stages[0].durationMs, null);
});
test('auth attribution rejects lost calls and does not fabricate ordinal pairing when backend commands overlap', () => {
  const v = fixture();
  v.frontend.replies.push({
    invokeSeq: 3,
    command: 'vault_list_accounts',
    startedAtMs: 60,
    headersAtMs: 70,
    status: 'tauri-ok',
  });
  v.frontend.invokeCount = 3;
  assert.throws(() => describeAuthAttribution(v, owned, bound));
  v.backend.attempts.push({
    id: 3,
    kind: 'accounts-refresh',
    rootVerified: true,
    startedAtMs: 5022,
    endedAtMs: 5028,
    outcome: 'completed',
    stages: [],
  });
  const report = describeAuthAttribution(v, owned, bound);
  assert.equal(report.serialCommandPairing[1].paired, false);
  assert.deepEqual(report.serialCommandPairing[1].pairs, []);
  v.frontend.replies[0].status = 'pending';
  v.frontend.replies[0].headersAtMs = null;
  assert.throws(() => describeAuthAttribution(v, owned, bound));
});
test('auth binary preflight rejects old build and accepts explicit marker across stream boundary', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'rf312-auth-test-'));
  try {
    const file = path.join(root, 'candidate.exe');
    await writeFile(file, 'old');
    await assert.rejects(authBinaryPreflight(file));
    await writeFile(
      file,
      Buffer.concat([Buffer.alloc(65530), Buffer.from('windows-native-auth-attribution')]),
    );
    assert.equal(await authBinaryPreflight(file), true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('authentication delegate uses an accepted mode of the real six-phase SDK runner', async () => {
  assert.equal(await runAuthenticationJourney(['--help']), 0);
});

test('account write substeps remain fixed and bounded and cannot enter account refresh or carry private details', () => {
  const v = fixture();
  const names = [
    'account-manifest-serialize',
    'account-directory-create',
    'account-directory-permission',
    'account-manifest-atomic-write',
    'account-manifest-permission',
  ];
  v.backend.attempts[0].stages = [
    { name: 'account-write-call', startedAtMs: 5001, endedAtMs: 5009, outcome: 'completed' },
    ...names.map((name, i) => ({
      name,
      startedAtMs: 5002 + i,
      endedAtMs: 5003 + i,
      outcome: 'completed',
    })),
  ];
  const report = describeAuthAttribution(v, owned, bound);
  assert.equal(report.backend.attempts[0].stages.length, 6);
  assert.equal(report.performanceMetrics, null);
  for (const mutate of [
    (q) => (q.backend.attempts[0].kind = 'accounts-refresh'),
    (q) => (q.backend.attempts[0].stages[1].name = 'icacls private-path'),
    (q) => (q.backend.attempts[0].stages[1].path = 'private-path'),
    (q) => (q.backend.attempts[0].stages[1].endedAtMs = 5011),
    (q) => (q.backend.attempts[0].stages[0].endedAtMs = 5005),
    (q) => (q.backend.attempts[0].stages[1].name = 'account-directory-create'),
  ]) {
    const q = structuredClone(v);
    mutate(q);
    assert.throws(() => checkAuthAttribution(q, owned, bound));
  }
});
