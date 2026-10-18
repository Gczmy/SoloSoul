import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

assert.ok(process.env.RF312_STARTUP_MILESTONE_SCRIPT, 'actual compiled milestone script required');
const script = readFileSync(process.env.RF312_STARTUP_MILESTONE_SCRIPT, 'utf8');
function fixture() {
  const entries = new Map([
    ['mark:solosoul:react-mount', [{ startTime: 200 }]],
    ['mark:solosoul:startup-dismissed', [{ startTime: 400 }]],
    ['paint:first-paint', [{ startTime: 30 }]],
    ['paint:first-contentful-paint', [{ startTime: 40 }]],
    ['navigation', [{ startTime: 0, duration: 150, domInteractive: 80, domContentLoadedEventEnd: 100, loadEventEnd: 150 }]],
  ]);
  const calls = [];
  const context = {
    document: { visibilityState: 'visible', querySelectorAll(selector) { assert.equal(selector, 'iframe,frame'); return []; } },
    location: { hostname: 'tauri.localhost', protocol: 'https:', pathname: '/login' },
    performance: {
      timeOrigin: 10000, now: () => 600,
      getEntriesByName(name, type) { calls.push(`${type}:${name}`); assert.ok(entries.has(`${type}:${name}`)); return entries.get(`${type}:${name}`); },
      getEntriesByType(type) { calls.push(type); assert.equal(type, 'navigation'); return entries.get(type); },
      mark() { assert.fail('collector must not manufacture marks'); },
      clearMarks() { assert.fail('collector must preserve the native buffer'); },
      measure() { assert.fail('no new measurements may be inserted'); },
    },
    PerformanceObserver: { supportedEntryTypes: ['mark', 'paint', 'navigation'] },
    requestAnimationFrame() { assert.fail('no future draw request'); },
    setTimeout() { assert.fail('no deferred probe'); },
  };
  context.window = {}; context.top = context.window;
  return { context, entries, calls };
}
const read = f => JSON.parse(vm.runInNewContext(script, f.context, { timeout: 1000 }));

test('reads only the two existing app marks and optional paint/navigation fields', () => {
  const f = fixture(), result = read(f);
  assert.deepEqual(result.reactMount, { count: 1, entries: [200] });
  assert.deepEqual(result.startupDismissed, { count: 1, entries: [400] });
  assert.equal(result.observedAtMs, 600); assert.equal(result.timeOriginMs, 10000);
  assert.deepEqual(f.calls, ['navigation', 'mark:solosoul:react-mount', 'mark:solosoul:startup-dismissed', 'paint:first-paint', 'paint:first-contentful-paint']);
});
test('missing entries remain empty without fabricated zero timestamps', () => {
  const f = fixture(); for (const key of f.entries.keys()) f.entries.set(key, []);
  const r = read(f); for (const key of ['reactMount', 'startupDismissed', 'firstPaint', 'firstContentfulPaint', 'navigation']) assert.deepEqual(r[key], { count: 0, entries: [] });
});
test('missing query APIs are explicit and do not invoke a partial API', () => {
  const f = fixture(); delete f.context.performance.getEntriesByType;
  const r = read(f); assert.equal(r.entryQueryAvailable, false); assert.equal(r.reactMount.count, 0); assert.deepEqual(f.calls, []);
});
test('absent PerformanceObserver cannot be promoted into API support', () => {
  const f = fixture(); delete f.context.PerformanceObserver;
  const r = read(f); assert.equal(r.supportedEntryTypesAvailable, false);
  assert.equal(r.paintEntrySupported, false); assert.equal(r.markEntrySupported, false); assert.equal(r.navigationEntrySupported, false);
  assert.deepEqual(r.reactMount.entries, [200]);
});
test('native buffer multiplicity and bounded prefix are retained', () => {
  const f = fixture(); f.entries.set('mark:solosoul:react-mount', Array.from({ length: 10 }, (_, n) => ({ startTime: n })));
  assert.deepEqual(read(f).reactMount, { count: 10, entries: [0, 1, 2, 3] });
});
test('document eligibility is observed without navigation or mutation', () => {
  const f = fixture(); f.context.location.pathname = '/foreign'; f.context.location.hostname = 'foreign.example';
  f.context.top = {}; f.context.document.visibilityState = 'hidden';
  const r = read(f); assert.equal(r.loginRoute, false); assert.equal(r.localOrigin, false); assert.equal(r.mainFrame, false); assert.equal(r.documentVisible, false);
});
test('navigation URLs, mark details, and arbitrary entries are never read', () => {
  const f = fixture();
  for (const [key, rows] of f.entries) f.entries.set(key, rows.map(row => new Proxy(row, { get(target, property) { assert.ok(Object.hasOwn(target, property), `unapproved property ${String(property)}`); return target[property]; } })));
  const r = read(f); assert.deepEqual(Object.keys(r.navigation.entries[0]).sort(), ['startTimeMs', 'durationMs', 'domInteractiveMs', 'domContentLoadedEndMs', 'loadEventEndMs'].sort());
  assert.ok(!JSON.stringify(r).includes('url')); assert.ok(!JSON.stringify(r).includes('detail'));
});
test('observation timestamp follows all synchronous buffer reads', () => {
  const f = fixture(); let clock = 0;
  const original = f.context.performance.getEntriesByName;
  f.context.performance.getEntriesByName = (...args) => { clock += 100; return original(...args); };
  f.context.performance.now = () => clock;
  assert.equal(read(f).observedAtMs, 400);
});
test('unsupported or unavailable paint is separate from existing app marks', () => {
  const f = fixture(); f.context.PerformanceObserver.supportedEntryTypes = ['mark', 'navigation'];
  f.entries.set('paint:first-paint', []); f.entries.set('paint:first-contentful-paint', []);
  const r = read(f); assert.equal(r.paintEntrySupported, false); assert.equal(r.firstPaint.count, 0); assert.equal(r.reactMount.count, 1);
});
test('zero navigation event values remain zero with no host/native clock conversion', () => {
  const f = fixture(); f.entries.get('navigation')[0].domInteractive = 0;
  const r = read(f); assert.equal(r.navigation.entries[0].domInteractiveMs, 0);
  assert.equal(r.timeOriginMs, 10000); assert.ok(!Object.hasOwn(r, 'nativeTimestamp'));
});
