#!/usr/bin/env node
/** 完整六阶段行程上的独立认证诊断；不把带观测器的时延当成新的未插桩性能基线。 */
import { createReadStream } from 'node:fs';
import { lstat, realpath, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { main as journeyMain } from './native-perf-sdk-journey.mjs';
import { parseJourneyArgs } from './native-perf-memory.mjs';
import { safeError, newJson, normalizeWindowsPath, validateInputs } from './native-perf-run.mjs';
import { describeAuthAttribution } from './native-perf-auth-contract.mjs';
import { checkMaintenanceAdmission } from './native-perf-maintenance-contract.mjs';
export async function authBinaryPreflight(exe) {
  const markers = ['windows-native-auth-attribution', 'windows-native-maintenance-admission'].map(
    (name) => Buffer.from(name),
  );
  const seen = new Set();
  const maxLength = Math.max(...markers.map((marker) => marker.length));
  let tail = Buffer.alloc(0);
  for await (const part of createReadStream(exe)) {
    const chunk = Buffer.concat([tail, part]);
    for (const [index, marker] of markers.entries()) if (chunk.includes(marker)) seen.add(index);
    if (seen.size === markers.length) return true;
    tail = chunk.subarray(Math.max(0, chunk.length - maxLength + 1));
  }
  throw Error('Authentication and maintenance attribution require a current nondefault binary');
}
export async function runAuthenticationJourney(args) {
  return journeyMain(args, 'sdk-input');
}
export async function main(args = process.argv.slice(2)) {
  const options = parseJourneyArgs(args);
  if (options.help) {
    process.stdout.write(
      'Usage: node scripts/native-perf-auth-attribution.mjs --exe ABS --fixture ABS --output NEW_ABS --samples N (N >= 3)\n',
    );
    return 0;
  }
  if (options.memoryIntervalMs !== null)
    throw Error('Authentication diagnostic forbids concurrent memory sampling');
  const verified = await validateInputs(options);
  await authBinaryPreflight(verified.exe);
  const code = await runAuthenticationJourney(args);
  const report = JSON.parse(
    await readFile(path.join(options.output, 'native-perf-sdk-journey-results.json'), 'utf8'),
  );
  const result = {
    schemaVersion: 1,
    task: 'RF-312',
    scope: 'windows-native-auth-attribution-group',
    diagnosticOnly: true,
    performanceMetrics: null,
    samplesRequested: options.samples,
    success: false,
    samples: [],
    definitions: {
      response:
        'Only Tauri-Response status header; no response body. Header arrival does not prove invoke Promise completion.',
      frontend:
        'Same document performance clock for request, headers, actual await completion and state write.',
      backend:
        'Separate process-monotonic clock; inclusive nested stage durations are not summed or subtracted from frontend durations.',
      pairing:
        'Per-command ordinal pairing only when both command streams are serial and their counts agree. Otherwise concurrency is explicitly ambiguous.',
    },
  };
  for (const sample of report.samples) {
    const item = {
      index: sample.index,
      root: sample.root,
      journeySuccess: sample.success,
      success: false,
    };
    try {
      if (
        normalizeWindowsPath(sample.root) !==
        normalizeWindowsPath(
          path.join(options.output, 'sample-' + String(sample.index).padStart(3, '0')),
        )
      )
        throw Error('Diagnostic sample root rejected');
      const file = path.join(sample.root, 'native-perf-auth-attribution.json'),
        stat = await lstat(file);
      if (
        !stat.isFile() ||
        stat.isSymbolicLink() ||
        stat.size > 262144 ||
        normalizeWindowsPath(await realpath(file)) !== normalizeWindowsPath(file)
      )
        throw Error('Diagnostic artifact path or bounds rejected');
      const raw = JSON.parse(await readFile(file, 'utf8'));
      item.diagnostic = describeAuthAttribution(raw, sample.owned, sample.bound);
      const admissionFile = path.join(sample.root, 'native-perf-maintenance-admission.json'),
        admissionStat = await lstat(admissionFile);
      if (
        !admissionStat.isFile() ||
        admissionStat.isSymbolicLink() ||
        admissionStat.size > 4194304 ||
        normalizeWindowsPath(await realpath(admissionFile)) !== normalizeWindowsPath(admissionFile)
      )
        throw Error('Maintenance diagnostic artifact path or bounds rejected');
      item.maintenance = checkMaintenanceAdmission(
        JSON.parse(await readFile(admissionFile, 'utf8')),
        raw,
        sample.owned,
        sample.bound,
      );
      item.success =
        sample.success === true &&
        item.diagnostic.frontend.flows.length === 2 &&
        item.diagnostic.frontend.flows.every((f) => f.outcome === 'finished');
    } catch (error) {
      item.error = safeError(error);
    }
    result.samples.push(item);
  }
  result.success =
    code === 0 &&
    report.success === true &&
    result.samples.length === options.samples &&
    result.samples.every((s) => s.success);
  await newJson(path.join(options.output, 'native-perf-auth-attribution-results.json'), result);
  process.stdout.write(
    'RF-312 authentication attribution: ' +
      path.join(options.output, 'native-perf-auth-attribution-results.json') +
      '\n',
  );
  return result.success ? 0 : 1;
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
