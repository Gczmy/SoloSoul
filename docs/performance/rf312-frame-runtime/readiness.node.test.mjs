import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';

const input = process.env.RF312_FRAME_STATE_SCRIPT;
assert.ok(input && path.isAbsolute(input));
const script = readFileSync(input, 'utf8');

function observe(options = {}) {
  const rect = (x, y, width = 120, height = 35) => ({ x, y, left: x, top: y, right: x + width, bottom: y + height, width, height });
  const password = {
    disabled: !!options.passwordDisabled,
    getBoundingClientRect: () => rect(options.offscreen ? -10 : 20, 20),
    contains: candidate => candidate === password,
    closest: () => options.noForm ? null : { querySelectorAll: selector => {
      assert.equal(selector, 'button[type=submit]'); return [submit];
    } },
  };
  const submit = {
    disabled: !!options.submitDisabled,
    getBoundingClientRect: () => rect(20, 80),
    contains: candidate => candidate === submit,
  };
  for (const node of [password, submit]) {
    Object.defineProperty(node, 'value', { get() { throw Error('Credential values must not be read'); } });
    Object.defineProperty(node, 'textContent', { get() { throw Error('Account or button text must not be read'); } });
  }
  const context = {
    location: { hostname: options.foreignOrigin ? 'example.invalid' : 'tauri.localhost', protocol: 'http:', pathname: options.otherPage ? '/' : '/login' },
    innerWidth: 400, innerHeight: 800,
    performance: { timeOrigin: 1000000000.5, now: () => 140.25 },
    getComputedStyle: () => ({ display: 'block', visibility: 'visible', opacity: '1' }),
    document: {
      visibilityState: options.hidden ? 'hidden' : 'visible',
      hasFocus: () => !options.unfocused,
      querySelectorAll: selector => {
        if (selector === 'input[type=password]') return options.duplicate ? [password, { ...password }] : [password];
        assert.equal(selector, 'iframe,frame'); return options.childFrame ? [{}] : [];
      },
      elementFromPoint: (_x, y) => options.coverPassword && y < 70 || options.coverSubmit && y > 70 ? {} : y < 70 ? password : submit,
    },
    setTimeout() { throw Error('Observation must not schedule page timers'); },
  };
  for (const name of ['ipc', 'fetch', '__TAURI_INTERNALS__']) {
    Object.defineProperty(context, name, { get() { throw Error('Business or transport APIs must not be accessed'); } });
  }
  context.window = context;
  context.top = options.childFrame ? {} : context;
  const raw = vm.runInNewContext(script, context, { timeout: 1000 });
  return JSON.parse(raw);
}

test('eligible real-shaped password form projects only allowed metadata, without values or IPC', () => {
  const value = observe();
  assert.deepEqual(Object.keys(value).sort(), [
    'schemaVersion', 'mainFrame', 'childFrames', 'localOrigin', 'loginRoute', 'passwordInputs', 'submitButtons',
    'enabled', 'visible', 'hitPassword', 'hitSubmit', 'documentFocused', 'documentVisible', 'timeOriginMs', 'atMs',
  ].sort());
  assert.equal(value.schemaVersion, 2);
  for (const name of ['mainFrame', 'localOrigin', 'loginRoute', 'enabled', 'visible', 'hitPassword', 'hitSubmit', 'documentFocused', 'documentVisible']) assert.equal(value[name], true, name);
  assert.equal(value.passwordInputs, 1); assert.equal(value.submitButtons, 1); assert.equal(value.childFrames, 0);
  assert.equal(value.timeOriginMs, 1000000000.5); assert.equal(value.atMs, 140.25);
});

test('covered password and submit controls cannot pass the corresponding hit test', () => {
  assert.equal(observe({ coverPassword: true }).hitPassword, false);
  assert.equal(observe({ coverSubmit: true }).hitSubmit, false);
});

test('screen-edge overflow excludes the password control instead of accepting its centre alone', () => {
  const value = observe({ offscreen: true });
  assert.equal(value.passwordInputs, 0); assert.equal(value.visible, false); assert.equal(value.enabled, false);
});

test('duplicate password inputs do not select an arbitrary form', () => {
  const value = observe({ duplicate: true });
  assert.equal(value.passwordInputs, 2); assert.equal(value.submitButtons, 0); assert.equal(value.enabled, false);
});

test('foreign origins, child frames and non-login routes remain explicitly ineligible', () => {
  assert.equal(observe({ foreignOrigin: true }).localOrigin, false);
  assert.equal(observe({ childFrame: true }).mainFrame, false);
  assert.equal(observe({ childFrame: true }).childFrames, 1);
  assert.equal(observe({ otherPage: true }).loginRoute, false);
});

test('disabled password and submit controls remain visible but not enabled', () => {
  for (const options of [{ passwordDisabled: true }, { submitDisabled: true }]) {
    const value = observe(options); assert.equal(value.visible, true); assert.equal(value.enabled, false);
  }
});

test('missing form produces an ineligible receipt rather than an optional-chain or spread exception', () => {
  const value = observe({ noForm: true });
  assert.equal(value.submitButtons, 0); assert.equal(value.enabled, false); assert.equal(value.hitSubmit, false);
});

test('a hidden document remains explicitly ineligible', () => {
  assert.equal(observe({ hidden: true }).documentVisible, false);
});

test('document input focus is independent from visible native-window eligibility', () => {
  const value = observe({ unfocused: true });
  assert.equal(value.documentFocused, false);
  for (const key of ['documentVisible', 'mainFrame', 'localOrigin', 'loginRoute', 'enabled', 'visible', 'hitPassword', 'hitSubmit']) {
    assert.equal(value[key], true, key);
  }
});
