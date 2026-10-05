#!/usr/bin/env node
/**
 * RF-312 Windows 原生应用采样。仅使用公开合成 Vault；所有 IPC 仍走真实 Tauri。
 * 用法：node scripts/native-perf-run.mjs --exe ABS --fixture ABS --output NEW_ABS --samples 5
 * 此工具保留样本目录及失败证据；不接收真实账户/密码，不捕获 IPC 参数或响应。
 */
import { spawn, execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, realpath, readFile, mkdir, writeFile, access } from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import net from 'node:net';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const PUBLIC_PASSWORD = 'perf-baseline-only-password';
const OBSERVER_SCOPE = 'windows-native-tauri-invoke-observer';
const PHASE_TIMEOUT_MS = 45_000;
const STARTUP_TIMEOUT_MS = 60_000;
const PREPARE_TIMEOUT_MS = 120_000;
const PROCESS_QUERY_TIMEOUT_MS = 15_000;
const PROCESS_CLEANUP_TIMEOUT_MS = 20_000;
const HELP =
  'Usage: node scripts/native-perf-run.mjs --exe ABS --fixture ABS --output NEW_ABS --samples N\nWindows only; N >= 3 (recommended 5). Uses only RF-312 synthetic fixtures. No GUI is started by --help.';

export function parseArgs(args) {
  if (args.length === 1 && args[0] === '--help') return { help: true };
  const values = {};
  const allowed = new Set(['--exe', '--fixture', '--output', '--samples']);
  for (let i = 0; i < args.length; i += 2) {
    const key = args[i];
    if (
      !allowed.has(key) ||
      Object.hasOwn(values, key) ||
      !args[i + 1] ||
      args[i + 1].startsWith('--')
    )
      throw new Error(
        'Expected each required option exactly once: --exe, --fixture, --output, --samples',
      );
    values[key] = args[i + 1];
  }
  if (allowed.size !== Object.keys(values).length)
    throw new Error('Missing required options: --exe, --fixture, --output, --samples');
  if (!/^[1-9]\d*$/.test(values['--samples'])) throw new Error('--samples must be an integer >= 3');
  const samples = Number(values['--samples']);
  if (!Number.isSafeInteger(samples) || samples < 3)
    throw new Error('--samples must be an integer >= 3');
  for (const key of ['--exe', '--fixture', '--output']) {
    if (!path.isAbsolute(values[key])) throw new Error(key + ' must be an absolute path');
  }
  const output = path.resolve(values['--output']);
  if (path.parse(output).root === output) throw new Error('--output cannot be a filesystem root');
  return {
    exe: path.resolve(values['--exe']),
    fixture: path.resolve(values['--fixture']),
    output,
    samples,
  };
}

async function regularPath(target, directory) {
  const stat = await lstat(target);
  if (stat.isSymbolicLink() || (directory ? !stat.isDirectory() : !stat.isFile()))
    throw new Error('Expected a regular ' + (directory ? 'directory' : 'file') + ': ' + target);
  const resolved = await realpath(target);
  if (
    (process.platform === 'win32' ? normalizeWindowsPath(resolved) : resolved.toLowerCase()) !==
    (process.platform === 'win32'
      ? normalizeWindowsPath(path.resolve(target))
      : path.resolve(target).toLowerCase())
  )
    throw new Error('Path traverses a link or alternate directory: ' + target);
  return resolved;
}

export function validateFixtureManifest(value) {
  if (
    value?.schemaVersion !== 1 ||
    value.scope !== 'synthetic-native-vault-fixture' ||
    value.generator !== 'solosoul-core/examples/perf_baseline' ||
    value.fixture !== 'deterministic-20th-object-property-match' ||
    ![100, 5000].includes(value.objectCount) ||
    value.accountId !== 'acc_rf312_' + value.objectCount ||
    value.accountName !== 'Performance Fixture' ||
    value.searchQuery !== 'needle' ||
    value.expectedSearchMatches !== Math.ceil(value.objectCount / 20) ||
    value.buildProfile !== 'release' ||
    value.kdf?.memoryKiB !== 65536 ||
    value.kdf?.iterations !== 3 ||
    value.kdf?.parallelism !== 4 ||
    value.includesProfile !== true ||
    value.includesUiPreferences !== true ||
    value.includesAttachments !== false ||
    value.includesOcrFixture !== false
  )
    throw new Error(
      'Fixture must be the verified Release RF-312 synthetic fixture with production KDF',
    );
  return value;
}

export const NATIVE_PERF_MARKERS = Object.freeze([
  'windows-native-perf-owned',
  'windows-native-perf-ready',
  'windows-native-perf-consumed',
  '--native-perf-prepare',
]);

// 仅排除误传默认构建；常量匹配不保证签名、可信来源或隔离逻辑正确。
export async function nativePerfPreflight(exe, chunkSize = 64 * 1024) {
  if (!Number.isSafeInteger(chunkSize) || chunkSize < 1 || chunkSize > 1024 * 1024)
    throw new Error('Native performance preflight chunk size must be 1..1048576 bytes');
  const markers = NATIVE_PERF_MARKERS.map((value) => Buffer.from(value, 'ascii'));
  const overlapSize = Math.max(...markers.map((value) => value.length)) - 1;
  const matched = new Set();
  let overlap = Buffer.alloc(0);
  let bytesScanned = 0;
  for await (const chunk of createReadStream(exe, { highWaterMark: chunkSize })) {
    bytesScanned += chunk.length;
    const window = Buffer.concat([overlap, chunk]);
    for (let i = 0; i < markers.length; i++)
      if (!matched.has(i) && window.includes(markers[i])) matched.add(i);
    if (matched.size === markers.length) break;
    // 复制短尾部，避免保留整个读取块；支持 marker 跨任意多个小块。
    overlap = Buffer.from(window.subarray(Math.max(0, window.length - overlapSize)));
  }
  const missing = NATIVE_PERF_MARKERS.filter((_, index) => !matched.has(index));
  if (missing.length)
    throw new Error(
      'Refusing to execute --exe: offline native-perf feature markers are missing (' +
        missing.join(', ') +
        '). Build with the native-perf feature; default builds are not allowed.',
    );
  return {
    method: 'streaming-ascii-marker-check',
    matchedMarkers: [...NATIVE_PERF_MARKERS],
    bytesScanned,
    guarantee: 'Excludes accidental default builds; not a binary signature or trust guarantee',
  };
}

export async function validateInputs(options, { media = false } = {}) {
  const exe = await regularPath(options.exe, false);
  if (path.extname(exe).toLowerCase() !== '.exe') throw new Error('--exe must be a Windows .exe');
  const binaryPreflight = await nativePerfPreflight(exe);
  const fixture = await regularPath(options.fixture, true);
  const markerPath = await regularPath(
    path.join(fixture, media ? 'rf312-media-fixture.json' : 'rf312-fixture.json'),
    false,
  );
  const mediaManifest = media
    ? await (await import('./native-perf-media-contract.mjs')).readMediaManifest(fixture)
    : null;
  const manifest = media
    ? mediaManifest.baseFixture
    : validateFixtureManifest(JSON.parse(await readFile(markerPath, 'utf8')));
  await regularPath(path.join(fixture, manifest.accountId), true);
  await regularPath(path.join(fixture, manifest.accountId, 'vault.db'), false);
  await regularPath(path.join(fixture, manifest.accountId, 'config.json'), false);
  await regularPath(path.join(fixture, 'ui_preferences.json'), false);
  await regularPath(path.dirname(options.output), true);
  const relative = path.relative(fixture, options.output);
  if (
    !relative ||
    (!relative.startsWith('..' + path.sep) && relative !== '..' && !path.isAbsolute(relative))
  )
    throw new Error('--output must not be inside the source fixture');
  try {
    await lstat(options.output);
    throw new Error('--output must not already exist');
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
  return {
    ...options,
    exe,
    fixture,
    manifest,
    markerPath,
    binaryPreflight,
    ...(media ? { mediaManifest } : {}),
  };
}

export function normalizeWindowsPath(value) {
  if (typeof value !== 'string' || !path.win32.isAbsolute(value))
    throw new Error('Expected an absolute Windows path');
  let normalized = value;
  if (normalized.startsWith('\\\\?\\UNC\\')) normalized = '\\\\' + normalized.slice(8);
  else if (normalized.startsWith('\\\\?\\')) normalized = normalized.slice(4);
  return path.win32.normalize(normalized).toLowerCase();
}

export function browserDataDirectoryCheck(apiUserDataFolder, actualBrowserDataDirectory) {
  normalizeWindowsPath(apiUserDataFolder);
  // WebView2 Runtime 在API UDF之下自动追加唯一的 EBWebView 目录。
  const expectedBrowserDataDirectory = path.win32.join(apiUserDataFolder, 'EBWebView');
  let matchesExpected = false;
  try {
    matchesExpected =
      normalizeWindowsPath(actualBrowserDataDirectory) ===
      normalizeWindowsPath(expectedBrowserDataDirectory);
  } catch {}
  return {
    apiUserDataFolder,
    expectedBrowserDataDirectory,
    matchesExpected,
    actualBrowserDataDirectory: matchesExpected ? actualBrowserDataDirectory : null,
    reason: matchesExpected
      ? null
      : 'Owned browser user-data-dir is not the exact API UDF/EBWebView directory; unexpected path redacted',
  };
}

export async function browserDataFilesystemCheck(apiUserDataFolder) {
  const expectedBrowserDataDirectory = path.win32.join(apiUserDataFolder, 'EBWebView');
  try {
    await regularPath(apiUserDataFolder, true);
    await regularPath(expectedBrowserDataDirectory, true);
    return { valid: true, reason: null };
  } catch (error) {
    return { valid: false, reason: safeError(error) };
  }
}

export function safeError(error) {
  const raw = String(error?.message ?? error)
    .split('\nCall log:')[0]
    .split('\n')
    .slice(0, 2)
    .join(' ');
  return raw.replaceAll(PUBLIC_PASSWORD, '[synthetic-password-redacted]').slice(0, 1000);
}

export function validateObserver(value, runId) {
  const identityMatched =
    value?.scope === OBSERVER_SCOPE && value?.schemaVersion === 1 && value?.runId === runId;
  const invalid = (reason) => ({
    valid: false,
    identityMatched,
    reason,
    total: null,
    commands: [],
    installedAtMs: Number.isFinite(value?.installedAtMs) ? value.installedAtMs : null,
    timeOriginMs: Number.isFinite(value?.timeOriginMs) ? value.timeOriginMs : null,
  });
  if (value?.scope !== OBSERVER_SCOPE || value.schemaVersion !== 1 || value.runId !== runId)
    return invalid('Observer identity/schema does not match this prepared native run');
  if (value.valid !== true)
    return invalid(
      'Observer invalid: ' +
        (Array.isArray(value.invalidReasons)
          ? value.invalidReasons
              .filter((x) => typeof x === 'string')
              .map((x) => x.slice(0, 100))
              .join(', ')
          : 'no reason supplied'),
    );
  if (
    !Number.isSafeInteger(value.total) ||
    value.total < 0 ||
    value.total !== value.observedCount ||
    !Array.isArray(value.commands) ||
    value.commands.length !== value.total ||
    !Number.isFinite(value.installedAtMs) ||
    !Number.isFinite(value.timeOriginMs)
  )
    return invalid('Observer count/timing fields are incomplete or inconsistent');
  const commands = [];
  for (const item of value.commands) {
    if (
      !item ||
      typeof item.command !== 'string' ||
      !/^[a-zA-Z0-9_:\-|.]+$/.test(item.command) ||
      item.command.length > 180 ||
      !Number.isFinite(item.atMs) ||
      item.atMs < 0
    )
      return invalid('Observer event is malformed; no count is reported');
    commands.push({ command: item.command, atMs: item.atMs });
  }
  return {
    valid: true,
    identityMatched: true,
    total: value.total,
    commands,
    installedAtMs: value.installedAtMs,
    timeOriginMs: value.timeOriginMs,
    reason: null,
  };
}

export function ipcDelta(before, after) {
  if (!before.valid || !after.valid)
    return { attempts: null, commands: null, reason: before.reason ?? after.reason };
  if (
    after.total < before.total ||
    before.timeOriginMs !== after.timeOriginMs ||
    before.installedAtMs !== after.installedAtMs ||
    before.commands.some(
      (event, index) =>
        event.command !== after.commands[index]?.command ||
        event.atMs !== after.commands[index]?.atMs,
    )
  )
    return {
      attempts: null,
      commands: null,
      reason: 'Observer reset or prefix changed during phase',
    };
  const counts = new Map();
  for (const item of after.commands.slice(before.total))
    counts.set(item.command, (counts.get(item.command) ?? 0) + 1);
  return {
    attempts: after.total - before.total,
    commands: [...counts].map(([command, attempts]) => ({ command, attempts })),
    reason: null,
  };
}

export function powerShellError(error, action, timeoutMs) {
  const stderr = String(error?.stderr ?? '').trim();
  let detail = stderr;
  if (stderr.startsWith('#< CLIXML')) {
    const entries = [...stderr.matchAll(/<S\b([^>]*)>([\s\S]*?)<\/S>/g)]
      .filter((match) => /\bS=["']Error["']/.test(match[1]))
      .map((match) =>
        match[2]
          .replace(/&#x([0-9a-f]+);|&#([0-9]+);/gi, (_, hex, decimal) =>
            String.fromCodePoint(Number.parseInt(hex ?? decimal, hex ? 16 : 10)),
          )
          .replace(
            /&lt;|&gt;|&quot;|&apos;|&amp;/g,
            (entity) =>
              ({ '&lt;': '<', '&gt;': '>', '&quot;': '"', '&apos;': "'", '&amp;': '&' })[entity],
          )
          .replace(/_x([0-9a-f]{4})_/gi, (_, hex) => String.fromCharCode(Number.parseInt(hex, 16))),
      );
    detail = entries.join('\n').trim();
  }
  const status = error?.killed
    ? 'timed out after ' + timeoutMs + 'ms'
    : 'failed' + (error?.code !== undefined ? ' (code ' + String(error.code) + ')' : '');
  // 不使用 execFile.message：它会先输出整段EncodedCommand而遮蔽实际stderr。
  return safeError(
    new Error(
      'PowerShell ' + action + ' ' + status + (detail ? ': ' + detail : '; no stderr error detail'),
    ),
  );
}

export function releaseUnfinishedChild(child, exit) {
  if (
    !child ||
    child.exitCode !== null ||
    child.signalCode ||
    Number.isInteger(exit?.exitCode) ||
    (typeof exit?.signal === 'string' && !!exit.signal)
  )
    return false;
  child.unref();
  return true; // 仅解除runner的引用；不声称结束了仍在运行的进程。
}

// 固定脚本仅查询本次根进程与可验证后代；不输出其他进程的命令行或内容。
export const PROCESS_SCRIPT = `
$ErrorActionPreference = 'Stop'
$expectedPid = [int][Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_PID')
$expectedExe = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_EXE')
$expectedStarted = [long][Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_STARTED')
$action = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_ACTION')
$all = @(Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CreationDate,ExecutablePath,Name)
function Identity($row) {
  if (!$row.CreationDate -or !$row.ExecutablePath) { return $null }
  $created = ([DateTimeOffset]$row.CreationDate.ToUniversalTime()).ToUnixTimeMilliseconds()
  return @{pid=[int]$row.ProcessId; parentPid=[int]$row.ParentProcessId; creationMs=$created; executablePath=[string]$row.ExecutablePath; name=[string]$row.Name}
}
if ($action -eq 'cleanup') {
  # PowerShell 5.1 ConvertFrom-Json在管道中返回一个JSON数组对象；直接@(...)会嵌套。
  $parsedKnown = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_KNOWN') | ConvertFrom-Json
  $known = @($parsedKnown)
  $results = @()
  $waits = @()
  $cleanupDeadline = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() + 5000
  foreach ($owned in $known) {
    $row = $all | Where-Object { [int]$_.ProcessId -eq [int]$owned.pid } | Select-Object -First 1
    $identity = if ($row) { Identity $row } else { $null }
    if (!$row) { $results += @{pid=$owned.pid; status='already-exited'}; continue }
    if (!$identity) { $results += @{pid=$owned.pid; status='unverifiable-not-terminated'}; continue }
    if ($identity.creationMs -ne [long]$owned.creationMs -or $identity.executablePath -ine [string]$owned.executablePath) {
      $results += @{pid=$owned.pid; status='identity-changed-not-terminated'}; continue
    }
    try {
      $process = [Diagnostics.Process]::GetProcessById([int]$owned.pid)
      $actualStarted = ([DateTimeOffset]$process.StartTime.ToUniversalTime()).ToUnixTimeMilliseconds()
      if ($actualStarted -ne [long]$owned.creationMs) { throw 'Process identity changed' }
      $process.Kill()
      $results += @{pid=$owned.pid; status='termination-pending'}
      $waits += @{process=$process; resultIndex=$results.Count-1}
    } catch { $results += @{pid=$owned.pid; status='termination-failed'} }
  }
  # 所有已验证进程共享5秒退出预算，避免N个进程串行等待5*N秒。
  foreach ($pending in $waits) {
    try {
      $remaining = [int][Math]::Max(0, $cleanupDeadline - [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())
      $pending.process.WaitForExit($remaining) | Out-Null
      if ($pending.process.HasExited) { $results[$pending.resultIndex].status = 'terminated' }
    } catch { $results[$pending.resultIndex].status = 'termination-failed' }
    finally { $pending.process.Dispose() }
  }
  @{cleanup=$results} | ConvertTo-Json -Depth 5 -Compress
  exit 0
}
$root = $all | Where-Object { [int]$_.ProcessId -eq $expectedPid } | Select-Object -First 1
$identity = if ($root) { Identity $root } else { $null }
if (!$identity -or $identity.executablePath -ine $expectedExe -or [Math]::Abs($identity.creationMs-$expectedStarted) -gt 10000) {
  @{rootVerified=$false; reason='Root process exited or identity cannot be verified'; processes=@()} | ConvertTo-Json -Depth 5 -Compress
  exit 0
}
$owned = @{$expectedPid=$identity}
$unverifiedDescendants = $false
$ownershipRefreshes = @()
$refreshedMissing = @{}
$confirmedAbsent = @{}
do {
  $added = $false
  foreach ($row in $all) {
    $parent = [int]$row.ParentProcessId
    $pidValue = [int]$row.ProcessId
    if ($owned.ContainsKey($pidValue) -or !$owned.ContainsKey($parent)) { continue }
    if ($confirmedAbsent.ContainsKey($pidValue)) { continue }
    $child = Identity $row
    if (!$child -and $action -eq 'memory-series') {
      # startup CIM 行可能先出现 PID，随后才有路径。每个缺字段候选至多复核一次，绝不读取未核验进程内存。
      if (!$refreshedMissing.ContainsKey($pidValue)) {
        $refreshedMissing[$pidValue] = $null
        try {
          $freshRows = @(Get-CimInstance Win32_Process -Filter ("ProcessId = " + $pidValue) -Property ProcessId,ParentProcessId,CreationDate,ExecutablePath,Name)
          if ($freshRows.Count -eq 0) {
            $confirmedAbsent[$pidValue] = $true
            $ownershipRefreshes += @{pid=$pidValue;parentPid=$parent;status='candidate-confirmed-absent';creationMs=$null}
          } elseif ($freshRows.Count -eq 1) {
            $freshChild = Identity $freshRows[0]
            if ($freshChild -and $freshChild.parentPid -eq $parent -and $freshChild.creationMs -ge $owned[$parent].creationMs) {
              $refreshedMissing[$pidValue] = $freshChild
              $ownershipRefreshes += @{pid=$pidValue;parentPid=$parent;status='candidate-fresh-identity-verified';creationMs=$freshChild.creationMs}
            }
          }
        } catch { }
      }
      if ($confirmedAbsent.ContainsKey($pidValue)) { continue }
      $child = $refreshedMissing[$pidValue]
    }
    if (!$child) { $unverifiedDescendants = $true; continue }
    if ($child -and $child.creationMs -ge $owned[$parent].creationMs) { $owned[$pidValue]=$child; $added=$true }
  }
} while ($added)
# 身份未完整记录就消失的候选不能承接活后代归属；沿原发现树有界核对，活孤儿继续拒绝。
if ($action -eq 'memory-series') {
  do {
    $absentAdded = $false
    foreach ($row in $all) {
      $pidValue = [int]$row.ProcessId
      $parent = [int]$row.ParentProcessId
      if (!$confirmedAbsent.ContainsKey($parent) -or $confirmedAbsent.ContainsKey($pidValue)) { continue }
      try {
        $remaining = @(Get-CimInstance Win32_Process -Filter ("ProcessId = " + $pidValue) -Property ProcessId,CreationDate,ExecutablePath)
        if ($remaining.Count -eq 0) {
          $confirmedAbsent[$pidValue] = $true
          $ownershipRefreshes += @{pid=$pidValue;parentPid=$parent;status='candidate-confirmed-absent';creationMs=$null}
          $absentAdded = $true
        } else { $unverifiedDescendants = $true }
      } catch { $unverifiedDescendants = $true }
    }
  } while ($absentAdded)
}
$browserChecks = @()
$expectedWebview = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_WEBVIEW_ROOT')
$expectedBrowserData = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_BROWSER_DATA_DIRECTORY')
function NormalizePath($value) {
  $value = [string]$value
  if ($value.StartsWith('\\\\?\\UNC\\')) { $value = '\\\\' + $value.Substring(8) }
  elseif ($value.StartsWith('\\\\?\\')) { $value = $value.Substring(4) }
  return [IO.Path]::GetFullPath($value).TrimEnd('\\').ToLowerInvariant()
}
if ($expectedWebview -and $expectedBrowserData) {
  $ownedBrowsers = @($owned.Values | Where-Object { $_.name -ieq 'msedgewebview2.exe' })
  $browserDetails = @()
  if ($action -eq 'memory-series' -and $ownedBrowsers.Count -gt 0) {
    # 仅对已核验的PID集合读取命令行；不在全进程查询中增加CommandLine，不缓存归属。
    $ownedFilter = ($ownedBrowsers | ForEach-Object { "ProcessId = " + [int]$_.pid }) -join ' OR '
    $browserDetails = @(Get-CimInstance Win32_Process -Filter $ownedFilter -Property ProcessId,CreationDate,ExecutablePath,CommandLine)
  }
  foreach ($ownedBrowser in $ownedBrowsers) {
    # 仅已验证的自有WebView2后代才读取CommandLine，且不输出原始内容或非受控路径。
    if ($action -eq 'memory-series') {
      $matchingDetails = @($browserDetails | Where-Object { [int]$_.ProcessId -eq $ownedBrowser.pid })
      if ($matchingDetails.Count -ne 1) { continue }
      $details = $matchingDetails[0]
    } else {
      $details = Get-CimInstance Win32_Process -Filter ("ProcessId = " + $ownedBrowser.pid) -Property ProcessId,CreationDate,ExecutablePath,CommandLine
    }
    $detailsIdentity = Identity $details
    if (!$detailsIdentity -or $detailsIdentity.creationMs -ne $ownedBrowser.creationMs -or $detailsIdentity.executablePath -ine $ownedBrowser.executablePath) { continue }
    $command = [string]$details.CommandLine
    if ($command -match '(?:^|\\s)--type(?:=|\\s)') { continue }
    $match = [regex]::Match($command, '(?:^|\\s)--user-data-dir(?:=|\\s+)(?:"(?<quoted>[^"]+)"|(?<bare>[^\\s]+))')
    $matchesExpected = $false
    if ($match.Success) {
      $candidate = if ($match.Groups['quoted'].Success) { $match.Groups['quoted'].Value } else { $match.Groups['bare'].Value }
      try { $matchesExpected = (NormalizePath $candidate) -eq (NormalizePath $expectedBrowserData) } catch {}
    }
    $browserChecks += @{pid=$ownedBrowser.pid; apiUserDataFolder=$expectedWebview; expectedBrowserDataDirectory=$expectedBrowserData; matchesExpected=$matchesExpected; observedDirectory=if($matchesExpected){$candidate}else{$null}; reason=if($matchesExpected){$null}else{'Browser user-data-dir missing or not the exact prepared UDF/EBWebView directory; unexpected path redacted'}}
  }
}
$browserDataMatched = !$unverifiedDescendants -and $browserChecks.Count -eq 1 -and $browserChecks[0].matchesExpected
$processes = @()
foreach ($row in $owned.Values) {
  $workingSet = $null; $privateBytes = $null; $cpuMs = $null; $reason = $null
  $memoryStatus = $null; $exitConfirmedAtMs = $null
  try {
    $process = [Diagnostics.Process]::GetProcessById($row.pid)
    $created = ([DateTimeOffset]$process.StartTime.ToUniversalTime()).ToUnixTimeMilliseconds()
    if ($created -ne $row.creationMs) { throw 'Process identity changed' }
    $workingSet = $process.WorkingSet64; $privateBytes=$process.PrivateMemorySize64; $cpuMs=$process.TotalProcessorTime.TotalMilliseconds
  } catch {
    $reason = 'Process memory or identity unavailable'
  }
  # 短命进程的内存属性可能为空但不抛异常；仅连续采样将此情况纳入同一次退出核验。
  if ($action -eq 'memory-series' -and ($reason -or $null -eq $workingSet -or $null -eq $privateBytes)) {
    if (!$reason) { $reason = 'Process memory reading incomplete' }
    # 一次固定 fresh CIM 核验仍有 PID 时拒绝；根进程和默认采样不走后代退出确认。
    if ($row.pid -ne $expectedPid) {
      try {
        $remaining = @(Get-CimInstance Win32_Process -Filter ("ProcessId = " + $row.pid) -Property ProcessId,CreationDate,ExecutablePath)
        if ($remaining.Count -eq 0) {
          $workingSet = $null; $privateBytes = $null; $cpuMs = $null
          $memoryStatus = 'confirmed-exited'
          $exitConfirmedAtMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
          $reason = 'Process exited before memory collection; absence confirmed by fresh CIM'
        }
      } catch { }
    }
  }
  $values = @{workingSetBytes=$workingSet; privateBytes=$privateBytes; cpuMs=$cpuMs; reason=$reason}
  if ($memoryStatus) { $values.memoryStatus=$memoryStatus; $values.exitConfirmedAtMs=$exitConfirmedAtMs }
  $processes += $row + $values
}
$result = @{rootVerified=$true; reason=$null; processes=$processes; browserDataChecks=$browserChecks; userDataDirectoryMatches=$browserDataMatched; unverifiedDescendants=$unverifiedDescendants}
if ($action -eq 'memory-series') { $result.ownershipRefreshes=$ownershipRefreshes }
$result | ConvertTo-Json -Depth 5 -Compress
`;

export function verifiedOwnedRows(snapshot, expected) {
  if (snapshot?.rootVerified !== true || !Array.isArray(snapshot.processes)) return [];
  const rows = snapshot.processes.filter(
    (row) =>
      Number.isSafeInteger(row.pid) &&
      row.pid > 0 &&
      Number.isSafeInteger(row.parentPid) &&
      Number.isFinite(row.creationMs) &&
      typeof row.executablePath === 'string' &&
      path.win32.isAbsolute(row.executablePath),
  );
  const root = rows.find(
    (row) =>
      row.pid === expected.pid &&
      normalizeWindowsPath(row.executablePath) === normalizeWindowsPath(expected.exe) &&
      Math.abs(row.creationMs - expected.startedAt) <= 10_000,
  );
  if (!root) return [];
  const owned = new Map([[root.pid, root]]);
  let changed;
  do {
    changed = false;
    for (const row of rows) {
      const parent = owned.get(row.parentPid);
      if (!owned.has(row.pid) && parent && row.creationMs >= parent.creationMs) {
        owned.set(row.pid, row);
        changed = true;
      }
    }
  } while (changed);
  return [...owned.values()];
}

async function processQuery(expected, action = 'sample', known = []) {
  const encoded = Buffer.from(PROCESS_SCRIPT, 'utf16le').toString('base64');
  const timeoutMs = action === 'cleanup' ? PROCESS_CLEANUP_TIMEOUT_MS : PROCESS_QUERY_TIMEOUT_MS;
  try {
    const { stdout } = await execFileAsync(
      'powershell.exe',
      ['-NoLogo', '-NoProfile', '-NonInteractive', '-EncodedCommand', encoded],
      {
        windowsHide: true,
        timeout: timeoutMs,
        maxBuffer: 1024 * 1024,
        env: {
          ...process.env,
          SOLOSOUL_NATIVE_PERF_PID: String(expected.pid),
          SOLOSOUL_NATIVE_PERF_EXE: expected.exe,
          SOLOSOUL_NATIVE_PERF_STARTED: String(expected.startedAt),
          SOLOSOUL_NATIVE_PERF_ACTION: action,
          SOLOSOUL_NATIVE_PERF_WEBVIEW_ROOT: expected.webview
            ? normalizeWindowsPath(expected.webview)
            : '',
          SOLOSOUL_NATIVE_PERF_BROWSER_DATA_DIRECTORY: expected.webview
            ? normalizeWindowsPath(
                browserDataDirectoryCheck(expected.webview, null).expectedBrowserDataDirectory,
              )
            : '',
          SOLOSOUL_NATIVE_PERF_KNOWN: JSON.stringify(known),
        },
      },
    );
    return JSON.parse(stdout.replace(/^\uFEFF/, '').trim());
  } catch (error) {
    throw new Error(powerShellError(error, action, timeoutMs));
  }
}

// 默认查询仍将缺失读数判失败；只有显式连续采样接受带 fresh-CIM 确认的已退出后代。
export function partitionMemoryRows(rows, rootPid, confirmedExitMode = false) {
  const exitedRows = rows.filter(
    (row) =>
      confirmedExitMode &&
      row.pid !== rootPid &&
      row.memoryStatus === 'confirmed-exited' &&
      row.workingSetBytes === null &&
      Number.isFinite(row.exitConfirmedAtMs) &&
      row.exitConfirmedAtMs >= row.creationMs &&
      row.exitConfirmedAtMs <= Date.now() &&
      row.reason === 'Process exited before memory collection; absence confirmed by fresh CIM',
  );
  const exited = new Set(exitedRows);
  const liveRows = rows.filter((row) => !exited.has(row));
  const complete =
    liveRows.some((row) => row.pid === rootPid) &&
    liveRows.every(
      (row) =>
        !row.memoryStatus &&
        row.reason === null &&
        Number.isFinite(row.workingSetBytes) &&
        row.workingSetBytes >= 0,
    );
  return {
    complete,
    liveRows,
    exitedRows,
    workingSetBytes: complete ? liveRows.reduce((sum, row) => sum + row.workingSetBytes, 0) : null,
  };
}

export class OwnedProcess {
  constructor(child, exe, startedAt, webview) {
    this.child = child;
    this.expected = { pid: child.pid, exe, startedAt, webview };
    this.known = new Map();
    this.unverifiedDescendants = false;
  }
  async sample({ confirmExitedDescendants = false } = {}) {
    const began = performance.now();
    try {
      if (this.child.exitCode !== null || this.child.signalCode)
        throw new Error(
          'Originally spawned root process has exited; no new process identity is accepted',
        );
      if (
        typeof confirmExitedDescendants !== 'boolean' ||
        (confirmExitedDescendants && !this.expected.webview)
      )
        throw new Error('Confirmed-exit sampling requires an explicit owned WebView tree');
      const snapshot = await processQuery(
        this.expected,
        confirmExitedDescendants ? 'memory-series' : 'sample',
      );
      if (this.child.exitCode !== null || this.child.signalCode)
        throw new Error('Originally spawned root process exited during sampling');
      const rows = verifiedOwnedRows(snapshot, this.expected);
      if (snapshot.unverifiedDescendants === true) this.unverifiedDescendants = true;
      for (const row of rows) this.known.set(row.pid + ':' + row.creationMs, row);
      const memory = partitionMemoryRows(rows, this.expected.pid, confirmExitedDescendants);
      const complete = snapshot.unverifiedDescendants !== true && memory.complete;
      const reportedRows = confirmExitedDescendants ? memory.liveRows : rows;
      const browserDataChecks =
        this.expected.webview && Array.isArray(snapshot.browserDataChecks)
          ? snapshot.browserDataChecks.map((check) => {
              const directory = browserDataDirectoryCheck(
                this.expected.webview,
                check.observedDirectory,
              );
              return {
                pid: check.pid,
                ...directory,
                matchesExpected: check.matchesExpected === true && directory.matchesExpected,
              };
            })
          : [];
      const exactBrowserDirectory =
        snapshot.userDataDirectoryMatches === true &&
        browserDataChecks.length === 1 &&
        browserDataChecks[0].matchesExpected;
      const browserDataFilesystem = exactBrowserDirectory
        ? await browserDataFilesystemCheck(this.expected.webview)
        : null;
      return {
        observedAt: new Date().toISOString(),
        collectionMs: performance.now() - began,
        workingSetBytes: complete ? memory.workingSetBytes : null,
        reason: complete
          ? null
          : (snapshot.reason ?? 'At least one verified process memory reading is unavailable'),
        userDataDirectoryMatches: exactBrowserDirectory && browserDataFilesystem?.valid === true,
        browserDataChecks,
        browserDataFilesystem,
        ...(confirmExitedDescendants
          ? {
              ownershipRefreshes: snapshot.ownershipRefreshes ?? [],
              exitedProcesses: memory.exitedRows.map((row) => ({
                pid: row.pid,
                parentPid: row.parentPid,
                creationMs: row.creationMs,
                executableName: path.win32.basename(row.executablePath),
                exitConfirmedAtMs: row.exitConfirmedAtMs,
                reason: row.reason,
              })),
            }
          : {}),
        processCount: reportedRows.length,
        processes: reportedRows.map((row) => ({
          pid: row.pid,
          parentPid: row.parentPid,
          creationMs: row.creationMs,
          executableName: path.win32.basename(row.executablePath),
          workingSetBytes: Number.isFinite(row.workingSetBytes) ? row.workingSetBytes : null,
          privateBytes: Number.isFinite(row.privateBytes) ? row.privateBytes : null,
          cpuMs: Number.isFinite(row.cpuMs) ? row.cpuMs : null,
          reason: row.reason ?? null,
        })),
      };
    } catch (error) {
      return {
        observedAt: new Date().toISOString(),
        collectionMs: performance.now() - began,
        workingSetBytes: null,
        processCount: null,
        processes: null,
        reason: safeError(error),
      };
    }
  }
  async cleanup() {
    await this.sample();
    if (!this.known.size)
      return { cleanup: [], reason: 'No verifiable owned process; no PID was terminated' };
    // 后代先终止；根在采样时可验证，即使根先退出，也逐一核对已观测的后代身份。
    const rows = [...this.known.values()].sort((a, b) =>
      a.pid === this.expected.pid
        ? 1
        : b.pid === this.expected.pid
          ? -1
          : b.creationMs - a.creationMs || b.pid - a.pid,
    );
    try {
      const result = await processQuery(
        this.expected,
        'cleanup',
        rows.map(({ pid, creationMs, executablePath }) => ({ pid, creationMs, executablePath })),
      );
      return { ...result, unverifiedDescendants: this.unverifiedDescendants };
    } catch (error) {
      return { cleanup: [], reason: safeError(error) };
    }
  }
}

export function cleanupOutcome(cleanup, exit) {
  const rows = cleanup?.cleanup;
  const complete =
    Array.isArray(rows) &&
    rows.length > 0 &&
    !cleanup.reason &&
    cleanup.unverifiedDescendants !== true &&
    rows.every((row) => ['terminated', 'already-exited'].includes(row.status)) &&
    (Number.isInteger(exit?.exitCode) || (typeof exit?.signal === 'string' && !!exit.signal));
  return {
    complete,
    reason: complete
      ? null
      : 'Owned process cleanup or exit could not be fully verified; further samples were stopped',
  };
}

export function startChild(exe, args, env) {
  // stdio 不捕获：产品日志可能包含内容；退出码和失败阶段另存 JSON。
  const child = spawn(exe, args, { windowsHide: true, shell: false, stdio: 'ignore', env });
  const completion = new Promise((resolve) => {
    child.once('error', (error) =>
      resolve({ exitCode: null, signal: null, error: safeError(error) }),
    );
    child.once('exit', (exitCode, signal) => resolve({ exitCode, signal, error: null }));
  });
  return { child, completion };
}

export function deadline(promise, ms, description) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(
        () => reject(new Error(description + ' timed out after ' + ms + 'ms')),
        ms,
      );
    }),
  ]).finally(() => clearTimeout(timer));
}

export async function unusedPort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const port = server.address().port;
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return port;
}

export async function sha256(file) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}

export async function newJson(file, value) {
  await writeFile(file, JSON.stringify(value, null, 2) + '\n', { flag: 'wx' });
}

export async function rejectDiagnosticBenchmark(root) {
  for (const filename of [
    'native-perf-chromium-log.json',
    'native-perf-sdk-cdp-requested.json',
    'native-perf-sdk-cdp.json',
    'native-perf-sdk-journey-requested.json',
    'native-perf-sdk-journey.json',
  ]) {
    try {
      await lstat(path.join(root, filename));
    } catch (error) {
      if (error.code === 'ENOENT') continue;
      throw error;
    }
    throw new Error(
      'Native diagnostic run is diagnostic only and cannot be used as a performance sample',
    );
  }
}
export function preparedManifest(value, sampleRoot, fixture, fixtureSource) {
  if (
    value?.scope !== 'windows-native-perf-owned' ||
    value.schemaVersion !== 1 ||
    typeof value.runId !== 'string' ||
    !/^[a-f0-9]{32}$/.test(value.runId) ||
    value.identifier !== 'com.solosoul.rf312perf.' + value.runId ||
    normalizeWindowsPath(value.root) !== normalizeWindowsPath(sampleRoot) ||
    value.nativePerfFeature !== true ||
    value.preparationStatus !== 'ready' ||
    typeof value.appVersion !== 'string' ||
    !value.appVersion ||
    normalizeWindowsPath(value.webview) !==
      normalizeWindowsPath(path.win32.join(sampleRoot, 'webview')) ||
    normalizeWindowsPath(value.vault) !==
      normalizeWindowsPath(path.win32.join(sampleRoot, 'vault')) ||
    (fixtureSource !== undefined &&
      normalizeWindowsPath(value.fixtureSource) !== normalizeWindowsPath(fixtureSource)) ||
    value.fixture?.objectCount !== fixture.objectCount ||
    value.fixture?.accountId !== fixture.accountId
  )
    throw new Error('Native preparation ownership manifest does not match this synthetic sample');
  return value;
}

async function twoFrames(page) {
  await deadline(
    page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    ),
    5000,
    'Two application animation frames',
  );
}

async function getObserver(page, runId, guard) {
  try {
    const value = await page.evaluate(() => window.__SOLOSOUL_NATIVE_PERF__?.snapshot?.() ?? null);
    const observer = validateObserver(value, runId);
    if (guard) {
      const integrity = await guard.check(observer);
      if (!integrity.valid)
        return {
          ...observer,
          valid: false,
          reason: integrity.reasons.join('; '),
          total: null,
          commands: [],
        };
    }
    return observer;
  } catch (error) {
    return { valid: false, reason: safeError(error), total: null, commands: [] };
  }
}

// 主文档之外的调用无法保证由此observer覆盖；一旦越界，全样本IPC计数失效。
export function documentIntegrity(
  { pageCount, frameCount, currentTimeOrigin, observerTimeOrigin },
  baseline,
) {
  const reasons = [];
  if (pageCount !== 1) reasons.push('Native session has an additional or missing WebView page');
  if (frameCount !== 1) reasons.push('Native session has an additional or missing frame');
  if (currentTimeOrigin !== baseline)
    reasons.push('Main document timeOrigin changed; SPA-only observation was lost');
  if (observerTimeOrigin !== undefined && observerTimeOrigin !== baseline)
    reasons.push('Observer timeOrigin does not match the original main document');
  return reasons;
}

function createObservationGuard(browser, page, baseline) {
  const reasons = new Set();
  const contexts = browser.contexts();
  const markPage = (other) => {
    if (other !== page) reasons.add('A new WebView page appeared during sampling');
  };
  const markFrame = () => reasons.add('An additional frame appeared during sampling');
  const markPopup = () => reasons.add('A popup appeared during sampling');
  for (const context of contexts) context.on('page', markPage);
  page.on('frameattached', markFrame);
  page.on('popup', markPopup);
  return {
    reasons,
    async check(observer) {
      if (observer && !observer.valid) reasons.add('Native observer invalid: ' + observer.reason);
      try {
        const currentTimeOrigin = await page.evaluate(() => performance.timeOrigin);
        const pageCount = browser
          .contexts()
          .reduce((sum, context) => sum + context.pages().length, 0);
        for (const reason of documentIntegrity(
          {
            pageCount,
            frameCount: page.frames().length,
            currentTimeOrigin,
            observerTimeOrigin: observer?.timeOriginMs,
          },
          baseline,
        ))
          reasons.add(reason);
      } catch (error) {
        reasons.add('Main document integrity could not be checked: ' + safeError(error));
      }
      return { valid: reasons.size === 0, reasons: [...reasons] };
    },
    dispose() {
      for (const context of contexts) context.off('page', markPage);
      page.off('frameattached', markFrame);
      page.off('popup', markPopup);
    },
  };
}

async function phase(page, owner, sample, runId, guard, name, action) {
  sample.activePhase = name;
  const before = await getObserver(page, runId, guard);
  const began = performance.now();
  let failed;
  let details;
  try {
    details = await deadline(action(), PHASE_TIMEOUT_MS, name);
    await twoFrames(page);
  } catch (error) {
    failed = safeError(error);
  }
  const durationMs = performance.now() - began;
  const after = await getObserver(page, runId, guard);
  const record = {
    name,
    success: !failed,
    durationMs,
    error: failed ?? null,
    details: details ?? null,
    ipc: ipcDelta(before, after),
    memory: await owner.sample(),
  };
  sample.phases.push(record);
  if (failed) throw new Error(name + ': ' + failed);
  return record;
}

async function connectMainPage(chromium, port, child, runId) {
  const began = performance.now();
  let lastError = 'No native WebView2 CDP endpoint';
  while (performance.now() - began < STARTUP_TIMEOUT_MS) {
    if (child.exitCode !== null || child.signalCode)
      throw new Error('Native application exited before CDP attachment');
    let browser;
    try {
      browser = await chromium.connectOverCDP('http://127.0.0.1:' + port, { timeout: 3000 });
      for (const context of browser.contexts()) {
        for (const page of context.pages()) {
          const observer = await getObserver(page, runId);
          if (observer.identityMatched)
            return { browser, page, attachMs: performance.now() - began };
        }
      }
      lastError = 'CDP target does not have the observer identity for this run';
    } catch (error) {
      lastError = safeError(error);
    }
    if (browser) await browser.close().catch(() => {});
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error('CDP attachment failed: ' + lastError);
}

async function passwordUnlock(page) {
  const input = page.locator('[data-login-method-region="password"] input');
  await input.waitFor({ state: 'visible', timeout: PHASE_TIMEOUT_MS });
  await input.fill(PUBLIC_PASSWORD);
  await page.locator('[data-login-password-submit]').click({ timeout: PHASE_TIMEOUT_MS });
  await page
    .locator('#desktop-navigation')
    .waitFor({ state: 'visible', timeout: PHASE_TIMEOUT_MS });
  await page.waitForFunction(() => location.pathname === '/', null, { timeout: PHASE_TIMEOUT_MS });
  return { endState: 'authenticated-home-visible', method: 'master-password' };
}

async function runSample(options, chromium, index) {
  const sampleRoot = path.join(options.output, 'sample-' + String(index).padStart(3, '0'));
  const sample = {
    index,
    startedAt: new Date().toISOString(),
    root: sampleRoot,
    success: false,
    activePhase: 'prepare',
    phases: [],
    unsupported: [
      {
        name: 'ocr',
        reason:
          'This stage has not prepared an OCR input and UI sampling journey; no inference metric is fabricated',
      },
      {
        name: 'attachment-preview',
        reason: 'Verified fixture has no attachment; no preview metric is fabricated',
      },
      {
        name: 'system-sleep-resume',
        reason: 'This runner measures application lock/reunlock, not Windows system sleep',
      },
    ],
  };
  let browser;
  let owner;
  let launch;
  let ownedCompletion;
  let guard;
  let spawnAt;
  const childEnv = { ...process.env };
  // 注册表联网刷新为可选部署环境变量；合成运行不继承。更新检查仍走正常产品网络。
  delete childEnv.SOLOSOUL_REGISTRY_PUBKEY;
  try {
    const prepareStarted = Date.now();
    const preparation = startChild(
      options.exe,
      ['--native-perf-prepare', sampleRoot, '--fixture', options.fixture],
      childEnv,
    );
    if (!preparation.child.pid)
      throw new Error((await preparation.completion).error ?? 'Preparation process did not spawn');
    ownedCompletion = preparation.completion;
    owner = new OwnedProcess(preparation.child, options.exe, prepareStarted);
    await owner.sample();
    const result = await deadline(preparation.completion, PREPARE_TIMEOUT_MS, 'Native preparation');
    sample.prepare = { ...result, durationMs: Date.now() - prepareStarted };
    if (result.exitCode !== 0)
      throw new Error('Native prepare did not exit 0: ' + JSON.stringify(result));
    const prepared = preparedManifest(
      JSON.parse(await readFile(path.join(sampleRoot, 'native-perf-owned.json'), 'utf8')),
      sampleRoot,
      options.manifest,
      options.fixture,
    );
    await access(path.join(sampleRoot, 'native-perf-ready.json'), constants.R_OK);
    sample.appVersion = typeof prepared.appVersion === 'string' ? prepared.appVersion : null;
    sample.identifier = prepared.identifier;
    sample.runId = prepared.runId;
    const port = await unusedPort();
    spawnAt = performance.now();
    const spawnWall = Date.now();
    launch = startChild(
      options.exe,
      ['--native-perf-root', sampleRoot, '--native-perf-port', String(port)],
      childEnv,
    );
    if (!launch.child.pid)
      throw new Error((await launch.completion).error ?? 'Application process did not spawn');
    ownedCompletion = launch.completion;
    owner = new OwnedProcess(launch.child, options.exe, spawnWall, prepared.webview);
    sample.processId = launch.child.pid;
    const earlyMemory = owner.sample();
    sample.activePhase = 'startup';
    const attachStartAt = performance.now();
    sample.cdp = {
      port,
      attachStartedAfterSpawnMs: attachStartAt - spawnAt,
      attachMs: null,
      connectedAfterSpawnMs: null,
      transport: 'real-WebView2-CDP',
    };
    const connected = await connectMainPage(chromium, port, launch.child, prepared.runId);
    browser = connected.browser;
    const page = connected.page;
    page.setDefaultTimeout(PHASE_TIMEOUT_MS);
    sample.cdp = {
      port,
      attachStartedAfterSpawnMs: attachStartAt - spawnAt,
      attachMs: connected.attachMs,
      connectedAfterSpawnMs: performance.now() - spawnAt,
      transport: 'real-WebView2-CDP',
    };
    await rejectDiagnosticBenchmark(sampleRoot);
    const initialObserver = await getObserver(page, prepared.runId);
    const initialTimeOrigin = await page.evaluate(() => performance.timeOrigin);
    if (!initialObserver.identityMatched || initialObserver.timeOriginMs !== initialTimeOrigin)
      throw new Error('Native observer identity/timeOrigin gate failed before password input');
    guard = createObservationGuard(browser, page, initialTimeOrigin);
    const input = page.locator('[data-login-method-region="password"] input');
    await input.waitFor({ state: 'visible', timeout: STARTUP_TIMEOUT_MS });
    await page.locator('[data-login-password-submit]').waitFor({ state: 'visible' });
    if (!(await page.locator('[data-login-card]').innerText()).includes('Performance Fixture'))
      throw new Error('Login card does not identify the public synthetic account');
    await page
      .locator('[data-login-password-submit]')
      .isEnabled()
      .then((enabled) => {
        if (!enabled) throw new Error('Login submit is not ready');
      });
    await twoFrames(page);
    const startupObserver = await getObserver(page, prepared.runId, guard);
    const startupDuration = performance.now() - spawnAt;
    if ([...guard.reasons].some((reason) => !reason.startsWith('Native observer invalid:')))
      throw new Error('Native main document gate failed before password input');
    const verifiedNativeMemory = await owner.sample();
    sample.nativeBrowserIsolation = {
      verified: verifiedNativeMemory.userDataDirectoryMatches === true,
      apiUserDataFolder: prepared.webview,
      expectedBrowserDataDirectory: browserDataDirectoryCheck(prepared.webview, null)
        .expectedBrowserDataDirectory,
      actualBrowserDataDirectory:
        verifiedNativeMemory.userDataDirectoryMatches === true
          ? verifiedNativeMemory.browserDataChecks[0].actualBrowserDataDirectory
          : null,
      filesystem: verifiedNativeMemory.browserDataFilesystem ?? null,
      checks: verifiedNativeMemory.browserDataChecks ?? [],
      reason:
        verifiedNativeMemory.userDataDirectoryMatches === true
          ? null
          : (verifiedNativeMemory.browserDataFilesystem?.reason ??
            'No single owned WebView2 browser was verified against the exact API UDF/EBWebView directory'),
    };
    if (!sample.nativeBrowserIsolation.verified)
      throw new Error(sample.nativeBrowserIsolation.reason);

    sample.phases.push({
      name: 'startup',
      success: true,
      durationMs: startupDuration,
      definition:
        'spawn to synthetic password login form visible and two animation frames; includes CDP attachment',
      earlyMemory: await earlyMemory,
      memory: await owner.sample(),
      ipc: ipcDelta(
        {
          valid: true,
          total: 0,
          commands: [],
          installedAtMs: startupObserver.installedAtMs,
          timeOriginMs: startupObserver.timeOriginMs,
        },
        startupObserver,
      ),
    });
    sample.webview = await page.evaluate(() => ({
      userAgent: navigator.userAgent,
      language: navigator.language,
      viewport: { width: innerWidth, height: innerHeight, devicePixelRatio },
      timeOriginMs: performance.timeOrigin,
      startupMarks: performance
        .getEntriesByType('mark')
        .filter((entry) => entry.name.startsWith('solosoul:'))
        .map((entry) => ({ name: entry.name, startTimeMs: entry.startTime })),
    }));
    const consumed = JSON.parse(
      await readFile(path.join(sampleRoot, 'native-perf-consumed.json'), 'utf8'),
    );
    if (
      consumed.schemaVersion !== 1 ||
      consumed.scope !== 'windows-native-perf-consumed' ||
      consumed.runId !== prepared.runId ||
      consumed.port !== port ||
      consumed.pid !== launch.child.pid
    )
      throw new Error(
        'Consumed native manifest PID/port/runId does not match the spawned application',
      );
    try {
      await access(path.join(sampleRoot, 'native-perf-ready.json'));
      throw new Error('Ready marker was not consumed');
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    await phase(page, owner, sample, prepared.runId, guard, 'password-unlock', () =>
      passwordUnlock(page),
    );
    await phase(page, owner, sample, prepared.runId, guard, 'workspace', async () => {
      await page
        .locator('#desktop-navigation')
        .getByRole('button', { name: 'Identity', exact: true })
        .click();
      await page
        .locator('[data-shell-content]')
        .getByRole('button', { name: 'Clear', exact: true })
        .click();
      await page.waitForFunction(
        () => location.pathname === '/workspace' && !location.search,
        null,
        { timeout: PHASE_TIMEOUT_MS },
      );
      await page.waitForFunction(
        () => document.querySelectorAll('[data-testid="workspace-object-card"]').length === 50,
        null,
        { timeout: PHASE_TIMEOUT_MS },
      );
      return {
        expectedObjectsInVault: options.manifest.objectCount,
        renderedCards: 50,
        journey: 'Identity category then Clear to all objects',
      };
    });
    await page
      .locator('#desktop-navigation')
      .getByRole('button', { name: 'Home', exact: true })
      .click();
    const searchCard = page
      .locator('[data-shell-content] [data-ui-card][role="button"]')
      .filter({ has: page.getByRole('heading', { name: 'Search', exact: true }) });
    await searchCard.click();
    const searchInput = page.getByPlaceholder('Search objects, profiles...', { exact: true });
    await searchInput.waitFor({ state: 'visible' });
    const expectedCards = Math.min(50, options.manifest.expectedSearchMatches);
    await phase(page, owner, sample, prepared.runId, guard, 'search-needle', async () => {
      await searchInput.fill('needle');
      await page.waitForFunction(
        (expected) =>
          document.querySelectorAll('[data-shell-content] [data-ui-card][role="button"]').length ===
          expected,
        expectedCards,
        { timeout: PHASE_TIMEOUT_MS },
      );
      return {
        query: 'needle',
        renderedCards: expectedCards,
        debounceMs: 300,
        note: 'GUI requests limit 50; backend total/hasMore are not treated as full match count',
      };
    });
    await phase(page, owner, sample, prepared.runId, guard, 'application-lock', async () => {
      await page
        .locator('#desktop-navigation')
        .getByRole('button', { name: 'Lock Vault', exact: true })
        .click();
      await page
        .locator('[data-login-method-region="password"] input')
        .waitFor({ state: 'visible' });
      await page.locator('#desktop-navigation').waitFor({ state: 'detached' });
      return { endState: 'password-login-visible', method: 'real-lock-button' };
    });
    await phase(page, owner, sample, prepared.runId, guard, 'password-reunlock', () =>
      passwordUnlock(page),
    );
    const finalObserver = await getObserver(page, prepared.runId, guard);
    sample.ipcAll = ipcDelta(
      {
        valid: true,
        total: 0,
        commands: [],
        installedAtMs: finalObserver.installedAtMs,
        timeOriginMs: finalObserver.timeOriginMs,
      },
      finalObserver,
    );
    sample.success = sample.phases.every((item) => item.success);
    sample.activePhase = null;
  } catch (error) {
    sample.error = safeError(error);
    sample.failedPhase = sample.activePhase;
    if (
      sample.activePhase === 'startup' &&
      spawnAt !== undefined &&
      !sample.phases.some((item) => item.name === 'startup')
    )
      sample.phases.push({
        name: 'startup',
        success: false,
        durationMs: performance.now() - spawnAt,
        error: sample.error,
        ipc: {
          attempts: null,
          commands: null,
          reason: 'Startup did not reach a verified native login document',
        },
        memory: owner ? await owner.sample() : null,
      });
  } finally {
    if (guard) {
      await guard.check();
      sample.observationIntegrity = {
        valid: guard.reasons.size === 0,
        reasons: [...guard.reasons],
      };
      if (guard.reasons.size) {
        const invalid = { attempts: null, commands: null, reason: [...guard.reasons].join('; ') };
        for (const item of sample.phases) item.ipc = { ...invalid };
        sample.ipcAll = { ...invalid };
      }
      guard.dispose();
    }
    // 先终止已核对的自有进程，再释放CDP客户端；不向未知CDP target发送关闭命令。
    if (owner) sample.cleanup = await owner.cleanup();
    if (browser)
      await browser.close().catch((error) => {
        sample.cdpDetachError = safeError(error);
      });
    if (ownedCompletion)
      sample.ownedProcessExit = await deadline(ownedCompletion, 6000, 'Owned process exit').catch(
        (error) => ({ exitCode: null, reason: safeError(error) }),
      );
    if (launch) sample.applicationExit = sample.ownedProcessExit;
    if (owner) {
      sample.cleanupIntegrity = cleanupOutcome(sample.cleanup, sample.ownedProcessExit);
      if (!sample.cleanupIntegrity.complete) {
        sample.success = false;
        sample.cleanupIncomplete = true;
        sample.ownedChildHandleReleased = releaseUnfinishedChild(
          owner.child,
          sample.ownedProcessExit,
        );
      }
    }
    sample.finishedAt = new Date().toISOString();
    await newJson(
      path.join(options.output, 'sample-' + String(index).padStart(3, '0') + '.json'),
      sample,
    );
  }
  return sample;
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  if (!sorted.length) return null;
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

function quantile(values, fraction) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted.length ? sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)] : null;
}

export function summarize(samples, requestedSamples = samples.length) {
  const names = [
    'startup',
    'password-unlock',
    'workspace',
    'search-needle',
    'application-lock',
    'password-reunlock',
  ];
  return names.map((name) => {
    const measurements = samples.flatMap((sample) =>
      sample.phases
        .filter(
          (phase) => phase.name === name && phase.success && Number.isFinite(phase.durationMs),
        )
        .map((phase) => phase.durationMs),
    );
    return {
      name,
      successfulSamples: measurements.length,
      requestedSamples,
      medianMs: median(measurements),
      p95Ms: quantile(measurements, 0.95),
    };
  });
}

export async function main(args = process.argv.slice(2)) {
  const parsed = parseArgs(args);
  if (parsed.help) {
    process.stdout.write(HELP + '\n');
    return 0;
  }
  const options = await validateInputs(parsed);
  if (process.platform !== 'win32') throw new Error('Native sampling is Windows only');
  // 先校验依赖，失败时不创建output、不运行任何EXE。
  const { chromium } = await import('playwright');
  const exeSha256 = await sha256(options.exe);
  const fixtureMarkerSha256 = await sha256(options.markerPath);
  await mkdir(options.output); // 原子拒绝既有目录，绝不覆盖或删除输入路径。
  const report = {
    schemaVersion: 1,
    task: 'RF-312',
    scope: 'windows-native-application-performance',
    startedAt: new Date().toISOString(),
    platform: process.platform,
    osRelease: os.release(),
    nodeVersion: process.version,
    samplesRequested: options.samples,
    exeSha256,
    binaryPreflight: options.binaryPreflight,
    fixtureMarkerSha256,
    fixture: {
      accountId: options.manifest.accountId,
      objectCount: options.manifest.objectCount,
      buildProfile: options.manifest.buildProfile,
      kdf: options.manifest.kdf,
    },
    environment: {
      network:
        'connected-network-behavior-preserved; normal native update checks are not intercepted',
      ipc: 'real Tauri invoke attempts including plugin commands; no arguments/results captured',
      memory:
        'sum of verified root and descendant Windows working sets; shared pages may be counted in multiple processes',
      observer:
        'counts become null with reason if identity, validity, prefix or overflow checks fail',
      isolation:
        'native-perf executable must validate and consume its prepared synthetic run exactly once',
    },
    samples: [],
  };
  let interrupted = false;
  const stop = () => {
    interrupted = true;
  };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  try {
    for (let i = 1; i <= options.samples && !interrupted; i++) {
      process.stdout.write('RF-312 native sample ' + i + '/' + options.samples + '\n');
      const sample = await runSample(options, chromium, i);
      report.samples.push(sample);
      if (sample.cleanupIncomplete) {
        report.stoppedReason = sample.cleanupIntegrity.reason;
        break;
      }
    }
  } finally {
    process.off('SIGINT', stop);
    process.off('SIGTERM', stop);
    report.interrupted = interrupted;
    report.finishedAt = new Date().toISOString();
    report.summary = summarize(report.samples, options.samples);
    report.success =
      !interrupted &&
      report.samples.length === options.samples &&
      report.samples.every((sample) => sample.success);
    await newJson(path.join(options.output, 'native-perf-results.json'), report);
  }
  process.stdout.write(
    'RF-312 results: ' + path.join(options.output, 'native-perf-results.json') + '\n',
  );
  return report.success ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
    .then((code) => {
      process.exitCode = code;
    })
    .catch((error) => {
      process.stderr.write('Native performance runner: ' + safeError(error) + '\n');
      process.exitCode = 1;
    });
}
