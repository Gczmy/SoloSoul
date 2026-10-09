import test from 'node:test';
import assert from 'node:assert/strict';
import { checkMaintenanceAdmission } from './native-perf-maintenance-contract.mjs';
const runId = 'a'.repeat(32),
  owned = { runId },
  bound = {
    runId,
    pid: 123,
    timeOriginMs: 1000,
    binding: { mainFrameId: 'frame', loaderId: 'loader' },
  };
function fixture() {
  const auth = {
    schemaVersion: 1,
    scope: 'windows-native-auth-attribution',
    runId,
    pid: 123,
    timeOriginMs: 1000,
    binding: {
      source: 'http://tauri.localhost/login',
      mainFrameId: 'frame',
      loaderId: 'loader',
      timeOriginMs: null,
      navigationEvents: 0,
      frameCreatedEvents: 0,
    },
    frontend: {
      schemaVersion: 1,
      scope: 'windows-native-auth-observer',
      runId,
      timeOriginMs: 1000,
      atMs: 20,
      valid: true,
      invalidReasons: [],
      invokeCount: 0,
      maxReplies: 128,
      maxFlows: 64,
      replies: [],
      flows: [],
    },
    backend: {
      schemaVersion: 1,
      scope: 'windows-native-auth-backend',
      runId,
      clockScope: 'process-monotonic',
      atMs: 10,
      valid: true,
      invalidReasons: [],
      maxAttempts: 64,
      maxStages: 128,
      attempts: [
        {
          id: 1,
          kind: 'login',
          rootVerified: true,
          startedAtMs: 2,
          endedAtMs: 8,
          outcome: 'interrupted',
          stages: [
            { name: 'root-read', startedAtMs: 2, endedAtMs: 3, outcome: 'completed' },
            { name: 'maintenance-admission', startedAtMs: 4, endedAtMs: 6, outcome: 'interrupted' },
          ],
        },
      ],
    },
  };
  const value = {
    schemaVersion: 1,
    scope: 'windows-native-maintenance-admission',
    runId,
    pid: 123,
    clockScope: 'process-monotonic',
    atMs: 10,
    valid: true,
    invalidReasons: [],
    coverage: ['update-source-preferences', 'owned-blocking-worker'],
    maxActivities: 256,
    registeredActivities: 2,
    maxFailures: 64,
    failures: [
      {
        attemptId: 1,
        stageToken: 1,
        atMs: 5,
        errorClass: 'operations-active',
        witnesses: [{ id: 1, kind: 'update-source-preferences', startedAtMs: 1 }],
      },
    ],
  };
  return { auth, value };
}
const check = (value, auth) => checkMaintenanceAdmission(value, auth, owned, bound);
test('maintenance witness binds fixed error to interrupted admission and one process clock', () => {
  const { auth, value } = fixture();
  assert.equal(check(value, auth), value);
  const mutations = [
    (v) => v.pid++,
    (v) => (v.runId = 'b'.repeat(32)),
    (v) => v.atMs++,
    (v) => (v.clockScope = 'frontend'),
    (v) => (v.valid = false),
    (v) => v.invalidReasons.push('private'),
    (v) => (v.coverage = ['all-workers']),
    (v) => v.maxActivities++,
    (v) => (v.registeredActivities = 257),
    (v) => v.maxFailures++,
    (v) => (v.failures[0].attemptId = 2),
    (v) => (v.failures[0].stageToken = 0),
    (v) => (v.failures[0].atMs = 6.5),
    (v) => (v.failures[0].atMs = 3.5),
    (v) => (v.failures[0].errorClass = 'private'),
    (v) => (v.failures[0].witnesses[0].kind = 'private'),
    (v) => (v.failures[0].witnesses[0].id = 0),
    (v) => (v.failures[0].witnesses[0].id = 3),
    (v) => (v.failures[0].witnesses[0].startedAtMs = 4.5),
    (v) => (v.failures[0].witnesses[0].startedAtMs = -1),
    (v) => (v.failures = []),
    (v) => v.failures.push(structuredClone(v.failures[0])),
    (v) => v.failures[0].witnesses.push(structuredClone(v.failures[0].witnesses[0])),
  ];
  for (const mutate of mutations) {
    const bad = structuredClone(value);
    mutate(bad);
    assert.throws(() => check(bad, auth));
  }
});
test('maintenance diagnostic rejects private fields and foreign or completed authentication stage', () => {
  const { auth, value } = fixture();
  for (const get of [(v) => v, (v) => v.failures[0], (v) => v.failures[0].witnesses[0]]) {
    const bad = structuredClone(value);
    get(bad).path = 'sentinel';
    assert.throws(() => check(bad, auth));
  }
  for (const mutate of [
    (a) => (a.binding.loaderId = 'foreign'),
    (a) => (a.backend.attempts[0].rootVerified = false),
    (a) => (a.backend.attempts[0].kind = 'accounts-refresh'),
    (a) => (a.backend.attempts[0].stages[1].outcome = 'completed'),
    (a) => (a.backend.path = 'sentinel'),
  ]) {
    const bad = structuredClone(auth);
    mutate(bad);
    assert.throws(() => check(value, bad));
  }
});
test('missing witness leaves cause unknown; successful or pending admission needs no failure', () => {
  const { auth, value } = fixture();
  value.failures[0].witnesses = [];
  for (const kind of [
    'operations-active',
    'directory-unavailable',
    'directory-busy',
    'unclassified',
  ]) {
    value.failures[0].errorClass = kind;
    assert.equal(check(value, auth), value);
  }
  value.failures = [];
  auth.backend.attempts[0].stages[1].outcome = 'completed';
  auth.backend.attempts[0].outcome = 'completed';
  assert.equal(check(value, auth), value);
  auth.backend.attempts[0].outcome = 'running';
  auth.backend.attempts[0].endedAtMs = null;
  auth.backend.attempts[0].stages[1].outcome = 'running';
  auth.backend.attempts[0].stages[1].endedAtMs = null;
  assert.equal(check(value, auth), value);
});
test('one actual witness must keep same metadata across failures; observations cannot extend beyond bounds', () => {
  const { auth, value } = fixture();
  const a = structuredClone(auth.backend.attempts[0]);
  a.id = 2;
  auth.backend.attempts.push(a);
  const f = structuredClone(value.failures[0]);
  f.attemptId = 2;
  value.failures.push(f);
  assert.equal(check(value, auth), value);
  value.failures[1].witnesses[0].kind = 'owned-blocking-worker';
  assert.throws(() => check(value, auth));
  value.failures[1].witnesses[0].kind = 'update-source-preferences';
  value.failures[1].witnesses[0].startedAtMs = 0.5;
  assert.throws(() => check(value, auth));
  const { auth: auth2, value: value2 } = fixture();
  value2.failures = Array.from({ length: 65 }, () => structuredClone(value2.failures[0]));
  assert.throws(() => check(value2, auth2));
  const { auth: auth3, value: value3 } = fixture();
  value3.failures[0].witnesses = Array.from({ length: 257 }, () =>
    structuredClone(value3.failures[0].witnesses[0]),
  );
  assert.throws(() => check(value3, auth3));
});

test('maximal bounded producer still fits its independent artifact byte limit', () => {
  const { auth, value } = fixture();
  value.registeredActivities = 256;
  const first = structuredClone(auth.backend.attempts[0]);
  auth.backend.attempts = Array.from({ length: 64 }, (_, i) => ({
    ...structuredClone(first),
    id: i + 1,
  }));
  value.failures = Array.from({ length: 64 }, (_, i) => ({
    attemptId: i + 1,
    stageToken: 1,
    atMs: 5,
    errorClass: 'operations-active',
    witnesses: Array.from({ length: 256 }, (_, n) => ({
      id: n + 1,
      kind: 'update-source-preferences',
      startedAtMs: 0.12345678901234566,
    })),
  }));
  assert.equal(check(value, auth), value);
  assert.ok(Buffer.byteLength(JSON.stringify(value, null, 2)) <= 4194304);
});
