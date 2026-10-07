/** RF-312：复用严格 owned 进程查询，串行采样，不接受未知进程或目录。 */
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { setTimeout as pause } from 'node:timers/promises';
import { parseArgs, safeError } from './native-perf-run.mjs';

export function parseJourneyArgs(args) {
  let intervalMs = null;
  const base = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i] !== '--memory-interval-ms') {
      base.push(args[i]);
      continue;
    }
    const value = args[++i];
    if (intervalMs !== null || !/^[1-9]\d*$/.test(value ?? ''))
      throw new Error('--memory-interval-ms must appear once with an integer from 1000 to 10000');
    intervalMs = Number(value);
    if (intervalMs < 1000 || intervalMs > 10000)
      throw new Error('--memory-interval-ms must be from 1000 to 10000');
  }
  const parsed = parseArgs(base);
  if (parsed.help && intervalMs !== null) throw new Error('--help must be used alone');
  return { ...parsed, memoryIntervalMs: intervalMs };
}

// OwnedProcess 在读数前后核验原 child 和 CIM 身份；这里校验报告完整性与全程身份一致性。
export function memoryReading(snapshot, expected, browserPid) {
  if (
    !snapshot ||
    snapshot.reason !== null ||
    !Number.isFinite(snapshot.workingSetBytes) ||
    snapshot.workingSetBytes < 0 ||
    !Number.isFinite(snapshot.collectionMs) ||
    snapshot.collectionMs < 0 ||
    !Array.isArray(snapshot.processes) ||
    snapshot.processes.length === 0 ||
    snapshot.processCount !== snapshot.processes.length
  )
    return { valid: false, reason: 'Owned process memory is unavailable or incomplete' };
  const exited = snapshot.exitedProcesses ?? [];
  if (
    !Array.isArray(exited) ||
    exited.some(
      (row) =>
        !row ||
        row.pid === expected.pid ||
        !Number.isSafeInteger(row.pid) ||
        row.pid < 1 ||
        !Number.isFinite(row.creationMs) ||
        !Number.isFinite(row.exitConfirmedAtMs) ||
        row.exitConfirmedAtMs < row.creationMs ||
        row.reason !== 'Process exited before memory collection; absence confirmed by fresh CIM' ||
        snapshot.processes.some((live) => live?.pid === row.pid),
    )
  )
    return {
      valid: false,
      reason: 'Exited descendants have no valid fresh-CIM absence confirmation',
    };
  const rows = snapshot.processes;
  if (rows.some((row) => !row || typeof row !== 'object' || typeof row.executableName !== 'string'))
    return { valid: false, reason: 'Owned process memory rows have invalid identities' };
  if (
    new Set(rows.map((row) => row.pid)).size !== rows.length ||
    rows.some(
      (row) =>
        !Number.isSafeInteger(row.pid) ||
        row.pid < 1 ||
        !Number.isFinite(row.creationMs) ||
        !Number.isFinite(row.workingSetBytes) ||
        row.workingSetBytes < 0 ||
        row.reason !== null,
    ) ||
    rows.reduce((sum, row) => sum + row.workingSetBytes, 0) !== snapshot.workingSetBytes
  )
    return { valid: false, reason: 'Owned process memory rows do not match their total' };
  const root = rows.find((row) => row.pid === expected.pid);
  if (
    !root ||
    root.executableName.toLowerCase() !== path.win32.basename(expected.exe).toLowerCase() ||
    Math.abs(root.creationMs - expected.launchedAt) > 10000 ||
    (expected.creationMs !== undefined && root.creationMs !== expected.creationMs)
  )
    return { valid: false, reason: 'Owned root identity changed or is missing' };
  const browsers = rows.filter((row) => row.executableName.toLowerCase() === 'msedgewebview2.exe');
  // WebView 尚未创建时，唯一已验证的 root 是完整的当前树；不把带未知 UDF 的树当作初始状态。
  if (browsers.length === 0 && rows.length === 1)
    return { valid: true, scope: 'root-only-before-webview', creationMs: root.creationMs };
  if (
    browsers.length === 0 ||
    snapshot.userDataDirectoryMatches !== true ||
    snapshot.browserDataFilesystem?.valid !== true ||
    snapshot.browserDataChecks?.length !== 1 ||
    snapshot.browserDataChecks[0].matchesExpected !== true ||
    !browsers.some((row) => row.pid === snapshot.browserDataChecks[0].pid) ||
    (browserPid !== undefined && snapshot.browserDataChecks[0].pid !== browserPid)
  )
    return { valid: false, reason: 'Owned WebView identity or exact UDF filesystem is unverified' };
  return { valid: true, scope: 'root-and-exact-udf-webview-tree', creationMs: root.creationMs };
}

export class MemorySampler {
  constructor({ sample, intervalMs, launchedAt, pid, exe, maxSamples = 512 }) {
    if (
      typeof sample !== 'function' ||
      !Number.isSafeInteger(intervalMs) ||
      intervalMs < 1000 ||
      intervalMs > 10000 ||
      !Number.isSafeInteger(maxSamples) ||
      maxSamples < 3 ||
      maxSamples > 512 ||
      !Number.isFinite(launchedAt) ||
      !Number.isSafeInteger(pid) ||
      pid < 1 ||
      !path.win32.isAbsolute(exe)
    )
      throw new Error('Invalid bounded memory sampler configuration');
    this.read = sample;
    this.intervalMs = intervalMs;
    this.expected = { launchedAt, pid, exe };
    this.maxSamples = maxSamples;
    this.samples = [];
    this.tail = Promise.resolve();
    this.abort = new AbortController();
    this.stopped = false;
    this.started = false;
    this.error = null;
  }
  start() {
    if (this.started || this.stopped) throw new Error('Memory sampler can only start once');
    this.started = true;
    this.loop = this.collect();
    return this;
  }
  checkpoint(kind) {
    if (
      !this.started ||
      this.stopped ||
      !['periodic', 'before-input', 'after-journey'].includes(kind)
    )
      return Promise.reject(new Error('Memory sampler is closed or checkpoint is invalid'));
    const next = this.tail.then(async () => {
      if (this.samples.length >= this.maxSamples) throw new Error('Memory sample bound exceeded');
      const startedAtMs = Date.now(),
        began = performance.now();
      let snapshot;
      try {
        snapshot = await this.read();
      } catch (error) {
        snapshot = { reason: safeError(error), workingSetBytes: null };
      }
      this.samples.push({
        index: this.samples.length + 1,
        kind,
        startedAtMs,
        finishedAtMs: Date.now(),
        collectionMs: performance.now() - began,
        snapshot,
      });
      return snapshot;
    });
    this.tail = next.catch((error) => {
      this.error ??= safeError(error);
    });
    return next;
  }
  async collect() {
    try {
      while (!this.stopped) {
        const began = performance.now();
        await this.checkpoint('periodic');
        if (this.stopped) break;
        // 目标为 start-to-start 间隔；查询较慢时不重叠，也不堆积补采样。
        await pause(Math.max(1, this.intervalMs - (performance.now() - began)), undefined, {
          signal: this.abort.signal,
        });
      }
    } catch (error) {
      if (error.name !== 'AbortError') this.error ??= safeError(error);
    }
  }
  async finish(browserPid) {
    if (!this.started) throw new Error('Memory sampler has not started');
    this.stopped = true;
    this.abort.abort();
    await this.loop;
    await this.tail;
    const finishedAtMs = Date.now(),
      readings = [];
    let creationMs,
      webviewSeen = false;
    for (const point of this.samples) {
      const reading = memoryReading(point.snapshot, { ...this.expected, creationMs }, browserPid);
      if (reading.valid) {
        creationMs ??= reading.creationMs;
        if (webviewSeen && reading.scope === 'root-only-before-webview') {
          reading.valid = false;
          reading.reason = 'Previously observed WebView tree disappeared';
        }
        webviewSeen ||= reading.scope === 'root-and-exact-udf-webview-tree';
      }
      if (
        reading.valid &&
        reading.scope === 'root-only-before-webview' &&
        point.kind !== 'periodic'
      ) {
        reading.valid = false;
        reading.reason = 'UI checkpoint has no verified WebView tree';
      }
      if (!Number.isFinite(point.collectionMs) || point.finishedAtMs < point.startedAtMs) {
        reading.valid = false;
        reading.reason = 'Memory collection clock is invalid';
      }
      readings.push({ ...point, validation: reading });
    }
    const values = readings
      .filter((point) => point.validation.valid)
      .map((point) => point.snapshot.workingSetBytes);
    const complete =
      !this.error &&
      Number.isSafeInteger(browserPid) &&
      browserPid > 0 &&
      readings.length >= 3 &&
      readings.every((point) => point.validation.valid) &&
      readings.some((point) => point.kind === 'before-input') &&
      readings.some((point) => point.kind === 'after-journey') &&
      webviewSeen;
    const intervals = readings
      .slice(1)
      .map((point, i) => point.startedAtMs - readings[i].startedAtMs);
    return {
      schemaVersion: 1,
      scope: 'windows-owned-process-tree-memory-series',
      complete,
      reason: complete
        ? null
        : (this.error ?? 'Memory coverage or identity validation is incomplete'),
      launchedAtMs: this.expected.launchedAt,
      finishedAtMs,
      requestedIntervalMs: this.intervalMs,
      confirmedExitedObservations: readings.reduce(
        (sum, point) => sum + (point.snapshot.exitedProcesses?.length ?? 0),
        0,
      ),
      sampleCount: readings.length,
      validSampleCount: values.length,
      observedMaximumWorkingSetBytes: complete ? Math.max(...values) : null,
      partialMaximumWorkingSetBytes: values.length ? Math.max(...values) : null,
      collectionTotalMs: readings.reduce((sum, point) => sum + point.collectionMs, 0),
      collectionMaximumMs: readings.length
        ? Math.max(...readings.map((point) => point.collectionMs))
        : null,
      actualMaximumStartIntervalMs: intervals.length ? Math.max(...intervals) : null,
      leadingUnobservedMs: readings.length
        ? readings[0].startedAtMs - this.expected.launchedAt
        : null,
      trailingUnobservedMs: readings.length ? finishedAtMs - readings.at(-1).finishedAtMs : null,
      samples: readings,
      definition:
        'Sampled sum of verified owned working sets, potentially counting shared pages more than once; collection windows are not atomic. Observed maximum is not a continuous OS peak. Fresh-CIM-confirmed exited descendants are recorded separately without a fabricated zero-memory reading. Collection duration measures query cost, not causal application slowdown.',
    };
  }
}
