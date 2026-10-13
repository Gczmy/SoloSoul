// Fixed backend categories; frontend transport totals remain independently validated.
import assert from 'node:assert/strict';

const exact = (value, keys) =>
  value !== null &&
  typeof value === 'object' &&
  !Array.isArray(value) &&
  Object.keys(value).length === keys.length &&
  keys.every((key) => Object.hasOwn(value, key));
const integer = (value, low, high) => Number.isSafeInteger(value) && value >= low && value <= high;
const time = (value, low, high) => Number.isFinite(value) && value >= low && value <= high;
const reject = () => {
  throw Error('Preference write observation rejected');
};

export function checkPreferenceWrites(value, identity) {
  if (
    !identity ||
    !/^[a-f0-9]{32}$/.test(identity.runId) ||
    !integer(identity.pid, 1, 0xffffffff) ||
    !exact(value, [
      'schemaVersion',
      'scope',
      'runId',
      'pid',
      'clockScope',
      'atMs',
      'valid',
      'invalidReasons',
      'maxAttempts',
      'coverage',
      'attempts',
    ]) ||
    value.schemaVersion !== 1 ||
    value.scope !== 'windows-native-preference-writes' ||
    value.runId !== identity.runId ||
    value.pid !== identity.pid ||
    value.clockScope !== 'process-monotonic' ||
    !time(value.atMs, 0, 300000) ||
    value.valid !== true ||
    !Array.isArray(value.invalidReasons) ||
    value.invalidReasons.length ||
    value.maxAttempts !== 256 ||
    value.coverage !== 'user_data_update_preference' ||
    !Array.isArray(value.attempts) ||
    value.attempts.length > 256
  )
    reject();
  let previous = 0;
  for (const [index, row] of value.attempts.entries()) {
    if (
      !exact(row, ['id', 'kind', 'accountVerified', 'startedAtMs', 'endedAtMs', 'outcome']) ||
      row.id !== index + 1 ||
      !['theme', 'backup-reminder', 'other'].includes(row.kind) ||
      row.accountVerified !== true ||
      !time(row.startedAtMs, previous, value.atMs) ||
      !['running', 'completed', 'interrupted'].includes(row.outcome) ||
      (row.outcome === 'running'
        ? row.endedAtMs !== null
        : !time(row.endedAtMs, row.startedAtMs, value.atMs))
    )
      reject();
    previous = row.startedAtMs;
  }
  return value;
}

export function preferenceWriteDelta(before, after, identity) {
  checkPreferenceWrites(before, identity);
  checkPreferenceWrites(after, identity);
  if (after.atMs < before.atMs || after.attempts.length < before.attempts.length) reject();
  for (const [index, row] of before.attempts.entries()) {
    const next = after.attempts[index];
    if (row.outcome !== 'running') assert.deepEqual(next, row);
    else {
      for (const key of ['id', 'kind', 'accountVerified', 'startedAtMs'])
        if (row[key] !== next[key]) reject();
      if (next.outcome !== 'running' && next.endedAtMs < before.atMs) reject();
    }
  }
  const attempts = after.attempts.slice(before.attempts.length);
  if (attempts.some((row) => row.startedAtMs < before.atMs)) reject();
  return {
    attempts,
    total: attempts.length,
    theme: attempts.filter((row) => row.kind === 'theme').length,
    backupReminder: attempts.filter((row) => row.kind === 'backup-reminder').length,
    other: attempts.filter((row) => row.kind === 'other').length,
    allCompleted: attempts.every((row) => row.outcome === 'completed'),
    performanceMetrics: null,
  };
}

export function correlatePreferenceInterval(before, after, identity, transportAttempts) {
  const delta = preferenceWriteDelta(before, after, identity);
  if (
    !integer(transportAttempts, 0, 256) ||
    delta.total !== transportAttempts ||
    !delta.allCompleted
  )
    reject();
  return delta;
}

// A layout checkpoint can precede completion of a concurrent background write.
// Keep its transport/count boundary intact; later evidence may only finish the
// same immutable IDs. The target theme must already be complete at that boundary.
export function correlateSettledThemeInterval(before, after, later, identity, transportAttempts) {
  const interval = preferenceWriteDelta(before, after, identity);
  const laterAdded = preferenceWriteDelta(after, later, identity);
  if (
    !integer(transportAttempts, 0, 256) ||
    interval.total !== transportAttempts ||
    interval.theme !== 1 ||
    interval.other !== 0 ||
    later.atMs <= after.atMs ||
    interval.attempts.some((row) => row.kind === 'theme' && row.outcome !== 'completed')
  )
    reject();
  const settledAttempts = interval.attempts.map((row) => later.attempts[row.id - 1]);
  if (settledAttempts.some((row) => row.outcome !== 'completed')) reject();
  return {
    interval,
    settledAttempts,
    settlementAtMs: later.atMs,
    laterAdded,
    performanceMetrics: null,
  };
}
