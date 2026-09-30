import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  rm,
  realpath,
  symlink,
  lstat,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  parseArgs,
  validateInputs,
  validateFixtureManifest,
  normalizeWindowsPath,
  validateObserver,
  ipcDelta,
  documentIntegrity,
  verifiedOwnedRows,
  preparedManifest,
  safeError,
  summarize,
  PROCESS_SCRIPT,
  cleanupOutcome,
  nativePerfPreflight,
  NATIVE_PERF_MARKERS,
  browserDataDirectoryCheck,
  browserDataFilesystemCheck,
  powerShellError,
  releaseUnfinishedChild,
  rejectDiagnosticBenchmark,
} from './native-perf-run.mjs';

const runId = '0123456789abcdef0123456789abcdef';
const marker = () => ({
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
});
const observation = (commands) => ({
  schemaVersion: 1,
  scope: 'windows-native-tauri-invoke-observer',
  runId,
  valid: true,
  invalidReasons: [],
  total: commands.length,
  observedCount: commands.length,
  installedAtMs: 1.25,
  timeOriginMs: 12345,
  commands: commands.map((command, index) => ({ command, atMs: index + 2 })),
});

test('arguments require absolute explicit paths, unique options and at least three samples', () => {
  const root = path.resolve(tmpdir(), 'rf312-argument-only');
  const valid = [
    '--exe',
    path.join(root, 'app.exe'),
    '--fixture',
    path.join(root, 'fixture'),
    '--output',
    path.join(root, 'results'),
    '--samples',
    '5',
  ];
  assert.equal(parseArgs(valid).samples, 5);
  assert.deepEqual(parseArgs(['--help']), { help: true });
  for (const args of [
    [],
    valid.slice(0, -2),
    [...valid, '--exe', 'extra.exe'],
    [...valid, '--unexpected', 'x'],
    [...valid.slice(0, -1), '2'],
    [...valid.slice(0, -1), '3.5'],
    [...valid.slice(0, -1), 'Infinity'],
    ['--exe', 'relative.exe', ...valid.slice(2)],
    ['--exe', valid[1], '--fixture', valid[3], '--output', path.parse(root).root, '--samples', '3'],
  ])
    assert.throws(() => parseArgs(args));
});

test('only Release synthetic fixture scope with production KDF and exact public account is accepted', () => {
  assert.equal(validateFixtureManifest(marker()).accountId, 'acc_rf312_100');
  const large = {
    ...marker(),
    objectCount: 5000,
    accountId: 'acc_rf312_5000',
    expectedSearchMatches: 250,
  };
  assert.equal(validateFixtureManifest(large).objectCount, 5000);
  for (const change of [
    { scope: 'real-user-vault' },
    { accountId: 'real-account' },
    { accountName: 'Personal' },
    { buildProfile: 'debug' },
    { kdf: { memoryKiB: 8192, iterations: 2, parallelism: 4 } },
    { expectedSearchMatches: 250 },
    { includesAttachments: true },
    { objectCount: 5010 },
  ])
    assert.throws(() => validateFixtureManifest({ ...marker(), ...change }));
});

test('Windows canonical prefixes fold only equivalent drive and UNC paths', () => {
  assert.equal(
    normalizeWindowsPath('\\\\?\\C:\\TEMP\\Sample'),
    normalizeWindowsPath('c:\\temp\\sample'),
  );
  assert.equal(
    normalizeWindowsPath('\\\\?\\UNC\\server\\share\\Sample'),
    normalizeWindowsPath('\\\\server\\share\\sample'),
  );
  assert.notEqual(
    normalizeWindowsPath('C:\\TEMP\\Sample'),
    normalizeWindowsPath('C:\\TEMP\\Other'),
  );
  assert.throws(() => normalizeWindowsPath('relative'));
});

test('prepared native manifest matches nested fixture, unique identifier and exact owned paths', () => {
  const owned = {
    schemaVersion: 1,
    scope: 'windows-native-perf-owned',
    nativePerfFeature: true,
    preparationStatus: 'ready',
    appVersion: '2.13.2',
    runId,
    identifier: 'com.solosoul.rf312perf.' + runId,
    root: '\\\\?\\C:\\owned\\sample',
    webview: '\\\\?\\C:\\owned\\sample\\webview',
    vault: '\\\\?\\C:\\owned\\sample\\vault',
    fixtureSource: '\\\\?\\C:\\fixtures\\vault100',
    fixture: { objectCount: 100, accountId: 'acc_rf312_100' },
  };
  assert.equal(
    preparedManifest(owned, 'C:\\owned\\sample', marker(), 'C:\\fixtures\\vault100'),
    owned,
  );
  for (const change of [
    { identifier: 'com.solosoul.app' },
    { nativePerfFeature: false },
    { runId: 'wrong' },
    { preparationStatus: 'preparing' },
    { appVersion: undefined },
    { root: 'C:\\other' },
    { webview: 'C:\\user-data' },
    { vault: 'C:\\user-vault' },
    { fixture: { objectCount: 5000, accountId: 'acc_rf312_5000' } },
    { fixtureSource: 'C:\\real-user-vault' },
  ])
    assert.throws(() =>
      preparedManifest(
        { ...owned, ...change },
        'C:\\owned\\sample',
        marker(),
        'C:\\fixtures\\vault100',
      ),
    );
});

test('observer counts actual command attempts including plugin calls and removes unexpected payload fields', () => {
  const value = observation(['login', 'plugin:os|platform', 'login']);
  value.commands[0].payload = { password: 'synthetic-sensitive-input' };
  value.response = { privateContent: 'synthetic-private-result' };
  const valid = validateObserver(value, runId);
  assert.equal(valid.valid, true);
  const empty = validateObserver(observation([]), runId);
  assert.deepEqual(ipcDelta(empty, valid).commands, [
    { command: 'login', attempts: 2 },
    { command: 'plugin:os|platform', attempts: 1 },
  ]);
  const serialized = JSON.stringify(valid);
  assert.doesNotMatch(
    serialized,
    /payload|response|password|privateContent|synthetic-sensitive-input/,
  );
});

test('identity mismatch, fallback, overflow, missing fields and reset produce null counts with reason', () => {
  const before = validateObserver(observation(['login']), runId);
  for (const change of [
    { runId: 'another-run' },
    { scope: 'mock' },
    { valid: false, total: null, invalidReasons: ['transport-fallback'] },
    { valid: false, total: null, invalidReasons: ['overflow'] },
    { total: 12 },
    { timeOriginMs: undefined },
    { commands: [{ command: 'invalid command', atMs: 2 }] },
  ]) {
    const result = validateObserver({ ...observation(['login']), ...change }, runId);
    assert.equal(result.valid, false);
    assert.equal(ipcDelta(before, result).attempts, null);
    assert.equal(typeof result.reason, 'string');
  }
  const reloaded = validateObserver(
    { ...observation(['login', 'lock']), timeOriginMs: 9876 },
    runId,
  );
  assert.equal(ipcDelta(before, reloaded).attempts, null);
  const changedPrefix = observation(['other', 'lock']);
  assert.equal(ipcDelta(before, validateObserver(changedPrefix, runId)).attempts, null);
});

test('single original document permits SPA but extra pages, frames and new documents invalidate observation', () => {
  const stable = {
    pageCount: 1,
    frameCount: 1,
    currentTimeOrigin: 12345,
    observerTimeOrigin: 12345,
  };
  assert.deepEqual(documentIntegrity(stable, 12345), []);
  for (const change of [
    { pageCount: 2 },
    { pageCount: 0 },
    { frameCount: 2 },
    { currentTimeOrigin: 9876 },
    { observerTimeOrigin: 9876 },
  ])
    assert.ok(documentIntegrity({ ...stable, ...change }, 12345).length > 0);
});

test('owned process selection excludes other SoloSoul instances, orphan WebViews and PID identity changes', () => {
  const expected = { pid: 100, exe: 'C:\\isolated\\solo_soul.exe', startedAt: 100000 };
  const row = (pid, parentPid, creationMs, executablePath) => ({
    pid,
    parentPid,
    creationMs,
    executablePath,
  });
  const rows = [
    row(100, 77, 100010, expected.exe),
    row(101, 100, 100011, 'C:\\runtime\\msedgewebview2.exe'),
    row(102, 101, 100012, 'C:\\runtime\\msedgewebview2.exe'),
    row(200, 77, 100005, expected.exe),
    row(201, 200, 100015, 'C:\\runtime\\msedgewebview2.exe'),
    row(300, 999, 100020, 'C:\\runtime\\msedgewebview2.exe'),
    row(301, 100, 99999, 'C:\\runtime\\msedgewebview2.exe'),
  ];
  assert.deepEqual(
    verifiedOwnedRows({ rootVerified: true, processes: rows }, expected).map((x) => x.pid),
    [100, 101, 102],
  );
  assert.deepEqual(
    verifiedOwnedRows({ rootVerified: true, processes: rows }, { ...expected, startedAt: 200000 }),
    [],
  );
  assert.deepEqual(
    verifiedOwnedRows(
      { rootVerified: true, processes: rows },
      { ...expected, exe: 'C:\\other.exe' },
    ),
    [],
  );
  assert.deepEqual(verifiedOwnedRows({ rootVerified: false, processes: rows }, expected), []);
});

test('errors redact the fixed synthetic password and remove Playwright call logs', () => {
  const error = new Error(
    'fill failed perf-baseline-only-password\nCall log:\n- fill("perf-baseline-only-password")',
  );
  const text = safeError(error);
  assert.doesNotMatch(text, /perf-baseline-only-password|Call log/);
  assert.match(text, /redacted/);
});

test('incomplete cleanup and unresolved exit stop sampling without terminating unknown identities', () => {
  const exited = { exitCode: 1, signal: null };
  const cleanup = {
    cleanup: [
      { pid: 123, status: 'terminated' },
      { pid: 456, status: 'already-exited' },
    ],
  };
  assert.equal(cleanupOutcome(cleanup, exited).complete, true);
  for (const status of [
    'unverifiable-not-terminated',
    'identity-changed-not-terminated',
    'termination-pending',
    'termination-failed',
  ])
    assert.equal(cleanupOutcome({ cleanup: [{ pid: 123, status }] }, exited).complete, false);
  assert.equal(cleanupOutcome({ ...cleanup, unverifiedDescendants: true }, exited).complete, false);
  assert.equal(cleanupOutcome({ cleanup: [], reason: 'CIM unavailable' }, exited).complete, false);
  assert.equal(cleanupOutcome(cleanup, { exitCode: null, reason: 'timeout' }).complete, false);
  assert.equal(cleanupOutcome(cleanup, { exitCode: null, signal: 'SIGTERM' }).complete, true);
});

test('PowerShell stderr errors replace encoded command noise and retain timeout context', () => {
  const stderr =
    '#< CLIXML\r\n<Objs><Obj S="progress"><S>Preparing modules</S></Obj>' +
    '<S S="Error">Where-Object : Cannot convert &quot;System.Object[]&quot; to System.Int32._x000D__x000A_</S>' +
    '<S S="Error">At line:17 char:19_x000D__x000A_</S></Objs>';
  const reason = powerShellError(
    { message: 'Command failed: powershell.exe -EncodedCommand VERY_LONG_BASE64', stderr, code: 1 },
    'cleanup',
    20000,
  );
  assert.match(reason, /Cannot convert "System.Object\[\]" to System.Int32/);
  assert.doesNotMatch(reason, /EncodedCommand|VERY_LONG_BASE64|Preparing modules|CLIXML/);
  const timeout = powerShellError(
    { message: 'Command failed: -EncodedCommand HIDDEN', killed: true, stderr: '' },
    'cleanup',
    20000,
  );
  assert.match(timeout, /timed out after 20000ms/);
  assert.doesNotMatch(timeout, /EncodedCommand|HIDDEN/);
  assert.match(
    powerShellError({ stderr: 'Access denied', code: 1 }, 'cleanup', 20000),
    /Access denied/,
  );
});

test('unfinished owned child reference is released on timeout without claiming termination', () => {
  let unrefCount = 0;
  const child = {
    exitCode: null,
    signalCode: null,
    unref() {
      unrefCount++;
    },
    kill() {
      throw new Error('No termination is authorized in this test');
    },
  };
  assert.equal(releaseUnfinishedChild(child, { exitCode: null, reason: 'timeout' }), true);
  assert.equal(unrefCount, 1);
  assert.equal(child.exitCode, null);
  assert.equal(releaseUnfinishedChild(child, { exitCode: 0 }), false);
  assert.equal(releaseUnfinishedChild({ ...child, exitCode: 1 }, { exitCode: null }), false);
  assert.equal(
    releaseUnfinishedChild({ ...child, signalCode: 'SIGTERM' }, { exitCode: null }),
    false,
  );
  assert.equal(unrefCount, 1);
});

test('failed samples remain represented and do not contribute to success percentiles', () => {
  const results = summarize([
    { phases: [{ name: 'startup', success: true, durationMs: 20 }] },
    { phases: [{ name: 'startup', success: false, durationMs: 999 }] },
    { phases: [{ name: 'startup', success: true, durationMs: 40 }] },
  ]);
  assert.deepEqual(results[0], {
    name: 'startup',
    successfulSamples: 2,
    requestedSamples: 3,
    medianMs: 30,
    p95Ms: 40,
  });
  assert.equal(results[1].medianMs, null);
});

test('summary uses standard odd/even medians and retains requested count after failed samples', () => {
  const scenarios = [
    { values: [50, 10, 30, 20, 40], failed: [], median: 30, p95: 50 },
    { values: [40, 10, 30, 20], failed: [], median: 25, p95: 40 },
    { values: [10, 40, 20, 30], failed: [999], median: 25, p95: 40 },
  ];
  for (const scenario of scenarios) {
    const samples = [
      ...scenario.values.map((durationMs) => ({
        phases: [{ name: 'startup', success: true, durationMs }],
      })),
      ...scenario.failed.map((durationMs) => ({
        phases: [{ name: 'startup', success: false, durationMs }],
      })),
    ];
    const result = summarize(samples)[0];
    assert.equal(result.successfulSamples, scenario.values.length);
    assert.equal(result.requestedSamples, samples.length);
    assert.equal(result.medianMs, scenario.median);
    assert.equal(result.p95Ms, scenario.p95);
  }
});

async function withFiles(run) {
  const root = await mkdtemp(path.join(tmpdir(), 'solosoul-rf312-runner-tests-'));
  const resolvedTemp = await realpath(tmpdir());
  try {
    const exe = path.join(root, 'synthetic.exe');
    const fixture = path.join(root, 'fixture');
    await mkdir(path.join(fixture, 'acc_rf312_100'), { recursive: true });
    await writeFile(
      exe,
      'Never execute this argument-only stub.\0' + NATIVE_PERF_MARKERS.join('\0'),
    );
    await writeFile(path.join(fixture, 'rf312-fixture.json'), JSON.stringify(marker()));
    await writeFile(path.join(fixture, 'acc_rf312_100', 'vault.db'), 'synthetic placeholder');
    await writeFile(path.join(fixture, 'acc_rf312_100', 'config.json'), '{}');
    await writeFile(path.join(fixture, 'ui_preferences.json'), '{}');
    await run({ exe, fixture, output: path.join(root, 'new-results'), samples: 3 }, root);
  } finally {
    const resolvedRoot = await realpath(root);
    if (
      path.dirname(resolvedRoot).toLowerCase() !== resolvedTemp.toLowerCase() ||
      !path.basename(resolvedRoot).startsWith('solosoul-rf312-runner-tests-') ||
      (await lstat(root)).isSymbolicLink()
    )
      throw new Error('Refusing test cleanup outside this created temporary directory');
    await rm(root, { recursive: true, force: false });
  }
}

test('offline native-perf preflight accepts all feature markers and rejects every missing marker', async () => {
  await withFiles(async (options) => {
    const accepted = await nativePerfPreflight(options.exe);
    assert.deepEqual(accepted.matchedMarkers, [...NATIVE_PERF_MARKERS]);
    assert.match(accepted.guarantee, /not a binary signature or trust guarantee/);
    for (const missing of NATIVE_PERF_MARKERS) {
      await writeFile(
        options.exe,
        NATIVE_PERF_MARKERS.filter((value) => value !== missing).join('\0'),
      );
      await assert.rejects(
        nativePerfPreflight(options.exe),
        (error) =>
          error.message.includes('Refusing to execute --exe') && error.message.includes(missing),
      );
    }
    await assert.rejects(lstat(options.output), { code: 'ENOENT' });
  });
});

test('offline marker scanning matches markers spanning multiple read blocks', async () => {
  await withFiles(async (options) => {
    const content =
      'MZ\0padding\0' + NATIVE_PERF_MARKERS.join('\0inter-marker-padding\0') + '\0tail';
    await writeFile(options.exe, content);
    const result = await nativePerfPreflight(options.exe, 7);
    assert.deepEqual(result.matchedMarkers, [...NATIVE_PERF_MARKERS]);
    assert.ok(result.bytesScanned <= Buffer.byteLength(content));
    await assert.rejects(nativePerfPreflight(options.exe, 0), /chunk size/);
  });
});

test('default-build stub is rejected offline before CLI output creation or executable launch', async () => {
  await withFiles(async (options) => {
    const original = 'MZ\0default-build-stub-without-native-perf-feature';
    await writeFile(options.exe, original);
    const runner = fileURLToPath(new URL('./native-perf-run.mjs', import.meta.url));
    const rejected = spawnSync(
      process.execPath,
      [
        runner,
        '--exe',
        options.exe,
        '--fixture',
        options.fixture,
        '--output',
        options.output,
        '--samples',
        '3',
      ],
      { encoding: 'utf8', windowsHide: true, timeout: 10000 },
    );
    assert.equal(rejected.status, 1, rejected.stdout + rejected.stderr);
    assert.match(
      rejected.stderr,
      /Refusing to execute --exe: offline native-perf feature markers are missing/,
    );
    assert.match(rejected.stderr, /default builds are not allowed/);
    assert.doesNotMatch(rejected.stdout, /RF-312 native sample/);
    await assert.rejects(lstat(options.output), { code: 'ENOENT' });
    assert.equal(await readFile(options.exe, 'utf8'), original);
  });
});

test('WebView2 browser data accepts only the exact API UDF/EBWebView directory', () => {
  const root = String.raw`C:\owned\sample\webview`;
  const canonicalBrowser = String.raw`\\?\C:\owned\sample\webview\EBWebView`;
  const accepted = browserDataDirectoryCheck(root, canonicalBrowser);
  assert.equal(accepted.matchesExpected, true);
  assert.equal(accepted.apiUserDataFolder, root);
  assert.equal(accepted.expectedBrowserDataDirectory, path.win32.join(root, 'EBWebView'));
  assert.equal(accepted.actualBrowserDataDirectory, canonicalBrowser);
  for (const actual of [
    root,
    path.win32.join(root, 'other'),
    path.win32.join(root, 'EBWebView', 'extra'),
    String.raw`C:\external\EBWebView`,
  ]) {
    const rejected = browserDataDirectoryCheck(root, actual);
    assert.equal(rejected.matchesExpected, false);
    assert.equal(rejected.actualBrowserDataDirectory, null);
    assert.match(rejected.reason, /exact API UDF\/EBWebView/);
  }
});

test('WebView2 exact owned directories reject filesystem junction redirection', async () => {
  await withFiles(async (_options, root) => {
    const apiRoot = path.join(root, 'owned-webview');
    const browserRoot = path.join(apiRoot, 'EBWebView');
    await mkdir(browserRoot, { recursive: true });
    assert.equal((await browserDataFilesystemCheck(apiRoot)).valid, true);
    const other = path.join(root, 'different-directory');
    await mkdir(other);
    const redirectedChildRoot = path.join(root, 'redirected-browser');
    await mkdir(redirectedChildRoot);
    await symlink(
      other,
      path.join(redirectedChildRoot, 'EBWebView'),
      process.platform === 'win32' ? 'junction' : 'dir',
    );
    const rejected = await browserDataFilesystemCheck(redirectedChildRoot);
    assert.equal(rejected.valid, false);
    assert.match(rejected.reason, /regular directory|link|alternate directory/);
    const redirectedApiRoot = path.join(root, 'redirected-api');
    await symlink(other, redirectedApiRoot, process.platform === 'win32' ? 'junction' : 'dir');
    assert.equal((await browserDataFilesystemCheck(redirectedApiRoot)).valid, false);
  });
});

test('file validation never creates output and refuses existing output without changing it', async () => {
  await withFiles(async (options) => {
    const result = await validateInputs(options);
    assert.equal(result.manifest.objectCount, 100);
    await assert.rejects(lstat(options.output), { code: 'ENOENT' });
    await mkdir(options.output);
    const original = path.join(options.output, 'preserve.txt');
    await writeFile(original, 'unchanged');
    await assert.rejects(validateInputs(options), /must not already exist/);
    assert.equal(await readFile(original, 'utf8'), 'unchanged');
  });
});

test('file validation refuses linked fixture and output placed inside source fixture', async () => {
  await withFiles(async (options, root) => {
    await assert.rejects(
      validateInputs({ ...options, output: path.join(options.fixture, 'results') }),
      /inside the source fixture/,
    );
    const linked = path.join(root, 'fixture-junction');
    await symlink(options.fixture, linked, process.platform === 'win32' ? 'junction' : 'dir');
    await assert.rejects(validateInputs({ ...options, fixture: linked }), /regular directory|link/);
  });
});

test('CLI help and invalid sample count exit without loading a GUI executable', () => {
  const runner = fileURLToPath(new URL('./native-perf-run.mjs', import.meta.url));
  const help = spawnSync(process.execPath, [runner, '--help'], {
    encoding: 'utf8',
    windowsHide: true,
  });
  assert.equal(help.status, 0);
  assert.match(help.stdout, /Windows only/);
  const root = path.resolve(tmpdir(), 'rf312-never-spawn');
  const invalid = spawnSync(
    process.execPath,
    [
      runner,
      '--exe',
      path.join(root, 'does-not-exist.exe'),
      '--fixture',
      path.join(root, 'none'),
      '--output',
      path.join(root, 'none'),
      '--samples',
      '2',
    ],
    { encoding: 'utf8', windowsHide: true },
  );
  assert.equal(invalid.status, 1);
  assert.match(invalid.stderr, /--samples must be an integer >= 3/);
});

test('PowerShell 5.1 cleanup consumes a flat JSON identity list and rejects changed identities offline', () => {
  const created = Date.parse('2026-09-30T08:00:00Z');
  const exe = String.raw`C:\synthetic\fixture.exe`;
  let offline = PROCESS_SCRIPT.slice(0, PROCESS_SCRIPT.indexOf('$root = $all'));
  offline = offline.replace(
    /^\$all = .*$/m,
    '$all = @([Environment]::GetEnvironmentVariable("SOLOSOUL_NATIVE_PERF_FAKE_ROWS") | ConvertFrom-Json)\n$all = $all[0]',
  );
  offline = offline.replace(
    '$process = [Diagnostics.Process]::GetProcessById([int]$owned.pid)',
    '$process = [PSCustomObject]@{StartTime=[DateTimeOffset]::FromUnixTimeMilliseconds([long]$owned.creationMs).UtcDateTime;HasExited=$false}',
  );
  offline = offline.replace('$process.Kill()', '$process.HasExited = $true');
  offline = offline.replace(
    '$pending.process.WaitForExit($remaining) | Out-Null',
    '# offline: no waiting',
  );
  offline = offline.replace('$pending.process.Dispose()', '[void]0');
  assert.doesNotMatch(offline, /Get-CimInstance|GetProcessById|\.Kill\(|WaitForExit|\.Dispose\(/);
  const fakeRows = [
    {
      ProcessId: 123,
      ParentProcessId: 0,
      CreationDate: '2026-09-30T08:00:00Z',
      ExecutablePath: exe,
      Name: 'fixture.exe',
    },
    {
      ProcessId: 456,
      ParentProcessId: 123,
      CreationDate: '2026-09-30T08:00:01Z',
      ExecutablePath: exe,
      Name: 'fixture.exe',
    },
    {
      ProcessId: 321,
      ParentProcessId: 123,
      CreationDate: null,
      ExecutablePath: exe,
      Name: 'fixture.exe',
    },
  ];
  // 仅补DateTime类型，与CIM数据形状一致；无实际CIM/Process API操作。
  offline = offline.replace(
    '$all = $all[0]',
    '$all = $all[0]\nforeach($row in $all){if($row.CreationDate){$row.CreationDate=[DateTime]$row.CreationDate}}',
  );
  const known = [123, 456, 321, 789].map((pid) => ({
    pid,
    creationMs: created,
    executablePath: exe,
  }));
  const result = spawnSync(
    'powershell.exe',
    [
      '-NoLogo',
      '-NoProfile',
      '-NonInteractive',
      '-EncodedCommand',
      Buffer.from(offline, 'utf16le').toString('base64'),
    ],
    {
      encoding: 'utf8',
      windowsHide: true,
      timeout: 10000,
      env: {
        ...process.env,
        SOLOSOUL_NATIVE_PERF_PID: '123',
        SOLOSOUL_NATIVE_PERF_EXE: exe,
        SOLOSOUL_NATIVE_PERF_STARTED: String(created),
        SOLOSOUL_NATIVE_PERF_ACTION: 'cleanup',
        SOLOSOUL_NATIVE_PERF_KNOWN: JSON.stringify(known),
        SOLOSOUL_NATIVE_PERF_FAKE_ROWS: JSON.stringify(fakeRows),
      },
    },
  );
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.deepEqual(
    JSON.parse(result.stdout.trim()).cleanup.map(({ pid, status }) => ({ pid, status })),
    [
      { pid: 123, status: 'terminated' },
      { pid: 456, status: 'identity-changed-not-terminated' },
      { pid: 321, status: 'unverifiable-not-terminated' },
      { pid: 789, status: 'already-exited' },
    ],
  );
});

test(
  'fixed PowerShell ownership/memory script parses without executing CIM or termination',
  { skip: process.platform !== 'win32' },
  () => {
    const parser = `
$ErrorActionPreference='Stop'
$source=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_SYNTAX')))
$tokens=$null; $errors=$null
[Management.Automation.Language.Parser]::ParseInput($source,[ref]$tokens,[ref]$errors) | Out-Null
if($errors.Count){ $errors | ForEach-Object { $_.Message }; exit 1 }
exit 0
`;
    const result = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-NonInteractive',
        '-EncodedCommand',
        Buffer.from(parser, 'utf16le').toString('base64'),
      ],
      {
        windowsHide: true,
        encoding: 'utf8',
        timeout: 15000,
        env: {
          ...process.env,
          SOLOSOUL_NATIVE_PERF_SYNTAX: Buffer.from(PROCESS_SCRIPT).toString('base64'),
        },
      },
    );
    assert.equal(result.status, 0, result.stdout + result.stderr);
  },
);

test('SDK requested or proof markers exclude benchmark acceptance even with malformed content or non-file markers', async () => {
  const parent = await realpath(tmpdir());
  const dir = await mkdtemp(path.join(tmpdir(), 'ss-rf312-sdk-benchmark-'));
  try {
    await rejectDiagnosticBenchmark(dir);
    for (const name of ['native-perf-sdk-cdp-requested.json', 'native-perf-sdk-cdp.json']) {
      const root = path.join(dir, name);
      await mkdir(root);
      await writeFile(path.join(root, name), '{invalid diagnostic marker');
      await assert.rejects(
        rejectDiagnosticBenchmark(root),
        /cannot be used as a performance sample/,
      );
      const directoryRoot = path.join(dir, name + '-directory');
      await mkdir(directoryRoot);
      await mkdir(path.join(directoryRoot, name));
      await assert.rejects(
        rejectDiagnosticBenchmark(directoryRoot),
        /cannot be used as a performance sample/,
      );
    }
  } finally {
    const actual = await realpath(dir);
    if (
      path.dirname(actual).toLowerCase() !== parent.toLowerCase() ||
      !path.basename(actual).startsWith('ss-rf312-sdk-benchmark-') ||
      (await lstat(dir)).isSymbolicLink()
    )
      throw new Error('Refusing SDK benchmark test cleanup outside the exclusive temporary root');
    await rm(dir, { recursive: true, force: false });
  }
});
