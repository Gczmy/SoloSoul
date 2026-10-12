import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const script = fs.readFileSync(
  new URL('../src-tauri/src/native_perf/sdk_surface.js', import.meta.url),
  'utf8',
);
const runId = '0123456789abcdef0123456789abcdef';
function capture({
  dataset = {},
  background = 'rgb(253, 252, 249)',
  text = 'rgb(17 17 17)',
  palette = {},
  duplicate = false,
  missingCard = false,
  expanded = 'true',
  focus = true,
  forced = false,
  reduced = false,
  readinessBudgetMs = null,
  bannerUntilMs = 0,
  bannerForever = false,
  shiftUntilMs = 0,
  frameStepMs = 100,
} = {}) {
  let now = 500;
  const bannerPresent = () => bannerForever || now < bannerUntilMs;
  const nodes = new Map();
  const make = () => ({
    getAttribute(name) {
      assert.equal(name, 'data-expanded');
      return expanded;
    },
    getBoundingClientRect() {
      return { x: 10, y: bannerPresent() || now < shiftUntilMs ? 88 : 40, width: 100, height: 60 };
    },
    get textContent() {
      throw Error('Business text must not be read');
    },
    get innerHTML() {
      throw Error('Business markup must not be read');
    },
  });
  for (const s of ['#desktop-navigation', '[data-appbar]', '[data-shell-content]'])
    nodes.set(s, make());
  const card = make();
  const root = {
    dataset: {
      theme: 'light',
      desktopPlatform: 'windows',
      nativeMaterial: 'mica',
      highContrast: 'false',
      ...dataset,
    },
  };
  const context = {
    document: {
      documentElement: root,
      hasFocus: () => focus,
      visibilityState: 'visible',
      querySelectorAll(selector) {
        if (selector === '[data-notification-banner="ocr"]') return bannerPresent() ? [make()] : [];
        assert(nodes.has(selector));
        return duplicate && selector === '[data-appbar]'
          ? [nodes.get(selector), make()]
          : [nodes.get(selector)];
      },
      querySelector(selector) {
        assert.equal(selector, '[data-shell-content] [data-ui-card]');
        return missingCard ? null : card;
      },
    },
    getComputedStyle(node) {
      if (node === root)
        return {
          getPropertyValue(key) {
            assert(['--bg-base', '--bg-elevated', '--text-primary'].includes(key));
            return (
              palette[key] ??
              { '--bg-base': '#fafaf6', '--bg-elevated': '#fdfcf9', '--text-primary': '#111111' }[
                key
              ]
            );
          },
        };
      return {
        backgroundColor: background,
        color: text,
        visibility: 'visible',
        display: 'block',
        borderTopLeftRadius: '12px',
      };
    },
    matchMedia(query) {
      assert(['(forced-colors: active)', '(prefers-reduced-transparency: reduce)'].includes(query));
      return { matches: query.includes('forced-colors') ? forced : reduced };
    },
    innerWidth: 1024,
    innerHeight: 750,
    devicePixelRatio: 1.25,
    location: { href: 'http://tauri.localhost/' },
    performance: { timeOrigin: 1000, now: () => now },
    requestAnimationFrame(callback) {
      now += frameStepMs;
      queueMicrotask(() => callback(now));
    },
    window: new Proxy(
      {},
      {
        get() {
          throw Error('No IPC or window mutation allowed');
        },
      },
    ),
  };
  const value = vm.runInNewContext(
    script.replace(
      '__REQUEST__',
      JSON.stringify({ runId, ...(readinessBudgetMs === null ? {} : { readinessBudgetMs }) }),
    ),
    context,
  );
  if (readinessBudgetMs !== null)
    return Promise.resolve(value).then((result) => JSON.parse(JSON.stringify(result)));
  return JSON.parse(JSON.stringify(value));
}
test('surface probe reads actual bounded theme, color tokens and viewport', () => {
  const state = capture();
  assert.equal(state.runId, runId);
  assert.equal(state.theme, 'light');
  assert.deepEqual(state.palette.base, [250, 250, 246, 1]);
  assert.deepEqual(state.viewport, [1024, 750, 1.25]);
  assert.equal(state.focused, true);
  assert.deepEqual(state.surfaces.card.rect, [10, 40, 100, 60]);
});
test('surface probe normalizes computed RGB and sRGB without preserving CSS strings', () => {
  for (const [background, expected] of [
    ['rgba(10, 20, 30, 0.5)', [10, 20, 30, 0.5]],
    ['rgb(10 20 30 / 0.5)', [10, 20, 30, 0.5]],
    ['color(srgb 0.2 0.4 0.6 / 0.5)', [51, 102, 153, 0.5]],
    ['#abc', [170, 187, 204, 1]],
  ])
    assert.deepEqual(capture({ background }).surfaces.card.background, expected);
});
test('duplicate appbar and missing card remain null for strict native rejection', () => {
  assert.equal(capture({ duplicate: true }).surfaces.appbar, null);
  assert.equal(capture({ missingCard: true }).surfaces.card, null);
});
test('surface probe never reads business text, markup or a business IPC', () => {
  const state = capture();
  assert.equal(Object.keys(state).length, 18);
  assert(!JSON.stringify(state).includes('textContent'));
  assert.deepEqual(Object.keys(state.palette).sort(), ['base', 'elevated', 'text']);
  assert.deepEqual(Object.keys(state.surfaces.card).sort(), [
    'background',
    'radius',
    'rect',
    'text',
  ]);
});
test('solid material, accessibility and actual unfocused dark state are observed', () => {
  const s = capture({
    dataset: { theme: 'dark', nativeMaterial: 'solid', highContrast: 'true' },
    focus: false,
    forced: true,
    reduced: true,
    expanded: 'false',
  });
  assert.equal(s.material, 'solid');
  assert.equal(s.theme, 'dark');
  assert.equal(s.highContrast, true);
  assert.equal(s.forcedColors, true);
  assert.equal(s.reducedTransparency, true);
  assert.equal(s.focused, false);
  assert.equal(s.expanded, false);
});
test('unsupported color syntax and malformed boolean attributes are not coerced into success', () => {
  assert.equal(capture({ background: 'url(private-path)' }).surfaces.card.background, null);
  assert.equal(
    capture({ dataset: { highContrast: 'unknown' }, expanded: 'unknown' }).highContrast,
    null,
  );
  assert.equal(capture({ expanded: 'unknown' }).expanded, null);
});

test('capture readiness waits for the existing OCR banner to naturally disappear', async () => {
  const state = await capture({ readinessBudgetMs: 10000, bannerUntilMs: 5500 });
  assert.equal(state.surfaces.card.rect[1], 40);
  assert(state.atMs >= 5500);
  assert.equal(Object.keys(state).length, 18);
});

test('capture readiness waits for geometry to settle after banner removal', async () => {
  const state = await capture({
    readinessBudgetMs: 10000,
    bannerUntilMs: 1500,
    shiftUntilMs: 1800,
  });
  assert.equal(state.surfaces.content.rect[1], 40);
  assert(state.atMs >= 2100);
});

test('persistent OCR banner rejects instead of consuming an unready frame', async () => {
  await assert.rejects(
    capture({ readinessBudgetMs: 1000, bannerForever: true }),
    /surface-readiness-timeout/,
  );
});

test('banner disappearing at the budget edge does not authorize an unsettled frame', async () => {
  await assert.rejects(
    capture({ readinessBudgetMs: 1000, bannerUntilMs: 1500 }),
    /surface-readiness-timeout/,
  );
});

test('capture readiness rejects missing bounds instead of accepting an unlimited wait', async () => {
  for (const readinessBudgetMs of [0, -1, 10001, 1.5, '1000'])
    await assert.rejects(capture({ readinessBudgetMs }), /surface-readiness-budget-invalid/);
});
