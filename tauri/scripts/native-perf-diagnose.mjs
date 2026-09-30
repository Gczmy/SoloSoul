#!/usr/bin/env node
/** RF-312 单轮现场诊断；不输入密码、不操作 UI，不产出性能指标。 */
import path from 'node:path';
import os from 'node:os';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { mkdir, readFile, readdir, lstat, realpath } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';
import {
  validateInputs,
  normalizeWindowsPath,
  preparedManifest,
  OwnedProcess,
  startChild,
  deadline,
  unusedPort,
  sha256,
  newJson,
  cleanupOutcome,
  releaseUnfinishedChild,
  safeError,
  powerShellError,
} from './native-perf-run.mjs';

const execFileAsync = promisify(execFile);
const helper = fileURLToPath(new URL('./native-perf-process-diagnostics.ps1', import.meta.url));
const HELP =
  'Usage: node scripts/native-perf-diagnose.mjs --exe ABS --fixture ABS --output NEW_ABS [--chromium-log]\nWindows only; one isolated synthetic run, three live observations. No password/UI actions or performance metrics.';

export function parseDiagnosticArgs(args) {
  if (args.length === 1 && args[0] === '--help') return { help: true };
  const allowed = new Set(['--exe', '--fixture', '--output']);
  const values = {};
  let logging = false;
  for (let i = 0; i < args.length; ) {
    const key = args[i];
    if (key === '--chromium-log') {
      if (logging) throw new Error('Duplicate --chromium-log');
      logging = true;
      i++;
      continue;
    }
    if (
      !allowed.has(key) ||
      Object.hasOwn(values, key.slice(2)) ||
      !args[i + 1] ||
      args[i + 1].startsWith('--')
    )
      throw new Error('Expected --exe, --fixture, --output exactly once; optional --chromium-log');
    if (!path.isAbsolute(args[i + 1])) throw new Error(key + ' must be absolute');
    values[key.slice(2)] = path.resolve(args[i + 1]);
    i += 2;
  }
  if (Object.keys(values).length !== 3) throw new Error('Missing required diagnostic options');
  if (path.parse(values.output).root === values.output)
    throw new Error('--output cannot be a filesystem root');
  return logging ? { ...values, logging: true } : values;
}

export async function chromiumLogBinaryPreflight(exe) {
  const marker = Buffer.from('windows-native-perf-chromium-log', 'ascii');
  let overlap = Buffer.alloc(0);
  for await (const chunk of createReadStream(exe, { highWaterMark: 64 * 1024 })) {
    const window = Buffer.concat([overlap, chunk]);
    if (window.includes(marker))
      return {
        method: 'streaming-logging-feature-marker',
        limitation: 'Excludes old native-perf builds; not a signature or trust guarantee',
      };
    overlap = Buffer.from(window.subarray(Math.max(0, window.length - marker.length + 1)));
  }
  throw new Error(
    'Chromium logging requires a rebuilt native-perf EXE with its logging feature marker',
  );
}

export function checkLoggingMarker(value, owned, rootPid, port) {
  const expectedLog = normalizeWindowsPath(
    path.win32.join(owned.root, 'temp', 'chromium-diagnostics.log'),
  );
  if (
    value?.schemaVersion !== 1 ||
    value.scope !== 'windows-native-perf-chromium-log' ||
    value.mode !== 'chromium-log' ||
    value.runId !== owned.runId ||
    value.pid !== rootPid ||
    value.port !== port ||
    value.performanceSample !== false ||
    value.nativeTempUnchanged !== true ||
    normalizeWindowsPath(value.root) !== normalizeWindowsPath(owned.root) ||
    typeof value.logFile !== 'string' ||
    value.logFile.startsWith('\\\\?\\') ||
    normalizeWindowsPath(value.logFile) !== expectedLog
  )
    throw new Error('Chromium log marker does not identify this exact owned diagnostic path/run');
  return value;
}

async function logProof(value) {
  try {
    const file = normalizeWindowsPath(value.logFile);
    for (const [target, isDir] of [
      [path.dirname(file), true],
      [file, false],
    ]) {
      const stat = await lstat(target);
      if (
        stat.isSymbolicLink() ||
        (isDir ? !stat.isDirectory() : !stat.isFile()) ||
        normalizeWindowsPath(await realpath(target)) !== target
      )
        throw new Error('Chromium log path is not the exact owned regular path');
    }
    const stat = await lstat(file);
    return {
      path: file,
      bytes: stat.size,
      sha256: await sha256(file),
      valid: true,
      nonEmpty: stat.size > 0,
      reason: stat.size ? null : 'Owned Chromium log remained empty; logger activity is not proven',
    };
  } catch (error) {
    return { valid: false, nonEmpty: false, reason: safeError(error) };
  }
}
export function checkConsumed(value, owned, rootPid, port) {
  if (
    value?.schemaVersion !== 1 ||
    value.scope !== 'windows-native-perf-consumed' ||
    value.runId !== owned.runId ||
    value.pid !== rootPid ||
    value.port !== port ||
    normalizeWindowsPath(value.root) !== normalizeWindowsPath(owned.root)
  )
    throw new Error('Consumed marker does not identify this exact owned run/PID/port');
  return value;
}

export function selectDiagnosticIdentities(owner, snapshot) {
  if (
    snapshot.userDataDirectoryMatches !== true ||
    snapshot.browserDataChecks?.length !== 1 ||
    !snapshot.browserDataChecks[0].matchesExpected ||
    !Array.isArray(snapshot.processes)
  )
    throw new Error('No exact verified owned browser directory; process diagnostics refused');
  const browserPid = snapshot.browserDataChecks[0].pid;
  const identities = [owner.expected.pid, browserPid].map((pid) => {
    const current = snapshot.processes.filter((p) => p.pid === pid);
    if (current.length !== 1) throw new Error('Expected one fresh root/browser process');
    const row = owner.known.get(pid + ':' + current[0].creationMs);
    if (!row || row.creationMs !== current[0].creationMs || row.parentPid !== current[0].parentPid)
      throw new Error('Fresh root/browser identity does not match owned process journal');
    return {
      pid: row.pid,
      parentPid: row.parentPid,
      creationMs: row.creationMs,
      executablePath: row.executablePath,
    };
  });
  if (
    identities[0].pid === identities[1].pid ||
    normalizeWindowsPath(identities[0].executablePath) !==
      normalizeWindowsPath(owner.expected.exe) ||
    path.win32.basename(identities[1].executablePath).toLowerCase() !== 'msedgewebview2.exe' ||
    identities[1].parentPid !== identities[0].pid
  )
    throw new Error('Owned root/browser relationship failed');
  return identities;
}

export function checkDiagnosticResult(value, identities, browserDirectory) {
  if (
    value?.schemaVersion !== 1 ||
    value.scope !== 'windows-native-perf-process-diagnostics' ||
    value.mode !== 'live' ||
    value.success !== true ||
    value.processes?.length !== 2 ||
    !Array.isArray(value.listeners)
  )
    throw new Error('Process diagnostic helper did not return a complete verified live result');
  for (const expected of identities) {
    const matches = value.processes.filter((p) => p.pid === expected.pid);
    const actual = matches[0];
    if (
      matches.length !== 1 ||
      actual.identityMatched !== true ||
      actual.parentPid !== expected.parentPid ||
      actual.creationMs !== expected.creationMs ||
      normalizeWindowsPath(actual.executablePath) !== normalizeWindowsPath(expected.executablePath)
    )
      throw new Error('Helper returned a changed or unexpected process identity');
  }
  const browser = value.processes.find((p) => p.pid === identities[1].pid);
  if (
    browser.browserFlags?.userDataDirectoryMatched !== true ||
    normalizeWindowsPath(browser.browserFlags.observedDirectory) !==
      normalizeWindowsPath(browserDirectory)
  )
    throw new Error('Live helper did not verify the exact owned browser directory');
  if (
    value.listeners.some(
      (row) =>
        !identities.some((p) => p.pid === row.owningPid) ||
        !Number.isInteger(row.localPort) ||
        row.localPort < 1 ||
        row.localPort > 65535 ||
        typeof row.localAddress !== 'string',
    )
  )
    throw new Error('Helper returned an unowned or invalid listener');
  return value;
}

export async function cdpProbe(value, identities, expectedPort, request = fetch) {
  if (value.listenersReason)
    return {
      status: 'listener-query-unavailable',
      attempted: false,
      expectedPort,
      reason: safeError(value.listenersReason),
    };
  const browser = value.processes.find((p) => p.pid === identities[1].pid);
  const listeners = value.listeners.filter(
    (row) => row.owningPid === browser.pid && row.localPort === expectedPort,
  );
  if (!listeners.length) return { status: 'no-owned-listener', attempted: false, expectedPort };
  if (
    browser.browserFlags.port !== expectedPort ||
    browser.browserFlags.address !== '127.0.0.1' ||
    !listeners.some((row) => ['127.0.0.1', '0.0.0.0', '::', '::1'].includes(row.localAddress))
  )
    return { status: 'listener-or-flags-mismatch', attempted: false, expectedPort };
  try {
    const response = await request('http://127.0.0.1:' + expectedPort + '/json/version', {
      signal: AbortSignal.timeout(3000),
      redirect: 'error',
    });
    if (!response.ok)
      return { status: 'http-error', attempted: true, httpStatus: response.status, expectedPort };
    // 只保存版本信息，不保存目标列表、页面内容或完整 websocket URL。
    const body = await response.json();
    const versions = Object.fromEntries(
      ['Browser', 'Protocol-Version', 'V8-Version', 'WebKit-Version'].map((key) => [
        key,
        typeof body[key] === 'string' ? body[key].slice(0, 180) : null,
      ]),
    );
    return {
      status: 'http-version-available',
      attempted: true,
      expectedPort,
      versions,
      limitation: 'Version endpoint only; no CDP target/observer/UI workflow validated',
    };
  } catch (error) {
    return { status: 'request-failed', attempted: true, expectedPort, reason: safeError(error) };
  }
}

async function fixtureFiles(root) {
  const proof = [];
  async function visit(dir) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const target = path.join(dir, entry.name);
      const stat = await lstat(target);
      if (
        stat.isSymbolicLink() ||
        normalizeWindowsPath(await realpath(target)) !== normalizeWindowsPath(target)
      )
        throw new Error('Source fixture traverses a link');
      if (stat.isDirectory()) await visit(target);
      else if (stat.isFile()) {
        if (proof.length >= 128) throw new Error('Unexpected source fixture file count');
        proof.push({
          path: path.relative(root, target).split(path.sep).join('/'),
          bytes: stat.size,
          sha256: await sha256(target),
        });
      } else throw new Error('Unexpected source fixture entry');
    }
  }
  await visit(root);
  return proof.sort((a, b) => a.path.localeCompare(b.path));
}

export function diagnosticHelperFailure(error) {
  try {
    const value = JSON.parse(
      String(error.stdout ?? '')
        .replace(/^\uFEFF/, '')
        .trim(),
    );
    if (
      value?.schemaVersion === 1 &&
      value.scope === 'windows-native-perf-process-diagnostics' &&
      value.mode === 'live' &&
      value.success === false &&
      typeof value.reason === 'string'
    ) {
      const stage = /^[a-z-]{1,50}$/.test(value.diagnosticStage ?? '')
        ? value.diagnosticStage
        : 'unspecified';
      return 'Owned process diagnostics refused at ' + stage + ': ' + safeError(value.reason);
    }
  } catch {}
  return error.stdout || error.stderr
    ? powerShellError(error, 'diagnostics', 30000)
    : safeError(error);
}
export function helperEnvironment(identities, owned, helperTemp, logging, inherited = process.env) {
  const env = { ...inherited };
  for (const key of Object.keys(env)) {
    const upper = key.toUpperCase();
    if (upper.startsWith('SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_') || ['TEMP', 'TMP'].includes(upper))
      delete env[key];
  }
  Object.assign(env, {
    TEMP: helperTemp,
    TMP: helperTemp,
    SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_MODE: 'live',
    SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_OWNED: JSON.stringify(identities),
    SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_BROWSER_DATA_DIRECTORY: path.join(owned.webview, 'EBWebView'),
  });
  if (logging) env.SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_LOG_FILE = logging.logFile;
  return env;
}
async function processDiagnostics(identities, owned, logging) {
  try {
    // .NET Framework 的 Add-Type 编译器不接受 Rust canonical 的 \\?\ TEMP。
    // 两种表示必须指向同一个已验证目录；只为本次 helper 子进程转换表示。
    const helperTemp = normalizeWindowsPath(path.join(owned.root, 'temp'));
    const tempStat = await lstat(helperTemp);
    if (
      !tempStat.isDirectory() ||
      tempStat.isSymbolicLink() ||
      normalizeWindowsPath(await realpath(helperTemp)) !== helperTemp
    )
      throw new Error('Diagnostic compiler TEMP is not the exact owned regular directory');
    const { stdout } = await execFileAsync(
      'powershell.exe',
      ['-NoLogo', '-NoProfile', '-NonInteractive', '-File', helper],
      {
        windowsHide: true,
        timeout: 30000,
        maxBuffer: 1024 * 1024,
        env: helperEnvironment(identities, owned, helperTemp, logging),
      },
    );
    return checkDiagnosticResult(
      JSON.parse(stdout.replace(/^\uFEFF/, '').trim()),
      identities,
      path.join(owned.webview, 'EBWebView'),
    );
  } catch (error) {
    throw new Error(diagnosticHelperFailure(error));
  }
}

export async function main(args = process.argv.slice(2)) {
  const parsed = parseDiagnosticArgs(args);
  if (parsed.help) {
    process.stdout.write(HELP + '\n');
    return 0;
  }
  if (process.platform !== 'win32') throw new Error('Native diagnostics are Windows only');
  const options = await validateInputs(parsed);
  const loggingBinaryPreflight = options.logging
    ? await chromiumLogBinaryPreflight(options.exe)
    : null;
  const sourceBefore = await fixtureFiles(options.fixture);
  const hashes = {
    exe: await sha256(options.exe),
    helper: await sha256(helper),
    diagnostic: await sha256(fileURLToPath(import.meta.url)),
    runner: await sha256(fileURLToPath(new URL('./native-perf-run.mjs', import.meta.url))),
  };
  await mkdir(options.output);
  const report = {
    schemaVersion: 1,
    task: 'RF-312',
    scope: 'windows-native-cdp-diagnostics',
    startedAt: new Date().toISOString(),
    platform: process.platform,
    osRelease: os.release(),
    nodeVersion: process.version,
    hashes,
    binaryPreflight: options.binaryPreflight,
    loggingRequested: options.logging === true,
    loggingBinaryPreflight,
    fixture: options.manifest,
    observations: [],
    captureComplete: false,
    performanceMetrics: null,
    limitation: 'One diagnostic run; no password input, UI action, or performance sample',
    sourceBefore,
  };
  const root = path.join(options.output, 'sample-001');
  let owner, launch, completion;
  let interrupted = false;
  const stop = () => {
    interrupted = true;
  };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  const childEnv = { ...process.env };
  delete childEnv.SOLOSOUL_REGISTRY_PUBKEY;
  try {
    const preparedAt = Date.now();
    const prep = startChild(
      options.exe,
      ['--native-perf-prepare', root, '--fixture', options.fixture],
      childEnv,
    );
    completion = prep.completion;
    if (!prep.child.pid) throw new Error((await completion).error ?? 'Preparation did not spawn');
    owner = new OwnedProcess(prep.child, options.exe, preparedAt);
    await owner.sample();
    report.prepare = await deadline(completion, 120000, 'Native preparation');
    if (report.prepare.exitCode !== 0) throw new Error('Native preparation failed');
    owner = null;
    completion = null;
    const owned = preparedManifest(
      JSON.parse(await readFile(path.join(root, 'native-perf-owned.json'), 'utf8')),
      root,
      options.manifest,
      options.fixture,
    );
    report.owned = owned;
    const port = await unusedPort();
    report.port = port;
    if (interrupted) throw new Error('Interrupted before GUI launch');
    const launchedAt = Date.now();
    launch = startChild(
      options.exe,
      [
        '--native-perf-root',
        root,
        '--native-perf-port',
        String(port),
        ...(options.logging ? ['--native-perf-diagnostics', 'chromium-log'] : []),
      ],
      childEnv,
    );
    completion = launch.completion;
    if (!launch.child.pid) throw new Error((await completion).error ?? 'Application did not spawn');
    owner = new OwnedProcess(launch.child, options.exe, launchedAt, owned.webview);
    report.processId = launch.child.pid;
    for (const offsetMs of [5000, 15000, 30000]) {
      if (interrupted) throw new Error('Interrupted during diagnostic capture');
      await delay(Math.max(0, launchedAt + offsetMs - Date.now()));
      if (interrupted) throw new Error('Interrupted during diagnostic capture');
      const observation = { targetOffsetMs: offsetMs, observedAt: new Date().toISOString() };
      try {
        observation.consumed = checkConsumed(
          JSON.parse(await readFile(path.join(root, 'native-perf-consumed.json'), 'utf8')),
          owned,
          launch.child.pid,
          port,
        );
        if (options.logging) {
          report.logging = checkLoggingMarker(
            JSON.parse(await readFile(path.join(root, 'native-perf-chromium-log.json'), 'utf8')),
            owned,
            launch.child.pid,
            port,
          );
          observation.logging = report.logging;
        }
        observation.ownedSnapshot = await owner.sample();
        const identities = selectDiagnosticIdentities(owner, observation.ownedSnapshot);
        observation.processDiagnostics = await processDiagnostics(
          identities,
          owned,
          report.logging,
        );
        if (
          options.logging &&
          observation.processDiagnostics.processes.find((p) => p.pid === identities[1].pid)
            .browserFlags.logging?.flagsMatched !== true
        )
          throw new Error(
            'Owned browser logging arguments did not match the native diagnostic contract',
          );
        // 监听之后再次核验原进程和UDF，再作无密码的loopback版本请求。
        const beforeProbe = selectDiagnosticIdentities(owner, await owner.sample());
        if (JSON.stringify(beforeProbe) !== JSON.stringify(identities))
          throw new Error('Identity changed before loopback probe');
        observation.cdp = await cdpProbe(observation.processDiagnostics, identities, port);
        observation.success = true;
      } catch (error) {
        observation.success = false;
        observation.reason = safeError(error);
      }
      observation.finishedAt = new Date().toISOString();
      report.observations.push(observation);
      await newJson(
        path.join(options.output, 'observation-' + report.observations.length + '.json'),
        observation,
      );
      process.stdout.write(
        'RF-312 diagnostic observation ' +
          report.observations.length +
          '/3: ' +
          (observation.success ? observation.cdp.status : 'failed') +
          '\n',
      );
    }
    report.captureComplete =
      report.observations.length === 3 && report.observations.every((o) => o.success);
  } catch (error) {
    report.reason = safeError(error);
  } finally {
    if (owner) report.cleanup = await owner.cleanup();
    if (completion)
      report.ownedProcessExit = await deadline(completion, 6000, 'Owned process exit').catch(
        (error) => ({ exitCode: null, reason: safeError(error) }),
      );
    if (owner) {
      report.cleanupIntegrity = cleanupOutcome(report.cleanup, report.ownedProcessExit);
      if (!report.cleanupIntegrity.complete)
        report.ownedChildHandleReleased = releaseUnfinishedChild(
          owner.child,
          report.ownedProcessExit,
        );
    }
    try {
      report.sourceAfter = await fixtureFiles(options.fixture);
      report.sourceUnchanged = JSON.stringify(sourceBefore) === JSON.stringify(report.sourceAfter);
    } catch (error) {
      report.sourceUnchanged = false;
      report.sourceProofError = safeError(error);
    }
    if (options.logging)
      report.chromiumLog = report.logging
        ? await logProof(report.logging)
        : { valid: false, nonEmpty: false, reason: 'No verified native logging marker' };
    process.off('SIGINT', stop);
    process.off('SIGTERM', stop);
    report.interrupted = interrupted;
    report.finishedAt = new Date().toISOString();
    report.success =
      !interrupted &&
      report.captureComplete &&
      report.sourceUnchanged &&
      report.cleanupIntegrity?.complete === true &&
      (!options.logging ||
        (report.chromiumLog?.valid === true && report.chromiumLog.nonEmpty === true));
    await newJson(path.join(options.output, 'native-perf-diagnostics.json'), report);
  }
  process.stdout.write(
    'RF-312 diagnostic evidence: ' +
      path.join(options.output, 'native-perf-diagnostics.json') +
      '\n',
  );
  return report.success ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
    .then((code) => {
      process.exitCode = code;
    })
    .catch((error) => {
      process.stderr.write('Native diagnostics: ' + safeError(error) + '\n');
      process.exitCode = 1;
    });
}
