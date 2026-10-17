import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFile } from 'node:fs/promises';

const query = await readFile(new URL('./snapshot.js', import.meta.url), 'utf8');
const original = await readFile(process.env.RF312_BASE_OBSERVER_SOURCE ?? new URL('../../../tauri/src-tauri/src/android_native_perf/observer.js', import.meta.url), 'utf8');
const id = 'a'.repeat(32);
function setup() {
  let clock = 0;
  const calls = [];
  const bridge = {postMessage(...args) { calls.push({receiver: this, args}); }};
  const internals = {};
  Object.defineProperty(internals, 'secretKey', {get() { throw new Error('secret getter accessed'); }});
  const document = {documentElement: {dataset: {platform: 'android'}}, querySelectorAll: () => []};
  const window = {ipc: bridge, fetch() {}, __TAURI_INTERNALS__: internals, location: {href: 'http://tauri.localhost/'}};
  window.top = window;
  const context = vm.createContext({window,document,URL,performance: {timeOrigin: 1000,now: () => ++clock}});
  vm.runInContext(original, context);
  const invoke = () => {};
  const protect = () => Object.defineProperty(internals, 'invoke', {value: invoke});
  const capture = first => vm.runInContext(query.replaceAll('__RUN_ID__', id).replaceAll('__BIND_FIRST__', String(first)), context);
  return {window, document, bridge, calls, protect, capture, internals, invoke};
}

test('not-ready query preserves the original observer until invoke is readonly', () => {
  const state = setup();
  const admission = JSON.parse(state.capture(true));
  assert.equal(admission.invokeReadonly, false);
  assert.equal(admission.observer, null);
  state.protect();
  const wire = '{"cmd":"ui_get_preferences","payload":"SYNTHETIC_SECRET"}';
  state.window.ipc.postMessage(wire);
  const first = JSON.parse(state.capture(true));
  assert.equal(first.observer.valid, true);
  assert.equal(first.observer.observedCount, 1);
  assert.equal(first.observer.runId, id);
  assert.equal(state.calls[0].receiver, state.bridge);
  assert.equal(state.calls[0].args[0], wire);
  assert.equal(JSON.stringify(first).includes('SYNTHETIC_SECRET'), false);
  assert.deepEqual(Object.keys(first.observer.commands[0]).sort(), ['atMs','command','transport']);
  assert.equal(Object.getOwnPropertyDescriptor(state.internals, 'invoke').value, state.invoke);
});

test('later snapshots preserve the bound document and complete event prefix', () => {
  const state = setup(); state.protect();
  state.window.ipc.postMessage('{"cmd":"ui_get_preferences"}');
  const first = JSON.parse(state.capture(true));
  state.window.ipc.postMessage('{"cmd":"login","payload":"SYNTHETIC_SECRET"}');
  const last = JSON.parse(state.capture(false));
  assert.equal(last.observer.observedCount, 2);
  assert.deepEqual(last.observer.commands.slice(0, 1), first.observer.commands);
  assert.equal(last.observer.timeOriginMs, first.observer.timeOriginMs);
  assert.equal(last.observer.valid, true);
});

test('extra frames and missing feature decline before invoking the observer', () => {
  const state = setup(); state.protect();
  state.document.querySelectorAll = () => [{}];
  const framed = JSON.parse(state.capture(true));
  assert.equal(framed.frames, 1);
  assert.equal(framed.observer, null);
  const plain = vm.createContext({window: {__TAURI_INTERNALS__: {}}, document: {documentElement: {dataset: {platform: 'android'}},querySelectorAll: () => []}});
  const absent = JSON.parse(vm.runInContext(query.replaceAll('__RUN_ID__', id).replaceAll('__BIND_FIRST__', 'true'), plain));
  assert.equal(absent.present, false);
  assert.equal(absent.observer, null);
});

test('replacement of the bridge remains an invalid observation', () => {
  const state = setup(); state.protect();
  state.window.ipc.postMessage('{"cmd":"ui_get_preferences"}');
  state.capture(true);
  state.window.ipc = state.bridge;
  const last = JSON.parse(state.capture(false));
  assert.equal(last.observer.valid, false);
  assert.ok(last.observer.invalidReasons.includes('bridge-wrapper-replaced'));
});
