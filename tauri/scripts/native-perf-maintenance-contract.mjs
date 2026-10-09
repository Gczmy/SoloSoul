/** RF-312：普通认证行程的维护失败见证；先复核已有完整文档契约。 */
import { checkAuthAttribution } from './native-perf-auth-contract.mjs';
const exact = (v, keys) =>
  v &&
  typeof v === 'object' &&
  !Array.isArray(v) &&
  Object.keys(v).length === keys.length &&
  keys.every((k) => Object.hasOwn(v, k));
const time = (v, min, max) => Number.isFinite(v) && v >= min && v <= max;
const integer = (v, min, max) => Number.isSafeInteger(v) && v >= min && v <= max;
const fail = () => {
  throw Error('Maintenance admission diagnostic rejected');
};
export function checkMaintenanceAdmission(v, auth, owned, bound) {
  checkAuthAttribution(auth, owned, bound);
  const backend = auth.backend;
  if (
    !exact(v, [
      'schemaVersion',
      'scope',
      'runId',
      'pid',
      'clockScope',
      'atMs',
      'valid',
      'invalidReasons',
      'coverage',
      'maxActivities',
      'registeredActivities',
      'maxFailures',
      'failures',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'windows-native-maintenance-admission' ||
    v.runId !== auth.runId ||
    v.pid !== auth.pid ||
    v.clockScope !== 'process-monotonic' ||
    v.atMs !== backend.atMs ||
    v.valid !== true ||
    !Array.isArray(v.invalidReasons) ||
    v.invalidReasons.length ||
    !Array.isArray(v.coverage) ||
    v.coverage.length !== 2 ||
    v.coverage[0] !== 'update-source-preferences' ||
    v.coverage[1] !== 'owned-blocking-worker' ||
    v.maxActivities !== 256 ||
    v.maxFailures !== 64 ||
    !integer(v.registeredActivities, 0, 256) ||
    !Array.isArray(v.failures) ||
    v.failures.length > 64
  )
    fail();
  const seen = new Set(),
    known = new Map();
  let previous = 0;
  for (const f of v.failures) {
    if (
      !exact(f, ['attemptId', 'stageToken', 'atMs', 'errorClass', 'witnesses']) ||
      !integer(f.attemptId, 1, 64) ||
      !integer(f.stageToken, 0, 127) ||
      !['operations-active', 'directory-busy', 'directory-unavailable', 'unclassified'].includes(
        f.errorClass,
      )
    )
      fail();
    const key = `${f.attemptId}:${f.stageToken}`;
    if (seen.has(key)) fail();
    seen.add(key);
    const a = backend.attempts.find((a) => a.id === f.attemptId),
      stage = a?.stages[f.stageToken];
    if (
      !a ||
      a.kind !== 'login' ||
      a.rootVerified !== true ||
      !stage ||
      stage.name !== 'maintenance-admission' ||
      stage.outcome === 'completed' ||
      !time(f.atMs, Math.max(previous, stage.startedAtMs), stage.endedAtMs ?? v.atMs)
    )
      fail();
    previous = f.atMs;
    if (
      !Array.isArray(f.witnesses) ||
      f.witnesses.length > 256 ||
      (f.errorClass !== 'operations-active' && f.witnesses.length)
    )
      fail();
    let prior = 0;
    for (const w of f.witnesses) {
      if (
        !exact(w, ['id', 'kind', 'startedAtMs']) ||
        !integer(w.id, prior + 1, v.registeredActivities) ||
        !['update-source-preferences', 'owned-blocking-worker'].includes(w.kind) ||
        !time(w.startedAtMs, 0, stage.startedAtMs)
      )
        fail();
      prior = w.id;
      const old = known.get(w.id);
      if (old && (old.kind !== w.kind || old.startedAtMs !== w.startedAtMs)) fail();
      known.set(w.id, w);
    }
  }
  for (const a of backend.attempts)
    for (const [token, stage] of a.stages.entries()) {
      if (
        stage.name === 'maintenance-admission' &&
        stage.outcome === 'interrupted' &&
        !seen.has(`${a.id}:${token}`)
      )
        fail();
    }
  return v;
}
