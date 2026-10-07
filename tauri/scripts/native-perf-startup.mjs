#!/usr/bin/env node
/** RF-312: paired fresh-profile and reused-profile native process startup. */
import { mkdir, readFile, readdir, lstat, realpath } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { isDeepStrictEqual } from 'node:util';
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
import { checkBound } from './native-perf-sdk-journey.mjs';
import { sameSdkIdentities } from './native-perf-sdk-cdp.mjs';
const eq = (a, b) => isDeepStrictEqual(a, b);
const exact = (v, keys) =>
  v &&
  typeof v === 'object' &&
  !Array.isArray(v) &&
  Object.keys(v).length === keys.length &&
  keys.every((k) => Object.hasOwn(v, k));
const samePath = (a, b) => normalizeWindowsPath(a) === normalizeWindowsPath(b);
const finite = (n, max = 300000) => Number.isFinite(n) && n >= 0 && n <= max;
const ID = /^[0-9a-f]{32}$/;
export function unmeasured(generation) {
  return [
    'password-unlock',
    'workspace',
    'search-needle',
    'application-lock',
    'password-reunlock',
    'OCR',
    'attachment-preview',
    'system-sleep',
    'other-platforms',
    ...(generation === 1 ? ['same-profile-warm-start'] : []),
  ];
}
export async function startupBinaryPreflight(exe) {
  const markers = [
    'windows-native-sdk-startup-requested',
    'windows-native-sdk-startup-restart-ticket',
  ];
  const found = new Set();
  let tail = '';
  for await (const chunk of createReadStream(exe)) {
    const text = tail + chunk.toString('latin1');
    for (const marker of markers) if (text.includes(marker)) found.add(marker);
    tail = text.slice(-128);
  }
  if (found.size !== markers.length)
    throw new Error(
      'Startup requires its rebuilt optional native-perf EXE; no application started',
    );
  return {
    markers,
    matched: true,
    note: 'offline compatibility check, not executable signature verification',
  };
}
export function checkStartup(value, owned, bound, generation, launchId) {
  if (
    ![1, 2].includes(generation) ||
    !ID.test(launchId) ||
    !ID.test(owned.runId) ||
    (generation === 1 ? launchId !== owned.runId : launchId === owned.runId)
  )
    throw new Error('Startup launch identity invalid');
  checkBound(bound, { root: owned.root, runId: launchId }, bound.pid, bound.port);
  if (
    !exact(value, [
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
      'startup',
    ]) ||
    value.schemaVersion !== 1 ||
    value.scope !== 'windows-native-sdk-startup' ||
    !samePath(value.root, owned.root) ||
    value.runId !== launchId ||
    value.objectCount !== owned.fixture.objectCount ||
    value.success !== true ||
    value.reason !== null ||
    value.failedStep !== null ||
    value.inputMethod !== 'SDK-CDP-read-only' ||
    !eq(value.calls, ['Runtime.evaluate', 'Page.getFrameTree']) ||
    value.pid !== bound.pid ||
    value.browserPid !== bound.browserPid ||
    value.pid === value.browserPid ||
    value.port !== bound.port ||
    !finite(value.elapsedMs) ||
    value.timeOriginMs !== bound.timeOriginMs ||
    !eq(value.binding, bound.binding) ||
    !eq(value.unmeasured, unmeasured(generation)) ||
    !exact(value.startup, ['generation', 'ownerRunId', 'evidenceRoot']) ||
    value.startup.generation !== generation ||
    value.startup.ownerRunId !== owned.runId ||
    !samePath(value.startup.evidenceRoot, path.win32.join(owned.root, `startup-0${generation}`))
  )
    throw new Error('Read-only startup proof rejected');
  const p = value.lastProbe;
  if (
    !exact(p, [
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
    p.schemaVersion !== 1 ||
    p.scope !== 'windows-native-sdk-ui-probe' ||
    p.runId !== launchId ||
    p.step !== 'startup' ||
    p.outcome !== 'ready' ||
    p.href !== 'http://tauri.localhost/login' ||
    p.origin !== 'http://tauri.localhost' ||
    p.timeOriginMs !== bound.timeOriginMs ||
    !finite(p.atMs) ||
    p.rootPresent !== true ||
    p.frameCount !== 0 ||
    p.target !== null ||
    !eq(p.inputTrust, { pointer: 0, text: 0, untrusted: 0 })
  )
    throw new Error('Read-only login probe rejected');
  const o = p.observer;
  if (
    !exact(o, [
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
    !Array.isArray(o.commands) ||
    o.commands.some((e) => !exact(e, ['command', 'atMs']))
  )
    throw new Error('Startup observer fields invalid');
  const observer = validateObserver(o, launchId);
  if (
    !observer.valid ||
    observer.timeOriginMs !== bound.timeOriginMs ||
    o.installedAtMs > p.atMs ||
    o.invalidReasons.length !== 0 ||
    o.commands.some((e) => e.atMs > p.atMs)
  )
    throw new Error('Startup observer invalid');
  const delta = ipcDelta(
    {
      valid: true,
      total: 0,
      commands: [],
      timeOriginMs: observer.timeOriginMs,
      installedAtMs: observer.installedAtMs,
    },
    observer,
  );
  const ipc = {
    attempts: delta.attempts,
    commands: Object.fromEntries(delta.commands.map((e) => [e.command, e.attempts])),
    reason: null,
  };
  const phase = value.phases?.[0];
  if (
    !Array.isArray(value.phases) ||
    value.phases.length !== 1 ||
    !exact(phase, ['name', 'success', 'durationMs', 'ipc', 'inputTrust', 'fromAtMs', 'toAtMs']) ||
    phase.name !== 'startup' ||
    phase.success !== true ||
    !finite(phase.durationMs) ||
    phase.fromAtMs !== 0 ||
    phase.toAtMs !== p.atMs ||
    !eq(phase.inputTrust, p.inputTrust) ||
    !eq(phase.ipc, ipc) ||
    !eq(value.ipcAll, ipc)
  )
    throw new Error('Startup timing/IPC mismatch');
  return value;
}
export function checkPair(first, warm) {
  if (
    first.runId === warm.runId ||
    first.pid === warm.pid ||
    first.browserPid === warm.browserPid ||
    first.port === warm.port ||
    warm.timeOriginMs <= first.timeOriginMs ||
    first.binding.mainFrameId === warm.binding.mainFrameId ||
    first.binding.loaderId === warm.binding.loaderId ||
    first.startup.ownerRunId !== warm.startup.ownerRunId ||
    !samePath(first.root, warm.root) ||
    first.startup.generation !== 1 ||
    warm.startup.generation !== 2
  )
    throw new Error('Warm launch reused identity, frame, clock or evidence');
}
const FILES = new Set([
  'native-perf-owned.json',
  'native-perf-consumed.json',
  'native-perf-startup-stopped.json',
  'native-perf-startup-restart-ticket.json',
  'native-perf-startup-restart-consumed.json',
  'startup-01/native-perf-sdk-journey-bound.json',
  'startup-01/native-perf-sdk-journey.json',
  'startup-02/native-perf-sdk-journey-bound.json',
  'startup-02/native-perf-sdk-journey.json',
]);
async function readOwned(root, name) {
  if (!FILES.has(name)) throw new Error('Unsupported startup marker');
  const file = path.join(root, name),
    stat = await lstat(file);
  for (const dir of [root, path.dirname(file)]) {
    const s = await lstat(dir);
    if (!s.isDirectory() || s.isSymbolicLink() || !samePath(await realpath(dir), dir))
      throw new Error('Startup marker directory traverses a link');
  }
  if (
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    stat.size > 1024 * 1024 ||
    !samePath(await realpath(file), file)
  )
    throw new Error('Startup marker must be bounded and regular');
  return JSON.parse((await readFile(file, 'utf8')).replace(/^\uFEFF/, ''));
}
async function waitOwned(root, name, child, stop) {
  const began = Date.now();
  while (Date.now() - began < 65000) {
    if (stop() || child.exitCode !== null || child.signalCode)
      throw new Error('Startup interrupted or owned process exited');
    try {
      return await readOwned(root, name);
    } catch (e) {
      if (e.code !== 'ENOENT') throw e;
    }
    await pause(100);
  }
  throw new Error('Startup owned marker timed out');
}
export async function inventory(root) {
  const rows = [];
  async function visit(dir, depth) {
    if (depth > 32 || rows.length > 128) throw new Error('Public fixture inventory exceeds bounds');
    for (const e of await readdir(dir, { withFileTypes: true })) {
      const file = path.join(dir, e.name),
        s = await lstat(file);
      if (s.isSymbolicLink() || !samePath(await realpath(file), file))
        throw new Error('Fixture traverses a link');
      if (s.isDirectory()) await visit(file, depth + 1);
      else if (s.isFile())
        rows.push({ path: path.relative(root, file), sha256: await sha256(file) });
      else throw new Error('Unexpected fixture entry');
    }
  }
  await visit(root, 0);
  return rows.sort((a, b) => a.path.localeCompare(b.path));
}
async function directoryIdentity(owned) {
  const rows = [];
  for (const dir of [owned.webview, owned.profile, owned.vault]) {
    const s = await lstat(dir);
    if (!s.isDirectory() || s.isSymbolicLink() || !samePath(await realpath(dir), dir))
      throw new Error('Warm profile directory changed');
    rows.push({ path: normalizeWindowsPath(dir), birthtimeMs: s.birthtimeMs });
  }
  return rows;
}
export function checkTicket(t, owned, first, hashes, identitiesCount) {
  if (
    !exact(t, [
      'schemaVersion',
      'scope',
      'root',
      'ownerRunId',
      'runId',
      'evidenceRoot',
      'ownedSha256',
      'consumedSha256',
      'proofSha256',
      'stoppedSha256',
      'exeSha256',
      'uiPreferencesSha256',
      'updateSourceCandidates',
      'priorPid',
      'priorBrowserPid',
      'priorPort',
      'exitCheck',
    ]) ||
    t.schemaVersion !== 1 ||
    t.scope !== 'windows-native-sdk-startup-restart-ticket' ||
    !samePath(t.root, owned.root) ||
    t.ownerRunId !== owned.runId ||
    !ID.test(t.runId) ||
    t.runId === owned.runId ||
    !samePath(t.evidenceRoot, path.win32.join(owned.root, 'startup-02')) ||
    t.priorPid !== first.pid ||
    t.priorBrowserPid !== first.browserPid ||
    t.priorPort !== first.port ||
    Object.keys(hashes).some((k) => t[k] !== hashes[k]) ||
    !validExitCheck(t.exitCheck, identitiesCount) ||
    !validSourceCandidates(t.updateSourceCandidates)
  )
    throw new Error('Warm ticket does not bind the stopped prime');
  return t;
}
function validSourceCandidates(value) {
  return (
    exact(value, ['manifest', 'release']) &&
    ['manifest', 'release'].every(
      (channel) =>
        Array.isArray(value[channel]) &&
        value[channel].length <= 128 &&
        value[channel].every(
          (url) => typeof url === 'string' && url.length <= 4096 && url.startsWith('https://'),
        ),
    )
  );
}
export function checkStartupPreferences(value, candidates) {
  if (!validSourceCandidates(candidates))
    throw new Error('Invalid native update-source candidates');
  const { updateSources: cache, ...fixed } = value ?? {};
  if (
    !eq(fixed, {
      theme: 'light',
      accentColor: 'ocean',
      customAccentHex: '',
      reduceMotion: false,
      androidGlass: 'local',
      language: 'en-US',
      hasSeenOnboarding: true,
      notificationPermissionRequested: true,
    })
  )
    throw new Error('Startup changed fixed UI preferences');
  if (cache === undefined) return;
  if (
    !exact(cache, ['manifest', 'release', 'lastChannel']) ||
    !['manifest', 'release'].includes(cache.lastChannel) ||
    !cache[cache.lastChannel]
  )
    throw new Error('Invalid startup update-source cache');
  for (const channel of ['manifest', 'release']) {
    const slot = cache[channel];
    if (slot === null) continue;
    if (
      !exact(slot, ['url', 'probedAt']) ||
      !candidates[channel].includes(slot.url) ||
      !Number.isSafeInteger(slot.probedAt) ||
      slot.probedAt <= 0 ||
      slot.probedAt > Math.floor(Date.now() / 1000)
    )
      throw new Error('Startup cache is not an allowed bounded source');
  }
}
async function checkEndFixture(owned, candidates) {
  const files = [];
  const account = owned.fixture.accountId;
  const rootNames = new Set([
    account,
    'accounts.json',
    'ui_preferences.json',
    'rf312-fixture.json',
    'accounts.bak',
    '.lock',
  ]);
  for (const entry of await readdir(owned.vault, { withFileTypes: true })) {
    if (!rootNames.has(entry.name)) throw new Error('Unexpected startup Vault entry');
    if (entry.name === account) continue;
    const file = path.join(owned.vault, entry.name),
      s = await lstat(file);
    if (!s.isFile() || s.isSymbolicLink() || !samePath(await realpath(file), file))
      throw new Error('Startup Vault entry is not regular');
  }
  const accountDir = path.join(owned.vault, account);
  const accountNames = new Set(
    owned.fixture.files
      .filter((f) => path.win32.dirname(f.relativePath) === account)
      .map((f) => path.win32.basename(f.relativePath)),
  );
  accountNames.add('vault.db.pre_enc.bak');
  for (const entry of await readdir(accountDir, { withFileTypes: true })) {
    const file = path.join(accountDir, entry.name),
      s = await lstat(file);
    if (
      !accountNames.has(entry.name) ||
      !s.isFile() ||
      s.isSymbolicLink() ||
      !samePath(await realpath(file), file)
    )
      throw new Error('Unexpected startup account entry');
  }
  for (const file of owned.fixture.files) {
    const target = path.join(owned.vault, file.relativePath),
      s = await lstat(target);
    for (const dir of [owned.vault, path.dirname(target)]) {
      const d = await lstat(dir);
      if (!d.isDirectory() || d.isSymbolicLink() || !samePath(await realpath(dir), dir))
        throw new Error('Startup fixture directory is not regular');
    }
    const limit = file.relativePath === 'ui_preferences.json' ? 1024 * 1024 : 256 * 1024 * 1024;
    if (
      !s.isFile() ||
      s.isSymbolicLink() ||
      s.size > limit ||
      !samePath(await realpath(target), target)
    )
      throw new Error('Startup fixture file is not bounded and regular');
    const actual = await sha256(target);
    if (file.relativePath !== 'ui_preferences.json' && actual !== file.sha256)
      throw new Error('Startup changed immutable Vault fixture');
    files.push({ path: file.relativePath, sha256: actual });
  }
  const preferences = JSON.parse(
    (await readFile(path.join(owned.vault, 'ui_preferences.json'), 'utf8')).replace(/^\uFEFF/, ''),
  );
  checkStartupPreferences(preferences, candidates);
  return { files, preferences, immutableFilesUnchanged: true };
}
function validExitCheck(e, count) {
  return (
    exact(e, ['verified', 'queryPid', 'checkedIdentities', 'checkedAtUnixMs']) &&
    e.verified === true &&
    Number.isSafeInteger(e.queryPid) &&
    e.queryPid > 0 &&
    e.checkedIdentities === count &&
    Number.isSafeInteger(e.checkedAtUnixMs) &&
    e.checkedAtUnixMs <= Date.now() &&
    Date.now() - e.checkedAtUnixMs <= 300000
  );
}
function checkUsed(u, t, owned, pid, port, ticketHash, count) {
  if (
    !exact(u, [
      'schemaVersion',
      'scope',
      'root',
      'ownerRunId',
      'runId',
      'pid',
      'port',
      'ticketSha256',
      'evidenceRoot',
      'exitCheck',
    ]) ||
    u.schemaVersion !== 1 ||
    u.scope !== 'windows-native-sdk-startup-restart-consumed' ||
    !samePath(u.root, owned.root) ||
    u.ownerRunId !== owned.runId ||
    u.runId !== t.runId ||
    u.pid !== pid ||
    u.port !== port ||
    u.ticketSha256 !== ticketHash ||
    !samePath(u.evidenceRoot, t.evidenceRoot) ||
    !validExitCheck(u.exitCheck, count)
  )
    throw new Error('Warm consumed identity mismatch');
  return u;
}
async function prepareChild(exe, args, env) {
  const at = Date.now(),
    launch = startChild(exe, args, env);
  if (!launch.child.pid) throw new Error('Preparation did not spawn');
  const owner = new OwnedProcess(launch.child, exe, at);
  try {
    await owner.sample();
    const exit = await deadline(launch.completion, 120000, 'Native startup preparation');
    if (exit.exitCode !== 0) throw new Error('Native startup preparation failed');
    return { pid: launch.child.pid, ...exit };
  } catch (e) {
    const cleanup = await owner.cleanup();
    const exit = await deadline(launch.completion, 6000, 'Preparation exit').catch((error) => ({
      exitCode: null,
      reason: safeError(error),
    }));
    const integrity = cleanupOutcome(cleanup, exit);
    if (!integrity.complete) releaseUnfinishedChild(launch.child, exit);
    const error = new Error(safeError(e));
    error.cleanupIncomplete = !integrity.complete;
    error.cleanup = { cleanup, exit, integrity };
    throw error;
  }
}
async function launchStartup(options, owned, generation, launchId, ticket, stop) {
  let port = await unusedPort();
  while (port === ticket?.priorPort) port = await unusedPort();
  const evidence = `startup-0${generation}`,
    result = { generation, launchId, port, success: false };
  let owner, launch;
  try {
    if (stop()) throw new Error('Interrupted before startup launch');
    const at = Date.now();
    launch = startChild(
      options.exe,
      [
        '--native-perf-root',
        owned.root,
        '--native-perf-port',
        String(port),
        '--native-perf-journey',
        'sdk-startup',
        ...(generation === 2 ? ['--native-perf-restart', 'startup'] : []),
      ],
      options.env,
    );
    if (!launch.child.pid) throw new Error('Startup application did not spawn');
    result.pid = launch.child.pid;
    owner = new OwnedProcess(launch.child, options.exe, at, owned.webview);
    const context = { root: owned.root, runId: launchId };
    result.bound = checkBound(
      await waitOwned(
        owned.root,
        `${evidence}/native-perf-sdk-journey-bound.json`,
        launch.child,
        stop,
      ),
      context,
      launch.child.pid,
      port,
    );
    result.beforeSnapshot = await owner.sample();
    const before = selectDiagnosticIdentities(owner, result.beforeSnapshot);
    if (before[1].pid !== result.bound.browserPid)
      throw new Error('Startup browser does not match actual owned UDF');
    result.beforeProcessDiagnostics = await processDiagnostics(before, owned);
    result.nativeProof = await waitOwned(
      owned.root,
      `${evidence}/native-perf-sdk-journey.json`,
      launch.child,
      stop,
    );
    const proof = checkStartup(result.nativeProof, owned, result.bound, generation, launchId);
    result.afterSnapshot = await owner.sample();
    const after = selectDiagnosticIdentities(owner, result.afterSnapshot);
    sameSdkIdentities(before, after);
    result.afterProcessDiagnostics = await processDiagnostics(after, owned);
    result.consumed =
      generation === 1
        ? checkConsumed(
            await readOwned(owned.root, 'native-perf-consumed.json'),
            owned,
            launch.child.pid,
            port,
          )
        : checkUsed(
            await readOwned(owned.root, 'native-perf-startup-restart-consumed.json'),
            ticket,
            owned,
            launch.child.pid,
            port,
            await sha256(path.join(owned.root, 'native-perf-startup-restart-ticket.json')),
            ticket.exitCheck.checkedIdentities,
          );
    result.phases = proof.phases;
    result.success = true;
  } catch (e) {
    result.error = safeError(e);
  } finally {
    if (owner) {
      result.cleanup = await owner.cleanup();
      result.ownedProcessExit = await deadline(launch.completion, 6000, 'Owned startup exit').catch(
        (e) => ({ exitCode: null, reason: safeError(e) }),
      );
      result.cleanupIntegrity = cleanupOutcome(result.cleanup, result.ownedProcessExit);
      result.identities = [...owner.known.values()].map((r) => ({
        pid: r.pid,
        creationMs: r.creationMs,
        executableName: path.win32.basename(r.executablePath),
      }));
      if (!result.cleanupIntegrity.complete) {
        result.success = false;
        result.cleanupIncomplete = true;
        result.ownedChildHandleReleased = releaseUnfinishedChild(
          owner.child,
          result.ownedProcessExit,
        );
      }
    }
  }
  return result;
}
async function samplePair(options, index, stop) {
  const root = path.join(options.output, 'sample-' + String(index).padStart(3, '0'));
  const result = { index, root, success: false, startedAt: new Date().toISOString() };
  try {
    result.prepare = await prepareChild(
      options.exe,
      ['--native-perf-prepare', root, '--fixture', options.fixture],
      options.env,
    );
    const owned = preparedManifest(
      await readOwned(root, 'native-perf-owned.json'),
      root,
      options.manifest,
      options.fixture,
    );
    result.owned = owned;
    result.profileBefore = await directoryIdentity(owned);
    result.first = await launchStartup(options, owned, 1, owned.runId, null, stop);
    if (!result.first.success) throw new Error('First-profile startup failed');
    result.profileAfterFirst = await directoryIdentity(owned);
    if (!eq(result.profileBefore, result.profileAfterFirst))
      throw new Error('First profile identity changed');
    const hashes = {
      ownedSha256: await sha256(path.join(root, 'native-perf-owned.json')),
      consumedSha256: await sha256(path.join(root, 'native-perf-consumed.json')),
      proofSha256: await sha256(path.join(root, 'startup-01/native-perf-sdk-journey.json')),
      exeSha256: options.exeSha256,
      uiPreferencesSha256: await sha256(path.join(owned.vault, 'ui_preferences.json')),
    };
    const stopped = {
      schemaVersion: 1,
      scope: 'windows-native-sdk-startup-stopped',
      root: owned.root,
      ownerRunId: owned.runId,
      pid: result.first.pid,
      browserPid: result.first.bound.browserPid,
      ...hashes,
      identities: result.first.identities,
    };
    await newJson(path.join(root, 'native-perf-startup-stopped.json'), stopped);
    hashes.stoppedSha256 = await sha256(path.join(root, 'native-perf-startup-stopped.json'));
    result.warmPrepare = await prepareChild(
      options.exe,
      ['--native-perf-warm-prepare', owned.root],
      options.env,
    );
    result.ticket = checkTicket(
      await readOwned(root, 'native-perf-startup-restart-ticket.json'),
      owned,
      result.first.nativeProof,
      hashes,
      stopped.identities.length,
    );
    if (result.ticket.exitCheck.checkedIdentities !== stopped.identities.length)
      throw new Error('Warm exit check did not cover all journaled identities');
    result.warm = await launchStartup(options, owned, 2, result.ticket.runId, result.ticket, stop);
    if (!result.warm.success) throw new Error('Same-profile startup failed');
    checkPair(result.first.nativeProof, result.warm.nativeProof);
    result.profileAfterWarm = await directoryIdentity(owned);
    if (!eq(result.profileBefore, result.profileAfterWarm))
      throw new Error('Warm profile was replaced');
    const { uiPreferencesSha256, ...originalHashes } = hashes;
    result.firstUiPreferencesSha256 = uiPreferencesSha256;
    result.originalEvidenceUnchanged = eq(originalHashes, {
      ownedSha256: await sha256(path.join(root, 'native-perf-owned.json')),
      consumedSha256: await sha256(path.join(root, 'native-perf-consumed.json')),
      proofSha256: await sha256(path.join(root, 'startup-01/native-perf-sdk-journey.json')),
      exeSha256: await sha256(options.exe),
      stoppedSha256: await sha256(path.join(root, 'native-perf-startup-stopped.json')),
    });
    if (!result.originalEvidenceUnchanged)
      throw new Error('Original consumed/proof/owner/stop/EXE changed');
    result.fixtureAfterWarm = await checkEndFixture(owned, result.ticket.updateSourceCandidates);
    result.success = true;
  } catch (e) {
    result.error = safeError(e);
    if (e.cleanupIncomplete) result.cleanupIncomplete = true;
    if (e.cleanup) result.prepareCleanup = e.cleanup;
  }
  result.cleanupIncomplete ||=
    result.first?.cleanupIncomplete || result.warm?.cleanupIncomplete || false;
  result.finishedAt = new Date().toISOString();
  await newJson(
    path.join(options.output, 'sample-' + String(index).padStart(3, '0') + '.json'),
    result,
  );
  return result;
}
export function summarizePairs(samples, n, integrity = true) {
  const complete =
    integrity && samples.length === n && samples.every((s) => s.success && !s.cleanupIncomplete);
  return {
    complete,
    acceptedPairs: complete ? n : 0,
    requestedPairs: n,
    metrics: complete
      ? {
          freshProfile: summarize(
            samples.map((s) => s.first),
            n,
          )[0],
          reusedProfile: summarize(
            samples.map((s) => s.warm),
            n,
          )[0],
        }
      : null,
  };
}
export async function main(args = process.argv.slice(2)) {
  const parsed = parseArgs(args);
  if (parsed.help) {
    process.stdout.write(
      'Usage: node scripts/native-perf-startup.mjs --exe ABS --fixture ABS --output NEW_ABS --samples N (N >= 3)\n',
    );
    return 0;
  }
  if (process.platform !== 'win32') throw new Error('Native startup is Windows only');
  const options = await validateInputs(parsed),
    preflight = await startupBinaryPreflight(options.exe),
    sourceBefore = await inventory(options.fixture);
  options.env = { ...process.env };
  delete options.env.SOLOSOUL_REGISTRY_PUBKEY;
  options.exeSha256 = await sha256(options.exe);
  await mkdir(options.output);
  const report = {
    schemaVersion: 1,
    task: 'RF-312',
    scope: 'windows-native-same-profile-startup',
    startedAt: new Date().toISOString(),
    samplesRequested: options.samples,
    exeSha256: options.exeSha256,
    preflight,
    nodeVersion: process.version,
    osRelease: os.release(),
    fixture: options.manifest,
    sourceBefore,
    samples: [],
    definitions: {
      startup:
        'native runtime configuration to usable password login form, two frames and SDK binding; excludes OS process spawn and restart preflight; includes SDK overhead',
      profiles:
        'new private profile followed by the same profile after verified owned process termination; no Vault/profile reset, no OS cache eviction; not OS cold startup',
      memory: 'verified before/after owned process working sets, not peak memory',
      unmeasured: unmeasured(2),
    },
  };
  let interrupted = false;
  const stop = () => {
    interrupted = true;
  };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  try {
    for (let i = 1; i <= options.samples && !interrupted; i++) {
      process.stdout.write(`RF-312 startup pair ${i}/${options.samples}\n`);
      const result = await samplePair(options, i, () => interrupted);
      report.samples.push(result);
      if (result.cleanupIncomplete) {
        report.stoppedReason = 'Owned cleanup incomplete; no further launch';
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
      report.sourceUnchanged = eq(sourceBefore, report.sourceAfter);
    } catch (e) {
      report.sourceUnchanged = false;
      report.sourceProofError = safeError(e);
    }
    report.summary = summarizePairs(
      report.samples,
      options.samples,
      report.sourceUnchanged && !interrupted,
    );
    await newJson(path.join(options.output, 'report.json'), report);
  }
  process.stdout.write(JSON.stringify(report.summary) + '\n');
  return report.summary.complete ? 0 : 1;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  main()
    .then((code) => {
      process.exitCode = code;
    })
    .catch((e) => {
      process.stderr.write(safeError(e) + '\n');
      process.exitCode = 1;
    });
