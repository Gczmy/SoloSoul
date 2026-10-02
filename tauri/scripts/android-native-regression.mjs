#!/usr/bin/env node
/** RF-310：显式设备、逐方法原生进程、严格拒绝跳过/零测试/崩溃。 */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function parseOptions(args) {
  const result = {};
  const keys = ['adb', 'serial', 'avd', 'apk', 'test-apk', 'output', 'mode'];
  for (let i = 0; i < args.length; i += 2) {
    const key = args[i]?.slice(2);
    if (!args[i]?.startsWith('--') || !keys.includes(key) || result[key] || !args[i + 1] || args[i + 1].startsWith('--')) throw new Error('Invalid or duplicate argument');
    result[key] = args[i + 1];
  }
  if (keys.some(key => !result[key])) throw new Error(`Required options: ${keys.join(', ')}`);
  if (!['supported', 'fallback'].includes(result.mode)) throw new Error('Invalid glass mode');
  if (!/^emulator-\d+$/.test(result.serial)) throw new Error('Only explicitly named dedicated emulators are allowed');
  if (!/^[A-Za-z0-9_-]+$/.test(result.avd)) throw new Error('Invalid AVD name');
  return result;
}

export function verifyDevice(text, serial) {
  const row = text.split('\n').map(line => line.trim().split(/\s+/)).find(parts => parts[0] === serial);
  if (!row || row[1] !== 'device') throw new Error(`Device ${serial} missing, offline or unauthorized`);
}

export function verifyInstrumentation(text, method) {
  const statuses = [...text.matchAll(/^INSTRUMENTATION_STATUS_CODE:\s*(-?\d+)\s*$/gm)].map(match => Number(match[1]));
  if (statuses.some(code => code < 0)) throw new Error('Native test failed or skipped');
  if (statuses.filter(code => code === 0).length !== 1 || statuses.filter(code => code === 1).length !== 1 ||
      !text.includes(`INSTRUMENTATION_STATUS: test=${method}\n`) ||
      !/^INSTRUMENTATION_CODE:\s*-1\s*$/m.test(text) || !/OK \(1 test\)/.test(text) ||
      /INSTRUMENTATION_FAILED|Process crashed|FAILURES!!!/.test(text)) throw new Error('Missing exact one-test successful native report');
}

export function casesFor(mode) {
  const glass = 'AndroidGlassInstrumentedTest';
  return [
    [glass, 'verifiesRequestedEnvironment'],
    ...(mode === 'supported' ? [
      [glass, 'nativeMenuSelectsExactlyOnceAndPauseCancels'],
      [glass, 'cancelledRequestCannotReopenAndBackCancelDoesNotNavigate'],
      [glass, 'systemDisablingBlurKeepsExistingMenuReadable'],
    ] : [[glass, 'unsupportedBlurReturnsFallbackWithoutOpeningNativeMenu']]),
    ['AndroidCardSurfaceInstrumentedTest', 'lightAndDarkCardsRenderWithoutOverflow'],
    ['AndroidThemeInstrumentedTest', 'systemThemeFollowsLightDarkLight'],
    ['AndroidThemeInstrumentedTest', 'explicitLightSurvivesSystemChanges'],
    ['AndroidThemeInstrumentedTest', 'explicitDarkSurvivesSystemChanges'],
    ['ResourceInstallerInstrumentedTest', 'packagedResourcesInstallAndSecondStartWritesNothing'],
    ['ResourcePreparationInstrumentedTest', 'slowInstallDoesNotBlockFramesAndRecreationDoesNotDuplicateIt'],
    ['ResourcePreparationInstrumentedTest', 'errorIsVisibleAndRetryUnblocksRealConsumers'],
  ];
}

const xml = text => String(text).replace(/[<>&"']/g, c => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&apos;' })[c]);

export function run(options) {
  const output = resolve(options.output);
  mkdirSync(output, { recursive: false }); // 防止覆盖先前验收结果。
  const report = { serial: options.serial, avd: options.avd, mode: options.mode, startedAt: new Date().toISOString(), tests: [], passed: false };
  const adb = (args, settings = {}) => {
    const result = spawnSync(options.adb, ['-s', options.serial, ...args], { encoding: 'utf8', timeout: 240000, maxBuffer: 32 * 1024 * 1024, ...settings });
    if (result.error || result.signal || result.status !== 0) {
      const error = new Error(`adb ${args[0]} failed: ${result.error?.message ?? result.stderr ?? result.signal}`);
      error.output = result.stdout;
      throw error;
    }
    return result.stdout;
  };
  let previousBlur;
  let selected = false;
  try {
    verifyDevice(adb(['devices', '-l']), options.serial);
    if (adb(['emu', 'avd', 'name']).split('\n')[0].trim() !== options.avd) throw new Error('Dedicated AVD identity mismatch');
    selected = true;
    report.api = Number(adb(['shell', 'getprop', 'ro.build.version.sdk']).trim());
    report.abi = adb(['shell', 'getprop', 'ro.product.cpu.abi']).trim();
    if (report.api < 31 || !['arm64-v8a', 'x86_64'].includes(report.abi)) throw new Error('Requires API >=31 and ARM64/x86_64');
    report.artifacts = ['apk', 'test-apk'].map(key => ({ path: resolve(options[key]), sha256: createHash('sha256').update(readFileSync(options[key])).digest('hex') }));
    previousBlur = adb(['shell', 'settings', 'get', 'global', 'disable_window_blurs']).trim();
    adb(['shell', 'settings', 'put', 'global', 'disable_window_blurs', options.mode === 'fallback' ? '1' : '0']);
    adb(['shell', 'am', 'force-stop', 'com.solosoul.app']);
    adb(['install', '-r', resolve(options.apk)]);
    adb(['install', '-r', resolve(options['test-apk'])]);
    for (const [name, method] of casesFor(options.mode)) {
      const test = { class: name, method, passed: false };
      report.tests.push(test);
      const started = Date.now();
      try {
        adb(['shell', 'am', 'force-stop', 'com.solosoul.app']);
        const text = adb(['shell', 'am', 'instrument', '-w', '-r', '-e', 'waitForActivitiesToComplete', 'false', '-e', 'glassMode', options.mode, '-e', 'class', `com.solosoul.app.${name}#${method}`, 'com.solosoul.app.test/androidx.test.runner.AndroidJUnitRunner']);
        writeFileSync(`${output}/${method}.log`, text);
        verifyInstrumentation(text.replaceAll('\r\n', '\n'), method);
        test.passed = true;
      } catch (error) {
        test.error = error.message;
        if (error.output) writeFileSync(`${output}/${method}.log`, error.output);
        try { writeFileSync(`${output}/${method}-failure.png`, adb(['exec-out', 'screencap', '-p'], { encoding: null })); } catch (captureError) { test.screenshotError = captureError.message; }
      } finally { test.seconds = (Date.now() - started) / 1000; }
      if (!test.passed) throw new Error(`${name}#${method}: ${test.error}`);
    }
    report.passed = true;
  } catch (error) { report.error = error.message; }
  finally {
    if (selected) {
      try {
        adb(['pull', '/sdcard/Android/data/com.solosoul.app/files', `${output}/device-files`]);
      } catch (error) { report.cleanupError = error.message; report.passed = false; }
      try { adb(['shell', 'am', 'force-stop', 'com.solosoul.app']); }
      catch (error) { report.cleanupError = error.message; report.passed = false; }
      if (previousBlur !== undefined) {
        try { adb(['shell', 'settings', previousBlur === 'null' ? 'delete' : 'put', 'global', 'disable_window_blurs', ...(previousBlur === 'null' ? [] : [previousBlur])]); }
        catch (error) { report.restoreError = error.message; report.passed = false; }
      }
    }
    report.count = report.tests.length;
    report.successes = report.tests.filter(test => test.passed).length;
    report.finishedAt = new Date().toISOString();
    // 设备缺失、初始化或收尾失败也必须成为可见的 JUnit 失败。
    const entries = [...report.tests];
    if (!report.passed && entries.every(test => test.passed)) entries.push({ class: 'DeviceRunner', method: 'environment', error: report.error ?? report.cleanupError ?? report.restoreError });
    writeFileSync(`${output}/report.json`, `${JSON.stringify(report, null, 2)}\n`);
    writeFileSync(`${output}/junit.xml`, `<testsuite name="android-${xml(options.mode)}" tests="${entries.length}" failures="${entries.filter(test => !test.passed).length}" skipped="0">${entries.map(test => `<testcase classname="${xml(test.class)}" name="${xml(test.method)}" time="${test.seconds ?? 0}">${test.passed ? '' : `<failure message="${xml(test.error)}"/>`}</testcase>`).join('')}</testsuite>\n`);
  }
  console.log(JSON.stringify(report, null, 2));
  return report.passed ? 0 : 1;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try { process.exitCode = run(parseOptions(process.argv.slice(2))); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
