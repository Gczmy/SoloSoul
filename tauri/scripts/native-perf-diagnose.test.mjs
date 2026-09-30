import assert from 'node:assert/strict';
import { test } from 'node:test';
import path from 'node:path';
import { mkdtemp, mkdir, writeFile, readFile, lstat, realpath, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { rejectDiagnosticBenchmark, NATIVE_PERF_MARKERS } from './native-perf-run.mjs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  parseDiagnosticArgs,
  helperEnvironment,
  checkLoggingMarker,
  chromiumLogBinaryPreflight,
  ordinaryTmpBinaryPreflight,
  selectedRuntimeBinaryPreflight,
  checkOrdinaryTmpMarker,
  diagnosticLaunchArgs,
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
      ordinaryTmp: false,
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

function canonicalTmpFixture() {
  const owned = {
    runId: 'a'.repeat(32),
    root: '\\\\?\\C:\\owned\\sample-001',
    profile: '\\\\?\\C:\\owned\\sample-001\\profile',
    webview: '\\\\?\\C:\\owned\\sample-001\\webview',
  };
  return {
    owned,
    value: {
      schemaVersion: 1,
      scope: 'windows-native-perf-ordinary-tmp',
      mode: 'ordinary-tmp',
      performanceSample: false,
      runId: owned.runId,
      pid: root.pid,
      port: 44001,
      root: owned.root,
      temp: path.win32.join(owned.root, 'temp'),
      tmp: 'C:\\owned\\sample-001\\temp',
      userProfile: owned.profile,
      webview: owned.webview,
    },
  };
}

test('ordinary native TMP requires explicit logging and rejects duplicate/value/unknown flags', () => {
  const base = path.resolve('owned');
  const args = [
    '--exe',
    path.join(base, 'app.exe'),
    '--fixture',
    path.join(base, 'fixture'),
    '--output',
    path.join(base, 'new'),
  ];
  assert.equal(parseDiagnosticArgs(args).ordinaryTmp, false);
  assert.equal(parseDiagnosticArgs([...args, '--chromium-log']).ordinaryTmp, false);
  for (const flags of [
    ['--ordinary-native-tmp'],
    ['--chromium-log', '--ordinary-native-tmp', '--ordinary-native-tmp'],
    ['--chromium-log', '--ordinary-native-tmp', 'C:\\another-temp'],
    ['--chromium-log', '--ordinary-native-tmp', '--ordinary-native-temp'],
  ])
    assert.throws(() => parseDiagnosticArgs([...args, ...flags]));
  for (const flags of [
    ['--chromium-log', '--ordinary-native-tmp'],
    ['--ordinary-native-tmp', '--chromium-log'],
  ]) {
    const parsed = parseDiagnosticArgs([...args, ...flags]);
    assert.equal(parsed.logging, true);
    assert.equal(parsed.ordinaryTmp, true);
  }
});

test('native launch keeps default/logging contracts and selects only the explicit ordinary-TMP mode', () => {
  const base = ['--native-perf-root', 'C:\\owned', '--native-perf-port', '44001'];
  assert.deepEqual(diagnosticLaunchArgs('C:\\owned', 44001, { ordinaryTmp: false }), base);
  assert.deepEqual(
    diagnosticLaunchArgs('C:\\owned', 44001, { logging: true, ordinaryTmp: false }),
    [...base, '--native-perf-diagnostics', 'chromium-log'],
  );
  assert.deepEqual(diagnosticLaunchArgs('C:\\owned', 44001, { logging: true, ordinaryTmp: true }), [
    ...base,
    '--native-perf-diagnostics',
    'chromium-log-ordinary-tmp',
  ]);
  assert.throws(() => diagnosticLaunchArgs('C:\\owned', 44001, { ordinaryTmp: true }));
});

test('ordinary TMP marker accepts only the same owned directory with all other canonical paths preserved', () => {
  const { value, owned } = canonicalTmpFixture();
  const before = JSON.stringify({ value, owned });
  assert.equal(checkOrdinaryTmpMarker(value, owned, root.pid, 44001), value);
  assert.equal(JSON.stringify({ value, owned }), before);
});

test('ordinary TMP marker rejects normalized equivalents for unchanged root/TEMP/profile/UDF', () => {
  const { value, owned } = canonicalTmpFixture();
  for (const key of ['root', 'temp', 'userProfile', 'webview']) {
    assert.throws(() =>
      checkOrdinaryTmpMarker({ ...value, [key]: value[key].slice(4) }, owned, root.pid, 44001),
    );
    assert.throws(() =>
      checkOrdinaryTmpMarker({ ...value, [key]: value[key].toLowerCase() }, owned, root.pid, 44001),
    );
  }
  for (const change of [
    { profile: owned.profile.slice(4) },
    { webview: owned.webview.slice(4) },
    { root: owned.root.slice(4) },
    { root: '\\\\?\\UNC\\server\\share' },
  ])
    assert.throws(() => checkOrdinaryTmpMarker(value, { ...owned, ...change }, root.pid, 44001));
});

test('ordinary TMP rejects extended, UNC, relative, external, nested and traversal directory representations', () => {
  const { value, owned } = canonicalTmpFixture();
  for (const tmp of [
    value.temp,
    '\\\\server\\share\\temp',
    'temp',
    'C:temp',
    '\\owned\\sample-001\\temp',
    'C:\\outside\\temp',
    value.tmp + '\\child',
    'C:\\owned\\sample-001\\webview\\..\\temp',
    value.tmp + '\\',
    value.tmp.replaceAll(String.fromCharCode(92), '/'),
    value.tmp.toLowerCase(),
    null,
  ])
    assert.throws(() => checkOrdinaryTmpMarker({ ...value, tmp }, owned, root.pid, 44001));
});

test('ordinary TMP marker rejects stale identity, changed diagnostic scope and fabricated performance acceptance', () => {
  const { value, owned } = canonicalTmpFixture();
  for (const change of [
    { schemaVersion: 2 },
    { scope: 'windows-native-perf-chromium-log' },
    { mode: 'ordinary-temp' },
    { runId: 'b'.repeat(32) },
    { pid: root.pid + 1 },
    { port: 44002 },
    { performanceSample: true },
    { temp: '\\\\?\\C:\\outside\\temp' },
    { userProfile: '\\\\?\\C:\\outside\\profile' },
    { webview: '\\\\?\\C:\\outside\\webview' },
  ])
    assert.throws(() => checkOrdinaryTmpMarker({ ...value, ...change }, owned, root.pid, 44001));
  assert.throws(() => checkOrdinaryTmpMarker(null, owned, root.pid, 44001));
  for (const key of Object.keys(value)) {
    const missing = { ...value };
    delete missing[key];
    assert.throws(() => checkOrdinaryTmpMarker(missing, owned, root.pid, 44001));
  }
});

async function withOrdinaryTmpStub(run) {
  const dir = await mkdtemp(path.join(tmpdir(), 'ss-rf312-ordinary-tmp-'));
  const resolvedParent = await realpath(tmpdir());
  try {
    const exe = path.join(dir, 'stub.exe');
    const fixture = path.join(dir, 'fixture');
    await mkdir(path.join(fixture, 'acc_rf312_100'), { recursive: true });
    const marker = {
      schemaVersion: 1,
      scope: 'synthetic-native-vault-fixture',
      generator: 'solosoul-core/examples/perf_baseline',
      fixture: 'deterministic-20th-object-property-match',
      objectCount: 100,
      accountId: 'acc_rf312_100',
      accountName: 'Performance Fixture',
      searchQuery: 'needle',
      expectedSearchMatches: 5,
      buildProfile: 'release',
      kdf: { memoryKiB: 65536, iterations: 3, parallelism: 4 },
      includesProfile: true,
      includesUiPreferences: true,
      includesAttachments: false,
      includesOcrFixture: false,
    };
    await writeFile(path.join(fixture, 'rf312-fixture.json'), JSON.stringify(marker));
    await writeFile(path.join(fixture, marker.accountId, 'vault.db'), 'synthetic placeholder');
    await writeFile(path.join(fixture, marker.accountId, 'config.json'), '{}');
    await writeFile(path.join(fixture, 'ui_preferences.json'), '{}');
    await run({ dir, exe, fixture, output: path.join(dir, 'new-results') });
  } finally {
    const resolvedDir = await realpath(dir);
    if (
      path.dirname(resolvedDir).toLowerCase() !== resolvedParent.toLowerCase() ||
      !path.basename(resolvedDir).startsWith('ss-rf312-ordinary-tmp-') ||
      (await lstat(dir)).isSymbolicLink()
    )
      throw new Error(
        'Refusing ordinary TMP test cleanup outside its exclusively created directory',
      );
    await rm(dir, { recursive: true, force: false });
  }
}

test('ordinary-TMP streaming marker rejects older binaries and matches across the 64KiB block boundary', async () => {
  await withOrdinaryTmpStub(async ({ exe }) => {
    await writeFile(exe, NATIVE_PERF_MARKERS.join('\0') + '\0windows-native-perf-chromium-log');
    await assert.rejects(ordinaryTmpBinaryPreflight(exe), /rebuilt.*ordinary-TMP feature marker/);
    const feature = 'windows-native-perf-ordinary-tmp';
    await writeFile(
      exe,
      Buffer.concat([Buffer.alloc(64 * 1024 - 9, 0x78), Buffer.from(feature), Buffer.from('tail')]),
    );
    const result = await ordinaryTmpBinaryPreflight(exe);
    assert.equal(result.method, 'streaming-ordinary-tmp-feature-marker');
    assert.match(result.limitation, /not a signature or trust guarantee/);
  });
});

test('older logging EXE is rejected before output creation or native preparation/GUI execution', async () => {
  await withOrdinaryTmpStub(async ({ exe, fixture, output }) => {
    const contents =
      'Never execute argument-only stub\0' +
      NATIVE_PERF_MARKERS.join('\0') +
      '\0windows-native-perf-chromium-log';
    await writeFile(exe, contents);
    const result = spawnSync(
      process.execPath,
      [
        fileURLToPath(new URL('./native-perf-diagnose.mjs', import.meta.url)),
        '--exe',
        exe,
        '--fixture',
        fixture,
        '--output',
        output,
        '--chromium-log',
        '--ordinary-native-tmp',
      ],
      { encoding: 'utf8', windowsHide: true, timeout: 10000 },
    );
    assert.equal(result.status, 1, result.stdout + result.stderr);
    assert.match(
      result.stderr,
      /Ordinary native TMP requires a rebuilt.*ordinary-TMP feature marker/,
    );
    assert.equal(result.stdout, '');
    await assert.rejects(lstat(output), { code: 'ENOENT' });
    assert.equal(await readFile(exe, 'utf8'), contents);
  });
});

test('runtime CLI requires source/version, logging and one experimental variable', () => {
  const base = path.resolve('owned');
  const required = [
    '--exe',
    path.join(base, 'app.exe'),
    '--fixture',
    path.join(base, 'fixture'),
    '--output',
    path.join(base, 'new'),
  ];
  const source = 'C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\Application\\153.0.4234.48';
  const pair = ['--runtime-source', source, '--runtime-version', '153.0.4234.48'];
  const valid = parseDiagnosticArgs([...required, '--chromium-log', ...pair]);
  assert.equal(valid.runtimeSource, path.resolve(source));
  assert.equal(valid.runtimeVersion, '153.0.4234.48');
  assert.equal(valid.ordinaryTmp, false);
  for (const extra of [
    pair,
    ['--chromium-log', '--runtime-source', source],
    ['--chromium-log', '--runtime-version', '153.0.4234.48'],
    ['--chromium-log', ...pair, '--ordinary-native-tmp'],
    ['--chromium-log', ...pair, '--runtime-source', source],
    ['--chromium-log', ...pair, '--runtime-version', '153.0.4234.48'],
    [
      '--chromium-log',
      '--runtime-source',
      '\\\\server\\share',
      '--runtime-version',
      '153.0.4234.48',
    ],
    ['--chromium-log', '--runtime-source', source, '--runtime-version', '0153.0.4234.48'],
    ['--chromium-log', '--runtime-source', source, '--runtime-version', '153.0.4234'],
    ['--chromium-log', '--runtime-source', source, '--runtime-version', '153.0.4234.65536'],
    ['--chromium-log', '--runtime-source', source, '--runtime-version', '153.0.4234.48 --argument'],
  ])
    assert.throws(() => parseDiagnosticArgs([...required, ...extra]));
});

test('runtime launch changes only explicit owned runtime selection and never permits ordinary TMP mixing', () => {
  const rootPath = 'C:\\owned\\sample-001';
  const base = ['--native-perf-root', rootPath, '--native-perf-port', '44001'];
  assert.deepEqual(diagnosticLaunchArgs(rootPath, 44001, { logging: true }), [
    ...base,
    '--native-perf-diagnostics',
    'chromium-log',
  ]);
  assert.deepEqual(
    diagnosticLaunchArgs(rootPath, 44001, { logging: true, runtimeSource: 'C:\\installed' }),
    [
      ...base,
      '--native-perf-diagnostics',
      'chromium-log',
      '--native-perf-runtime',
      path.win32.join(rootPath, 'runtime'),
    ],
  );
  assert.throws(() => diagnosticLaunchArgs(rootPath, 44001, { runtimeSource: 'C:\\installed' }));
  assert.throws(() =>
    diagnosticLaunchArgs(rootPath, 44001, {
      logging: true,
      runtimeSource: 'C:\\installed',
      ordinaryTmp: true,
    }),
  );
});

test('selected Runtime feature preflight rejects old TMP binaries and accepts split marker', async () => {
  await withOrdinaryTmpStub(async ({ exe }) => {
    await writeFile(
      exe,
      NATIVE_PERF_MARKERS.join('\0') +
        '\0windows-native-perf-chromium-log\0windows-native-perf-ordinary-tmp',
    );
    await assert.rejects(
      selectedRuntimeBinaryPreflight(exe),
      /rebuilt.*selected-Runtime feature marker/,
    );
    await writeFile(
      exe,
      Buffer.concat([
        Buffer.alloc(64 * 1024 - 7, 0x78),
        Buffer.from('windows-native-perf-selected-runtime'),
      ]),
    );
    const result = await selectedRuntimeBinaryPreflight(exe);
    assert.equal(result.method, 'streaming-selected-runtime-feature-marker');
  });
});

test('old EXE is rejected before Runtime inspection, copying, output creation or native execution', async () => {
  await withOrdinaryTmpStub(async ({ exe, fixture, output }) => {
    await writeFile(
      exe,
      NATIVE_PERF_MARKERS.join('\0') +
        '\0windows-native-perf-chromium-log\0windows-native-perf-ordinary-tmp',
    );
    const result = spawnSync(
      process.execPath,
      [
        fileURLToPath(new URL('./native-perf-diagnose.mjs', import.meta.url)),
        '--exe',
        exe,
        '--fixture',
        fixture,
        '--output',
        output,
        '--chromium-log',
        '--runtime-source',
        'C:\\nonexistent-runtime',
        '--runtime-version',
        '153.0.4234.48',
      ],
      { encoding: 'utf8', windowsHide: true, timeout: 10000 },
    );
    assert.equal(result.status, 1, result.stdout + result.stderr);
    assert.match(
      result.stderr,
      /Runtime selection requires a rebuilt.*selected-Runtime feature marker/,
    );
    assert.equal(result.stdout, '');
    await assert.rejects(lstat(output), { code: 'ENOENT' });
  });
});
