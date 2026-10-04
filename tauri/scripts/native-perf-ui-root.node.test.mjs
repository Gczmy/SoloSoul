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

function probe({
  rootExists = true,
  children = false,
  marked = false,
  screenPresent = true,
  state = 'loading',
} = {}) {
  const calls = [];
  const root = { hasChildNodes: () => children };
  const screen = { dataset: { state } };
  for (const target of [root, screen]) {
    for (const key of ['textContent', 'innerHTML', 'value']) {
      Object.defineProperty(target, key, {
        get() {
          throw new Error('private DOM access');
        },
      });
    }
  }
  const window = {
    __SOLOSOUL_NATIVE_PERF__: {
      snapshot: () => {
        calls.push('snapshot');
        return null;
      },
    },
  };
  window.top = window;
  const result = runInNewContext(expression, {
    window,
    location: { origin: 'http://tauri.localhost', href: 'http://tauri.localhost/' },
    document: {
      URL: 'http://tauri.localhost/',
      readyState: 'complete',
      getElementById(id) {
        calls.push(id);
        if (id === 'root') return rootExists ? root : null;
        if (id === 'startup-screen') return screenPresent ? screen : null;
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
        assert.equal(name, 'solosoul:react-mount');
        assert.equal(type, 'mark');
        return marked ? [{}] : [];
      },
    },
  });
  assert.deepEqual(calls, ['root', 'startup-screen', 'snapshot']);
  return JSON.parse(JSON.stringify(result));
}

test('missing root and empty root are distinct without reading DOM content', () => {
  assert.equal(probe({ rootExists: false }).uiRootDiagnostic.rootExists, false);
  const empty = probe();
  assert.equal(empty.uiRootDiagnostic.rootExists, true);
  assert.equal(empty.uiRootPresent, false);
  assert.equal(empty.uiRootDiagnostic.reactMountMarked, false);
});
test('mount mark and startup error are independent fixed observations', () => {
  const result = probe({ marked: true, state: 'error' });
  assert.equal(result.uiRootDiagnostic.reactMountMarked, true);
  assert.equal(result.uiRootDiagnostic.startupState, 'error');
  assert.equal(result.uiRootDiagnostic.rootHasChildren, false);
});
test('populated root preserves the original successful root predicate', () => {
  const result = probe({ children: true, marked: true, state: 'ready' });
  assert.equal(result.uiRootPresent, true);
  assert.equal(result.uiRootDiagnostic.rootHasChildren, true);
});
test('unknown and absent startup state are reduced to fixed unavailable value', () => {
  const unknown = probe({ state: 'private-state-sentinel' });
  assert.equal(unknown.uiRootDiagnostic.startupState, 'unavailable');
  assert.ok(!JSON.stringify(unknown).includes('private'));
  const absent = probe({ screenPresent: false });
  assert.equal(absent.uiRootDiagnostic.startupScreenPresent, false);
  assert.equal(absent.uiRootDiagnostic.startupState, 'unavailable');
});
