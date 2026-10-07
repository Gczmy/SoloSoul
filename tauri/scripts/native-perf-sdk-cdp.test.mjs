import assert from 'node:assert/strict';
import { test } from 'node:test';
import path from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  lstat,
  realpath,
  rm,
  symlink,
} from 'node:fs/promises';
import { NATIVE_PERF_MARKERS } from './native-perf-run.mjs';
import {
  parseSdkArgs,
  sdkBinaryPreflight,
  sdkLaunchArgs,
  checkSdkRequested,
  checkSdkProof,
  sameSdkIdentities,
  readSdkJson,
  waitSdkProof,
  SDK_REQUESTED_FILE,
  SDK_PROOF_FILE,
} from './native-perf-sdk-cdp.mjs';

const BS = String.fromCharCode(92);
const EXTENDED = BS + BS + '?' + BS;
const owned = { root: EXTENDED + 'C:' + BS + 'owned' + BS + 'sample-001', runId: 'a'.repeat(32) };
const root = {
  pid: 101,
  parentPid: 7,
  creationMs: 1000,
  executablePath: 'C:' + BS + 'isolated' + BS + 'solo_soul.exe',
};
const browser = {
  pid: 102,
  parentPid: 101,
  creationMs: 1100,
  executablePath: 'C:' + BS + 'runtime' + BS + 'msedgewebview2.exe',
};
const identities = [root, browser];
function requested() {
  return {
    schemaVersion: 1,
    scope: 'windows-native-sdk-cdp-requested',
    mode: 'sdk-cdp',
    performanceSample: false,
    root: owned.root,
    runId: owned.runId,
    pid: root.pid,
    port: 44001,
    windowLabel: 'main',
  };
}
function proof() {
  return {
    ...requested(),
    scope: 'windows-native-sdk-cdp-diagnostic',
    expectedOrigin: 'http://tauri.localhost',
    browserPid: browser.pid,
    success: true,
    uiReadyDiagnostic: {
      schemaVersion: 1,
      boundary: 'startup-handoff',
      outcome: 'handoff',
      initialUiRootPresent: false,
      initialHandoffMarked: false,
      finalUiRootPresent: true,
      finalHandoffMarked: true,
      initialStartup: { state: 'loading', phase: 'preferences', reason: 'none' },
      finalStartup: { state: 'ready', phase: 'accounts', reason: 'none' },
    },
    stage: 'complete',
    reason: null,
    calls: ['Page.getFrameTree', 'Runtime.evaluate'],
    elapsedMs: 100,
    binding: {
      source: 'http://tauri.localhost/',
      mainFrameId: 'MAIN-A',
      loaderId: 'LOAD-A',
      timeOriginMs: 12345,
      navigationEvents: 0,
      frameCreatedEvents: 0,
    },
    document: {
      origin: 'http://tauri.localhost',
      readyState: 'complete',
      mainFrame: true,
      frameCount: 0,
      uiRootPresent: true,
    },
    observer: {
      schemaVersion: 1,
      scope: 'windows-native-tauri-invoke-observer',
      runId: owned.runId,
      valid: true,
      total: 2,
      observedCount: 2,
      installedAtMs: 1,
      timeOriginMs: 12345,
      invalidReasons: [],
      commands: [
        { command: 'list_accounts', atMs: 2 },
        { command: 'plugin:window|set_title', atMs: 3 },
      ],
    },
  };
}
async function withDirectory(run) {
  const parent = await realpath(tmpdir());
  const dir = await mkdtemp(path.join(await realpath(tmpdir()), 'ss-rf312-sdk-node-'));
  try {
    await run(dir);
  } finally {
    const actual = await realpath(dir);
    if (
      path.dirname(actual).toLowerCase() !== parent.toLowerCase() ||
      !path.basename(actual).startsWith('ss-rf312-sdk-node-') ||
      (await lstat(dir)).isSymbolicLink()
    )
      throw new Error(
        'Refusing SDK test cleanup outside the exclusive synthetic temporary directory',
      );
    await rm(dir, { recursive: true, force: false });
  }
}
async function stubFixture(dir) {
  const fixture = path.join(dir, 'fixture');
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
  await mkdir(path.join(fixture, marker.accountId), { recursive: true });
  await writeFile(path.join(fixture, 'rf312-fixture.json'), JSON.stringify(marker));
  await writeFile(path.join(fixture, marker.accountId, 'vault.db'), 'public synthetic placeholder');
  await writeFile(path.join(fixture, marker.accountId, 'config.json'), '{}');
  await writeFile(path.join(fixture, 'ui_preferences.json'), '{}');
  return fixture;
}

test('SDK CLI requires exactly three paths and refuses other experimental/control modes', () => {
  const base = path.resolve(tmpdir(), 'rf312-sdk-args');
  const args = [
    '--exe',
    path.join(base, 'app.exe'),
    '--fixture',
    path.join(base, 'fixture'),
    '--output',
    path.join(base, 'new'),
  ];
  assert.deepEqual(parseSdkArgs(args), { exe: args[1], fixture: args[3], output: args[5] });
  for (const value of [
    [],
    args.slice(0, -2),
    [...args, '--exe', args[1]],
    [...args, '--sdk-cdp'],
    [...args, '--chromium-log'],
    [...args, '--ordinary-native-tmp'],
    [...args, '--runtime-source', base],
    [...args, '--samples', '1'],
    [...args, '--password', 'anything'],
    ['--exe', 'relative', ...args.slice(2)],
    [...args.slice(0, -1), path.parse(base).root],
  ])
    assert.throws(() => parseSdkArgs(value));
});
test('SDK help starts no native preparation or GUI process', () => {
  const result = spawnSync(
    process.execPath,
    [fileURLToPath(new URL('./native-perf-sdk-cdp.mjs', import.meta.url)), '--help'],
    { windowsHide: true, encoding: 'utf8', timeout: 10000 },
  );
  assert.equal(result.status, 0);
  assert.match(
    result.stdout,
    /no TCP\/HTTP CDP endpoint connection\/attach, password\/UI actions or performance metrics/,
  );
  assert.equal(result.stderr, '');
});
test('SDK streaming preflight rejects older binaries and accepts cross-block feature markers', async () => {
  await withDirectory(async (dir) => {
    const exe = path.join(dir, 'stub.exe');
    await writeFile(exe, NATIVE_PERF_MARKERS.join('\0'));
    await assert.rejects(sdkBinaryPreflight(exe), /SDK CDP requires a rebuilt/);
    await writeFile(
      exe,
      Buffer.concat([
        Buffer.alloc(65536 - 8, 0x78),
        Buffer.from('windows-native-sdk-cdp-requested'),
        Buffer.from('tail'),
      ]),
    );
    assert.equal((await sdkBinaryPreflight(exe)).method, 'streaming-sdk-cdp-feature-marker');
    assert.match((await sdkBinaryPreflight(exe, 7)).limitation, /not a signature/);
    for (const size of [0, -1, 1.5, 1048577]) await assert.rejects(sdkBinaryPreflight(exe, size));
  });
});
test(
  'old native-perf EXE fails before output creation or any EXE execution',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires the Windows-only SDK diagnostic CLI to reach its offline feature preflight',
  },
  async () => {
    await withDirectory(async (dir) => {
      const fixture = await stubFixture(dir);
      const exe = path.join(dir, 'stub.exe');
      const contents = 'Do not execute this synthetic stub\0' + NATIVE_PERF_MARKERS.join('\0');
      await writeFile(exe, contents);
      const output = path.join(dir, 'new-output');
      const result = spawnSync(
        process.execPath,
        [
          fileURLToPath(new URL('./native-perf-sdk-cdp.mjs', import.meta.url)),
          '--exe',
          exe,
          '--fixture',
          fixture,
          '--output',
          output,
        ],
        { encoding: 'utf8', windowsHide: true, timeout: 10000 },
      );
      assert.equal(result.status, 1, result.stdout + result.stderr);
      assert.match(result.stderr, /SDK CDP requires a rebuilt.*no EXE was executed/);
      assert.equal(result.stdout, '');
      await assert.rejects(lstat(output), { code: 'ENOENT' });
      assert.equal(await readFile(exe, 'utf8'), contents);
    });
  },
);
test('SDK launch selects only the run-only native SDK diagnostics contract', () => {
  assert.deepEqual(sdkLaunchArgs(owned.root, 44001), [
    '--native-perf-root',
    owned.root,
    '--native-perf-port',
    '44001',
    '--native-perf-diagnostics',
    'sdk-cdp',
  ]);
  for (const [folder, port] of [
    ['relative', 44001],
    [owned.root, 1023],
    [owned.root, 65536],
    [owned.root, 1.5],
  ])
    assert.throws(() => sdkLaunchArgs(folder, port));
});
test('requested marker requires exact canonical root, run, PID, port and main window', () => {
  assert.deepEqual(checkSdkRequested(requested(), owned, root.pid, 44001), requested());
  for (const change of [
    { schemaVersion: 2 },
    { scope: 'windows-native-perf-owned' },
    { mode: 'chromium-log' },
    { performanceSample: true },
    { root: owned.root.slice(4) },
    { root: owned.root.toLowerCase() },
    { runId: 'b'.repeat(32) },
    { pid: 999 },
    { port: 44002 },
    { windowLabel: 'popup' },
  ])
    assert.throws(() => checkSdkRequested({ ...requested(), ...change }, owned, root.pid, 44001));
  for (const key of Object.keys(requested())) {
    const missing = requested();
    delete missing[key];
    assert.throws(() => checkSdkRequested(missing, owned, root.pid, 44001));
  }
});
test('successful SDK proof returns only validated metadata and protocol calls, never performance metrics or payloads', () => {
  const value = proof();
  value.untrustedPayload = { secret: 'never capture this field' };
  value.binding.untrustedDom = 'not captured';
  const accepted = checkSdkProof(value, owned, identities, 44001, 1000);
  assert.equal(accepted.success, true);
  assert.deepEqual(accepted.calls, ['Page.getFrameTree', 'Runtime.evaluate']);
  assert.equal(accepted.browserPid, browser.pid);
  assert.equal(Object.hasOwn(accepted, 'performanceMetrics'), false);
  assert.equal(JSON.stringify(accepted).includes('never capture'), false);
  assert.equal(JSON.stringify(accepted).includes('untrustedDom'), false);
});
test('SDK proof refuses failed, incomplete, wrong-origin, wrong-browser and non-exact protocol calls', () => {
  for (const change of [
    { success: false, stage: 'runtime-evaluate', reason: 'runtime-evaluate-failed' },
    { success: true, stage: 'frame-tree' },
    { reason: 'unexpected' },
    { expectedOrigin: 'https://tauri.localhost' },
    { browserPid: 999 },
    { browserPid: root.pid },
    { calls: ['Runtime.evaluate', 'Page.getFrameTree'] },
    { calls: ['Page.getFrameTree'] },
    { calls: ['Page.getFrameTree', 'Runtime.evaluate', 'Runtime.evaluate'] },
    { elapsedMs: -1 },
    { elapsedMs: 20000.1 },
    { elapsedMs: NaN },
  ])
    assert.throws(() => checkSdkProof({ ...proof(), ...change }, owned, identities, 44001, 1000));
  for (const key of Object.keys(proof())) {
    const missing = proof();
    delete missing[key];
    assert.throws(() => checkSdkProof(missing, owned, identities, 44001, 1000));
  }
});
test('SDK source rejects credentials, ports, other origins, malformed values and raw URL exposure', () => {
  for (const source of [
    'http://tauri.localhost:80/',
    'http://tauri.localhost:1234/',
    'https://tauri.localhost/',
    'http://user:secret@tauri.localhost/',
    'http://tauri.localhost.evil/',
    'http://evil/secret',
    'http://TAURI.localhost/',
    'not-a-url',
    'http://tauri.localhost' + BS + '@evil/',
    'http://tauri.localhost/\nsecret',
    'a'.repeat(2049),
  ]) {
    const value = proof();
    value.binding.source = source;
    assert.throws(
      () => checkSdkProof(value, owned, identities, 44001, 1000),
      (error) => !error.message.includes(source) && /SDK source/.test(error.message),
    );
  }
});
test('SDK main frame/loader/document guard rejects navigation, children, missing UI container and loading documents', () => {
  for (const change of [
    { mainFrameId: '' },
    { loaderId: '' },
    { mainFrameId: 'unsafe id' },
    { timeOriginMs: 0 },
    { timeOriginMs: Infinity },
    { navigationEvents: 1 },
    { frameCreatedEvents: 1 },
  ]) {
    const value = proof();
    Object.assign(value.binding, change);
    assert.throws(() => checkSdkProof(value, owned, identities, 44001, 1000));
  }
  for (const change of [
    { origin: 'http://evil' },
    { readyState: 'interactive' },
    { mainFrame: false },
    { frameCount: 1 },
    { frameCount: null },
    { uiRootPresent: false },
  ]) {
    const value = proof();
    Object.assign(value.document, change);
    assert.throws(() => checkSdkProof(value, owned, identities, 44001, 1000));
  }
});
test('SDK observer must remain valid in the same run/timeOrigin with exact bounded count metadata', () => {
  for (const change of [
    { valid: false },
    { runId: 'b'.repeat(32) },
    { timeOriginMs: 54321 },
    { observedCount: 3 },
    { total: -1 },
    { invalidReasons: ['document-changed'] },
    { installedAtMs: -1 },
    {
      commands: [
        { command: 'x', atMs: 0 },
        { command: 'x', atMs: 0 },
      ],
    },
  ]) {
    const value = proof();
    Object.assign(value.observer, change);
    assert.throws(() => checkSdkProof(value, owned, identities, 44001, 1000));
  }
  const value = proof();
  value.observer.commands = Array.from({ length: 16385 }, () => ({
    command: 'get_config',
    atMs: 2,
  }));
  value.observer.total = value.observer.observedCount = 16385;
  assert.throws(() => checkSdkProof(value, owned, identities, 44001, 1000));
});
test('SDK proof accepts the deadline boundary but refuses late, negative and absent arrival measurements', () => {
  assert.equal(checkSdkProof(proof(), owned, identities, 44001, 45000).success, true);
  for (const arrival of [45000.1, -1, NaN, Infinity, undefined])
    assert.throws(() => checkSdkProof(proof(), owned, identities, 44001, arrival));
});
test('SDK identity comparison rejects PID reuse, changed parents/paths and extra processes', () => {
  assert.equal(sameSdkIdentities(identities, structuredClone(identities)), true);
  for (const change of [
    { pid: 999 },
    { parentPid: 999 },
    { creationMs: 1101 },
    { executablePath: 'C:' + BS + 'other' + BS + 'msedgewebview2.exe' },
  ]) {
    const after = structuredClone(identities);
    Object.assign(after[1], change);
    assert.throws(() => sameSdkIdentities(identities, after));
  }
  assert.throws(() => sameSdkIdentities(identities, [...identities, browser]));
  assert.throws(() =>
    checkSdkProof(proof(), owned, [root, { ...browser, parentPid: 999 }], 44001, 1000),
  );
});
test('SDK marker reading accepts exact owned regular JSON and refuses traversal names, malformed or oversized files', async () => {
  await withDirectory(async (dir) => {
    await writeFile(path.join(dir, SDK_PROOF_FILE), JSON.stringify(proof()));
    const read = await readSdkJson(dir, SDK_PROOF_FILE);
    assert.deepEqual(read.value, proof());
    assert.equal(read.file, SDK_PROOF_FILE);
    for (const name of [
      '../native-perf-sdk-cdp.json',
      'other.json',
      BS + BS + 'server' + BS + 'data',
    ])
      await assert.rejects(readSdkJson(dir, name));
    await writeFile(path.join(dir, SDK_PROOF_FILE), '{not json}');
    await assert.rejects(
      readSdkJson(dir, SDK_PROOF_FILE),
      /not valid JSON; raw content is not captured/,
    );
    await writeFile(path.join(dir, SDK_PROOF_FILE), Buffer.alloc(1048577));
    await assert.rejects(readSdkJson(dir, SDK_PROOF_FILE), /1MiB/);
  });
});
test('SDK file/directory junctions and non-file proof markers never traverse another folder', async () => {
  await withDirectory(async (dir) => {
    const outside = path.join(dir, 'actual');
    await mkdir(outside);
    await writeFile(path.join(outside, SDK_PROOF_FILE), JSON.stringify(proof()));
    const alias = path.join(dir, 'alias');
    await symlink(outside, alias, 'junction');
    await assert.rejects(readSdkJson(alias, SDK_PROOF_FILE), /exact regular owned path/);
    const normal = path.join(dir, 'normal');
    await mkdir(normal);
    await mkdir(path.join(normal, SDK_PROOF_FILE));
    await assert.rejects(readSdkJson(normal, SDK_PROOF_FILE), /exact regular owned path/);
    await assert.rejects(waitSdkProof(normal, 0, { now: () => 1 }), /bounded regular file/);
    assert.deepEqual(
      JSON.parse(await readFile(path.join(outside, SDK_PROOF_FILE), 'utf8')),
      proof(),
    );
  });
});
test('SDK proof wait is bounded without callbacks and refuses late files or root interruption', async () => {
  await withDirectory(async (dir) => {
    let time = 0;
    await assert.rejects(
      waitSdkProof(dir, 0, {
        now: () => time,
        sleep: async (ms) => {
          time += ms;
        },
      }),
      /bounded 45-second/,
    );
    assert.equal(time, 45001);
    await writeFile(path.join(dir, SDK_PROOF_FILE), '{}');
    assert.equal(await waitSdkProof(dir, 0, { now: () => 45000 }), 45000);
    await assert.rejects(waitSdkProof(dir, 0, { now: () => 45001 }), /bounded 45-second/);
    await assert.rejects(
      waitSdkProof(dir, 0, { now: () => 1, shouldStop: () => true }),
      /interrupted/,
    );
  });
});

test('SDK proof lookup must recheck time after the async filesystem query and reject a 45000-to-45001 crossing', async () => {
  await withDirectory(async (dir) => {
    await writeFile(path.join(dir, SDK_PROOF_FILE), '{}');
    const readings = [45000, 45001];
    await assert.rejects(
      waitSdkProof(dir, 0, { now: () => readings.shift() }),
      /bounded 45-second/,
    );
    assert.equal(readings.length, 0);
  });
});

test('RF312 proof rejects missing, premature, timed-out or private handoff metadata', () => {
  for (const [key, changed] of [
    ['schemaVersion', 2],
    ['boundary', 'private-boundary'],
    ['outcome', 'timeout'],
    ['outcome', 'startup-error'],
    ['initialUiRootPresent', 'private-flag'],
    ['finalUiRootPresent', false],
    ['finalHandoffMarked', false],
    ['extra', 'private-extra'],
    ['initialStartup', { state: 'loading', phase: 'private-phase', reason: 'none' }],
    ['finalStartup', { state: 'ready', phase: 'accounts', reason: 'private-reason' }],
  ]) {
    const value = proof();
    value.uiReadyDiagnostic[key] = changed;
    assert.throws(
      () => checkSdkProof(value, owned, identities, 44001, 1000),
      (error) => !error.message.includes('private') && /UI handoff/.test(error.message),
    );
  }
  const accepted = checkSdkProof(proof(), owned, identities, 44001, 1000);
  assert.deepEqual(accepted.uiReadyDiagnostic, proof().uiReadyDiagnostic);
  assert.equal(accepted.performanceSample, false);
});
