import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const script = fs.readFileSync(new URL('./observer.js', import.meta.url), 'utf8');
const runId = '1234567890abcdef1234567890abcdef';

function harness({ id = runId, fetchError = null } = {}) {
  const calls = [];
  const warnings = [];
  const response = Promise.resolve({ ok: true });
  const originalFetch = function (...args) {
    calls.push({ args, receiver: this });
    if (fetchError) throw fetchError;
    return response;
  };
  const context = {
    URL,
    Reflect,
    Object,
    Set,
    String,
    document: { querySelectorAll: () => [] },
    performance: {
      timeOrigin: 1000,
      now: (() => {
        let n = 0;
        return () => ++n;
      })(),
    },
    console: {
      warn: function (...args) {
        warnings.push({ args, receiver: this });
      },
    },
    window: {
      fetch: originalFetch,
      location: { href: 'http://tauri.localhost/' },
      __SOLOSOUL_NATIVE_PERF_RUN_ID__: id,
    },
  };
  context.window.top = context.window;
  vm.createContext(context);
  vm.runInContext(script, context);
  const snapshot = () =>
    JSON.parse(JSON.stringify(context.window.__SOLOSOUL_NATIVE_PERF__.snapshot()));
  return { context, calls, warnings, response, snapshot };
}

test('counts application, plugin and channel invokes, excludes ordinary fetch and never reads body', () => {
  const h = harness();
  const options = { method: 'POST' };
  Object.defineProperty(options, 'body', {
    get() {
      throw new Error('payload was read');
    },
  });
  assert.equal(h.context.window.fetch('http://ipc.localhost/auth_unlock', options), h.response);
  h.context.window.fetch('http://ipc.localhost/plugin%3Aos%7Cplatform', { method: 'post' });
  h.context.window.fetch({
    url: 'http://ipc.localhost/plugin%3A__TAURI_CHANNEL__%7Cfetch',
    method: 'POST',
  });
  h.context.window.fetch('http://tauri.localhost/assets/app.js');
  h.context.window.fetch('http://ipc.localhost.fake/auth_unlock', { method: 'POST' });
  h.context.window.fetch('http://ipc.localhost/ignore_get');
  const result = h.snapshot();
  assert.equal(result.valid, true);
  assert.equal(result.total, 3);
  assert.equal(result.runId, runId);
  assert.deepEqual(
    result.commands.map((entry) => entry.command),
    ['auth_unlock', 'plugin:os|platform', 'plugin:__TAURI_CHANNEL__|fetch'],
  );
  assert.deepEqual(Object.keys(result.commands[0]), ['command', 'atMs']);
  assert.equal(h.calls[0].args[1], options);
  assert.equal(h.calls[0].receiver, h.context.window);
});

test('forwards fetch errors and return values unchanged', () => {
  const error = new Error('fetch failure');
  const h = harness({ fetchError: error });
  assert.throws(
    () => h.context.window.fetch('http://ipc.localhost/get_accounts', { method: 'POST' }),
    (thrown) => thrown === error,
  );
  assert.equal(h.snapshot().observedCount, 1);
});

test('default transport fallback makes the whole total unavailable without suppressing warnings', () => {
  const h = harness();
  h.context.window.fetch('http://ipc.localhost/auth_unlock', { method: 'POST' });
  const warning =
    'IPC custom protocol failed, Tauri will now use the postMessage interface instead';
  const detail = new Error('CSP');
  h.context.console.warn(warning, detail);
  h.context.console.warn('ordinary warning');
  const result = h.snapshot();
  assert.equal(result.total, null);
  assert.equal(result.observedCount, 1);
  assert.deepEqual(result.invalidReasons, ['transport-fallback']);
  assert.equal(h.warnings[0].args[1], detail);
  assert.equal(h.warnings[0].receiver, h.context.console);
  assert.equal(h.warnings.length, 2);
});

test('overflow retains a bounded command list and invalidates the total', () => {
  const h = harness();
  const limit = h.snapshot().maxEvents;
  for (let i = 0; i <= limit; i += 1)
    h.context.window.fetch('http://ipc.localhost/get_accounts', { method: 'POST' });
  const result = h.snapshot();
  assert.equal(result.commands.length, limit);
  assert.equal(result.observedCount, limit + 1);
  assert.equal(result.total, null);
  assert.ok(result.invalidReasons.includes('overflow'));
});

test('fetch and warn replacement are sticky failures even after restoration', () => {
  const h = harness();
  const wrappedFetch = h.context.window.fetch;
  const wrappedWarn = h.context.console.warn;
  h.context.window.fetch = () => Promise.resolve();
  h.context.console.warn = () => {};
  assert.equal(h.snapshot().valid, false);
  h.context.window.fetch = wrappedFetch;
  h.context.console.warn = wrappedWarn;
  const result = h.snapshot();
  assert.equal(result.total, null);
  assert.ok(result.invalidReasons.includes('fetch-wrapper-replaced'));
  assert.ok(result.invalidReasons.includes('warn-wrapper-replaced'));
});

test('invalid origin, missing run identity and duplicate installation fail closed', () => {
  const missing = harness({ id: null });
  assert.equal(missing.snapshot().total, null);
  const h = harness();
  h.context.window.fetch('https://ipc.localhost/get_accounts', { method: 'POST' });
  assert.ok(h.snapshot().invalidReasons.includes('unsupported-ipc-origin'));
  const duplicate = harness();
  vm.runInContext(script, duplicate.context);
  assert.ok(duplicate.snapshot().invalidReasons.includes('duplicate-installation'));
});

test('snapshot exposes copies rather than mutable counter internals', () => {
  const h = harness();
  h.context.window.fetch('http://ipc.localhost/get_accounts', { method: 'POST' });
  const raw = h.context.window.__SOLOSOUL_NATIVE_PERF__.snapshot();
  raw.commands[0].command = 'changed';
  raw.commands.push({ command: 'extra', atMs: 0 });
  assert.equal(h.snapshot().total, 1);
  assert.equal(h.snapshot().commands[0].command, 'get_accounts');
});

test('malformed IPC command cannot silently disappear from a supposedly complete count', () => {
  const h = harness();
  h.context.window.fetch('http://ipc.localhost/%GG', { method: 'POST' });
  assert.equal(h.snapshot().total, null);
  assert.ok(h.snapshot().invalidReasons.includes('fetch-observation-error'));
});

test('document navigation and additional frames invalidate total without recording navigation paths', () => {
  const changed = harness();
  changed.context.performance.timeOrigin += 1;
  assert.ok(changed.snapshot().invalidReasons.includes('document-time-origin-changed'));
  const child = harness();
  child.context.window.top = {};
  assert.ok(child.snapshot().invalidReasons.includes('non-main-frame'));
  const frames = harness();
  frames.context.document.querySelectorAll = () => [{}];
  assert.ok(frames.snapshot().invalidReasons.includes('additional-frames'));
  assert.equal(frames.snapshot().total, null);
  assert.equal('href' in frames.snapshot(), false);
});
