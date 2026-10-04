import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const rust = fs.readFileSync(
  new URL('../src-tauri/src/native_perf/sdk_cdp.rs', import.meta.url),
  'utf8',
);
const script = rust.match(/const READINESS_EXPRESSION: &str = r#"([\s\S]*?)"#;/)?.[1];
assert.ok(script, 'actual bounded native readiness script exists');
const runId = '1234567890abcdef1234567890abcdef';

function harness({ handedOff = false, state = 'loading', reject = false } = {}) {
  const listeners = new Map();
  const timers = new Map();
  const calls = [];
  const status = { handedOff, state };
  let nextTimer = 0;
  const context = {
    Promise,
    performance: {
      getEntriesByName(name, type) {
        assert.equal(name, 'solosoul:startup-dismissed');
        assert.equal(type, 'mark');
        return status.handedOff ? [{}] : [];
      },
    },
    setTimeout(callback, delay) {
      assert.equal(delay, 8000);
      timers.set(++nextTimer, callback);
      return nextTimer;
    },
    clearTimeout(id) {
      timers.delete(id);
    },
    window: {
      __SOLOSOUL_NATIVE_PERF_RUN_ID__: runId,
      __SOLOSOUL_STARTUP__: { diagnostic: () => ({ state: status.state }) },
      __TAURI_INTERNALS__: {
        invoke(command, args) {
          calls.push(JSON.parse(JSON.stringify({ command, args })));
          return reject ? Promise.reject(new Error('private-transport-detail')) : Promise.resolve();
        },
      },
      addEventListener(name, callback) {
        listeners.set(name, callback);
      },
      removeEventListener(name, callback) {
        assert.equal(listeners.get(name), callback);
        listeners.delete(name);
      },
    },
  };
  Object.defineProperty(context, 'document', {
    get() {
      throw new Error('readiness control must not read DOM text, inputs or account data');
    },
  });
  vm.createContext(context);
  vm.runInContext(script, context);
  return {
    calls,
    listeners,
    timers,
    status,
    event(name) {
      listeners.get(name)?.();
    },
    timeout() {
      for (const callback of [...timers.values()]) callback();
    },
  };
}
function expectOutcome(h, outcome) {
  assert.deepEqual(h.calls, [
    {
      command: 'plugin:event|emit',
      args: {
        event: 'solosoul-native-perf-ui-gate',
        payload: { schemaVersion: 1, scope: 'windows-native-ui-ready-gate', runId, outcome },
      },
    },
  ]);
  assert.equal(h.listeners.size, 0);
  assert.equal(h.timers.size, 0);
}

test('one real handoff event controls when native document identity can be bound', () => {
  const h = harness();
  assert.equal(h.calls.length, 0);
  h.event('solosoul:startup-handoff');
  assert.equal(h.calls.length, 0, 'event without actual mark is insufficient');
  h.status.state = 'ready';
  h.event('solosoul:startup-state');
  assert.equal(h.calls.length, 0, 'ready state before two frames is insufficient');
  h.status.handedOff = true;
  h.event('solosoul:startup-handoff');
  expectOutcome(h, 'handoff');
  h.event('solosoul:startup-handoff');
  h.timeout();
  expectOutcome(h, 'handoff');
});
test('register then check handles handoff completed before native gate installation', () => {
  expectOutcome(harness({ handedOff: true, state: 'ready' }), 'handoff');
});
test('startup errors produce a fixed control outcome without copying private diagnostics', () => {
  const h = harness();
  h.status.state = 'error';
  h.event('solosoul:startup-state');
  expectOutcome(h, 'startup-error');
});
test('deadline is terminal and never declares UI readiness', () => {
  const h = harness();
  h.timeout();
  expectOutcome(h, 'timeout');
  h.status.handedOff = true;
  h.event('solosoul:startup-handoff');
  expectOutcome(h, 'timeout');
});
test('control transport rejection does not become an unhandled rejection or retry', async () => {
  const h = harness({ handedOff: true, reject: true });
  await Promise.resolve();
  expectOutcome(h, 'handoff');
});
