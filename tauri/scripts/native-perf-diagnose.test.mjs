import assert from 'node:assert/strict';
import { test } from 'node:test';
import path from 'node:path';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { rejectDiagnosticBenchmark } from './native-perf-run.mjs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  parseDiagnosticArgs,
  helperEnvironment,
  checkLoggingMarker,
  chromiumLogBinaryPreflight,
  diagnosticHelperFailure,
  checkConsumed,
  selectDiagnosticIdentities,
  checkDiagnosticResult,
  cdpProbe,
} from './native-perf-diagnose.mjs';

const root = {
  pid: 101,
  parentPid: 7,
  creationMs: 1000,
  executablePath: 'C:\\owned\\solo_soul.exe',
};
const browser = {
  pid: 102,
  parentPid: 101,
  creationMs: 1100,
  executablePath: 'C:\\runtime\\msedgewebview2.exe',
};
const identities = [root, browser];
const directory = 'C:\\owned\\sample-001\\webview\\EBWebView';
const owner = {
  expected: { pid: root.pid, exe: root.executablePath },
  known: new Map(identities.map((p) => [p.pid + ':' + p.creationMs, p])),
};
function snapshot() {
  return {
    userDataDirectoryMatches: true,
    browserDataChecks: [{ pid: browser.pid, matchesExpected: true }],
    processes: structuredClone(identities),
  };
}
function diagnostic() {
  return {
    schemaVersion: 1,
    scope: 'windows-native-perf-process-diagnostics',
    mode: 'live',
    success: true,
    processes: identities.map((p) => ({
      ...p,
      identityMatched: true,
      browserFlags:
        p.pid === browser.pid
          ? {
              port: 44001,
              address: '127.0.0.1',
              userDataDirectoryMatched: true,
              observedDirectory: directory,
            }
          : {},
    })),
    listeners: [],
  };
}

test('diagnostic CLI accepts exactly three absolute paths and rejects benchmark/control overrides', () => {
  const base = path.resolve('owned');
  assert.deepEqual(
    parseDiagnosticArgs([
      '--exe',
      path.join(base, 'app.exe'),
      '--fixture',
      path.join(base, 'fixture'),
      '--output',
      path.join(base, 'new'),
    ]),
    {
      exe: path.join(base, 'app.exe'),
      fixture: path.join(base, 'fixture'),
      output: path.join(base, 'new'),
    },
  );
  for (const args of [
    [],
    ['--exe', 'relative'],
    ['--exe', base, '--exe', base],
    ['--samples', '1'],
    ['--fixture', base, '--output', base, '--exe', path.parse(base).root, '--password', 'anything'],
  ])
    assert.throws(() => parseDiagnosticArgs(args));
  assert.throws(() =>
    parseDiagnosticArgs(['--exe', base, '--fixture', base, '--output', path.parse(base).root]),
  );
});

test('help exits without a preparation or GUI process', () => {
  const result = spawnSync(
    process.execPath,
    [fileURLToPath(new URL('./native-perf-diagnose.mjs', import.meta.url)), '--help'],
    { encoding: 'utf8', windowsHide: true },
  );
  assert.equal(result.status, 0);
  assert.match(result.stdout, /No password\/UI actions or performance metrics/);
  assert.equal(result.stderr, '');
});

test('consumed marker must match exact run, PID, port and owned root', () => {
  const owned = { runId: 'a'.repeat(32), root: 'C:\\owned\\sample-001' };
  const value = {
    schemaVersion: 1,
    scope: 'windows-native-perf-consumed',
    runId: owned.runId,
    root: owned.root,
    pid: root.pid,
    port: 44001,
  };
  assert.equal(checkConsumed(value, owned, root.pid, 44001), value);
  for (const change of [
    { pid: 999 },
    { port: 9 },
    { runId: 'b'.repeat(32) },
    { root: 'C:\\other' },
    { scope: 'ready' },
  ])
    assert.throws(() => checkConsumed({ ...value, ...change }, owned, root.pid, 44001));
});

test('only root and fresh journal-matched browser may be queried', () => {
  assert.deepEqual(selectDiagnosticIdentities(owner, snapshot()), identities);
  for (const bad of [
    { ...snapshot(), userDataDirectoryMatches: false },
    { ...snapshot(), browserDataChecks: [] },
    { ...snapshot(), browserDataChecks: [{ pid: 999, matchesExpected: true }] },
  ])
    assert.throws(() => selectDiagnosticIdentities(owner, bad));
  const reused = snapshot();
  reused.processes[1].creationMs++;
  assert.throws(() => selectDiagnosticIdentities(owner, reused));
  const duplicate = snapshot();
  duplicate.processes.push(duplicate.processes[0]);
  assert.throws(() => selectDiagnosticIdentities(owner, duplicate));
  const wrong = { ...owner, known: new Map(owner.known) };
  wrong.known.set('102:1100', { ...browser, parentPid: 999 });
  assert.throws(() => selectDiagnosticIdentities(wrong, snapshot()));
});

test('helper response rejects stale identities, unexpected browser UDF and foreign listeners', () => {
  const good = diagnostic();
  assert.equal(checkDiagnosticResult(good, identities, directory), good);
  for (const change of [
    { success: false },
    { mode: 'self-test' },
    { processes: [good.processes[0]] },
    { listeners: [{ owningPid: 900, localPort: 44001, localAddress: '127.0.0.1' }] },
    { listeners: [{ owningPid: browser.pid, localPort: 0, localAddress: '127.0.0.1' }] },
  ])
    assert.throws(() => checkDiagnosticResult({ ...good, ...change }, identities, directory));
  const reused = diagnostic();
  reused.processes[1].creationMs++;
  assert.throws(() => checkDiagnosticResult(reused, identities, directory));
  const outside = diagnostic();
  outside.processes[1].browserFlags.observedDirectory = 'C:\\user-vault';
  assert.throws(() => checkDiagnosticResult(outside, identities, directory));
});

test('no verified owned listener means no HTTP request', async () => {
  let called = 0;
  const request = async () => {
    called++;
    throw new Error('must not call');
  };
  const value = diagnostic();
  const result = await cdpProbe(value, identities, 44001, request);
  assert.equal(result.status, 'no-owned-listener');
  assert.equal(result.attempted, false);
  value.listeners = [
    { owningPid: root.pid, localPort: 44001, localAddress: '127.0.0.1' },
    { owningPid: browser.pid, localPort: 44002, localAddress: '127.0.0.1' },
  ];
  assert.equal((await cdpProbe(value, identities, 44001, request)).attempted, false);
  assert.equal(called, 0);
});

test('mismatched remote flags or non-loopback listener blocks HTTP', async () => {
  const value = diagnostic();
  value.listeners = [{ owningPid: browser.pid, localPort: 44001, localAddress: '192.0.2.1' }];
  let calls = 0;
  const request = async () => {
    calls++;
  };
  assert.equal(
    (await cdpProbe(value, identities, 44001, request)).status,
    'listener-or-flags-mismatch',
  );
  value.listeners[0].localAddress = '127.0.0.1';
  value.processes[1].browserFlags.port = 44002;
  assert.equal((await cdpProbe(value, identities, 44001, request)).attempted, false);
  assert.equal(calls, 0);
});

test('version endpoint result never claims CDP attach or GUI/performance success', async () => {
  const value = diagnostic();
  value.listeners = [{ owningPid: browser.pid, localPort: 44001, localAddress: '127.0.0.1' }];
  const result = await cdpProbe(value, identities, 44001, async (url, options) => {
    assert.equal(url, 'http://127.0.0.1:44001/json/version');
    assert.ok(options.signal);
    assert.equal(options.redirect, 'error');
    return {
      ok: true,
      json: async () => ({
        Browser: 'WebView2/154',
        'Protocol-Version': '1.3',
        webSocketDebuggerUrl: 'ws://secret/target',
        'V8-Version': 'v8',
      }),
    };
  });
  assert.equal(result.status, 'http-version-available');
  assert.match(result.limitation, /no CDP target/);
  assert.equal(result.versions.Browser, 'WebView2/154');
  assert.equal(JSON.stringify(result).includes('secret'), false);
  assert.equal(Object.hasOwn(result, 'performanceMetrics'), false);
});

test('HTTP failure preserves diagnostic failure without fabricated version', async () => {
  const value = diagnostic();
  value.listeners = [{ owningPid: browser.pid, localPort: 44001, localAddress: '127.0.0.1' }];
  const failed = await cdpProbe(value, identities, 44001, async () => {
    throw new Error('refused');
  });
  assert.equal(failed.status, 'request-failed');
  assert.equal(failed.reason, 'refused');
  assert.equal(failed.versions, undefined);
  const nonOk = await cdpProbe(value, identities, 44001, async () => ({ ok: false, status: 403 }));
  assert.equal(nonOk.status, 'http-error');
});

test('helper rejection preserves only fixed-schema reason, never raw stdout', () => {
  const rejected = {
    stdout: JSON.stringify({
      schemaVersion: 1,
      scope: 'windows-native-perf-process-diagnostics',
      mode: 'live',
      success: false,
      diagnosticStage: 'fresh-identity',
      reason: 'Owned root exited',
      secret: 'DO-NOT-LOG',
    }),
    stderr: '',
    code: 1,
  };
  const message = diagnosticHelperFailure(rejected);
  assert.match(message, /fresh-identity: Owned root exited/);
  assert.equal(message.includes('DO-NOT-LOG'), false);
  assert.equal(
    diagnosticHelperFailure({ stdout: 'DO-NOT-LOG', stderr: '', code: 1 }).includes('DO-NOT-LOG'),
    false,
  );
});

test('listener query failure is unknown, not proof of absence', async () => {
  const value = diagnostic();
  value.listenersReason = 'TCP table unavailable';
  let called = false;
  const result = await cdpProbe(value, identities, 44001, async () => {
    called = true;
  });
  assert.equal(result.status, 'listener-query-unavailable');
  assert.equal(result.reason, value.listenersReason);
  assert.equal(result.attempted, false);
  assert.equal(called, false);
});

test('logging is explicit and duplicate CLI paths/flags are refused', () => {
  const base = path.resolve('owned');
  const args = [
    '--exe',
    path.join(base, 'app.exe'),
    '--fixture',
    path.join(base, 'fixture'),
    '--output',
    path.join(base, 'new'),
  ];
  assert.equal(parseDiagnosticArgs([...args, '--chromium-log']).logging, true);
  assert.throws(() => parseDiagnosticArgs([...args, '--exe', path.join(base, 'other.exe')]));
  assert.throws(() => parseDiagnosticArgs([...args, '--chromium-log', '--chromium-log']));
  assert.throws(() => parseDiagnosticArgs([...args, '--chromium-log', 'arbitrary-log-path']));
});

test('logging marker rejects another run/port, extended path and non-owned log', () => {
  const owned = { runId: 'a'.repeat(32), root: 'C:\\owned\\sample-001' };
  const marker = {
    schemaVersion: 1,
    scope: 'windows-native-perf-chromium-log',
    mode: 'chromium-log',
    root: owned.root,
    runId: owned.runId,
    pid: root.pid,
    port: 44001,
    nativeTempUnchanged: true,
    performanceSample: false,
    logFile: path.win32.join(owned.root, 'temp', 'chromium-diagnostics.log'),
  };
  assert.equal(checkLoggingMarker(marker, owned, root.pid, 44001), marker);
  for (const change of [
    { logFile: 'C:\\outside\\chromium-diagnostics.log' },
    { logFile: '\\\\?\\' + marker.logFile },
    { runId: 'b'.repeat(32) },
    { port: 44002 },
    { pid: 999 },
    { performanceSample: true },
    { nativeTempUnchanged: false },
  ])
    assert.throws(() => checkLoggingMarker({ ...marker, ...change }, owned, root.pid, 44001));
});

test('old executable marker is rejected offline; diagnostic marker prevents benchmark acceptance', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'ss-rf312-node-log-'));
  try {
    const exe = path.join(dir, 'fixture.exe');
    await writeFile(exe, 'windows-native-perf-owned old build');
    await assert.rejects(chromiumLogBinaryPreflight(exe), /rebuilt/);
    await writeFile(exe, 'prefix windows-native-perf-chromium-log suffix');
    const accepted = await chromiumLogBinaryPreflight(exe);
    assert.equal(accepted.method, 'streaming-logging-feature-marker');
    await rejectDiagnosticBenchmark(dir);
    await writeFile(path.join(dir, 'native-perf-chromium-log.json'), '{}');
    await assert.rejects(rejectDiagnosticBenchmark(dir), /cannot be used as a performance sample/);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test('off-mode helper environment deletes inherited logging even with case aliases', () => {
  const inherited = {
    SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_LOG_FILE: 'C:\\outside',
    solosoul_native_perf_diagnostics_log_file: '',
    SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_MODE: 'self-test',
    Temp: 'outside',
    tMp: 'outside',
    PATH: 'preserve',
  };
  const owned = { webview: 'C:\\owned\\sample-001\\webview' };
  const env = helperEnvironment(identities, owned, 'C:\\owned\\temp', null, inherited);
  assert.equal(
    Object.keys(env).some((k) => k.toUpperCase() === 'SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_LOG_FILE'),
    false,
  );
  assert.equal(env.SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_MODE, 'live');
  assert.equal(env.TEMP, 'C:\\owned\\temp');
  assert.equal(env.TMP, env.TEMP);
  assert.equal(env.PATH, 'preserve');
  assert.equal(inherited.Temp, 'outside');
  assert.equal(inherited.SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_MODE, 'self-test');
  const logging = { logFile: 'C:\\owned\\temp\\chromium-diagnostics.log' };
  const on = helperEnvironment(identities, owned, env.TEMP, logging, inherited);
  assert.equal(
    Object.keys(on).filter((k) => k.toUpperCase() === 'SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_LOG_FILE')
      .length,
    1,
  );
  assert.equal(on.SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_LOG_FILE, logging.logFile);
});
