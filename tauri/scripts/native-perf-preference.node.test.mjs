import test from 'node:test';
import assert from 'node:assert/strict';
import {
  checkPreferenceWrites,
  preferenceWriteDelta,
  correlatePreferenceInterval,
} from './native-perf-preference-contract.mjs';

const identity = { runId: '0123456789abcdef0123456789abcdef', pid: 41 };
const snapshot = (attempts = [], atMs = 10) => ({
  schemaVersion: 1,
  scope: 'windows-native-preference-writes',
  ...identity,
  clockScope: 'process-monotonic',
  atMs,
  valid: true,
  invalidReasons: [],
  maxAttempts: 256,
  coverage: 'user_data_update_preference',
  attempts,
});
const row = (id = 1, kind = 'theme', startedAtMs = 1, endedAtMs = 2) => ({
  id,
  kind,
  accountVerified: true,
  startedAtMs,
  endedAtMs,
  outcome: endedAtMs === null ? 'running' : 'completed',
});

test('concurrent theme and reminder writes remain two transport calls and separate categories', () => {
  const before = snapshot([], 0);
  const after = snapshot([row(1, 'theme', 1, 4), row(2, 'backup-reminder', 2, 3)]);
  const delta = correlatePreferenceInterval(before, after, identity, 2);
  assert.equal(delta.total, 2);
  assert.equal(delta.theme, 1);
  assert.equal(delta.backupReminder, 1);
  assert.equal(delta.other, 0);
  assert.equal(delta.performanceMetrics, null);
  assert.throws(() => correlatePreferenceInterval(before, after, identity, 1));
});

test('foreign identity, unknown fields, categories, clocks and coverage are rejected', () => {
  const source = snapshot([row()]);
  assert.equal(checkPreferenceWrites(source, identity), source);
  for (const mutate of [
    (v) => {
      v.runId = 'f'.repeat(32);
    },
    (v) => {
      v.pid++;
    },
    (v) => {
      v.valid = false;
    },
    (v) => {
      v.invalidReasons = ['attempt-overflow'];
    },
    (v) => {
      v.maxAttempts = 257;
    },
    (v) => {
      v.clockScope = 'wall';
    },
    (v) => {
      v.atMs = -1;
    },
    (v) => {
      v.coverage = 'all';
    },
    (v) => {
      v.payload = 'sentinel';
    },
    (v) => {
      v.attempts[0].value = 'sentinel';
    },
    (v) => {
      v.attempts[0].kind = 'lastBackupReminderAt';
    },
    (v) => {
      v.attempts[0].accountVerified = false;
    },
    (v) => {
      v.attempts[0].id = 2;
    },
    (v) => {
      v.attempts[0].startedAtMs = 11;
    },
    (v) => {
      v.attempts[0].endedAtMs = 0;
    },
    (v) => {
      v.attempts[0].endedAtMs = 11;
    },
    (v) => {
      v.attempts[0].outcome = 'running';
    },
    (v) => {
      v.attempts[0].outcome = 'failed-text';
    },
  ]) {
    const value = structuredClone(source);
    mutate(value);
    assert.throws(() => checkPreferenceWrites(value, identity));
  }
});

test('completed history is immutable; running history may finish after the prior checkpoint', () => {
  const before = snapshot([row(1, 'theme', 1, null)], 2);
  const after = snapshot([row(1, 'theme', 1, 3), row(2, 'backup-reminder', 4, 5)], 6);
  assert.equal(preferenceWriteDelta(before, after, identity).backupReminder, 1);
  for (const mutate of [
    (v) => {
      v.attempts[0].kind = 'other';
    },
    (v) => {
      v.attempts[0].endedAtMs = 1.5;
    },
    (v) => {
      v.attempts[1].startedAtMs = 1.5;
    },
    (v) => {
      v.atMs = 1;
    },
    (v) => {
      v.attempts = [];
    },
  ]) {
    const value = structuredClone(after);
    mutate(value);
    assert.throws(() => preferenceWriteDelta(before, value, identity));
  }
  const terminal = snapshot([row()], 3);
  const changed = structuredClone(terminal);
  changed.attempts[0].outcome = 'interrupted';
  assert.throws(() => preferenceWriteDelta(terminal, changed, identity));
});

test('unknown, interrupted and incomplete intervals cannot become successful theme evidence', () => {
  const before = snapshot([], 0);
  assert.equal(preferenceWriteDelta(before, snapshot([row(1, 'other')]), identity).other, 1);
  const interrupted = row();
  interrupted.outcome = 'interrupted';
  assert.throws(() => correlatePreferenceInterval(before, snapshot([interrupted]), identity, 1));
  assert.throws(() =>
    correlatePreferenceInterval(before, snapshot([row(1, 'theme', 1, null)]), identity, 1),
  );
  assert.throws(() => correlatePreferenceInterval(before, snapshot([row()]), identity, 2));
});

test('bounds and sequential IDs are checked for every row', () => {
  const rows = Array.from({ length: 256 }, (_, index) => row(index + 1, 'other', index, index));
  checkPreferenceWrites(snapshot(rows, 256), identity);
  assert.throws(() =>
    checkPreferenceWrites(snapshot([...rows, row(257, 'other', 256, 256)], 256), identity),
  );
  const repeated = snapshot([row(1), row(1)]);
  assert.throws(() => checkPreferenceWrites(repeated, identity));
});
