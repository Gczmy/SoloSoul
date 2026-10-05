import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import {
  checkOcrJourney,
  checkOcrPicker,
  summarizeOcrJourneys,
  verifyPublicOcrInput,
  OCR_ASSET_SHA,
  OCR_TEXT,
  OCR_UNMEASURED,
} from './native-perf-ocr-contract.mjs';
import { journeyBinaryPreflight } from './native-perf-sdk-journey.mjs';
const runId = 'a'.repeat(32),
  owned = {
    runId,
    root: 'C:/owned',
    fixture: { marker: { publicAssets: [{ fileName: 'ocr_test.png', sha256: OCR_ASSET_SHA }] } },
  };
const bound = {
  pid: 10,
  browserPid: 11,
  port: 9222,
  timeOriginMs: 1000,
  binding: { mainFrameId: 'MAIN', loaderId: 'LOADER' },
};
const fixture = { objectCount: 100 },
  trust = { pointer: 7, text: 1, untrusted: 0 };
function picker() {
  const c = (hwnd, parent, name, id) => ({
    hwnd,
    parent,
    class: name,
    id,
    visible: true,
    enabled: true,
    password: false,
  });
  return {
    schemaVersion: 1,
    scope: 'owned-ocr-native-file-picker',
    pid: 10,
    mainHwnd: 10,
    dialogHwnd: 20,
    directOwnerVerified: true,
    foregroundVerified: true,
    controls: [
      c(21, 20, 'ComboBoxEx32', 1148),
      c(22, 21, 'ComboBox', 1001),
      c(23, 22, 'Edit', 1001),
      c(24, 20, 'Button', 1),
    ],
    filenameEditHwnd: 23,
    openButtonHwnd: 24,
    inputMethod: 'WM_SETTEXT+readback+BM_CLICK',
    inputPathVerified: true,
    actionStartedAtMs: 1000,
    openedObservedAtMs: 1200,
    selectionSubmittedAtMs: 1400,
    closedObservedAtMs: 1500,
    closedVerified: true,
  };
}
function proof() {
  const result = {
    kind: 'fixed-public-image',
    text: 'Hello PP-OCRv6\nSoloSoul OCR\n1234567890',
    normalizedText: OCR_TEXT,
    visible: true,
    paintedFrames: 2,
    tier: 'small',
    firstScanInvokeCount: 1,
  };
  const ipc = { attempts: 1, commands: { ocr_scan_image: 1 }, reason: null };
  return {
    schemaVersion: 1,
    scope: 'windows-native-sdk-first-ocr',
    runId,
    root: owned.root,
    pid: 10,
    port: 9222,
    objectCount: 100,
    browserPid: 11,
    binding: structuredClone(bound.binding),
    timeOriginMs: 1000,
    inputMethod: 'SDK-CDP-Input+owned-Win32-picker',
    success: true,
    reason: null,
    failedStep: null,
    calls: [
      'Input.insertText',
      'Input.dispatchMouseEvent',
      'Runtime.evaluate',
      'Page.getFrameTree',
    ],
    phases: ['startup', 'password-unlock', 'workspace', 'ocr-page', 'first-ocr'].map((name, i) => ({
      name,
      success: true,
      durationMs: 20,
      ipc: i === 4 ? structuredClone(ipc) : { attempts: 0, commands: {}, reason: null },
      inputTrust: { ...trust },
      fromAtMs: 0,
      toAtMs: 1000,
      ...(i === 4 ? { ocrEndState: structuredClone(result) } : {}),
    })),
    lastProbe: {
      schemaVersion: 1,
      scope: 'windows-native-sdk-ui-probe',
      runId,
      step: 'ocrResultReady',
      outcome: 'ready',
      href: 'http://tauri.localhost/ocr',
      origin: 'http://tauri.localhost',
      timeOriginMs: 1000,
      atMs: 3000,
      rootPresent: true,
      frameCount: 0,
      target: null,
      inputTrust: { ...trust },
      observer: {
        schemaVersion: 1,
        scope: 'windows-native-tauri-invoke-observer',
        runId,
        valid: true,
        total: 1,
        observedCount: 1,
        invalidReasons: [],
        installedAtMs: 0,
        timeOriginMs: 1000,
        maxEvents: 16384,
        commands: [{ command: 'ocr_scan_image', atMs: 2000 }],
      },
      ocr: structuredClone(result),
    },
    ipcAll: ipc,
    elapsedMs: 3100,
    unmeasured: [...OCR_UNMEASURED],
    ocr: {
      schemaVersion: 1,
      scope: 'windows-native-sdk-public-first-ocr',
      resource: {
        relativePath: 'profile/Documents/ocr_test.png',
        bytes: 6274,
        sha256: OCR_ASSET_SHA,
        width: 500,
        height: 200,
        tier: 'small',
        preferencesAbsent: true,
      },
      firstPerProcess: true,
      picker: picker(),
      result,
      resultObservedAtMs: 3000,
      performanceMetrics: {
        pickerOpenedObservedMs: 200,
        pickerSelectionClosedMs: 300,
        firstOcrResultObservedMs: 1600,
        totalOcrObservedMs: 2000,
      },
      closedPickerVerified: true,
    },
  };
}
test('complete native picker and public three-line OCR proof accepts', () =>
  assert.equal(checkOcrJourney(proof(), owned, bound, fixture).ocr.firstPerProcess, true));
test('wrong HWND owner, arbitrary edit, hidden controls, cyclic trees and duplicate Open fail', () => {
  const edits = [
    (v) => v.pid++,
    (v) => (v.directOwnerVerified = false),
    (v) => (v.foregroundVerified = false),
    (v) => (v.controls[0].id = 1001),
    (v) => (v.controls[2].parent = 20),
    (v) => (v.controls[2].password = true),
    (v) => (v.controls[2].enabled = false),
    (v) => (v.controls[2].visible = false),
    (v) => (v.controls[1].parent = 23),
    (v) => v.controls.push({ ...v.controls[3], hwnd: 30 }),
    (v) => (v.filenameEditHwnd = 22),
    (v) => (v.inputMethod = 'direct-IPC'),
    (v) => (v.inputPathVerified = false),
    (v) => (v.closedVerified = false),
    (v) => (v.closedObservedAtMs = 1300),
    (v) => (v.controls[0].private = 'payload'),
  ];
  for (const edit of edits) {
    const v = picker();
    edit(v);
    assert.throws(() => checkOcrPicker(v, 10));
  }
});
test('document, IPC, result, tier and native time tampering fail closed', () => {
  const edits = [
    (v) => v.pid++,
    (v) => v.browserPid++,
    (v) => (v.binding.loaderId = 'REPLACED'),
    (v) => (v.inputMethod = 'synthetic-click'),
    (v) => (v.lastProbe.href = 'http://other.invalid/ocr'),
    (v) => (v.lastProbe.observer.valid = false),
    (v) => (v.lastProbe.observer.extra = 'private'),
    (v) => (v.ipcAll.commands.ocr_scan_image = 2),
    (v) => (v.lastProbe.ocr.text = 'Hello'),
    (v) => (v.ocr.result.text += 'unexpected'),
    (v) => (v.ocr.result.tier = 'medium'),
    (v) => (v.ocr.resource.sha256 = 'b'.repeat(64)),
    (v) => (v.ocr.resource.relativePath = '../outside.png'),
    (v) => (v.ocr.resource.preferencesAbsent = false),
    (v) => (v.ocr.firstPerProcess = false),
    (v) => (v.ocr.resultObservedAtMs = 1200),
    (v) => (v.ocr.performanceMetrics.firstOcrResultObservedMs = 1),
    (v) => (v.ocr.result.private = 'payload'),
    (v) => v.phases.pop(),
    (v) => v.calls.push('Runtime.callFunctionOn'),
    (v) => (v.lastProbe.inputTrust.untrusted = 1),
    (v) => (v.unmeasured = []),
  ];
  for (const edit of edits) {
    const v = proof();
    edit(v);
    assert.throws(() => checkOcrJourney(v, owned, bound, fixture));
  }
});
test('failed or incomplete full groups suppress all OCR metrics', () => {
  const samples = [1, 2, 3].map(() => ({
    success: true,
    cleanupIntegrity: { complete: true },
    nativeProof: proof(),
  }));
  const valid = summarizeOcrJourneys(samples, 3, true);
  assert.equal(valid[2].medianMs, 1600);
  assert.equal(valid[2].successfulSamples, 3);
  for (const group of [
    samples.slice(0, 2),
    [{ ...samples[0], success: false }, ...samples.slice(1)],
    [{ ...samples[0], cleanupIntegrity: { complete: false } }, ...samples.slice(1)],
  ])
    assert.ok(
      summarizeOcrJourneys(group, 3, true).every((v) => v.medianMs === null && v.p95Ms === null),
    );
  assert.ok(summarizeOcrJourneys(samples, 3, false).every((v) => v.medianMs === null));
});
test('public input bytes must match actual selected plaintext PNG; preserved original cannot overwrite', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'ss-ocr-public-'));
  const file = path.join(root, 'profile', 'Documents', 'ocr_test.png');
  try {
    await mkdir(path.dirname(file), { recursive: true });
    const bytes = await readFile(
      new URL('../crates/solosoul-core/tests/fixtures/ocr_test.png', import.meta.url),
    );
    await writeFile(file, bytes);
    assert.equal((await verifyPublicOcrInput(root, true)).sha256, OCR_ASSET_SHA);
    assert.deepEqual(await readFile(path.join(root, 'native-perf-ocr-input.png')), bytes);
    await assert.rejects(verifyPublicOcrInput(root, true));
    const bad = Buffer.from(bytes);
    bad[100] ^= 1;
    await writeFile(file, bad);
    await assert.rejects(verifyPublicOcrInput(root));
    await writeFile(file, Buffer.concat([Buffer.from('SOLC'), bytes]));
    await assert.rejects(verifyPublicOcrInput(root));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test('OCR binary preflight rejects previous fixed-PDF build marker alone', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'ss-ocr-marker-')),
    file = path.join(root, 'mock.exe');
  try {
    const base = '--native-perf-media-prepare windows-native-sdk-media-journey-requested';
    await writeFile(file, base);
    await assert.rejects(journeyBinaryPreflight(file, true, false, false, true));
    await writeFile(file, base + ' windows-native-sdk-ocr-journey-requested');
    assert.equal((await journeyBinaryPreflight(file, true, false, false, true)).present, true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
