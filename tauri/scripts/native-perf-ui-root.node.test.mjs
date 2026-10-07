import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const source = readFileSync(
  new URL('../src-tauri/src/native_perf/sdk_cdp.rs', import.meta.url),
  'utf8',
);
const expression = source.match(/const EXPRESSION: &str = r#"([\s\S]*?)"#;/)?.[1];
assert.ok(expression);

function harness(options = {}) {
  const state = {
    rootExists: true,
    children: false,
    marked: false,
    screenPresent: true,
    state: 'loading',
    phase: 'preferences',
    reason: 'none',
    handoff: true,
    ...options,
  };
  const listeners = new Map();
  const timers = new Map();
  const calls = [];
  const root = { hasChildNodes: () => state.children };
  const screen = {
    get dataset() {
      return { state: state.state };
    },
  };
  for (const target of [root, screen])
    for (const key of ['textContent', 'innerHTML', 'value']) {
      Object.defineProperty(target, key, {
        get() {
          throw new Error('private DOM access');
        },
      });
    }
  const window = {
    __SOLOSOUL_NATIVE_PERF__: {
      snapshot: () => {
        calls.push('snapshot');
        return null;
      },
    },
    __SOLOSOUL_STARTUP__: {
      diagnostic: () => ({
        schemaVersion: 1,
        state: state.state,
        phase: state.phase,
        reason: state.reason,
      }),
    },
    addEventListener(name, callback) {
      listeners.set(name, callback);
    },
    removeEventListener(name, callback) {
      if (listeners.get(name) === callback) listeners.delete(name);
    },
  };
  window.top = window;
  const result = Promise.resolve(
    runInNewContext(expression, {
      window,
      location: { origin: 'http://tauri.localhost', href: 'http://tauri.localhost/' },
      document: {
        URL: 'http://tauri.localhost/',
        readyState: 'complete',
        getElementById(id) {
          calls.push(id);
          if (id === 'root') return state.rootExists ? root : null;
          if (id === 'startup-screen') return state.screenPresent ? screen : null;
          throw new Error('unexpected DOM query');
        },
        querySelectorAll(selector) {
          assert.equal(selector, 'iframe,frame');
          return [];
        },
      },
      performance: {
        timeOrigin: 1000,
        now: () => 1,
        getEntriesByName(name, type) {
          assert.equal(type, 'mark');
          if (name === 'solosoul:react-mount') return state.marked ? [{}] : [];
          assert.equal(name, 'solosoul:startup-dismissed');
          return state.handoff ? [{}] : [];
        },
      },
      setTimeout(callback, delay) {
        assert.equal(delay, 8000);
        timers.set(1, callback);
        return 1;
      },
      clearTimeout(id) {
        timers.delete(id);
      },
    }),
  ).then((value) => JSON.parse(JSON.stringify(value)));
  return {
    state,
    listeners,
    timers,
    calls,
    result,
    emit(name) {
      listeners.get(name)?.();
    },
    timeout() {
      assert.equal(timers.size, 1);
      timers.get(1)();
    },
  };
}
async function probe(options) {
  const h = harness(options);
  const result = await h.result;
  assert.equal(h.calls.filter((value) => value === 'snapshot').length, 1);
  return result;
}

test('missing root and empty root are distinct without reading DOM content', async () => {
  assert.equal((await probe({ rootExists: false })).uiRootDiagnostic.rootExists, false);
  const empty = await probe();
  assert.equal(empty.uiRootDiagnostic.rootExists, true);
  assert.equal(empty.uiRootPresent, false);
  assert.equal(empty.uiRootDiagnostic.reactMountMarked, false);
});
test('mount mark and startup error are independent fixed observations', async () => {
  const result = await probe({ marked: true, state: 'error' });
  assert.equal(result.uiRootDiagnostic.reactMountMarked, true);
  assert.equal(result.uiRootDiagnostic.startupState, 'error');
  assert.equal(result.uiRootDiagnostic.rootHasChildren, false);
});
test('populated root preserves the original successful root predicate', async () => {
  const result = await probe({ children: true, marked: true, state: 'ready' });
  assert.equal(result.uiRootPresent, true);
  assert.equal(result.uiRootDiagnostic.rootHasChildren, true);
});
test('unknown and absent startup state are reduced to fixed unavailable value', async () => {
  const unknown = await probe({
    state: 'private-state-sentinel',
    phase: 'private-phase',
    reason: 'private-reason',
  });
  assert.equal(unknown.uiRootDiagnostic.startupState, 'unavailable');
  assert.ok(!JSON.stringify(unknown).includes('private'));
  const absent = await probe({ screenPresent: false });
  assert.equal(absent.uiRootDiagnostic.startupScreenPresent, false);
  assert.equal(absent.uiRootDiagnostic.startupState, 'unavailable');
});
test('RF312 retains the early state and waits for actual handoff, not just a populated root or event', async () => {
  const h = harness({ handoff: false });
  let completed = false;
  h.result.then(() => {
    completed = true;
  });
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(completed, false);
  h.state.children = true;
  h.emit('solosoul:startup-handoff');
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(completed, false);
  h.state.handoff = true;
  h.state.state = 'ready';
  h.state.phase = 'accounts';
  h.emit('solosoul:startup-handoff');
  const result = await h.result;
  assert.equal(result.uiRootPresent, true);
  assert.equal(result.uiReadyDiagnostic.outcome, 'handoff');
  assert.equal(result.uiReadyDiagnostic.initialUiRootPresent, false);
  assert.equal(result.uiReadyDiagnostic.initialHandoffMarked, false);
  assert.equal(result.uiReadyDiagnostic.initialStartup.phase, 'preferences');
  assert.equal(result.uiReadyDiagnostic.finalStartup.phase, 'accounts');
  assert.equal(h.listeners.size, 0);
  assert.equal(h.timers.size, 0);
});
test('RF312 startup error is terminal and preserves its fixed stage without retrying', async () => {
  const h = harness({ handoff: false });
  h.state.state = 'error';
  h.state.reason = 'backend-unavailable';
  h.emit('solosoul:startup-state');
  const result = await h.result;
  assert.equal(result.uiReadyDiagnostic.outcome, 'startup-error');
  assert.equal(result.uiReadyDiagnostic.finalStartup.reason, 'backend-unavailable');
  assert.equal(result.uiRootPresent, false);
  assert.equal(h.listeners.size, 0);
  assert.equal(h.timers.size, 0);
});
test('RF312 bounded timeout remains failure with early and final observations', async () => {
  const h = harness({ handoff: false });
  h.timeout();
  const result = await h.result;
  assert.equal(result.uiReadyDiagnostic.outcome, 'timeout');
  assert.equal(result.uiReadyDiagnostic.initialStartup.state, 'loading');
  assert.equal(result.uiReadyDiagnostic.finalHandoffMarked, false);
  assert.equal(h.listeners.size, 0);
  assert.equal(h.timers.size, 0);
});

function startupHarness() {
  const events = [];
  const timers = new Map();
  let next = 0;
  const window = {
    dispatchEvent(event) {
      events.push(event.type);
    },
  };
  runInNewContext(readFileSync(new URL('../public/startup.js', import.meta.url), 'utf8'), {
    window,
    document: {
      documentElement: { dataset: {}, style: { setProperty() {} } },
      getElementById: () => null,
      addEventListener() {},
    },
    navigator: { language: 'en-US' },
    localStorage: { getItem: () => null },
    matchMedia: () => ({ matches: false }),
    performance: { now: () => 1 },
    setTimeout(callback) {
      timers.set(++next, callback);
      return next;
    },
    clearTimeout(id) {
      timers.delete(id);
    },
    Event: class {
      constructor(type) {
        this.type = type;
      }
    },
  });
  return { startup: window.__SOLOSOUL_STARTUP__, events, timers };
}
test('RF312 public startup diagnostic exposes only fixed phase/state/reason and remains terminal', () => {
  const h = startupHarness();
  h.startup.phase('capabilities');
  assert.deepEqual(JSON.parse(JSON.stringify(h.startup.diagnostic())), {
    schemaVersion: 1,
    state: 'loading',
    phase: 'capabilities',
    reason: 'none',
  });
  h.startup.fail('backend-unavailable');
  h.startup.phase('accounts');
  assert.equal(h.startup.ready(), false);
  assert.equal(h.startup.diagnostic().phase, 'capabilities');
  assert.equal(h.startup.diagnostic().reason, 'backend-unavailable');
  assert.deepEqual(h.events, ['solosoul:startup-state']);
  assert.equal(h.timers.size, 0);
});
test('RF312 startup diagnostics normalize unknown caller strings without reading UI content', () => {
  const h = startupHarness();
  h.startup.phase('private-phase');
  h.startup.fail('private-reason');
  assert.ok(!JSON.stringify(h.startup.diagnostic()).includes('private'));
  assert.equal(h.startup.diagnostic().phase, 'unavailable');
  assert.equal(h.startup.diagnostic().reason, 'unavailable');
});
