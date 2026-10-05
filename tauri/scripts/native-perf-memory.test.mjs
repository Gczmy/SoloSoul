import test from 'node:test';
import assert from 'node:assert/strict';
import { setTimeout as pause } from 'node:timers/promises';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { MemorySampler, memoryReading, parseJourneyArgs } from './native-perf-memory.mjs';
import { summarizeJourneys } from './native-perf-sdk-journey.mjs';
import * as ownedProcesses from './native-perf-run.mjs';

const launchedAt = Date.now(),
  expected = { pid: 100, exe: 'C:\\test\\solo_soul.exe', launchedAt };
const options = (sample, extras = {}) => ({ sample, intervalMs: 1000, ...expected, ...extras });
function snapshot(bytes = 30) {
  return {
    observedAt: new Date().toISOString(),
    collectionMs: 1,
    workingSetBytes: bytes,
    reason: null,
    processCount: 2,
    userDataDirectoryMatches: true,
    browserDataFilesystem: { valid: true },
    browserDataChecks: [{ pid: 200, matchesExpected: true }],
    processes: [
      {
        pid: 100,
        parentPid: 1,
        creationMs: launchedAt,
        executableName: 'solo_soul.exe',
        workingSetBytes: 10,
        reason: null,
      },
      {
        pid: 200,
        parentPid: 100,
        creationMs: launchedAt + 1,
        executableName: 'msedgewebview2.exe',
        workingSetBytes: bytes - 10,
        reason: null,
      },
    ],
  };
}
const base = [
  '--exe',
  '/tmp/test.exe',
  '--fixture',
  '/tmp/fixture',
  '--output',
  '/tmp/new',
  '--samples',
  '5',
];
test('memory interval is optional and bounded; original arguments remain strict', () => {
  assert.equal(parseJourneyArgs(base).memoryIntervalMs, null);
  assert.equal(parseJourneyArgs([...base, '--memory-interval-ms', '2000']).memoryIntervalMs, 2000);
  assert.equal(parseJourneyArgs(['--help']).help, true);
  for (const extra of [
    ['--memory-interval-ms', '999'],
    ['--memory-interval-ms', '10001'],
    ['--memory-interval-ms', 'NaN'],
    ['--memory-interval-ms'],
    ['--unknown', '2000'],
    ['--memory-interval-ms', '1000', '--memory-interval-ms', '1000'],
  ])
    assert.throws(() => parseJourneyArgs([...base, ...extra]));
  assert.throws(() => parseJourneyArgs(['--help', '--memory-interval-ms', '1000']));
});
test('memory readings reject incomplete rows, changed identity and unverified UDF', () => {
  assert.equal(memoryReading(snapshot(), expected, 200).valid, true);
  for (const mutate of [
    (s) => s.workingSetBytes++,
    (s) => (s.processes[0].workingSetBytes = null),
    (s) => s.processes[0].pid++,
    (s) => (s.processes[0].executableName = null),
    (s) => (s.processes[0] = null),
    (s) => s.processCount++,
    (s) => (s.processes[1].pid = 100),
    (s) => (s.reason = 'unavailable'),
    (s) => (s.userDataDirectoryMatches = false),
    (s) => (s.browserDataFilesystem.valid = false),
    (s) => (s.browserDataChecks[0].matchesExpected = false),
    (s) => s.browserDataChecks[0].pid++,
  ]) {
    const s = snapshot();
    mutate(s);
    assert.equal(memoryReading(s, expected, 200).valid, false);
  }
  assert.equal(
    memoryReading(snapshot(), { ...expected, creationMs: launchedAt + 5 }, 200).valid,
    false,
  );
  assert.equal(memoryReading(snapshot(), expected, 201).valid, false);
});
test('root-only startup permits only the original single root, never an unknown browser', () => {
  const s = snapshot();
  s.processes.pop();
  s.processCount = 1;
  s.workingSetBytes = 10;
  s.userDataDirectoryMatches = false;
  s.browserDataFilesystem = null;
  s.browserDataChecks = [];
  assert.equal(memoryReading(s, expected, 200).scope, 'root-only-before-webview');
  const unknown = snapshot();
  unknown.userDataDirectoryMatches = false;
  assert.equal(memoryReading(unknown, expected, 200).valid, false);
});
test('periodic sampling and checkpoints serialize; accepted observed maximum includes intermediate rows', async () => {
  let active = 0,
    maximumActive = 0,
    calls = 0;
  const sampler = new MemorySampler(
    options(async () => {
      maximumActive = Math.max(maximumActive, ++active);
      await pause(5);
      active--;
      return snapshot(++calls === 2 ? 80 : 30);
    }),
  ).start();
  await Promise.all([sampler.checkpoint('before-input'), sampler.checkpoint('after-journey')]);
  const series = await sampler.finish(200);
  assert.equal(maximumActive, 1);
  assert.equal(series.complete, true);
  assert.equal(series.observedMaximumWorkingSetBytes, 80);
  assert.equal(series.sampleCount, 3);
  assert.ok(series.collectionTotalMs > 0);
  assert.equal(series.samples[0].kind, 'periodic');
  await assert.rejects(sampler.checkpoint('periodic'));
  assert.throws(() => sampler.start());
});
test('finish waits for a pending query before caller may clean up owner', async () => {
  let release,
    queried = 0,
    cleaned = false;
  const gate = new Promise((resolve) => (release = resolve));
  const sampler = new MemorySampler(
    options(async () => {
      queried++;
      await gate;
      assert.equal(cleaned, false);
      return snapshot();
    }),
  ).start();
  await Promise.resolve();
  let finished = false;
  const completion = sampler.finish(200).then((series) => {
    finished = true;
    return series;
  });
  await pause(5);
  assert.equal(finished, false);
  release();
  await completion;
  cleaned = true;
  await pause(10);
  assert.equal(queried, 1);
});
test('failed queries and missing UI checkpoints cannot contribute an accepted maximum', async () => {
  let calls = 0;
  const sampler = new MemorySampler(
    options(async () => {
      if (++calls === 2) throw new Error('query failed');
      return snapshot();
    }),
  ).start();
  await sampler.checkpoint('before-input');
  await sampler.checkpoint('after-journey');
  const series = await sampler.finish(200);
  assert.equal(series.complete, false);
  assert.equal(series.observedMaximumWorkingSetBytes, null);
  assert.equal(series.validSampleCount, 2);
  assert.equal(series.partialMaximumWorkingSetBytes, 30);
  assert.equal(series.samples[1].snapshot.reason, 'query failed');
  const missing = new MemorySampler(options(async () => snapshot())).start();
  const partial = await missing.finish(200);
  assert.equal(partial.complete, false);
  assert.equal(partial.observedMaximumWorkingSetBytes, null);
});
test('sample cap fails closed and stops collecting without overlapping cleanup', async () => {
  const sampler = new MemorySampler(options(async () => snapshot(), { maxSamples: 3 })).start();
  await sampler.checkpoint('before-input');
  await sampler.checkpoint('after-journey');
  await assert.rejects(sampler.checkpoint('periodic'), /bound/);
  const series = await sampler.finish(200);
  assert.equal(series.complete, false);
  assert.equal(series.sampleCount, 3);
  assert.equal(series.observedMaximumWorkingSetBytes, null);
});
test('previously observed WebView or root identity cannot be replaced later', async () => {
  for (const change of ['no-webview', 'root-created']) {
    let calls = 0;
    const sampler = new MemorySampler(
      options(async () => {
        const s = snapshot();
        if (++calls === 3) {
          if (change === 'root-created') s.processes[0].creationMs++;
          else {
            s.processes.pop();
            s.processCount = 1;
            s.workingSetBytes = 10;
          }
        }
        return s;
      }),
    ).start();
    await sampler.checkpoint('before-input');
    await sampler.checkpoint('after-journey');
    const series = await sampler.finish(200);
    assert.equal(series.complete, false);
    assert.equal(series.observedMaximumWorkingSetBytes, null);
  }
});
test('failed memory sample never contributes successful UI phases to performance medians', () => {
  const phase = { name: 'startup', success: true, durationMs: 1 };
  const sample = {
    uiSuccess: true,
    success: false,
    phases: [phase],
    memorySeries: { complete: false },
  };
  assert.equal(summarizeJourneys([sample], 5)[0].medianMs, null);
});

test('only explicit memory-series mode excludes an identity-verified, freshly confirmed exited descendant', () => {
  const rows = [
    { pid: 100, creationMs: launchedAt, workingSetBytes: 10, reason: null },
    {
      pid: 200,
      creationMs: launchedAt + 1,
      workingSetBytes: null,
      reason: 'Process exited before memory collection; absence confirmed by fresh CIM',
      memoryStatus: 'confirmed-exited',
      exitConfirmedAtMs: launchedAt + 2,
    },
  ];
  assert.equal(typeof ownedProcesses.partitionMemoryRows, 'function');
  assert.equal(ownedProcesses.partitionMemoryRows(rows, 100, false).complete, false);
  const result = ownedProcesses.partitionMemoryRows(rows, 100, true);
  assert.equal(result.complete, true);
  assert.deepEqual(
    result.liveRows.map((row) => row.pid),
    [100],
  );
  assert.deepEqual(
    result.exitedRows.map((row) => row.pid),
    [200],
  );
  assert.equal(result.workingSetBytes, 10);
  for (const mutate of [
    (rs) => (rs[0].memoryStatus = 'confirmed-exited'),
    (rs) => (rs[1].memoryStatus = 'unknown'),
    (rs) => (rs[1].exitConfirmedAtMs = null),
    (rs) => (rs[1].exitConfirmedAtMs = launchedAt - 1),
    (rs) => (rs[1].workingSetBytes = 0),
    (rs) => delete rs[1].memoryStatus,
  ]) {
    const bad = structuredClone(rows);
    mutate(bad);
    assert.equal(ownedProcesses.partitionMemoryRows(bad, 100, true).complete, false);
  }
});

test(
  'actual PowerShell memory block confirms absence once; live, unavailable and root cases remain failures',
  { skip: process.platform === 'win32' ? false : 'Requires Windows PowerShell 5.1' },
  () => {
    const source = ownedProcesses.PROCESS_SCRIPT;
    const block = source.slice(
      source.indexOf('$processes = @()'),
      source.indexOf('$result = @{rootVerified=$true'),
    );
    assert.ok(block.includes('Process exited before memory collection'));
    for (const [action, alive, root, confirms] of [
      ['sample', false, false, false],
      ['memory-series', false, false, true],
      ['memory-series', true, false, false],
      ['memory-series', false, true, false],
    ]) {
      const script = `
$ErrorActionPreference='Stop'
$expectedPid=${root ? 2147483600 : 100}; $action='${action}'
$owned=@{2147483600=@{pid=2147483600;creationMs=1;executablePath='C:\\owned\\icacls.exe';name='icacls.exe'}}
$script:queries=0
function Get-CimInstance { param($ClassName,$Filter,$Property)
  $script:queries++
  ${alive ? '[PSCustomObject]@{ProcessId=2147483600}' : ''}
}
${block}
@{rows=$processes;queries=$script:queries} | ConvertTo-Json -Depth 5 -Compress
`;
      const result = spawnSync(
        'powershell.exe',
        [
          '-NoLogo',
          '-NoProfile',
          '-NonInteractive',
          '-EncodedCommand',
          Buffer.from(script, 'utf16le').toString('base64'),
        ],
        { windowsHide: true, encoding: 'utf8', timeout: 15000 },
      );
      assert.equal(result.status, 0, result.stdout + result.stderr);
      const value = JSON.parse(result.stdout.replace(/^\uFEFF/, '').trim());
      assert.equal(value.rows.length, 1);
      assert.equal(value.rows[0].memoryStatus === 'confirmed-exited', confirms);
      assert.equal(value.queries, action === 'memory-series' && !root ? 1 : 0);
      assert.equal(value.rows[0].workingSetBytes, null);
    }
  },
);

test(
  'actual startup ownership refresh accepts only a current same-parent identity and never claims a live orphan',
  { skip: process.platform === 'win32' ? false : 'Requires Windows PowerShell 5.1' },
  () => {
    for (const scenario of [
      'default',
      'verified',
      'absent',
      'missing',
      'wrong-parent',
      'live-orphan',
    ]) {
      const script = `
$script:queries=0
$stamp=[DateTime]::UtcNow
$rootRow=[PSCustomObject]@{ProcessId=2147483500;ParentProcessId=1;CreationDate=$stamp;ExecutablePath='C:\\owned\\root.exe';Name='root.exe'}
$childRow=[PSCustomObject]@{ProcessId=2147483504;ParentProcessId=2147483500;CreationDate=$stamp.AddMilliseconds(1);ExecutablePath=$null;Name='child.exe'}
$grandchildRow=[PSCustomObject]@{ProcessId=2147483508;ParentProcessId=2147483504;CreationDate=$stamp.AddMilliseconds(2);ExecutablePath='C:\\owned\\grandchild.exe';Name='grandchild.exe'}
function Get-CimInstance { param($ClassName,$Filter,$Property)
  $script:queries++
  if (!$Filter) { @($rootRow,$childRow) ${scenario === 'live-orphan' ? '+ @($grandchildRow)' : ''}; return }
  if ('${scenario}' -eq 'live-orphan' -and $Filter -eq 'ProcessId = 2147483508') { $grandchildRow; return }
  if ('${scenario}' -in @('absent','live-orphan')) { return }
  $fresh=$childRow.PSObject.Copy()
  if ('${scenario}' -eq 'verified') { $fresh.ExecutablePath='C:\\owned\\child.exe' }
  if ('${scenario}' -eq 'wrong-parent') { $fresh.ExecutablePath='C:\\owned\\child.exe';$fresh.ParentProcessId=2 }
  $fresh
}
[Environment]::SetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_STARTED',([DateTimeOffset]$stamp).ToUnixTimeMilliseconds().ToString())
${ownedProcesses.PROCESS_SCRIPT}
`;
      const result = spawnSync(
        'powershell.exe',
        [
          '-NoLogo',
          '-NoProfile',
          '-NonInteractive',
          '-EncodedCommand',
          Buffer.from(script, 'utf16le').toString('base64'),
        ],
        {
          windowsHide: true,
          encoding: 'utf8',
          timeout: 15000,
          env: {
            ...process.env,
            SOLOSOUL_NATIVE_PERF_PID: '2147483500',
            SOLOSOUL_NATIVE_PERF_EXE: 'C:\\owned\\root.exe',
            SOLOSOUL_NATIVE_PERF_ACTION: scenario === 'default' ? 'sample' : 'memory-series',
            SOLOSOUL_NATIVE_PERF_WEBVIEW_ROOT: '',
            SOLOSOUL_NATIVE_PERF_BROWSER_DATA_DIRECTORY: '',
          },
        },
      );
      assert.equal(result.status, 0, scenario + result.stdout + result.stderr);
      const value = JSON.parse(result.stdout.replace(/^\uFEFF/, '').trim());
      assert.equal(value.rootVerified, true, scenario);
      assert.equal(
        value.unverifiedDescendants,
        !['verified', 'absent'].includes(scenario),
        scenario,
      );
      assert.equal(value.processes.length, scenario === 'verified' ? 2 : 1, scenario);
      if (scenario === 'default') assert.equal(value.ownershipRefreshes, undefined);
      if (scenario === 'verified')
        assert.equal(value.ownershipRefreshes[0].status, 'candidate-fresh-identity-verified');
      if (scenario === 'absent')
        assert.equal(value.ownershipRefreshes[0].status, 'candidate-confirmed-absent');
    }
  },
);

test(
  'batched owned browser details preserve identity and UDF gates without querying foreign command lines',
  { skip: process.platform === 'win32' ? false : 'Requires Windows PowerShell 5.1' },
  () => {
    for (const scenario of [
      'default',
      'batch',
      'changed-time',
      'changed-path',
      'duplicate-browser',
      'wrong-udf',
    ]) {
      const script = `
$stamp=[DateTime]::UtcNow
$script:commandQueries=@()
$script:foreignRead=$false
$rootRow=[PSCustomObject]@{ProcessId=2147483500;ParentProcessId=1;CreationDate=$stamp;ExecutablePath='C:\\owned\\root.exe';Name='root.exe'}
$browser=[PSCustomObject]@{ProcessId=2147483504;ParentProcessId=2147483500;CreationDate=$stamp.AddMilliseconds(1);ExecutablePath='C:\\runtime\\msedgewebview2.exe';Name='msedgewebview2.exe'}
$gpu=[PSCustomObject]@{ProcessId=2147483508;ParentProcessId=2147483504;CreationDate=$stamp.AddMilliseconds(2);ExecutablePath='C:\\runtime\\msedgewebview2.exe';Name='msedgewebview2.exe'}
$renderer=[PSCustomObject]@{ProcessId=2147483512;ParentProcessId=2147483504;CreationDate=$stamp.AddMilliseconds(3);ExecutablePath='C:\\runtime\\msedgewebview2.exe';Name='msedgewebview2.exe'}
$foreign=[PSCustomObject]@{ProcessId=2147483516;ParentProcessId=1;CreationDate=$stamp;ExecutablePath='C:\\foreign\\msedgewebview2.exe';Name='msedgewebview2.exe'}
$script:allFake=@($rootRow,$browser,$gpu,$renderer,$foreign)
function Get-CimInstance { param($ClassName,$Filter,$Property)
  if (!$Filter) {
    if ($Property -contains 'CommandLine') { $script:foreignRead=$true }
    $script:allFake; return
  }
  $ids=@([regex]::Matches($Filter,'ProcessId\\s*=\\s*([0-9]+)') | ForEach-Object {[int]$_.Groups[1].Value})
  if ($Property -contains 'CommandLine') {
    $script:commandQueries+=@{filter=$Filter;ids=$ids}
    if ($ids -contains 2147483516) { $script:foreignRead=$true }
  }
  foreach ($row in $script:allFake | Where-Object {$ids -contains [int]$_.ProcessId}) {
    $detail=$row.PSObject.Copy()
    if ($Property -contains 'CommandLine') {
      $command=if($row.ProcessId -eq 2147483504){'msedgewebview2.exe --user-data-dir="C:\\owned\\webview\\EBWebView"'}else{'msedgewebview2.exe --type=renderer'}
      if ('${scenario}' -eq 'duplicate-browser' -and $row.ProcessId -eq 2147483508) {$command='msedgewebview2.exe --user-data-dir="C:\\owned\\webview\\EBWebView"'}
      if ('${scenario}' -eq 'wrong-udf' -and $row.ProcessId -eq 2147483504) {$command='msedgewebview2.exe --user-data-dir="C:\\foreign\\profile"'}
      $detail | Add-Member -NotePropertyName CommandLine -NotePropertyValue $command
      if ($row.ProcessId -eq 2147483504 -and '${scenario}' -eq 'changed-time') {$detail.CreationDate=$stamp.AddMilliseconds(99)}
      if ($row.ProcessId -eq 2147483504 -and '${scenario}' -eq 'changed-path') {$detail.ExecutablePath='C:\\foreign\\msedgewebview2.exe'}
    }
    $detail
  }
}
[Environment]::SetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_STARTED',([DateTimeOffset]$stamp).ToUnixTimeMilliseconds().ToString())
${ownedProcesses.PROCESS_SCRIPT.replace(
  '$result | ConvertTo-Json -Depth 5 -Compress',
  '$result.commandQueries=$script:commandQueries; $result.foreignRead=$script:foreignRead; $result | ConvertTo-Json -Depth 5 -Compress',
)}
`;
      const scriptRoot = mkdtempSync(path.join(os.tmpdir(), 'rf1092-cim-block-'));
      const scriptPath = path.join(scriptRoot, 'probe.ps1');
      writeFileSync(scriptPath, '\uFEFF' + script, 'utf8');
      let result;
      try {
        result = spawnSync(
          'powershell.exe',
          [
            '-NoLogo',
            '-NoProfile',
            '-NonInteractive',
            '-ExecutionPolicy',
            'Bypass',
            '-File',
            scriptPath,
          ],
          {
            windowsHide: true,
            encoding: 'utf8',
            timeout: 15000,
            env: {
              ...process.env,
              SOLOSOUL_NATIVE_PERF_PID: '2147483500',
              SOLOSOUL_NATIVE_PERF_EXE: 'C:\\owned\\root.exe',
              SOLOSOUL_NATIVE_PERF_ACTION: scenario === 'default' ? 'sample' : 'memory-series',
              SOLOSOUL_NATIVE_PERF_WEBVIEW_ROOT: 'C:\\owned\\webview',
              SOLOSOUL_NATIVE_PERF_BROWSER_DATA_DIRECTORY: 'C:\\owned\\webview\\EBWebView',
            },
          },
        );
      } finally {
        rmSync(scriptRoot, { recursive: true, force: true });
      }
      assert.equal(
        result.status,
        0,
        scenario + (result.error?.message ?? '') + result.stdout + result.stderr,
      );
      const value = JSON.parse(result.stdout.replace(/^\uFEFF/, '').trim());
      assert.equal(value.rootVerified, true, scenario);
      assert.equal(value.foreignRead, false, scenario);
      assert.equal(value.commandQueries.length, scenario === 'default' ? 3 : 1, scenario);
      const ids = value.commandQueries.flatMap((query) => query.ids).sort((a, b) => a - b);
      assert.deepEqual(ids, [2147483504, 2147483508, 2147483512], scenario);
      assert.equal(
        value.userDataDirectoryMatches,
        ['default', 'batch'].includes(scenario),
        scenario,
      );
      assert.equal(
        value.browserDataChecks.length,
        scenario === 'duplicate-browser'
          ? 2
          : ['changed-time', 'changed-path'].includes(scenario)
            ? 0
            : 1,
        scenario,
      );
      if (scenario === 'wrong-udf')
        assert.equal(value.browserDataChecks[0].observedDirectory, null);
      assert.equal(result.stdout.includes('CommandLine'), false);
    }
  },
);

test(
  'actual memory block confirms null properties only for absent descendants and keeps default/root/live failures',
  { skip: process.platform === 'win32' ? false : 'Requires Windows PowerShell 5.1' },
  () => {
    const source = ownedProcesses.PROCESS_SCRIPT;
    const block = source.slice(
      source.indexOf('$processes = @()'),
      source.indexOf('$result = @{rootVerified=$true'),
    );
    const provider = '$process = [Diagnostics.Process]::GetProcessById($row.pid)';
    assert.equal(block.split(provider).length, 2);
    for (const scenario of [
      'null-absent',
      'private-only-absent',
      'working-set-only-absent',
      'null-live',
      'null-reused',
      'null-query-error',
      'null-root',
      'null-default',
      'throw-absent',
      'throw-live',
      'identity-changed-live',
      'valid',
      'valid-zero',
    ]) {
      const absent = scenario.endsWith('-absent');
      const defaultMode = scenario === 'null-default';
      const root = scenario === 'null-root';
      const valid = scenario.startsWith('valid');
      const script = `
$ErrorActionPreference='Stop'
$stamp=[DateTime]::UtcNow
$expectedPid=${root ? 2147483200 : 100}
$action='${defaultMode ? 'sample' : 'memory-series'}'
$owned=@{2147483200=@{pid=2147483200;creationMs=([DateTimeOffset]$stamp).ToUnixTimeMilliseconds();executablePath='C:\\owned\\icacls.exe';name='icacls.exe'}}
$ws=${valid || scenario === 'private-only-absent' ? (scenario === 'valid-zero' ? '0' : '4096') : '$null'}
$private=${valid || scenario === 'working-set-only-absent' ? (scenario === 'valid-zero' ? '0' : '8192') : '$null'}
$fakeProcess=[PSCustomObject]@{StartTime=$stamp;WorkingSet64=$ws;PrivateMemorySize64=$private;TotalProcessorTime=[PSCustomObject]@{TotalMilliseconds=15.625}}
if ('${scenario}' -like 'throw-*') {
  $fakeProcess | Add-Member -Force -MemberType ScriptProperty -Name WorkingSet64 -Value {throw 'simulated getter failure'}
}
if ('${scenario}' -eq 'identity-changed-live') {$fakeProcess.StartTime=$stamp.AddMilliseconds(1)}
$script:queries=@()
function Get-CimInstance { param($ClassName,$Filter,$Property)
  $script:queries+=@([ordered]@{filter=$Filter;properties=$Property})
  if ('${scenario}' -eq 'null-query-error') {throw 'simulated CIM failure'}
  $freshCreation=$stamp; $freshPath='C:\\owned\\icacls.exe'
  if ('${scenario}' -eq 'null-reused') {$freshCreation=$stamp.AddMilliseconds(1);$freshPath='C:\\foreign\\other.exe'}
  ${absent ? '' : '[PSCustomObject]@{ProcessId=2147483200;CreationDate=$freshCreation;ExecutablePath=$freshPath}'}
}
${block.replace(provider, '$process = $fakeProcess')}
@{rows=$processes;queries=$script:queries} | ConvertTo-Json -Depth 5 -Compress
`;
      const dir = mkdtempSync(path.join(os.tmpdir(), 'rf1093-memory-block-'));
      const file = path.join(dir, 'probe.ps1');
      writeFileSync(file, '\uFEFF' + script, 'utf8');
      let result;
      try {
        result = spawnSync(
          'powershell.exe',
          ['-NoLogo', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', file],
          { windowsHide: true, encoding: 'utf8', timeout: 15000 },
        );
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
      assert.equal(result.status, 0, scenario + result.stdout + result.stderr);
      assert.equal(result.stderr.trim(), '', scenario + result.stderr);
      const value = JSON.parse(result.stdout.replace(/^\uFEFF/, '').trim());
      const row = value.rows[0];
      assert.equal(value.rows.length, 1, scenario);
      assert.equal(row.memoryStatus === 'confirmed-exited', absent, scenario);
      assert.equal(value.queries.length, valid || defaultMode || root ? 0 : 1, scenario);
      for (const query of value.queries) {
        assert.equal(query.filter, 'ProcessId = 2147483200', scenario);
        assert.deepEqual(query.properties, ['ProcessId', 'CreationDate', 'ExecutablePath']);
      }
      if (absent) {
        assert.equal(row.workingSetBytes, null);
        assert.equal(row.privateBytes, null);
        assert.equal(row.cpuMs, null);
        assert.ok(Number.isFinite(row.exitConfirmedAtMs));
        assert.equal(
          row.reason,
          'Process exited before memory collection; absence confirmed by fresh CIM',
        );
      } else if (valid) {
        assert.equal(row.reason, null);
        assert.equal(row.workingSetBytes, scenario === 'valid-zero' ? 0 : 4096);
      } else {
        assert.equal(row.memoryStatus, undefined);
        assert.equal(row.workingSetBytes, null);
        assert.equal(defaultMode ? row.reason === null : typeof row.reason === 'string', true);
      }
    }
  },
);
