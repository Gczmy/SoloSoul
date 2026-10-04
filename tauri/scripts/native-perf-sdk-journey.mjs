#!/usr/bin/env node
/** RF-312：固定公开合成Vault、真实WebView2 SDK Input行程，独立于只读诊断。 */
import { mkdir, readFile, readdir, lstat, realpath, rename } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { setTimeout as pause } from 'node:timers/promises';
import {
  parseArgs,
  validateInputs,
  sha256,
  newJson,
  preparedManifest,
  unusedPort,
  startChild,
  OwnedProcess,
  deadline,
  cleanupOutcome,
  releaseUnfinishedChild,
  normalizeWindowsPath,
  validateObserver,
  ipcDelta,
  summarize,
  safeError,
} from './native-perf-run.mjs';
import {
  checkConsumed,
  selectDiagnosticIdentities,
  processDiagnostics,
} from './native-perf-diagnose.mjs';
import { sameSdkIdentities } from './native-perf-sdk-cdp.mjs';

export const PHASES = Object.freeze([
  'startup',
  'password-unlock',
  'workspace',
  'search-needle',
  'application-lock',
  'password-reunlock',
]);
const METHODS = new Set([
  'Page.getFrameTree',
  'Runtime.evaluate',
  'Input.dispatchMouseEvent',
  'Input.insertText',
]);
const FILES = new Set([
  'native-perf-owned.json',
  'native-perf-consumed.json',
  'native-perf-sdk-journey-bound.json',
  'native-perf-sdk-journey.json',
]);
const exactKeys = (value, keys) =>
  value &&
  typeof value === 'object' &&
  !Array.isArray(value) &&
  Object.keys(value).length === keys.length &&
  keys.every((key) => Object.hasOwn(value, key));
const validBinding = (value) =>
  exactKeys(value, [
    'source',
    'mainFrameId',
    'loaderId',
    'timeOriginMs',
    'navigationEvents',
    'frameCreatedEvents',
  ]) &&
  value.source === 'http://tauri.localhost/login' &&
  [value.mainFrameId, value.loaderId].every(
    (id) => typeof id === 'string' && /^[A-Za-z0-9_.:-]{1,128}$/.test(id),
  ) &&
  value.timeOriginMs === null &&
  value.navigationEvents === 0 &&
  value.frameCreatedEvents === 0;
const validTrust = (value) =>
  exactKeys(value, ['pointer', 'text', 'untrusted']) &&
  Object.values(value).every((n) => Number.isSafeInteger(n) && n >= 0 && n <= 128) &&
  value.untrusted === 0;
const MARKER = 'windows-native-sdk-ui-journey-requested';
export async function journeyBinaryPreflight(exe) {
  let tail = '';
  for await (const chunk of createReadStream(exe)) {
    const text = tail + chunk.toString('latin1');
    if (text.includes(MARKER))
      return {
        present: true,
        marker: MARKER,
        limitation: 'feature marker; not executable trust or signature verification',
      };
    tail = text.slice(-MARKER.length);
  }
  throw new Error('SDK journey requires its rebuilt native-perf EXE; no application started');
}
export function checkBound(value, owned, pid, port) {
  if (
    !exactKeys(value, [
      'schemaVersion',
      'scope',
      'runId',
      'root',
      'pid',
      'port',
      'browserPid',
      'binding',
      'timeOriginMs',
    ]) ||
    value?.schemaVersion !== 1 ||
    value.scope !== 'windows-native-sdk-ui-bound' ||
    value.runId !== owned.runId ||
    normalizeWindowsPath(value.root) !== normalizeWindowsPath(owned.root) ||
    value.pid !== pid ||
    value.port !== port ||
    !Number.isSafeInteger(value.browserPid) ||
    value.browserPid <= 0 ||
    !validBinding(value.binding) ||
    typeof value.binding.mainFrameId !== 'string' ||
    !value.binding.mainFrameId ||
    typeof value.binding.loaderId !== 'string' ||
    !value.binding.loaderId ||
    !Number.isFinite(value.timeOriginMs) ||
    value.timeOriginMs <= 0
  )
    throw new Error('SDK journey document binding mismatch');
  return value;
}
export function checkJourney(value, owned, bound, fixture) {
  if (
    !exactKeys(value, [
      'schemaVersion',
      'scope',
      'runId',
      'root',
      'pid',
      'port',
      'objectCount',
      'browserPid',
      'binding',
      'timeOriginMs',
      'inputMethod',
      'success',
      'reason',
      'failedStep',
      'calls',
      'phases',
      'lastProbe',
      'ipcAll',
      'elapsedMs',
      'unmeasured',
    ]) ||
    value?.schemaVersion !== 1 ||
    value.scope !== 'windows-native-sdk-ui-journey' ||
    value.success !== true ||
    value.reason !== null ||
    value.failedStep !== null ||
    value.runId !== owned.runId ||
    normalizeWindowsPath(value.root) !== normalizeWindowsPath(owned.root) ||
    value.pid !== bound.pid ||
    value.port !== bound.port ||
    value.browserPid !== bound.browserPid ||
    value.objectCount !== fixture.objectCount ||
    !Number.isFinite(value.elapsedMs) ||
    value.elapsedMs < 0 ||
    value.elapsedMs > 310000 ||
    JSON.stringify(value.unmeasured) !==
      JSON.stringify([
        'OCR',
        'attachment-preview',
        'system-sleep',
        'same-profile-warm-start',
        'other-platforms',
      ]) ||
    value.inputMethod !== 'SDK-CDP-Input' ||
    value.timeOriginMs !== bound.timeOriginMs ||
    !validBinding(value.binding) ||
    Object.keys(value.binding).some((key) => value.binding[key] !== bound.binding[key]) ||
    !Array.isArray(value.calls) ||
    value.calls.length > 128 ||
    value.calls.some((call) => !METHODS.has(call)) ||
    !value.calls.includes('Input.dispatchMouseEvent') ||
    value.calls.filter((call) => call === 'Input.insertText').length !== 3 ||
    !Array.isArray(value.phases) ||
    value.phases.length !== PHASES.length
  )
    throw new Error('SDK UI journey proof rejected');
  for (let index = 0; index < PHASES.length; index++) {
    const phase = value.phases[index];
    if (
      !exactKeys(phase, [
        'name',
        'success',
        'durationMs',
        'ipc',
        'inputTrust',
        'fromAtMs',
        'toAtMs',
      ]) ||
      phase.name !== PHASES[index] ||
      phase.success !== true ||
      !Number.isFinite(phase.durationMs) ||
      phase.durationMs < 0 ||
      phase.durationMs > 300000 ||
      !Number.isSafeInteger(phase.ipc?.attempts) ||
      phase.ipc.attempts < 0 ||
      phase.ipc.reason !== null ||
      !Number.isFinite(phase.fromAtMs) ||
      !Number.isFinite(phase.toAtMs) ||
      phase.toAtMs < phase.fromAtMs ||
      !validTrust(phase.inputTrust) ||
      !exactKeys(phase.ipc, ['attempts', 'commands', 'reason']) ||
      !phase.ipc.commands ||
      typeof phase.ipc.commands !== 'object' ||
      Array.isArray(phase.ipc.commands) ||
      Object.values(phase.ipc.commands).some(
        (count) => !Number.isSafeInteger(count) || count < 0,
      ) ||
      Object.values(phase.ipc.commands).reduce((sum, count) => sum + count, 0) !==
        phase.ipc.attempts
    )
      throw new Error('SDK phase timing/IPC/trust proof rejected');
  }
  const last = value.lastProbe;
  if (
    !exactKeys(last, [
      'schemaVersion',
      'scope',
      'runId',
      'step',
      'outcome',
      'href',
      'origin',
      'timeOriginMs',
      'atMs',
      'rootPresent',
      'frameCount',
      'target',
      'inputTrust',
      'observer',
    ]) ||
    last?.schemaVersion !== 1 ||
    last.scope !== 'windows-native-sdk-ui-probe' ||
    last.origin !== 'http://tauri.localhost' ||
    last.target !== null ||
    !Number.isFinite(last.atMs) ||
    last.atMs < 0 ||
    last.runId !== owned.runId ||
    last.timeOriginMs !== bound.timeOriginMs ||
    last.href !== 'http://tauri.localhost/' ||
    last.step !== 'home' ||
    last.outcome !== 'ready' ||
    last.rootPresent !== true ||
    last.frameCount !== 0 ||
    !validTrust(last.inputTrust) ||
    last.inputTrust.text < 3 ||
    last.inputTrust.pointer < 10
  )
    throw new Error('SDK final document or trusted input mismatch');
  if (
    !exactKeys(last.observer, [
      'schemaVersion',
      'scope',
      'runId',
      'valid',
      'total',
      'observedCount',
      'invalidReasons',
      'installedAtMs',
      'timeOriginMs',
      'maxEvents',
      'commands',
    ]) ||
    last.observer.commands.some((event) => !exactKeys(event, ['command', 'atMs']))
  )
    throw new Error('SDK observer contains unsupported fields');
  const observer = validateObserver(last.observer, owned.runId);
  if (
    !observer.valid ||
    observer.timeOriginMs !== bound.timeOriginMs ||
    last.observer.invalidReasons.length !== 0 ||
    last.observer.installedAtMs > last.atMs ||
    last.observer.commands.some((event) => event.atMs > last.atMs)
  )
    throw new Error('SDK full-session observer invalid');
  const ipcAll = ipcDelta(
    {
      valid: true,
      total: 0,
      commands: [],
      timeOriginMs: observer.timeOriginMs,
      installedAtMs: observer.installedAtMs,
    },
    observer,
  );
  if (
    !exactKeys(value.ipcAll, ['attempts', 'commands', 'reason']) ||
    value.ipcAll.reason !== null ||
    ipcAll.attempts !== value.ipcAll.attempts ||
    !value.ipcAll.commands ||
    Object.keys(value.ipcAll.commands).length !== ipcAll.commands.length ||
    ipcAll.commands.some((item) => value.ipcAll.commands[item.command] !== item.attempts)
  )
    throw new Error('SDK full-session IPC proof mismatch');
  return { ...value, ipcAll };
}
export function summarizeJourneys(samples, requestedSamples, integrity = true) {
  return summarize(
    samples.map((sample) =>
      integrity && sample.success === true ? sample : { ...sample, phases: [] },
    ),
    requestedSamples,
  );
}
async function readOwned(root, filename) {
  if (!FILES.has(filename)) throw new Error('Unsupported owned marker');
  const file = path.join(root, filename),
    stat = await lstat(file);
  if (
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    stat.size > 1024 * 1024 ||
    normalizeWindowsPath(await realpath(file)) !== normalizeWindowsPath(file)
  )
    throw new Error('Owned SDK marker is not bounded and regular');
  return JSON.parse((await readFile(file, 'utf8')).replace(/^\uFEFF/, ''));
}
async function waitOwned(root, filename, child, stop, timeoutMs) {
  const began = Date.now();
  while (Date.now() - began < timeoutMs) {
    if (stop() || child.exitCode !== null || child.signalCode)
      throw new Error('SDK journey interrupted or owned application exited');
    try {
      return await readOwned(root, filename);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    await pause(100);
  }
  throw new Error('SDK journey owned marker timed out');
}
async function inventory(root) {
  const result = [];
  async function visit(dir, depth = 0) {
    if (depth > 32 || result.length > 128) throw new Error('Synthetic inventory bounds exceeded');
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name),
        stat = await lstat(file);
      if (
        stat.isSymbolicLink() ||
        normalizeWindowsPath(await realpath(file)) !== normalizeWindowsPath(file)
      )
        throw new Error('Synthetic source traverses a link');
      if (stat.isDirectory()) await visit(file, depth + 1);
      else if (stat.isFile())
        result.push({ path: path.relative(root, file), sha256: await sha256(file) });
      else throw new Error('Unexpected synthetic entry');
    }
  }
  await visit(root);
  return result.sort((a, b) => a.path.localeCompare(b.path));
}
async function sample(options, index, shouldStop) {
  const root = path.join(options.output, 'sample-' + String(index).padStart(3, '0'));
  const result = { index, root, success: false, phases: [], startedAt: new Date().toISOString() };
  let owner, completion, launch;
  const env = { ...process.env };
  delete env.SOLOSOUL_REGISTRY_PUBKEY;
  try {
    const at = Date.now(),
      prepare = startChild(
        options.exe,
        ['--native-perf-prepare', root, '--fixture', options.fixture],
        env,
      );
    completion = prepare.completion;
    if (!prepare.child.pid) throw new Error('Native SDK preparation did not spawn');
    owner = new OwnedProcess(prepare.child, options.exe, at);
    await owner.sample();
    result.prepare = await deadline(completion, 120000, 'SDK fixture preparation');
    if (result.prepare.exitCode !== 0) throw new Error('Native SDK fixture preparation failed');
    owner = null;
    completion = null;
    const owned = preparedManifest(
      await readOwned(root, 'native-perf-owned.json'),
      root,
      options.manifest,
      options.fixture,
    );
    result.owned = owned;
    const port = await unusedPort();
    result.port = port;
    if (shouldStop()) throw new Error('Interrupted before SDK UI launch');
    const launchedAt = Date.now();
    launch = startChild(
      options.exe,
      [
        '--native-perf-root',
        root,
        '--native-perf-port',
        String(port),
        '--native-perf-journey',
        'sdk-input',
      ],
      env,
    );
    completion = launch.completion;
    if (!launch.child.pid) throw new Error('SDK UI application did not spawn');
    result.processId = launch.child.pid;
    owner = new OwnedProcess(launch.child, options.exe, launchedAt, owned.webview);
    const bound = checkBound(
      await waitOwned(root, 'native-perf-sdk-journey-bound.json', launch.child, shouldStop, 65000),
      owned,
      launch.child.pid,
      port,
    );
    result.bound = bound;
    result.beforeSnapshot = await owner.sample();
    const before = selectDiagnosticIdentities(owner, result.beforeSnapshot);
    if (before[1].pid !== bound.browserPid)
      throw new Error('SDK browser does not match verified owned browser before input');
    result.beforeProcessDiagnostics = await processDiagnostics(before, owned);
    result.consumed = checkConsumed(
      await readOwned(root, 'native-perf-consumed.json'),
      owned,
      launch.child.pid,
      port,
    );
    if (shouldStop()) throw new Error('Interrupted before input authorization');
    const authorization = {
      schemaVersion: 1,
      scope: 'windows-native-sdk-ui-input-authorized',
      runId: owned.runId,
      pid: launch.child.pid,
      browserPid: bound.browserPid,
    };
    await newJson(path.join(root, '.native-perf-sdk-journey-authorized.tmp'), authorization);
    await rename(
      path.join(root, '.native-perf-sdk-journey-authorized.tmp'),
      path.join(root, 'native-perf-sdk-journey-authorized.json'),
    );
    result.nativeProof = await waitOwned(
      root,
      'native-perf-sdk-journey.json',
      launch.child,
      shouldStop,
      310000,
    );
    const proof = checkJourney(result.nativeProof, owned, bound, options.manifest);
    result.afterSnapshot = await owner.sample();
    const after = selectDiagnosticIdentities(owner, result.afterSnapshot);
    sameSdkIdentities(before, after);
    result.afterProcessDiagnostics = await processDiagnostics(after, owned);
    result.phases = proof.phases;
    result.ipcAll = proof.ipcAll;
    result.success = true;
  } catch (error) {
    result.error = safeError(error);
  } finally {
    if (owner) result.cleanup = await owner.cleanup();
    if (completion)
      result.ownedProcessExit = await deadline(completion, 6000, 'Owned SDK journey exit').catch(
        (error) => ({ exitCode: null, reason: safeError(error) }),
      );
    if (owner) {
      result.cleanupIntegrity = cleanupOutcome(result.cleanup, result.ownedProcessExit);
      if (!result.cleanupIntegrity.complete) {
        result.success = false;
        result.cleanupIncomplete = true;
        result.ownedChildHandleReleased = releaseUnfinishedChild(
          owner.child,
          result.ownedProcessExit,
        );
      }
    }
    result.finishedAt = new Date().toISOString();
    await newJson(
      path.join(options.output, 'sample-' + String(index).padStart(3, '0') + '.json'),
      result,
    );
  }
  return result;
}
export async function main(args = process.argv.slice(2)) {
  const parsed = parseArgs(args);
  if (parsed.help) {
    process.stdout.write(
      'Usage: node scripts/native-perf-sdk-journey.mjs --exe ABS --fixture ABS --output NEW_ABS --samples N (N >= 3)\n',
    );
    return 0;
  }
  if (process.platform !== 'win32') throw new Error('SDK UI journey is Windows only');
  const options = await validateInputs(parsed),
    preflight = await journeyBinaryPreflight(options.exe),
    sourceBefore = await inventory(options.fixture);
  await mkdir(options.output);
  const report = {
    schemaVersion: 1,
    task: 'RF-312',
    scope: 'windows-native-sdk-input-performance',
    startedAt: new Date().toISOString(),
    samplesRequested: options.samples,
    nodeVersion: process.version,
    osRelease: os.release(),
    exeSha256: await sha256(options.exe),
    preflight,
    fixture: options.manifest,
    sourceBefore,
    samples: [],
    definitions: {
      startup:
        'native runtime configuration to public login form plus two frames and SDK document binding; excludes OS spawn and includes SDK overhead',
      actions:
        'native Instant around fixed browser Input actions, visible end state, two frames and same-frame SDK checks; includes SDK overhead',
      memory:
        'verified owned Windows working sets before input and after journey; no peak-memory claim',
      ipc: 'real Tauri fetch observer; names/counts only; invalid or changed prefix rejects sample',
      unmeasured: [
        'OCR',
        'attachment-preview',
        'system-sleep',
        'same-profile-warm-start',
        'other-platforms',
      ],
    },
  };
  let interrupted = false;
  const stop = () => {
    interrupted = true;
  };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  try {
    for (let index = 1; index <= options.samples && !interrupted; index++) {
      process.stdout.write(`RF-312 SDK UI sample ${index}/${options.samples}\n`);
      const result = await sample(options, index, () => interrupted);
      report.samples.push(result);
      if (result.cleanupIncomplete) {
        report.stoppedReason = result.cleanupIntegrity.reason;
        break;
      }
    }
  } finally {
    process.off('SIGINT', stop);
    process.off('SIGTERM', stop);
    report.interrupted = interrupted;
    report.finishedAt = new Date().toISOString();
    try {
      report.sourceAfter = await inventory(options.fixture);
      report.sourceUnchanged = JSON.stringify(sourceBefore) === JSON.stringify(report.sourceAfter);
    } catch (error) {
      report.sourceUnchanged = false;
      report.sourceProofError = safeError(error);
    }
    report.summary = summarizeJourneys(
      report.samples,
      options.samples,
      report.sourceUnchanged && !interrupted,
    );
    report.success =
      !interrupted &&
      report.sourceUnchanged &&
      report.samples.length === options.samples &&
      report.samples.every((sample) => sample.success);
    await newJson(path.join(options.output, 'native-perf-sdk-journey-results.json'), report);
  }
  process.stdout.write(
    'RF-312 SDK UI results: ' +
      path.join(options.output, 'native-perf-sdk-journey-results.json') +
      '\n',
  );
  return report.success ? 0 : 1;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  main()
    .then((code) => {
      process.exitCode = code;
    })
    .catch((error) => {
      process.stderr.write(safeError(error) + '\n');
      process.exitCode = 1;
    });
