import assert from 'node:assert/strict';
import { test } from 'node:test';
import { verifyProductionShards } from './check-production-shards.mjs';

function fixture() {
  const inventory = {
    errors: [],
    suites: [
      {
        suites: [
          {
            specs: ['one', 'two', 'three', 'four'].map((id) => ({
              id,
              tests: [{ projectName: 'chromium', expectedStatus: 'passed', results: [] }],
            })),
          },
        ],
      },
    ],
  };
  const merged = structuredClone(inventory);
  for (const spec of merged.suites[0].suites[0].specs) {
    Object.assign(spec.tests[0], { status: 'expected', results: [{ status: 'passed' }] });
  }
  return { inventory, merged, specs: merged.suites[0].suites[0].specs };
}

test('all nested production cases and existing retry semantics are preserved', () => {
  const { inventory, merged, specs } = fixture();
  Object.assign(specs[0].tests[0], {
    status: 'flaky',
    results: [{ status: 'failed' }, { status: 'passed' }],
  });
  assert.deepEqual(verifyProductionShards(inventory, merged), {
    total: 4,
    expected: 3,
    flaky: 1,
    skipped: 0,
  });
});

test('missing or duplicate shard cases fail coverage validation', () => {
  for (const change of [(specs) => specs.pop(), (specs) => (specs[0] = specs[1])]) {
    const { inventory, merged, specs } = fixture();
    change(specs);
    assert.throws(() => verifyProductionShards(inventory, merged), /count mismatch|Duplicate/);
  }
});

test('substituting a different case with the same total cannot pass', () => {
  const { inventory, merged, specs } = fixture();
  specs[0].id = 'unexpected-replacement';
  assert.throws(() => verifyProductionShards(inventory, merged), /Missing production case/);
});

test('failed, interrupted or unexpectedly skipped cases cannot pass', () => {
  for (const status of ['unexpected', 'skipped']) {
    const { inventory, merged, specs } = fixture();
    Object.assign(specs[0].tests[0], { status, results: [{ status: 'interrupted' }] });
    assert.throws(() => verifyProductionShards(inventory, merged), /Unsuccessful/);
  }
});

test('a success label without a matching execution cannot pass', () => {
  const { inventory, merged, specs } = fixture();
  specs[0].tests[0].results = [];
  assert.throws(() => verifyProductionShards(inventory, merged), /No matching execution/);
});

test('shards cannot change a case expectation to skip it', () => {
  const { inventory, merged, specs } = fixture();
  specs[0].tests[0].expectedStatus = 'skipped';
  assert.throws(() => verifyProductionShards(inventory, merged), /Changed production expectation/);
});

test('a skip already declared in the full inventory retains its existing behavior', () => {
  const { inventory, merged, specs } = fixture();
  inventory.suites[0].suites[0].specs[0].tests[0].expectedStatus = 'skipped';
  Object.assign(specs[0].tests[0], {
    expectedStatus: 'skipped',
    status: 'skipped',
    results: [{ status: 'skipped' }],
  });
  assert.equal(verifyProductionShards(inventory, merged).skipped, 1);
});

test('empty reports and global runner failures cannot pass', () => {
  const { inventory, merged } = fixture();
  assert.throws(() => verifyProductionShards(inventory, { suites: [], errors: [] }), /Empty/);
  merged.errors.push({ message: 'worker crashed' });
  assert.throws(() => verifyProductionShards(inventory, merged), /global Playwright errors/);
});
