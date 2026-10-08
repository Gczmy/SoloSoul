import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseOptions, verifyDevice, verifyInstrumentation, casesFor, run } from './android-native-regression.mjs';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('runner rejects ambiguous devices, missing options and duplicate options', () => {
  const args = ['--adb', '/sdk/adb', '--serial', 'emulator-5586', '--avd', 'SoloSoul_RF201', '--apk', 'app.apk', '--test-apk', 'test.apk', '--output', '/tmp/new', '--mode', 'supported'];
  assert.equal(parseOptions(args).serial, 'emulator-5586');
  assert.throws(() => parseOptions(args.slice(0, -2)));
  assert.throws(() => parseOptions([...args, '--serial', 'emulator-5554']));
  assert.throws(() => parseOptions(args.map(value => value === 'emulator-5586' ? 'phone' : value)));
  for (const status of ['offline', 'unauthorized']) assert.throws(() => verifyDevice(`emulator-5586 ${status}`, 'emulator-5586'));
  assert.throws(() => verifyDevice('emulator-5554 device', 'emulator-5586'));
  verifyDevice('List of devices attached\nemulator-5586 device product:test', 'emulator-5586');
});

test('only exact successful one-test native report is accepted', () => {
  const report = 'INSTRUMENTATION_STATUS: test=foo\nINSTRUMENTATION_STATUS_CODE: 1\nINSTRUMENTATION_STATUS: test=foo\nINSTRUMENTATION_STATUS_CODE: 0\nOK (1 test)\nINSTRUMENTATION_CODE: -1\n';
  verifyInstrumentation(report, 'foo');
  for (const invalid of [report.replace('CODE: 0', 'CODE: -3'), report.replace('CODE: 0', 'CODE: -2'), report.replace('OK (1 test)', 'OK (0 tests)'), report.replace('test=foo', 'test=bar').replace('test=foo', 'test=bar'), report.replace('INSTRUMENTATION_CODE: -1', ''), `${report}Process crashed`]) assert.throws(() => verifyInstrumentation(invalid, 'foo'));
});

test('supported and fallback lanes execute distinct glass assertions and same system-theme cases', () => {
  assert.equal(casesFor('supported').length, 13);
  assert.equal(casesFor('fallback').length, 11);
  for (const lane of ['supported', 'fallback']) {
    assert.ok(casesFor(lane).some(([type, method]) => type === 'AndroidCardSurfaceInstrumentedTest' && method === 'lightAndDarkCardsRenderWithoutOverflow'));
    assert.ok(casesFor(lane).some(([type, method]) => type === 'AndroidCardSurfaceInstrumentedTest' && method === 'sharedThemeLiquidRendersAndRecoversWithoutContinuousDrawing'));
    assert.ok(casesFor(lane).some(([type, method]) => type === 'AndroidThemeInstrumentedTest' && method === 'savedSchemesAndCustomAccentSurviveColdStartup'));
  }
  assert.ok(casesFor('fallback').some(([, name]) => name === 'unsupportedBlurReturnsFallbackWithoutOpeningNativeMenu'));
  assert.equal(casesFor('fallback').filter(([name]) => name === 'AndroidThemeInstrumentedTest').length, 4);
});

test('missing adb produces nonzero result and JUnit environment failure, never a zero-test pass', () => {
  const root = mkdtempSync(join(tmpdir(), 'solosoul-native-negative-'));
  try {
    const output = join(root, 'results');
    assert.equal(run({ adb: join(root, 'missing-adb'), serial: 'emulator-5586', avd: 'Dedicated', mode: 'supported', output }), 1);
    const report = JSON.parse(readFileSync(join(output, 'report.json')));
    assert.equal(report.passed, false);
    assert.match(readFileSync(join(output, 'junit.xml'), 'utf8'), /tests="1" failures="1" skipped="0"/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
