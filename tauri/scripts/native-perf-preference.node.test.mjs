import test from 'node:test';
import assert from 'node:assert/strict';
import {
  checkPreferenceWrites,
  preferenceWriteDelta,
  correlatePreferenceInterval,
  correlateSettledThemeInterval,
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

const settling = () => ({
  before: snapshot([], 0),
  after: snapshot([row(1, 'theme', 1, 4), row(2, 'backup-reminder', 5, null)], 6),
  later: snapshot(
    [row(1, 'theme', 1, 4), row(2, 'backup-reminder', 5, 7), row(3, 'theme', 8, 9)],
    10,
  ),
});

test('later completion preserves the original running interval and independent transport total', () => {
  const { before, after, later } = settling();
  const original = structuredClone({ before, after, later });
  assert.throws(() => correlatePreferenceInterval(before, after, identity, 2));
  const result = correlateSettledThemeInterval(before, after, later, identity, 2);
  assert.equal(result.interval.total, 2);
  assert.equal(result.interval.theme, 1);
  assert.equal(result.interval.backupReminder, 1);
  assert.equal(result.interval.allCompleted, false);
  assert.equal(result.settledAttempts.length, 2);
  assert.equal(result.settledAttempts[1].endedAtMs, 7);
  assert.equal(result.laterAdded.total, 1);
  assert.equal(result.laterAdded.theme, 1);
  assert.equal(result.performanceMetrics, null);
  assert.deepEqual({ before, after, later }, original);
  assert.throws(() => correlateSettledThemeInterval(before, after, later, identity, 3));
  assert.throws(() => correlateSettledThemeInterval(before, after, later, identity, NaN));
});

test('theme must be exactly one completed target write at the original checkpoint', () => {
  for (const kind of ['theme', 'other']) {
    const after = snapshot([row(1, 'theme', 1, 2), row(2, kind, 3, 4)], 5);
    assert.throws(() =>
      correlateSettledThemeInterval(snapshot([], 0), after, { ...after, atMs: 6 }, identity, 2),
    );
  }
  const { before, after, later } = settling();
  after.attempts[0] = row(1, 'theme', 1, null);
  later.attempts[0] = row(1, 'theme', 1, 7);
  assert.throws(() => correlateSettledThemeInterval(before, after, later, identity, 2));
  const background = snapshot([row(1, 'backup-reminder')], 5);
  assert.throws(() =>
    correlateSettledThemeInterval(before, background, { ...background, atMs: 6 }, identity, 1),
  );
});

test('later evidence cannot rewrite terminal history or change running identity', () => {
  for (const mutate of [
    (v) => {
      v.runId = 'f'.repeat(32);
    },
    (v) => {
      v.pid++;
    },
    (v) => {
      v.clockScope = 'wall';
    },
    (v) => {
      v.atMs = 6;
    },
    (v) => {
      v.attempts[0].endedAtMs = 3;
    },
    (v) => {
      v.attempts[1].id = 3;
    },
    (v) => {
      v.attempts[1].kind = 'other';
    },
    (v) => {
      v.attempts[1].accountVerified = false;
    },
    (v) => {
      v.attempts[1].startedAtMs = 4.5;
    },
    (v) => {
      v.attempts[1].endedAtMs = 5.5;
    },
    (v) => {
      v.attempts[1].endedAtMs = null;
      v.attempts[1].outcome = 'running';
    },
    (v) => {
      v.attempts[1].outcome = 'interrupted';
    },
    (v) => {
      v.attempts = v.attempts.slice(0, 1);
    },
  ]) {
    const { before, after, later } = settling();
    mutate(later);
    assert.throws(() => correlateSettledThemeInterval(before, after, later, identity, 2));
  }
});

test('later additional writes never fill a missing original transport count', () => {
  const { before, after, later } = settling();
  after.attempts = after.attempts.slice(0, 1);
  assert.throws(() => correlateSettledThemeInterval(before, after, later, identity, 2));
  // A genuinely later background write is separately reported, never counted in the interval.
  later.attempts[1].startedAtMs = 6.5;
  const result = correlateSettledThemeInterval(before, after, later, identity, 1);
  assert.equal(result.interval.total, 1);
  assert.equal(result.laterAdded.total, 2);
});
