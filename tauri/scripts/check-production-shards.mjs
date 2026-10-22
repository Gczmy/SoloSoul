import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

function collectCases(report) {
  if (!Array.isArray(report.suites) || !Array.isArray(report.errors) || report.errors.length) {
    throw new Error('Missing suite data or global Playwright errors');
  }
  const cases = new Map();
  function visit(suites) {
    for (const suite of suites) {
      for (const spec of suite.specs ?? []) {
        for (const test of spec.tests ?? []) {
          if (!spec.id || !test.projectName) throw new Error('Missing production case identity');
          const key = JSON.stringify([spec.id, test.projectName]);
          if (cases.has(key)) throw new Error(`Duplicate production case: ${key}`);
          cases.set(key, test);
        }
      }
      visit(suite.suites ?? []);
    }
  }
  visit(report.suites);
  if (!cases.size) throw new Error('Empty production suite');
  return cases;
}

export function verifyProductionShards(inventory, merged) {
  const expected = collectCases(inventory);
  const actual = collectCases(merged);
  if (expected.size !== actual.size) {
    throw new Error(
      `Production case count mismatch: expected ${expected.size}, got ${actual.size}`,
    );
  }
  const summary = { total: expected.size, expected: 0, flaky: 0, skipped: 0 };
  for (const [key, original] of expected) {
    const test = actual.get(key);
    if (!test) throw new Error(`Missing production case: ${key}`);
    if (test.expectedStatus !== original.expectedStatus) {
      throw new Error(`Changed production expectation: ${key}`);
    }
    const declaredSkip = original.expectedStatus === 'skipped' && test.status === 'skipped';
    if (!declaredSkip && !['expected', 'flaky'].includes(test.status)) {
      throw new Error(`Unsuccessful or unexpectedly skipped production case: ${key}`);
    }
    if (!test.results?.some((result) => result.status === original.expectedStatus)) {
      throw new Error(`No matching execution result for production case: ${key}`);
    }
    summary[test.status] += 1;
  }
  return summary;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  if (process.argv.length !== 4) {
    console.error('Usage: node check-production-shards.mjs <inventory.json> <merged.json>');
    process.exitCode = 1;
  } else {
    try {
      const read = (path) => JSON.parse(readFileSync(path, 'utf8').replace(/^\uFEFF/, ''));
      console.log(
        JSON.stringify(verifyProductionShards(read(process.argv[2]), read(process.argv[3]))),
      );
    } catch (error) {
      console.error(error.message);
      process.exitCode = 1;
    }
  }
}
