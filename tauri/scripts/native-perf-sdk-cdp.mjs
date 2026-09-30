#!/usr/bin/env node
/** RF-312：WebView2 SDK 内只读 CDP 诊断；不通过 TCP 连接 CDP、不输入密码、不采样性能。 */
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { createReadStream } from 'node:fs';
import { createHash } from 'node:crypto';
import { mkdir, lstat, realpath, readdir, open, readFile } from 'node:fs/promises';
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
  validateObserver,
} from './native-perf-run.mjs';
import {
  checkConsumed,
  selectDiagnosticIdentities,
  processDiagnostics,
} from './native-perf-diagnose.mjs';

export const SDK_REQUESTED_FILE = 'native-perf-sdk-cdp-requested.json';
export const SDK_PROOF_FILE = 'native-perf-sdk-cdp.json';
const FEATURE = 'windows-native-sdk-cdp-requested';
const EXPECTED_ORIGIN = 'http://tauri.localhost';
const MAX_JSON_BYTES = 1024 * 1024;
const WAIT_MS = 45000;
const BS = String.fromCharCode(92);
const EXTENDED = BS + BS + '?' + BS;
const CALLS = ['Page.getFrameTree', 'Runtime.evaluate'];
const HELP =
  'Usage: node scripts/native-perf-sdk-cdp.mjs --exe ABS --fixture ABS --output NEW_ABS\nWindows only; one isolated synthetic run. Read-only SDK protocol calls; no TCP/HTTP CDP endpoint connection/attach, password/UI actions or performance metrics.';

export function parseSdkArgs(args) {
  if (args.length === 1 && args[0] === '--help') return { help: true };
  const values = {};
  const allowed = new Set(['--exe', '--fixture', '--output']);
  for (let i = 0; i < args.length; i += 2) {
    const key = args[i];
    if (
      !allowed.has(key) ||
      Object.hasOwn(values, key.slice(2)) ||
      !args[i + 1] ||
      args[i + 1].startsWith('--')
    )
      throw new Error(
        'Expected --exe, --fixture, --output exactly once; SDK mode has no overrides',
      );
    if (!path.isAbsolute(args[i + 1])) throw new Error(key + ' must be absolute');
    values[key.slice(2)] = path.resolve(args[i + 1]);
  }
  if (Object.keys(values).length !== 3) throw new Error('Missing required SDK diagnostic paths');
  if (path.parse(values.output).root === values.output)
    throw new Error('--output cannot be a filesystem root');
  return values;
}

export async function sdkBinaryPreflight(exe, chunkSize = 64 * 1024) {
  if (!Number.isSafeInteger(chunkSize) || chunkSize < 1 || chunkSize > MAX_JSON_BYTES)
    throw new Error('Invalid SDK feature preflight chunk size');
  const marker = Buffer.from(FEATURE, 'ascii');
  let overlap = Buffer.alloc(0);
  for await (const chunk of createReadStream(exe, { highWaterMark: chunkSize })) {
    const window = Buffer.concat([overlap, chunk]);
    if (window.includes(marker))
      return {
        method: 'streaming-sdk-cdp-feature-marker',
        limitation: 'Excludes old builds; not a signature or trust guarantee',
      };
    overlap = Buffer.from(window.subarray(Math.max(0, window.length - marker.length + 1)));
  }
  throw new Error(
    'SDK CDP requires a rebuilt native-perf EXE with windows-native-sdk-cdp-requested; no EXE was executed',
  );
}

export function sdkLaunchArgs(root, port) {
  if (
    typeof root !== 'string' ||
    !path.win32.isAbsolute(root) ||
    !Number.isInteger(port) ||
    port < 1024 ||
    port > 65535
  )
    throw new Error('SDK launch requires an explicit prepared root and port');
  return [
    '--native-perf-root',
    root,
    '--native-perf-port',
    String(port),
    '--native-perf-diagnostics',
    'sdk-cdp',
  ];
}
function checkIdentity(value, owned, rootPid, port, scope) {
  if (
    typeof owned?.root !== 'string' ||
    !owned.root.startsWith(EXTENDED) ||
    !/^[A-Za-z]:\\/.test(owned.root.slice(4)) ||
    path.win32.normalize(owned.root) !== owned.root ||
    !/^[a-f0-9]{32}$/.test(owned.runId ?? '') ||
    !Number.isInteger(rootPid) ||
    rootPid < 1 ||
    !Number.isInteger(port) ||
    port < 1024 ||
    port > 65535 ||
    value?.schemaVersion !== 1 ||
    value.scope !== scope ||
    value.mode !== 'sdk-cdp' ||
    value.performanceSample !== false ||
    value.root !== owned.root ||
    value.runId !== owned.runId ||
    value.pid !== rootPid ||
    value.port !== port ||
    value.windowLabel !== 'main'
  )
    throw new Error(
      'SDK marker does not identify this exact canonical owned run/root/PID/port/main window',
    );
}
export function checkSdkRequested(value, owned, rootPid, port) {
  checkIdentity(value, owned, rootPid, port, 'windows-native-sdk-cdp-requested');
  return {
    schemaVersion: 1,
    scope: value.scope,
    mode: 'sdk-cdp',
    performanceSample: false,
    root: value.root,
    runId: value.runId,
    pid: value.pid,
    port: value.port,
    windowLabel: 'main',
  };
}
function validatedSource(value) {
  if (
    typeof value !== 'string' ||
    value.length > 2048 ||
    !/^http:\/\/tauri\.localhost(?:\/|$)/.test(value) ||
    /[\x00-\x20\\]/.test(value)
  )
    throw new Error('SDK source is not the exact expected application origin');
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error('SDK source is not a valid application URL');
  }
  if (
    url.origin !== EXPECTED_ORIGIN ||
    url.protocol !== 'http:' ||
    url.hostname !== 'tauri.localhost' ||
    url.username ||
    url.password ||
    url.port
  )
    throw new Error('SDK source is not the exact expected application origin');
  return value;
}
const token = (value) => typeof value === 'string' && /^[a-zA-Z0-9_.:-]{1,180}$/.test(value);
export function checkSdkProof(value, owned, identities, port, arrivedAfterSpawnMs) {
  if (
    !Array.isArray(identities) ||
    identities.length !== 2 ||
    identities[0].pid === identities[1].pid ||
    identities[1].parentPid !== identities[0].pid ||
    path.win32.basename(identities[1].executablePath ?? '').toLowerCase() !== 'msedgewebview2.exe'
  )
    throw new Error('SDK validation requires the already verified root/browser identities');
  checkIdentity(value, owned, identities[0].pid, port, 'windows-native-sdk-cdp-diagnostic');
  if (
    !Number.isFinite(arrivedAfterSpawnMs) ||
    arrivedAfterSpawnMs < 0 ||
    arrivedAfterSpawnMs > WAIT_MS
  )
    throw new Error('SDK proof arrived outside the bounded 45-second window');
  if (value.success !== true || value.stage !== 'complete' || value.reason !== null)
    throw new Error(
      'Native SDK protocol did not complete successfully; owned failure evidence is retained',
    );
  if (
    value.expectedOrigin !== EXPECTED_ORIGIN ||
    value.browserPid !== identities[1].pid ||
    !Number.isFinite(value.elapsedMs) ||
    value.elapsedMs < 0 ||
    value.elapsedMs > 20000 ||
    !Array.isArray(value.calls) ||
    JSON.stringify(value.calls) !== JSON.stringify(CALLS)
  )
    throw new Error('SDK protocol identity, calls, origin or elapsed duration is invalid');
  const binding = value.binding;
  const doc = value.document;
  if (
    !binding ||
    !token(binding.mainFrameId) ||
    !token(binding.loaderId) ||
    !Number.isFinite(binding.timeOriginMs) ||
    binding.timeOriginMs <= 0 ||
    binding.navigationEvents !== 0 ||
    binding.frameCreatedEvents !== 0 ||
    !doc ||
    doc.origin !== EXPECTED_ORIGIN ||
    doc.readyState !== 'complete' ||
    doc.mainFrame !== true ||
    doc.frameCount !== 0 ||
    doc.uiRootPresent !== true
  )
    throw new Error('SDK proof lost its single main document/frame/loader binding');
  const source = validatedSource(binding.source);
  const observer = validateObserver(value.observer, owned.runId);
  if (
    !observer.valid ||
    !Array.isArray(value.observer?.invalidReasons) ||
    value.observer.invalidReasons.length !== 0 ||
    observer.commands.length > 16384 ||
    observer.timeOriginMs !== binding.timeOriginMs ||
    observer.installedAtMs < 0 ||
    observer.commands.some((item) => item.atMs < observer.installedAtMs)
  )
    throw new Error('SDK observer does not match this complete single document/run/timeOrigin');
  return {
    ...checkSdkRequested(
      { ...value, scope: 'windows-native-sdk-cdp-requested' },
      owned,
      identities[0].pid,
      port,
    ),
    scope: 'windows-native-sdk-cdp-diagnostic',
    expectedOrigin: EXPECTED_ORIGIN,
    browserPid: identities[1].pid,
    success: true,
    stage: 'complete',
    reason: null,
    calls: [...CALLS],
    elapsedMs: value.elapsedMs,
    binding: {
      source,
      mainFrameId: binding.mainFrameId,
      loaderId: binding.loaderId,
      timeOriginMs: binding.timeOriginMs,
      navigationEvents: 0,
      frameCreatedEvents: 0,
    },
    document: {
      origin: EXPECTED_ORIGIN,
      readyState: 'complete',
      mainFrame: true,
      frameCount: 0,
      uiRootPresent: true,
    },
    observer: {
      schemaVersion: 1,
      scope: 'windows-native-tauri-invoke-observer',
      runId: owned.runId,
      ...observer,
    },
  };
}
export function sameSdkIdentities(before, after) {
  if (!Array.isArray(before) || !Array.isArray(after) || before.length !== 2 || after.length !== 2)
    throw new Error('SDK process identity list is incomplete');
  for (let i = 0; i < 2; i++) {
    const a = before[i],
      b = after[i];
    if (
      a.pid !== b.pid ||
      a.parentPid !== b.parentPid ||
      a.creationMs !== b.creationMs ||
      normalizeWindowsPath(a.executablePath) !== normalizeWindowsPath(b.executablePath)
    )
      throw new Error('SDK root/browser identity changed while reading the proof');
  }
  return true;
}
export async function readSdkJson(root, filename) {
  if (![SDK_REQUESTED_FILE, SDK_PROOF_FILE, 'native-perf-consumed.json'].includes(filename))
    throw new Error('Only exact owned SDK diagnostic markers may be read');
  const target = path.join(root, filename);
  for (const [item, dir] of [
    [root, true],
    [target, false],
  ]) {
    const stat = await lstat(item);
    if (
      stat.isSymbolicLink() ||
      (dir ? !stat.isDirectory() : !stat.isFile()) ||
      normalizeWindowsPath(await realpath(item)) !== normalizeWindowsPath(item)
    )
      throw new Error('SDK marker path is not an exact regular owned path');
  }
  const original = await lstat(target);
  if (!Number.isSafeInteger(original.size) || original.size < 2 || original.size > MAX_JSON_BYTES)
    throw new Error('SDK marker exceeds its bounded 1MiB JSON size');
  const handle = await open(target, 'r');
  try {
    const opened = await handle.stat();
    if (
      opened.ino !== original.ino ||
      opened.dev !== original.dev ||
      opened.size !== original.size ||
      !opened.isFile()
    )
      throw new Error('SDK marker changed before reading');
    const buffer = Buffer.alloc(original.size + 1);
    let bytes = 0;
    while (bytes < buffer.length) {
      const result = await handle.read(buffer, bytes, buffer.length - bytes, bytes);
      if (!result.bytesRead) break;
      bytes += result.bytesRead;
    }
    const after = await lstat(target);
    if (
      bytes !== original.size ||
      after.ino !== original.ino ||
      after.dev !== original.dev ||
      after.size !== original.size ||
      after.mtimeMs !== original.mtimeMs ||
      after.isSymbolicLink() ||
      normalizeWindowsPath(await realpath(target)) !== normalizeWindowsPath(target)
    )
      throw new Error('SDK marker changed during reading');
    let value;
    try {
      value = JSON.parse(buffer.subarray(0, bytes).toString('utf8'));
    } catch {
      throw new Error('SDK marker is not valid JSON; raw content is not captured');
    }
    return {
      value,
      file: filename,
      bytes,
      sha256: createHash('sha256').update(buffer.subarray(0, bytes)).digest('hex'),
    };
  } finally {
    await handle.close();
  }
}
export async function waitSdkProof(
  root,
  launchedAt,
  { now = Date.now, sleep = delay, shouldStop = () => false } = {},
) {
  while (true) {
    if (shouldStop())
      throw new Error('SDK diagnostic interrupted or originally spawned process exited');
    const elapsed = now() - launchedAt;
    if (!Number.isFinite(elapsed) || elapsed < 0 || elapsed > WAIT_MS)
      throw new Error('SDK proof did not arrive within the bounded 45-second window');
    try {
      const stat = await lstat(path.join(root, SDK_PROOF_FILE));
      if (stat.isSymbolicLink() || !stat.isFile() || stat.size < 2 || stat.size > MAX_JSON_BYTES)
        throw new Error('SDK proof marker is not a bounded regular file');
      const arrival = now() - launchedAt;
      if (!Number.isFinite(arrival) || arrival < 0 || arrival > WAIT_MS)
        throw new Error('SDK proof did not arrive within the bounded 45-second window');
      return arrival;
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    await sleep(Math.min(200, WAIT_MS - elapsed + 1));
  }
}
async function fixtureInventory(root) {
  const proof = [];
  let dirs = 0,
    bytes = 0;
  async function visit(dir, prefix, depth) {
    if (depth > 32 || ++dirs > 128)
      throw new Error('Unexpected synthetic fixture directory count/depth');
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const target = path.join(dir, entry.name);
      const stat = await lstat(target);
      if (
        stat.isSymbolicLink() ||
        normalizeWindowsPath(await realpath(target)) !== normalizeWindowsPath(target)
      )
        throw new Error('Source fixture traverses a link');
      const relative = prefix ? prefix + '/' + entry.name : entry.name;
      if (stat.isDirectory()) await visit(target, relative, depth + 1);
      else if (stat.isFile()) {
        if (proof.length >= 128 || (bytes += stat.size) > 1024 * 1024 * 1024)
          throw new Error('Unexpected synthetic fixture inventory');
        proof.push({ path: relative, bytes: stat.size, sha256: await sha256(target) });
      } else throw new Error('Unexpected synthetic fixture entry');
    }
  }
  await visit(root, '', 0);
  return proof.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}
function failureSummary(value) {
  const slug = (text) => (typeof text === 'string' && /^[a-z0-9-]{1,80}$/.test(text) ? text : null);
  return {
    success: false,
    stage: slug(value?.stage),
    reason: slug(value?.reason),
    calls: Array.isArray(value?.calls)
      ? value.calls.filter((item) => CALLS.includes(item)).slice(0, 2)
      : [],
    elapsedMs: Number.isFinite(value?.elapsedMs) ? value.elapsedMs : null,
    limitation: 'Rejected native proof; raw fields remain only in its owned evidence file',
  };
}

export async function main(args = process.argv.slice(2)) {
  const parsed = parseSdkArgs(args);
  if (parsed.help) {
    process.stdout.write(HELP + '\n');
    return 0;
  }
  if (process.platform !== 'win32') throw new Error('Native SDK diagnostics are Windows only');
  const options = await validateInputs(parsed);
  const sdkPreflight = await sdkBinaryPreflight(options.exe);
  const sourceBefore = await fixtureInventory(options.fixture);
  const hashes = {
    exe: await sha256(options.exe),
    sdkScript: await sha256(fileURLToPath(import.meta.url)),
    runner: await sha256(fileURLToPath(new URL('./native-perf-run.mjs', import.meta.url))),
    diagnostic: await sha256(fileURLToPath(new URL('./native-perf-diagnose.mjs', import.meta.url))),
    processHelper: await sha256(
      fileURLToPath(new URL('./native-perf-process-diagnostics.ps1', import.meta.url)),
    ),
  };
  await mkdir(options.output);
  const report = {
    schemaVersion: 1,
    task: 'RF-312',
    scope: 'windows-native-sdk-cdp-diagnostics',
    startedAt: new Date().toISOString(),
    platform: process.platform,
    osRelease: os.release(),
    nodeVersion: process.version,
    hashes,
    binaryPreflight: options.binaryPreflight,
    sdkPreflight,
    fixture: options.manifest,
    sourceBefore,
    performanceMetrics: null,
    sdkProtocolCalls: null,
    success: false,
    limitation:
      'One read-only SDK protocol diagnostic; no TCP/HTTP CDP endpoint connection/attach, password/UI actions, benchmark or full-session IPC metrics',
  };
  const root = path.join(options.output, 'sample-001');
  let owner, launch, completion, proofValue;
  let interrupted = false;
  const stop = () => {
    interrupted = true;
  };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  const childEnv = { ...process.env };
  delete childEnv.SOLOSOUL_REGISTRY_PUBKEY;
  try {
    const prepAt = Date.now();
    const prep = startChild(
      options.exe,
      ['--native-perf-prepare', root, '--fixture', options.fixture],
      childEnv,
    );
    completion = prep.completion;
    if (!prep.child.pid)
      throw new Error((await completion).error ?? 'Native preparation did not spawn');
    owner = new OwnedProcess(prep.child, options.exe, prepAt);
    await owner.sample();
    report.prepare = await deadline(completion, 120000, 'Native SDK preparation');
    if (report.prepare.exitCode !== 0) throw new Error('Native SDK preparation failed');
    owner = null;
    completion = null;
    const owned = preparedManifest(
      JSON.parse(await readFile(path.join(root, 'native-perf-owned.json'), 'utf8')),
      root,
      options.manifest,
      options.fixture,
    );
    report.owned = owned;
    for (const filename of [SDK_REQUESTED_FILE, SDK_PROOF_FILE]) {
      try {
        await lstat(path.join(root, filename));
        throw new Error('SDK markers must not exist before this native launch');
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
    }
    const port = await unusedPort();
    report.port = port;
    if (interrupted) throw new Error('Interrupted before SDK native launch');
    const launchedAt = Date.now();
    launch = startChild(options.exe, sdkLaunchArgs(root, port), childEnv);
    completion = launch.completion;
    if (!launch.child.pid)
      throw new Error((await completion).error ?? 'SDK application did not spawn');
    report.processId = launch.child.pid;
    owner = new OwnedProcess(launch.child, options.exe, launchedAt, owned.webview);
    await owner.sample();
    report.proofArrivedAfterSpawnMs = await waitSdkProof(root, launchedAt, {
      shouldStop: () => interrupted || launch.child.exitCode !== null || !!launch.child.signalCode,
    });
    report.beforeOwnedSnapshot = await owner.sample();
    const identities = selectDiagnosticIdentities(owner, report.beforeOwnedSnapshot);
    report.beforeProcessDiagnostics = await processDiagnostics(identities, owned);
    report.consumed = checkConsumed(
      (await readSdkJson(root, 'native-perf-consumed.json')).value,
      owned,
      launch.child.pid,
      port,
    );
    report.requested = checkSdkRequested(
      (await readSdkJson(root, SDK_REQUESTED_FILE)).value,
      owned,
      launch.child.pid,
      port,
    );
    const proof = await readSdkJson(root, SDK_PROOF_FILE);
    proofValue = proof.value;
    const validated = checkSdkProof(
      proofValue,
      owned,
      identities,
      port,
      report.proofArrivedAfterSpawnMs,
    );
    report.proofFile = {
      file: proof.file,
      bytes: proof.bytes,
      sha256: proof.sha256,
    };
    report.afterOwnedSnapshot = await owner.sample();
    const after = selectDiagnosticIdentities(owner, report.afterOwnedSnapshot);
    sameSdkIdentities(identities, after);
    report.afterProcessDiagnostics = await processDiagnostics(after, owned);
    if (interrupted) throw new Error('Interrupted during SDK proof verification');
    report.sdk = validated;
    report.sdkProtocolCalls = validated.calls;
    report.captureComplete = true;
  } catch (error) {
    report.reason = safeError(error);
    if (proofValue !== undefined) report.nativeSdkFailure = failureSummary(proofValue);
  } finally {
    if (owner) report.cleanup = await owner.cleanup();
    if (completion)
      report.ownedProcessExit = await deadline(completion, 6000, 'Owned SDK process exit').catch(
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
      report.sourceAfter = await fixtureInventory(options.fixture);
      report.sourceUnchanged = JSON.stringify(sourceBefore) === JSON.stringify(report.sourceAfter);
    } catch (error) {
      report.sourceUnchanged = false;
      report.sourceProofError = safeError(error);
    }
    process.off('SIGINT', stop);
    process.off('SIGTERM', stop);
    report.interrupted = interrupted;
    report.finishedAt = new Date().toISOString();
    report.success =
      !interrupted &&
      report.captureComplete === true &&
      report.sourceUnchanged === true &&
      report.cleanupIntegrity?.complete === true;
    await newJson(path.join(options.output, 'native-perf-sdk-cdp-diagnostics.json'), report);
  }
  process.stdout.write(
    'RF-312 SDK diagnostic evidence: ' +
      path.join(options.output, 'native-perf-sdk-cdp-diagnostics.json') +
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
      process.stderr.write('Native SDK diagnostics: ' + safeError(error) + '\n');
      process.exitCode = 1;
    });
}
