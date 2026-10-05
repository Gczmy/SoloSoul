/** RF-312：媒体输入合同与原生准备证明；仅显式 sdk-media 模式使用。 */
import { lstat, realpath, readFile } from 'node:fs/promises';
import path from 'node:path';
import { validateFixtureManifest, sha256, normalizeWindowsPath } from './native-perf-run.mjs';
const exact = (v, keys) =>
  v &&
  typeof v === 'object' &&
  !Array.isArray(v) &&
  Object.keys(v).length === keys.length &&
  keys.every((k) => Object.hasOwn(v, k));
const same = (a, b) => {
  if (Array.isArray(a))
    return Array.isArray(b) && a.length === b.length && a.every((v, i) => same(v, b[i]));
  if (a && typeof a === 'object')
    return b && exact(b, Object.keys(a)) && Object.keys(a).every((k) => same(a[k], b[k]));
  return a === b;
};
export const PUBLIC_MEDIA_ASSETS = Object.freeze(
  [
    [
      'ocr_test.png',
      'image/png',
      6274,
      '56a9d54d9b70a3ee1b22e0b08b755b6cfc15ee125df4b11d52f8523583dcacaa',
    ],
    [
      'text_only.pdf',
      'application/pdf',
      3035,
      'ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe',
    ],
    [
      'scanned.pdf',
      'application/pdf',
      1921,
      '8fda928a9940813a4a245a2db1e943c64b4e717d20274f9b4f27ee5b9b3fc4a1',
    ],
    [
      'preview.txt',
      'text/plain',
      71,
      '24602f12d06466931694ba59d04f040af0334a63d28b10ac83f07ea5b16e0339',
    ],
  ].map(([fileName, mimeType, bytes, sha256]) =>
    Object.freeze({ fileName, mimeType, bytes, sha256 }),
  ),
);
export function validateMediaManifest(value) {
  if (
    !exact(value, [
      'schemaVersion',
      'scope',
      'generator',
      'baseFixture',
      'mediaObjectId',
      'includesAttachments',
      'includesOcrFixture',
      'publicAssets',
      'attachments',
      'closedFiles',
    ]) ||
    value.schemaVersion !== 2 ||
    value.scope !== 'synthetic-native-media-vault-fixture' ||
    value.generator !== 'solosoul-core/examples/perf_baseline --media-fixture-output' ||
    value.mediaObjectId !== 'obj_perf_00000000' ||
    value.includesAttachments !== true ||
    value.includesOcrFixture !== true ||
    !same(value.publicAssets, PUBLIC_MEDIA_ASSETS) ||
    !Array.isArray(value.attachments) ||
    value.attachments.length !== 4 ||
    !Array.isArray(value.closedFiles) ||
    value.closedFiles.length !== 8
  )
    throw new Error('Media fixture differs from exact public contract');
  validateFixtureManifest(value.baseFixture);
  if (
    !exact(value.baseFixture, [
      'accountId',
      'accountName',
      'buildProfile',
      'determinism',
      'expectedSearchMatches',
      'fixture',
      'generator',
      'includesAttachments',
      'includesOcrFixture',
      'includesProfile',
      'includesUiPreferences',
      'kdf',
      'objectCount',
      'schemaVersion',
      'scope',
      'searchQuery',
    ]) ||
    !exact(value.baseFixture.kdf, ['iterations', 'memoryKiB', 'parallelism']) ||
    value.baseFixture.determinism !==
      'object-content-only; salt, encryption nonce and account/profile timestamps vary'
  )
    throw new Error('Media base fixture is not the exact fixed contract');
  const ids = new Set();
  for (const [index, att] of value.attachments.entries()) {
    if (
      !exact(att, ['id', 'relativePath']) ||
      typeof att.id !== 'string' ||
      !/^att_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(att.id) ||
      ids.has(att.id) ||
      att.relativePath !==
        'attachments/obj_perf_00000000/' + att.id + '/' + PUBLIC_MEDIA_ASSETS[index].fileName
    )
      throw new Error('Media attachment path/ID rejected');
    ids.add(att.id);
  }
  const account = value.baseFixture.accountId;
  const paths = [
    account + '/config.json',
    account + '/vault.db',
    'accounts.json',
    'ui_preferences.json',
    ...value.attachments.map((a) => a.relativePath),
  ].sort();
  if (
    value.closedFiles.some(
      (f, i) =>
        !exact(f, ['relativePath', 'sha256']) ||
        f.relativePath !== paths[i] ||
        typeof f.sha256 !== 'string' ||
        !/^[a-f0-9]{64}$/.test(f.sha256),
    )
  )
    throw new Error('Media closed-file proof rejected');
  return value;
}
async function regular(file, maxBytes) {
  const stat = await lstat(file);
  if (
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    stat.size > maxBytes ||
    (process.platform === 'win32'
      ? normalizeWindowsPath(await realpath(file)) !== normalizeWindowsPath(file)
      : (await realpath(file)) !== path.resolve(file))
  )
    throw new Error('Media file must be bounded and regular');
}
export async function readMediaManifest(root) {
  const file = path.join(root, 'rf312-media-fixture.json');
  await regular(file, 128 * 1024);
  const value = validateMediaManifest(JSON.parse(await readFile(file, 'utf8')));
  for (const f of value.closedFiles) {
    const full = path.join(root, ...f.relativePath.split('/'));
    await regular(full, 256 * 1024 * 1024);
    if ((await sha256(full)) !== f.sha256)
      throw new Error('Media closed-file bytes differ from marker');
  }
  return value;
}
export function checkPreparedMedia(owned, source) {
  const marker = validateMediaManifest(owned.fixture?.marker);
  if (
    !['baseFixture', 'publicAssets', 'attachments'].every((k) => same(marker[k], source[k])) ||
    owned.fixture.searchMatches !== source.baseFixture.expectedSearchMatches ||
    !Array.isArray(owned.fixture.files) ||
    owned.fixture.files.length !== 9
  )
    throw new Error('Prepared media proof does not bind the source contract');
  const files = [
    ...marker.closedFiles,
    { relativePath: 'rf312-media-fixture.json', sha256: null },
  ].sort((a, b) =>
    a.relativePath < b.relativePath ? -1 : a.relativePath > b.relativePath ? 1 : 0,
  );
  if (
    owned.fixture.files.some(
      (f, i) =>
        !exact(f, ['relativePath', 'sha256']) ||
        f.relativePath !== files[i].relativePath ||
        !/^[a-f0-9]{64}$/.test(f.sha256) ||
        (files[i].sha256 !== null && f.sha256 !== files[i].sha256),
    )
  )
    throw new Error('Prepared media files differ from owned marker');
  return owned;
}

export async function verifyPreparedMediaFiles(owned, source) {
  checkPreparedMedia(owned, source);
  const actual = await readMediaManifest(owned.vault);
  if (!same(actual, owned.fixture.marker))
    throw new Error('Prepared media marker differs from raw owned marker');
  for (const f of owned.fixture.files) {
    const full = path.join(owned.vault, ...f.relativePath.split('/'));
    await regular(full, 256 * 1024 * 1024);
    if ((await sha256(full)) !== f.sha256)
      throw new Error('Prepared media bytes differ from owned proof');
  }
}
