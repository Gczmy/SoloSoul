import assert from 'node:assert/strict';
import { test } from 'node:test';
import path from 'node:path';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
import { watch, writeFileSync } from 'node:fs';
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  readdir,
  lstat,
  realpath,
  rm,
  symlink,
  rename,
} from 'node:fs/promises';
import {
  CORE_FILES,
  validateRuntimeRelative,
  validateCoreMetadata,
  validateAmd64Pe,
  installedEvergreenFolder,
  inspectRuntimeSource,
  stageRuntimeCopy,
  checkSelectedRuntimeMarker,
  checkSelectedRuntimeBrowser,
} from './native-perf-runtime.mjs';

const BS = String.fromCharCode(92);
const EXTENDED = BS + BS + '?' + BS;
const VERSION = '153.0.4234.32';
const sha = (value) => createHash('sha256').update(value).digest('hex');
const ordinal = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
const toCanonical = (value) => (value.startsWith(EXTENDED) ? value : EXTENDED + value);
const toOrdinary = (value) => (value.startsWith(EXTENDED) ? value.slice(4) : value);
function pe(payload = 1) {
  const value = Buffer.alloc(512, payload);
  value.writeUInt16LE(0x5a4d, 0);
  value.writeUInt32LE(128, 60);
  value.writeUInt32LE(0x4550, 128);
  value.writeUInt16LE(0x8664, 132);
  value.writeUInt16LE(3, 134);
  value.writeUInt16LE(240, 148);
  value.writeUInt16LE(0x20b, 152);
  return value;
}
function metadata(relative, digest) {
  return {
    relative,
    sha256: digest,
    architecture: 'AMD64',
    fileVersion: VERSION,
    productVersion: VERSION,
    signatureStatus: 'Valid',
    signerSubject: 'CN=Microsoft Corporation, O=Microsoft Corporation, C=US',
    signerThumbprint: 'A'.repeat(40),
  };
}
async function fixtureProof(sourceFolder) {
  const files = [];
  const directories = [];
  async function visit(dir, prefix = '') {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const relative = prefix ? prefix + '/' + entry.name : entry.name;
      const file = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        directories.push(relative);
        await visit(file, relative);
      } else {
        const contents = await readFile(file);
        files.push({ relative, bytes: contents.length, sha256: sha(contents) });
      }
    }
  }
  await visit(sourceFolder);
  files.sort((a, b) => ordinal(a.relative, b.relative));
  directories.sort(ordinal);
  return {
    sourceKind: 'copied-local-evergreen',
    sourceFolder,
    expectedVersion: VERSION,
    files,
    directories,
    coreFiles: CORE_FILES.map((relative) =>
      metadata(relative, files.find((file) => file.relative === relative).sha256),
    ),
  };
}
async function withFiles(run) {
  const work = await mkdtemp(path.join(await realpath(tmpdir()), 'ss-rf312-runtime-'));
  const parent = await realpath(tmpdir());
  try {
    const source = path.join(work, 'source');
    const root = path.join(work, 'owned');
    await mkdir(source);
    await mkdir(root);
    for (const [index, relative] of CORE_FILES.entries()) {
      const file = path.join(source, ...relative.split('/'));
      await mkdir(path.dirname(file), { recursive: true });
      await writeFile(file, pe(index + 1));
    }
    await mkdir(path.join(source, 'empty', 'nested'), { recursive: true });
    await writeFile(path.join(source, 'resources.pak'), 'public synthetic runtime resource');
    const owned = {
      schemaVersion: 1,
      scope: 'windows-native-perf-owned',
      nativePerfFeature: true,
      preparationStatus: 'ready',
      runId: 'a'.repeat(32),
      identifier: 'com.solosoul.rf312perf.' + 'a'.repeat(32),
      root: toCanonical(root),
    };
    await writeFile(path.join(root, 'native-perf-owned.json'), JSON.stringify(owned));
    const proof = await fixtureProof(source);
    await run({ work, source, root, owned, proof });
  } finally {
    const resolved = await realpath(work);
    if (
      path.dirname(resolved).toLowerCase() !== parent.toLowerCase() ||
      !path.basename(resolved).startsWith('ss-rf312-runtime-') ||
      (await lstat(work)).isSymbolicLink()
    )
      throw new Error(
        'Refusing Runtime test cleanup outside its exclusively created temporary root',
      );
    await rm(work, { recursive: true, force: false });
  }
}
function selected(manifest, manifestSha256) {
  return {
    schemaVersion: 1,
    scope: 'windows-native-perf-selected-runtime',
    mode: 'copied-local-evergreen',
    performanceSample: false,
    root: manifest.root,
    runId: manifest.runId,
    pid: 101,
    port: 44001,
    runtimeFolder: manifest.runtimeFolder,
    browserExecutableFolder: toOrdinary(manifest.runtimeFolder),
    sourceFolder: manifest.sourceFolder,
    expectedVersion: VERSION,
    availableVersion: VERSION,
    manifestSha256,
    executableSha256: manifest.files.find((file) => file.relative === 'msedgewebview2.exe').sha256,
  };
}
function browser(manifest) {
  const identity = {
    pid: 102,
    parentPid: 101,
    creationMs: 1100,
    executablePath: path.win32.join(toOrdinary(manifest.runtimeFolder), 'msedgewebview2.exe'),
  };
  const value = {
    schemaVersion: 1,
    scope: 'windows-native-perf-process-diagnostics',
    mode: 'live',
    success: true,
    identityVerified: true,
    processes: [
      {
        ...identity,
        identityMatched: true,
        version: { fileVersion: VERSION, productVersion: VERSION, reason: null },
      },
    ],
  };
  return { identity, value };
}
function declaration(proof, owned) {
  return {
    schemaVersion: 1,
    scope: 'windows-native-perf-copied-runtime',
    root: owned.root,
    runId: owned.runId,
    ...proof,
    runtimeFolder: path.win32.join(owned.root, 'runtime'),
  };
}

test('Windows Runtime relative names reject traversal, namespaces, aliases and device names', () => {
  for (const value of [
    '',
    '/root',
    '../a',
    'a/../b',
    'a//b',
    'a/./b',
    'a:' + BS + 'b',
    'a' + BS + 'b',
    'a.',
    'a ',
    'CON',
    'a/LPT1.dll',
    'COM0',
    'bad?file',
    'bad' + String.fromCharCode(0) + 'file',
    'a'.repeat(1025),
  ])
    assert.throws(() => validateRuntimeRelative(value));
  for (const value of CORE_FILES) assert.equal(validateRuntimeRelative(value), value);
});
test('Runtime relative paths allow exactly 64 segments and reject a file or directory at segment 65', () => {
  const directories64 = Array.from({ length: 64 }, () => 'a').join('/');
  const file64 = Array.from({ length: 63 }, () => 'a')
    .concat('file.bin')
    .join('/');
  assert.equal(validateRuntimeRelative(directories64), directories64);
  assert.equal(validateRuntimeRelative(file64), file64);
  assert.throws(() => validateRuntimeRelative(directories64 + '/file.bin'));
  assert.throws(() => validateRuntimeRelative(directories64 + '/directory'));
});
test('PE parser validates actual AMD64 PE32+ structure instead of trusting metadata', () => {
  assert.equal(validateAmd64Pe(pe()), 'AMD64');
  for (const change of [
    (b) => b.writeUInt16LE(0, 0),
    (b) => b.writeUInt32LE(0xffffffff, 60),
    (b) => b.writeUInt32LE(0, 128),
    (b) => b.writeUInt16LE(0xaa64, 132),
    (b) => b.writeUInt16LE(0x10b, 152),
    (b) => b.writeUInt16LE(0, 134),
    (b) => b.writeUInt16LE(0, 148),
  ]) {
    const bytes = pe();
    change(bytes);
    assert.throws(() => validateAmd64Pe(bytes));
  }
  assert.throws(() => validateAmd64Pe(Buffer.alloc(32)));
  assert.throws(() => validateAmd64Pe(pe(), 300));
});
test('core signature/version samples require exact versions and Valid Microsoft organization', () => {
  const good = metadata(CORE_FILES[0], 'a'.repeat(64));
  assert.equal(validateCoreMetadata(good, VERSION), good);
  for (const change of [
    { relative: 'other.dll' },
    { architecture: 'ARM64' },
    { fileVersion: '154.0.4258.37' },
    { productVersion: '153.0.4234.32 extra' },
    { signatureStatus: 'NotSigned' },
    { signatureStatus: 'UnknownError' },
    { signerSubject: 'CN=Fake Microsoft Corporation, O=Other' },
    { signerThumbprint: 'A'.repeat(39) },
    { sha256: 'A'.repeat(64) },
  ])
    assert.throws(() => validateCoreMetadata({ ...good, ...change }, VERSION));
  for (const value of ['153', '153.0.1', '153.0.1.2.3', '153.0.1.65536', '0153.0.1.2'])
    assert.throws(() => validateCoreMetadata(good, value));
});
test(
  'real inspection accepts only the explicit installed Evergreen folder and rejects others before signatures',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    const programFiles = 'C:' + BS + 'Program Files (x86)';
    assert.equal(
      installedEvergreenFolder(VERSION, programFiles),
      path.win32.join(programFiles, 'Microsoft', 'EdgeWebView', 'Application', VERSION),
    );
    for (const bad of [
      '',
      BS + BS + 'server' + BS + 'share',
      EXTENDED + programFiles,
      'relative',
      'C:relative',
    ])
      assert.throws(() => installedEvergreenFolder(VERSION, bad));
    await withFiles(async ({ source, root }) => {
      await assert.rejects(inspectRuntimeSource(source, VERSION), /exact installed Evergreen/);
      await assert.rejects(inspectRuntimeSource(toCanonical(source), VERSION), /ordinary absolute/);
      assert.deepEqual((await readdir(root)).sort(), ['native-perf-owned.json']);
    });
  },
);
test(
  'full Runtime tree copies real files and empty directories, with an exclusive bound manifest',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root, source }) => {
      const before = JSON.stringify(proof);
      const manifest = await stageRuntimeCopy(proof, owned);
      assert.equal(manifest.scope, 'windows-native-perf-copied-runtime');
      assert.equal(manifest.root, owned.root);
      assert.equal(manifest.runtimeFolder, path.win32.join(owned.root, 'runtime'));
      assert.equal(JSON.stringify(proof), before);
      assert.deepEqual(await fixtureProof(path.join(root, 'runtime')), {
        ...proof,
        sourceFolder: path.join(root, 'runtime'),
      });
      assert.equal(
        (await lstat(path.join(root, 'runtime', 'empty', 'nested'))).isDirectory(),
        true,
      );
      assert.deepEqual(await readdir(path.join(root, 'runtime', 'empty', 'nested')), []);
      assert.deepEqual(
        JSON.parse(await readFile(path.join(root, 'native-perf-runtime.json'), 'utf8')),
        manifest,
      );
      assert.deepEqual(await fixtureProof(source), proof);
    });
  },
);
test(
  'Runtime reuse cannot replace the runtime folder or previously published manifest',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root }) => {
      await stageRuntimeCopy(proof, owned);
      const manifest = await readFile(path.join(root, 'native-perf-runtime.json'));
      const exe = await readFile(path.join(root, 'runtime', 'msedgewebview2.exe'));
      await assert.rejects(stageRuntimeCopy(proof, owned), /exclusive-copy/);
      assert.deepEqual(await readFile(path.join(root, 'native-perf-runtime.json')), manifest);
      assert.deepEqual(await readFile(path.join(root, 'runtime', 'msedgewebview2.exe')), exe);
      assert.equal(
        JSON.parse(await readFile(path.join(root, 'native-perf-runtime-failure.json'), 'utf8'))
          .phase,
        'exclusive-copy',
      );
    });
  },
);
test(
  'changed source files are rejected before Runtime creation and failures keep evidence',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root, source }) => {
      await writeFile(path.join(source, 'resources.pak'), 'source changed');
      await assert.rejects(stageRuntimeCopy(proof, owned), /changed since/);
      await assert.rejects(lstat(path.join(root, 'runtime')), { code: 'ENOENT' });
      const failure = JSON.parse(
        await readFile(path.join(root, 'native-perf-runtime-failure.json'), 'utf8'),
      );
      assert.equal(failure.runId, owned.runId);
      assert.equal(failure.performanceSample, false);
      assert.equal(failure.phase, 'source-validation');
    });
  },
);
test(
  'source junctions are rejected without traversing or writing their other directory',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ work, source, proof, owned, root }) => {
      const other = path.join(work, 'other');
      await mkdir(other);
      await writeFile(path.join(other, 'keep'), 'untouched');
      await symlink(other, path.join(source, 'redirected'), 'junction');
      await assert.rejects(stageRuntimeCopy(proof, owned), /reparse/);
      await assert.rejects(lstat(path.join(root, 'runtime')), { code: 'ENOENT' });
      assert.equal(await readFile(path.join(other, 'keep'), 'utf8'), 'untouched');
    });
  },
);
test(
  'ownership mismatches refuse copied Runtime writes and cannot publish evidence into another root',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root }) => {
      for (const change of [
        { root: toOrdinary(owned.root) },
        { runId: 'b'.repeat(32) },
        { scope: 'production' },
        { identifier: 'com.solosoul' },
      ])
        await assert.rejects(
          stageRuntimeCopy(proof, { ...owned, ...change }),
          /ownership|owned root/,
        );
      assert.deepEqual((await readdir(root)).sort(), ['native-perf-owned.json']);
    });
  },
);
test(
  'core declarations and physical PE architecture are both checked before copying',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, source, root }) => {
      const wrong = structuredClone(proof);
      wrong.coreFiles[0].architecture = 'ARM64';
      await assert.rejects(stageRuntimeCopy(wrong, owned), /AMD64/);
      const bytes = pe();
      bytes.writeUInt16LE(0xaa64, 132);
      await writeFile(path.join(source, 'msedgewebview2.exe'), bytes);
      const refreshed = await fixtureProof(source);
      await assert.rejects(stageRuntimeCopy(refreshed, owned), /AMD64 PE32/);
      await assert.rejects(lstat(path.join(root, 'runtime')), { code: 'ENOENT' });
    });
  },
);
test(
  'bounded and exact Runtime file proof rejects duplicate paths, excess bytes and missing core',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root }) => {
      for (const change of [
        { files: [...proof.files, proof.files[0]] },
        { files: proof.files.map((f, index) => (index ? f : { ...f, bytes: 1073741825 })) },
        { coreFiles: proof.coreFiles.slice(1) },
        { files: proof.files.map((f, index) => (index ? f : { ...f, relative: '../outside' })) },
      ])
        await assert.rejects(
          stageRuntimeCopy({ ...proof, ...change }, owned),
          /inventory|metadata|relative/,
        );
      const many = Array.from({ length: 3001 }, (_, i) => ({
        relative: 'file-' + String(i).padStart(5, '0'),
        bytes: 1,
        sha256: 'a'.repeat(64),
      }));
      await assert.rejects(stageRuntimeCopy({ ...proof, files: many }, owned), /inventory/);
      await assert.rejects(lstat(path.join(root, 'runtime')), { code: 'ENOENT' });
    });
  },
);
test(
  'preexisting Runtime manifest is never overwritten and failed copy remains reviewable',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root }) => {
      await writeFile(path.join(root, 'native-perf-runtime.json'), 'existing evidence');
      await assert.rejects(stageRuntimeCopy(proof, owned), /exclusive-manifest/);
      assert.equal(
        await readFile(path.join(root, 'native-perf-runtime.json'), 'utf8'),
        'existing evidence',
      );
      assert.equal((await lstat(path.join(root, 'runtime', 'msedgewebview2.exe'))).isFile(), true);
      assert.equal(
        JSON.parse(await readFile(path.join(root, 'native-perf-runtime-failure.json'), 'utf8'))
          .phase,
        'exclusive-manifest',
      );
    });
  },
);
test('selected marker binds exact canonical identity, ordinary folder, source version and manifest hash', () => {
  // 仅校验声明形状；使用纯合成 Windows 路径，保留 Unix 对该逻辑的覆盖。
  const files = CORE_FILES.map((relative, index) => {
    const contents = pe(index + 1);
    return { relative, bytes: contents.length, sha256: sha(contents) };
  }).sort((a, b) => ordinal(a.relative, b.relative));
  const proof = {
    sourceKind: 'copied-local-evergreen',
    sourceFolder: 'C:' + BS + 'rf312-source',
    expectedVersion: VERSION,
    files,
    directories: ['EBWebView', 'EBWebView/x64'],
    coreFiles: CORE_FILES.map((relative) =>
      metadata(relative, files.find((file) => file.relative === relative).sha256),
    ),
  };
  const owned = {
    root: EXTENDED + 'C:' + BS + 'rf312-owned',
    runId: 'a'.repeat(32),
  };
  const manifest = declaration(proof, owned);
  const hash = 'd'.repeat(64);
  const value = selected(manifest, hash);
  assert.equal(checkSelectedRuntimeMarker(value, manifest, 101, 44001, hash), value);
  for (const change of [
    { root: toOrdinary(value.root) },
    { runId: 'b'.repeat(32) },
    { pid: 999 },
    { port: 44002 },
    { mode: 'fixed-version' },
    { scope: 'selected' },
    { performanceSample: true },
    { runtimeFolder: toOrdinary(value.runtimeFolder) },
    { browserExecutableFolder: value.runtimeFolder },
    { browserExecutableFolder: value.browserExecutableFolder + BS + 'other' },
    { sourceFolder: proof.sourceFolder + BS + 'other' },
    { expectedVersion: '154.0.4258.37' },
    { availableVersion: '154.0.4258.37' },
    { manifestSha256: 'c'.repeat(64) },
    { executableSha256: 'b'.repeat(64) },
  ])
    assert.throws(() =>
      checkSelectedRuntimeMarker({ ...value, ...change }, manifest, 101, 44001, hash),
    );
  assert.throws(() => checkSelectedRuntimeMarker(value, manifest, 101, 44001, 'D'.repeat(64)));
  assert.throws(() =>
    checkSelectedRuntimeMarker(
      value,
      { ...manifest, runtimeFolder: manifest.runtimeFolder + BS + 'other' },
      101,
      44001,
      hash,
    ),
  );
});
test(
  'browser proof checks actual physical copied EXE and exact recorded FileVersion/ProductVersion',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned }) => {
      const manifest = await stageRuntimeCopy(proof, owned);
      const { identity, value } = browser(manifest);
      const result = await checkSelectedRuntimeBrowser(identity, value, manifest);
      assert.equal(result.matched, true);
      assert.equal(result.fileVersion, VERSION);
      assert.equal(
        result.sha256,
        manifest.files.find((f) => f.relative === 'msedgewebview2.exe').sha256,
      );
      for (const change of [{ success: false }, { identityVerified: false }, { scope: 'unknown' }])
        await assert.rejects(
          checkSelectedRuntimeBrowser(identity, { ...value, ...change }, manifest),
          /verified process/,
        );
      for (const change of [
        { creationMs: 1101 },
        { parentPid: 999 },
        { identityMatched: false },
        { executablePath: path.join(proof.sourceFolder, 'msedgewebview2.exe') },
        { version: { fileVersion: '154.0.4258.37', productVersion: VERSION, reason: null } },
        { version: { fileVersion: VERSION, productVersion: '154.0.4258.37', reason: null } },
      ])
        await assert.rejects(
          checkSelectedRuntimeBrowser(
            identity,
            { ...value, processes: [{ ...value.processes[0], ...change }] },
            manifest,
          ),
        );
      await assert.rejects(
        checkSelectedRuntimeBrowser(
          identity,
          { ...value, processes: [...value.processes, ...value.processes] },
          manifest,
        ),
        /verified process/,
      );
    });
  },
);
test(
  'modified copied browser bytes cannot be accepted using stale Runtime metadata',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root }) => {
      const manifest = await stageRuntimeCopy(proof, owned);
      const { identity, value } = browser(manifest);
      const file = path.join(root, 'runtime', 'msedgewebview2.exe');
      const bytes = await readFile(file);
      bytes[500] ^= 1;
      await writeFile(file, bytes);
      await assert.rejects(checkSelectedRuntimeBrowser(identity, value, manifest), /hash changed/);
    });
  },
);
test(
  'browser directory junction fallback is rejected even if its version and bytes match the source',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root, source }) => {
      const manifest = await stageRuntimeCopy(proof, owned);
      const { identity, value } = browser(manifest);
      const runtime = path.join(root, 'runtime');
      await rename(runtime, path.join(root, 'original-runtime'));
      await symlink(source, runtime, 'junction');
      await assert.rejects(checkSelectedRuntimeBrowser(identity, value, manifest), /physical path/);
    });
  },
);

test(
  'explicit directory inventory rejects missing empty dirs, case aliases and file conflicts',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root }) => {
      for (const directories of [
        proof.directories.filter((d) => d !== 'empty/nested'),
        [...proof.directories, 'empty'].sort(ordinal),
        [...proof.directories, 'EMPTY'].sort(ordinal),
        [...proof.directories, 'resources.pak'].sort(ordinal),
        proof.directories.filter((d) => d !== 'EBWebView'),
      ])
        await assert.rejects(
          stageRuntimeCopy({ ...proof, directories }, owned),
          /directory inventory|changed since/,
        );
      await assert.rejects(lstat(path.join(root, 'runtime')), { code: 'ENOENT' });
    });
  },
);

test(
  'source mutation after copy begins rejects the new runtime and preserves partial evidence',
  {
    skip:
      process.platform === 'win32'
        ? false
        : 'Requires Windows drive and extended paths for physical owned Runtime fixtures',
  },
  async () => {
    await withFiles(async ({ proof, owned, root, source }) => {
      const resource = path.join(source, 'resources.pak');
      const changedBytes = await readFile(resource);
      changedBytes[0] ^= 1;
      let changed = false;
      let mutationError;
      const watcher = watch(root, { persistent: false }, (_event, filename) => {
        if (String(filename) === 'runtime' && !changed) {
          changed = true;
          try {
            writeFileSync(resource, changedBytes);
          } catch (error) {
            mutationError = error;
          }
        }
      });
      try {
        await assert.rejects(stageRuntimeCopy(proof, owned), /changed during staging/);
        assert.equal(changed, true);
        assert.equal(mutationError, undefined);
        assert.equal((await lstat(path.join(root, 'runtime'))).isDirectory(), true);
        await assert.rejects(lstat(path.join(root, 'native-perf-runtime.json')), {
          code: 'ENOENT',
        });
        const evidence = JSON.parse(
          await readFile(path.join(root, 'native-perf-runtime-failure.json'), 'utf8'),
        );
        assert.equal(evidence.phase, 'after-copy-verification');
        assert.equal(evidence.performanceSample, false);
      } finally {
        watcher.close();
      }
    });
  },
);
