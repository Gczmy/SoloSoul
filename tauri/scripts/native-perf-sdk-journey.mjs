#!/usr/bin/env node
/** RF-312：固定公开合成Vault、真实WebView2 SDK Input行程，独立于只读诊断。 */
import { mkdir, readFile, readdir, lstat, realpath, rename } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { setTimeout as pause } from 'node:timers/promises';
import {
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
import { verifyPreparedMediaFiles } from './native-perf-media-contract.mjs';
import { MemorySampler, parseJourneyArgs } from './native-perf-memory.mjs';
import { checkPdfDiagnostic, PDF_UNMEASURED } from './native-perf-pdf-contract.mjs';
import {
  checkPdfFirstPage,
  summarizePdfFirstPage,
  PDF_PREVIEW_UNMEASURED,
} from './native-perf-pdf-preview-contract.mjs';
import { inspectPdfPixels } from './native-perf-pdf-pixels.mjs';
import {
  checkOcrJourney,
  summarizeOcrJourneys,
  verifyPublicOcrInput,
  OCR_UNMEASURED,
} from './native-perf-ocr-contract.mjs';

export const PHASES = Object.freeze([
  'startup',
  'password-unlock',
  'workspace',
  'search-needle',
  'application-lock',
  'password-reunlock',
]);
export const MEDIA_PHASES = Object.freeze([
  ...PHASES.slice(0, 3),
  'attachment-list',
  'attachment-image-preview',
  'attachment-text-preview',
  ...PHASES.slice(3),
]);
export const unmeasuredFor = (media) => [
  'OCR',
  media ? 'PDF-preview' : 'attachment-preview',
  'system-sleep',
  'same-profile-warm-start',
  'other-platforms',
];
const expectedMedia = (image) => ({
  kind: image ? 'image' : 'text',
  decoded: image,
  textMatches: !image,
  width: image ? 500 : 0,
  height: image ? 200 : 0,
  visible: true,
  paintedFrames: 2,
});
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
export async function journeyBinaryPreflight(
  exe,
  media = false,
  pdfDiagnostic = false,
  pdfPreview = false,
  ocr = false,
) {
  const required = media
    ? ['--native-perf-media-prepare', 'windows-native-sdk-media-journey-requested']
    : [MARKER];
  if (pdfDiagnostic)
    required.push(
      'windows-native-sdk-pdf-diagnostic-requested',
      'windows-native-sdk-pdf-target-diagnostic-requested',
      'windows-native-sdk-pdf-component-diagnostic-requested',
      'windows-native-sdk-pdf-structure-diagnostic-requested',
    );
  if (pdfPreview)
    required.push(
      'windows-native-sdk-pdf-first-page-requested',
      'windows-native-sdk-pdf-cipher-binding-requested',
      'windows-native-sdk-pdf-frame-stability-requested',
    );
  if (ocr) required.push('windows-native-sdk-ocr-journey-requested');
  const found = new Set();
  const overlap = Math.max(...required.map((s) => s.length));
  let tail = '';
  for await (const chunk of createReadStream(exe)) {
    const text = tail + chunk.toString('latin1');
    for (const marker of required) if (text.includes(marker)) found.add(marker);
    if (found.size === required.length)
      return {
        present: true,
        marker: media ? required : MARKER,
        limitation: 'feature marker; not executable trust or signature verification',
      };
    tail = text.slice(-overlap);
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
export function checkJourney(value, owned, bound, fixture, media = false) {
  const phases = media ? MEDIA_PHASES : PHASES;
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
    value.scope !==
      (media ? 'windows-native-sdk-media-journey' : 'windows-native-sdk-ui-journey') ||
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
    JSON.stringify(value.unmeasured) !== JSON.stringify(unmeasuredFor(media)) ||
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
    value.phases.length !== phases.length
  )
    throw new Error('SDK UI journey proof rejected');
  for (let index = 0; index < phases.length; index++) {
    const phase = value.phases[index];
    const preview =
      media && ['attachment-image-preview', 'attachment-text-preview'].includes(phase.name);
    if (
      !exactKeys(phase, [
        'name',
        'success',
        'durationMs',
        'ipc',
        'inputTrust',
        'fromAtMs',
        'toAtMs',
        ...(preview ? ['mediaEndState'] : []),
      ]) ||
      phase.name !== phases[index] ||
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
    if (preview) {
      const image = phase.name === 'attachment-image-preview';
      const expected = expectedMedia(image);
      if (
        !exactKeys(phase.mediaEndState, Object.keys(expected)) ||
        Object.keys(expected).some((k) => phase.mediaEndState[k] !== expected[k]) ||
        !(phase.ipc.commands[image ? 'fs_read_file_as_data_url' : 'fs_read_file_as_text'] >= 1)
      )
        throw new Error('SDK decoded/painted public media end state rejected');
    }
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
export function summarizeMediaJourneys(samples, requestedSamples, integrity = true) {
  const complete =
    integrity &&
    samples.length === requestedSamples &&
    samples.every(
      (s) =>
        s.success === true &&
        s.cleanupIntegrity?.complete === true &&
        Array.isArray(s.phases) &&
        s.phases.length === MEDIA_PHASES.length &&
        s.phases.every(
          (p, i) =>
            p.name === MEDIA_PHASES[i] &&
            p.success === true &&
            Number.isFinite(p.durationMs) &&
            p.durationMs >= 0,
        ),
    );
  return MEDIA_PHASES.map((name) => {
    const values = complete
      ? samples
          .map((s) => s.phases.find((p) => p.name === name))
          .filter((p) => p?.success && Number.isFinite(p.durationMs) && p.durationMs >= 0)
          .map((p) => p.durationMs)
          .sort((a, b) => a - b)
      : [];
    const valid = values.length === requestedSamples;
    return {
      name,
      successfulSamples: valid ? values.length : 0,
      requestedSamples,
      medianMs: valid
        ? (values[Math.floor((values.length - 1) / 2)] +
            values[Math.ceil((values.length - 1) / 2)]) /
          2
        : null,
      p95Ms: valid ? values[Math.ceil(values.length * 0.95) - 1] : null,
    };
  });
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
  let owner, completion, launch, memorySampler;
  const env = { ...process.env };
  delete env.SOLOSOUL_REGISTRY_PUBKEY;
  try {
    const at = Date.now(),
      prepare = startChild(
        options.exe,
        [
          options.media ? '--native-perf-media-prepare' : '--native-perf-prepare',
          root,
          '--fixture',
          options.fixture,
        ],
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
    if (options.media) await verifyPreparedMediaFiles(owned, options.mediaManifest);
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
        options.pdfDiagnostic
          ? 'sdk-pdf-diagnostic'
          : options.pdfPreview
            ? 'sdk-pdf-preview'
            : options.ocr
              ? 'sdk-ocr'
              : options.media
                ? 'sdk-media'
                : 'sdk-input',
      ],
      env,
    );
    completion = launch.completion;
    if (!launch.child.pid) throw new Error('SDK UI application did not spawn');
    result.processId = launch.child.pid;
    owner = new OwnedProcess(launch.child, options.exe, launchedAt, owned.webview);
    if (options.memoryIntervalMs !== null)
      memorySampler = new MemorySampler({
        sample: () => owner.sample({ confirmExitedDescendants: true }),
        intervalMs: options.memoryIntervalMs,
        launchedAt,
        pid: launch.child.pid,
        exe: options.exe,
      }).start();
    const bound = checkBound(
      await waitOwned(root, 'native-perf-sdk-journey-bound.json', launch.child, shouldStop, 65000),
      owned,
      launch.child.pid,
      port,
    );
    result.bound = bound;
    result.beforeSnapshot = await (memorySampler
      ? memorySampler.checkpoint('before-input')
      : owner.sample());
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
    if (options.ocr) result.ocrInputBefore = await verifyPublicOcrInput(root);
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
    const proof = options.pdfDiagnostic
      ? checkPdfDiagnostic(result.nativeProof, owned, bound, options.manifest)
      : options.pdfPreview
        ? checkPdfFirstPage(result.nativeProof, owned, bound, options.manifest)
        : options.ocr
          ? checkOcrJourney(result.nativeProof, owned, bound, options.manifest)
          : checkJourney(result.nativeProof, owned, bound, options.manifest, options.media);
    if (options.pdfDiagnostic) {
      if (proof.pdfDiagnostic.schemaVersion !== 4)
        throw new Error('Current PDF diagnostic requires bounded structure evidence');
      const screenshot = proof.pdfDiagnostic.screenshot;
      const file = path.join(root, screenshot.fileName);
      const stat = await lstat(file);
      if (
        !stat.isFile() ||
        stat.isSymbolicLink() ||
        stat.size !== screenshot.bytes ||
        normalizeWindowsPath(await realpath(file)) !== normalizeWindowsPath(file) ||
        (await sha256(file)) !== screenshot.sha256
      )
        throw new Error('PDF screenshot bytes differ from native proof');
    }
    if (options.pdfPreview) {
      for (const screenshot of proof.pdfPreview.samples) {
        const file = path.join(root, screenshot.fileName),
          stat = await lstat(file);
        if (
          !stat.isFile() ||
          stat.isSymbolicLink() ||
          stat.size !== screenshot.bytes ||
          normalizeWindowsPath(await realpath(file)) !== normalizeWindowsPath(file) ||
          (await sha256(file)) !== screenshot.sha256
        )
          throw new Error('PDF first-page screenshot bytes differ from native proof');
        const pixels = inspectPdfPixels(await readFile(file));
        if (Object.keys(pixels).some((k) => pixels[k] !== screenshot[k]))
          throw new Error('Independent PDF pixel decoder differs from native proof');
      }
    }
    if (options.ocr) result.ocrInputAfter = await verifyPublicOcrInput(root, true);
    result.afterSnapshot = await (memorySampler
      ? memorySampler.checkpoint('after-journey')
      : owner.sample());
    if (memorySampler) result.memorySeries = await memorySampler.finish(bound.browserPid);
    const after = selectDiagnosticIdentities(owner, result.afterSnapshot);
    sameSdkIdentities(before, after);
    result.afterProcessDiagnostics = await processDiagnostics(after, owned);
    result.phases = options.pdfDiagnostic || options.pdfPreview ? [] : proof.phases;
    if (options.pdfDiagnostic) result.diagnosticOnly = true;
    result.ipcAll = proof.ipcAll;
    result.success = true;
  } catch (error) {
    result.error = safeError(error);
  } finally {
    if (memorySampler) {
      result.uiSuccess = result.success;
      result.memorySeries ??= await memorySampler.finish(result.bound?.browserPid);
      if (!result.memorySeries.complete) result.success = false;
    }
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
export async function main(args = process.argv.slice(2), mode = 'sdk-input') {
  if (
    !['sdk-input', 'sdk-media', 'sdk-pdf-diagnostic', 'sdk-pdf-preview', 'sdk-ocr'].includes(mode)
  )
    throw new Error('Unsupported SDK measurement mode');
  const pdfDiagnostic = mode === 'sdk-pdf-diagnostic';
  const pdfPreview = mode === 'sdk-pdf-preview';
  const ocr = mode === 'sdk-ocr';
  const media = mode === 'sdk-media' || pdfDiagnostic || pdfPreview || ocr;
  const parsed = parseJourneyArgs(args);
  if (parsed.help) {
    process.stdout.write(
      `Usage: node scripts/${pdfDiagnostic ? 'native-perf-pdf-diagnostic' : pdfPreview ? 'native-perf-pdf-preview' : ocr ? 'native-perf-ocr' : media ? 'native-perf-media' : 'native-perf-sdk-journey'}.mjs --exe ABS --fixture ABS --output NEW_ABS --samples N [--memory-interval-ms 1000..10000] (N >= 3)\n`,
    );
    return 0;
  }
  if (process.platform !== 'win32') throw new Error('SDK UI journey is Windows only');
  if (pdfDiagnostic && parsed.memoryIntervalMs !== null)
    throw new Error('PDF capability diagnostic forbids memory/performance sampling');
  const options = {
      ...(await validateInputs(parsed, { media })),
      media,
      pdfDiagnostic,
      pdfPreview,
      ocr,
    },
    preflight = await journeyBinaryPreflight(options.exe, media, pdfDiagnostic, pdfPreview, ocr),
    sourceBefore = await inventory(options.fixture);
  await mkdir(options.output);
  const report = {
    schemaVersion: 1,
    task: 'RF-312',
    scope: pdfDiagnostic
      ? 'windows-native-sdk-pdf-capability-diagnostic'
      : pdfPreview
        ? 'windows-native-sdk-public-pdf-first-page-performance'
        : ocr
          ? 'windows-native-sdk-public-first-ocr-performance'
          : media
            ? 'windows-native-sdk-media-performance'
            : 'windows-native-sdk-input-performance',
    ...(pdfDiagnostic ? { diagnosticOnly: true, performanceMetrics: null } : {}),
    startedAt: new Date().toISOString(),
    samplesRequested: options.samples,
    nodeVersion: process.version,
    osRelease: os.release(),
    exeSha256: await sha256(options.exe),
    preflight,
    fixture: options.manifest,
    ...(media ? { mediaFixture: options.mediaManifest } : {}),
    sourceBefore,
    samples: [],
    memoryIntervalMs: options.memoryIntervalMs,
    definitions: {
      startup:
        'native runtime configuration to public login form plus two frames and SDK document binding; excludes OS spawn and includes SDK overhead',
      actions:
        'native Instant around fixed browser Input actions, visible end state, two frames and same-frame SDK checks; includes SDK overhead',
      memory:
        options.memoryIntervalMs === null
          ? 'verified owned Windows working sets before input and after journey; no peak-memory claim'
          : 'nonoverlapping verified owned process-tree queries from launch through final UI checkpoint; actual intervals and collection windows recorded; sampled observed maximum is not a continuous peak',
      ipc: 'real Tauri fetch observer; names/counts only; invalid or changed prefix rejects sample',
      unmeasured: pdfDiagnostic
        ? PDF_UNMEASURED
        : pdfPreview
          ? PDF_PREVIEW_UNMEASURED
          : ocr
            ? OCR_UNMEASURED
            : unmeasuredFor(media),
      ...(ocr
        ? {
            firstOcr:
              'native Instant around genuine SDK Select-file, owned foreground native filename edit/Open button, and fixed public result in viewport after two frames and same-frame binding; picker timestamps separated; first-result latency includes modal close, IPC, queue, first model load/inference, rendering, SDK and bounded scrolling; not a pure inference benchmark',
          }
        : {}),
      ...(pdfPreview
        ? {
            pdfFirstPage:
              'native Instant before SDK preview pointer input to two public text-mask captures at least 250ms apart, with current main document/embed identity checked before and after each capture; includes SDK, decode and polling overhead; earlier image publication may contribute; excludes final image publication, close action, other pages and arbitrary PDFs',
          }
        : {}),
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
    report.summary = pdfDiagnostic
      ? []
      : (pdfPreview
          ? summarizePdfFirstPage
          : ocr
            ? summarizeOcrJourneys
            : media
              ? summarizeMediaJourneys
              : summarizeJourneys)(
          report.samples,
          options.samples,
          report.sourceUnchanged && !interrupted,
        );
    if (options.memoryIntervalMs !== null) {
      const accepted =
        report.sourceUnchanged &&
        !interrupted &&
        (!media ||
          (report.samples.length === options.samples && report.samples.every((s) => s.success)))
          ? report.samples.filter((sample) => sample.success && sample.memorySeries?.complete)
          : [];
      const values = accepted
        .map((sample) => sample.memorySeries.observedMaximumWorkingSetBytes)
        .sort((a, b) => a - b);
      report.memorySummary = {
        samplesRequested: options.samples,
        successfulSamples: values.length,
        medianObservedMaximumWorkingSetBytes: values.length
          ? (values[Math.floor((values.length - 1) / 2)] +
              values[Math.ceil((values.length - 1) / 2)]) /
            2
          : null,
        p95ObservedMaximumWorkingSetBytes: values.length
          ? values[Math.ceil(values.length * 0.95) - 1]
          : null,
      };
    }
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
