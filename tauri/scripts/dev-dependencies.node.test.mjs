import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { collectDevDependencies, devDependencyUpdates } from './dev-dependencies.mjs';

function fixture(t) {
  const parent = path.resolve(os.tmpdir());
  const root = mkdtempSync(path.join(parent, 'solosoul-dev-dependencies-'));
  t.after(() => {
    assert.equal(path.dirname(path.resolve(root)), parent);
    assert.ok(path.basename(root).startsWith('solosoul-dev-dependencies-'));
    rmSync(root, { recursive: true, force: true });
  });
  return root;
}

test('explicit optimizer covers static, re-exported, dynamic and deep runtime imports', (t) => {
  const root = fixture(t);
  mkdirSync(path.join(root, 'nested'));
  writeFileSync(
    path.join(root, 'entry.tsx'),
    `import React from 'react';
     import { invoke } from '@tauri-apps/api/core';
     export { useStore } from 'zustand';
     const camera = () => import('html5-qrcode');
     // import 'comment-only';
     const example = "import 'string-only'";
     import type { Page } from '@playwright/test';
     import local from '@/lib/local';
     import sibling from './sibling';
     import fs from 'node:fs';`,
  );
  writeFileSync(path.join(root, 'nested/deep.js'), `const store = require('zustand/vanilla');`);
  const dependencies = {
    react: '*',
    '@tauri-apps/api': '*',
    zustand: '*',
    'html5-qrcode': '*',
    'comment-only': '*',
    'string-only': '*',
  };
  assert.deepEqual(collectDevDependencies(root, dependencies), [
    '@tauri-apps/api/core',
    'html5-qrcode',
    'react',
    'react/jsx-dev-runtime',
    'react/jsx-runtime',
    'zustand',
    'zustand/vanilla',
  ]);
});

test('test, declaration and mock imports do not become runtime optimizer entries', (t) => {
  const root = fixture(t);
  for (const name of ['__tests__', '__mocks__', 'test']) {
    mkdirSync(path.join(root, name));
    writeFileSync(path.join(root, name, 'fixture.ts'), `import 'test-only';`);
  }
  for (const name of ['view.test.tsx', 'view.spec.ts', 'view.stories.jsx', 'types.d.ts']) {
    writeFileSync(path.join(root, name), `import 'test-only';`);
  }
  writeFileSync(path.join(root, 'runtime.ts'), `export { x } from 'runtime/deep';`);
  assert.deepEqual(collectDevDependencies(root, { runtime: '*', 'test-only': '*' }), [
    'runtime/deep',
  ]);
});

test('new runtime dependencies are picked up without maintaining a second include list', (t) => {
  const root = fixture(t);
  writeFileSync(path.join(root, 'entry.ts'), `import 'alpha';`);
  assert.deepEqual(collectDevDependencies(root, { alpha: '*' }), ['alpha']);
  writeFileSync(path.join(root, 'feature.ts'), `export * from '@scope/beta/subpath';`);
  assert.deepEqual(collectDevDependencies(root, { alpha: '*', '@scope/beta': '*' }), [
    '@scope/beta/subpath',
    'alpha',
  ]);
});

test('ordinary edits keep HMR; a new package subpath restarts optimization once', async (t) => {
  const root = fixture(t);
  const entry = path.join(root, 'entry.ts');
  const dependencies = { alpha: '*', beta: '*' };
  writeFileSync(entry, `import 'alpha';`);
  const plugin = devDependencyUpdates(
    root,
    dependencies,
    collectDevDependencies(root, dependencies),
  );
  let restarts = 0;
  const server = {
    restart: async () => {
      restarts += 1;
    },
    config: { logger: { error: assert.fail } },
  };
  assert.equal(plugin.hotUpdate({ file: entry, server }), undefined);
  writeFileSync(path.join(root, 'new-feature.ts'), `import 'beta/deep';`);
  writeFileSync(entry, `import 'alpha'; import './new-feature';`);
  assert.deepEqual(plugin.hotUpdate({ file: entry, server }), []);
  assert.deepEqual(plugin.hotUpdate({ file: entry, server }), []);
  await Promise.resolve();
  assert.equal(restarts, 1);
});

test('a failed restart reports the error and leaves the dependency change actionable', async (t) => {
  const root = fixture(t);
  const entry = path.join(root, 'entry.ts');
  writeFileSync(entry, `import 'alpha';`);
  const plugin = devDependencyUpdates(root, { alpha: '*', beta: '*' }, ['alpha']);
  const errors = [];
  let restarts = 0;
  const server = {
    restart: async () => {
      restarts += 1;
      throw new Error('controlled restart failure');
    },
    config: { logger: { error: (message) => errors.push(message) } },
  };
  writeFileSync(entry, `import 'alpha'; import 'beta';`);
  assert.deepEqual(plugin.hotUpdate({ file: entry, server }), []);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(errors, ['Dependency optimization restart failed: controlled restart failure']);
  assert.deepEqual(plugin.hotUpdate({ file: entry, server }), []);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(restarts, 2);
});
