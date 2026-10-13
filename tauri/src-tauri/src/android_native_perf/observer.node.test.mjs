import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFile } from 'node:fs/promises';
const script = await readFile(new URL('./observer.js', import.meta.url), 'utf8');
const id = 'a'.repeat(32);
function setup({
  fetchMethodRead = false,
  stickyBridge = false,
  readonlyIpcGlobal = false,
  readonlyBridge = false,
  lateInvoke = false,
  readonlyFetch = false,
  frames = 0,
  result,
  error,
} = {}) {
  let clock = 0;
  const bridgeCalls = [],
    fetchCalls = [];
  const invoke = () => {};
  const internals = {};
  if (!lateInvoke) Object.defineProperty(internals, 'invoke', { value: invoke });
  const bridge = {};
  const post = function (...args) {
    bridgeCalls.push({ receiver: this, args });
    if (error) throw error;
    return result;
  };
  Object.defineProperty(bridge, 'postMessage', {
    value: post,
    writable: !readonlyBridge,
    configurable: !readonlyBridge,
    enumerable: true,
  });
  const originalBridge = stickyBridge
    ? new Proxy(bridge, {
        defineProperty(target, key, descriptor) {
          if (key === 'postMessage') return true; // Android host reports flags but ignores shadowing.
          return Reflect.defineProperty(target, key, descriptor);
        },
      })
    : bridge;
  const window = {
    ipc: originalBridge,
    __TAURI_INTERNALS__: internals,
    location: { href: 'http://tauri.localhost/' },
  };
  if (readonlyIpcGlobal)
    Object.defineProperty(window, 'ipc', {
      value: originalBridge,
      writable: false,
      configurable: false,
    });
  Object.defineProperty(window, 'fetch', {
    value: function (...args) {
      const observedMethod = fetchMethodRead ? args[1]?.method : undefined;
      fetchCalls.push({ receiver: this, args, observedMethod });
      return result;
    },
    writable: !readonlyFetch,
    configurable: !readonlyFetch,
    enumerable: true,
  });
  window.top = window;
  const performance = { timeOrigin: 10000, now: () => ++clock };
  const context = vm.createContext({
    window,
    performance,
    document: { querySelectorAll: () => Array(frames) },
    URL,
    console,
  });
  vm.runInContext(script, context);
  return {
    window,
    originalBridge,
    context,
    bridgeCalls,
    fetchCalls,
    invoke,
    post,
    performance,
    observer: window.__SOLOSOUL_ANDROID_NATIVE_PERF__,
  };
}
const json = (value) => JSON.parse(JSON.stringify(value));
test('counts early bridge sends before binding, without changing readonly invoke or descriptor flags', () => {
  const s = setup(),
    descriptor = Object.getOwnPropertyDescriptor(s.window.ipc, 'postMessage');
  s.window.ipc.postMessage(
    '{"cmd":"login","payload":{"password":"never-record-me"},"__TAURI_INVOKE_KEY__":"secret"}',
  );
  const proof = json(s.observer.bind(id));
  assert.equal(proof.valid, true);
  assert.equal(proof.observedCount, 1);
  assert.deepEqual(Object.keys(proof.commands[0]).sort(), ['atMs', 'command', 'transport']);
  assert.equal(proof.commands[0].command, 'login');
  assert.ok(proof.commands[0].atMs < proof.atMs);
  assert.ok(
    !JSON.stringify(proof).includes('never-record-me') && !JSON.stringify(proof).includes('secret'),
  );
  assert.equal(s.window.__TAURI_INTERNALS__.invoke, s.invoke);
  assert.equal(
    Object.getOwnPropertyDescriptor(s.window.__TAURI_INTERNALS__, 'invoke').writable,
    false,
  );
  assert.equal(descriptor.writable, true);
  assert.equal(descriptor.configurable, true);
  assert.equal(descriptor.enumerable, true);
});
test('preserves bridge receiver, exact arguments, synchronous return and thrown error identity', () => {
  const result = { sentinel: true },
    s = setup({ result }),
    receiver = { unrelated: true },
    arg = { private: true };
  const message = '{"cmd":"vault_list_accounts","payload":{}}';
  assert.equal(s.window.ipc.postMessage.call(receiver, message, arg), result);
  assert.equal(s.bridgeCalls[0].receiver, receiver);
  assert.equal(s.bridgeCalls[0].args[0], message);
  assert.equal(s.bridgeCalls[0].args[1], arg);
  const error = new Error('private failure text'),
    failing = setup({ error });
  assert.throws(
    () => failing.window.ipc.postMessage(message),
    (actual) => actual === error,
  );
  assert.ok(!JSON.stringify(failing.observer.bind(id)).includes(error.message));
});
test('channel fetch counts headers only and preserves the original promise without reading bodies', () => {
  const promise = Promise.resolve('private-response'),
    s = setup({ result: promise });
  const options = {
    method: 'POST',
    get body() {
      throw Error('body must not be read');
    },
    get headers() {
      throw Error('headers must not be read');
    },
  };
  const url = 'http://ipc.localhost/plugin%3A__TAURI_CHANNEL__%7Cfetch';
  assert.equal(s.window.fetch(url, options), promise);
  assert.equal(s.fetchCalls[0].args[0], url);
  assert.equal(s.fetchCalls[0].args[1], options);
  const proof = json(s.observer.bind(id));
  assert.equal(proof.valid, true);
  assert.equal(proof.commands[0].command, 'plugin:__TAURI_CHANNEL__|fetch');
  assert.equal(proof.commands[0].transport, 'android-channel-fetch');
});
test('rejects unsupported envelopes and overflow while forwarding every original call', () => {
  const s = setup();
  s.window.ipc.postMessage('{"payload":{"secret":"no"},"cmd":"login"}');
  assert.equal(s.observer.bind(id).valid, false);
  assert.equal(s.bridgeCalls.length, 1);
  const many = setup();
  for (let i = 0; i < 16385; i++) many.window.ipc.postMessage('{"cmd":"login"}');
  const proof = json(many.observer.bind(id));
  assert.equal(proof.valid, false);
  assert.equal(proof.observedCount, 16385);
  assert.equal(proof.commands.length, 16384);
  assert.equal(many.bridgeCalls.length, 16385);
  assert.ok(proof.invalidReasons.includes('event-overflow'));
});
test('unmodifiable transports are never forced writable or accepted as complete', () => {
  const s = setup({ readonlyIpcGlobal: true, readonlyBridge: true, readonlyFetch: true });
  assert.equal(s.window.ipc.postMessage, s.post);
  assert.equal(Object.getOwnPropertyDescriptor(s.window.ipc, 'postMessage').writable, false);
  const proof = json(s.observer.bind(id));
  assert.equal(proof.valid, false);
  assert.ok(
    proof.invalidReasons.includes('bridge-wrapper-unavailable') &&
      proof.invalidReasons.includes('fetch-wrapper-unavailable'),
  );
});
test('detects wrapper/frame/clock replacement, prevents rebinding, and returns independent snapshots', () => {
  const s = setup();
  s.observer.bind(id);
  s.window.ipc.postMessage('{"cmd":"login"}');
  const first = s.observer.snapshot();
  first.commands[0].command = 'mutated';
  assert.equal(s.observer.snapshot().commands[0].command, 'login');
  assert.throws(() => s.observer.bind('b'.repeat(32)), /another measurement/);
  assert.throws(() => s.observer.bind('short'), /Invalid measurement/);
  s.performance.timeOrigin = 20000;
  s.window.ipc.postMessage = () => {};
  assert.equal(s.observer.snapshot().valid, false);
  assert.equal(setup({ frames: 1 }).observer.bind(id).valid, false);
});
test('duplicate initialization invalidates the old observer without double wrapping', () => {
  const s = setup(),
    original = s.window.ipc.postMessage;
  vm.runInContext(script, s.context);
  assert.equal(s.window.ipc.postMessage, original);
  assert.ok(json(s.observer.bind(id)).invalidReasons.includes('duplicate-installation'));
});

test('accepts actual Tauri order: observer precedes the readonly core invoke definition', () => {
  const s = setup({ lateInvoke: true });
  Object.defineProperty(s.window.__TAURI_INTERNALS__, 'invoke', { value: s.invoke });
  s.window.ipc.postMessage('{"cmd":"get_ui_preferences","payload":{}}');
  const proof = json(s.observer.bind(id));
  assert.equal(proof.valid, true);
  assert.equal(proof.observedCount, 1);
  assert.equal(s.window.__TAURI_INTERNALS__.invoke, s.invoke);
  assert.equal(
    Object.getOwnPropertyDescriptor(s.window.__TAURI_INTERNALS__, 'invoke').configurable,
    false,
  );
});

test('invalid channel command is redacted without reading bodies or changing forwarding', () => {
  const s = setup();
  const options = {
    method: 'POST',
    get body() {
      throw new Error('body must not be read');
    },
  };
  s.window.fetch('http://ipc.localhost/private%20unrecognized%20value', options);
  const proof = json(s.observer.bind(id));
  assert.equal(proof.valid, false);
  assert.equal(s.fetchCalls.length, 1);
  assert.equal(proof.commands[0].command, null);
  assert.ok(!JSON.stringify(proof).includes('private unrecognized value'));
});
test('public invalidation never echoes supplied private values into diagnostics', () => {
  const s = setup();
  s.observer.invalidate({ private: 'never-record-external-value' });
  const proof = json(s.observer.bind(id));
  assert.equal(proof.valid, false);
  assert.ok(proof.invalidReasons.every((reason) => typeof reason === 'string'));
  assert.ok(!JSON.stringify(proof).includes('never-record-external-value'));
});

test('preserves method getter semantics on unrelated fetch and refuses unobservable IPC methods', () => {
  const ordinary = setup({ fetchMethodRead: true });
  let reads = 0;
  const options = {
    get method() {
      reads++;
      return 'POST';
    },
  };
  ordinary.window.fetch('https://public.invalid/resource', options);
  assert.equal(reads, 1);
  assert.equal(ordinary.observer.bind(id).valid, true);
  const ipc = setup({ fetchMethodRead: true }),
    error = new Error('original-fetch-method-failure');
  let ipcReads = 0;
  const ipcOptions = {
    get method() {
      ipcReads++;
      throw error;
    },
  };
  assert.throws(
    () => ipc.window.fetch('http://ipc.localhost/plugin:__TAURI_CHANNEL__|fetch', ipcOptions),
    (e) => e === error,
  );
  assert.equal(ipcReads, 1);
  assert.equal(ipc.observer.bind(id).valid, false);
});

test('captures sticky Java host sends without mutating the original host method or native receiver', () => {
  const s = setup({ stickyBridge: true }),
    post = s.originalBridge.postMessage;
  s.window.ipc.postMessage('{"cmd":"ui_get_preferences","payload":{}}');
  const proof = json(s.observer.bind(id));
  assert.equal(proof.valid, true);
  assert.equal(proof.observedCount, 1);
  assert.equal(s.bridgeCalls.length, 1);
  assert.equal(s.bridgeCalls[0].receiver, s.originalBridge);
  assert.equal(s.originalBridge.postMessage, post);
});
