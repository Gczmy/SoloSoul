/** RF-312：同一owned文档的认证诊断；两类时钟分开，只有无并发歧义才按命令顺序配对。 */
const exact = (v, keys) =>
  v &&
  typeof v === 'object' &&
  !Array.isArray(v) &&
  Object.keys(v).length === keys.length &&
  keys.every((k) => Object.hasOwn(v, k));
const time = (v, min = 0, max = Number.MAX_VALUE) => Number.isFinite(v) && v >= min && v <= max;
const steps = Object.freeze({
  started: ['login-await-start'],
  'login-await-start': ['login-await-ok', 'login-await-error'],
  'login-await-error': ['failed'],
  'login-await-ok': ['accounts-await-start', 'failed'],
  'accounts-await-start': ['accounts-await-ok', 'accounts-await-error'],
  'accounts-await-ok': ['state-set-start', 'failed'],
  'accounts-await-error': ['state-set-start', 'failed'],
  'state-set-start': ['state-set-done', 'failed'],
  'state-set-done': ['finished', 'failed'],
  finished: [],
  failed: [],
});
const stages = new Set([
  'root-read',
  'maintenance-admission',
  'sync-disable',
  'blocking-queue',
  'worker-vault-read',
  'core-unlock',
  'audit-call',
  'cleanup-dispatch',
  'accounts-list',
  'recovery',
  'precheck',
  'master-config',
  'kdf',
  'verify',
  'account-write-call',
  'kdf-upgrade',
  'vault-open',
  'session-publish',
  'pin-reset-call',
]);
const commandKind = Object.freeze({ login: 'login', vault_list_accounts: 'accounts-refresh' });
function reject(message) {
  throw Error(message);
}
function ended(v, max) {
  return v.outcome === 'running'
    ? v.endedAtMs === null
    : ['completed', 'interrupted'].includes(v.outcome) && time(v.endedAtMs, v.startedAtMs, max);
}
export function checkAuthAttribution(v, owned, bound) {
  if (
    !exact(v, [
      'schemaVersion',
      'scope',
      'runId',
      'pid',
      'binding',
      'timeOriginMs',
      'frontend',
      'backend',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'windows-native-auth-attribution' ||
    !/^[a-f0-9]{32}$/.test(v.runId) ||
    v.runId !== owned.runId ||
    v.runId !== bound.runId ||
    v.pid !== bound.pid ||
    !Number.isSafeInteger(v.pid) ||
    v.pid <= 0 ||
    v.timeOriginMs !== bound.timeOriginMs ||
    !time(v.timeOriginMs, 1)
  )
    reject('Authentication attribution identity rejected');
  const b = v.binding,
    expected = bound.binding;
  if (
    !exact(b, [
      'source',
      'mainFrameId',
      'loaderId',
      'timeOriginMs',
      'navigationEvents',
      'frameCreatedEvents',
    ]) ||
    !expected ||
    b.mainFrameId !== expected.mainFrameId ||
    b.loaderId !== expected.loaderId ||
    ![
      'http://tauri.localhost/',
      'http://tauri.localhost/login',
      'http://tauri.localhost/objects',
      'http://tauri.localhost/search',
    ].includes(b.source) ||
    b.timeOriginMs !== null ||
    b.navigationEvents !== 0 ||
    b.frameCreatedEvents !== 0
  )
    reject('Authentication attribution document rejected');
  const f = v.frontend,
    n = v.backend;
  if (
    !exact(f, [
      'schemaVersion',
      'scope',
      'runId',
      'timeOriginMs',
      'atMs',
      'valid',
      'invalidReasons',
      'invokeCount',
      'maxReplies',
      'maxFlows',
      'replies',
      'flows',
    ]) ||
    f.schemaVersion !== 1 ||
    f.scope !== 'windows-native-auth-observer' ||
    f.runId !== v.runId ||
    f.timeOriginMs !== v.timeOriginMs ||
    f.valid !== true ||
    !Array.isArray(f.invalidReasons) ||
    f.invalidReasons.length ||
    !time(f.atMs) ||
    !Number.isSafeInteger(f.invokeCount) ||
    f.invokeCount < 0 ||
    f.invokeCount > 16384 ||
    f.maxReplies !== 128 ||
    f.maxFlows !== 64 ||
    !Array.isArray(f.replies) ||
    f.replies.length > 128 ||
    !Array.isArray(f.flows) ||
    f.flows.length > 64
  )
    reject('Authentication frontend schema or clock rejected');
  let lastSeq = 0;
  for (const reply of f.replies) {
    if (
      !exact(reply, ['invokeSeq', 'command', 'startedAtMs', 'headersAtMs', 'status']) ||
      !Number.isSafeInteger(reply.invokeSeq) ||
      reply.invokeSeq <= lastSeq ||
      reply.invokeSeq > f.invokeCount ||
      !Object.hasOwn(commandKind, reply.command) ||
      !time(reply.startedAtMs, 0, f.atMs) ||
      (reply.status === 'pending'
        ? reply.headersAtMs !== null
        : !['tauri-ok', 'tauri-error', 'transport-error', 'sync-throw'].includes(reply.status) ||
          !time(reply.headersAtMs, reply.startedAtMs, f.atMs))
    )
      reject('Authentication response record rejected');
    lastSeq = reply.invokeSeq;
  }
  for (const [index, flow] of f.flows.entries()) {
    if (
      !exact(flow, ['id', 'events']) ||
      flow.id !== index + 1 ||
      !Array.isArray(flow.events) ||
      !flow.events.length ||
      flow.events.length > 16
    )
      reject('Authentication flow schema rejected');
    let prior = null,
      last = 0;
    for (const event of flow.events) {
      if (
        !exact(event, ['stage', 'atMs']) ||
        !time(event.atMs, last, f.atMs) ||
        (prior === null ? event.stage !== 'started' : !steps[prior]?.includes(event.stage))
      )
        reject('Authentication flow transition rejected');
      prior = event.stage;
      last = event.atMs;
    }
  }
  if (
    !exact(n, [
      'schemaVersion',
      'scope',
      'runId',
      'clockScope',
      'atMs',
      'valid',
      'invalidReasons',
      'maxAttempts',
      'maxStages',
      'attempts',
    ]) ||
    n.schemaVersion !== 1 ||
    n.scope !== 'windows-native-auth-backend' ||
    n.runId !== v.runId ||
    n.clockScope !== 'process-monotonic' ||
    n.valid !== true ||
    !Array.isArray(n.invalidReasons) ||
    n.invalidReasons.length ||
    !time(n.atMs) ||
    n.maxAttempts !== 64 ||
    n.maxStages !== 128 ||
    !Array.isArray(n.attempts) ||
    n.attempts.length > 64
  )
    reject('Authentication backend schema or clock rejected');
  for (const [index, attempt] of n.attempts.entries()) {
    if (
      !exact(attempt, [
        'id',
        'kind',
        'rootVerified',
        'startedAtMs',
        'endedAtMs',
        'outcome',
        'stages',
      ]) ||
      attempt.id !== index + 1 ||
      !['login', 'accounts-refresh'].includes(attempt.kind) ||
      attempt.rootVerified !== true ||
      !time(attempt.startedAtMs, 0, n.atMs) ||
      !ended(attempt, n.atMs) ||
      !Array.isArray(attempt.stages) ||
      attempt.stages.length > 128
    )
      reject('Authentication backend attempt rejected');
    const end = attempt.endedAtMs ?? n.atMs;
    let prior = attempt.startedAtMs;
    for (const stage of attempt.stages) {
      if (
        !exact(stage, ['name', 'startedAtMs', 'endedAtMs', 'outcome']) ||
        !stages.has(stage.name) ||
        !time(stage.startedAtMs, prior, end) ||
        !ended(stage, end) ||
        (attempt.outcome !== 'running' && stage.outcome === 'running') ||
        (attempt.kind === 'accounts-refresh' &&
          !['blocking-queue', 'worker-vault-read', 'accounts-list'].includes(stage.name)) ||
        (attempt.kind === 'login' && stage.name === 'accounts-list')
      )
        reject('Authentication backend stage rejected');
      prior = stage.startedAtMs;
    }
  }
  return v;
}
export function describeAuthAttribution(v, owned, bound) {
  checkAuthAttribution(v, owned, bound);
  const f = v.frontend,
    n = v.backend;
  const used = new Set();
  const flows = f.flows.map((flow) => {
    const awaits = [];
    for (const [prefix, command] of [
      ['login', 'login'],
      ['accounts', 'vault_list_accounts'],
    ]) {
      const start = flow.events.find((e) => e.stage === prefix + '-await-start');
      if (!start) continue;
      const end = flow.events.find((e) =>
        [prefix + '-await-ok', prefix + '-await-error'].includes(e.stage),
      );
      const candidates = f.replies.filter(
        (reply) =>
          reply.command === command &&
          reply.startedAtMs >= start.atMs &&
          reply.startedAtMs <= (end?.atMs ?? f.atMs),
      );
      if (candidates.length !== 1 || used.has(candidates[0].invokeSeq))
        reject('Authentication await to request is missing or ambiguous');
      const reply = candidates[0];
      used.add(reply.invokeSeq);
      if (
        end &&
        (reply.status === 'pending' ||
          reply.headersAtMs > end.atMs ||
          (end.stage.endsWith('-ok') && reply.status !== 'tauri-ok'))
      )
        reject('Authentication response and awaited outcome disagree');
      awaits.push({
        command,
        invokeSeq: reply.invokeSeq,
        status: reply.status,
        headersDelayMs: reply.headersAtMs === null ? null : reply.headersAtMs - reply.startedAtMs,
        promiseWaitMs: end ? end.atMs - start.atMs : null,
        afterHeadersToAwaitMs: end ? end.atMs - reply.headersAtMs : null,
      });
    }
    const start = flow.events.find((e) => e.stage === 'state-set-start'),
      end = flow.events.find((e) => e.stage === 'state-set-done');
    return {
      id: flow.id,
      outcome: flow.events.at(-1).stage,
      awaits,
      stateWriteMs: start && end ? end.atMs - start.atMs : null,
    };
  });
  const serialCommandPairing = Object.entries(commandKind).map(([command, kind]) => {
    const replies = f.replies.filter((r) => r.command === command),
      attempts = n.attempts.filter((a) => a.kind === kind);
    if (replies.length !== attempts.length)
      reject('Authentication frontend/backend call counts disagree');
    const serial = (rows, end) =>
      rows.every(
        (row, i) => !i || (rows[i - 1][end] !== null && rows[i - 1][end] <= row.startedAtMs),
      );
    const paired = serial(replies, 'headersAtMs') && serial(attempts, 'endedAtMs');
    return {
      command,
      paired,
      reason: paired ? null : 'concurrent-calls-prevent-ordinal-pairing',
      pairs: paired
        ? replies.map((reply, i) => ({ invokeSeq: reply.invokeSeq, attemptId: attempts[i].id }))
        : [],
    };
  });
  return {
    diagnosticOnly: true,
    performanceMetrics: null,
    frontend: { clockScope: 'document-performance', atMs: f.atMs, flows },
    backend: {
      clockScope: n.clockScope,
      atMs: n.atMs,
      attempts: n.attempts.map((a) => ({
        ...a,
        durationMs: a.endedAtMs === null ? null : a.endedAtMs - a.startedAtMs,
        stages: a.stages.map((s) => ({
          ...s,
          durationMs: s.endedAtMs === null ? null : s.endedAtMs - s.startedAtMs,
        })),
      })),
    },
    serialCommandPairing,
  };
}
